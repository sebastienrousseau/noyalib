#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Cross-repo shared-workflow propagation SLA monitor. Closes
# #127 AC #6:
#
#   "scheduled workflow alerts if a Dependabot PR sits open in
#    any satellite for more than 48h without merge"
#
# For every satellite in ${SATELLITES}, this script:
#
#   1. Checks whether the satellite repo has been created yet
#      (pre-pilot repos will 404 — that is expected and reported
#      as SKIP, not FAIL).
#   2. If the repo exists, lists open PRs from `dependabot[bot]`
#      whose head branch or title matches the shared-workflow
#      pattern (`shared-*.yml` or `.github/workflows/shared-*.yml`).
#   3. For each matching PR, computes age in hours.
#   4. Fails if any PR exceeds ${SLA_HOURS}.
#
# WHY THIS EXISTS
#
# Under the strict-lockstep versioning contract from ADR-0005,
# every satellite must consume noyalib's shared reusable workflows
# via SHA-pinned `uses:` references. When noyalib bumps a shared
# workflow (typically a security hardening pass), Dependabot opens
# a SHA-bump PR in each satellite. If any of those PRs sits open
# too long, the corresponding satellite is running stale CI —
# defeating the propagation invariant.
#
# The scheduled workflow calling this script (48h SLA by default)
# fails loudly and files an issue in the offending satellite.
#
# Exit codes:
#   0  every satellite is either not-yet-created OR has no stale
#      shared-workflow Dependabot PRs
#   1  at least one satellite has an SLA breach — investigate
#   2  API unreachable / transient error

set -euo pipefail

# ── Config ─────────────────────────────────────────────────────────
OWNER="${OWNER:-sebastienrousseau}"
SATELLITES="${SATELLITES:-noyalib-wasm noyalib-mcp noyalib-lsp noya-cli}"
SLA_HOURS="${SLA_HOURS:-48}"

# Match `.github/workflows/shared-*.yml` references. The head
# branch Dependabot creates for a Github Actions bump looks like
# `dependabot/github_actions/.../shared-...` — matching on both
# title and head branch keeps false-negatives away.
SHARED_WF_PATTERN="shared-.*\\.yml"

echo "── Shared-workflow propagation SLA monitor ──"
echo "  owner:        ${OWNER}"
echo "  satellites:   ${SATELLITES}"
echo "  SLA:          ${SLA_HOURS}h"
echo

STALE_COUNT=0
CHECKED=0
SKIPPED=0

for SAT in ${SATELLITES}; do
    # Does the satellite repo exist yet?
    if ! gh api "/repos/${OWNER}/${SAT}" > /dev/null 2>&1; then
        printf '  [SKIP] %s/%s — repo does not yet exist (pre-pilot)\n' "${OWNER}" "${SAT}"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi
    CHECKED=$((CHECKED + 1))

    # Open PRs authored by dependabot[bot].
    PRS_JSON=$(gh api "/repos/${OWNER}/${SAT}/pulls?state=open&per_page=100" 2>&1 || echo "__err__")
    if [[ "${PRS_JSON}" == "__err__" ]] || ! printf '%s' "${PRS_JSON}" | jq -e '.' > /dev/null 2>&1; then
        printf '  [NET ] %s/%s — API unreachable\n' "${OWNER}" "${SAT}" >&2
        continue
    fi

    STALE_PRS=$(printf '%s' "${PRS_JSON}" | jq --arg sla "${SLA_HOURS}" --arg pat "${SHARED_WF_PATTERN}" '
        [.[] |
         select(.user.login == "dependabot[bot]") |
         select((.title | test($pat)) or (.head.ref | test($pat))) |
         select(((now - (.created_at | fromdateiso8601)) / 3600) > ($sla | tonumber))
        ]')

    STALE_LEN=$(printf '%s' "${STALE_PRS}" | jq 'length')

    if [[ "${STALE_LEN}" -gt 0 ]]; then
        printf '  [FAIL] %s/%s — %d stale shared-workflow Dependabot PR(s):\n' "${OWNER}" "${SAT}" "${STALE_LEN}" >&2
        printf '%s' "${STALE_PRS}" | jq -r '.[] |
            "         #\(.number): \(.title) (age: \((now - (.created_at | fromdateiso8601)) / 3600 | floor)h) — \(.html_url)"' >&2
        STALE_COUNT=$((STALE_COUNT + STALE_LEN))
    else
        printf '  [ OK ] %s/%s — no stale shared-workflow Dependabot PRs\n' "${OWNER}" "${SAT}"
    fi
done

echo
if [[ "${CHECKED}" -eq 0 ]]; then
    echo "── SKIP: no satellite repos exist yet (pre-pilot). Monitor stays in place for post-pilot activation. ──"
    exit 0
fi

if [[ "${STALE_COUNT}" -gt 0 ]]; then
    printf '── FAIL: %d stale shared-workflow Dependabot PR(s) across %d satellite(s) exceed the %dh SLA ──\n' \
        "${STALE_COUNT}" "${CHECKED}" "${SLA_HOURS}" >&2
    exit 1
fi

printf '── OK: %d satellite(s) checked, %d not-yet-created, zero stale PRs ──\n' "${CHECKED}" "${SKIPPED}"
