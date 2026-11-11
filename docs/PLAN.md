<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Ecosystem implementation plan

Updated **2026-11-11**. Goal: close the measured gaps in all six repositories
so every category can earn a defensible, maintained 10/10 within its declared
product scope. The [assessment](ROADMAP-TO-10.md) gives the baseline and the
[evidence snapshot](ecosystem-assessment-2026-11-10.json) pins its sources.
This plan replaces the historical monorepo/`xtask` production-readiness plan;
its history remains in Git.

Implementation is in progress on the single future `feat/v0.0.61` branch.
The dated assessment remains the baseline. The
[contract ledger](ECOSYSTEM-CONTRACTS.md) maps supported behavior to its
tests, and the [implementation record](IMPLEMENTATION-STATUS.md) tracks
observed results and open criteria. Publishing remains held.

Daybreak access is outside this plan and is not an acceptance prerequisite.
Continue the local and GitHub gates; retain G1 as an actual independent
human review requirement through an available channel.

The November 11 follow-up closes the complete hosted alloc test-harness gap,
fixes terminal-block-scalar formatting and typed anchored empty keys, and expands actual
six-consumer parity to eleven cases. The three integrated dependency PRs
are closed with credit, and the hosted strict MCP Inspector check passes.
The [implementation record](IMPLEMENTATION-STATUS.md) retains test scopes,
new oracle findings and the remaining acceptance work. Daybreak remains
outside the acceptance criteria.

The CI correction checkpoint passes the unchanged package and complexity
budgets, the installed WASM browser consumer, CodeQL and all six actual
consumer boundaries in GitHub. The latest scanner correction rejects unindented mapping values through
all three loader paths, and twelve shared cases pass through all six products.
The follow-up also corrects explicit typed-key spelling on AST fallback.
At the explicit-key checkpoint, nine new fuzz inputs have failing-first
tests and scoped sanitizer replay evidence. Complete std, baseline alloc and
portable batteries pass 6,745, 6,723 and 7,064 tests; all twelve bare-metal
checks pass. Later campaigns expose plain-scalar tab indentation and a
canonical-duplicate typed-key mismatch. The signed continuation correction and formatter repair pass 6,748 std,
6,726 baseline alloc and 7,067 portable tests. The
[duplicate-key decision](adr/0019-typed-mapping-duplicate-identity.md) remains
proposed; its finding is retained and its contract choice is pending. A
further development-only empty-line tab repair passes all 153 properties,
eighteen alloc oracle tests and original-input replay. Production source
and Cargo declarations remain unchanged. Campaign accounting now
separates completed totals from lower bounds and unknown measurements. The
[CI correction evidence](ecosystem-ci-repair-evidence-2026-11-11.json)
records the exact source scopes and raw log hashes. Sustained campaigns
and independent review remain acceptance work.

## Completion campaign

The renewed request is to finish every acceptance row. Work proceeds in
this order on the existing future branches:

1. Close reproduced parser/oracle findings and retain full-value checks.
   Run the exact-source fuzz inventory and fix failures before crediting it.
2. Add resource gates using actual product boundaries, beginning with WASM
   parse/edit/free memory growth. A real retained-document leak must fail
   the same gate that passes repeated normal runs.
3. Complete remaining protocol, filesystem, diagnostics and consumer cases;
   obtain fresh target/OS, package, recovery and performance receipts.
4. Consolidate the review packet and resolve the duplicate-key contract,
   held MCP source decision, independent reviewer and continuity owner.
5. Reassess all 66 category/repository cells against recorded evidence.
   Keep an unmet criterion open until its observation exists.

Hosted checkpoint `a346c3e` has eighty successful checks and ten conditional
skips. Subsequent changes require their own CI receipt. The WASM resource
and runtime-contract work now has signed implementation and local
[assurance evidence](ecosystem-wasm-assurance-2026-11-11.json). The remaining
human review has no recorded reviewer yet; a reviewer identity is requested
while independent technical work continues.

## Constraints and work order

The live values are edition 2024, `rust-version = "1.89.0"`.

- Six crates move by exactly +0.0.1, with identical versions and exact core
  pins, under [ADR-0005](adr/0005-workspace-split.md).
