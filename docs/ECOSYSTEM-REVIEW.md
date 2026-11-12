<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Ecosystem review packet

Prepared 2026-11-11 for the six `feat/v0.0.61` branches. This packet
supports independent review; it does not record a review that has happened.
No reviewer or continuity owner has accepted the role yet. Daybreak access
is not required. A named reviewer may use GitHub or an available channel.

## Sources and evidence

The [candidate manifest](ecosystem-candidate.json) pins all five satellite
sources. The core PR's head determines the candidate core; the build adapter
records its source and local dependency override and refuses tracked source
or lockfile changes afterward. These builds do not prove registry resolution.

Start with the [contract ledger](ECOSYSTEM-CONTRACTS.md),
[implementation record](IMPLEMENTATION-STATUS.md), and the dated
[66-cell assessment](ROADMAP-TO-10.md). The assessment is a baseline,
not a final score for the changed sources. Each task's remaining acceptance
is explicit in the implementation record.

| Evidence | Scope |
|---|---|
| [CI correction record](ecosystem-ci-repair-evidence-2026-11-11.json) | Source identities, reproduced scanner/formatter/typed-key defects, complete std/alloc/portable batteries, bare-metal checks, product tests, consumer parity and raw log hashes |
| [WASM assurance record](ecosystem-wasm-assurance-2026-11-11.json) | Actual optimized exports, installed bundle, runtime contracts, real memory-fault injection and browser process cleanup |
| [MCP boundary record](ecosystem-mcp-boundaries-2026-11-11.json) | Deterministic session-cap failure and correction, full 132-test battery and four real-server example scenarios |
| [MCP lifecycle record](ecosystem-mcp-lifecycle-2026-11-11.json) | Job-owned blocking permits, 135-test battery, Rust 1.89, state fuzzer smoke and Windows CI correction; exact hosted and later source scopes are separate |
| [Protocol/recovery follow-up](ecosystem-protocol-recovery-2026-11-11.json) | LSP framing red/green, lone-CR oracle regression, candidate runner correction and signed detached recovery with its failed immutable-core edit replay |
| [Name bounds and SDK capacity](ecosystem-name-bounds-2026-11-12.json) | Tag/anchor boundary red/green, exact scanner hashes, std/alloc/portable batteries, reduced complexity and actual SDK shared-budget fault injection |
| [Verbatim identity follow-up](ecosystem-verbatim-tags-2026-11-12.json) | Primary-handle override red/green, final scanner checks, interrupted obsolete-source campaigns and distinct browser-startup CI evidence |
| [Performance observations](ecosystem-performance-2026-11-11.json) | Seventy scenarios repeated three times, raw variance, cold consumers and signed-source resource outcomes; no dedicated-host latency claim |
| PR #521 checks | Exact hosted core/candidate sources, platform and toolchain results; inspect each conditional skip separately |
| Candidate artifacts | Per-product source receipts and runtime reports; local paths are retained evidence locations, not public download claims |

The [WASM raw archive](evidence/wasm-assurance-2026-11-11.tar.gz),
[MCP raw archive](evidence/mcp-boundaries-2026-11-11.tar.gz),
[lifecycle archive](evidence/mcp-lifecycle-2026-11-11.tar.gz),
[protocol/recovery archive](evidence/protocol-recovery-followup-2026-11-11.tar.gz) and
[performance/resource archive](evidence/performance-resources-2026-11-11.tar.gz)
are retained with this packet. Check their archive and member digests
against the corresponding JSON record. Installer action source is linked
by immutable upstream refs rather than copied into the evidence archive.

The earlier production checkpoint is
`a57b7198669e41fe3034db4e44f2e274baf96388`. The BOM and line-break oracle
repairs preserve its production source and Cargo inputs. The later name-bound
correction changes the shared scanner; review its evidence separately.
Full batteries at the earlier production checkpoint pass 6,748 std, 6,726 baseline
alloc and 7,067 portable tests. All twelve isolated stable/MSRV bare-metal
checks pass. The name-bound checkpoint passes 6,751 std, 6,732 alloc
and 7,073 portable tests, twelve bare-metal checks and both sanitizer replay
profiles. The subsequent literal-tag correction in `2455c64` passes 6,752
std, 6,733 alloc and 7,074 portable tests, twelve bare-metal checks and
eleven std plus three alloc sanitizer inputs. Each receipt identifies its source.

## Review boundaries

Review the scanner continuation prefix and formatter repair together:
tabs before required spaces are refused on content lines; blank/comment
lines and tabs after required indentation retain their behavior. Check
LF, CRLF and lone CR and formatter idempotence without expanding refusals.

Review [complete name bounds](adr/0020-enforce-complete-name-bounds.md):
tag accounting includes directive expansion, the stored default handle
and suffix; anchor accounting includes EOF. Exactly 1,024 bytes remain
accepted across Value, typed and CST consumers. Keep the tag character
grammar and unknown-handle rejection intact.
Verify that an overridden primary handle cannot rewrite a verbatim local
URI or charge an unused directive prefix to its length budget.

Review typed key spelling separately from canonical duplicate identity.
Trimming trailing separation restores surviving written keys; it cannot
recover entries already removed by canonical deduplication. Preserve
untyped `Value` semantics and distinct-typed key-collision errors.

Review oracle repairs for complete candidate-value preservation and fresh
archive agreement. Empty-line separation repairs must not change quoted or
block scalar content. Keep failed campaigns and their original inputs;
seed replay is not a sustained-campaign result.

Review MCP filesystem confinement, atomic writes, transport/session bounds
and cancellation ownership. Production and fuzz lockfiles are different
audit scopes. Review the held tag by its immutable object identity before
any publication decision.

Review WASM JS graph ownership, safe integer conversion, UTF-8 spans and
unchanged-on-refusal behavior. Resource gates run the actual optimized
boundary: real document/JS retention must fail the same budgets as normal
work. Missing metrics cannot become zero growth. Browser cleanup must only
signal the helper's own private process group.

Review each installer action by its pinned source, downloaded artifact and
credential/cache behavior. Local macOS execution does not establish Linux,
Windows, marketplace or registry acceptance.

## Decisions still required

| Decision | Concrete choice and evidence needed |
|---|---|
| Typed duplicate identity | Accept a choice in [ADR-0019](adr/0019-typed-mapping-duplicate-identity.md) before either semantic change. Retain failing-first policy, loader and alias cases. |
| Held MCP v0.0.60 | Tag object `b28d529f8bae59bbe94bdd91d25ec50173c8702b` also contains confirmed runtime session-admission and blocking-work ownership defects. A development-only dependency justification is insufficient. Signed detached recovery `3561c28` passes 134 native tests but still fails the later Unicode edit input in immutable registry core 0.0.60; its source/ref recovery decision, exact-source gates, independent review and closing scan remain required. The existing tag is untouched. |
| Independent review | Reviewer identity, exact reviewed commits, findings with disposition and actual approval through the chosen channel. No self-review presented as independence. |
| Continuity | An accepted owner and privately retained recovery arrangements for signing, registry and organization access. Do not put secrets in this packet. |

## Acceptance record

A reviewer records the exact six commits, date, reviewed boundaries,
remaining findings and approval status in the chosen channel. Attach its
link here only after that record exists. Reassess the 66 cells after the
technical and human criteria pass. A green subset, local dependency
override or renamed score cannot establish complete release acceptance.
