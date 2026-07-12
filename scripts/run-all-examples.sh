#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Run EVERY `[[example]]` target to completion — not just compile it.
#
# WHY THIS EXISTS
#
# CI already compiles every example (`cargo build --examples
# --all-features`), but a compiled example can still panic at run time.
# `make examples` used to run a hand-maintained subset that silently
# drifted out of date and skipped every feature-gated example. This
# script closes that hole: it auto-discovers all example targets from
# `cargo metadata`, runs each one with EXACTLY its `required-features`
# (matching the command a user would copy from the example header), and
# fails if any example exits non-zero. Auto-discovery means a newly
# added example is covered the moment it lands — nothing to update here.
#
# Run locally:   bash scripts/run-all-examples.sh
# CI wiring:     .github/workflows/ci.yml (run-examples job)

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# name<TAB>comma,separated,features   (one line per example target)
mapfile -t EXAMPLES < <(
  cargo metadata --format-version 1 --no-deps \
    | jq -r '
        .packages[].targets[]
        | select(.kind[] == "example")
        | "\(.name)\t\((.["required-features"] // []) | join(","))"
      ' \
    | sort
)

total=${#EXAMPLES[@]}
[ "$total" -gt 0 ] || { echo "ERROR: no example targets discovered." >&2; exit 1; }

echo "Running $total example targets to completion…"
pass=0
fail=0
failed=()
for row in "${EXAMPLES[@]}"; do
  name="${row%%$'\t'*}"
  feats="${row#*$'\t'}"
  args=(run --example "$name" --quiet --locked)
  [ -n "$feats" ] && args+=(--features "$feats")
  printf '  %-28s %s' "$name" "${feats:+[$feats] }"
  if cargo "${args[@]}" >/dev/null 2>&1; then
    printf '\033[32mok\033[0m\n'; pass=$((pass + 1))
  else
    printf '\033[31mFAIL\033[0m\n'; fail=$((fail + 1)); failed+=("$name")
  fi
done

echo
echo "examples: $pass passed, $fail failed (of $total)"
if [ "$fail" -ne 0 ]; then
  printf 'FAILED: %s\n' "${failed[*]}" >&2
  exit 1
fi
echo "All $total examples ran to completion."
