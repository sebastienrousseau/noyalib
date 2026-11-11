<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Ecosystem implementation record

The [assessment](ROADMAP-TO-10.md) remains the dated baseline. The
[implementation plan](PLAN.md) defines completion, and the
[contract ledger](ECOSYSTEM-CONTRACTS.md) links behavior to tests.
This record separates passing local checks from release and review gates.

The changes are on the six local `feat/v0.0.61` branches. The
[implementation evidence](ecosystem-implementation-evidence-2026-11-10.json)
records the signed satellite heads, test scopes, lockfile hashes and consumer
artifacts. The satellite builds use an untracked local core path override
because v0.0.61 is unpublished. All five satellites passed the Rust 1.89.0
host build; the alloc-only core and WASM binding also built for wasm32. Published artifacts have separate
[verification receipts](ecosystem-publication-receipts-2026-11-10.json).

## Release hold

MCP v0.0.60 remains signed and unpublished. The tag object is
`b28d529f8bae59bbe94bdd91d25ec50173c8702b`, source
`aa3c9db6409c108bc5c1911c076181763a965021`. The
[release run](https://github.com/sebastienrousseau/noyalib-mcp/actions/runs/42897537938)
was cancelled. Public MCP remains v0.0.59.

[RUSTSEC-2026-0386](https://rustsec.org/advisories/RUSTSEC-2026-0386.html)
matches Tokio 1.53.1 in the held tag's fuzz lock. The exact tagged
package listing excludes the fuzz workspace. The production lock already
uses patched Tokio 1.54.1. Git source archives still contain the affected
development lock. The future branch upgrades that lock to 1.54.1 and passes
a clean scan. The remote alert needs a closing scan on the corrected ref.

The maintainer still needs to record whether the development-only finding
is acceptable for the immutable tag. If approved, the release gates apply
to the original signed source. If rejected, agree on a recovery compatible
with the signed tag and lockstep policy before publishing. No tag moved
and nothing was published during this work.

## Fuzz follow-up, 2026-11-11

Six runtime corrections are signed on the future branch: repeated recursive
path traversal, binary-looking strings, unsigned radix strings, clipped
block scalars at EOF, anchor declaration order and synthetic sibling
selection during local CST repair. The last defect reached MCP edit and
could panic on a valid UTF-8 replacement. Its original input now passes
sanitizer replay, and a real stdio regression confirms the session serves
the following request. Root shape changes escalate to a full parse.

The current candidate passed 6,188 default hosted tests and 7,314 tests in
the all-feature run including doctests, with eight existing ignores.
Alloc-only checks passed 2,010 baseline and 2,153 portable-feature tests.
All twelve baseline/portable bare-metal combinations passed on stable and
Rust 1.89.0. The local dependency graphs retain the same features and MSRV.

Archive oracles also disagreed on omitted flow keys and reused anchor IDs.
Their repairs preserve the complete JSON projection and require a fresh
oracle parse to agree. The original same-name anchor regression remains
compared. The obsolete blanket trailing-newline tolerance is removed.
Saphyr contributes an acceptance check; a value comparison remains open.

The first campaign wave retained twenty passing hour-long receipts, five
failures, three interrupted runs and two unstarted targets. A second pinned
wave found the oracle defects, unsigned-string drift and the MCP panic.
Those receipts describe their original source, not the newer corrections.
Every affected target still needs a sustained run on the final candidate.
The tracked fuzz locks now name v0.0.61, and the version gate checks all
tracked Cargo locks. It also works from a renamed checkout.

The future workflows honor `dry_run` for signing and GitHub release creation
on tags. The immutable v0.0.60 workflow predates that correction. The new
runtime fixes are also absent from that held source and need to be included
in the release recovery decision.

## Candidate integration follow-up, 2026-11-11

The CLI action update ([#131](https://github.com/sebastienrousseau/noya-cli/pull/131)),
LSP packager update ([#115](https://github.com/sebastienrousseau/noyalib-lsp/pull/115))
and WASM binding update ([#123](https://github.com/sebastienrousseau/noyalib-wasm/pull/123)) are incorporated in signed commits with Dependabot authorship
and changelog credit. The VSCE consumer passes six extension tests, VSIX
packaging and npm audit. WASM passes 31 Node tests, Rust 1.89 wasm32, actual
optimized npm installation, TypeScript and Chromium checks. Its module is
615,695 bytes raw and 213,419 bytes with gzip level 9, within the unchanged
628,759 and 219,025 byte limits. Nine binding delta audits pass cargo-vet.

A refreshed RustSec snapshot at `2c684a5d6746d4508eff9e3cb529b3e33f56d438`
identified RUSTSEC-2026-0486 in both CLI locks. Both now use rustix 1.1.6
and pass the same pinned advisory scan, cargo-vet and cargo-deny without
an added suppression. CLI tests, clippy and Rust 1.89 pass. The other five
repositories passed the refreshed lock, vet and deny gates.

Strict MCP Inspector 2.14.0 exposed a legal boolean schema that some clients
reject. Object schemas now name all JSON value types and use single-type
nullable branches. Captured pre-change schemas and positive/negative inputs
confirm the accepted value contract is preserved. Native suites, clippy,
Rust 1.89, schema equivalence, strict Inspector and its locked npm audit pass.

Two further oracle regressions preserve rejection of an under-indented
quoted continuation and repair an archive ID overwritten after an alias.
Both original findings pass sanitizer replay. All 142 hosted properties
and seven alloc-only oracle tests pass. New sustained runs remain required;
their receipts must identify the actual candidate used.

The [candidate manifest](ecosystem-candidate.json) pins the five signed
satellite heads. The candidate workflow defines native Linux, macOS and
Windows checks plus an actual WASM consumer check against the checked-out
core. Six candidate gate tests and actionlint pass locally. Remote
execution remains acceptance work. Local overrides preserve registry pins
and locks, and never count as registry-resolution proof.

Daybreak access is outside the plan. The independent human review criterion
remains open through an available review channel.

## Implementation and acceptance

“Local evidence” means the named check passed in its stated scope. Missing
platforms, independent review, registry resolution and CI are listed as
remaining evidence. They are not implied by the local results.

| Task | Local evidence | Remaining acceptance |
|---|---|---|
| S1 | Fourteen tracked Cargo locks passed against clean RustSec revision `28d73fc95e5972af3403225a217def57dcaba3c4`. All six current root graphs passed cargo-deny and the existing cargo-vet policy. VS Code and WASM test npm scopes reported zero advisories; MCP wrapper declares no dependencies. | Retain the same gates on registry-resolved locks and every supported CI scope. Core differential fuzz uses an unmaintained archive oracle; the warning remains visible. |
| R0 | Checked the exact held tag's lockfiles and package listing. | Written maintainer decision, exact-source release gates and a closing scan. |
| MCP1 | Patched fuzz Tokio; scanned all three locks; built both targets. The tools campaign ran 8,903,049 inputs in 3,601 seconds with no finding in the log. | Corrected-ref remote scan and merge. |
| D1 | Reconciled published package scope, version references, LSP capabilities, WASM entry points and unverified benchmark claims. | Current scorecard and remaining channel claims need their acceptance evidence. |
| LSP1 | Capability and framed handshake tests agree with the documentation. Neovim and VS Code exercised the advertised methods. | Carry the evidence through CI at the release ref. |
| SHIM2 | Clarified the standalone std facade and the separate alloc-only core compatibility profile. Both ran in their scoped matrix. | Carry the evidence through CI at the release ref. |
| Q1 | Contract ledger links product operations and refusals to the tests that exercise them. | Fill the specifically untested product and deployment combinations in the ledger. |
| Q2 | Thirty targets discovered; scheduled runner records elapsed time, inputs, executions and findings. MCP tools completed a full campaign; HTTP headers ran 958,775 inputs in 61 seconds. The facade runner smoke completed 102,941 inputs. | One-hour receipts for every remaining target and a successful scheduled run. |
| Q3 | Structured LLVM line totals replace the ambiguous last column. Invalid, timed-out and absent measurements remain N/A. The fresh snapshot measured 139 probes, seven N/A, and scored 92.1%. | Repeat on the final signed release sources and registry graph. |
| P1 | Three release-profile repetitions ran for all six products. The [baseline](ecosystem-performance-2026-11-10.json) records source, artifact, medians, memory and observed variance. | Dedicated-host end-to-end latency and resource cases. |
| P2 | Optimized WASM raw/gzip limits verified. A padded module failed the gate. | Measured budgets and injection checks for the remaining scenarios and products. |
| X1 | Staged CLI install/uninstall, installed VSIX, Node and browser WASM, renamed Rust consumer and MCP offline drill passed. | Fresh-clone, failure and recovery transcripts across the rest of the support matrix. |
| C1 | Current hosted test binaries: 6,188 passed. All-feature tests and doctests: 7,314 passed, eight existing ignores. Shared fixtures check semantics and refusals. | Complete the case-level parity and diagnostics ledger; keep intentional loader differences explicit. |
| C2 | Documented scanner, typed, value and CST responsibilities. Six fuzz-discovered runtime defects have regression tests and signed fixes. | Independent architecture review and measured maintenance evidence before any extraction. |
| C3 | Alloc core and accelerated: 1,747 tests each; portable: 1,832; hosted schema composition: 23; no-std-feature library: 250. Baseline and portable built on three bare-metal targets with stable and Rust 1.89.0. Default, minimal std and alloc float consumers had no active libm. | Rerun the final registry consumer and CI target matrix before release. |
| CLI1 | All-feature CLI probe passed 123 tests. Generated manpages match; staged install, both binary version checks and uninstall passed with a custom target directory. | Hosted Action and package-manager acceptance on the remaining operating systems. |
| CLI2 | A refused rename preserved destination contents and removed the temporary file; mode and symlink tests also pass. | Cross-OS interruption and metadata evidence, with independent review. |
| LSP2 | Tests cover UTF-16 positions, stale changes, initialization, header limits and exit while stdin is open. Final real client checks passed. | Resource and latency budgets for editor workloads. |
| LSP3 | Neovim 0.12.6 and installed VS Code 1.141.0 VSIX passed. The VSIX startup bug was reproduced and fixed. | Installed marketplace artifacts and the rest of the claimed OS matrix. |
| MCP2 | Stdio, negotiated and stateless HTTP, legacy SSE, SDK, limits and guard suites passed 124 tests. Added HTTP header fuzz coverage. | Complete revision-by-transport acceptance for disconnect, cancellation, concurrency and supported filesystems; independent review. |
| MCP3 | Published v0.0.59 npm package recovered from the verified native archive offline, refused corrupt cache and ran a real stdio parse. A future npm release gate repeats the drill. | Apply all five channel checks to v0.0.60 after the hold is resolved. |
| WASM1 | Thirty real Node WASM tests passed. Browser tests cover projection, spans, failed edits, cycles, sparse arrays and resource release. | Fill unmeasured numeric, memory and lifetime boundary cases. |
| WASM2 | Optimized module is 598,344 bytes raw and 208,851 bytes gzip, within the 628,759 and 219,025 byte limits. Three Node benchmark runs record cold import, warmed operations and process RSS. | Establish sustained memory-growth bounds and comparable browser timings. |
| WASM3 | Installed optimized tarball includes both licenses; TypeScript 7.0.2, Node 24.22.2 and a webpack 5.112.2 bundle in Chromium passed. | Repeat against the eventual registry artifact and any newly claimed platforms. |
| SHIM1 | The standalone captured contract and extended tag, number and location cases passed. Equivalent compatibility entry points now have a separate benchmark from resolver-profile costs. | Complete extended location, tag and number cases and measure equivalent core overhead. |
| I1 | Four shared cases passed through six real boundaries. The receipt records Node, Rust and SDK versions, plus the generated module digest. The scheduled workflow builds each checkout from its lock. | First hosted workflow receipt and additional contract cases. |
| S2 | Reviewed changed source and trust boundaries, retained existing vet criteria, and recorded the source and advisory identities. | Independent security review, registry locks and closing scans. |
| R1 | Reverified six public registry crates against signed GitHub assets, exact tags, GPG, Sigstore and GitHub source provenance. | Install and recovery evidence for all claimed secondary channels and held MCP v0.0.60. |
| R2 | Published MCP offline/corruption/restore drill passed. CLI staged uninstall left no files. | Remaining supported channels, missing services, corruption and privately recorded recovery ownership. |
| G1 | Review scope and evidence are ready for a real independent reviewer. | Accepted reviewer and continuity owner, then real review evidence. |
| G2 | Dated baseline and implemented-source evidence are separate. | Reassess all 66 cells once the remaining acceptance checks and independent review pass. |

## Before promoting the branch

Keep the exact registry pins. After the core version exists, remove the local
overrides, resolve the satellite locks against the published core bytes,
compare the dependency delta and rerun the release gates. Run the contract,
audit and consumer checks at the exact signed sources. Do not promote a
local path build to a registry release receipt.

The release versions, scorecard and publishing workflows retain their gates.
A partial receipt can help prioritize work but cannot establish a 10/10
rating or authorize release.