- At most two version branches exist. Finish or explicitly resolve the
  current release, check merged stale branch refs, then use the single
  future branch for sequenced commits. Do not create parallel task branches.
- Preserve signed commits and tags. Never move the existing MCP v0.0.60 tag
  to include a later correction. Commit or stash all files before switching.
- Preserve Rust 1.89.0, public APIs, feature semantics and the core's complete
  documented alloc-only/no_std behavior. Applications and JS bindings may
  require std; represent this accurately in their support matrices.
- Retain unconditional `forbid(unsafe_code)`. Keep loader parity, bounded
  resource behavior and unchanged-on-failed-edit guarantees.
- Every behavior fix includes the reproducer and observed red/green evidence
  in its signed commit. Characterise behavior before structural refactoring.
- Keep CI green in the session that changed it. Do not lower floors, weaken
  audit criteria, add blanket exemptions, omit targets or inflate trivial
  assertions to improve a score. Coverage work targets meaningful reachable
  production paths, not a universal 100% floor.
- API, feature, MSRV or access-policy changes remain maintainer decisions.
  New query/lint binaries, extra crates, bindings and protocol capabilities
  are optional product proposals outside this assurance programme.

Tasks are sized as independently reviewable changes. **S** means roughly
half to one focused engineering day, **M** one to three, **L** three to five.
These are planning estimates, excluding CI queues, review and external
service delays; they are not release dates. Split an L task if its acceptance
cannot be demonstrated in one review. Owner names below are roles, not
claims that another maintainer has already agreed to work.

Dependency order:

1. P0: complete the security/source decision and reconcile critical claims.
2. P1: repair measurement gaps and establish contracts and baselines.
3. P2: implement boundary regressions, portability and interoperability checks.
4. P3: retain consumer/recovery receipts and establish independent review.
5. Reassess each category at the resulting commits; close only verified work.

Independent tasks may be sequenced in either order on the same future branch.
A failed acceptance check leaves its task open and blocks dependent work.

## P0: release hold and factual blockers

| ID | Repository / owner | Action and observable done-condition | Verification / artifact | Depends; effort; risk |
|---|---|---|---|---|
| S1 | All / security maintainer | Enumerate every Cargo manifest and lockfile, fuzz and standalone test crates, Node lockfiles and generated wrapper inputs. Scan each scope; classify every finding by package, version, source and affected use. | Commit-bound inventory plus `cargo audit`, `cargo deny check`, `cargo vet` for each supported Cargo workspace; wrapper audit reports. Unsupported checks state why and retain the alternative evidence. No scope silently omitted. | None; M; medium |
| R0 | MCP / release maintainer | Resolve alert 25 and the immutable held tag before publication. Record whether the original tagged source can be released with a justified development-only finding, or requires a separately authorized recovery. Any changed-source proposal must satisfy signing, version and tag rules before execution. | Written source/ref decision referencing `b28d529f8bae59bbe94bdd91d25ec50173c8702b`, scan scope and production reachability evidence. Run 42897537938 remains cancelled until this decision and release gates are satisfied. | S1; S; high |
| MCP1 | MCP / security maintainer | Update affected Tokio 1.53.1 in `fuzz/Cargo.lock` through Cargo to a patched compatible release; review its dependency delta and audit evidence. Preserve production SDK 3.9.0 and registry const-oid 0.10.3 unless evidence requires another change. | Fresh advisory scan of both root and fuzz locks, `cargo +nightly fuzz build`, replay existing corpus, relevant tests and source verification. New scan of corrected ref resolves alert 25; no manual false-positive dismissal. | S1; S; medium |
| D1 | All / documentation maintainer | Reconcile public claims against source and actual package contents. Remove unsupported speed/size and channel assertions or attach reproducible evidence. Correct stale monorepo paths and distinguish historical reports from current evidence. | Claim ledger with source/test/consumer link per retained claim; docs examples, links, spelling, Markdown and version gates pass. | None; M; low |
| LSP1 | LSP / editor maintainer | Align README capability claims and protocol coverage with actual dispatch and initialize response. Replace raw unframed echo guidance with an executable framed client example. Missing completion/symbol features remain explicitly unsupported pending a separate product decision. | Capability table derived from tested `initialize` and method responses; documented framed handshake runs against installed server. No unapproved new API. | D1; S; low |
| SHIM2 | Facade / compatibility maintainer | Correct `docs/CONTRACT.md` no_std claim and old workspace paths. Clearly distinguish the std-requiring facade from core alloc-only `compat-serde-yaml`. | Manifest dependency graph and executed consumer examples agree with docs; core compatibility bare-metal checks remain green. | D1; S; low |

