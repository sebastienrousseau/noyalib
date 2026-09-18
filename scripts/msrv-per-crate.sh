#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# scripts/msrv-per-crate.sh — verify each workspace crate
# compiles cleanly against its declared `rust-version`.
#
# A workspace-wide `cargo +<msrv> check` only enforces the floor
# of the crate at the workspace root. With satellite crates that
# can declare higher floors (e.g. `noyalib-lsp` may need a newer
# rustc than the core lib), the workspace check leaves drift
# undetected — a satellite crate adopting a 1.85-only feature
# wouldn't break the gate until a downstream user pinned to 1.75.
#
# This script walks each `crates/*/Cargo.toml`, reads its
# `rust-version` field, installs that toolchain on demand, and
# runs `cargo +<msrv> check --manifest-path …` against the
# crate. Fails on the first mismatch.
#
# Usage:
#   ./scripts/msrv-per-crate.sh                 # check every crate
#   ./scripts/msrv-per-crate.sh noyalib-lsp     # check just one
#
# Run from the workspace root.

set -euo pipefail
IFS=$'\n\t'

ONLY="${1:-}"

# Locate every `crates/*/Cargo.toml` carrying a `rust-version`.
# (There was a skip here for `crates/xtask/`, an internal tool that
# tracked the host toolchain rather than a downstream-visible MSRV.
# That crate was retired with the workspace split, so the branch could
# no longer be taken and is gone.)
mapfile -t MANIFESTS < <(
    for m in crates/*/Cargo.toml; do
        # Filter on `rust-version = "..."` presence.
        if grep -qE '^rust-version *=' "$m"; then
            echo "$m"
        fi
    done
)

if [[ ${#MANIFESTS[@]} -eq 0 ]]; then
    echo "no crates carry a rust-version field — nothing to check"
    exit 0
fi

PASSED=0
FAILED=0

for manifest in "${MANIFESTS[@]}"; do
    crate_dir="$(dirname "$manifest")"
    crate_name="$(basename "$crate_dir")"

    if [[ -n "$ONLY" && "$crate_name" != "$ONLY" ]]; then
        continue
    fi

    msrv=$(grep -E '^rust-version *=' "$manifest" \
              | head -1 \
              | sed 's/.*"\(.*\)".*/\1/')

    if [[ -z "$msrv" ]]; then
        echo "skip ${crate_name} — rust-version is empty"
        continue
    fi

    echo "── ${crate_name}: rustc ${msrv} ──"

    # Install the toolchain if it's not already present. The
    # `--quiet` flag isn't accepted as a positional after the
    # `install` subcommand on every rustup version we ship to,
    # so let the install banner through and let CI's grep
    # scrollback hide it.
    if ! rustup toolchain list | grep -qE "(^|/)${msrv}(-|$)"; then
        rustup toolchain install "$msrv" --profile minimal --no-self-update
    fi

    # `cargo +<msrv> check` — typecheck only, no codegen, fast.
    # `--locked` ensures Cargo.lock isn't regenerated against a
    # newer toolchain's resolver.
    if cargo "+$msrv" check \
            --manifest-path "$manifest" \
            --locked \
            --quiet 2>&1; then
        echo "  ✓ ${crate_name} (rustc ${msrv})"
        PASSED=$((PASSED + 1))
    else
        echo "  ✗ ${crate_name} (rustc ${msrv})"
        FAILED=$((FAILED + 1))
    fi
done

echo
echo "── per-crate MSRV check complete ──"
echo "  passed: ${PASSED}"
echo "  failed: ${FAILED}"

if [[ $FAILED -gt 0 ]]; then
    exit 1
fi
