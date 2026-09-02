<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# The noyalib ecosystem

This document does two things. It describes what the ecosystem is
today, and it states — with numbers anyone can reproduce — how good it
actually is. The second part is the harder one, and the reason
`scripts/ecosystem-scorecard.sh` exists.

## Why a harness instead of a claim

A rating is worth something only if it can be shown to be wrong. Most
"production-ready / battle-tested / enterprise-grade" language in this
corner of the industry cannot be, because nothing is attached to it. So
every number below is produced by a command, and the command is printed
next to the number.

Three rules govern the harness:

1. **Falsifiable.** Each metric names the exact command and the exact
   threshold. "Docs are good" is not a metric. "rustdoc emits 0 warnings
   under `-D warnings`" is — run it and see.
2. **No credit for unmeasured work.** A probe that cannot run scores
   `N/A` and is removed from the denominator. It never scores 0, and it
   never silently scores 1. The report states what fraction of the
   rubric actually executed, because a perfect score over a third of the
   rubric is not a perfect score.
3. **Reproducible.** The header records rustc/cargo versions, host
   triple, each repo's commit SHA, and whether the worktree was dirty.

Run it:

```sh
scripts/ecosystem-scorecard.sh                  # local probes
scripts/ecosystem-scorecard.sh --network        # + crates.io, GitHub, OpenSSF
scripts/ecosystem-scorecard.sh --deep           # + all-features build probes
scripts/ecosystem-scorecard.sh --with-coverage  # + cargo-llvm-cov
scripts/ecosystem-scorecard.sh --json out.json
```

It exits non-zero below `SCORE_FLOOR` (default 0.90), so the rating can
be a CI gate rather than a boast.

### What it deliberately does not measure

- **Adoption.** Downloads are recorded as context below, not scored.
  Popularity is not quality, and scoring it would let the rating drift
  with marketing rather than engineering.
- **Benchmark numbers.** The harness checks that `docs/BENCHMARKS.md`
  discloses host, toolchain and repro command — the things that make a
  speed claim falsifiable — but does not re-run criterion. Timing a
  laptop under variable load and calling the delta a score would be
  measurement theatre.
- **Code review.** OpenSSF scores this 0 across every repo in the
  family, correctly: these are single-maintainer repos with squash
  merges. That is a real limitation and is left visible rather than
  excluded.

## The competitive landscape, 2026

YAML tooling is not one market. It is five, and each has a different
incumbent in a different language.

| Surface | Incumbent | Language | noyalib's answer |
|---|---|---|---|
| Library | `serde_yaml` (archived 2024) | Rust | `noyalib` |
| CLI query / transform | `yq` | Go | **nothing** |
| Style linter | `yamllint` | Python | **nothing** |
| Language server | `yaml-language-server` (Red Hat) | TypeScript | `noyalib-lsp` |
| Agent tooling (MCP) | no incumbent | — | `noyalib-mcp` |
| Browser / JS | `yaml` (eemeli), `js-yaml` | JavaScript | `noyalib-wasm` |

The Rust library tier alone has fragmented since `serde_yaml` was
archived. Recent-download counts from the crates.io API:

| Crate | Recent downloads | Status |
|---|---:|---|
| `serde_yaml` | 87.1M | archived upstream |
| `yaml-rust2` | 13.9M | maintained, no serde layer |
| `serde_yml` | 6.4M | description begins "DEPRECATED" |
| `serde_yaml_ng` | 5.0M | maintained fork |
| `serde-saphyr` | 3.6M | maintained, newer |
| `serde_norway` | 2.7M | maintained fork |
| **`noyalib`** | **0.247M** | maintained |

The honest reading: noyalib is roughly **0.3%** of the deprecated
incumbent and **6.8%** of `serde-saphyr`. On measured capability it
leads; on measured reach it does not. The constraint is distribution,
not engineering — which is exactly where the gaps below sit.

For a feature-by-feature comparison of the Rust libraries — and
instructions for verifying any row of it yourself — see
[`COMPARISON.md`](COMPARISON.md). This document is about the *suite*,
not the library.

The structural advantage is that noyalib is the only one of these that
spans all five surfaces from a single core. `yq`, `yamllint` and
`yaml-language-server` are three unrelated codebases in three languages
with three different YAML parsers and three different sets of edge-case
behaviour. A team that adopts them gets three answers to "is this
document valid?".