The current MCP high alert matches an affected development lockfile; the
production lock uses patched Tokio 1.54.1. This is not an established
production exploit. S1/R0 must retain that distinction while resolving the
finding. The existing signed source cannot be silently replaced to make a
scan green.

## P1: trustworthy measurements and contracts

| ID | Repository / owner | Action and observable done-condition | Verification / artifact | Depends; effort; risk |
|---|---|---|---|---|
| Q1 | All / QA maintainer | Create capability-to-test ledgers: each advertised operation, limit and refusal links to a test at the product boundary. Record unsupported combinations and error contracts. | Source/API inventory compared with test names and actual execution; every advertised row has positive, negative and boundary evidence or an open task. | D1, LSP1, SHIM2; M; low |
| Q2 | All / QA maintainer | Add sustained, bounded fuzz campaigns beyond seed replay, appropriate properties and minimized regression inputs. MCP adds transport/framing/state coverage instead of duplicate tool targets. Retain production-path coverage gap reports. | Per-target elapsed time, executed cases, corpus digest, crash triage and replay links for an initial campaign of at least one hour per target; repeatable scheduled campaign thereafter. Existing floors remain intact; justified exclusions listed. | Q1, S1, MCP1 for MCP; L per repo; medium |
| Q3 | Core harness / tooling maintainer | Parse line coverage from structured llvm-cov data or verified headers. Current `$(NF)` parsing can read branch coverage. Fix stale report provenance and keep N/A visible. | Regression fixtures cover modern headers, branch columns, empty facade totals, missing tools, timeout and dirty/mixed-SHA checkouts. Expected line/function/region values differ deliberately; test fails on old parser. Generated report records exact sources and executed rubric fraction. | None; M; medium |
| P1 | All / performance maintainer | Capture baselines on controlled hardware: corpus hashes, source and dependency commits, features, CPU/OS/toolchains, repetitions and variance. Measure throughput, p50/p95 latency where relevant, peak memory and artifact sizes. | Raw machine-readable results plus reproducible commands and summaries; same-machine repeat establishes noise. Core historical tables stay dated until replaced by new results. | Q1; M per repo; low |
| P2 | All / performance maintainer | Set justified scenario budgets from measured baselines and user needs; make meaningful regressions detectable without a noisy shared-runner timing gate. | A controlled injected regression breaches the gate; unchanged-source repeats remain within measured noise. Use stable instruction/allocation/size checks where appropriate and dedicated timing hardware for latency gates. Document variance and review process for budget changes. | P1; M per repo; medium |
| X1 | All / developer experience maintainer | Exercise fresh clone, documented setup, local gates, install, intentional failure, recovery and uninstall. Audit diagnostic actions and exit/error contracts instead of adding convenience API just for a score. | Clean-environment transcripts on declared OSes; each documentation command resolves; error matrix includes malformed input, unavailable files, budget exhaustion and failed edits with unchanged data. | Q1; M per repo; low |

A target count, benchmark file or nominal coverage floor cannot close Q2 or
P1. Do not create an independent facade coverage denominator by introducing
runtime code solely for measurement. For WASM, native llvm-cov results and
actual browser/JS boundary evidence remain separate.

## P2: repository implementation work

### Core

