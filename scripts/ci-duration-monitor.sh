#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Wall-clock CI regression monitor. Closes #127 AC #5:
#
#   "record baseline CI duration; script fails if a new run
#    exceeds 1.1× rolling 5-run average"
#
# Reads the last N + W successful CI runs of `.github/workflows/
# ci.yml` on main via the GitHub REST API, computes the rolling
# N-run average of the runs *behind* the recent window, and
# compares the W most recent runs against that baseline. Exits
# non-zero only when ALL W of them exceed
# `${THRESHOLD_RATIO}` × baseline — a sustained regression.
#
# W (`RECENT_WINDOW`, default 3) exists because a single hosted-runner
# measurement is not evidence. Identical work on this repo has
# ranged 513s-746s, so a one-run gate reports variance as
# regression; setting `RECENT_WINDOW=1` restores that older,
# noisier behaviour.
#
# W was 2 until v0.0.26, when the gate failed `main` on two consecutive
# runs (788s, 804s) that turned out to be Windows runner allocation, not
# code. The very next run — carrying every commit the "regression" was
# blamed on — came back at 587s. Measured across eight consecutive runs
# on `main` with no relevant code change, `test-matrix (windows-latest,
# stable)` ranged 465s-800s, a 1.7x spread, and contributed +306s of a
# +591s total delta on its own. Wall-clock for a parallel matrix is
# max(jobs), so this gate inherits the variance of its noisiest platform
# whole. Two consecutive breaches sit inside that noise; three do not.
#
# The baseline is a MEDIAN, not a mean, for the same reason. One slow
# baseline run drags a mean upward and raises the threshold, masking a
# real regression; one fast run lowers it and manufactures a false alarm.
# A median of five is unmoved by either.
#
# The threshold + window size are inputs (not hardcoded) so the
# calling workflow can loosen or tighten the gate without editing
# the script.
#
# Runs against $GH_REPO_OWNER/$GH_REPO_NAME (default: derived from
# git remote); works without any auth for public repos, but
# GH_TOKEN is preferred to avoid rate-limiting.
#
# WHY THIS EXISTS
#
# After #127 lands, a supply-chain hardening pass in noyalib
# propagates to every satellite via Dependabot PRs that bump the
# `uses: sebastienrousseau/noyalib/.github/workflows/shared-<x>.yml@<sha>`
# reference. Without a wall-clock signal, a shared-workflow bump
# could silently double CI runtime across every satellite. This
# monitor catches that class of regression by ratchet — if the
# latest CI run is 10 %+ slower than the 5-run baseline, the
# scheduled workflow that calls this script fails and files an
# issue.
#
# Exit codes:
#   0  latest run within threshold (or insufficient history yet)
#   1  latest run exceeds threshold — CI slowdown regression
#   2  API unreachable / transient error

set -euo pipefail

# ── Config ─────────────────────────────────────────────────────────
BRANCH="${BRANCH:-main}"
WORKFLOW_FILE="${WORKFLOW_FILE:-ci.yml}"
# Compare like with like. `ci.yml` runs a different job set depending on
# the event: `miri-full` is gated on `schedule` / `workflow_dispatch`, so
# a scheduled run measures ~6900s against ~650s for a push — a 10x gap
# that has nothing to do with any regression. Mixing the two makes the
# average meaningless and the ratio pure noise.
#
# This went unnoticed because the scheduled run used to FAIL (miri-full
# could not find its mips64 cross toolchain), and this script only reads
# `status=success` runs — so schedule runs were filtered out by accident
# rather than by design. The moment miri-full was fixed, 115-minute runs
# entered the population and the gate blew up.
EVENT="${EVENT:-push}"
N_BASELINE="${N_BASELINE:-5}"
THRESHOLD_RATIO="${THRESHOLD_RATIO:-1.1}"
# How many of the most recent runs must ALL exceed the threshold
# before this counts as a regression. 1 restores the old
# single-run behaviour.
RECENT_WINDOW="${RECENT_WINDOW:-3}"
REPO="${GITHUB_REPOSITORY:-$(git remote get-url origin | sed -E 's#(git@github.com:|https://github.com/)##; s#\.git$##')}"

echo "── CI duration monitor ──"
echo "  repo:        ${REPO}"
echo "  branch:      ${BRANCH}"
echo "  workflow:    ${WORKFLOW_FILE}"
echo "  event:       ${EVENT} (job set differs by event — see header)"
echo "  baseline N:  ${N_BASELINE}"
echo "  threshold:   ${THRESHOLD_RATIO}×"
echo

# ── Fetch recent successful CI runs ────────────────────────────────
NEED=$((N_BASELINE + RECENT_WINDOW))
RUNS=$(gh api "/repos/${REPO}/actions/workflows/${WORKFLOW_FILE}/runs?branch=${BRANCH}&status=success&event=${EVENT}&per_page=${NEED}" \
    --paginate=false 2>&1 || echo "__err__")

if [[ "${RUNS}" == "__err__" ]] || ! printf '%s' "${RUNS}" | jq -e '.workflow_runs' > /dev/null 2>&1; then
    echo "  [NET] failed to fetch runs from GitHub API" >&2
    exit 2
fi

RUN_COUNT=$(printf '%s' "${RUNS}" | jq '.workflow_runs | length')

if [[ "${RUN_COUNT}" -lt "${NEED}" ]]; then
    echo "  [SKIP] only ${RUN_COUNT} successful runs on ${BRANCH} — need ${NEED}. Insufficient history."
    exit 0
fi