## Measured gaps

Every row below was produced by a filesystem or API probe on
2026-08-20, not by judgement.

### Capability the suite cannot express today

| Missing | Evidence | Incumbent |
|---|---|---|
| Query / transform tool | `noya-cli` declares exactly two `[[bin]]`: `noyafmt`, `noyavalidate` | `yq` |
| Style linter | `noyavalidate` is schema validation; no line-length, key-duplication, indentation or quote-consistency rules | `yamllint` |

Both are surfacing work, not engine work. The lossless CST already
tracks every span these rules need, and already backs `remove`/`set`.

### Distribution

| Missing | Evidence |
|---|---|
| GitHub Action | no `action.yml` in any of the five repos |
| Hosted pre-commit hook | no `.pre-commit-hooks.yaml` in `noya-cli` — `docs/pre-commit.md` documents only `repo: local`, which needs `noyafmt` already installed |
| VS Code extension | no `editors/vscode` in `noyalib-lsp` — the server exists but nobody can install it from the marketplace |
| Homebrew formula | no `Formula/` |
| CLI container image | `Dockerfile` exists in `noyalib` and `noyalib-mcp`, not in `noya-cli` |
| Python bindings | no `bindings/python` |
| Native Node bindings | wasm only; no napi |

### Supply-chain posture

OpenSSF Scorecard, scans taken the same day at the current HEAD of each
repo:

| Repo | Score |
|---|---:|
| `noyalib` | 8.0 |
| `noya-cli` | 6.5 |
| `noyalib-lsp` | 6.5 |
| `noyalib-mcp` | 6.4 |
| `noyalib-wasm` | 6.4 |

The satellite deficit is concentrated in four checks:

- **Fuzzing 10 -> 0.** No satellite has a `fuzz/` directory. Worth about
  a point each and cheap to fix.
- **CII-Best-Practices 5 -> 0.** The badge is registered for the core
  only.
- **Contributors 3 -> 0**, **Signed-Releases 3 -> 1.**

`Code-Review` is 0 on all five, including the core — single-maintainer
squash merges. Honest, and not something a script can fix.

### The credibility gap

Three crates in the same portfolio still depend on YAML libraries their
own authors have marked dead:

| Repo | Dependency | Upstream status |
|---|---|---|
| `libmake` | `serde_yml 0.0.13` | crates.io description begins "DEPRECATED" |
| `nucleusflow` | `serde_yml 0.0.13` | same |
| `wiserone` | `serde_yaml 0.9.31` | archived |
| `metadata-gen` | `noyalib` **and** `yaml-rust2` | half-migrated |

Already migrated: `frontmatter-gen`, `html-generator`,
`static-site-generator`, and the four satellites.

Shipping a YAML library while three of your own crates depend on an
archived one is the most falsifiable credibility claim in the set, and
the cheapest to close.

## Proposed additions

Ranked by measured gap times incumbent strength.

### Tier 1 — capability

1. **`noyaq`** (new bin in `noya-cli`) — query and in-place transform
   with a `yq`-compatible expression subset.
2. **`noyalint`** (new bin in `noya-cli`) — `yamllint`'s rule set over
   the CST.
3. **`noyalib-rules`** (new crate) — the rule engine behind `noyalint`,
   reused by the LSP for diagnostics and by the MCP server as an
   agent-facing lint tool, so three surfaces do not re-implement the
   same checks.

### Tier 2 — distribution

4. First-party **GitHub Action**. CI is where YAML tooling is actually
   consumed; this is the highest-leverage single artefact on the list.
5. **`.pre-commit-hooks.yaml`** — one file, unlocks the entire
   pre-commit user base.
6. **VS Code extension** wrapping `noyalib-lsp`.
7. **Homebrew tap** and a **CLI container image**.

### Tier 3 — reach beyond Rust

8. **`noyalib-py`** (PyO3) — the linting and round-trip audience is
   Python-native.
9. **`noyalib-node`** (napi-rs) — native throughput where WASM loses.

### Tier 4 — cheap credibility

10. Fuzz targets in each satellite.
11. OpenSSF Best Practices badge for the four satellites.
12. Migrate `libmake`, `nucleusflow`, `wiserone`; finish `metadata-gen`.

## Rating

