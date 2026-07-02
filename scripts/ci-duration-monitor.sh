#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Wall-clock CI regression monitor. Closes #127 AC #5:
#
#   "record baseline CI duration; script fails if a new run
#    exceeds 1.1× rolling 5-run average"
#
# Reads the last N + 1 successful CI runs of `.github/workflows/
# ci.yml` on main via the GitHub REST API, computes the rolling
# N-run average of runs 2..N+1, and compares the latest run
# (run 1) against that baseline. Exits non-zero if the latest
# run exceeds `${THRESHOLD_RATIO}` × baseline.
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
N_BASELINE="${N_BASELINE:-5}"
THRESHOLD_RATIO="${THRESHOLD_RATIO:-1.1}"
REPO="${GITHUB_REPOSITORY:-$(git remote get-url origin | sed -E 's#(git@github.com:|https://github.com/)##; s#\.git$##')}"

echo "── CI duration monitor ──"
echo "  repo:        ${REPO}"
echo "  branch:      ${BRANCH}"
echo "  workflow:    ${WORKFLOW_FILE}"
echo "  baseline N:  ${N_BASELINE}"
echo "  threshold:   ${THRESHOLD_RATIO}×"
echo

# ── Fetch recent successful CI runs ────────────────────────────────
NEED=$((N_BASELINE + 1))
RUNS=$(gh api "/repos/${REPO}/actions/workflows/${WORKFLOW_FILE}/runs?branch=${BRANCH}&status=success&per_page=${NEED}" \
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

LATEST=$(printf '%s' "${DURATIONS}" | head -1)
BASELINE_LIST=$(printf '%s' "${DURATIONS}" | tail -n +2 | head -"${N_BASELINE}")

# ── Rolling average (integer arithmetic via awk to avoid python) ──
BASELINE_AVG=$(printf '%s\n' "${BASELINE_LIST}" | awk 'BEGIN {sum=0; n=0} {sum+=$1; n+=1} END {printf "%.1f", sum/n}')

# Threshold in seconds.
LATEST_INT=${LATEST%.*}
THRESHOLD_SEC=$(awk -v b="${BASELINE_AVG}" -v t="${THRESHOLD_RATIO}" 'BEGIN {printf "%.1f", b * t}')

RATIO=$(awk -v l="${LATEST_INT}" -v b="${BASELINE_AVG}" 'BEGIN {printf "%.2f", l / b}')

echo "  latest run:  ${LATEST_INT}s"
echo "  baseline (${N_BASELINE}-run avg): ${BASELINE_AVG}s"
echo "  threshold:   ${THRESHOLD_SEC}s (${THRESHOLD_RATIO}× baseline)"
echo "  observed:    ${RATIO}× baseline"
echo

REGRESSION=$(awk -v l="${LATEST_INT}" -v t="${THRESHOLD_SEC}" 'BEGIN {print (l > t) ? 1 : 0}')

if [[ "${REGRESSION}" == "1" ]]; then
    cat >&2 <<EOF
  [FAIL] wall-clock regression: latest run ${LATEST_INT}s > threshold ${THRESHOLD_SEC}s
         (observed ${RATIO}×; gate is ${THRESHOLD_RATIO}×)

  Investigate the shared-workflow bumps landed since the last
  green baseline. See #127 AC #5 for the invariant.
EOF
    exit 1
fi

echo "  [ OK ] latest run within threshold."
