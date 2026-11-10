<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# The noyalib ecosystem

The ecosystem has six lockstep crate repositories sharing one parser and
editing engine. This inventory was reconciled on **2026-11-10**. The current
[eleven-category assessment](ROADMAP-TO-10.md) rates each repo separately;
[PLAN.md](PLAN.md) names the implementation work and acceptance evidence
needed to reach 10/10. The dated
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
its stale no_std claim is tracked as SHIM2 in the plan. Adding a feature to
change that contract requires maintainer direction.

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
| WASM | `noyalib-wasm` 0.0.60 | npm `@sebastienrousseau/noyalib-wasm` with provenance | Actual export-map, TypeScript, browser and bundler consumers |
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

Known measurement work is explicit in Q3 of the plan: line coverage currently
uses the last `TOTAL` field, which can be a branch column; release asset
presence is weaker than cryptographic identity and a real install; fuzz
target counts do not establish sustained campaigns. Current coverage in the
assessment was read from the actual CI column headers, separately from this
parser. The external OpenSSF API was unavailable, so no fresh aggregate
OpenSSF rating is asserted.

### Historical automated report

The generated block below is retained exactly as the **2026-11-09** report,
not regenerated or relabelled as a current six-repo release assessment.
Its 93.8% over executed probes is not equivalent to 9.38/10 across all eleven
engineering categories. `docs/scorecard.json` is also a historical artifact;
it is not the new assessment's source of truth. For current source identities
and findings use the dated evidence JSON linked above.

<details>
<summary>Show the historical automated probe report</summary>

<!-- SCORECARD:BEGIN — generated by scripts/ecosystem-scorecard.sh. Regenerate rather than edit. -->

**Rating: A (93.8% weighted)** — 134 executed probes, 12 scored N/A.

Measured 2026-11-09T21:35:37Z on `aarch64-apple-darwin`, rustc 1.99.0 (b940084d7 2026-09-28). Rubric executed: 92%.

