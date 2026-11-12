<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Ecosystem contracts and verification

Use this ledger to decide which behavior a change must preserve. A suite
in the evidence column is a regression guard, not proof that every
deployment or input has been exercised. The
[implementation record](IMPLEMENTATION-STATUS.md) records what ran,
and the [plan](PLAN.md) sets the acceptance criteria.

## Core loaders and portability

The [architecture](ARCHITECTURE.md) describes the scanner, event layer
and three consumers. A shared YAML example can agree semantically
across loaders without making their outputs or allocations identical.

| Surface | Contract and refusal | Regression evidence |
|---|---|---|
| Typed streaming | Deserialize supported string/slice inputs through the typed path. `from_str` falls back for nested anchor declarations and other constructs needing the general loader. Direct `StreamingDeserializer` exposes a supported subset and does not switch loaders automatically. Keep location and field path when a typed value fails. | [Streaming suite](../crates/noyalib/tests/streaming.rs), [error suite](../crates/noyalib/tests/errors.rs) |
| Dynamic and borrowed values | Preserve tags, aliases and scalar resolution according to the profile; borrow where the representation permits. Never promise that quoted or transformed strings borrow. | [Values suite](../crates/noyalib/tests/values.rs), [anchors suite](../crates/noyalib/tests/anchors.rs) |
| Loader parity and no-span paths | Compare values and errors for equivalent profiles. A no-span path is not a source-location oracle. Scan and parser limits still apply. | [Reading suite](../crates/noyalib/tests/reading.rs), [limits suite](../crates/noyalib/tests/limits.rs), [regressions](../crates/noyalib/tests/regressions.rs) |
| Tag and anchor name bounds | Limit resolved tag URIs and anchor names to 1,024 bytes, including EOF and directive expansion. At-limit accepted tags must survive serialization and reparsing. | [Boundary regressions](../crates/noyalib/tests/limits/dos_hardening.rs), exercised through Value, typed and CST entry points with std and alloc-only dependencies. |
| CST and editing | Preserve untouched comments and presentation. Reparse after edits where required and leave original source intact on refusal. | [CST suite](../crates/noyalib/tests/cst.rs), [alloc API regressions](../crates/noyalib/tests/alloc_apis.rs) |
| Streams and recovery | Retain order, separate documents, enforce document and stream budgets and report recovery errors without accepting a damaged document. | [Streaming suite](../crates/noyalib/tests/streaming.rs), [alloc API regressions](../crates/noyalib/tests/alloc_apis.rs) |
| Serialization | Typed, dynamic, string and formatting sinks agree within the selected scalar and output profile. The alloc path keeps float formatting without a runtime math dependency. | [Serialization suite](../crates/noyalib/tests/ser.rs), [float consumer check](../scripts/check-float-portability.py) |
| Optional policies | Apply strict unknown-field handling, caller-provided includes, property interpolation, alias budgets and shared anchor ownership where advertised. Filesystem operations require a host. | [Integration suite](../crates/noyalib/tests/integrations.rs), [property suite](../crates/noyalib/tests/properties.rs), [alloc API regressions](../crates/noyalib/tests/alloc_apis.rs) |
| Compatibility and conformance | Respect intentional YAML 1.1 and archived compatibility profile differences. A conformance exception needs its own reason and test. | [Conformance suite](../crates/noyalib/tests/conformance.rs), [captured facade contract](../crates/noyalib/tests/integrations/serde_yaml_contract.rs) |

| Build profile | Test expectation and limit |
|---|---|
| Default and minimal std | Isolated consumers use the published feature configuration; neither resolves an active runtime libm edge. Hosted IO, networking and integrations follow their feature gates. |
| Alloc core, accelerated and full portable | Exercise semantics, strict errors, formatting, CST rollback, float behavior and portable feature combinations through the [alloc suites](../crates/noyalib/tests/no_std.rs). Host tests use std for the harness only. |
| Thumb, RISC-V and AArch64 bare metal | Test baseline and combined portable features at stable and Rust 1.89.0. Pointer-width atomics and a downstream allocator are required. The exact triples are listed in [no_std support](NO_STD.md). |
| Schema with core std disabled | [Schema/CST composition](../crates/noyalib/tests/alloc_schema.rs) is a hosted configuration. The schema dependency graph can use std. |
| OS integrations | Readers, writers, filesystem includes, Tokio and Rayon are host capabilities. A bare-metal compile does not promise their behavior. |

Scanner and event code own lexical position and budget accounting. Typed
streaming owns its eligibility and fallback rules. The value loader
owns materialized YAML semantics, and the CST owns source preservation.
Keep a failing input and characterize these boundaries before extracting
code. A complexity report by itself does not justify unifying loaders.

## Satellites

