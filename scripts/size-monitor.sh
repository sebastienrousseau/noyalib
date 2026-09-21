#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Shipped-size regression gate: packages the crate and compares the
# artefact against the declared ceiling in scripts/size-budgets.toml.
# See that file for the budget contract.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

BUDGET=$(grep -A2 '^\[crate-package\]' scripts/size-budgets.toml | grep max_bytes | grep -oE '[0-9]+')
# Packaging a dirty tree can measure bytes that are not represented by the
# audited commit. Cargo's cleanliness check is therefore part of this gate.
cargo package -p noyalib --no-verify -q
NOYALIB_PKGID=$(cargo pkgid -p noyalib)
NOYALIB_VERSION=${NOYALIB_PKGID##*#}
NOYALIB_CRATE="target/package/noyalib-${NOYALIB_VERSION}.crate"
if [ ! -f "${NOYALIB_CRATE}" ]; then
  echo "[FAIL] cargo did not produce ${NOYALIB_CRATE}." >&2
  exit 1
fi
SIZE=$(wc -c < "${NOYALIB_CRATE}" | tr -d ' ')

echo "── shipped-size monitor ──"
echo "  artefact: ${NOYALIB_CRATE}"
echo "  size:     ${SIZE} bytes"
echo "  budget:   ${BUDGET} bytes"
if [ "${SIZE}" -gt "${BUDGET}" ]; then
  echo "  [FAIL] the packaged crate exceeds its declared budget." >&2
  echo "         If the growth is intentional, raise the ceiling in" >&2
  echo "         scripts/size-budgets.toml in this same commit, with the reason." >&2
  exit 1
fi
echo "  [OK] within budget ($(( (BUDGET - SIZE) * 100 / BUDGET ))% headroom)"
