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

The held source also contains reproduced session-admission and blocking-work
ownership defects. A development-only finding justification is insufficient.
A detached recovery candidate is being verified against registry core 0.0.60;
its source/ref decision, independent review and release gates remain open.
No tag moved and nothing was published during this work.

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

## Portable battery and hosted candidates, 2026-11-11

The [follow-up receipt](ecosystem-followup-evidence-2026-11-11.json) records
signed source identities and hashes of the raw verification logs. The full
hosted batteries pass 6,734 all-feature std tests, 6,705 baseline alloc-only
tests and 7,046 portable alloc-only tests. All 590 doctests pass, with eight
existing ignores. The expanded ten-test oracle helper suite is included.
CI now runs the complete baseline, accelerated
and portable batteries. All twelve baseline/portable bare-metal checks pass
on stable and Rust 1.89.0 across the three supported targets.

The six-consumer corpus grew from four to eleven cases. Its first execution
exposed a formatter refusal: adding a line break after a terminal block
scalar changed the parsed value. The signed correction preserves EOF and
keeps the semantic guard. All 1,090 CST tests pass, including the formerly
refused L24T formatter case. The eleven cases then passed through actual
core, facade, CLI, LSP, MCP and Node WASM consumers.

Fresh campaigns found two additional archive differences. The terminal-space
case matches the vendored official L24T expectation; leading-colon flow
scalars follow YAML 1.2.2 section 7.3.3. Narrow source normalization retains
the candidate value and requires the complete reparsed oracle value to
agree. Both original byte inputs pass sanitizer replay. The ten helper
tests include observed failures before implementation; all 145 hosted
properties and 158 diagnostic tests pass. Fresh sustained runs are still
required for these corrections; fixed-input replay is a separate result.

A later original input exposed a typed-key loader inconsistency. An omitted
anchored scalar kept its property span, so AST fallback exposed the canonical
null spelling as a string key. The signed fix restores an empty typed key
without changing canonical Value keys or diagnostic spans. The exact 170-byte
input is retained in both the crate fixtures and the fuzz regressions; all six
typed-key tests and its sanitizer replay pass on std and alloc paths.

Concurrent verification also exposed a scheduling-sensitive elapsed-time
assertion. The replacement counts uncached lookahead bytes under cfg(test),
bounds work by input length and requires forced cache invalidation to exceed
the bound. The original queue-compaction bound remains. Owned, borrowed and
CST integration tests check all values in the same large flow inputs. No
counter is compiled into production builds. The initial failure and passing
replacement are both retained in the receipt.

The final optimized WASM module is 613,905 bytes raw and 211,810 bytes with
gzip level 9, within the unchanged limits. Actual npm installation,
TypeScript 7.0.2, Node 24.22.2 and Chromium pass; all 31 real Node WASM tests
also pass. The earlier binding-update size receipt remains historical.