| ID | Owner | Change and done-condition | Verification / artifact | Depends; effort; risk |
|---|---|---|---|---|
| C1 | Core QA maintainer | Complete parser, typed/Value, borrowed, no-span, streaming and CST contract ledgers. Add missing parity, diagnostic and failed-edit refusal cases without changing intentional loader roles. | New counterexamples fail on the prior source and pass after any fix; cross-loader properties, conformance and mutator oracles agree within their documented domains. | Q1; L; high |
| C2 | Core architecture maintainer | Review loader invariants, CST/scanner coupling, cache and failure ownership, using current architecture and complexity evidence. Refactor only demonstrated maintenance problems, in separate commits from behavior fixes. | Reviewed boundary map and decisions; characterisation suites and complexity gate before/after each extraction; no loader unification based only on file length. | C1; M; medium |
| C3 | Core portability maintainer | Maintain feature/target/toolchain ledger for default, minimal std, alloc-only, optional no_std-compatible features and required-std features. Pin std/libm graph expectations and float/diagnostic parity. | Isolated actual registry consumers on Rust 1.89.0 and stable; supported bare-metal checks; behavior tests on alloc-only host; negative tests reject accidental std/libm edges. Each supported feature composition has evidence. | Q1; M; high |

The current core contract includes Serde, values, CST editing and supported
feature compositions in alloc-only builds, including required floating-point
behavior. It is not satisfied by a bare `cargo check` alone. Async, filesystem
and other deliberately std-requiring integrations remain documented as such.
Unsupported combinations receive a clear support statement, not a promise
that operating-system functionality exists on bare metal.

### CLI

| ID | Owner | Change and done-condition | Verification / artifact | Depends; effort; risk |
|---|---|---|---|---|
| CLI1 | CLI QA maintainer | Complete process-level tests for both shipped binaries: flags, stdin/stdout, stable exit codes, multi-file handling, check/write/repair, JSON/SARIF only where actually supported, hooks, Action and generated assets. | Installed binary transcripts and regression inputs, Action self-test, hook run, manpage/completion drift check, permissions and read-only/error cases on declared OSes. | Q1; M; medium |
| CLI2 | CLI I/O maintainer | Review atomic replacement and path/permission handling; add interruption, failed-write and platform-specific replacement cases. Preserve original bytes on failure. | Filesystem boundary tests demonstrate original data and metadata behavior; injected failures produce actionable diagnostics and no partial write. Independent review retained. | CLI1, S1; M; high |

### LSP

| ID | Owner | Change and done-condition | Verification / artifact | Depends; effort; risk |
|---|---|---|---|---|
| LSP2 | LSP protocol maintainer | Exercise lifecycle, UTF-16 positions, CRLF, malformed/oversized framing, duplicate/stale changes, shutdown and cache invalidation at real stdio boundary. Fix only demonstrated mismatches to declared LSP scope. | Framed subprocess transcripts and minimized inputs; capability response matches actual dispatch; diagnostics target the intended Unicode ranges. Resource and latency budgets measured. | LSP1, Q1; L; high |
| LSP3 | Editor integration maintainer | Test the installed VS Code extension and one documented independent LSP client; verify startup binary selection, diagnostics, formatting and hover. Reconcile every claimed marketplace separately. | Extension-host smoke tests plus versioned client/OS matrix and registry-installed package checks. A built VSIX alone does not close this task. | LSP2, X1; M; medium |

### MCP

| ID | Owner | Change and done-condition | Verification / artifact | Depends; effort; risk |
|---|---|---|---|---|
| MCP2 | MCP protocol/security maintainer | Build revision-by-transport capability/security matrix for existing stdio, streamable HTTP and legacy SSE. Test malformed framing, cancellation, disconnect, session isolation, token/origin handling where applicable, root/symlink confinement, bounded concurrent work and failed edits. | Pinned SDK and real client/Inspector versions with transcripts; stateless and legacy requests, negative authorization cases, supported-OS filesystem tests and transport fuzz regressions. Unsupported methods/extensions explicit. Independent security review of guards and writes. | MCP1, Q1; L; high |
| MCP3 | MCP packaging maintainer | Test installed npm bootstrap cold/cache/offline/error paths and Cargo/container/MCP Registry consumers. Include license files and artifact identity for the held v0.0.60 only after R0 clears it. | npm package bytes, signature/provenance verification, exact downloaded binary checksum, stdio parse/edit smoke and registry/container identity receipts. Reject unsupported hosts and corrupted cache/downloads predictably. | R0, MCP2, X1; M; medium |