| Repo | Metric | Measured | Score | Probe |
|---|---|---|---:|---|
| `noyalib` | tests | 6704 passed / 0 failed | 1.00 | `cargo test --workspace --locked` |
| `noyalib` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
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
| `noyalib` | actions_sha_pinned | 175/175 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib` | fuzz_targets | 18 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noyalib` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noyalib` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noyalib` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noyalib` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib/dependabot/alerts?state=open` |
| `noyalib` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noyalib` | crates_io_current | tree 0.0.60 / crates.io 0.0.59 | 0.00 | `crates.io/api/v1/crates/noyalib .max_stable_version` |
| `noyalib` | openssf_scorecard | 8.3/10 | 0.83 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib .score` |
| `noya-cli` | tests | 122 passed / 0 failed | 1.00 | `cargo test --workspace --locked` |
| `noya-cli` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| `noya-cli` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noya-cli` | unsafe_forbidden | 1/1 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noya-cli` | msrv_declared | 1.89.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noya-cli` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noya-cli` | missing_docs_lint | 1/1 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noya-cli` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noya-cli` | bench_methodology | 1/3 disclosed (host, toolchain, command) | 0.33 | `grep host/toolchain/repro-command in docs/BENCHMARKS.md` |
| `noya-cli` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json \| .vulnerabilities.count` |
| `noya-cli` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noya-cli` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noya-cli` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noya-cli` | dependency_closure | 105 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe \| sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noya-cli` | actions_sha_pinned | 77/77 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noya-cli` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noya-cli` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noya-cli` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noya-cli` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noya-cli` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noya-cli` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noya-cli/dependabot/alerts?state=open` |
| `noya-cli` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noya-cli` | crates_io_current | tree 0.0.58 / crates.io 0.0.58 | 1.00 | `crates.io/api/v1/crates/noya-cli .max_stable_version` |
| `noya-cli` | openssf_scorecard | 8.3/10 | 0.83 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noya-cli .score` |
| `noyalib-lsp` | tests | 75 passed / 0 failed | 1.00 | `cargo test --workspace --locked` |
| `noyalib-lsp` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| `noyalib-lsp` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib-lsp` | unsafe_forbidden | 2/2 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib-lsp` | msrv_declared | 1.89.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib-lsp` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib-lsp` | missing_docs_lint | 2/2 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib-lsp` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib-lsp` | bench_methodology | 1/3 disclosed (host, toolchain, command) | 0.33 | `grep host/toolchain/repro-command in docs/BENCHMARKS.md` |
| `noyalib-lsp` | audit_vulnerabilities | 0 advisories | 1.00 | `cargo-audit audit --json \| .vulnerabilities.count` |
| `noyalib-lsp` | deny_check | pass | 1.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib-lsp` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noyalib-lsp` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib-lsp` | dependency_closure | 71 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe \| sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-lsp` | actions_sha_pinned | 62/62 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-lsp` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-lsp` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noyalib-lsp` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noyalib-lsp` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noyalib-lsp` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noyalib-lsp` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-lsp/dependabot/alerts?state=open` |
| `noyalib-lsp` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noyalib-lsp` | crates_io_current | tree 0.0.58 / crates.io 0.0.58 | 1.00 | `crates.io/api/v1/crates/noyalib-lsp .max_stable_version` |
| `noyalib-lsp` | openssf_scorecard | 8.3/10 | 0.83 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-lsp .score` |
| `noyalib-mcp` | tests | 124 passed / 0 failed | 1.00 | `cargo test --workspace --locked` |
| `noyalib-mcp` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| `noyalib-mcp` | rustfmt | clean | 1.00 | `cargo fmt --all --check` |
| `noyalib-mcp` | unsafe_forbidden | 2/2 crate roots | 1.00 | `grep -l 'forbid(unsafe_code)' on every lib.rs/main.rs` |
| `noyalib-mcp` | msrv_declared | 1.89.0 | 1.00 | `grep rust-version Cargo.toml` |
| `noyalib-mcp` | rustdoc_strict | 0 warnings | 1.00 | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features (matches CI + docs.rs)` |
| `noyalib-mcp` | missing_docs_lint | 2/2 roots deny missing_docs | 1.00 | `grep for deny/warn(missing_docs) in crate roots` |
| `noyalib-mcp` | readme_examples | compile | 1.00 | `scripts/check-readme-examples.sh` |
| `noyalib-mcp` | bench_methodology | 1/3 disclosed (host, toolchain, command) | 0.33 | `grep host/toolchain/repro-command in docs/BENCHMARKS.md` |
| `noyalib-mcp` | audit_vulnerabilities | 1 advisories | 0.00 | `cargo-audit audit --json \| .vulnerabilities.count` |
| `noyalib-mcp` | deny_check | fail | 0.00 | `cargo deny check (advisories, bans, licenses, sources)` |
| `noyalib-mcp` | vet_audited | pass | 1.00 | `cargo vet --locked` |
| `noyalib-mcp` | reuse_compliance | compliant | 1.00 | `reuse lint (REUSE 3.3)` |
| `noyalib-mcp` | dependency_closure | 134 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe \| sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-mcp` | actions_sha_pinned | 74/74 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-mcp` | fuzz_targets | 1 libFuzzer targets | 0.50 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-mcp` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noyalib-mcp` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noyalib-mcp` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noyalib-mcp` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noyalib-mcp` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-mcp/dependabot/alerts?state=open` |
| `noyalib-mcp` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noyalib-mcp` | crates_io_current | tree 0.0.58 / crates.io 0.0.58 | 1.00 | `crates.io/api/v1/crates/noyalib-mcp .max_stable_version` |
| `noyalib-mcp` | openssf_scorecard | 8.2/10 | 0.82 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-mcp .score` |
| `noyalib-wasm` | tests | 57 passed / 0 failed | 1.00 | `cargo test --workspace --locked` |
| `noyalib-wasm` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
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
| `noyalib-wasm` | dependency_closure | 33 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe \| sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-wasm` | actions_sha_pinned | 65/65 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-wasm` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-wasm` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noyalib-wasm` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noyalib-wasm` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noyalib-wasm` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noyalib-wasm` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-wasm/dependabot/alerts?state=open` |
| `noyalib-wasm` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noyalib-wasm` | crates_io_current | tree 0.0.58 / crates.io 0.0.58 | 1.00 | `crates.io/api/v1/crates/noyalib-wasm .max_stable_version` |
| `noyalib-wasm` | openssf_scorecard | 8.2/10 | 0.82 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-wasm .score` |
| `noyalib-serde-yaml` | tests | 6 passed / 0 failed | 1.00 | `cargo test --workspace --locked` |
| `noyalib-serde-yaml` | clippy | 0 diagnostics (rc=0) | 1.00 | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
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
| `noyalib-serde-yaml` | dependency_closure | 10 unique runtime crates (leaf, not scored) | n/a | `cargo tree -e normal --prefix none --no-dedupe \| sort -u — recorded, not scored: a leaf binary's tree is not inherited` |
| `noyalib-serde-yaml` | actions_sha_pinned | 52/52 external uses pinned | 1.00 | `grep 'uses:.*@<40-hex>' .github/workflows` |
| `noyalib-serde-yaml` | fuzz_targets | 2 libFuzzer targets | 1.00 | `find fuzz/fuzz_targets -name '*.rs' (floor 2)` |
| `noyalib-serde-yaml` | line_coverage | not run | n/a | `pass --with-coverage to measure` |
| `noyalib-serde-yaml` | release_gpg_signed | 3 .asc assets | 1.00 | `gh release view --json assets \| grep '\.asc$'` |
| `noyalib-serde-yaml` | release_sigstore | 3 .bundle assets | 1.00 | `gh release view --json assets \| grep '\.bundle$'` |
| `noyalib-serde-yaml` | release_sbom | 6 sbom assets | 1.00 | `gh release view --json assets \| grep -i sbom` |
| `noyalib-serde-yaml` | dependabot_open | 0 open alerts | 1.00 | `gh api /repos/sebastienrousseau/noyalib-serde-yaml/dependabot/alerts?state=open` |
| `noyalib-serde-yaml` | ci_main_green | success | 1.00 | `gh run list --branch main --workflow CI --status completed --limit 1 --json conclusion` |
| `noyalib-serde-yaml` | crates_io_current | tree 0.0.58 / crates.io 0.0.58 | 1.00 | `crates.io/api/v1/crates/noyalib-serde-yaml .max_stable_version` |
| `noyalib-serde-yaml` | openssf_scorecard | 7.3/10 | 0.73 | `api.securityscorecards.dev/projects/github.com/sebastienrousseau/noyalib-serde-yaml .score` |
| `ecosystem` | version_lockstep | core 0.0.60 not tagged yet | n/a | `git tag -l v0.0.60 in noyalib (pending release; satellites pin only published versions)` |
| `ecosystem` | host_coverage | 6/6 hosts present | 1.00 | `directory presence for noyalib noya-cli noyalib-lsp noyalib-mcp noyalib-wasm noyalib-serde-yaml` |

<sub>Generated by `scripts/ecosystem-scorecard.sh` v1.0.0. Regenerate rather than edit.</sub>
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
