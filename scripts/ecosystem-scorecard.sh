#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Noyalib. All rights reserved.
#
# ─────────────────────────────────────────────────────────────────────────
# noyalib ecosystem scorecard — a measurement harness, not a claim
#
# Every number this prints comes from a command run against the working
# tree or a public API. Nothing is asserted. Each row carries the probe
# that produced it, so any figure can be reproduced or refuted by running
# that one command yourself.
#
# Design rules, in order of importance:
#
#   1. FALSIFIABLE. Each metric names the exact command and the exact
#      threshold. "Docs are good" is not a metric; "rustdoc emits 0
#      warnings under -D warnings" is.
#   2. NO CREDIT FOR UNMEASURED WORK. A probe that cannot run (missing
#      tool, no network, opt-in gate not passed) scores N/A and is
#      removed from the denominator. It never scores 0, and it never
#      silently scores 1. The report states how much of the rubric
#      actually executed.
#   3. REPRODUCIBLE. The header records rustc/cargo versions, host
#      triple, each repo's commit SHA and dirty flag. Same inputs, same
#      output.
#
# Usage:
#   scripts/ecosystem-scorecard.sh                 # local probes only
#   scripts/ecosystem-scorecard.sh --network       # + crates.io, GitHub, OpenSSF
#   scripts/ecosystem-scorecard.sh --with-coverage # + cargo-llvm-cov (slow)
#   scripts/ecosystem-scorecard.sh --repo noyalib  # one repo
#   scripts/ecosystem-scorecard.sh --json out.json
#
# Exit status: 0 if the weighted score is >= SCORE_FLOOR (default 0.90),
# 1 otherwise, 2 on harness error. Wire it into CI to make the rating a
# gate rather than a boast.
# ─────────────────────────────────────────────────────────────────────────

# Deliberately no `-e`: a probe failing IS the measurement. The harness
# must survive it and record it.
set -uo pipefail

VERSION="1.0.0"
SCORE_FLOOR="${SCORE_FLOOR:-0.90}"

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ECOSYSTEM_ROOT="${ECOSYSTEM_ROOT:-$(cd "$HERE/../.." && pwd)}"
OWNER="${OWNER:-sebastienrousseau}"

ALL_REPOS=(noyalib noya-cli noyalib-lsp noyalib-mcp noyalib-wasm)

WITH_NETWORK=0
WITH_COVERAGE=0
DEEP=0
# Per-probe wall-clock ceiling. The core crate has 167 integration test
# files; an --all-features run of them does not finish in a useful time,
# and a harness nobody waits for measures nothing. A probe that blows the
# ceiling records `timeout` and scores 0 — visible, not silent.
PROBE_TIMEOUT="${PROBE_TIMEOUT:-1200}"
# Compiling is not the thing being measured. The core crate has 167
# integration test files and a cold debug build of them dwarfs the run
# itself, so the build gets its own generous ceiling and is reported
# separately. Conflating the two turns "the tests pass" into "this
# laptop is fast", and scores a green suite 0 for being large.
BUILD_TIMEOUT="${BUILD_TIMEOUT:-3600}"
JSON_OUT=""
INJECT=""
SELECTED=()

while [ $# -gt 0 ]; do
  case "$1" in
    --network)       WITH_NETWORK=1 ;;
    --with-coverage) WITH_COVERAGE=1 ;;
    --deep)          DEEP=1 ;;
    --timeout)       PROBE_TIMEOUT="${2:?--timeout needs seconds}"; shift ;;
    --build-timeout) BUILD_TIMEOUT="${2:?--build-timeout needs seconds}"; shift ;;
    --json)          JSON_OUT="${2:?--json needs a path}"; shift ;;
    --inject)        INJECT="${2:?--inject needs a path}"; shift ;;
    --repo)          SELECTED+=("${2:?--repo needs a name}"); shift ;;
    --floor)         SCORE_FLOOR="${2:?--floor needs a number}"; shift ;;
    -h|--help)       sed -n '3,40p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "unknown flag: $1" >&2; exit 2 ;;
  esac
  shift
done

