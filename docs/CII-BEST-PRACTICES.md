<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Noyalib. All rights reserved. -->

# OpenSSF Best Practices Badge — self-assessment

noyalib holds the passing-level badge: project
[**13057**](https://www.bestpractices.dev/projects/13057),
100% of the 65 passing criteria, achieved 2026-05-31. The badge
is embedded in the workspace `README.md` header. This file is
the maintenance record: each criterion mapped to the artefact
that satisfies it, so answers can be re-verified when the
project changes and the Silver application can start from it.

## Passing-level criteria — prefilled answers

The 65 criteria are organised under six headings on the badge
site. Each row below maps the criterion → the noyalib
artefact that satisfies it.

### Basics

| Criterion | Satisfied by |
| :--- | :--- |
| Project website / repository URL | `https://github.com/sebastienrousseau/noyalib` |
| Description / what the project does | Repository description + `README.md` headline |
| Stable URL for the project | The GitHub repo URL above |
| Discussion mechanism | GitHub Issues + Discussions enabled |
| License is OSI-approved | `MIT OR Apache-2.0` — see `LICENSE-MIT`, `LICENSE-APACHE` |
| Project provides documentation | `README.md`, `docs/USER-GUIDE.md`, `docs/ARCHITECTURE.md`, `docs/POLICIES.md`, `docs/BENCHMARKS.md` |
| Documentation includes "Quick Start" | `README.md` §"Quick Start", `crates/noyalib/README.md`, `GETTING_STARTED.md` |
| Documentation has a security policy | `SECURITY.md` |
| Maintainer-direct contact | `sebastian.rousseau@gmail.com` (per `SECURITY.md`) |
| Public bug tracker | GitHub Issues |
| Acknowledgement of contributions | `CONTRIBUTING.md`, GitHub PR / issue author attribution |

### Change control

| Criterion | Satisfied by |
| :--- | :--- |
| Source under version control | Git, GitHub-hosted |
| Unique version identifier per release | SemVer tags `v0.0.x` |
| Release notes per version | `docs/release-notes/v0.0.X.md` for each tagged release, indexed by [`docs/release-notes/README.md`](release-notes/README.md) |
| Standardised file structure | Cargo workspace conventions; `crates/`, `doc/`, `pkg/` |
| Changelog kept | `CHANGELOG.md` (Keep-a-Changelog format) |

### Reporting

| Criterion | Satisfied by |
| :--- | :--- |
| Bug reports tracked | GitHub Issues, with templates in `.github/ISSUE_TEMPLATE/` |
| Bug report responses ≤ 14 days | Issue-response SLA documented in `SECURITY.md` (48 h initial response) |
| Vulnerability report channel | `SECURITY.md` — disclosure via `sebastian.rousseau@gmail.com`, 48 h response |
| Security audit log | Audit reports tracked in `docs/POLICIES.md` § "Audit pipeline" |

### Quality

| Criterion | Satisfied by |
| :--- | :--- |
| Working build system | `cargo build --workspace --all-features` |
| Working test system | `cargo test --workspace --all-features` (~5 400 tests) |
| Tests run on every change | `.github/workflows/ci.yml` triggers on push + pull_request |
| Code-coverage measurement | `.github/workflows/ci.yml` § `Coverage gate (≥96%)` — `cargo llvm-cov` |
| Coverage tool integration | Same |
| New features include tests | Required by review process; enforced by `cargo-machete`, strict-doc gate |
| Documented coding style | Workspace-level lints in `crates/noyalib/Cargo.toml`; `cargo fmt` enforced by CI |
| Code review of every change | `main` ruleset requires PR + 1 approving review + code-owner review + last-push approval (post-this-commit) |

### Security

| Criterion | Satisfied by |
| :--- | :--- |
| Cryptographic best practices | Releases signed via cosign keyless + SLSA L3 build provenance attestations on every artefact |
| Inputs validated before use | Parser enforces `ParserConfig` limits (`max_depth`, `max_document_length`, `max_alias_expansions`, …) |
| Hardened against vulnerabilities | `#![forbid(unsafe_code)]` workspace-wide, fuzz suite (10 targets) + Miri soak runs in `.github/workflows/security.yml` |
| Vulnerability disclosure tested | One historical CVE-equivalent (issue #46 RecursionLimitExceeded false-positive) — patched in v0.0.6 within the same release cycle |
| Security expertise consulted | Audit pipeline: `cargo-deny`, `cargo-vet`, `cargo-audit`, `cargo-machete`, CodeQL — see `docs/POLICIES.md` |

### Analysis

| Criterion | Satisfied by |
| :--- | :--- |
| Static analysis applied | `cargo clippy --workspace --all-features -- -D warnings` on every PR; CodeQL on `.github/workflows/security.yml` |
| Dynamic analysis applied | Differential fuzz (10 s smoke per PR) + soak fuzz (1 h per target weekly); Miri (focused per PR + full weekly) |
| Coverage-guided fuzzing | `cargo-fuzz` with `libFuzzer`, 10 targets: `fuzz_borrowed_alias`, `fuzz_diff`, `fuzz_double_quoted`, `fuzz_from_value`, `fuzz_multi_doc`, `fuzz_no_span_loader`, `fuzz_parse`, `fuzz_roundtrip`, `fuzz_strict`, `fuzz_yaml_v1_1` |
| Memory-safety analysis | `#![forbid(unsafe_code)]` (the strongest possible static guarantee) + Miri runs to verify transitive `unsafe` blocks in dev-deps |

## Silver / Gold level (future)

The "Silver" badge adds 30+ more criteria around release
process maturity and "Gold" 30 more around supply-chain
transparency. The passing entry is complete; tracking Silver
for the v0.1.0 milestone via
<https://www.bestpractices.dev/en/projects/13057/silver/edit>.
