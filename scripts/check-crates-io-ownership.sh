#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Guard against mid-split namespace squats.
#
# For every crate name that the workspace-split project depends on
# (noyalib, noyalib-wasm, noyalib-mcp, noyalib-lsp, noya-cli), this
# script asserts that the crates.io owner is still `sebastienrousseau`.
# If any owner has changed — because the crate was force-transferred,
# yanked-then-taken-over, or a delisted-and-re-registered exploit —
# CI fails immediately.
#
# Runs as a scheduled workflow (daily at 04:00 UTC) plus on every PR
# labelled `workspace-split` so the pre-work is validated whenever
# the split project touches the tree.
#
# Exit codes:
#   0  every satellite name still owned by sebastienrousseau
#   1  at least one owner drift detected (see stderr for names)
#   2  crates.io API unreachable / transient network error
#
# References:
#   Issue #125 acceptance criterion #5
#   ADR-0005 (workspace split, cross-repo dep policy)

set -euo pipefail

EXPECTED_OWNER="sebastienrousseau"
EXPECTED_OWNER_ID="186843"

CRATES=(
    noyalib
    noyalib-wasm
    noyalib-mcp
    noyalib-lsp
    noya-cli
)

DRIFT=0
UNREACHABLE=0

for CRATE in "${CRATES[@]}"; do
    HTTP=$(curl -sfL -o /dev/null -w "%{http_code}" \
        "https://crates.io/api/v1/crates/${CRATE}/owners" \
        --max-time 10 \
        --retry 3 \
        --retry-delay 2 \
        || echo "network-error")

    case "${HTTP}" in
        network-error|000)
            printf '  [NET ] %s — crates.io unreachable\n' "${CRATE}" >&2
            UNREACHABLE=$((UNREACHABLE + 1))
            continue
            ;;
        404)
            printf '  [FAIL] %s — HTTP 404, name is UNCLAIMED (was reserved by us? someone deleted it?)\n' "${CRATE}" >&2
            DRIFT=$((DRIFT + 1))
            continue
            ;;
        200)
            ;;
        *)
            printf '  [WARN] %s — unexpected HTTP %s\n' "${CRATE}" "${HTTP}" >&2
            UNREACHABLE=$((UNREACHABLE + 1))
            continue
            ;;
    esac

    OWNER_JSON=$(curl -sfL "https://crates.io/api/v1/crates/${CRATE}/owners" \
        --max-time 10 --retry 3 --retry-delay 2)

    OWNER_LOGIN=$(printf '%s' "${OWNER_JSON}" | jq -r '.users[0].login // "??"')
    OWNER_ID=$(printf '%s' "${OWNER_JSON}" | jq -r '.users[0].id // "??"')

    if [[ "${OWNER_LOGIN}" == "${EXPECTED_OWNER}" && "${OWNER_ID}" == "${EXPECTED_OWNER_ID}" ]]; then
        printf '  [ OK ] %s — owner: %s (id=%s)\n' "${CRATE}" "${OWNER_LOGIN}" "${OWNER_ID}"
    else
        printf '  [FAIL] %s — owner DRIFT: got %s (id=%s), expected %s (id=%s)\n' \
            "${CRATE}" "${OWNER_LOGIN}" "${OWNER_ID}" \
            "${EXPECTED_OWNER}" "${EXPECTED_OWNER_ID}" >&2
        DRIFT=$((DRIFT + 1))
    fi
done

echo
if [[ ${DRIFT} -gt 0 ]]; then
    printf '── FAIL: %d owner drift(s) detected. Investigate before opening the next split PR.\n' "${DRIFT}" >&2
    exit 1
fi

if [[ ${UNREACHABLE} -gt 0 ]]; then
    printf '── WARN: %d crate(s) unreachable due to network; retry when connectivity returns.\n' "${UNREACHABLE}" >&2
    exit 2
fi

printf '── OK: all %d satellite names still owned by %s.\n' "${#CRATES[@]}" "${EXPECTED_OWNER}"
