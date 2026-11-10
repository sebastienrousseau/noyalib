<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# The noyalib ecosystem

The ecosystem has six lockstep crate repositories sharing one parser and
editing engine. This inventory was reconciled on **2026-11-10**. The current
[eleven-category assessment](ROADMAP-TO-10.md) rates each repo separately;
[PLAN.md](PLAN.md) names the implementation work and acceptance evidence
needed to reach 10/10. See the [implementation record](IMPLEMENTATION-STATUS.md)
for observed changes and remaining work. The dated
[evidence snapshot](ecosystem-assessment-2026-11-10.json) pins all six source
commits, CI results, coverage totals, alerts and publication state.

## Repositories and product scope

| Repository | Role | Declared runtime contract |
|---|---|---|
| [noyalib](https://github.com/sebastienrousseau/noyalib) | Parser, Serde, values, streaming and lossless CST editing | Rust std or alloc-only no_std; optional integrations have explicit feature requirements |
| [noya-cli](https://github.com/sebastienrousseau/noya-cli) | `noyafmt` and `noyavalidate` | Native std applications; formatting, validation and conservative repair |
| [noyalib-lsp](https://github.com/sebastienrousseau/noyalib-lsp) | Editor diagnostics, formatting and hover | LSP over framed stdio; supported methods must agree with actual dispatch |
| [noyalib-mcp](https://github.com/sebastienrousseau/noyalib-mcp) | Agent tools for YAML operations | Native std application; existing stdio, streamable HTTP and legacy SSE transports |
| [noyalib-wasm](https://github.com/sebastienrousseau/noyalib-wasm) | JavaScript and lossless document bindings | WebAssembly with generated JS/TypeScript; each package/runtime claim requires execution |
| [noyalib-serde-yaml](https://github.com/sebastienrousseau/noyalib-serde-yaml) | Package-renamed compatibility facade | Thin std-requiring facade over the core compatibility profile |

The core's no_std contract includes documented behavior, not just compilation.
Applications requiring an OS and JS bindings are assessed against their own
runtime scope. The facade's standalone manifest currently enables core std;
the future branch documents the distinction from the core's alloc-only
compatibility profile. Adding a feature to change the facade contract
requires maintainer direction.

The CLI Action and hosted pre-commit hooks, VS Code extension packaging,
satellite fuzz harnesses, MCP npm/registry publication and signed/attested
release artifacts already exist. Earlier text describing them as absent
was stale. Additional query/lint binaries, Python/native Node bindings or
new protocol capabilities require their own product decisions.

## Publication state

As checked on 2026-11-10, five registry crates are at v0.0.60. MCP is at
v0.0.59; its signed v0.0.60 tag exists, but publication is held for the
assessment and P0 findings. This is an incomplete family release, not a
completed lockstep cut. Preserve the existing tag when resolving the hold.

| Repository | Registry crate | Additional channels with release evidence | Remaining consumer acceptance |
|---|---|---|---|
| Core | `noyalib` 0.0.60 | Signed GitHub release | Repeatable registry consumer and recovery receipts |
| CLI | `noya-cli` 0.0.60 | Signed native/package assets; public multiarch container | Package-manager installs, rollback and every claimed target |
| LSP | `noyalib-lsp` 0.0.60 | Signed native assets and VSIX; VS Code Marketplace | Installed extension/server integration and each other claimed marketplace |
| MCP | `noyalib-mcp` 0.0.59 | Signed native assets; npm `@sebastienrousseau/noyalib-mcp`; container; MCP Registry | v0.0.60 hold/source decision and repeated five-channel acceptance |
| WASM | `noyalib-wasm` 0.0.60 | npm `@sebastienrousseau/noyalib-wasm` with provenance | Installed-package entry points, TypeScript, browser and bundler consumers |
| Facade | `noyalib-serde-yaml` 0.0.60 | Signed GitHub release | Renamed-dependency downstream compatibility and recovery |

A signed downloadable package, a package-manager recipe and a successful
installed consumer are distinct observations. R1/R2 in the plan retain the
identity and execution evidence for each claimed channel. The supporting
Homebrew/Scoop/AUR repositories are channel dependencies; this assessment
does not claim to have audited all of their code.

## Automated probes and maturity ratings

`scripts/ecosystem-scorecard.sh` measures seven groups: correctness,
code health, docs, supply chain, robustness, release and integrity. It
reports executed probes and N/A separately. Missing tools, unavailable
services and unmeasured scopes must not become invented passes or failures.

```sh
scripts/ecosystem-scorecard.sh
scripts/ecosystem-scorecard.sh --network
scripts/ecosystem-scorecard.sh --deep
scripts/ecosystem-scorecard.sh --with-coverage
scripts/ecosystem-scorecard.sh --json out.json
```

The workflow in `.github/workflows/ecosystem-scorecard.yml` runs the harness
weekly and on relevant changes, using a 0.90 floor and uploading JSON. A
probe score describes only its measured scopes and revisions. It does not
establish architecture quality, independent review, measured performance,
client interoperability or every runtime boundary.

The historical harness read the last `TOTAL` field, which could contain
branch coverage. The updated harness reads line coverage from structured
LLVM totals and leaves missing or failed measurements as N/A. Release asset
presence still needs cryptographic identity and a real install; fuzz target
counts do not establish sustained campaigns. The external OpenSSF API was unavailable, so no fresh aggregate
OpenSSF rating is asserted.

### Future-branch probe snapshot

The saved [report JSON](scorecard.json) is a local v0.0.61 snapshot taken
while the fixes were in progress. Each checkout's recorded head and
worktree state are shown below. Satellite builds used the local core
override. Registry and remote CI rows describe already published versions
and runs. Some earlier probes precede the signed follow-up changes; for
example, all six later passed the unchanged cargo-vet policy. See the
[implementation record](IMPLEMENTATION-STATUS.md) for the subsequent checks.

The 2026-11-09 generated table remains in Git history. Neither probe
snapshot is a rating across the eleven engineering categories.

<details>
<summary>Show the local automated probe snapshot</summary>

<!-- SCORECARD:BEGIN -->

**Rating: A- (92.1% weighted).** 139 measured; 7 N/A.

Measured 2026-11-10T22:23:26Z on `aarch64-apple-darwin`, rustc 1.99.0 (b940084d7 2026-09-28). Rubric executed: 95%.

| Repository | Version | Source | Worktree |
|---|---|---|---|
| `noyalib` | 0.0.61 | `d082f2a2d32c456173020ed8321005236c12b2cf` | dirty |
| `noya-cli` | 0.0.61 | `bdfc6957fa4dd4329e29f30ef2a8a60ab840cd88` | dirty |
| `noyalib-lsp` | 0.0.61 | `225a2f5a45c75ecaf7a681654a8a0a1d3d89a2a6` | dirty |
| `noyalib-mcp` | 0.0.61 | `3a07fd7060648cdfda6bc68b006732daa537d2ef` | dirty |
| `noyalib-wasm` | 0.0.61 | `11c024559e7e380ba3c35055422d9ccfa7d0cb85` | dirty |
| `noyalib-serde-yaml` | 0.0.61 | `57dea34e9ee5b00ee20d2b86531e817c801089da` | dirty |

| Repo | Metric | Measured | Score | Probe |
|---|---|---|---:|---|
| `noyalib` | tests | 7298 passed / 0 failed | 1.00 | `cargo test --workspace --locked --all-features` |
| `noyalib` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --all-features -- -D warnings` |
| `noyalib` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib` | unsafe_forbidden | 2/2 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib` | msrv_declared | 1.89.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib` | missing_docs_lint | 2/2 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib` | bench_methodology | 3/3 disclosed (host, toolchain, command) | 1.00 | `grep host/toolchain/repro-command in docs/BENCHMARKS.md` |
| `noyalib` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json \| .vulnerabilities.count` |
| `noyalib` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noyalib` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib` | dependency_closure | 11 unique runtime crates | 1.00 | `cargo tree -e normal --prefix none --no-dedupe \| sort -u (library budget 60; propagates to consumers)` |
| `noyalib` | actions_sha_pinned | 183/183 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib` | fuzz_targets | 18 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib` | line_coverage | 95.2594% lines | 1.00 | `cargo +nightly llvm-cov --json --summary-only (lines; target 95%)` |
| `noyalib` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noyalib` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noyalib` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noyalib` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib/dependabot/alerts?state=open` |
| `noyalib` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noyalib` | crates_io_current | tree 0.0.61 / crates.io 0.0.60 | 0.00 | `crates.io/api/v1/crates/noyalib .max_stable_version` |
| `noyalib` | openssf_scorecard | 8.3/10 | 0.83 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib .score` |
| `noya-cli` | tests | 123 passed / 0 failed | 1.00 | `cargo test --workspace --locked --all-features` |
| `noya-cli` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --all-features -- -D warnings` |
| `noya-cli` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noya-cli` | unsafe_forbidden | 1/1 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noya-cli` | msrv_declared | 1.89.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noya-cli` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noya-cli` | missing_docs_lint | 1/1 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noya-cli` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noya-cli` | bench_methodology | 1/3 disclosed (host, toolchain, command) | 0.33 | `grep host/toolchain/repro-command in docs/BENCHMARKS.md` |
| `noya-cli` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json \| .vulnerabilities.count` |
| `noya-cli` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noya-cli` | vet_audited | fail | 0.00 | `cargo vet --locked` |
| `noya-cli` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noya-cli` | dependency_closure | 104 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe \| sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noya-cli` | actions_sha_pinned | 85/85 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noya-cli` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noya-cli` | line_coverage | 97.3016% lines | 1.00 | `cargo +nightly llvm-cov --json --summary-only (lines; target 95%)` |
| `noya-cli` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noya-cli` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noya-cli` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noya-cli` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noya-cli/dependabot/alerts?state=open` |
| `noya-cli` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noya-cli` | crates_io_current | tree 0.0.61 / crates.io 0.0.60 | 0.00 | `crates.io/api/v1/crates/noya-cli .max_stable_version` |
| `noya-cli` | openssf_scorecard | 8.3/10 | 0.83 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noya-cli .score` |
| `noyalib-lsp` | tests | 82 passed / 0 failed | 1.00 | `cargo test --workspace --locked --all-features` |
| `noyalib-lsp` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --all-features -- -D warnings` |
| `noyalib-lsp` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib-lsp` | unsafe_forbidden | 2/2 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib-lsp` | msrv_declared | 1.89.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib-lsp` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib-lsp` | missing_docs_lint | 2/2 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib-lsp` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib-lsp` | bench_methodology | 1/3 disclosed (host, toolchain, command) | 0.33 | `grep host/toolchain/repro-command in docs/BENCHMARKS.md` |
| `noyalib-lsp` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json \| .vulnerabilities.count` |
| `noyalib-lsp` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib-lsp` | vet_audited | fail | 0.00 | `cargo vet --locked` |
| `noyalib-lsp` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib-lsp` | dependency_closure | 70 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe \| sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-lsp` | actions_sha_pinned | 70/70 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-lsp` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-lsp` | line_coverage | 94.7% lines | 1.00 | `cargo +nightly llvm-cov --json --summary-only (lines; target 95%)` |
| `noyalib-lsp` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noyalib-lsp` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noyalib-lsp` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noyalib-lsp` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-lsp/dependabot/alerts?state=open` |
| `noyalib-lsp` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noyalib-lsp` | crates_io_current | tree 0.0.61 / crates.io 0.0.60 | 0.00 | `crates.io/api/v1/crates/noyalib-lsp .max_stable_version` |
| `noyalib-lsp` | openssf_scorecard | 8.3/10 | 0.83 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-lsp .score` |
| `noyalib-mcp` | tests | 124 passed / 0 failed | 1.00 | `cargo test --workspace --locked --all-features` |
| `noyalib-mcp` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --all-features -- -D warnings` |
| `noyalib-mcp` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib-mcp` | unsafe_forbidden | 2/2 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib-mcp` | msrv_declared | 1.89.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib-mcp` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib-mcp` | missing_docs_lint | 2/2 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib-mcp` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib-mcp` | bench_methodology | 1/3 disclosed (host, toolchain, command) | 0.33 | `grep host/toolchain/repro-command in docs/BENCHMARKS.md` |
| `noyalib-mcp` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json \| .vulnerabilities.count` |
| `noyalib-mcp` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib-mcp` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noyalib-mcp` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib-mcp` | dependency_closure | 141 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe \| sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-mcp` | actions_sha_pinned | 85/85 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-mcp` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-mcp` | line_coverage | 98.2593% lines | 1.00 | `cargo +nightly llvm-cov --json --summary-only (lines; target 95%)` |
| `noyalib-mcp` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noyalib-mcp` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noyalib-mcp` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noyalib-mcp` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-mcp/dependabot/alerts?state=open` |
| `noyalib-mcp` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noyalib-mcp` | crates_io_current | tree 0.0.61 / crates.io 0.0.59 | 0.00 | `crates.io/api/v1/crates/noyalib-mcp .max_stable_version` |
| `noyalib-mcp` | openssf_scorecard | 8.2/10 | 0.82 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-mcp .score` |
| `noyalib-wasm` | tests | 57 passed / 0 failed | 1.00 | `cargo test --workspace --locked --all-features` |
| `noyalib-wasm` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --all-features -- -D warnings` |
| `noyalib-wasm` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib-wasm` | unsafe_forbidden | 1/1 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib-wasm` | msrv_declared | 1.89.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib-wasm` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib-wasm` | missing_docs_lint | 1/1 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib-wasm` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib-wasm` | bench_methodology | 0/3 disclosed (host, toolchain, command) | 0.00 | `grep host/toolchain/repro-command in docs/BENCHMARKS.md` |
| `noyalib-wasm` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json \| .vulnerabilities.count` |
| `noyalib-wasm` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib-wasm` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noyalib-wasm` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib-wasm` | dependency_closure | 32 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe \| sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-wasm` | actions_sha_pinned | 73/73 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-wasm` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-wasm` | line_coverage | 73.1441% lines | 0.77 | `cargo +nightly llvm-cov --json --summary-only (lines; target 95%)` |
| `noyalib-wasm` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noyalib-wasm` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noyalib-wasm` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noyalib-wasm` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-wasm/dependabot/alerts?state=open` |
| `noyalib-wasm` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noyalib-wasm` | crates_io_current | tree 0.0.61 / crates.io 0.0.60 | 0.00 | `crates.io/api/v1/crates/noyalib-wasm .max_stable_version` |
| `noyalib-wasm` | openssf_scorecard | 8.3/10 | 0.83 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-wasm .score` |
| `noyalib-serde-yaml` | tests | 6 passed / 0 failed | 1.00 | `cargo test --workspace --locked --all-features` |
| `noyalib-serde-yaml` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked --all-features -- -D warnings` |
| `noyalib-serde-yaml` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib-serde-yaml` | unsafe_forbidden | 1/1 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib-serde-yaml` | msrv_declared | 1.89.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib-serde-yaml` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib-serde-yaml` | missing_docs_lint | 1/1 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib-serde-yaml` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib-serde-yaml` | bench_methodology | 1/3 disclosed (host, toolchain, command) | 0.33 | `grep host/toolchain/repro-command in docs/BENCHMARKS.md` |
| `noyalib-serde-yaml` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json \| .vulnerabilities.count` |
| `noyalib-serde-yaml` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib-serde-yaml` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noyalib-serde-yaml` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib-serde-yaml` | dependency_closure | 9 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe \| sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-serde-yaml` | actions_sha_pinned | 60/60 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-serde-yaml` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-serde-yaml` | line_coverage | unmeasured (rc=0 or invalid/empty JSON) | n/a | `cargo +nightly llvm-cov --json --summary-only; failed/empty reports are N/A` |
| `noyalib-serde-yaml` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noyalib-serde-yaml` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noyalib-serde-yaml` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noyalib-serde-yaml` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-serde-yaml/dependabot/alerts?state=open` |
| `noyalib-serde-yaml` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noyalib-serde-yaml` | crates_io_current | tree 0.0.61 / crates.io 0.0.60 | 0.00 | `crates.io/api/v1/crates/noyalib-serde-yaml .max_stable_version` |
| `noyalib-serde-yaml` | openssf_scorecard | 7.4/10 | 0.74 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-serde-yaml .score` |
| `ecosystem` | version_lockstep | core 0.0.61 not tagged yet | n/a | `git tag -l v0.0.61 in noyalib (pending release; satellites pin only published versions)` |
| `ecosystem` | host_coverage | 6/6 hosts present | 1.00 | `directory presence for noyalib noya-cli noyalib-lsp noyalib-mcp noyalib-wasm noyalib-serde-yaml` |

<sub>Recorded by the scorecard harness v1.0.0. Render the saved JSON rather than editing the table.</sub>
<!-- SCORECARD:END -->

</details>

## Family layout

The shared docs workflow enforces the family skeleton. Repository-specific
source and delivery directories remain owned by their repository.

| Surface | Maintained evidence |
|---|---|
| Identity and support | README, CHANGELOG, SECURITY, CONTRIBUTING, DEVELOPMENT, governance and support files |
| Contributor rules | `AGENTS.md`, signed commits/tags, exact lockstep version gate |
| Licensing | `REUSE.toml`, inline SPDX declarations and license texts |
| Documentation | `docs/`, architecture manual, mdBook summary and link checks |
| Development | Editor/lint configs, pre-commit config and devcontainer |
| Regression and supply chain | Fuzz corpora, audit/deny/vet configuration and current workflow evidence |
| Core shared gates | Reusable workflows pinned by satellite repos to reviewed source commits |

[ADR-0005](adr/0005-workspace-split.md) controls crate ownership and versioning.
[PLAN.md](PLAN.md) controls the current implementation work order. The
[assessment](ROADMAP-TO-10.md) is updated only when its acceptance evidence
changes; historical grades are not carried forward as current facts.