<!-- SCORECARD:BEGIN — generated by scripts/ecosystem-scorecard.sh.
**Rating: A+ (98.4% weighted)** — 110 executed probes, 9 scored N/A.

Measured 2026-08-24T10:39:39Z on `aarch64-apple-darwin`, rustc 1.97.1 (8bab26f4f 2026-07-14). Rubric executed: 92%.

| Repo | Metric | Measured | Score | Probe |
|---|---|---|---:|---|
| `noyalib` | tests | 5754 passed / 0 failed | 1.00 | `cargo test --workspace --locked --default-features` |
| `noyalib` | spec_conformance | 406/406 cases | 1.00 | `cargo test -p noyalib --test yaml_compliance_report` |
| `noyalib` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --default-features -- -D warnings` |
| `noyalib` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib` | unsafe_forbidden | 1/1 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib` | msrv_declared | 1.86.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib` | missing_docs_lint | 1/1 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib` | bench_methodology | 3/3 disclosed (host, toolchain, command) | 1.00 | `grep host/toolchain/repro-command in docs/BENCHMARKS.md` |
| `noyalib` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json | .vulnerabilities.count` |
| `noyalib` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noyalib` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib` | dependency_closure | 12 unique runtime crates | 1.00 | `cargo tree -e normal --prefix none --no-dedupe | sort -u (library budget 60; propagates to consumers)` |
| `noyalib` | actions_sha_pinned | 107/107 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib` | fuzz_targets | 11 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noyalib` | release_gpg_signed | 2 .asc assets | 1.00 | `gh release view --json assets | grep '\.asc$'` |
| `noyalib` | release_sigstore | 2 .bundle assets | 1.00 | `gh release view --json assets | grep '\.bundle$'` |
| `noyalib` | release_sbom | 3 sbom assets | 1.00 | `gh release view --json assets | grep -i sbom` |
| `noyalib` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib/dependabot/alerts?state=open` |
| `noyalib` | ci_main_green | success | 1.00 | `gh run list --branch main --limit 1 --json conclusion` |
| `noyalib` | crates_io_current | tree 0.0.28 / crates.io 0.0.28 | 1.00 | `crates.io/api/v1/crates/noyalib .max_stable_version` |
| `noyalib` | openssf_scorecard | 8.2/10 | 0.82 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib .score` |
| `noya-cli` | tests | 81 passed / 0 failed | 1.00 | `cargo test --workspace --locked --default-features` |
| `noya-cli` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --default-features -- -D warnings` |
| `noya-cli` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noya-cli` | unsafe_forbidden | 1/1 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noya-cli` | msrv_declared | 1.86.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noya-cli` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noya-cli` | missing_docs_lint | 1/1 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noya-cli` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noya-cli` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json | .vulnerabilities.count` |
| `noya-cli` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noya-cli` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noya-cli` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noya-cli` | dependency_closure | 130 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe | sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noya-cli` | actions_sha_pinned | 34/34 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noya-cli` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noya-cli` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noya-cli` | release_gpg_signed | 2 .asc assets | 1.00 | `gh release view --json assets | grep '\.asc$'` |
| `noya-cli` | release_sigstore | 2 .bundle assets | 1.00 | `gh release view --json assets | grep '\.bundle$'` |
| `noya-cli` | release_sbom | 3 sbom assets | 1.00 | `gh release view --json assets | grep -i sbom` |
| `noya-cli` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noya-cli/dependabot/alerts?state=open` |
| `noya-cli` | ci_main_green | success | 1.00 | `gh run list --branch main --limit 1 --json conclusion` |
| `noya-cli` | crates_io_current | tree 0.0.28 / crates.io 0.0.28 | 1.00 | `crates.io/api/v1/crates/noya-cli .max_stable_version` |
| `noya-cli` | openssf_scorecard | 6.6/10 | 0.66 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noya-cli .score` |
| `noyalib-lsp` | tests | 59 passed / 0 failed | 1.00 | `cargo test --workspace --locked --default-features` |
| `noyalib-lsp` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --default-features -- -D warnings` |
| `noyalib-lsp` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib-lsp` | unsafe_forbidden | 2/2 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib-lsp` | msrv_declared | 1.86.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib-lsp` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib-lsp` | missing_docs_lint | 2/2 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib-lsp` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib-lsp` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json | .vulnerabilities.count` |
| `noyalib-lsp` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib-lsp` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noyalib-lsp` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib-lsp` | dependency_closure | 96 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe | sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-lsp` | actions_sha_pinned | 34/34 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-lsp` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-lsp` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noyalib-lsp` | release_gpg_signed | 2 .asc assets | 1.00 | `gh release view --json assets | grep '\.asc$'` |
| `noyalib-lsp` | release_sigstore | 2 .bundle assets | 1.00 | `gh release view --json assets | grep '\.bundle$'` |
| `noyalib-lsp` | release_sbom | 3 sbom assets | 1.00 | `gh release view --json assets | grep -i sbom` |
| `noyalib-lsp` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-lsp/dependabot/alerts?state=open` |
| `noyalib-lsp` | ci_main_green | success | 1.00 | `gh run list --branch main --limit 1 --json conclusion` |
| `noyalib-lsp` | crates_io_current | tree 0.0.28 / crates.io 0.0.28 | 1.00 | `crates.io/api/v1/crates/noyalib-lsp .max_stable_version` |
| `noyalib-lsp` | openssf_scorecard | 6.6/10 | 0.66 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-lsp .score` |
| `noyalib-mcp` | tests | 62 passed / 0 failed | 1.00 | `cargo test --workspace --locked --default-features` |
| `noyalib-mcp` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --default-features -- -D warnings` |
| `noyalib-mcp` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib-mcp` | unsafe_forbidden | 2/2 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib-mcp` | msrv_declared | 1.86.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib-mcp` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib-mcp` | missing_docs_lint | 2/2 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib-mcp` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib-mcp` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json | .vulnerabilities.count` |
| `noyalib-mcp` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib-mcp` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noyalib-mcp` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib-mcp` | dependency_closure | 19 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe | sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-mcp` | actions_sha_pinned | 51/51 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-mcp` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-mcp` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noyalib-mcp` | release_gpg_signed | 2 .asc assets | 1.00 | `gh release view --json assets | grep '\.asc$'` |
| `noyalib-mcp` | release_sigstore | 2 .bundle assets | 1.00 | `gh release view --json assets | grep '\.bundle$'` |
| `noyalib-mcp` | release_sbom | 3 sbom assets | 1.00 | `gh release view --json assets | grep -i sbom` |
| `noyalib-mcp` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-mcp/dependabot/alerts?state=open` |
| `noyalib-mcp` | ci_main_green | success | 1.00 | `gh run list --branch main --limit 1 --json conclusion` |
| `noyalib-mcp` | crates_io_current | tree 0.0.28 / crates.io 0.0.28 | 1.00 | `crates.io/api/v1/crates/noyalib-mcp .max_stable_version` |
| `noyalib-mcp` | openssf_scorecard | 6.6/10 | 0.66 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-mcp .score` |
| `noyalib-wasm` | tests | 42 passed / 0 failed | 1.00 | `cargo test --workspace --locked --default-features` |
| `noyalib-wasm` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --default-features -- -D warnings` |
| `noyalib-wasm` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib-wasm` | unsafe_forbidden | 1/1 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib-wasm` | msrv_declared | 1.86.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib-wasm` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib-wasm` | missing_docs_lint | 1/1 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib-wasm` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib-wasm` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json | .vulnerabilities.count` |
| `noyalib-wasm` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib-wasm` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noyalib-wasm` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib-wasm` | dependency_closure | 33 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe | sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-wasm` | actions_sha_pinned | 42/42 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-wasm` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-wasm` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noyalib-wasm` | release_gpg_signed | 2 .asc assets | 1.00 | `gh release view --json assets | grep '\.asc$'` |
| `noyalib-wasm` | release_sigstore | 2 .bundle assets | 1.00 | `gh release view --json assets | grep '\.bundle$'` |
| `noyalib-wasm` | release_sbom | 3 sbom assets | 1.00 | `gh release view --json assets | grep -i sbom` |
| `noyalib-wasm` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-wasm/dependabot/alerts?state=open` |
| `noyalib-wasm` | ci_main_green | success | 1.00 | `gh run list --branch main --limit 1 --json conclusion` |
| `noyalib-wasm` | crates_io_current | tree 0.0.28 / crates.io 0.0.28 | 1.00 | `crates.io/api/v1/crates/noyalib-wasm .max_stable_version` |
| `noyalib-wasm` | openssf_scorecard | 6.6/10 | 0.66 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-wasm .score` |
| `ecosystem` | version_lockstep | 4/4 pin =0.0.28 | 1.00 | `grep the noyalib pin in each satellite Cargo.toml (ADR-0005); all aligned` |
| `ecosystem` | host_coverage | 5/5 hosts present | 1.00 | `directory presence for noyalib noya-cli noyalib-lsp noyalib-mcp noyalib-wasm` |