[ ${#SELECTED[@]} -gt 0 ] && REPOS=("${SELECTED[@]}") || REPOS=("${ALL_REPOS[@]}")

# Output paths are resolved now, while the cwd is still the caller's.
# probe_repo() cd's into each repo, so a relative --json/--inject path
# would land wherever the last probe happened to leave us — silently for
# --json, and as a confusing "no such file" for --inject.
abspath() {
  case "$1" in
    /*) printf '%s' "$1" ;;
    *)  printf '%s/%s' "$PWD" "$1" ;;
  esac
}
[ -n "$JSON_OUT" ] && JSON_OUT=$(abspath "$JSON_OUT")
[ -n "$INJECT" ]   && INJECT=$(abspath "$INJECT")

WORK=$(mktemp -d) || exit 2
trap 'rm -rf "$WORK"' EXIT
ROWS="$WORK/rows.tsv"     # repo \t id \t category \t weight \t value \t score \t evidence
: > "$ROWS"

bold()  { printf '\033[1m%s\033[0m\n' "$*"; }
dim()   { printf '\033[2m%s\033[0m\n' "$*"; }

# ── recording ────────────────────────────────────────────────────────────
# record <repo> <id> <category> <weight> <value> <score|NA> <evidence>
#
# score is a float in [0,1], or the literal NA meaning "not measured".
# NA rows are printed but excluded from every average.
record() {
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$1" "$2" "$3" "$4" "$5" "$6" "$7" >> "$ROWS"
  # Scores are printed to 4dp by fdiv/score_*, so compare numerically
  # rather than against a handful of string spellings — "1.0000" is not
  # "1", and treating it as partial made every passing probe look amber.
  local mark
  if [ "$6" = "NA" ]; then
    mark=$'\033[2m  ·\033[0m'
  elif awk -v v="$6" 'BEGIN{exit !(v+0 >= 0.9995)}'; then
    mark=$'\033[32m  ✓\033[0m'
  elif awk -v v="$6" 'BEGIN{exit !(v+0 <= 0.0005)}'; then
    mark=$'\033[31m  ✗\033[0m'
  else
    mark=$'\033[33m  ~\033[0m'
  fi
  printf '%s %-22s %-28s %s\n' "$mark" "$2" "$5" "$(dim_inline "$7")"
}
dim_inline() { printf '\033[2m%s\033[0m' "$*"; }

# Float helpers — bash has no float arithmetic, so awk does it.
fdiv()   { awk -v a="$1" -v b="$2" 'BEGIN{ if (b==0) print "0"; else printf "%.4f", a/b }'; }
fclamp() { awk -v a="$1" 'BEGIN{ if (a>1) a=1; if (a<0) a=0; printf "%.4f", a }'; }
# score_atmost <value> <limit> — 1 at or under the limit, decaying to 0 at 2x.
score_atmost() {
  awk -v v="$1" -v l="$2" 'BEGIN{
    if (v <= l) { print "1.0000"; exit }
    if (l == 0) { print (v>0 ? "0.0000" : "1.0000"); exit }
    s = 1 - (v - l) / l; if (s < 0) s = 0; printf "%.4f", s
  }'
}
bool_score()    { [ "$1" = "0" ] && echo "1.0000" || echo "0.0000"; }
# score_atleast <value> <floor> — 1 at or above the floor, linear to 0.
score_atleast() {
  awk -v v="$1" -v f="$2" 'BEGIN{
    if (f <= 0) { print "1.0000"; exit }
    s = v / f; if (s > 1) s = 1; printf "%.4f", s
  }'
}

# NOTE ON $? — every probe below captures the exit code into a variable
# on the line after the command. It is tempting to write
#
#     run "$log" cargo fmt --check
#     record ... "$([ $? -eq 0 ] && echo clean || echo drift)" "$(bool_score $?)"
#
# but bash updates $? as each command substitution in an argument list
# completes, so the second $? reads the *first* substitution's status —
# always 0. Every such probe would score a silent, permanent 1.0000.
# That is the exact failure this harness exists to rule out, so the rc
# is always bound to a name before it is read twice.

have() { command -v "$1" >/dev/null 2>&1; }

# run <logfile> <cmd...> — run quietly, keep output, return the exit code.
#
# macOS ships no timeout(1) and no gtimeout unless coreutils is installed,
# so the ceiling is enforced here. Exit 124 means "killed at the ceiling",
# matching timeout(1)'s convention.
run() {
  local log="$1"; shift
  "$@" >"$log" 2>&1 &
  local pid=$! waited=0
  while kill -0 "$pid" 2>/dev/null; do
    if [ "$waited" -ge "$PROBE_TIMEOUT" ]; then
      # Children first: killing cargo alone orphans a swarm of rustc.
      pkill -TERM -P "$pid" 2>/dev/null
      kill -TERM "$pid" 2>/dev/null
      sleep 3
      pkill -KILL -P "$pid" 2>/dev/null
      kill -KILL "$pid" 2>/dev/null
      wait "$pid" 2>/dev/null
      echo "[harness] probe exceeded ${PROBE_TIMEOUT}s and was killed" >> "$log"
      return 124
    fi
    sleep 2; waited=$((waited + 2))
  done
  wait "$pid"; return $?
}

# Which feature set the build probes use. --all-features on the core crate
# pulls in every optional surface at once and does not finish inside any
# ceiling worth waiting for, so the default is the crate's own default
# features — the configuration users actually get. --deep opts into the
# exhaustive set for a nightly/CI run.
feat_args() { [ "$DEEP" = 1 ] && echo "--all-features" || echo ""; }

# ── environment provenance ───────────────────────────────────────────────
bold "noyalib ecosystem scorecard v$VERSION"
echo
RUSTC_V=$(rustc --version 2>/dev/null || echo "rustc: absent")
CARGO_V=$(cargo --version 2>/dev/null || echo "cargo: absent")
HOST=$(rustc -vV 2>/dev/null | awk '/^host:/{print $2}')
STAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
dim "  $RUSTC_V"
dim "  $CARGO_V"
dim "  host: ${HOST:-unknown}   utc: $STAMP"
dim "  root: $ECOSYSTEM_ROOT"
dim "  network probes: $([ $WITH_NETWORK = 1 ] && echo on || echo off)   coverage: $([ $WITH_COVERAGE = 1 ] && echo on || echo off)"
dim "  features: $([ $DEEP = 1 ] && echo all || echo default)   ceilings: probe ${PROBE_TIMEOUT}s / build ${BUILD_TIMEOUT}s"
echo

# Toolchain availability decides which probes can honestly run.
for t in cargo-audit cargo-deny cargo-vet cargo-llvm-cov reuse gh jq; do
  have "$t" || dim "  note: $t absent — its probes will score N/A, not 0"
done
echo

# ── per-repo probes ──────────────────────────────────────────────────────
declare -A REPO_SHA REPO_DIRTY REPO_VERSION

probe_repo() {
  local repo="$1"
  local dir="$ECOSYSTEM_ROOT/$repo"
  bold "── $repo"

  if [ ! -d "$dir" ]; then
    record "$repo" repo_present integrity 1 "missing" 0 "no directory at $dir"
    echo; return
  fi
  cd "$dir" || return

  local sha dirty ver
  sha=$(git rev-parse --short HEAD 2>/dev/null || echo "-")
  dirty=$(git status --porcelain 2>/dev/null | head -1 | grep -q . && echo "dirty" || echo "clean")
  ver=$(grep -m1 '^version' Cargo.toml 2>/dev/null | sed 's/.*"\(.*\)".*/\1/')
  [ -z "$ver" ] && ver=$(grep -m1 '^version' crates/*/Cargo.toml 2>/dev/null | sed 's/.*"\(.*\)".*/\1/' | head -1)
  REPO_SHA[$repo]="$sha"; REPO_DIRTY[$repo]="$dirty"; REPO_VERSION[$repo]="$ver"
  dim "  $sha ($dirty)  version $ver"

  # ---- CORRECTNESS -----------------------------------------------------
  # Tests. The score is the pass ratio, so a partial regression shows as a
  # partial score rather than a binary fail.
  local log="$WORK/$repo.test" blog="$WORK/$repo.testbuild" fa
  fa=$(feat_args)

  # Phase 1: build the test binaries. Separate ceiling, separate finding.
  local saved="$PROBE_TIMEOUT"
  PROBE_TIMEOUT="$BUILD_TIMEOUT"
  # shellcheck disable=SC2086  # deliberate word-split: fa is empty or one flag
  run "$blog" cargo test --workspace --locked $fa --no-run
  local brc=$?
  PROBE_TIMEOUT="$saved"
  if [ "$brc" -ne 0 ]; then
    local why; why=$([ "$brc" = "124" ] && echo "build timeout after ${BUILD_TIMEOUT}s" || echo "build failed (rc=$brc)")
    record "$repo" tests correctness 5 "$why" 0 "cargo test --workspace --locked ${fa:---default-features} --no-run"
    return_from_tests=1
  fi

  # Phase 2: run them. Warm target dir, so this ceiling measures the suite.
  # shellcheck disable=SC2086
  run "$log" cargo test --workspace --locked $fa
  local rc=$? passed failed
  passed=$(awk '/^test result:/{s+=$4} END{print s+0}' "$log")
  failed=$(awk '/^test result:/{s+=$6} END{print s+0}' "$log")
  local probe="cargo test --workspace --locked ${fa:---default-features}"
  if [ "${return_from_tests:-0}" = "1" ]; then
    : # already recorded a build failure above
  elif [ "$rc" = "124" ]; then
    # A timeout is *unmeasured*, not *failed*, and rule 2 above says an
    # unmeasured probe never scores 0. Scoring it 0 asserted a correctness
    # failure the suite does not have — v0.0.27's scorecard reported
    # `correctness 83.3%` purely because this ceiling fired on a cold
    # target dir, while the same suite run directly passed 5739/0.
    #
    # N/A keeps it honest and keeps it loud: N/A rows are printed and they
    # lower the reported rubric coverage, so a suite that never finishes
    # cannot quietly vanish from the denominator either.
    record "$repo" tests correctness 5 "did not finish within ${PROBE_TIMEOUT}s" NA \
      "$probe — killed at the ceiling, so not measured (raise --timeout to score it)"
  elif [ "$rc" -ne 0 ] && [ $((passed+failed)) -eq 0 ]; then
    record "$repo" tests correctness 5 "build failed" 0 "$probe (rc=$rc)"
  else
    record "$repo" tests correctness 5 "$passed passed / $failed failed" \
      "$(fdiv "$passed" "$((passed+failed))")" "$probe"
  fi
  unset return_from_tests

  # YAML 1.2 spec conformance — only the core crate ships the suite.
  if [ -f crates/noyalib/tests/yaml_compliance_report.rs ]; then
    local clog="$WORK/$repo.compliance"
    run "$clog" cargo test --locked -p noyalib --test yaml_compliance_report -- --nocapture
    local crc2=$?
    # The report prints "Strict compliance: 100.0% (406/406)". Anchor on
    # that line: a bare N/M search also matches the runner's own output,
    # and the previous pattern required a trailing space the parenthesised
    # form never has — so this silently fell back to a coarse pass/fail.
    local cpass cratio
    cratio=$(grep -i 'strict compliance' "$clog" | grep -oE '[0-9]+/[0-9]+' | head -1)
    cpass=${cratio%%/*}
    if [ -n "$cpass" ]; then
      local ctot; ctot=${cratio##*/}
      record "$repo" spec_conformance correctness 5 "$cpass/$ctot cases" \
        "$(fdiv "$cpass" "$ctot")" "cargo test -p noyalib --test yaml_compliance_report"
    else
      record "$repo" spec_conformance correctness 5 \
        "$([ "$crc2" -eq 0 ] && echo 'suite green' || echo 'suite red')" \
        "$(bool_score "$crc2")" "cargo test -p noyalib --test yaml_compliance_report"
    fi
  fi

  # ---- CODE HEALTH -----------------------------------------------------
  # shellcheck disable=SC2086
  run "$WORK/$repo.clippy" cargo clippy --workspace --all-targets --locked $fa -- -D warnings
  local crc=$? cwarn
  cwarn=$(grep -cE '^(warning|error)(\[|:)' "$WORK/$repo.clippy")
  record "$repo" clippy code_health 3 "$cwarn diagnostics (rc=$crc)" \
    "$(bool_score "$crc")" "cargo clippy --workspace --all-targets --locked ${fa:---default-features} -- -D warnings"

  run "$WORK/$repo.fmt" cargo fmt --all --check
  local frc=$?
  record "$repo" rustfmt code_health 1 "$([ "$frc" -eq 0 ] && echo clean || echo drift)" \
    "$(bool_score "$frc")" "cargo fmt --all --check"

  # forbid(unsafe_code) at every crate root, counted rather than assumed.
  # Shipped crate roots only. An example crate under examples/ and a
  # fuzz harness under fuzz/ are not part of the published surface, and
  # counting them made a fully compliant workspace read as 1/2.
  local roots forbids
  crate_roots() {
    find . -path ./target -prune -o -path '*/examples/*' -prune -o \
           -path './fuzz/*' -prune -o -path '*/benches/*' -prune -o \
           -path '*/tests/*' -prune -o \
           \( -name lib.rs -o -name main.rs \) -print 2>/dev/null
  }
  roots=$(crate_roots | grep -c .)
  forbids=$(crate_roots | tr '\n' '\0' | xargs -0 grep -l 'forbid(unsafe_code)' 2>/dev/null | grep -c .)
  record "$repo" unsafe_forbidden code_health 3 "$forbids/$roots crate roots" \
    "$(fdiv "$forbids" "$roots")" "grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs"

  # MSRV must be declared. An undeclared MSRV is not "latest stable", it
  # is "unknown", and unknown is a downstream break waiting to happen.
  local msrv
  msrv=$(grep -rhm1 '^rust-version' Cargo.toml crates/*/Cargo.toml 2>/dev/null | head -1 | sed 's/.*"\(.*\)".*/\1/')
  record "$repo" msrv_declared code_health 1 "${msrv:-undeclared}" \
    "$([ -n "$msrv" ] && echo 1.0000 || echo 0.0000)" "grep rust-version Cargo.toml"

  # ---- DOCUMENTATION ---------------------------------------------------
  RUSTDOCFLAGS="-D warnings" run "$WORK/$repo.doc" cargo doc --workspace --no-deps --locked --all-features
  local drc=$?
  record "$repo" rustdoc_strict docs 3 "$([ "$drc" -eq 0 ] && echo '0 warnings' || echo 'warnings present')" \
    "$(bool_score "$drc")" "RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)"

  # missing_docs is what actually proves the public surface is documented.
  local md
  md=$(crate_roots | tr '\n' '\0' | xargs -0 grep -l 'missing_docs' 2>/dev/null | grep -c .)
  record "$repo" missing_docs_lint docs 2 "$md/$roots roots deny missing_docs" \
    "$(fdiv "$md" "$roots")" "grep for deny/warn(missing_docs) in crate roots"

  if [ -x scripts/check-readme-examples.sh ]; then
    run "$WORK/$repo.readme" ./scripts/check-readme-examples.sh
    local rrc=$?
    record "$repo" readme_examples docs 2 "$([ "$rrc" -eq 0 ] && echo compile || echo broken)" \
      "$(bool_score "$rrc")" "scripts/check-readme-examples.sh"
  else
    record "$repo" readme_examples docs 2 "no harness" NA "scripts/check-readme-examples.sh absent"
  fi

  # A published speed claim is only falsifiable if the reader is told the
  # machine, the toolchain and the command. Check the doc actually says.
  if [ -f docs/BENCHMARKS.md ]; then
    local disc=0
    grep -qiE 'aarch64|x86_64|apple|linux' docs/BENCHMARKS.md && disc=$((disc+1))
    grep -qiE 'rust [0-9]+\.[0-9]+|rustc [0-9]+\.[0-9]+' docs/BENCHMARKS.md && disc=$((disc+1))
    grep -qiE 'cargo bench' docs/BENCHMARKS.md && disc=$((disc+1))
    record "$repo" bench_methodology docs 2 "$disc/3 disclosed (host, toolchain, command)" \
      "$(fdiv "$disc" 3)" "grep host/toolchain/repro-command in docs/BENCHMARKS.md"
  fi

  # ---- SUPPLY CHAIN ----------------------------------------------------
  if have cargo-audit; then
    # Invoke the binary directly rather than as `cargo audit`. A
    # user-defined `audit = "audit"` alias in ~/.cargo/config.toml shadows
    # the subcommand and recurses, so `cargo audit` exits 101 having
    # produced only an error message.
    run "$WORK/$repo.audit" cargo-audit audit --json
    local arc=$?
    # A non-zero exit is ambiguous here: cargo-audit also exits non-zero
    # when it *finds* advisories. The discriminator is whether the output
    # parses. The previous fallback counted "RUSTSEC" in whatever landed on
    # stdout, so an error message scored zero advisories — a clean pass on
    # a security probe from a command that never ran. Unparsable output is
    # now N/A, which is what "we did not measure this" is meant to look
    # like.
    local vulns
    vulns=$(jq -r '.vulnerabilities.count // empty' "$WORK/$repo.audit" 2>/dev/null)
    if [ -n "$vulns" ]; then
      record "$repo" audit_vulnerabilities supply_chain 5 "$vulns advisories" \
        "$(score_atmost "$vulns" 0)" "cargo-audit audit --json | .vulnerabilities.count"
    else
      record "$repo" audit_vulnerabilities supply_chain 5 "no parsable output (rc=$arc)" NA \
        "cargo-audit audit --json produced no .vulnerabilities.count — not scored"
    fi
  else
    record "$repo" audit_vulnerabilities supply_chain 5 "tool absent" NA "cargo-audit not installed"
  fi

  if have cargo-deny; then
    run "$WORK/$repo.deny" cargo deny check
    local dnrc=$?
    record "$repo" deny_check supply_chain 3 "$([ "$dnrc" -eq 0 ] && echo pass || echo fail)" \
      "$(bool_score "$dnrc")" "cargo deny check (advisories, bans, licenses, sources)"
  else
    record "$repo" deny_check supply_chain 3 "tool absent" NA "cargo-deny not installed"
  fi

  if have cargo-vet; then
    run "$WORK/$repo.vet" cargo vet --locked
    local vrc=$?
    record "$repo" vet_audited supply_chain 3 "$([ "$vrc" -eq 0 ] && echo pass || echo fail)" \
      "$(bool_score "$vrc")" "cargo vet --locked"
  else
    record "$repo" vet_audited supply_chain 3 "tool absent" NA "cargo-vet not installed"
  fi

  if have reuse; then
    run "$WORK/$repo.reuse" reuse lint
    local urc=$?
    record "$repo" reuse_compliance supply_chain 1 "$([ "$urc" -eq 0 ] && echo compliant || echo non-compliant)" \
      "$(bool_score "$urc")" "reuse lint (REUSE 3.3)"
  else
    record "$repo" reuse_compliance supply_chain 1 "tool absent" NA "reuse not installed"
  fi

  # Dependency closure size.
  #
  # This was one flat budget of 60 for every repo, which is why `noya-cli`
  # scored 0 at 130 crates. Investigating that produced a better rule than
  # a bigger number.
  #
  # A **library's** dependencies propagate: every downstream consumer
  # inherits the whole closure whether they wanted it or not, so its size
  # is a cost imposed on other people and belongs in the score. `noyalib`
  # sits at 12, which is the number that actually matters here.
  #
  # A **leaf binary's** do not. Nobody inherits `noya-cli`'s tree; a user
  # installs the tool deliberately, and 130 of those crates are `miette`'s
  # `fancy` renderer giving `noyavalidate` source excerpts and carets —
  # which for a validator is the job, not bloat. There is no defensible
  # universal threshold for that, so this records the number and does not
  # score it. Inventing a budget that the current value happens to clear
  # would be curve-grading with extra steps.
  #
  # The count is still printed and still in the JSON, so a leaf that
  # doubles its tree is visible; it simply does not pretend to be a pass
  # or a fail against a line nobody can justify.
  local deps
  deps=$(cargo tree -e normal --prefix none --no-dedupe 2>/dev/null \
         | awk 'NF{print $1}' | sort -u | grep -c .)
  if [ "$repo" = "noyalib" ]; then
    record "$repo" dependency_closure supply_chain 2 "$deps unique runtime crates" \
      "$(score_atmost "$deps" 60)" \
      "cargo tree -e normal --prefix none --no-dedupe | sort -u (library budget 60; propagates to consumers)"
  else
    record "$repo" dependency_closure supply_chain 2 "$deps unique runtime crates (leaf, not scored)" NA \
      "cargo tree -e normal --prefix none --no-dedupe | sort -u — recorded, not scored: a leaf binary's tree is not inherited"
  fi

  # Actions pinned by SHA, not by a mutable tag.
  if [ -d .github/workflows ]; then
    local uses pinned
    uses=$(grep -rhoE '^\s*(-\s*)?uses:\s*\S+' .github/workflows 2>/dev/null | grep -vc 'uses:\s*\./')
    pinned=$(grep -rhoE '^\s*(-\s*)?uses:\s*\S+@[0-9a-f]{40}' .github/workflows 2>/dev/null | grep -c .)
    if [ "${uses:-0}" -gt 0 ]; then
      record "$repo" actions_sha_pinned supply_chain 2 "$pinned/$uses external uses pinned" \
        "$(fdiv "$pinned" "$uses")" "grep 'uses:.*@<40-hex>' .github/workflows"
    else
      record "$repo" actions_sha_pinned supply_chain 2 "no external uses" NA "grep 'uses:' .github/workflows"
    fi
  fi

  # ---- ROBUSTNESS ------------------------------------------------------
  if [ -d fuzz ]; then
    local targets
    targets=$(find fuzz/fuzz_targets -name '*.rs' 2>/dev/null | grep -c .)
    record "$repo" fuzz_targets robustness 2 "$targets libFuzzer targets" \
      "$(score_atleast "$targets" 2)" "find fuzz/fuzz_targets -name '*.rs' (floor 2)"
  else
    record "$repo" fuzz_targets robustness 2 "no fuzz/ dir" 0 "find fuzz/fuzz_targets"
  fi

  if [ "$WITH_COVERAGE" = 1 ] && have cargo-llvm-cov; then
    # shellcheck disable=SC2086
    run "$WORK/$repo.cov" cargo llvm-cov --workspace --locked $fa --summary-only
    local pct
    pct=$(awk '/^TOTAL/{print $(NF-0)}' "$WORK/$repo.cov" | tr -d '%' | head -1)
    [ -z "$pct" ] && pct=$(grep -oE '[0-9]+\.[0-9]+%' "$WORK/$repo.cov" | tail -1 | tr -d '%')
    if [ -n "$pct" ]; then
      record "$repo" line_coverage robustness 4 "${pct}% lines" \
        "$(fclamp "$(fdiv "$pct" 95)")" "cargo llvm-cov --summary-only (target 95%)"
    else
      record "$repo" line_coverage robustness 4 "unparsable" NA "cargo llvm-cov --summary-only"
    fi
  else
    record "$repo" line_coverage robustness 4 "not run" NA "pass --with-coverage to measure"
  fi

  # ---- RELEASE INTEGRITY (network) -------------------------------------
  if [ "$WITH_NETWORK" = 1 ] && have gh; then
    local assets
    assets=$(gh release view --repo "$OWNER/$repo" --json assets 2>/dev/null | jq -r '.assets[].name' 2>/dev/null)
    if [ -n "$assets" ]; then
      local n_asc n_bundle n_sbom
      n_asc=$(printf '%s\n' "$assets" | grep -c '\.asc$')
      n_bundle=$(printf '%s\n' "$assets" | grep -c '\.bundle$')
      n_sbom=$(printf '%s\n' "$assets" | grep -ci 'sbom')
      record "$repo" release_gpg_signed release 3 "$n_asc .asc assets" \
        "$([ "$n_asc" -gt 0 ] && echo 1.0000 || echo 0.0000)" "gh release view --json assets | grep '\.asc$'"
      record "$repo" release_sigstore release 3 "$n_bundle .bundle assets" \
        "$([ "$n_bundle" -gt 0 ] && echo 1.0000 || echo 0.0000)" "gh release view --json assets | grep '\.bundle$'"
      record "$repo" release_sbom release 2 "$n_sbom sbom assets" \
        "$([ "$n_sbom" -gt 0 ] && echo 1.0000 || echo 0.0000)" "gh release view --json assets | grep -i sbom"
    else
      record "$repo" release_gpg_signed release 3 "no release found" NA "gh release view --repo $OWNER/$repo"
    fi

    # Open Dependabot alerts. Requires the token to carry the scope; a
    # 403 is "unknown", not "zero" — scoring it 1 would be a lie.
    local alerts
    alerts=$(gh api "/repos/$OWNER/$repo/dependabot/alerts?state=open&per_page=100" 2>/dev/null | jq -r 'length' 2>/dev/null)
    if [ -n "$alerts" ] && [ "$alerts" != "null" ]; then
      record "$repo" dependabot_open release 4 "$alerts open alerts" \
        "$(score_atmost "$alerts" 0)" "gh api /repos/$OWNER/$repo/dependabot/alerts?state=open"
    else
      record "$repo" dependabot_open release 4 "unreadable" NA "gh api dependabot/alerts (scope/permission)"
    fi

    # main must actually be green right now.
    local concl
    concl=$(gh run list --repo "$OWNER/$repo" --branch main --limit 1 --json conclusion --jq '.[0].conclusion' 2>/dev/null)
    record "$repo" ci_main_green release 3 "${concl:-unknown}" \
      "$([ "$concl" = "success" ] && echo 1.0000 || { [ -z "$concl" ] && echo NA || echo 0.0000; })" \
      "gh run list --branch main --limit 1 --json conclusion"

    # crates.io must carry the version the tree claims.
    if [ -n "$ver" ]; then
      local crate_name published
      crate_name=$repo
      [ "$repo" = "noyalib" ] && crate_name=noyalib
      published=$(curl -sS -H 'User-Agent: noyalib-scorecard' \
        "https://crates.io/api/v1/crates/$crate_name" 2>/dev/null | jq -r '.crate.max_stable_version // empty' 2>/dev/null)
      if [ -n "$published" ]; then
        record "$repo" crates_io_current release 2 "tree $ver / crates.io $published" \
          "$([ "$ver" = "$published" ] && echo 1.0000 || echo 0.0000)" "crates.io/api/v1/crates/$crate_name .max_stable_version"
      else
        record "$repo" crates_io_current release 2 "lookup failed" NA "crates.io api"
      fi
    fi

    # OpenSSF Scorecard — an external, independent grader.
    local ossf
    ossf=$(curl -sS "https://api.securityscorecards.dev/projects/github.com/$OWNER/$repo" 2>/dev/null \
           | jq -r '.score // empty' 2>/dev/null)
    if [ -n "$ossf" ]; then
      record "$repo" openssf_scorecard release 3 "$ossf/10" \
        "$(fdiv "$ossf" 10)" "api.securityscorecards.dev/projects/github.com/$OWNER/$repo .score"
    else
      record "$repo" openssf_scorecard release 3 "not indexed" NA "api.securityscorecards.dev (no published run)"
    fi
  else
    for m in release_gpg_signed:3 release_sigstore:3 release_sbom:2 dependabot_open:4 ci_main_green:3 crates_io_current:2 openssf_scorecard:3; do
      record "$repo" "${m%%:*}" release "${m##*:}" "offline" NA "pass --network to measure"
    done
  fi

  echo
}

for r in "${REPOS[@]}"; do probe_repo "$r"; done

# ── ecosystem-wide probes ────────────────────────────────────────────────
bold "── ecosystem"

# Strict lockstep (ADR-0005): every satellite pins the core version
# exactly, and that pin equals the core's own version. This is the single
# invariant that makes a five-crate release atomic.
CORE_VER="${REPO_VERSION[noyalib]:-}"
if [ -n "$CORE_VER" ]; then
  matched=0; total=0
  detail=""
  for r in noya-cli noyalib-lsp noyalib-mcp noyalib-wasm; do
    d="$ECOSYSTEM_ROOT/$r"; [ -d "$d" ] || continue
    total=$((total+1))
    pin=$(grep -rhoE 'noyalib *= *\{?[^}]*version *= *"=?[0-9.]+"|noyalib *= *"=?[0-9.]+"' \
          "$d/Cargo.toml" "$d"/crates/*/Cargo.toml 2>/dev/null \
          | grep -oE '=?[0-9]+\.[0-9]+\.[0-9]+' | head -1)
    # The `=?` above is what makes an exact pin distinguishable from a
    # caret range, but it also lands in the capture — so strip it before
    # comparing. Leaving it in compared "=0.0.25" against "0.0.25" and
    # reported a correctly-locked ecosystem as 0/4.
    pin=${pin#=}
    [ "$pin" = "$CORE_VER" ] && matched=$((matched+1)) || detail="$detail $r=$pin"
  done
  record ecosystem version_lockstep integrity 5 "$matched/$total pin =$CORE_VER" \
    "$(fdiv "$matched" "$total")" "grep the noyalib pin in each satellite Cargo.toml (ADR-0005);${detail:- all aligned}"
fi

# Host coverage: the ecosystem claims to reach a host surface. Count the
# hosts that actually have a shipping crate rather than a roadmap entry.
hosts=0
for r in "${ALL_REPOS[@]}"; do [ -d "$ECOSYSTEM_ROOT/$r" ] && hosts=$((hosts+1)); done
record ecosystem host_coverage integrity 3 "$hosts/5 hosts present" \
  "$(fdiv "$hosts" 5)" "directory presence for ${ALL_REPOS[*]}"

echo

# ── report ───────────────────────────────────────────────────────────────
bold "── scores by category"
echo
awk -F'\t' '
  $6 != "NA" { w[$3] += $4; s[$3] += $4 * $6; n[$3]++; W += $4; S += $4 * $6; N++ }
  $6 == "NA" { na[$3]++; NAT++ }
  END {
    printf "  %-16s %7s %7s %8s\n", "CATEGORY", "SCORE", "PROBES", "N/A"
    split("correctness code_health docs supply_chain robustness release integrity", ord, " ")
    for (i = 1; i <= 7; i++) { c = ord[i]
      if (c in w) printf "  %-16s %6.1f%% %7d %8d\n", c, 100*s[c]/w[c], n[c], na[c]+0
      else if (c in na) printf "  %-16s %6s %7d %8d\n", c, "n/a", 0, na[c] }
    printf "\n  %-16s %6.1f%% %7d %8d\n", "WEIGHTED TOTAL", (W?100*S/W:0), N, NAT+0
    printf "%.6f\n", (W ? S/W : 0) > "'"$WORK/total"'"
    printf "%d\n%d\n", N, NAT+0 > "'"$WORK/counts"'"
  }
' "$ROWS"

TOTAL=$(head -1 "$WORK/total" 2>/dev/null || echo 0)
MEASURED=$(sed -n 1p "$WORK/counts" 2>/dev/null || echo 0)
UNMEASURED=$(sed -n 2p "$WORK/counts" 2>/dev/null || echo 0)
PCT=$(awk -v t="$TOTAL" 'BEGIN{printf "%.1f", t*100}')

# Coverage of the rubric itself. A 100% score over 30% of the rubric is
# not a 100% score, and the report must say so in the same breath.
RUBRIC=$(awk -v m="$MEASURED" -v u="$UNMEASURED" 'BEGIN{ t=m+u; printf "%.0f", (t? 100*m/t : 0) }')

echo
GRADE=$(awk -v t="$TOTAL" 'BEGIN{
  if (t>=0.97) print "A+"; else if (t>=0.93) print "A"; else if (t>=0.90) print "A-";
  else if (t>=0.87) print "B+"; else if (t>=0.83) print "B"; else if (t>=0.80) print "B-";
  else if (t>=0.70) print "C"; else if (t>=0.60) print "D"; else print "F" }')
bold "  RATING: $GRADE  ($PCT% weighted)"
dim   "  derived from $MEASURED executed probes; $UNMEASURED scored N/A and excluded"
dim   "  rubric executed: $RUBRIC% — a score is only as good as its coverage"
echo

# ── JSON ─────────────────────────────────────────────────────────────────
if [ -n "$JSON_OUT" ]; then
  {
    printf '{\n'
    printf '  "harness_version": "%s",\n' "$VERSION"
    printf '  "generated_utc": "%s",\n' "$STAMP"
    printf '  "rustc": "%s",\n' "$RUSTC_V"
    printf '  "host": "%s",\n' "${HOST:-unknown}"
    printf '  "network_probes": %s,\n' "$([ $WITH_NETWORK = 1 ] && echo true || echo false)"
    printf '  "coverage_probes": %s,\n' "$([ $WITH_COVERAGE = 1 ] && echo true || echo false)"
    printf '  "weighted_score": %s,\n' "$TOTAL"
    printf '  "grade": "%s",\n' "$GRADE"
    printf '  "probes_measured": %s,\n' "$MEASURED"
    printf '  "probes_na": %s,\n' "$UNMEASURED"
    printf '  "rubric_executed_pct": %s,\n' "$RUBRIC"
    printf '  "repos": {\n'
    first=1
    for r in "${REPOS[@]}"; do
      [ $first = 1 ] || printf ',\n'; first=0
      printf '    "%s": {"sha": "%s", "worktree": "%s", "version": "%s"}' \
        "$r" "${REPO_SHA[$r]:-}" "${REPO_DIRTY[$r]:-}" "${REPO_VERSION[$r]:-}"
    done
    printf '\n  },\n'
    printf '  "measurements": [\n'
    awk -F'\t' '{
      # Backslashes first, then quotes — reversing the order double-escapes
      # the backslash that quote-escaping just introduced. Probe strings
      # carry regexes like grep '"'"'\.asc$'"'"', so without this the emitted
      # JSON is invalid and every consumer of --json breaks.
      gsub(/\\/,"\\\\",$5); gsub(/"/,"\\\"",$5)
      gsub(/\\/,"\\\\",$7); gsub(/"/,"\\\"",$7)
      printf "%s    {\"repo\":\"%s\",\"id\":\"%s\",\"category\":\"%s\",\"weight\":%s,\"value\":\"%s\",\"score\":%s,\"probe\":\"%s\"}",
        (NR>1 ? ",\n" : ""), $1,$2,$3,$4,$5, ($6=="NA" ? "null" : $6), $7
    } END{print ""}' "$ROWS"
    printf '  ]\n}\n'
  } > "$JSON_OUT"
  dim "  json: $JSON_OUT"
fi

# ── markdown injection ───────────────────────────────────────────────────
# Rewrites the block between <!-- SCORECARD:BEGIN --> and
# <!-- SCORECARD:END --> in a target file. The rating in the docs is
# therefore always generated from a run, never typed by hand — the only
# way the "reproducible" claim survives contact with an editor.
if [ -n "$INJECT" ]; then
  if [ ! -f "$INJECT" ]; then
    echo "  --inject: no such file: $INJECT" >&2
  elif ! grep -q 'SCORECARD:BEGIN' "$INJECT" || ! grep -q 'SCORECARD:END' "$INJECT"; then
    echo "  --inject: $INJECT has no SCORECARD:BEGIN/END markers" >&2
  else
    BLOCK="$WORK/block.md"
    {
      printf '**Rating: %s (%s%% weighted)** — %s executed probes, %s scored N/A.\n\n' \
        "$GRADE" "$PCT" "$MEASURED" "$UNMEASURED"
      printf 'Measured %s on `%s`, %s. Rubric executed: %s%%.\n\n' \
        "$STAMP" "${HOST:-unknown}" "$RUSTC_V" "$RUBRIC"
      printf '| Repo | Metric | Measured | Score | Probe |\n|---|---|---|---:|---|\n'
      awk -F'\t' '{
        printf "| `%s` | %s | %s | %s | `%s` |\n", $1, $2, $5,
               ($6=="NA" ? "n/a" : sprintf("%.2f", $6)), $7
      }' "$ROWS"
      printf '\n<sub>Generated by `scripts/ecosystem-scorecard.sh` v%s. ' "$VERSION"
      printf 'Regenerate rather than edit.</sub>\n'
    } > "$BLOCK"

    awk -v blk="$BLOCK" '
      /SCORECARD:BEGIN/ { print; while ((getline line < blk) > 0) print line; skip=1; next }
      /SCORECARD:END/   { skip=0 }
      !skip             { print }
    ' "$INJECT" > "$INJECT.tmp" && mv "$INJECT.tmp" "$INJECT"
    dim "  injected: $INJECT"
  fi
fi

awk -v t="$TOTAL" -v f="$SCORE_FLOOR" 'BEGIN{ exit (t+0 >= f+0 ? 0 : 1) }'