OAuth, new extension support, Tasks or a larger tool API are not assumed
necessary for every existing transport. Adopt capabilities only through
explicit product scope and threat-model decisions; test the advertised
supported revision and transport combinations.

### WASM

| ID | Owner | Change and done-condition | Verification / artifact | Depends; effort; risk |
|---|---|---|---|---|
| WASM1 | WASM boundary maintainer | Complete exported API/value/error contract tests: numeric boundaries, tags/non-string keys, cycles/shared references, depth/element limits, byte vs JS positions and document lifetime/failure behavior. | Run actual wasm-bindgen exports under Node and a headless browser; compile generated TypeScript; failed mutations preserve source and resources can be released. Native coverage is supplementary. | Q1; L; high |
| WASM2 | WASM performance maintainer | Measure generated raw/compressed package and module bytes, cold initialization, warmed calls and memory growth across representative workloads. | Actual package measurements with tool versions, corpus and reproducible JS harness; justified byte/memory budgets and regression signal. Remove unverified 180 KB/70 KB claims until backed by those results. | WASM1, P1; M; medium |
| WASM3 | JS packaging maintainer | Correct unscoped npm examples to the actual published scope; reconcile `exports`/files/declarations with docs and claimed ESM/CJS/web/bundler/runtime support. Test each retained claim from the installed npm tarball; document unavailable subpaths. | Node, headless browser and declared bundler builds execute parse, stringify and lossless editing. Pin tested runtime versions; add Bun/Deno only if claimed, otherwise mark unverified. | WASM1, D1; M; medium |

### Compatibility facade

| ID | Owner | Change and done-condition | Verification / artifact | Depends; effort; risk |
|---|---|---|---|---|
| SHIM1 | Compatibility maintainer | Extend the 18-case captured contract into a documented supported API/type/error matrix, realistic Cargo-rename consumers, tags/numbers/merge keys and intentional differences. Guard re-export drift and measure overhead on equivalent semantics. | Downstream compile/run suites, location/error snapshots, differential minimized inputs with justified differences, no archived crate in runtime graph and a matching direct-core baseline. Core owns implementation fixes; facade stays thin. | SHIM2, Q1; L; high |

## P3: shared integration, supply chain and sustainability

| ID | Repository / owner | Change and observable done-condition | Verification / artifact | Depends; effort; risk |
|---|---|---|---|---|
| I1 | All / integration maintainer | Run one versioned semantic corpus through core, CLI, LSP, MCP, real WASM and facade consumers. Compare only like operations/profiles; encode deliberate legacy/JSON differences. | Cross-repo CI artifact identifies six exact commits, expected projections, real client versions, failures and documented exceptions. A packaging/transport failure cannot count as semantic agreement. | C1, CLI1, LSP2, MCP2, WASM1, SHIM1; L; high |
| S2 | All / security maintainer plus reviewer | Review trust boundaries, dependency/audit provenance, parser and wrapper budgets, workflow permissions, release signatures and source-to-artifact identity. | Reviewed threat-model delta and current full/delta audit evidence; no unbounded trust or blanket advisory ignores. Findings have reproducers, accountable decisions and linked closing scans. | S1, repository boundary tasks; L; high |
| R1 | All / release maintainer | Retain machine-readable consumer verification receipts for every claimed channel, bound to signed tag/source, digest, workflow identity and publication version. Automate checks beyond asset presence. | Crates.io clean consumer and artifact bytes verified; GPG, cosign and GitHub provenance identity verified where issued; npm attestations, VSIX, container and MCP Registry checked where claimed. Unknown or failed channels remain visible. | R0, S2, repository packaging tasks; L; high |
| R2 | All / release maintainer | Rehearse offline installation and recovery/rollback on supported channels without publishing test artifacts to production registries. Exercise missing/corrupt artifacts and unavailable services. | Sandbox recovery transcript tied to real release artifacts, documented account recovery owners and failure behavior; no tag rewrites or live destructive rollback. | R1, X1; M; medium |
| G1 | All / maintainer | Establish an actual independent review path and continuity ownership for GitHub, signing and publication. Ask an eligible human reviewer to assess the implementation and ratings; decide protection policy only after sustainable reviewer availability exists. | Accepted maintainer responsibilities, access/recovery inventory kept privately, public policy describing roles, and real reviewed PR evidence. Required-review configuration changes need maintainer direction; no fake contributors or self-approvals. | S2; external dependency; high |
| G2 | All / release maintainer and reviewer | Close the plan at the implemented commits, recalculate every category against its acceptance rows and publish dated evidence and remaining limitations. | All 66 category/repo cells link to passing acceptance artifacts and independent review; reopen any stale or contradicted claim. Schedule evidence renewal on releases and relevant changes. | All applicable tasks; M; low |