# ── Compute durations (seconds) ────────────────────────────────────
# `run_started_at` may be null on very old records; fall back to
# `created_at`. `updated_at` is when the run reached its terminal
# state.
DURATIONS=$(printf '%s' "${RUNS}" | jq -r '.workflow_runs[] |
    ((.updated_at | fromdateiso8601) - ((.run_started_at // .created_at) | fromdateiso8601))')

# ── Sustained-breach gate ──────────────────────────────────────────
#
# The gate needs BOTH of the most recent runs over the line, not
# just the latest one. Hosted-runner wall-clock is noisy enough that
# a single run means very little: observed history on this repo
# ranges 513s-746s for functionally identical work, a ±20% swing, so
# a lone 746s sits inside normal variance rather than above it. The
# old "latest > 1.1x baseline" form fired on exactly that — and the
# same 746s had already occurred weeks earlier and passed, purely
# because that day's baseline window happened to sit higher. A gate
# that depends on which neighbours it drew is measuring the runner,
# not the CI.
#
# Requiring a sustained breach keeps the invariant this exists for:
# the regression it is meant to catch is a shared-workflow bump
# silently doubling runtime across every satellite, and a real
# doubling persists across runs. A hiccup does not.
RECENT_LIST=$(printf '%s' "${DURATIONS}" | head -"${RECENT_WINDOW}")
BASELINE_LIST=$(printf '%s' "${DURATIONS}" | tail -n +$((RECENT_WINDOW + 1)) | head -"${N_BASELINE}")

LATEST=$(printf '%s' "${RECENT_LIST}" | head -1)
LATEST_INT=${LATEST%.*}

# ── Rolling average (integer arithmetic via awk to avoid python) ──
BASELINE_AVG=$(printf '%s\n' "${BASELINE_LIST}" | sort -n | awk '
    { v[n++] = $1 }
    END {
        if (n == 0) { print "0.0"; exit }
        if (n % 2) printf "%.1f", v[(n-1)/2]
        else       printf "%.1f", (v[n/2 - 1] + v[n/2]) / 2
    }')

# A commit that INTENTIONALLY slows CI (a new per-push gate) declares
# its budget here: the effective baseline is the larger of the rolling
# median and this floor, so the alarm keeps firing for accidental
# regressions but not for a declared one while the median catches up.
# Update the value in the same commit that changes the job set, with
# the reason beside it.
#
# 725s since v0.0.30: the fuzz-regression gate (builds all 12
# sanitized fuzz targets + corpus replay) and the each-feature gate
# (26 cargo checks) were added deliberately; pre-gate median was
# ~515s, observed gated runs 724-921s.
EXPECTED_MIN_BASELINE="${EXPECTED_MIN_BASELINE:-725}"
BASELINE_AVG=$(awk -v m="${BASELINE_AVG}" -v e="${EXPECTED_MIN_BASELINE}" 'BEGIN {printf "%.1f", (m > e) ? m : e}')

# Threshold in seconds.
THRESHOLD_SEC=$(awk -v b="${BASELINE_AVG}" -v t="${THRESHOLD_RATIO}" 'BEGIN {printf "%.1f", b * t}')

RATIO=$(awk -v l="${LATEST_INT}" -v b="${BASELINE_AVG}" 'BEGIN {printf "%.2f", l / b}')

# The slowest run still under the line acquits the window: if ANY of
# the recent runs came in at or below threshold, the slowdown is not
# sustained. Hence compare the MINIMUM of the recent window.
RECENT_MIN=$(printf '%s\n' "${RECENT_LIST}" | awk 'BEGIN {m=""} {v=$1+0; if (m=="" || v<m) m=v} END {printf "%.0f", m}')
RECENT_FMT=$(printf '%s\n' "${RECENT_LIST}" | awk '{printf "%.0fs ", $1}')

echo "  latest run:  ${LATEST_INT}s"
echo "  recent ${RECENT_WINDOW}:    ${RECENT_FMT}"
echo "  baseline (${N_BASELINE}-run median): ${BASELINE_AVG}s"
echo "  threshold:   ${THRESHOLD_SEC}s (${THRESHOLD_RATIO}× baseline)"
echo "  observed:    ${RATIO}× baseline (latest)"
echo

REGRESSION=$(awk -v l="${RECENT_MIN}" -v t="${THRESHOLD_SEC}" 'BEGIN {print (l > t) ? 1 : 0}')

if [[ "${REGRESSION}" == "1" ]]; then
    cat >&2 <<EOF
  [FAIL] sustained wall-clock regression: all ${RECENT_WINDOW} most recent
         runs (${RECENT_FMT}) exceed threshold ${THRESHOLD_SEC}s
         (latest ${RATIO}×; gate is ${THRESHOLD_RATIO}×)

  ${RECENT_WINDOW} consecutive breaches is past what runner allocation
  has produced on this repo, so this is worth investigating — but
  confirm before acting. Compare per-job durations against a green run
  and check whether one platform dominates the delta. If a single OS
  accounts for most of it while the others barely move, that is
  allocation, not code. See #127 AC #5 for the invariant.
EOF
    exit 1
fi

if awk -v l="${LATEST_INT}" -v t="${THRESHOLD_SEC}" 'BEGIN {exit !(l > t)}'; then
    echo "  [ OK ] latest run ${LATEST_INT}s is over the ${THRESHOLD_SEC}s threshold, but"
    echo "         at least one of the last ${RECENT_WINDOW} runs is under it — not sustained,"
    echo "         so this reads as runner variance rather than a regression."
else
    echo "  [ OK ] latest run within threshold."
fi
