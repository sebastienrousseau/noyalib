#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Self-tests for the gate scripts. The gates are load-bearing (a bug
# weakens every release silently), so each is exercised against
# known-good and known-bad fixtures. Adapted from the pattern in
# Takazudo/zudo-front-builder's scripts/__tests__.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
pass=0; fail=0
ok()  { pass=$((pass+1)); }
bad() { fail=$((fail+1)); echo "  [FAIL] $1" >&2; }

# ── verify-release-versions ────────────────────────────────────────
# Derive the tree's own version so this fixture survives every bump
# (it went stale at the v0.0.31 bump when hardcoded).
CUR="v$(grep -m1 '^version = ' crates/noyalib/Cargo.toml | cut -d'"' -f2)"
# A branch carries its version from creation; the dated CHANGELOG
# heading is the one release-time step. So mid-cycle the gate may
# fail ONLY on the missing heading — verify the positive path by
# inserting a temporary heading, then restoring.
perl -0pi -e "s/^## \[Unreleased\]\n/## [Unreleased]\n\n## [$CUR] - 2099-01-01\n/m" CHANGELOG.md
if ./scripts/verify-release-versions.sh "$CUR" >/dev/null 2>&1; then ok; else bad "gate rejects the tree's own version ($CUR) even with a heading"; fi
git checkout -q -- CHANGELOG.md
# A version nothing agrees on: must fail.
if ./scripts/verify-release-versions.sh v9.9.9 >/dev/null 2>&1; then bad "gate accepted v9.9.9"; else ok; fi
# Stale CITATION.cff must fail (restored via git checkout).
perl -0pi -e "s/^## \[Unreleased\]\n/## [Unreleased]\n\n## [$CUR] - 2099-01-01\n/m" CHANGELOG.md
perl -pi -e 's/^version: .*/version: 0.0.1/' CITATION.cff
if ./scripts/verify-release-versions.sh "$CUR" >/dev/null 2>&1; then bad "gate missed a stale CITATION.cff"; else ok; fi
git checkout -q -- CITATION.cff CHANGELOG.md

# ── check-docs-links ───────────────────────────────────────────────
if ./scripts/check-docs-links.sh >/dev/null 2>&1; then ok; else bad "link gate rejects the clean tree"; fi
echo '[broken](does-not-exist.md)' > docs/__selftest_broken.md
if ./scripts/check-docs-links.sh >/dev/null 2>&1; then bad "link gate missed a broken link"; else ok; fi
rm -f docs/__selftest_broken.md

# ── ci-duration-monitor declared-budget floor ──────────────────────
# The floor arithmetic: max(median, EXPECTED_MIN_BASELINE). Probe the
# awk expression the script uses rather than the network path.
floor=$(awk -v m="500" -v e="725" 'BEGIN {printf "%.1f", (m > e) ? m : e}')
if [ "$floor" = "725.0" ]; then ok; else bad "declared-budget floor arithmetic ($floor)"; fi
floor=$(awk -v m="900" -v e="725" 'BEGIN {printf "%.1f", (m > e) ? m : e}')
if [ "$floor" = "900.0" ]; then ok; else bad "rolling median must win once above the floor ($floor)"; fi

echo "gate self-tests: $pass passed, $fail failed"
[ "$fail" = 0 ]
