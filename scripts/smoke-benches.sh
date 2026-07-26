#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Smoke-run EVERY `[[bench]]` target — execute each benchmark once via
# Criterion's `--test` mode (runs every benchmark function a single time
# without collecting measurements). This guarantees the benchmark
# harnesses actually run against the current API, not merely compile.
#
# WHY THIS EXISTS
#
# `cargo build --benches` (and the CI check job) only *compiles* the
# benches. A bench can compile yet panic on the first iteration (bad
# fixture, renamed API, changed default). Full `cargo bench` is far too
# slow for a per-PR gate, so this runs the `--test` fast path: every
# bench executes once, any panic fails the gate, no measurement noise.
# Bench targets are auto-discovered from `cargo metadata` with their
# `required-features`, so new benches are covered automatically.
#
# Run locally:   bash scripts/smoke-benches.sh
# CI wiring:     .github/workflows/ci.yml (smoke-benches job)

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

mapfile -t BENCHES < <(
  cargo metadata --format-version 1 --no-deps \
    | jq -r '
        .packages[].targets[]
        | select(.kind[] == "bench")
        | "\(.name)\t\((.["required-features"] // []) | join(","))"
      ' \
    | sort
)

total=${#BENCHES[@]}
[ "$total" -gt 0 ] || { echo "ERROR: no bench targets discovered." >&2; exit 1; }

echo "Smoke-running $total bench targets (Criterion --test, one iteration each)…"
pass=0
fail=0
failed=()
for row in "${BENCHES[@]}"; do
  name="${row%%$'\t'*}"
  feats="${row#*$'\t'}"
  args=(bench --bench "$name" --locked)
  [ -n "$feats" ] && args+=(--features "$feats")
  args+=(-- --test)
  printf '  %-24s %s' "$name" "${feats:+[$feats] }"
  if cargo "${args[@]}" >/dev/null 2>&1; then
    printf '\033[32mok\033[0m\n'; pass=$((pass + 1))
  else
    printf '\033[31mFAIL\033[0m\n'; fail=$((fail + 1)); failed+=("$name")
  fi
done

echo
echo "benches: $pass passed, $fail failed (of $total)"
if [ "$fail" -ne 0 ]; then
  printf 'FAILED: %s\n' "${failed[*]}" >&2
  exit 1
fi
echo "All $total bench harnesses ran."
