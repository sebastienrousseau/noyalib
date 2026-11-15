<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Ecosystem assessment and road to 10/10

Assessed **2026-11-10**, against the six clean `main` snapshots below.
This replaces the v0.0.21 assessment. Scores describe engineering maturity
within each repository's documented role. They are judgement informed by
source, CI, registry and release evidence, not probabilities of correctness.
No repository currently meets every acceptance criterion for 10/10.

The executable work order is [PLAN.md](PLAN.md). The dated
[evidence snapshot](ecosystem-assessment-2026-11-10.json) records full commits,
CI runs, coverage totals, workflow floors, alerts and publication state.
Missing evidence is an open assurance gap, not proof of defective code.

## Release state

Five crates are published at v0.0.60. MCP remains published at v0.0.59.
Its [v0.0.60 workflow](https://github.com/sebastienrousseau/noyalib-mcp/actions/runs/42897537938)
was cancelled before publication for this assessment. Its signed tag is
preserved at source `aa3c9db6409c108bc5c1911c076181763a965021`.
The family v0.0.60 release is incomplete.

Complete the P0 work in the plan before reconsidering publication. The
longer 10/10 programme is separate from that release decision: a category
average never overrides a concrete release blocker. Updating this assessment
does not authorize moving the existing tag or publishing changed source
under it.

## Scope and rubric

Scope is all six lockstep crate repos: core, CLI, LSP, MCP, WASM and the
`serde_yaml` facade. Homebrew, Scoop, AUR, editor marketplaces, npm, container
and MCP registries are assessed as distribution channels. Their hosting
repositories are not represented as separately audited codebases here.

Eleven categories cover the previous roadmap's axes and the automated
harness's seven groups. Architecture and distribution are explicit; each
satellite gets its own assessment instead of one aggregate satellite grade.

| Score | Meaning |
|---|---|
| 0 to 5 | Absent, failing, or too little evidence to establish the scoped contract |
| 6 | Implemented, with material evidence or assurance gaps |
| 7 | CI exercised, with consumer or contract gaps |
| 8 | Strong automation and exercised boundaries; acceptance work remains |
| 9 | Broad evidence and verified consumers; final acceptance work remains |
| 10 | All scoped category criteria below closed, independently reviewed, and maintained by regression checks |

These are ordinal ratings, deliberately rounded to whole points. The
assessment does not claim a mathematical difference between an 8 and a 9.
Unavailable external scores remain unavailable. A passing probe or a high
coverage percentage alone cannot establish 10/10.

| Category | Acceptance required for 10/10 | Harness relationship |
|---|---|---|
| Functionality / API | Every advertised capability mapped to executable positive, negative and boundary cases; intentional limits explicit | correctness |
| Correctness / testing | Contract tests at real product boundaries, regression inputs, sustained fuzz evidence, reviewed coverage gaps and relevant properties | correctness, robustness |
| Architecture / maintainability | Reviewed boundary contracts, dependency direction and failure ownership; characterisation evidence before structural changes | code_health |
| Performance / resources | Commit-bound reproducible throughput, latency, memory and size baselines; justified budgets and a demonstrated regression signal | robustness; timing is outside the harness |
| Security / supply chain | All manifest/lockfile and wrapper scopes scanned; audit evidence current; findings fixed or individually justified; authentic release artifacts verified | supply_chain, integrity |
| Documentation | Claims match source and published packages; examples run at supported consumer boundaries; version and link checks pass | docs |
| Portability / no_std | Declared targets, toolchains and feature compositions exercised; core alloc-only behavior and float parity preserved | correctness, code_health |
| Developer experience / diagnostics | Fresh setup and install succeed; failure messages, exit codes and recovery paths tested; local commands agree with CI | code_health, docs |
| Interoperability | Versioned compatibility matrix with real clients and deliberate differences; unsupported capabilities are explicit | correctness, release |
| Distribution / consumers | Every claimed channel installs the exact release; bytes, signatures and provenance verified; install failure and rollback paths exercised | release, integrity |
| Release / governance | Immutable signed sources, green exact-source gates, recovery rehearsal and sustainable independent review with real maintainers | release, integrity, supply_chain |

A 10 is a dated, bounded acceptance judgement, never a guarantee that future
bugs cannot exist. New functionality is not required merely to increase a
score. Public API, MSRV and feature changes need maintainer direction.

## Ratings out of 10

| Category | Core | CLI | LSP | MCP | WASM | Facade |
|---|---:|---:|---:|---:|---:|---:|
| Functionality / API | 9 | 8 | 7 | 9 | 8 | 8 |
| Correctness / testing | 9 | 8 | 8 | 8 | 8 | 8 |
| Architecture / maintainability | 8 | 8 | 8 | 8 | 8 | 9 |
| Performance / resources | 8 | 6 | 6 | 6 | 6 | 6 |
| Security / supply chain | 9 | 8 | 8 | 6 | 8 | 8 |
| Documentation | 8 | 7 | 6 | 7 | 6 | 6 |
| Portability / no_std | 9 | 8 | 8 | 8 | 7 | 7 |
| Developer experience / diagnostics | 8 | 8 | 7 | 8 | 7 | 8 |
| Interoperability | 9 | 8 | 7 | 8 | 7 | 8 |
| Distribution / consumers | 9 | 9 | 8 | 7 | 8 | 9 |
| Release / governance | 6 | 6 | 6 | 6 | 6 | 6 |

Confidence is high for the recorded commits, API responses, CI conclusions,
coverage totals and identified contradictions. Architectural, ergonomic and
performance maturity ratings have moderate confidence: this pass inspected
contracts and evidence, rather than conducting a new exhaustive code audit,
user study, penetration test or benchmark campaign. All ratings remain
provisional until independent review in G1.

## Evidence baseline

| Repo | Assessed commit | Exact-source CI | Published crate |
|---|---|---|---|
| [Core](https://github.com/sebastienrousseau/noyalib) | `2b3966235352c44b70071390faf6f06ec8b4d65d` | [42787265272](https://github.com/sebastienrousseau/noyalib/actions/runs/42787265272) | 0.0.60 |
| [CLI](https://github.com/sebastienrousseau/noya-cli) | `9fdd09a177b6bf29084f0897db0d3e6823a27e96` | [42799812146](https://github.com/sebastienrousseau/noya-cli/actions/runs/42799812146) | 0.0.60 |
| [LSP](https://github.com/sebastienrousseau/noyalib-lsp) | `3fccfbe5782e5b43f6ac6c8bb04595063e024e8d` | [42792082961](https://github.com/sebastienrousseau/noyalib-lsp/actions/runs/42792082961) | 0.0.60 |
| [MCP](https://github.com/sebastienrousseau/noyalib-mcp) | `aa3c9db6409c108bc5c1911c076181763a965021` | [42896967498](https://github.com/sebastienrousseau/noyalib-mcp/actions/runs/42896967498) | 0.0.59; 0.0.60 held |
| [WASM](https://github.com/sebastienrousseau/noyalib-wasm) | `b00e5ece6281c1d0e137445464734037f6531e53` | [42792100355](https://github.com/sebastienrousseau/noyalib-wasm/actions/runs/42792100355) | 0.0.60 |
| [Facade](https://github.com/sebastienrousseau/noyalib-serde-yaml) | `91e2b3cbdf04549893c983704c81f6d4cdf61865` | [42792116009](https://github.com/sebastienrousseau/noyalib-serde-yaml/actions/runs/42792116009) | 0.0.60 |

All executed jobs in these CI runs succeeded. Conditional skipped jobs are
recorded as skipped in the JSON, not credited as successful checks. Relevant
PR, release and consumer checks add evidence, but do not convert a skipped
main-branch Miri or signature job into a pass.

### Coverage and robustness

Values come from the successful coverage jobs' `TOTAL` rows, with the
column headers checked. They are not inferred from a job name such as
"Coverage gate (≥95%)", which hides caller-specific thresholds.

| Repo | Lines | Functions | Regions | Current floors: line / function / region | Fuzz targets |
|---|---:|---:|---:|---|---:|
| Core | 95.25% | 96.93% | 94.08% | 94 / 96 / 93 | 18 main + 2 alloc-only |
| CLI | 97.09% | 96.91% | 95.91% | 88 / 96 / 90 | 2 |
| LSP | 95.57% | 98.17% | 95.30% | 94 / 96 / 93 | 2 |
| MCP | 98.26% | 97.52% | 96.36% | 94 / 96 / 93 | 1 |
| WASM | 99.00% | 100.00% | 99.10% | 94 / 96 / 93 | 2 |
| Facade | N/A | N/A | N/A | No independent coverable runtime functions | 2 |

Coverage includes instrumented `cfg(test)` source. WASM native coverage is
not browser coverage; subprocess, protocol and generated wrapper paths have
different denominators. Branch coverage is unmeasured in these totals.
The facade's thin re-export is assessed through its downstream contract,
not assigned a fabricated 100% coverage result. Target counts establish
inventory, not fuzzing duration or effectiveness.

### Security and governance

GitHub returned zero open Dependabot alerts in all six repos. Code scanning
returned zero open alerts in five and one current **high** alert in MCP:
[alert 25](https://github.com/sebastienrousseau/noyalib-mcp/security/code-scanning/25),
Scorecard `VulnerabilitiesID`, against the assessed main commit.

[RUSTSEC-2026-0386](https://rustsec.org/advisories/RUSTSEC-2026-0386.html)
affects Tokio's `AsyncFd` constructors. The
[MCP production lock](https://github.com/sebastienrousseau/noyalib-mcp/blob/aa3c9db6409c108bc5c1911c076181763a965021/Cargo.lock)
uses patched Tokio **1.54.1**; its
[fuzz lock](https://github.com/sebastienrousseau/noyalib-mcp/blob/aa3c9db6409c108bc5c1911c076181763a965021/fuzz/Cargo.lock)
uses affected **1.53.1**. This is a matching affected dependency, not a proven
production exploit or a proven false positive. Fix and rescan the fuzz scope;
keep the alert open until the scanner confirms resolution. Registry
`const-oid` 0.10.3 and SDK 3.9.0 are already in the production graph; their
successful audit work does not cover a different fuzz lockfile.

All six `main` protections are strict, but require **zero approving
reviews**. The direct-collaborator API returned one human per repo. This
establishes a review and continuity gap; it does not establish who can
recover every signing, registry or organisation account. G1 must establish
that separately with the maintainer. No contributors will be invented to
improve a number.

The external OpenSSF API could not be reached during this assessment.
Its previous scores are historical, not current evidence. GitHub's current
Scorecard alert remains valid evidence independently of that API outage.

## Per-repository gaps and route to 10

Each row explains its rating and names plan tasks that close the remaining
acceptance criteria. Shared tasks apply to every repo listed in the plan.

### Core: noyalib

Evidence includes [CI](https://github.com/sebastienrousseau/noyalib/actions/runs/42787265272),
[architecture](ARCHITECTURE.md), [benchmarks](BENCHMARKS.md),
[feature manifest](../crates/noyalib/Cargo.toml) and the
[unconditional unsafe prohibition](../crates/noyalib/src/lib.rs).

| Category | Basis and remaining acceptance work | Tasks |
|---|---|---|
| Functionality: 9 | Broad parser, Serde, CST and streaming surface; finish a capability-to-test ledger including refusals and limits | C1, Q1 |
| Correctness: 9 | Conformance, properties, differential fuzz, Kani and regression CI; sustained editor fuzz and production-only gap review still needed | C1, Q2 |
| Architecture: 8 | Documented distinct loaders and complexity gate; review their parity obligations and error ownership before any extraction | C2 |
| Performance: 8 | Real historical benchmark tables and size gate; refresh commit-bound baselines with memory and variance | P1, P2 |
| Security: 9 | Audit, deny, vet, offline build, unsafe prohibition and verified release; expand lockfile scope and independent security review | S1, S2, G1 |
| Documentation: 8 | Extensive manual and runnable examples; stale planning claims and feature documentation need systematic reconciliation | D1, C1 |
| Portability: 9 | Real alloc-only behavior, float consumers and bare-metal CI; retain a complete feature-to-target support ledger and negative graph checks | C3 |
| DX: 8 | Typed paths and rich diagnostics; test fresh setup and classify actionable error/recovery cases across loaders | C1, X1 |
| Interoperability: 9 | Serde and multiple integrations exercised; record supported combinations and intentional behavior differences | C1, I1 |
| Distribution: 9 | Authentic registry source, signed release and consumer tests verified; retain reusable receipts and recovery evidence | R1, R2 |
| Governance: 6 | Signed lockstep releases and protections; independent review and access continuity remain open | G1, G2 |

### CLI: noya-cli

Evidence: [source](https://github.com/sebastienrousseau/noya-cli/tree/9fdd09a177b6bf29084f0897db0d3e6823a27e96),
including `src/atomic.rs`, `tests/`, `action.yml`, `.pre-commit-hooks.yaml`,
`docs/cli-reference.md` and `docs/BENCHMARKS.md`.

| Category | Basis and remaining acceptance work | Tasks |
|---|---|---|
| Functionality: 8 | Two shipped tools cover formatting, validation and repair; map each flag and exit contract to real binary tests | CLI1 |
| Correctness: 8 | Platform matrix, YAML suite and fuzz replay; extend atomic-write and interruption failures at process boundaries | CLI1, Q2 |
| Architecture: 8 | Dispatch, text and atomic I/O are separated; review I/O ownership and characterise failure paths | CLI2 |
| Performance: 6 | Criterion harness exists; README's approximate 10 ms lacks an accompanying reproducible result | P1, P2, D1 |
| Security: 8 | Dependency and workflow gates pass; audit all lock scopes and review write/path handling independently | S1, S2, CLI2 |
| Documentation: 7 | Generated flags, manpages and examples exist; align install and performance claims with verified channels | D1, CLI1 |
| Portability: 8 | OS/toolchain matrix and native assets; exercise filesystem differences and package installation on supported OSes | CLI1, R2 |
| DX: 8 | Completions, manpages, Action and hooks exist; verify fresh install, stdin, failure messages and uninstall | X1, CLI1 |
| Interoperability: 8 | Shared core, Action and hooks; add a cross-surface semantics corpus and executable integration recipes | I1, CLI1 |
| Distribution: 9 | Archives, packages and multiarch container verified; run package-manager and rollback checks with retained receipts | R1, R2 |
| Governance: 6 | Shared review and continuity gap | G1, G2 |

### LSP: noyalib-lsp

Evidence: [source](https://github.com/sebastienrousseau/noyalib-lsp/tree/3fccfbe5782e5b43f6ac6c8bb04595063e024e8d),
particularly `src/lib.rs::dispatch`, `tests/framing.rs`, `tests/protocol.rs`,
`editors/vscode/`, `README.md` and `docs/protocol-coverage.md`.

| Category | Basis and remaining acceptance work | Tasks |
|---|---|---|
| Functionality: 7 | Dispatch implements formatting and hover; README advertises completion and symbols that dispatch lacks | LSP1 |
| Correctness: 8 | Framing, protocol and handler tests; strengthen Unicode positions, lifecycle, malformed framing and stale changes | LSP2, Q2 |
| Architecture: 8 | Separate formatting, hover and diagnostic handlers; review sync, cache invalidation and request lifecycle contracts | LSP2 |
| Performance: 6 | Handler benchmark harness without recorded baseline; measure edit-to-diagnostic latency and memory | P1, P2 |
| Security: 8 | Input and dependency gates; review schema retrieval and framing/resource boundaries independently | S1, S2, LSP2 |
| Documentation: 6 | README and protocol table contradict each other; editor setup still uses an unframed echo as a smoke check | LSP1, D1 |
| Portability: 8 | Three-OS Rust matrix and VSIX build; add real editor/OS boundary checks | LSP3, X1 |
| DX: 7 | Editor setup and VSIX exist; executable setup, capability discovery and diagnostics need consistent guidance | LSP1, LSP3, X1 |
| Interoperability: 7 | Protocol 3.17 scope documented; real-client capability and Unicode position matrix incomplete | LSP2, LSP3, I1 |
| Distribution: 8 | Signed native releases and Marketplace evidence; verify extension's installed server path and other claimed channels | LSP3, R1, R2 |
| Governance: 6 | Shared review and continuity gap | G1, G2 |

### MCP: noyalib-mcp

Evidence: [source](https://github.com/sebastienrousseau/noyalib-mcp/tree/aa3c9db6409c108bc5c1911c076181763a965021),
including `tests/protocol.rs`, `tests/limits.rs`, `src/transport/`,
`src/fsio.rs`, `pkg/npm-wrapper/`, `supply-chain/` and the linked alert above.

| Category | Basis and remaining acceptance work | Tasks |
|---|---|---|
| Functionality: 9 | SDK 3.9.0, multiple protocol revisions, tools and three transports exercised; publish exact capability/transport limits | MCP2 |
| Correctness: 8 | Protocol, root and limit regressions; only one tool fuzzer, no equivalent sustained transport/state fuzz evidence | MCP2, Q2 |
| Architecture: 8 | Filesystem and transport guards are separated; review cancellation, session and write ownership together | MCP2 |
| Performance: 6 | Tool harness exists; request, concurrency and backpressure budgets lack reproducible measurements | P1, P2, MCP2 |
| Security: 6 | Production crypto audit completed, but current high alert matches affected fuzz Tokio; independently review transports and filesystem confinement | S1, S2, MCP1, MCP2 |
| Documentation: 7 | Current SDK and distribution documented; align capability/revision, auth and wrapper contracts with actual support | MCP2, D1 |
| Portability: 8 | OS matrix and native assets; validate transport and filesystem limits on each supported OS | MCP2, X1 |
| DX: 8 | Cargo, npm bootstrap and container install work at v0.0.59; exercise errors, offline/cache and configuration paths | MCP3, X1 |
| Interoperability: 8 | Legacy and stateless protocol tests; pin real client/Inspector versions and transport/security cases | MCP2, I1 |
| Distribution: 7 | All five v0.0.59 channels verified; v0.0.60 intentionally unpublished and awaits source/hold decision | R0, MCP3, R1, R2 |
| Governance: 6 | Immutable signed tag preserved; shared review and continuity gap remains | R0, G1, G2 |

### WASM: noyalib-wasm

Evidence: [source](https://github.com/sebastienrousseau/noyalib-wasm/tree/b00e5ece6281c1d0e137445464734037f6531e53),
including `src/guard.rs`, `tests/wasm.rs`, `benches/bench.js`,
`docs/js-api.md`, `docs/bundling.md`, the
[package-content gate](https://github.com/sebastienrousseau/noyalib-wasm/blob/b00e5ece6281c1d0e137445464734037f6531e53/scripts/check-npm-package.sh)
and the release workflow that generates `pkg/package.json`.

| Category | Basis and remaining acceptance work | Tasks |
|---|---|---|
| Functionality: 8 | JS and lossless document APIs, guarded conversion; test every generated export against its documented value/error shape | WASM1 |
| Correctness: 8 | Native 99% lines and Node WASM tests; real browser, cycle, alias and numeric boundaries need a complete contract ledger | WASM1, Q2 |
| Architecture: 8 | Conversion guard and native core separation; review JS ownership, lifetimes and unchanged-on-error guarantees | WASM1 |
| Performance: 6 | Real JS boundary harness exists; missing reproducible cold-start, memory, latency and generated size baseline | P1, P2, WASM2 |
| Security: 8 | Guard tests and npm provenance; complete JS graph/resource and build-tool audit scope | WASM1, S1, S2 |
| Documentation: 6 | JS docs use an unscoped npm name and promise web/nodejs subpaths and rough sizes; reconcile with the actual scoped package, export map and bytes | WASM3, D1 |
| Portability: 7 | Node boundary exercised; browser and claimed bundler/runtime combinations need package-level execution | WASM3 |
| DX: 7 | TypeScript declarations and JS reference exist; compile and run copyable recipes from the installed tarball | WASM3, X1 |
| Interoperability: 7 | Generated wrapper works in Node; JS number, tags, non-string keys and module-format semantics need a tested matrix | WASM1, WASM3, I1 |
| Distribution: 8 | npm signatures/provenance and registry crate verified; retain browser/export-map and package-manager consumer receipts | WASM3, R1, R2 |
| Governance: 6 | Shared review and continuity gap | G1, G2 |

### Compatibility facade: noyalib-serde-yaml

Evidence: [source](https://github.com/sebastienrousseau/noyalib-serde-yaml/tree/91e2b3cbdf04549893c983704c81f6d4cdf61865),
including `src/lib.rs`, `Cargo.toml`, `tests/drop_in.rs`,
`docs/CONTRACT.md` and `docs/MIGRATION.md`.

| Category | Basis and remaining acceptance work | Tasks |
|---|---|---|
| Functionality: 8 | Thin re-export with captured compatibility contract; enumerate API and behavioral limits instead of unrestricted drop-in claims | SHIM1 |
| Correctness: 8 | Downstream and shared YAML tests; expand representative compile contracts and documented differential cases | SHIM1, Q2 |
| Architecture: 9 | One small re-export module; verify export/dependency drift and core compatibility ownership | SHIM1 |
| Performance: 6 | Shim overhead harness without a captured equivalence baseline | P1, P2, SHIM1 |
| Security: 8 | No archived runtime implementation; audit test/fuzz scopes and inherited compatibility resource limits | S1, S2, SHIM1 |
| Documentation: 6 | Contract doc claims standalone no_std, but manifest enables core std; test paths also retain monorepo locations | SHIM2, D1 |
| Portability: 7 | Rust/OS matrix, but facade requires std; correct the claim while preserving core alloc-only compatibility support | SHIM2, C3 |
| DX: 8 | Cargo rename migration works; run realistic downstream compile/error examples and rollback | SHIM1, X1 |
| Interoperability: 8 | Eighteen captured cases are valuable, not exhaustive equivalence; extend supported API/type/error matrix | SHIM1, I1 |
| Distribution: 9 | Registry crate and signed release verified; add reusable renamed-dependency consumer receipt | R1, SHIM1 |
| Governance: 6 | Shared review and continuity gap | G1, G2 |

## Corrections to earlier plans

MCP npm and registry packaging, satellite fuzz harnesses, CLI Action and
pre-commit hooks, VS Code packaging, core bare-metal builds, schema hardening,
SBOMs and release attestations already exist. They are not new feature work.
The core forbids unsafe code unconditionally, including SIMD paths.
The MSRV policy and comparison/migration docs also already exist.

The retired `xtask` monorepo restructure is not an active workstream. No
0.1.0 jump is proposed: releases remain +0.0.1 for exactly six crates under
ADR-0005. Query tools, a new linter crate, Python/native Node bindings,
remote schema catalogues and new MCP capabilities are optional product
proposals, not prerequisites invented to force a score to 10.
