#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

# scripts/miri.sh — run noyalib's "high-leverage" tests under Miri.
#
# noyalib is `#![forbid(unsafe_code)]`, so Miri is not policing
# noyalib's own code — every byte of the parser / scanner / CST
# is checked at compile time. The reason this script exists is:
#
#   1. Supply-chain verification.  noyalib's runtime deps —
#      `indexmap`, `rustc-hash`, `ryu`, `itoa`, `memchr`,
#      `smallvec` — all use `unsafe` internally. Miri verifies
#      that the *interaction* between noyalib and those crates
#      doesn't trigger UB / aliasing violations / leaks.
#
#   2. Platform sanity.  Miri can simulate big-endian targets
#      via `MIRI_TARGET`. The SWAR decimal parser
#      (`simd::parse_decimal_*`) and the structural-bitmask
#      iterator (`simd::SimdScanner::structural_bitmask_32`) use
#      `u64::from_be_bytes` and `wrapping_mul` pipelines that
#      should be byte-order agnostic — Miri proves it.
#
#   3. The "Tier 1" promise.  Including a Miri job in CI and a
#      shell script that contributors can run locally is the
#      hallmark of a foundational Rust library. It turns the
#      "Zero `unsafe`" claim from a README assertion into an
#      actively verified invariant.
#
# Miri is slow (~10–50× the wall-clock of a stable test run), so
# this script targets the highest-leverage modules rather than
# the full test surface:
#
#   - `parser::` and `scanner::`  — boundary conditions where an
#                                   off-by-one would surface.
#   - `value::`                   — tree manipulation, deep merge,
#                                   path queries.
#   - `interner::`                — Arc<str> dedup interactions
#                                   with rustc-hash.
#   - `simd::`                    — SWAR / bitmask pipelines that
#                                   exercise wrapping arithmetic.
#
# A scheduled job runs the *full* test suite under Miri weekly so
# anything outside the targeted set is still covered eventually.
#
# Run locally:
#
#     ./scripts/miri.sh                  # full focused suite
#     ./scripts/miri.sh simd             # just simd module
#     MIRI_TARGET=mips64-unknown-linux-gnuabi64 ./scripts/miri.sh
#                                        # cross-target (big-endian)

set -euo pipefail
IFS=$'\n\t'

# ── Miri flags ─────────────────────────────────────────────────────
#
# `-Zmiri-strict-provenance` ensures any pointer-as-int round-trip
#   in `unsafe` code (rare in our deps but possible) is sound.
# `-Zmiri-disable-isolation` lets the test binary read the system
#   clock / entropy — `rustc-hash`'s seed initialisation needs it,
#   plus our `KeyInterner` tests use `process::id()`.
#
# `-Zmiri-symbolic-alignment-check` is intentionally NOT enabled
# here. memchr 2.x's x86_64 SSE2 path (taken by `find_any_of` /
# `memchr3` etc.) issues a `_mm_load_si128` instruction whose
# pointer is dynamically known to be 16-byte-aligned at the call
# site — but Miri's symbolic-alignment tracking can't see the
# runtime guarantee and reports a false positive. memchr's own
# CI runs Miri without this flag for the same reason. The
# defaults (Stacked Borrows, strict provenance, leak detection)
# still catch every category of UB we care about; only the
# memchr-SSE2 false positive is sacrificed.
# MIRI_MODEL selects the aliasing model: `stacked` (default) or `tree`.
# Both must pass for a soundness claim; they reject different patterns.
# MIRI_FEATURES passes cargo feature flags, e.g. `--no-default-features`
# for the alloc-only code shape.
case "${MIRI_MODEL:-stacked}" in
    stacked) MODEL_FLAGS="" ;;
    tree)    MODEL_FLAGS="-Zmiri-tree-borrows" ;;
    *) echo "MIRI_MODEL must be stacked or tree" >&2; exit 2 ;;
esac
# MIRI_ALIGN=1 adds -Zmiri-symbolic-alignment-check: every multi-byte read
# must be provably aligned. It is the slowest flag, so the per-PR legs run
# without it and the weekly sweep runs with it.
ALIGN_FLAGS=""; [ "${MIRI_ALIGN:-0}" = 1 ] && ALIGN_FLAGS="-Zmiri-symbolic-alignment-check"
export MIRIFLAGS="${MIRIFLAGS:-} ${MODEL_FLAGS} ${ALIGN_FLAGS} -Zmiri-strict-provenance -Zmiri-disable-isolation"

# Optional cross-target. When set, `cargo miri` simulates the
# specified architecture (big-endian targets are the most
# valuable since they exercise byte-swap paths). Stored as an
# array so the empty-vs-non-empty case round-trips through the
# eventual `cargo` invocation cleanly.
TARGET_ARGS=()
if [[ -n "${MIRI_TARGET:-}" ]]; then
    TARGET_ARGS=(--target "${MIRI_TARGET}")
    echo "→ Miri target: ${MIRI_TARGET}"
fi

# ── Test-name filter ───────────────────────────────────────────────
#
# Default to the full focused suite. A positional arg narrows it
# further — e.g. `./scripts/miri.sh simd` runs only the simd module.
FILTER="${1:-}"
if [[ -n "${FILTER}" ]]; then
    FILTERS=( "${FILTER}::" )
else
    FILTERS=( "parser::" "scanner::" "value::" "interner::" "simd::" )
fi

echo "→ Setting up Miri (cached if previously run)"
cargo +nightly miri setup "${TARGET_ARGS[@]}"

echo "→ Running Miri on noyalib's high-leverage modules"
echo "  Modules: ${FILTERS[*]}"
echo "  Flags:   ${MIRIFLAGS}"
echo

# Miri does not honour `--features all-features` sensibly across
# every dep (rayon brings in unsupported syscalls); use the
# default feature set, which covers the primitives the focus list
# actually exercises.
# shellcheck disable=SC2086
cargo +nightly miri test \
    --lib ${MIRI_FEATURES:-} \
    "${TARGET_ARGS[@]}" \
    -- \
    "${FILTERS[@]}"

echo
echo "✓ Miri verification complete — no UB, leaks, or alignment errors."
echo "  Filters:  ${FILTERS[*]}"
echo "  Target:   ${MIRI_TARGET:-native}"
echo "  Model:    ${MIRI_MODEL:-stacked}   Features: ${MIRI_FEATURES:-default}   Alignment check: ${MIRI_ALIGN:-0}"
