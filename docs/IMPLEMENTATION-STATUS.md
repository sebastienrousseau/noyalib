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
because v0.0.61 is unpublished. Published artifacts have separate
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

## Implementation and acceptance

“Local evidence” means the named check passed in its stated scope. Missing
platforms, independent review, registry resolution and CI are listed as
remaining evidence. They are not implied by the local results.

| Task | Local evidence | Remaining acceptance |
|---|---|---|
| S1 | Fourteen tracked Cargo locks passed against clean RustSec revision `28d73fc95e5972af3403225a217def57dcaba3c4`. All six current root graphs passed the existing cargo-vet policy. VS Code and WASM test npm scopes reported zero advisories; MCP wrapper declares no dependencies. | Retain the same gates on registry-resolved locks and every supported CI scope. Core differential fuzz uses an unmaintained archive oracle; the warning remains visible. |
| R0 | Checked the exact held tag's lockfiles and package listing. | Written maintainer decision, exact-source release gates and a closing scan. |
| MCP1 | Patched fuzz Tokio; scanned all three locks; built both targets. The tools campaign ran 8,903,049 inputs in 3,601 seconds with no finding in the log. | Corrected-ref remote scan and merge. |
| D1 | Reconciled published package scope, version references, LSP capabilities, WASM entry points and unverified benchmark claims. | Current scorecard and remaining channel claims need their acceptance evidence. |
| LSP1 | Capability and framed handshake tests agree with the documentation. Neovim and VS Code exercised the advertised methods. | Carry the evidence through CI at the release ref. |
| SHIM2 | Clarified the standalone std facade and the separate alloc-only core compatibility profile. Both ran in their scoped matrix. | Carry the evidence through CI at the release ref. |
| Q1 | Contract ledger links product operations and refusals to the tests that exercise them. | Fill the specifically untested product and deployment combinations in the ledger. |
| Q2 | Thirty targets discovered; scheduled runner records elapsed time, inputs, executions and findings. MCP tools completed a full campaign; HTTP header target ran 958,775 inputs in 61 seconds. | One-hour receipts for every remaining target and a successful scheduled run. |
| Q3 | Structured LLVM line totals replace the ambiguous last column. Invalid, timed-out and absent measurements remain N/A. The fresh snapshot measured 139 probes, seven N/A, and scored 92.1%. | Repeat on the final signed release sources and registry graph. |
| P1 | All six release-profile benchmark executables or packages built. The runner records source, lock, artifact, host and repetition data. | Complete measured repetitions with background builds idle and extend end-to-end latency and resource cases. |
| P2 | Optimized WASM raw/gzip limits verified. A padded module failed the gate. | Measured budgets and injection checks for the remaining scenarios and products. |
| X1 | Staged CLI install/uninstall, installed VSIX, Node and browser WASM, renamed Rust consumer and MCP offline drill passed. | Fresh-clone, failure and recovery transcripts across the rest of the support matrix. |
| C1 | Core default suite: 6,704 passed, two existing ignores. Deep all-feature probe: 7,298 passed. Shared fixtures check semantics and refusals. | Complete the case-level parity and diagnostics ledger; keep intentional loader differences explicit. |
| C2 | Documented scanner, typed, value and CST responsibilities. Core runtime source is identical to the v0.0.60 tag. | Independent architecture review and measured maintenance evidence before any extraction. |
| C3 | Alloc core and accelerated: 1,747 tests each; portable: 1,832; hosted schema composition: 23; no-std-feature library: 250. Baseline and portable built on three bare-metal targets with stable and Rust 1.89.0. Default, minimal std and alloc float consumers had no active libm. | Rerun the final registry consumer and CI target matrix before release. |
| CLI1 | All-feature CLI probe passed 123 tests. Generated manpages match; staged install, both binary version checks and uninstall passed with a custom target directory. | Hosted Action and package-manager acceptance on the remaining operating systems. |
| CLI2 | A refused rename preserved destination contents and removed the temporary file; mode and symlink tests also pass. | Cross-OS interruption and metadata evidence, with independent review. |
| LSP2 | Tests cover UTF-16 positions, stale changes, initialization, header limits and exit while stdin is open. Final real client checks passed. | Resource and latency budgets for editor workloads. |
| LSP3 | Neovim 0.12.6 and installed VS Code 1.141.0 VSIX passed. The VSIX startup bug was reproduced and fixed. | Installed marketplace artifacts and the rest of the claimed OS matrix. |
| MCP2 | Stdio, negotiated and stateless HTTP, legacy SSE, SDK, limits and guard suites passed 124 tests. Added HTTP header fuzz coverage. | Complete revision-by-transport acceptance for disconnect, cancellation, concurrency and supported filesystems; independent review. |
| MCP3 | Published v0.0.59 npm package recovered from the verified native archive offline, refused corrupt cache and ran a real stdio parse. A future npm release gate repeats the drill. | Apply all five channel checks to v0.0.60 after the hold is resolved. |
| WASM1 | Thirty real Node WASM tests passed. Browser tests cover projection, spans, failed edits, cycles, sparse arrays and resource release. | Fill unmeasured numeric, memory and lifetime boundary cases. |
| WASM2 | Optimized module is 598,344 bytes raw and 208,851 bytes gzip, within the 628,759 and 219,025 byte limits. | Complete repeated startup, latency and memory measurements. |
| WASM3 | Installed optimized tarball includes both licenses; TypeScript 7.0.2, Node 24.22.2 and a webpack 5.112.2 bundle in Chromium passed. | Repeat against the eventual registry artifact and any newly claimed platforms. |
| SHIM1 | Native facade tests and the Cargo-rename consumer passed, including deliberate legacy behavior. | Complete extended location, tag and number cases and measure equivalent core overhead. |
| I1 | Four shared cases passed through six real boundaries. The scheduled workflow builds each checkout from its lock and retains the result. | First hosted workflow receipt and additional contract cases. |
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
