<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.15 Release Notes

The **loader-parity completion + coverage-hardening** cut. Finishes the
three-loader DoS-budget parity begun in v0.0.14 — extending the remaining
budgets to the span-free `NoSpanLoader` and the distinct-typed key-collision
guard to the streaming loader — and lands a workspace-wide coverage
campaign that drives roughly sixteen files to effective-100%.

No breaking API changes. No MSRV change (still 1.85). No new runtime
dependencies. The only behavioural change is that a few previously
under-guarded DoS budgets and one collision case are now enforced on the
loader paths that were still missing them.

## Why this release exists

v0.0.14 was the loader-parity cut: it brought the `NoSpanLoader` (`Value`
fast path) up to par with the span-full `Loader` on the key-collision
guard and four DoS budgets. Two gaps remained after that cut, and are
closed here:

1. Three budgets — `max_events`, the total-scalar-bytes cap, and the
   `alias_anchor_ratio` — were still span-full-only. On adversarial
   input, the `NoSpanLoader` path could exceed limits the span-full path
   rejects.
2. The distinct-typed key-collision guard ran on the AST loaders but not
   on the **streaming** deserialiser — the third of the three loaders.
   `1: a\n"1": b\n` deserialised through the streaming path could still
   silently collapse.

Both are now fixed, so all three loaders — streaming, span-full `Loader`,
and `NoSpanLoader` — enforce the same DoS budgets and the same
collision semantics, with cross-path tests proving it.

Alongside the parity work, this release folds in a workspace coverage
campaign (test-only) that hardens the regression suite substantially.

## What changed

### Loader parity (security)

- **`NoSpanLoader` DoS-budget parity completed** — `max_events`,
  total-scalar-bytes, and `alias_anchor_ratio` budgets now enforced on
  the `Value` fast path, matching the streaming and span-full loaders.
  Covered by `tests/no_span_loader_parity.rs`.
- **Streaming key-collision guard** — the distinct-typed-key
  `Error::KeyCollision` guard now runs on the streaming deserialiser,
  closing the last loader where `1` vs `"1"` could collapse. Covered by
  `tests/key_collision_streaming.rs`.
- Removed an orphaned, unreachable tag-preserving deserialiser path in
  `de/deserializer.rs` (dead code; no behavioural effect).

### Testing / tooling (no behavioural change)

- Workspace coverage campaign: ~16 files driven to effective-100% —
  `de` / `include`, `schema_validate`, `compat/serde_yaml` (100% fn),
  `base64` (99.4% line), `cst/coerce`, `error` (98.7% region),
  `ser`, `value/number` (97.5% region), `cst/green` (100%), `recovery`,
  and the CST formatter.
- Corrected a **wrong-but-green** regression test (an `!include`
  depth-cap test that tripped the cycle guard instead of the depth
  guard) and an over-claimed "defensive" annotation (formatter arms that
  malformed input actually reaches).
- `make coverage-gap` restored under `cargo-llvm-cov ≥ 0.8.7` — the
  empty `--ignore-filename-regex` is now guarded, matching the CI
  workflow.

## What did not change

- Public API surface — no additions, removals, or signature changes.
- MSRV — still Rust 1.85.
- Dependencies — no new runtime deps; `#![forbid(unsafe_code)]` intact.
- Default parse/serialise behaviour for well-formed input — identical.

## Follow-ups noted for v0.0.16

- Still open from v0.0.14: migrate `max_sequence_length` /
  `max_mapping_keys` from `Error::Serialize("… limit exceeded")` to
  `Error::Budget(BudgetBreach::…)` for full `Error::kind()` classifier
  parity; and wire `cargo-semver-checks` into the release workflow.
- Coverage floor: literal 98% region is not reachable by tests alone —
  defensive arms (`invariant_violated`, `unreachable!()`, in-file test
  `panic!`) are permanently counted, and `#[coverage(off)]` can only
  exclude whole functions. The remaining region gap is that floor, not
  missing tests.