All six future branches are now pushed. The facade's fully merged v0.0.58
branch was verified against main and backed up before retiring its stale
ref, preserving the two-version-branch limit. Dependabot PRs
[CLI #131](https://github.com/sebastienrousseau/noya-cli/pull/131),
[LSP #115](https://github.com/sebastienrousseau/noyalib-lsp/pull/115) and
[WASM #123](https://github.com/sebastienrousseau/noyalib-wasm/pull/123) are
closed with links to their signed integrations. Original bot authorship and
changelog credit remain. The pinned satellite candidates include the
formatter delta in their first-party audits; all five pass cargo-vet.

The hosted [strict MCP Inspector check](https://github.com/sebastienrousseau/noyalib-mcp/actions/runs/43040927622)
passes on the exact MCP candidate with the audited core. Cross-platform
candidate CI and final-source fuzz acceptance remain open at that earlier checkpoint. No registry
publication, tag change or default-branch merge took place in this follow-up.

## CI correction checkpoint, 2026-11-11

Signed core source `0ff5b3c2119ce60a1830b205b6c5c9c466234fad` passes
[CodeQL](https://github.com/sebastienrousseau/noyalib/runs/130618470574),
[package size](https://github.com/sebastienrousseau/noyalib/actions/runs/43047370527/job/130614953440),
[complexity](https://github.com/sebastienrousseau/noyalib/actions/runs/43047370527/job/130614952935)
and the [installed WASM consumer](https://github.com/sebastienrousseau/noyalib/actions/runs/43047369493/job/130614932550).
The candidate workflow now runs on pull-request refs, removing the manual
default-branch execution path that CodeQL flagged. No alert suppression or
budget increase was added. All fifteen native product/OS jobs pass.

The loader extraction retains both declaration-span guards and alias-cost
calculations. Std and alloc anchor, serializer and property suites pass.
The `process_event` complexity baseline shrinks from 177 to 173 lines.
Removing obsolete source coordinates from test comments keeps every test
and assertion. At signed source `266f584`, the clean package with the two new oracle
regressions is 1,319,431 bytes against the unchanged 1,320,000-byte ceiling.
The subsequent comment-separation case retains that ceiling.

Chromium cleanup waits for inherited streams to close and retries transient
profile-directory races with a fixed bound. The real child-process test
fails with the former exit-only wait. Both shutdown tests and the installed
npm consumer's 21 browser checks pass. Persistent cleanup errors still fail
the gate. All five refreshed satellite source reviews pass cargo-vet.

Signed source `266f5844d06ef4df229a83db706b7aa8a1954f27` repairs two
further comparison assumptions. Multiline flow-map plain keys receive a
semantics-preserving quoted spelling for libyaml. Large unsigned radix
strings match only the exact integer outside the default i64 range,
consistent with ADR-0004. Production resolution is unchanged. Both original
inputs pass sanitizer replay. The subsequent compatibility campaign found
libyaml accepting a comment marker attached to a closed quote. That
specific scanner mismatch is now pinned with its original input. All three
original inputs pass sanitizer replay; 148 hosted properties and thirteen
alloc-only oracle tests pass. A fresh compatibility campaign runs on signed
source `390af59`; the prior failure remains recorded.

The hosted [six-consumer parity job](https://github.com/sebastienrousseau/noyalib/actions/runs/43047369493/job/130614933181)
passes. The [CI correction evidence](ecosystem-ci-repair-evidence-2026-11-11.json)
records source identities, check URLs and hashes for the checkpoint
archive. These observations supplement the earlier full batteries; the
final signed-source matrix, independent review and registry release gates
remain separate. Daybreak access is not required.

The six campaign runners now retain observed execution counts as lower
bounds when the final summary is absent. Missing counts and durations stay
unknown. A completion summary, full duration, successful exit, unchanged
lock and no finding are required to pass. Four regression cases fail with
the old accounting and pass in every repository. A real short smoke reports
fourteen executions in four seconds; it is not a sustained release receipt.
Re-reading three failed logs recovers lower bounds of 929,138, 65 and
23,735 executions. Original receipts remain unchanged and marked failed.

## Scanner, oracle and typed-key checkpoint, 2026-11-11

Signed core source `5b8a1f9ac109e758088fbf24035454d2b6e73f64` rejects a
block-level scalar whose required mapping key expires without a colon.
The original input was accepted through value, streaming and CST parsing;
all three now reject it. Valid indented values, mapping siblings and
indentless sequences remain covered. The changelog records the breaking
acceptance change.

The development-oracle follow-up at `0c987e3` handles trailing line
separation and legal inline tabs after sequence markers. Both spelling
repairs compose, preserving the complete candidate value and requiring a
fresh archive parse to agree. Scalar content remains untouched. All 152
properties, seventeen alloc-only oracle tests, fourteen differential
regressions and twelve compatibility regressions pass with both seed corpora.

The later production fix `e78a36b` preserves written explicit typed-map
keys when AST fallback consumes trailing separation. A tag elsewhere no
longer respells `0x1F` as "31" or `~` as "null". Only the key-text slice
changes; diagnostic spans and canonical `Value` keys retain their contracts.
Both new regressions fail before the fix. All eight typed-key tests and
the exact original 188-byte sanitizer replay pass afterward. The original
bytes are retained in both the crate fixtures and fuzz regressions.

At signed checkpoint `6d2f324db7f04b5788deaf2393ab58b7727e5664`, the
complete batteries pass 6,745 std, 6,723 baseline alloc and 7,064 portable
feature tests, with zero failures or ignores. All 590 doctests pass with
eight existing ignores. Strict clippy and the unchanged 81-entry complexity
baseline pass. All twelve isolated bare-metal checks pass across stable
and Rust 1.89, baseline and portable features, on three architectures.
All five products pass strict clippy, tests, doctests and cargo-vet against
the corrected core. All thirty family hygiene checks pass. The official
clean package is 1,319,801 bytes against the unchanged 1,320,000-byte limit.

The [evidence record](ecosystem-ci-repair-evidence-2026-11-11.json) retains
source identities and a verified raw-output archive. The thirty-target
inventory keeps its original source identities and outcomes. Fresh one-hour
oracle campaigns use immutable signed snapshots of the corrected source.
Final hosted CI, sustained final-source acceptance, independent review and
registry checks remain open. The MCP immutable-tag decision remains held.
Daybreak access is not an acceptance prerequisite.

## Plain continuation indentation follow-up

Signed fix `6b913fe` rejects tabs that precede required plain-scalar
continuation spaces. The exact 12-byte input previously passed value,
streaming and CST parsing; its failing-first regression now rejects it
through all three paths. Tabs after required spaces, in content, and on
blank or comment lines retain their behavior.

The stricter check exposed a formatter layout defect in official case
UV7Q. Reindenting its parent left too few spaces before a continuation tab.
The formatter now supplies the required prefix spaces. Its new regression
passes with LF, CRLF and lone CR at indentation sizes two and four, including
idempotence. The existing formatter refusal set is unchanged.

At signed checkpoint `a57b7198669e41fe3034db4e44f2e274baf96388`, complete
batteries pass 6,748 std, 6,726 baseline alloc and 7,067 portable tests,
with zero failures or ignores. All 590 doctests pass with eight existing
ignores. Strict clippy and the unchanged 81-entry complexity baseline pass.
All retained differential and compatibility findings and their current
corpora pass sanitizer replay. The official clean package is 1,319,847 bytes
within the unchanged 1,320,000-byte limit.

All twelve stable/MSRV bare-metal checks and all twenty native product
and audit gates pass against this checkpoint. The thirteen-case corpus
passes through all six actual consumer interfaces. Thirty-one Node WASM
tests pass. The optimized module is 614,712 bytes raw and 212,267 bytes with
gzip level 9, below the unchanged 628,759 and 219,025-byte limits. The
installed tarball passes TypeScript, Node and 21 Chromium checks. The first
browser attempt did not expose a debugging port; a separate startup probe
succeeded and one repeat passed. Both logs remain in the evidence, and the
startup failure's cause is unconfirmed.

The subsequent duplicate-key finding remains unresolved. Streaming returns
both written keys for `null: 1` followed by `~: 2`; a tag elsewhere forces
AST fallback and canonical duplicate handling keeps only the last. The
[proposed decision](adr/0019-typed-mapping-duplicate-identity.md) records
both choices and their required checks. Its original 88 bytes and failed
campaign remain in the raw evidence. No oracle exception hides this result.

## Empty-line oracle follow-up

Development-only fix `1ca7573` retains the six-byte tab-only final-line
finding. Archived libyaml rejects that separation after an empty sequence
item. The repair changes only whitespace tokens on otherwise empty lines;
quoted and block scalar tokens remain untouched. Callers still require
unchanged candidate semantics and complete fresh archive-value agreement.
The new test fails before the repair. All 153 properties, eighteen alloc
oracle tests, strict clippy and the original sanitizer replay pass. All
retained findings and current corpora also pass replay.

The clean package from the signed checkpoint is 1,319,876 bytes under
the unchanged 1,320,000-byte limit.

Production source and Cargo declarations remain byte-identical to
`a57b719`; the complete batteries, product and bare-metal results retain
that production scope. Final-source sustained campaigns, the duplicate-key
decision, independent review and release acceptance remain open.

## Runtime and oracle assurance follow-up

Signed core fix `48f2b96` extends the empty-line repair to plain-scalar
tokens that consume a trailing tab-only line. The original six bytes are
retained. The expanded regression failed before the change and passes
with LF, CRLF and lone CR. All 153 properties, eighteen alloc oracle tests,
strict clippy and both complete replay inventories pass. Candidate values
and fresh archive values still have to agree in full. Production source
and Cargo declarations remain identical to `a57b719`. The clean signed
package is 1,319,791 bytes within the unchanged 1,320,000-byte limit.

WASM signed source `2ed44f3aee8d76128b903b57d1371332e511a12f` adds
34 shared public-boundary checks. They execute against actual optimized
Node, Bun and Deno exports; classic Chromium passes 55 combined checks.
The installed bundler consumer runs the same contract before its memory
workload. Numeric safety, non-finite projection versus validation, tags,
UTF-8 spans, cycles, shared references and atomic refused edits are covered.
Public exports, Cargo features, MSRV and dependencies remain unchanged.

The resource workload checks both WASM linear memory and JavaScript heap
after collection. Normal operations must stay within 131,072 and 524,288
bytes of growth after warm-up. Retaining real documents and retaining
parsed JavaScript values each breach the same unchanged budgets; losing
heap observations also fails. Four Node tests, three installed-browser
tests and three shutdown regressions pass. The cleanup correction
`9ab1502` reaches a private browser process group's descendants after its
parent exits. Its failing-first regression and actual browser consumers
pass after the fix.

The final WASM export, package, size, resource and shutdown battery passes
all nine checks. The optimized module remains 614,712 bytes raw and
212,267 bytes at gzip level 9. Deno 2.9.8 and Bun 1.4.4 installers are
pinned to reviewed immutable official action commits. The candidate CI
retains runtime reports and repeats these checks on Linux. The
[assurance receipt](ecosystem-wasm-assurance-2026-11-11.json) records raw
log hashes and its archive. Local timing observations do not establish
a dedicated-host latency baseline, registry acceptance or human review.

## MCP work ownership and candidate checks, 2026-11-11

The [lifecycle evidence](ecosystem-mcp-lifecycle-2026-11-11.json) retains
source identities and raw failures, corrections and CI observations.
The signed blocking-work correction `55384dc` reserves one of 64
process-wide slots until the actual closure ends. Controlled timeout and
cancellation cases fail before the guard and pass afterward; refused work
cannot execute, and completion or panic returns capacity. The full battery
passes 135 tests with no failures or ignores. Strict clippy, dependency
policies and the unchanged Rust 1.89 floor pass. The MSRV check uses the
installed SDK 26.5 because the host linker cannot read SDK 27.2's new
architecture declaration; the initial host-linker failure is retained.

Signed MCP source `55ef95b` adds a bounded fuzzer for the actual private SDK
session manager. It retains transports and models create, restore, close,
concurrent admission and reuse. The first sanitizer smoke completes 119,419
executions in 61 seconds without a finding. All three MCP targets build
and replay their seeds; the original tool regression also replays. Adding
three direct fuzz-only edges leaves all 197 resolved packages and their
features identical. This does not establish wire-framing fuzz coverage.

The Windows example launcher now resolves the Bash executable explicitly.
The failing hosted run is retained. At core `68999cd`, all 80 checks pass
with ten conditional skips, including the three-OS native consumer matrix,
all four real-server MCP examples on Windows, WASM runtimes/resources,
six-consumer parity and focused Miri. That candidate pins MCP `2834333`;
the later worker and fuzz commits require their own hosted receipt. The
new candidate workflow also exercises all three MCP sanitizer targets.

The detached held-release recovery uses registry core `=0.0.60`, with no
source override. Its 134-test battery, strict Inspector 2.14.0, dependency
policies, source/version gates and local package assembly pass. The eight
session and three blocking-work cases pass after observed failures. All
three locks pass refreshed database `fd6f39814c4d3c359fdbe223d0280dec4933f78f`;
the original fuzz lock fails the same scan. This is a separate candidate,
not a changed tag or a published release. It does not repair the core/CST
findings that need v0.0.61. Independent review and source/ref recovery remain
open.

The old mixed-source fuzz dispatcher has been superseded, preserving its
18 passing, two failing, three interrupted and seven unstarted records.
Interrupted receipts retain lower bounds or unknown durations. The current
signed inventory contains 31 targets; every target needs a fresh sustained
receipt. The pending 88-byte typed-duplicate finding remains a failure,
with no oracle exception or inferred semantic approval.

## Protocol and held-source follow-up, 2026-11-11

The [follow-up record](ecosystem-protocol-recovery-2026-11-11.json) retains
raw observations, a signed recovery bundle and exact source identities.
LSP `39c93fa` accepts ASCII case-insensitive `Content-Length` field names.
Three actual-server tests receive parse errors before the fix and initialize
successfully afterward. All 76 native tests and nine doctests pass, along
with strict clippy, dependency policies and documentation gates. Library
code, dependency features and Rust floor are unchanged.

A fresh compatibility campaign finds a nine-byte input with lone-CR line
breaks. The existing tab-separation divergence classifier used `lines()`,
which recognized LF and CRLF but missed lone CR. Correction `e95be7b`
recognizes all three physical line endings and retains the original input
and LF/CRLF counterparts. All eighteen target regression inputs replay
under AddressSanitizer, and strict clippy checks the actual fuzz binary.
No production behavior, accepted-value comparison or divergence class
changes. The failed campaign remains retained; its correction needs a new
sustained receipt. Typed duplicate identity remains unresolved.

The new candidate sanitizer job initially called the satellite runner with
an unsupported `--target-dir`. It now uses the checked-out core runner
from the product directory. Actual local executions pass all three scopes:
tools records 7,825 executions in 20 seconds, HTTP guards 28,753 in two,
and session state fifteen in five. These short runs verify the invocation;
they do not replace the one-hour campaign or a corrected hosted result.

The detached signed MCP recovery is `3561c28`, preserving tag object
`b28d529f8bae59bbe94bdd91d25ec50173c8702b`. Its clean signed-source crate
assembles and verifies 138 files against registry core `=0.0.60`. The 134
native tests, strict Inspector, Rust 1.89 and three refreshed Cargo-lock
scans pass. Original sanitizer seeds replay. The later seven-byte Unicode
edit input still panics in registry core v0.0.60; the exact failed replay
and original input are retained. This is a failed release gate, not an
exception or publication approval. The v0.0.61 candidate already passes
that input. Keep v0.0.60 publication held while the source strategy and
independent review remain unresolved.

## Full family campaign preflight, 2026-11-11

Six new matrix guards fail against the earlier core-only composition and
pass after discovering every pinned repository. The actual local inventory
yields 31 targets. Wrong checkouts, altered tracked fuzz manifests, omitted
products, unsafe target names and empty inventories are refused. Actionlint,
workflow security analysis and all 22 historical-dispatch tests pass.

The new full-family workflow is an opt-in Security dispatch. It uses the
core campaign driver for every product and preserves the known 88-byte
typed-duplicate input as a real comparison failure. Its one-hour runtime
receipts remain pending; the unchanged weekly workflows and local campaign
records retain their own sources and results. No release or review gate is
waived by the added execution path.

## Campaign cache isolation correction, 2026-11-12

The first full-family inventory discovered all 31 targets and started its
matrix. CodeQL then reported high cache-poisoning alerts
[66](https://github.com/sebastienrousseau/noyalib/security/code-scanning/66),
[67](https://github.com/sebastienrousseau/noyalib/security/code-scanning/67)
and [68](https://github.com/sebastienrousseau/noyalib/security/code-scanning/68):
manual-dispatch context can expose the default branch's cache scope to
external candidate code even with read-only repository permissions.
No restored cache is needed for that exposure.

The manual run's findings and source/job records are retained, and that
execution path is stopped. The family workflow now accepts only the
`full-fuzz` label event on a PR, checks out its exact head, and keeps all
untrusted execution in the PR cache scope. The Security workflow's original
dispatch and scheduled behavior is restored. No alert is dismissed and
no suppression is added. The closing branch CodeQL run
[43129554384](https://github.com/sebastienrousseau/noyalib/actions/runs/43129554384)
completed successfully on `17d064f`; the branch's open-alert query returned
an empty list on 2026-11-12. Partial and unstarted campaigns receive no
one-hour credit; completed local receipts retain their exact sources.

## BOM oracle scope, 2026-11-12

The [oracle receipt](ecosystem-bom-oracle-2026-11-12.json) retains raw
failures, corrections and code hashes. The full-family compatibility
campaign retained the six-byte
`crash-6fc6e95b29a08538c0acc522513024451e5f5092`. The archived parser accepts
an unquoted content BOM; the core correctly refuses it under YAML 1.2.2
section 5.2. A private classifier recognizes only that located scanner
error at an established unquoted content position. It leaves possible
document prefixes, quoted content, unrelated errors and invalid offsets
outside the rule. Accepted-value comparisons remain unchanged.

The new guard fails before implementation and passes afterward. Its two
unit tests pass with std and alloc-only core dependencies in separate
build directories; strict std/alloc clippy and all nineteen compatibility
regression inputs pass. The shared classifier and test harnesses live in
private fuzz crates, keeping the published package and production bytes
unchanged. Candidate CI runs both unit-test profiles separately.
This replay repairs an oracle finding; a new sustained campaign remains
required. The duplicate-key policy remains separate open work.

## Complete name bounds, 2026-11-12

The 1,056-byte roundtrip finding
`crash-3eee8e14d8486eecc2c0280af136869ea275d727` exposed inconsistent
tag accounting. A secondary tag parsed with a 1,027-byte stored URI,
but its emitted verbatim spelling failed to reparse. The scanner now
checks the resolved handle and suffix before cloning a declared prefix.
A sibling anchor-name check covers the final byte at EOF.

Both new boundary tests failed before the correction because all three
consumers accepted 1,025 bytes. The focused std run now passes 35 tests,
including exact-limit and oversized ASCII/UTF-8 names, primary, secondary,
verbatim and declared-prefix tags, and accepted-value roundtrips.
The original input is retained in both fuzz regression corpora. The
[proposed ADR](adr/0020-enforce-complete-name-bounds.md) records the smaller
acceptance set. A separate extraction moves the resolution body unchanged
into a private helper and reduces its recorded function size from 134 to
120 lines, with no ceiling increase.

The final scanner passes 6,751 std, 6,732 baseline alloc and 7,073 portable
tests, with no failures or ignored tests in those batteries. All twelve
stable/MSRV bare-metal combinations pass. AddressSanitizer replays pass
all ten std and two alloc-only roundtrip regression inputs. The authoritative
package gate passes at 1,319,928 bytes against the unchanged 1,320,000-byte
ceiling. These are local checks, not sustained-campaign or release approval.
The [receipt and raw archive](ecosystem-name-bounds-2026-11-12.json) retain
the exact sources, failures and final verification scopes.

## MCP shared blocking capacity, 2026-11-12

Signed MCP `26af763` adds a real SDK integration test without changing
production code or dependencies. It holds the runtime's blocking pool,
queues 64 timed-out calls across two independent handlers and requires
the next call to report the shared busy limit. After the pool drains,
a fresh handler must return the expected parsed document.

A temporary per-call semaphore fault made that shared-capacity assertion
fail. The production file was restored byte-for-byte; all 136 native tests,
strict clippy, deny, vet and hygiene gates pass. Hosted
[Inspector run 43134279149](https://github.com/sebastienrousseau/noyalib-mcp/actions/runs/43134279149)
also passes. The candidate manifest pins the signed test commit. The same
[receipt](ecosystem-name-bounds-2026-11-12.json) separates this SDK evidence
from the later core scanner verification.

## Implementation and acceptance

The next signed MCP checkpoint fixes session admission in `839e637` and
corrects executable references in `41d49d1`. A cap of one previously
admitted two concurrent creations. The deterministic queued-reader
reproducer fails before the shared creation/restore admission lock and
passes afterward. All eight session tests and the full 132-test MCP
battery pass with no failures or ignores; strict clippy, deny and vet pass.
No public API, feature, MSRV or dependency changes are involved.

Four real-server example scenarios pass, including a server that answers
correctly and then exits with failure. File demos operate in private
scratch roots, preserve surrounding source and verify readback. The
references now describe the actual six tools, prompts, resources,
`--root`, readable tool failures and the limits of timeout cancellation.
The [MCP boundary receipt](ecosystem-mcp-boundaries-2026-11-11.json)
retains failures, corrections, exact sources and raw logs. Three-OS hosted
example acceptance remains separate.

The [new performance observations](ecosystem-performance-2026-11-11.json)
retain three repetitions of 70 scenarios, complete raw Criterion/JS
samples, source/binary/workload identities and cold CLI/LSP/MCP cycles.
All 21 benchmark executions pass. The added cold CLI probe initially
omitted `--stdin`; its failure and corrected separate runs remain visible.
Own fuzz workers were absent during the in-process benchmark collection;
historical workers resumed before the cold cycles. Substantial unrelated
host load and run-to-run variation prevent a dedicated-host timing claim
or justified latency threshold. The old MCP and LSP microbenchmarks
exercise core primitives, not live protocol dispatch.

All eleven signed-source WASM resource outcomes match expectations:
three normal Node and three normal installed-browser repetitions pass,
real WASM/JS retention faults fail in both engines, and missing Node heap
observations fail. The raw performance/resource and WASM archives are
retained under [evidence](evidence/), with member hashes in their records.
These observations do not close independent review, registry or sustained
final-source campaign acceptance.

“Local evidence” means the named check passed in its stated scope. Missing
platforms, independent review, registry resolution and CI are listed as
remaining evidence. They are not implied by the local results.

| Task | Local evidence | Remaining acceptance |
|---|---|---|
| S1 | Fourteen tracked Cargo locks pass the refreshed RustSec revision `2c684a5d6746d4508eff9e3cb529b3e33f56d438` after correcting both CLI locks to rustix 1.1.6. The initial failure and corrected receipts are retained. All six current root graphs passed cargo-deny and the existing cargo-vet policy. VS Code and WASM test npm scopes reported zero advisories; MCP wrapper declares no dependencies. | Retain the same gates on registry-resolved locks and every supported CI scope. Core differential fuzz uses an unmaintained archive oracle; the warning remains visible. |
| R0 | Checked the exact held tag, prepared signed detached recovery `3561c28`, and retained a failing Unicode edit replay in immutable registry core 0.0.60. | Written maintainer decision, exact-source release gates and a closing scan. |
| MCP1 | Patched fuzz Tokio; scanned all three locks; built both targets. The tools campaign ran 8,903,049 inputs in 3,601 seconds with no finding in the log. | Corrected-ref remote scan and merge. |
| D1 | Reconciled published package scope, version references, LSP capabilities, WASM entry points and unverified benchmark claims. | Current scorecard and remaining channel claims need their acceptance evidence. |
| LSP1 | Capability and framed handshake tests agree with the documentation. Neovim and VS Code exercised the advertised methods. | Carry the evidence through CI at the release ref. |
| SHIM2 | Clarified the standalone std facade and the separate alloc-only core compatibility profile. Both ran in their scoped matrix. | Carry the evidence through CI at the release ref. |
| Q1 | Contract ledger links product operations and refusals to the tests that exercise them. | Fill the specifically untested product and deployment combinations in the ledger. |
| Q2 | Thirty-one targets discovered; scheduled runner records elapsed time, inputs, execution-count scope and findings. Interrupted counts are preserved as lower bounds and missing durations remain unknown; four accounting regressions pass in every repo. MCP tools completed a full campaign; HTTP headers ran 958,775 inputs in 61 seconds. The facade runner smoke completed 102,941 inputs. | One-hour receipts for every remaining target and a successful scheduled run. |
| Q3 | Structured LLVM line totals replace the ambiguous last column. Invalid, timed-out and absent measurements remain N/A. The fresh snapshot measured 139 probes, seven N/A, and scored 92.1%. | Repeat on the final signed release sources and registry graph. |
| P1 | The [latest observations](ecosystem-performance-2026-11-11.json) retain three repetitions of 70 scenarios and real cold CLI/LSP/MCP cycles, with raw samples, hashes and variance. Substantial host noise is explicit. | Dedicated-host end-to-end latency and remaining resource cases. |
| P2 | WASM byte and real WASM/JS growth budgets reject padding, retained documents/values and missing observations. MCP admission rejects the deterministic concurrent cap violation. | Measured budgets and injection checks for remaining scenarios and products; no shared-host latency threshold. |
| X1 | Staged CLI install/uninstall, installed VSIX, Node and browser WASM, renamed Rust consumer and MCP offline drill passed. | Fresh-clone, failure and recovery transcripts across the rest of the support matrix. |
| C1 | At the continuation checkpoint, all-feature test binaries pass 6,748 tests with no failures or ignores, including 152 properties. Shared fixtures check semantics and refusals. | Complete the case-level parity and diagnostics ledger; keep intentional loader differences explicit. |
| C2 | Documented loader responsibilities and fixed the recorded runtime defects. The collection-anchor extraction retains behavior under std and alloc characterization suites and reduces the measured complexity baseline. | Independent architecture review of the resulting boundaries. |
| C3 | Complete alloc-only hosted batteries at the continuation checkpoint pass 6,726 baseline and 7,067 portable tests, including seventeen oracle helper tests. Hosted schema composition previously passed 23 tests. Baseline and portable built on three bare-metal targets with stable and Rust 1.89.0. Default, minimal std and alloc float consumers had no active libm. | Rerun the final registry consumer and CI target matrix before release. |
| CLI1 | All-feature CLI probe passed 123 tests. Generated manpages match; staged install, both binary version checks and uninstall passed with a custom target directory. | Hosted Action and package-manager acceptance on the remaining operating systems. |
| CLI2 | A refused rename preserved destination contents and removed the temporary file; mode and symlink tests also pass. | Cross-OS interruption and metadata evidence, with independent review. |
| LSP2 | Tests cover UTF-16 positions, stale changes, initialization, header limits, case-insensitive field names and exit while stdin is open. The corrected binary passes 76 native tests and nine doctests; real client checks passed at their recorded sources. | Resource and latency budgets for editor workloads. |
| LSP3 | Neovim 0.12.6 and installed VS Code 1.141.0 VSIX passed. The VSIX startup bug was reproduced and fixed. | Installed marketplace artifacts and the rest of the claimed OS matrix. |
| MCP2 | The current stdio, HTTP/SSE, SDK, limit and guard battery passes 135 tests; three worker-lifetime cases cover timeout, cancellation and panic. Eight session tests cover deterministic cap enforcement, concurrent create/restore, and reuse. Four actual example scenarios pass, including failed-exit refusal. | Complete revision-by-transport acceptance for disconnect, cancellation, remaining concurrency and supported filesystems; independent review. |
| MCP3 | Published v0.0.59 npm package recovered from the verified native archive offline, refused corrupt cache and ran a real stdio parse. A future npm release gate repeats the drill. | Apply all five channel checks to v0.0.60 after the hold is resolved. |
| WASM1 | Thirty-one real Node WASM tests and 34 shared runtime-contract checks pass. Classic Chromium passes 55 combined checks. Actual Node/browser memory and cleanup fault regressions pass. | Complete final-source hosted and sustained campaign acceptance; keep per-case coverage explicit. |
| WASM2 | Optimized module is 614,712 bytes raw and 212,267 bytes gzip within unchanged limits. Eleven signed-source Node/browser resource outcomes match expectations, including three normal repetitions per engine and real WASM/JS retention faults. | Comparable dedicated-host timing and eventual registry-module acceptance. |
| WASM3 | Installed optimized tarball includes both licenses; TypeScript, Node and webpack/Chromium pass. Optimized Node, Bun 1.4.4, Deno 2.9.8 and classic Chromium execute the shared contract. | Eventual registry artifact and final-source hosted runtime receipts. |
| SHIM1 | The standalone captured contract and extended tag, number and location cases passed. Equivalent compatibility entry points now have a separate benchmark from resolver-profile costs. | Complete extended location, tag and number cases and measure equivalent core overhead. |
| I1 | Eleven shared cases pass in the earlier hosted job; the expanded thirteen-case corpus passes locally through the corrected products. Receipts record Node, Rust and SDK versions and source identities. The workflow builds each checkout from its lock. | Scheduled execution, additional contract cases and final registry-source parity. |
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
