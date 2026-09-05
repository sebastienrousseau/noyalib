# Testing strategy

noyalib's test pyramid is unusually wide for a library of its size.
This document explains *why* — what each layer catches, where it
runs, and how to extend it.

## The five layers

```mermaid
graph TD
    A[Doctests<br/>~384] --> B[Unit tests<br/>inline #[cfg(test)]]
    B --> C[Integration tests<br/>tests/*.rs · 139 files]
    C --> D[Property + proptest]
    D --> E[Fuzz · libfuzzer<br/>10 targets]
    F[Miri · UB detection] -.parallel.-> C
    G[Bench · Criterion<br/>16 harnesses] -.parallel.-> C
    H[Spec compliance<br/>406/406 strict yaml-test-suite] -.parallel.-> C
```

Each layer catches a different bug class. None replaces another —
they compose.

## Doctests (~384)

Every public item carries a runnable example in its docstring. The
noyalib README is also wired into the doctest sweep via:

```rust
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_doctests {}
```

So every `` ```rust `` block in the README is exercised on every
PR. This catches API drift in code samples — a frequent rot
vector in libraries with marketing-grade READMEs.

**Run:** `cargo test --doc -p noyalib --all-features`

## Unit tests

Co-located with implementation in `#[cfg(test)] mod tests` blocks.
Used for tightly-scoped invariants — encoding tables, scanner
state machines, format-config heuristics. Not the place for
end-to-end flows; those live in `tests/`.

## Integration tests (139 files in `crates/noyalib/tests/`)

Each file is a focused theme. Selection of the most load-bearing:

| File | Tests | What it locks |
|---|---|---|
| `official_suite.rs` | 406 | YAML 1.2 spec compliance — every case from `yaml-test-suite` |
| `yaml_compliance_report.rs` | — | Cross-checks against `yaml-rust2`, `serde_yaml_ng`, `saphyr` outputs |
| `borrowed_alias_resolution.rs` | 12 | P4 borrowed-path alias resolution + bomb defence |
| `yaml_version.rs` | 11 | P2 `version(YamlVersion::V1_1)` toggle |
| `json_surrogate_escape.rs` | 13 | JSON-style `\uXXXX\uXXXX` surrogate-pair pairing |
| `scanner_panic_regressions.rs` | 2 | Fuzz-discovered scanner panics — no-DoS contract |
| `legacy_sexagesimal.rs` | 19 | YAML 1.1 base-60 numeric resolution |
| `merge_keys_streaming.rs` | — | `<<:` merge-key policy permutations |
| `cst_format.rs` / `cst_round_trip.rs` | — | CST byte-faithful round-trip |
| `proptest.rs` | — | Property-based generation against the parser |
| `serde_ecosystem.rs` | — | `serde_path_to_error`, `serde_ignored` interop |

**Run:** `cargo test --workspace --all-features`

## Property + proptest

`proptest` generates structured YAML inputs to assert universal
properties — `parse(emit(value)) == value`, `parse(format(s))` is
total over valid YAML, etc. Catches edge cases that case-by-case
tests miss because the input space is hostile in shape, not just
content.

## Fuzz (10 targets, libfuzzer)

Located in `fuzz/fuzz_targets/`:

| Target | Coverage |
|---|---|
| `fuzz_parse` | Top-level `from_str::<Value>` panic surface |
| `fuzz_roundtrip` | `parse(emit(parse(s)))` differential |
| `fuzz_from_value` | `Value` → typed deserialise |
| `fuzz_multi_doc` | `load_all_as` document-boundary scanning |
| `fuzz_strict` | `from_str_strict<T>` unknown-key surfacing |
| `fuzz_diff` | Differential against `yaml-rust2` / `saphyr` / `serde_yaml_ng` |
| `fuzz_borrowed_alias` | P4 alias resolution + bomb cap |
| `fuzz_no_span_loader` | `NoSpanLoader` path — three-loader parity |
| `fuzz_yaml_v1_1` | P2 1.1 resolver bundle + individual flag overrides |
| `fuzz_double_quoted` | Surrogate pair pairing + scalar escape branches |

CI runs a 10-second smoke pass per PR via the `Differential fuzz`
job. Long-form pre-release fuzz runs are operator-driven.

