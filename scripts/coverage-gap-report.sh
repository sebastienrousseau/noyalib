#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# scripts/coverage-gap-report.sh — print a punchlist of files
# below the workspace coverage threshold.
#
# Used during the Phase 1 coverage-hardening work (see PLAN.md):
# the script runs `cargo +nightly llvm-cov` with a minimal
# --summary-only output, parses the per-file rows, and prints
# every file whose region / line / function coverage is under the
# given threshold (default 98 %).
#
# Usage:
#   ./scripts/coverage-gap-report.sh             # default 98 % threshold
#   ./scripts/coverage-gap-report.sh 95          # custom threshold
#
# Output is a TSV-shaped block followed by a one-line summary:
#
#   file                          regions  lines  functions
#   src/streaming.rs              88.59    87.35   94.32
#   src/parser/loader.rs          92.34    94.73   91.67
#   ...
#   12 files below 98 %.
#
# The TSV output is grep-friendly so a follow-up script can pipe
# it into a TODO list or a coverage dashboard.

set -euo pipefail
IFS=$'\n\t'

THRESHOLD="${1:-98}"

# Optional second arg: a regex of files to ignore. Defaults to empty
# (measure everything). `cargo-llvm-cov` >= 0.8.7 rejects an empty
# `--ignore-filename-regex`, so — mirroring
# .github/workflows/shared-coverage.yml — we only pass the flag when
# the value is non-empty. Passing '' unconditionally makes the tool
# exit non-zero ("empty string is not allowed").
IGNORE_REGEX="${2:-}"

IGNORE_ARGS=()
if [ -n "${IGNORE_REGEX}" ]; then
    IGNORE_ARGS+=(--ignore-filename-regex "${IGNORE_REGEX}")
fi

echo "→ Running cargo +nightly llvm-cov (this takes ~2 min)..."

# Capture the summary table from *stdout* only; build / doctest
# progress goes to stderr, which we tee to a diagnostic log. Merging
# the two (an earlier `2>&1`) polluted the table with `Compiling …`
# and `test … ok` lines, which the awk row filter below then
# miscounted as under-threshold files.
SUMMARY="$(NOYALIB_COVERAGE=1 cargo +nightly llvm-cov \
    --workspace --all-features --no-fail-fast \
    ${IGNORE_ARGS[@]+"${IGNORE_ARGS[@]}"} \
    --summary-only 2> >(tee /tmp/noyalib-coverage.log >&2))"

echo
echo "→ Parsing rows below ${THRESHOLD} %..."
echo

# llvm-cov's summary table columns (current layout):
#   Filename Regions Missed Cover Functions Missed Cover Lines Missed Cover Branches Missed Cover
# The numeric coverage cells are 6, 9, 12 (region, function, line — yes, that order).
#
# We build awk that prints any row whose region, line, OR function
# percentage falls below the threshold. Header rules and the TOTAL
# row are skipped. Numbers are normalized to floats for comparison.

printf 'file\tregions\tlines\tfunctions\n'
echo "$SUMMARY" | awk -v T="${THRESHOLD}" '
    /^-{10,}/        { next }
    /^Filename/      { next }
    /^TOTAL/         { next }
    # A real summary row has the coverage cells as `NN.NN%`. Requiring
    # the percent-suffixed cells rejects any stray build/test line that
    # happens to survive with >= 12 fields.
    NF >= 12 && $4 ~ /%$/ && $7 ~ /%$/ && $10 ~ /%$/ {
        # Region cover (col 4) is index 4 after splitting on whitespace.
        # llvm-cov columns:
        #   1=file, 2=regs, 3=missed, 4=region%, 5=fns, 6=missed, 7=fn%,
        #   8=lines, 9=missed, 10=line%, 11=branches?, 12=missed?
        file=$1; reg=$4; fn=$7; ln=$10
        gsub("%","",reg); gsub("%","",fn); gsub("%","",ln)
        if (reg+0 < T+0 || ln+0 < T+0 || fn+0 < T+0) {
            printf "%s\t%.2f\t%.2f\t%.2f\n", file, reg, ln, fn
            count++
        }
    }
    END { printf "\n%d files below %s %%.\n", count, T > "/dev/stderr" }
'

echo
echo "→ Full summary at /tmp/noyalib-coverage.log"
