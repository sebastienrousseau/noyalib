<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Developing noyalib

The single entry point for working on this repository. User-facing
documentation lives in [`docs/`](docs/) and the
[User Guide](docs/USER-GUIDE.md); contribution etiquette and review
expectations live in [`CONTRIBUTING.md`](CONTRIBUTING.md). This file
is the *how*: toolchain, tasks, and reproducing every CI gate locally.

## Why `crates/noyalib` and not `src/` at the root

A deliberate relic. Before ADR-0005 every satellite lived in this
workspace under `crates/*`; they moved to their own repositories and
the core stayed where it was. Flattening would touch ~190 path
references (shared workflows the satellites pin by SHA, coverage
regexes, REUSE annotations, the OSS-Fuzz project files, and the
source links baked into every published rustdoc), so the directory
stays until a flattening is worth its own structure-only cycle.
Treat `crates/noyalib/` as the crate root and the repository root as
the workspace root, and nothing else needs to be known about it.

## Toolchain

| What | Version | Why |
| :--- | :--- | :--- |
| Rust stable | 1.86.0 or later | MSRV, CI-enforced (`msrv-core`) |
| Rust nightly | any recent | Miri, cargo-fuzz, coverage (`llvm-cov` with `coverage_attribute`) |
| cargo-deny, cargo-vet | latest | supply-chain gates |
| cargo-hack | latest | per-feature and powerset builds |
| cargo-fuzz | latest, **installed from source** (`cargo install --locked cargo-fuzz`) | the prebuilt musl binary mis-infers the fuzz target triple |
| Kani + kissat | latest | proof suite (weekly CI; `#[kani::solver(kissat)]` — the default solver does not terminate on these harnesses) |
| reuse (pipx/pip) | ≥ 3.3 | REUSE lint |

```bash
git clone https://github.com/sebastienrousseau/noyalib
cd noyalib
make            # check + clippy + test — the default gate
```

## Task map

`make` targets are the canonical dev tasks (see the
[`Makefile`](Makefile) header for the full list):

| Task | Command |
| :--- | :--- |
| Everything a PR needs first | `make` |
| Full test suite | `make test` |
| Lints / formatting | `make clippy` / `make fmt` |
| Docs as CI builds them | `make doc` |
| Focused Miri (unsafe-dep interaction + big-endian) | `make miri` |
| SBOM / license attribution | `make sbom` / `make notice` |
| Offline vendor tree | `make vendor` |
| All examples | `make examples` |
| Per-crate MSRV verification | `make msrv-per-crate` |

## Reproducing the CI gates

Every job in [`ci.yml`](.github/workflows/ci.yml) has a local
equivalent. The ones that catch people out:

| CI job | Local reproduction | Gotcha |
| :--- | :--- | :--- |
| `docs-strict` | `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features` | feature-gated intra-doc links need explicit paths |
| `coverage-gate` | `cargo +nightly llvm-cov --all-features` | thresholds: 95% fn / 94% line / 93% region — rationale in the job's comment |
| `each-feature` | `cargo hack check -p noyalib --each-feature --exclude-features nightly-simd,compare-saphyr --no-dev-deps` | **cargo-hack rewrites Cargo.toml while running** — never run other cargo commands concurrently |
| `fuzz-regression` | `cd fuzz && cargo +nightly fuzz build`, then replay `corpus/seed` + `regressions/<target>` per target with `-- -runs=0` | needs nightly + source-installed cargo-fuzz |
| `cargo-vet` | `cargo vet` (after a dep change: `cargo vet regenerate exemptions`) | exemptions must move with every dependency bump |
| `no-std` legs | `cargo check --no-default-features` (+ feature combos from `shared-no-std.yml`) | `std`-only items need `#[cfg(feature = "std")]` |
| `msrv-core` | `cargo +1.86.0 check` / clippy on the default and no-default sets | dev-deps (criterion 0.8) need 1.86+, so `--all-targets` on older toolchains fails by design |

## Test layout

- `crates/noyalib/tests/` — integration suites, one file per surface;
  every fixed bug carries a pinning test in the same commit.
- `crates/noyalib/tests/yaml-test-suite/` — the vendored official
  YAML 1.2 suite (406/406 must stay green).
- `tests/serde_yaml_contract.rs` — the 18-case behavioural contract
  with expectations captured live from `serde_yaml 0.9.34`.
- `fuzz/` — 12 libFuzzer targets; `fuzz/corpus/seed` +
  `fuzz/regressions/` replay per push. Differences of opinion with
  other parsers never enter `regressions/` — they are pinned as unit
  tests (see `tests/competitor_bugs.rs`).
- Kani proofs live beside the code they verify (`src/simd.rs`) and
  run weekly (`kani-proofs.yml`).

Full testing philosophy: [`docs/TESTING.md`](docs/TESTING.md).

## The ecosystem model

Six crates release in lockstep at the same `=0.0.X`
([ADR-0005](docs/adr/0005-workspace-split.md)): `noyalib` (this repo)
plus the satellites `noya-cli`, `noyalib-lsp`, `noyalib-wasm`,
`noyalib-mcp`, and `noyalib-serde-yaml`. Satellites pin the core
exactly; a core release is only half done until every satellite ships
the same version. Satellites consume this repo's `shared-*.yml`
workflows by SHA. The ecosystem map lives in
[`docs/ECOSYSTEM.md`](docs/ECOSYSTEM.md).

Version bumps are strictly `+0.0.1`. Every version-bearing file is
checked by `scripts/verify-release-versions.sh vX.Y.Z`; the release
pipeline (`release.yml`) is tag-triggered, supports a
`workflow_dispatch` dry run, and publishes to crates.io via OIDC
trusted publishing.

## House rules

- CI must be green in the same session that turned it red.
- Commits are signed; releases are signed tags (`KEYS.asc`).
- Structure cleanups never couple to code changes.
- New behaviour lands with its test in the same commit; a regression
  fix lands with the input that found it.