**Run a 30-second smoke locally:**

```sh
cargo +nightly fuzz run fuzz_parse -- -max_total_time=30 -timeout=5
```

The fuzz pass for v0.0.1 caught one pre-existing scanner panic
(scanner.rs:1644 — `slice index starts at 2 but ends at 0` on
adversarial implicit-key tracking) that the existing corpus had
never triggered. Both crashing inputs are now locked in
`scanner_panic_regressions.rs`.

## Miri (UB detection)

`cargo +nightly miri test` runs the entire suite under the Miri
interpreter, which catches undefined behaviour stdlib usage that
ordinary tests miss — uninitialised reads, out-of-bounds aliasing,
strict-provenance violations.

The workspace forbids `unsafe_code` so noyalib's own code can't
introduce UB. Miri is therefore catching deps' problems
*reachable from noyalib's code*. It has caught `memchr` SSE2
false positives on `-Zmiri-symbolic-alignment-check` (which is now
disabled in `scripts/miri.sh`) and would catch any future regression
in `indexmap`'s iteration invariants or `serde`'s borrow lifetime
plumbing.

CI runs a focused Miri pass per PR (60-min timeout) and a full
Miri + big-endian sweep on schedule.

**Run:** `./scripts/miri.sh`

## Benchmarks (16 Criterion harnesses)

Performance is treated as a correctness invariant — regressions
are bugs the same way wrong output is. Per-crate harnesses:

| Crate | Harnesses | What they cover |
|---|---|---|
| `noyalib` | 16 | core parser, comparison vs serde_yaml_ng / yaml-rust2 / saphyr, SIMD, incremental repair, validation overhead, large-doc soak, structural bitmask, numeric parse |
| `noya-cli` (split repo) | 1 | clap argv parse + command-tree construction |
| `noyalib-lsp` (split repo) | 1 | `textDocument/formatting` + parse-for-diagnostics |
| `noyalib-mcp` (split repo) | 1 | the four `tools/call` operations |

CI runs the comparison + microbench suites via CodSpeed on every
PR (the `CodSpeed perf dashboard` workflow); regressions block
merge.

**Run:** `cargo bench --workspace`

## Spec compliance: 406/406 strict

The `yaml-test-suite` is the canonical 1.2 conformance corpus.
noyalib's compliance harness reports **406/406 strict-pass, 0
failures, and 0 skips out of 406 total cases** (each
case directory yields one or more variant assertions; the totals
reflect those variant counts). The full breakdown rebuilds on
every CI run into `target/yaml-compliance-report.md`.

The strict-pass set is locked by `tests/official_suite.rs` so a
regression — any case dropping from pass to skip / fail — fails
CI immediately. Each new correctness fix lands with the
corresponding suite case unblocked.

## Coverage gate

CI's `Coverage gate (≥96%)` job runs `cargo +nightly llvm-cov`
across the workspace and fails under:

- `--fail-under-functions 96` — every public/private fn
- `--fail-under-lines 94` — line coverage
- `--fail-under-regions 93` — region (branch) coverage

Excluded from the report: `noyalib-wasm/src/lib.rs` (JsValue
marshalling needs a wasm-bindgen runtime; covered separately by
`wasm_bindgen_test` under `wasm-pack test`) and the `protocol.rs`
end-to-end subprocess tests in MCP/LSP (don't run cleanly under
llvm-cov instrumentation; the same logic is covered by per-module
unit tests).

## Adding a test

Match the layer to the bug class:

| Bug class | Layer |
|---|---|
| API contract / public docstring drift | doctest |
| Single-function invariant | unit test |
| End-to-end flow | integration test in `tests/*.rs` |
| Universal property over inputs | proptest |
| Adversarial input panics | fuzz target + regression test in `tests/scanner_panic_regressions.rs` |
| Memory-safety / UB | already covered — Miri runs the full suite |
| Performance regression | Criterion bench |
| Spec compliance gap | add the case to `yaml-test-suite`, ensure `official_suite.rs` covers it |

Every new public surface ships with new tests **in the same
commit / PR** — never as a follow-up. This invariant is in
`CONTRIBUTING.md` and enforced socially in review.
