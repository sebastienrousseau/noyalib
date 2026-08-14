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

# crates.io's crawler policy requires a User-Agent that identifies the
# client and offers a way to make contact. Requests without one are
# rejected with 403 — not always from a workstation, where curl's
# default `curl/8.x` is tolerated, but reliably from shared cloud IPs
# like GitHub's hosted runners. That is why this check passed locally
# while failing in CI on every crate at once.
CRATES_IO_UA="noyalib-ownership-check (+https://github.com/sebastienrousseau/noyalib)"

for CRATE in "${CRATES[@]}"; do
    # NOTE: deliberately no `-f` here. With `--fail` curl exits non-zero
    # on an HTTP error, so `|| echo …` *appended* to the status curl had
    # already written via `-w` — which is how a 403 surfaced as the
    # nonsense status "403network-error" and got misfiled as a network
    # fault. Without `-f`, HTTP errors are not curl failures, so
    # `%{http_code}` is authoritative and the `if !` below fires only on
    # a genuine transport error, replacing the value rather than
    # extending it.
    if ! HTTP=$(curl -sSL -o /dev/null -w "%{http_code}" \
        -A "${CRATES_IO_UA}" \
        "https://crates.io/api/v1/crates/${CRATE}/owners" \
        --max-time 10 \
        --retry 3 \
        --retry-delay 2 2>/dev/null); then
        HTTP="network-error"
    fi

    case "${HTTP}" in
        network-error|000)
            printf '  [NET ] %s — crates.io unreachable\n' "${CRATE}" >&2
            UNREACHABLE=$((UNREACHABLE + 1))
            continue
            ;;
        403)
            printf '  [NET ] %s — HTTP 403 from crates.io (User-Agent rejected or rate-limited)\n' "${CRATE}" >&2
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
        -A "${CRATES_IO_UA}" \
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