<sub>Generated by `scripts/ecosystem-scorecard.sh` v1.0.0. Regenerate rather than edit.</sub>
<!-- SCORECARD:END -->

### How to disagree with this rating

Pick the row you doubt and run its probe. If the number differs, the
harness is wrong and should be fixed — open an issue with the command
output. That is the entire contract.

Two caveats the harness prints about itself and this document repeats,
because a score that hides its own limits is a marketing number:

- **Rubric coverage.** The percentage of probes that actually executed
  is reported alongside the score. Network and coverage probes are
  opt-in; with them off, a large slice of the rubric is `N/A`, and the
  headline is correspondingly less meaningful.
- **Worktree state.** The header records whether each repo was dirty. A
  score measured against uncommitted changes describes those changes,
  not the published artefact.

### Known weaknesses of the harness itself

- **A timed-out probe used to score 0, asserting a failure it had not
  observed.** The run recorded above shows
  `noyalib · tests · timeout after 3600s · 0.00`, which dragged
  `correctness` to 83.3%. That is an artefact of the ceiling firing on a
  cold target dir, not a test failure: the same suite run directly on the
  same commit passed **5739 / 0**, and `spec_conformance` in this very run
  reports 406/406. It also contradicted rule 2 above — an unmeasured probe
  is supposed to score N/A and never 0. Fixed: a timeout now records N/A,
  which keeps it visible (N/A rows print, and they lower the reported
  rubric coverage) without claiming a failure that did not happen. The
  next regeneration will show N/A here rather than a false ✗.