| Product | Supported operation and boundary | Executable evidence and open scope |
|---|---|---|
| CLI | Format stdin/files and check drift. Write only after parsing and formatting succeed. | [Formatter subprocess suite](https://github.com/sebastienrousseau/noya-cli/blob/main/tests/noyafmt_cli.rs); stdin, files, help, output and failure paths. |
| CLI | Validate documents and optional schema; apply conservative repair. Distinguish invalid data, usage errors and IO errors. | [Validator suite](https://github.com/sebastienrousseau/noya-cli/blob/main/tests/noyavalidate_cli.rs) and [fix failures](https://github.com/sebastienrousseau/noya-cli/blob/main/tests/noyavalidate_fix_failures.rs). |
| CLI | Replace via a synced sibling file; preserve mode within supported filesystem semantics. No power-loss guarantee. | [Atomic writer](https://github.com/sebastienrousseau/noya-cli/blob/main/src/atomic.rs), hook and asset checks. Cross-OS metadata and interruption evidence remains open. |
| CLI | Install both binaries, manpages and shell completions, then uninstall cleanly. | Staged Makefile install; package-manager recipes and every supported OS remain separate acceptance checks. |
| LSP | Framed stdio initialization, full document sync, diagnostics, formatting and hover. Reject stale versions and invalid UTF-16 positions. | [Protocol suite](https://github.com/sebastienrousseau/noyalib-lsp/blob/main/tests/protocol.rs) and isolated Neovim run. |
| LSP | Bound headers and message bodies; stop after exit. Unsupported completion, symbols and schema diagnostics are not advertised. | [Framing suite](https://github.com/sebastienrousseau/noyalib-lsp/blob/main/tests/framing.rs) and [protocol coverage](https://github.com/sebastienrousseau/noyalib-lsp/blob/main/docs/protocol-coverage.md). |
| LSP | VS Code extension starts the selected binary, independent client uses ordinary LSP. | Extension npm tests and bundle are separate from real Neovim. The locally installed VSIX passed in the extension host. Marketplace acceptance remains open. |
| MCP | stdio negotiated and stateless revisions, annotated tool catalogue, structured results, prompts and resources. | [Protocol suite](https://github.com/sebastienrousseau/noyalib-mcp/blob/main/tests/protocol.rs) and [SDK client suite](https://github.com/sebastienrousseau/noyalib-mcp/blob/main/tests/serve.rs). |
| MCP | Streamable HTTP supports negotiated sessions and stateless requests; legacy SSE has separate endpoints and lifecycle. Refuse unsafe origins and invalid media types. | [HTTP suite](https://github.com/sebastienrousseau/noyalib-mcp/blob/main/tests/http.rs). Revision, cancellation, disconnect and concurrent-work acceptance remains scoped to observed tests. |
| MCP | Bound live sessions and outstanding blocking jobs. Request timeout or cancellation keeps capacity reserved until the job ends. | [Signed work-ownership decision](https://github.com/sebastienrousseau/noyalib-mcp/blob/55ef95bceb54ac29006b5ccc9d17d3f31fefc11b/docs/adr/0002-bounded-blocking-work.md), deterministic session regression, three worker-lifetime tests and actual-manager state fuzzer. Independent review remains open. |
| MCP | Bound documents, file types and recursion; constrain file tools to the configured root. A failed edit preserves data. | [Limit suite](https://github.com/sebastienrousseau/noyalib-mcp/blob/main/tests/limits.rs) and stdio edit/refusal tests. Independent guard and write review remains open. |
| MCP | npm wrapper checks archive and binary digests and refuses a missing or corrupt offline cache. | Installed v0.0.59 recovery drill uses the actual signed release archive; v0.0.60 release remains held. |
| WASM | Native-value or tag-stripped projection, stringify, validate, query and merge. Projection preserves non-finite numbers; JSON validation refuses them. JS integer conversion refuses unsafe magnitudes. | [Shared runtime contract](https://github.com/sebastienrousseau/noyalib-wasm/blob/2ed44f3aee8d76128b903b57d1371332e511a12f/scripts/runtime-contract-workload.mjs): 34 actual checks under Node, Bun, Deno and Chromium, plus WASM tests. |
| WASM | Lossless document gets, UTF-8 byte spans, comments, set, replacement and release. Limit recursion, references and children before materializing them; leave failed edits unchanged. | Shared contract and generated TypeScript. [Resource workload](https://github.com/sebastienrousseau/noyalib-wasm/blob/2ed44f3aee8d76128b903b57d1371332e511a12f/scripts/runtime-resource-workload.mjs) measures actual repeated exports; retained documents and JS values breach the same growth budgets as normal runs. |
| WASM | Scoped npm bundler package contains generated JS, WASM, types and both licenses. Source-built web/Node targets have separate entry points. | Actual tarball install, declaration check and module execution. The webpack and Chromium pair passed. Other bundlers need their own evidence. |
| Facade | Cargo rename exposes Serde-compatible values, errors, locations and documented legacy scalar/merge behavior. | [Drop-in suite](https://github.com/sebastienrousseau/noyalib-serde-yaml/blob/main/tests/drop_in.rs), core captured contract and renamed downstream consumer. |
| Facade | Standalone package requires std. Core compat remains independently alloc-only. Archived serde_yaml is a differential development oracle. | [Facade contract](https://github.com/sebastienrousseau/noyalib-serde-yaml/blob/main/docs/CONTRACT.md) and core portable tests. |

## Cross-product comparisons

The [shared corpus](assurance-corpus.json) tests declared projections,
including malformed input, Unicode, comment preservation and the legacy
leading-zero difference. CLI formatting, LSP formatting and WASM parsing
return different shapes by design. A transport failure or skipped product
does not count as semantic agreement. Use the
[parity runner](../scripts/check-ecosystem-parity.py) and retain its receipts
with the product and core source identities.
