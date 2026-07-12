<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Profile-Guided Optimization (PGO) — `noyalib`

Profile-Guided Optimization is a compiler technique that lays
out hot paths inline, cold paths off the fast track, and
optimises branch ordering based on the *actual* execution
profile of a representative workload — rather than the static
heuristics LLVM uses by default. For YAML parsing, where the
scanner's per-byte dispatch is the dominant cost, PGO typically
yields **5–15% speedup** on `from_str::<Value>` and
`cst::parse_document` against representative inputs.

This document is the operational guide: how to build a PGO'd
`noyalib` binary, what's instrumented, and what gains to
expect.

> **Status**: opt-in. The default `cargo install noya-cli`
> path does **not** use PGO (PGO requires a two-pass build
> that's longer and produces target-specific artefacts).
> Distro packagers and downstream teams who want the extra
> 5–15% can run `scripts/pgo.sh` to produce a PGO'd binary.

## Quick start

```bash
# Run the full PGO pipeline against the default training corpus.
./scripts/pgo.sh

# Custom training input:
./scripts/pgo.sh path/to/representative-workload.yaml

# Train a different binary (default is noyafmt):
PGO_TARGET=noyavalidate ./scripts/pgo.sh
```

The script:

1. Builds an instrumented binary with
   `RUSTFLAGS=-Cprofile-generate=…`.
2. Runs the instrumented binary against the training corpus
   (default: every YAML under `tests/yaml-test-suite/` plus
   `benches/fixtures/`).
3. Merges the per-process `*.profraw` files via
   `llvm-profdata merge` into a single `merged.profdata`.
4. Builds the optimised binary with
   `RUSTFLAGS=-Cprofile-use=…/merged.profdata`.

Output: `target/release/<binary>` is now PGO'd. The script
prints a `hyperfine` recipe for measuring the gain against
the non-PGO baseline.

## Requirements

- **rustc 1.85.0+**. PGO has been stable since Rust 1.45;
  noyalib's MSRV (1.85.0) easily clears that.
- **`llvm-profdata`** on `$PATH`. Install via
  `rustup component add llvm-tools-preview`. The script
  auto-discovers the rustup-shipped binary if `$PATH` lookup
  fails.
- **Training corpus** that resembles real production input.
  The default corpus (the YAML test suite + benches fixtures)
  is broad — for per-team optimisation, replace it with your
  own representative YAML.

## What gets instrumented

PGO instruments **every basic block** of the trained binary
including its statically-linked dependencies. For
`noyafmt` / `noyavalidate`, that's:

- `noyalib::parser::scanner` — the per-byte dispatch loop.
- `noyalib::parser::events` — the event-tree state machine.
- `noyalib::parser::loader` — the AST builder.
- `noyalib::cst::*` — when the binary touches the CST path
  (`noyafmt --check`, `--fix`).
- `noyalib::ser` — emit-side hot paths.
- Transitive deps — `serde`, `indexmap`, `ryu`, `itoa`,
  `memchr`.

The compiler then re-orders branches in the optimised pass so
the *most-taken* arm of each match is checked first. For YAML,
that's overwhelmingly *plain scalar* in the scanner dispatch
and *Mapping* / *Sequence* in the event-tree machine.

## Expected gains

Measured on Apple M4, Rust 1.94 stable, 97 KB synthetic
mapping-of-records document:

| Workload | Baseline | PGO | Speedup |
|---|---|---|---|
| `noyafmt --check` (parse + canonical compare) | 5.2 ms | 4.4 ms | **1.18×** |
| `noyavalidate --schema` (parse + JSON Schema check) | 9.1 ms | 7.9 ms | **1.15×** |
| `from_str::<Value>` micro-bench | 2.69 ms | 2.31 ms | **1.16×** |
| `cst::parse_document` micro-bench | 4.59 ms | 4.04 ms | **1.14×** |

Gains are workload-specific. Ranges of 5–20% are typical for
programs whose hot path is well-defined and stable; YAML
parsing is exactly that kind of workload.

## Combining with the workspace `[profile.release]` settings

The PGO build composes with the workspace release profile
(`opt-level = 3`, `lto = "fat"`, `codegen-units = 1`,
`overflow-checks = true`). The training pass uses the same
release profile (instrumentation aside), so the optimised
build benefits from both LTO + PGO together.

## CI integration

The `scripts/pgo.sh` workflow is intentionally kept out of
the per-PR CI path because:

1. The two-pass build doubles the wall-clock cost.
2. PGO artefacts are target-specific (per architecture).
3. A small per-PR perf regression isn't best caught via PGO —
   `CodSpeed` already tracks per-bench drift on the standard
   release profile.

PGO belongs in the **release pipeline** when the binary is
about to ship to a wide audience. The
[`.github/workflows/release-binaries.yml`](https://github.com/sebastienrousseau/noyalib/blob/main/.github/workflows/release-binaries.yml)
workflow is the natural integration point — build PGO'd
artefacts for the published binaries (`noyafmt`,
`noyavalidate`, `noyalib-mcp`, `noyalib-lsp`) per host triple.

## Cross-platform notes

- **Linux**: works out of the box with the rustup-shipped
  `llvm-tools-preview`.
- **macOS**: same. Apple-silicon-specific PGO data is
  produced; do not reuse a Mac-trained `merged.profdata` on
  Linux x86_64.
- **Windows**: works with `llvm-profdata.exe` from the
  rustup `llvm-tools-preview` component. The MSVC linker
  handles the `-Cprofile-use=` flag; no extra setup needed.

Trained `merged.profdata` is **per host triple** — re-run
the script on each release runner.

## Troubleshooting

- *"cannot find llvm-profdata"* → install via
  `rustup component add llvm-tools-preview`.
- *"profile data file is empty"* → the training step didn't
  produce any `.profraw` files. Verify the binary actually
  executed against the training corpus (the script `tee`s
  output via `/dev/null`; check exit codes).
- *"profile data is incompatible"* → the rustc version that
  generated the profile differs from the one consuming it.
  Re-run the full pipeline on the current toolchain.

## Further reading

- [The Rust Reference — profile-guided optimization](https://doc.rust-lang.org/rustc/profile-guided-optimization.html).
- [LLVM Profile-Guided Optimization](https://llvm.org/docs/HowToBuildWithPGO.html).
- The `cargo-pgo` crate provides a higher-level driver
  alternative to the shell script — drop-in replacement for
  `scripts/pgo.sh` if your team prefers a Cargo-native UX.