Each repo's security and governance targets require G1. If no independent
reviewer is available, those criteria remain open and ratings stay below 10.
That is a real sustainability dependency, not something to bypass in CI.

## Verification commands and evidence policy

Run commands from the repository owning the change, using its current
workflow configuration. These are the starting commands, not a substitute
for caller-specific feature sets, regression inputs or boundary checks.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --all-features --locked
cargo audit
cargo deny check
cargo vet
reuse lint
codespell
scripts/check-docs-links.sh
scripts/verify-release-versions.sh
```

Run Markdown lint with the repository's `.markdownlint.yaml` and the same
file exclusions as its docs workflow. For feature-gated suites use their
actual workflow toolchain and feature arguments. Nightly-only features must
not be forced through an incompatible stable all-features command. Run
MSRV, no_std, coverage and fuzz builds in separate target directories.
No cache shared between those compile surfaces may establish a false pass.

Additional owner-specific checks:

| Repo / task | Required boundary check |
|---|---|
| Core C3 | Existing isolated float consumers for default/std/alloc, alloc-only behavioral matrix, schema/CST composition and documented bare-metal CI targets on supported toolchains |
| CLI CLI1/CLI2 | Real binary tests, Action self-test, hooks, `make check-assets`, filesystem failure injection and supported-OS package install |
| LSP LSP2/LSP3 | `cargo test --test framing --test protocol`, installed server framed-client sessions and extension-host tests |
| MCP MCP1/MCP2/MCP3 | `cargo +nightly fuzz build` in fuzz scope, corpus replay, limits/protocol/transport suites, npm wrapper tests and installed bootstrap consumer |
| WASM WASM1/WASM3 | `wasm-pack test --node`, headless browser WASM suite, installed-package TypeScript and bundler/runtime consumers |
| Facade SHIM1 | `cargo test --test drop_in`, shared YAML suite, core compatibility contract, renamed-dependency downstream builds and differential cases |
| Performance P1/P2 | `cargo bench --bench cli_dispatch`, `lsp_handlers`, `mcp_tools`, `shim_overhead` in their repos; core's supported benchmark commands; WASM `wasm-pack build --release --target nodejs` then `node benches/bench.js` |
| Harness Q3 | Fixture-based parser/provenance regression checks first; then fresh six-repo `scripts/ecosystem-scorecard.sh --network --deep --with-coverage --json ...` with exact-source report |

Every acceptance receipt records UTC time, source/dependency commits,
version, toolchain/runtime, features, target/OS, command, result and artifact
location. Preserve raw output where it supports a conclusion. Label
unavailable services and intentionally unsupported checks explicitly.
The existing harness score remains a probe result; 100% on executed probes
does not establish all eleven maturity categories at 10/10.

## Completion and release checkpoints

- **Checkpoint A, assessment:** all six repos rated, evidence pinned and
  this work order written. This documentation update satisfies that scope.
- **Checkpoint B, release decision:** P0 claims reconciled, MCP finding and
  immutable-source decision resolved, all required exact-source gates green.
  The maintainer can then decide publication independently of the longer
  maturity programme. A score average cannot waive a security finding.
- **Checkpoint C, category acceptance:** close one task at a time after its
  done-condition is observed. Record actual implementation commits and
  artifacts; do not replace open tasks with optimistic percentages.
- **Checkpoint D, 10/10:** G2 establishes every scoped criterion in every
  repo, including independent review. Any unverified criterion stays open.

Do not assign speculative release dates. Sequence future versions by +0.0.1
only after branch and current-release state are reconciled. Publish release
notes and channel claims only after the corresponding artifacts exist.
