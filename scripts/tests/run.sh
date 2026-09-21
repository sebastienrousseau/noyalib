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

# These tests mutate tracked files and put them back. Restoring with
# `git checkout -- <file>` restores the *committed* content, which
# silently discards uncommitted work in that file — it threw away a
# freshly regenerated docs/ECOSYSTEM.md that had taken half an hour to
# measure. Save and restore the bytes that were actually there.
SNAP="$(mktemp -d)"
trap 'rm -rf "$SNAP"' EXIT
save()    { for f in "$@"; do mkdir -p "$SNAP/$(dirname "$f")"; cp "$f" "$SNAP/$f"; done; }
restore() { for f in "$@"; do cp "$SNAP/$f" "$f"; done; }

# ── verify-release-versions ────────────────────────────────────────
# Derive the tree's own version so this fixture survives every bump
# (it went stale at the v0.0.31 bump when hardcoded).
CUR="v$(grep -m1 '^version = ' crates/noyalib/Cargo.toml | cut -d'"' -f2)"
save CHANGELOG.md CITATION.cff docs/ECOSYSTEM.md
# A branch carries its version from creation; the dated CHANGELOG
# heading is the one release-time step. So mid-cycle the gate may
# fail ONLY on the missing heading — verify the positive path by
# inserting a temporary heading, then restoring.
perl -0pi -e "s/^## \[Unreleased\]\n/## [Unreleased]\n\n## [$CUR] - 2099-01-01\n/m" CHANGELOG.md
if ./scripts/verify-release-versions.sh "$CUR" >/dev/null 2>&1; then ok; else bad "gate rejects the tree's own version ($CUR) even with a heading"; fi
restore CHANGELOG.md
# A version nothing agrees on: must fail.
if ./scripts/verify-release-versions.sh v9.9.9 >/dev/null 2>&1; then bad "gate accepted v9.9.9"; else ok; fi
# Stale CITATION.cff must fail (restored via git checkout).
perl -0pi -e "s/^## \[Unreleased\]\n/## [Unreleased]\n\n## [$CUR] - 2099-01-01\n/m" CHANGELOG.md
perl -pi -e 's/^version: .*/version: 0.0.1/' CITATION.cff
if ./scripts/verify-release-versions.sh "$CUR" >/dev/null 2>&1; then bad "gate missed a stale CITATION.cff"; else ok; fi
restore CITATION.cff CHANGELOG.md

# ── ECOSYSTEM.md rating-table freshness ────────────────────────────
# The weekly scorecard workflow gates the *live* score against a floor
# and never compares it to the *committed* table, which is how the table
# sat at 0.0.33 for thirty-one releases with CI green throughout.
if ./scripts/verify-release-versions.sh "$CUR" >/dev/null 2>&1; then :; fi
perl -0pi -e "s/^## \\[Unreleased\\]\\n/## [Unreleased]\\n\\n## [$CUR] - 2099-01-01\\n/m" CHANGELOG.md
perl -pi -e 's/tree \d+\.\d+\.\d+/tree 0.0.1/' docs/ECOSYSTEM.md
if ./scripts/verify-release-versions.sh "$CUR" >/dev/null 2>&1; then bad "gate missed a stale ECOSYSTEM.md rating table"; else ok; fi
restore docs/ECOSYSTEM.md CHANGELOG.md

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

# ── check-documented-commands ──────────────────────────────────────
# The gate reads `git ls-files`, so a fixture must be known to git to be
# seen at all. `git add -N` records intent-to-add without staging
# content; `git reset` on the path undoes it.
cmdfix() { printf '%b' "$1" > docs/__selftest_cmd.md; git add -N docs/__selftest_cmd.md; }
cmdclean() { git reset -q -- docs/__selftest_cmd.md 2>/dev/null; command rm -f docs/__selftest_cmd.md; }
if ./scripts/check-documented-commands.sh >/dev/null 2>&1; then ok; else bad "command gate rejects the clean tree"; fi
# A retired cargo subcommand in a fenced block.
cmdfix '# t\n\n```sh\ncargo notarealsubcommand --flag\n```\n'
if ./scripts/check-documented-commands.sh >/dev/null 2>&1; then bad "command gate missed an unknown cargo subcommand"; else ok; fi
# ...and in an inline code span, which is where the real defect lived:
# `cargo xtask pgo-build` sat in a README *paragraph*, so a gate that
# reads only fenced blocks passes while the command cannot run.
cmdfix '# t\n\nRun `cargo notarealsubcommand build` to do the thing.\n'
if ./scripts/check-documented-commands.sh >/dev/null 2>&1; then bad "command gate missed an inline-span command"; else ok; fi
# A missing make target and a missing script.
cmdfix '# t\n\n```sh\nmake __no_such_target__\n```\n'
if ./scripts/check-documented-commands.sh >/dev/null 2>&1; then bad "command gate missed a missing make target"; else ok; fi
cmdfix '# t\n\n```sh\n./scripts/__no_such_script__.sh\n```\n'
if ./scripts/check-documented-commands.sh >/dev/null 2>&1; then bad "command gate missed a missing script"; else ok; fi
# Prose in a comment must NOT be read as a command.
cmdfix '# t\n\n```sh\n# cargo will fetch the index first\ncargo build\n```\n'
if ./scripts/check-documented-commands.sh >/dev/null 2>&1; then ok; else bad "command gate read a shell comment as an instruction"; fi
cmdclean

# ── feature-powerset hosted-target contract ───────────────────────
# Integration tests, examples and benches use the documented default
# surface. The isolated library-only sweep lives in ci.yml; omitting
# this baseline makes the weekly all-target sweep compile hosted tests
# as if they were alloc-only consumers and fail before testing a single
# optional feature.
hosted_target_step=$(sed -n \
    '/name: Check hosted targets with each optional feature/,/exclude-features/p' \
    .github/workflows/feature-powerset.yml)
if grep -q -- '--each-feature --features default --all-targets' <<<"$hosted_target_step"; then
    ok
else
    bad "hosted target sweep must retain the default feature baseline"
fi

# ── every gate script is exercised above ───────────────────────────
# Without this, a new `scripts/check-*.sh` joins the release path with
# no self-test and nobody notices — which is how three of the five got
# here. A gate too expensive to self-test says so, in this list, with
# the reason; silence is not an option.
EXEMPT_READ="exercised by its own CI job against the real tree; each run builds cargo projects and takes minutes"
declare -a EXEMPT=("check-readme-examples.sh" "check-install-guides.sh" "check-crates-io-ownership.sh")
for g in scripts/check-*.sh; do
    name="$(basename "$g")"
    if grep -q "scripts/${name}" scripts/tests/run.sh; then
        continue
    fi
    skip=0
    for e in "${EXEMPT[@]}"; do [ "$name" = "$e" ] && skip=1; done
    if [ "$skip" = 1 ]; then
        ok
    else
        bad "$name has no self-test and is not listed as exempt ($EXEMPT_READ)"
    fi
done

echo "gate self-tests: $pass passed, $fail failed"
[ "$fail" = 0 ]