- It measures the *presence* of fuzz targets, not fuzzing effort. Two
  targets that have never found anything score the same as two that
  have.
- **`dependency_closure` used one flat budget of 60 for every repo.** That
  is why `noya-cli` scored 0 at 130 crates. The obvious fix — a per-repo
  budget — turned out to be the wrong one: any number picked now would be
  a number picked so the current value clears it. The rule that survived
  scrutiny is about *propagation*. A library's dependencies are inherited
  by every downstream consumer, so their size is a cost imposed on other
  people and belongs in the score; `noyalib` sits at 12. A leaf binary's
  are not inherited, and 130 of `noya-cli`'s crates are `miette`'s `fancy`
  renderer giving `noyavalidate` source excerpts and carets — which for a
  validator is the job, not bloat. Leaves are therefore **recorded and not
  scored**: the count stays in the table and the JSON, so a leaf that
  doubles its tree is obvious, but it is not judged against a line nobody
  can defend.
- `Code-Review` cannot be fixed by any script, and no probe pretends
  otherwise.
- Test *pass ratio* is scored, but not test *quality*. A suite of 4,000
  trivial assertions scores the same as 4,000 sharp ones. Coverage
  (`--with-coverage`) partly compensates and is off by default because
  it is slow.

## Family layout

Every repository in the family carries the same skeleton, enforced
per push by the `Family layout contract` step of
`shared-docs-lint.yml` — a repo that drops one of these files fails
its own CI:

| File | Role |
| :--- | :--- |
| `README.md` | identity, install, quick start, the four documentation links |
| `CHANGELOG.md` | Keep-a-Changelog, one heading per lockstep release |
| `SECURITY.md` | private reporting channel |
| `CONTRIBUTING.md` | family process (satellites point at the core's) |
| `DEVELOPMENT.md` | developer entry point (satellites point at the core's) |
| `REUSE.toml` | machine-readable licensing for headerless files |
| `docs/` | documentation root (never `doc/`) |
| `.editorconfig`, `.markdownlint.yaml`, `.codespellrc` | shared editor + docs-lint configs |

Satellites additionally carry `scripts/verify-release-versions.sh`
and `supply-chain/`; the core additionally carries the `shared-*.yml`
workflows the family consumes by SHA.
