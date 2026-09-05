<!--
SPDX-FileCopyrightText: 2026 Noyalib
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# noyalib ecosystem — road to 10/10

Last reconciled against the tree at **v0.0.21**. Every number in §1 was
counted from the repository at that point, not carried forward; where a
previous revision of this document was wrong, §0 says so.

---

## 0. What changed since the last revision

This document had drifted. Recorded plainly so the corrections are
auditable:

| Claim (previous) | Reality at v0.0.21 |
|---|---|
| 467 public fns | **488** |
| 142 test files | **161** |
| 77 examples | **79** |
| A1 comment mutation → M1 (v0.0.18–0.0.19) | landed in **v0.0.21** |
| A2 reorder, A5 key spans → "planned" | landed in **v0.0.19** |
| A3 extended remove → M2, unqualified | **partly** done — see EPIC A |
| D1 bare-metal → M3 (v0.0.21) | landed in **v0.0.20**, closed #210 |
| #210 open | **closed** |
| Coverage gate "≥95%" | 95% functions, deliberately rebaselined from 96 |

The coverage rebaseline deserves a note rather than a silent number: 96%
left under three functions of slack, and the metric proved unstable
across runs of the *same commit* (78 uncovered then 77). A threshold a
no-op rename can breach measures LLVM's instantiation accounting, not
the test suite. Line (94%) and region (93%) floors were stable and are
unchanged.

---

## 1. Current-state scorecard

| Category | Now | Evidence (counted at v0.0.21) | Gap to 10/10 |
|---|---|---|---|
| **API / functionality** | 9 | 488 public fns; lossless CST editors incl. `set`/`insert`/`remove`/`rename_key`/`rename_anchor`/`swap_items`/`move_item`/`set_comment`/`remove_comment`; streaming; async | #221 **closed** in v0.0.23 — flow-member and sole-entry removal completed sub-ask 4; sub-ask 5 was resolved in v0.0.21 by a structural oracle rather than auto-quoting |
| **Correctness / testing** | 9.5 | 161 test files, 5 961 tests, coverage gate (95 fn / 94 line / 93 region), Miri, differential fuzz vs saphyr | Fuzz is a PR smoke, not continuous; no structured fuzzers for the *editors*; property-test breadth uneven |
| **Performance** | 9 | 16 benches, SIMD, `fast-int`/`fast-float`, `parallel` | No published numbers; no CI regression gate; no criterion baselines |
| **Security / supply-chain** | 9 | cargo-vet, cargo-deny, cargo-audit, CodeQL, OSSF scorecard, REUSE 850/850, `unsafe` forbidden except `simd`; schema-validator hardening pinned by test (v0.0.21); SLSA L3 attestation + keyless sigstore in release.yml; `build.rs` contract CI-enforced (`build-script-contract`); CycloneDX SBOM signed + attested per release (v0.0.29); weekly `cargo hack --feature-powerset --depth 2` sweep (v0.0.29, found the `ariadne`-without-`std` break on day one) | No OpenSSF badge; the schema validator's recursion bound is the `jsonschema` crate's fixed 129 (pinned by test, not caller-configurable — upstream limitation; parser/serializer `max_depth` ARE caller-configurable). Earlier revisions of this row claimed no SBOM, no SLSA, unaudited `build.rs`, and an unconfigurable depth bound generally — all four were stale. |
| **Documentation** | 9 | rustdoc-strict + broken-intra-doc-link gate, `USER-GUIDE.md`, ADRs, 79 examples | No docs.rs feature-matrix proof; no cookbook; no competitive comparison page |
| **no_std / portability** | **10** | `no_std`+alloc; wasm32 and bare-metal `thumbv7em` / `riscv32imac` / `aarch64-unknown-none` build (v0.0.20) **and are gated in CI** (v0.0.21) | — |
| **DX / ergonomics** | 8.5 | miette/ariadne diagnostics, typed path API, recovery | No derive helpers/builders; fix-hints not uniform across the error taxonomy |
| **Interop / ecosystem** | 9 | serde, `compat-serde-yaml` shim, figment, schemars, garde/validator, tokio, sval | Successor position is *earned but unclaimed* — see EPIC F1 |
| **Satellites** | 7.5 | wasm/mcp/lsp/cli all shipping | Each v0.0.x; **`noyalib-mcp` predates the 2026-07-28 MCP spec**; no npm/registry packaging |
| **Release / governance** | 9.5 | ADR-0005 lockstep, signed commits, Keep-a-Changelog | MSRV/deprecation policy undocumented; release partly manual |

**Weighted read:** core ≈ **9.2/10**, satellites ≈ **7.5/10**.

---

## 2. Gaps & issues (grounded)

### 2.1 Project-tracked

- **#221 — CST edit API.** Sub-asks 1 (comment mutation), 2 (`rename_key`
  + key spans) and 3 (reorder) are **done**. Remaining:
  - **(4) Extended `remove`** — multi-line values, nested collections and
    nested sequence items *do* work. Sole entries and flow-collection
    members are refused; `remove_subtree` does not exist.
  - **(5) Quoting-aware fragment emit** — **closed in v0.0.29.**
    The *`_value`* inserters already quoted via `Emit`; `set` gained a
    structural oracle in v0.0.21; and v0.0.29 closed the remainder:
    `insert_entry` runs under the same oracle (its new-key splice was
    entirely unguarded — a lone `U+000D` in the fragment escaped, and
    the key half spliced verbatim), keys refuse `<<`/non-printables and
    auto-quote like `rename_key`, and `guarded_insert` pins container
    growth to exactly one entry, so `push_back("s", "v\n  - w")` can no
    longer append two items *inside* the elided container. Pinned by
    `tests/cst_insert_containment.rs`. The lesson from the v0.0.21
    `remove` bug applied: every fast path now has an oracle, not an
    intuition.
- **#210 — bare-metal `no_std`.** **Closed in v0.0.20.**

### 2.2 Analysis-surfaced

- **A `remove` data-loss bug, found and fixed in v0.0.21.** The oracle
  guard only ran for multi-line edits; single-line entries took an
  unguarded path. In a flow collection an entry shares its line with its
  siblings *and its parent*, so `remove("a.x")` on `a: {x: 1, y: 2}`
  deleted the whole document and returned `Ok`. The lesson generalises:
  **any fast path that skips the oracle needs a proof, not an intuition.**
- **Schema validator hardening was untested, not absent.** Measured at
  v0.0.21: external `$ref` *is* refused, and recursion *is* bounded (the
  `jsonschema` crate stops at depth 129). Both properties come from how the
  dependency is configured — `default-features = false` — rather than from
  anything this crate asserts, so a feature flag or a version bump could
  have removed either silently. **Fixed in v0.0.21** by
  `tests/schema_hardening.rs`, which pins both. An earlier revision of this
  roadmap claimed the bounds were missing; they were merely unguarded.
- **`build.rs` unaudited — stale; closed before this revision.** The
  `build-script-contract` CI job greps the script for network /
  filesystem / process capability and fails on any reach beyond its
  documented contract (declare cfgs, read `$RUSTC --version`, read one
  env var).
- **Fuzzing is a PR smoke.** No corpus accretion, no continuous run, and
  no structured fuzzers for the editors — only the parser is differentially
  fuzzed against saphyr.
- **No performance regression gate**, and no published numbers to anchor
  the SIMD / `fast-float` claims.
- **SIMD `unsafe` audit.** `simd`/`nightly-simd` opt out of
  `unsafe_code = forbid` with no published soundness note.

---

## 3. Implementation plan — core crate

Format: **Epic → tasks → acceptance → effort (S≤1d / M≤1wk / L>1wk) →
risk → categories moved.**

### EPIC A — Finish the CST surgical-edit API (closes #221)

- ~~**A1. Comment mutation**~~ — **done, v0.0.21.**
  `set_comment(path, position, text)` / `remove_comment(path, position)`
  with `CommentPosition ∈ {Inline, Before}`, leading blocks written at the
  node's own indentation. *Deferred:* a separate `insert_comment` and a
  `Trailing` position — `set_comment` covers both intents today; add them
  only if a caller shows the distinction matters.
- ~~**A2. Sequence reorder**~~ — **done, v0.0.19** (`swap_items`, `move_item`).
- **A3. Extended `remove`** — remaining: `remove_subtree`, sole-entry
  removal yielding a valid empty collection, and flow-collection members.
  - *Accept:* flow-member removal rewrites the flow collection rather than
    the line; a fuzz target removes a random path and asserts
    `value == original − path`; every refusal leaves the source
    byte-identical (already tested).
  - M · medium risk · **API, correctness**
- ~~**A4. Fragment containment**~~ — **done, v0.0.21.** Investigation
  changed the shape of this task twice.
  - Routing `set` through `Emit` would have **broken its documented
    contract**: it splices verbatim on purpose, and `set(p, "{x: 1}")`
    turning a scalar into a mapping is legitimate use. The safe route already
    exists — `set_value` renders a typed value, and was verified to
    round-trip every hazardous input exactly (`"true"`, `""`,
    `"v # x"`, `"v\nc: 3"`, `"x: y"`, `"- item"`).
  - The real defect was narrower and worse: a fragment could reach
    **outside its path**. `set("a", "v\nc: 3")` gave the document a new
    key `c` and returned `Ok`, because the result is valid YAML. **Fixed**
    by a structural oracle — the document's shape outside the edited path
    must be unchanged.
  - The oracle compares *shape*, not values, and that distinction was
    found the hard way: a value-comparing first version wrongly rejected
    edits to **anchored** values, whose aliases legitimately change
    elsewhere. It also parses fallibly, because an invalid splice commits
    optimistically by design and surfaces via `validate`.
  - **Completed, v0.0.21.** `push_back` and `insert_after` had the same
    hole — `push_back("s", "v\nqq: 7")` appended to the sequence *and*
    gave the document a top-level `qq`. Both now run through
    `guarded_insert`, which elides the container being inserted into
    (its shape must change) and requires everything else to match; for
    `insert_after("items[2]", ..)` the container is the parent sequence.
    `insert_entry` was already covered, delegating to the guarded `set`.
  - Property tests at 512 cases each now generalise the round-trip claim
    beyond enumerated inputs: arbitrary scalars survive `set_value` at
    top level and nested, and `set` never silently corrupts — whatever
    it returns, siblings survive and the entry count holds.
- ~~**A5. Read-only key spans**~~ — **done, v0.0.19** (`key_span`).

### EPIC B — Testing to a defensible 10

- **B1. Coverage floor.** Keep 95/94/93 rather than chase 100. The v0.0.19
  experience is the argument: the function metric moved between runs of an
  identical commit, so a higher floor buys flakiness, not assurance.
  Instead: **B1′ — make the metric trustworthy** by pinning the nightly
  used for coverage, so run-to-run variance is attributable. S · low ·
  **testing**
- **B2. Editor fuzzing** — structured `arbitrary` targets per mutator
  (set/insert/remove/rename/reorder/comment): apply a random edit, assert
  the oracle invariant or a clean refusal. The v0.0.21 data-loss bug is
  exactly what this catches. M · low · **testing, correctness**
- **B3. Continuous fuzzing** — OSS-Fuzz, or a nightly ≥30 min job with a
  persisted corpus. M · low · **testing, security**
- **B4. Property-test parity** — a checklist test that fails when a public
  codec entry point has no round-trip property. M · low · **testing**

### EPIC C — Performance, measured and defended

- **C1. Published benchmarks** — criterion baselines + `BENCHMARKS.md` with
  numbers against `serde-saphyr`, `yaml-rust2` and (as a historical
  baseline) `serde_yaml`, on a stated corpus and machine. S–M · low ·
  **performance, docs**
- **C2. CI regression gate** — fail on >X% regression. Budget for runner
  noise: the CI *duration* monitor in this repo already demonstrates the
  failure mode, where a single slow run tripped a 1.1× threshold that the
  rolling average then absorbed. Use a median-of-N, not a single sample.
  M · medium · **performance**
- **C3. SIMD soundness note** — document the `unsafe` invariants; Miri over
  the scalar-fallback equivalence tests; a `simd == scalar` differential
  property. M · medium · **performance, security**

### EPIC D — no_std / portability

- ~~**D1. Bare-metal build**~~ — **done, v0.0.20.** Four root causes, two
  more than #210 documented: dependencies pulling `std` unconditionally;
  `FxHashMap`/`FxHashSet` being std-only aliases; `indexmap`'s default
  hasher being std-only (worked around with a cfg-defaulted alias so no
  public signature changed); and `core` having no `f64::fract`/`mul_add`.
- ~~**D2. Keep it built**~~ — **done, v0.0.21.** `shared-no-std.yml` gains
  a `bare-metal` matrix over `thumbv7em-none-eabihf`,
  `riscv32imac-unknown-none-elf` and `aarch64-unknown-none`. Recorded there:
  wasm32 passed *throughout* the #210 bug, because the target has `std`
  available and masked three dependencies pulling it in unconditionally —
  so wasm32 alone was never sufficient cover.
- **D3. `alloc`-free surface audit** — document which APIs need `alloc` vs
  pure `core`. M · medium · **no_std, docs**

### EPIC E — Security / supply-chain to 10

Updated against 2026 practice: SBOM alone is table stakes; provenance and
VEX are what auditors now ask for.

- **E1. Per-release SBOM** — CycloneDX *and* SPDX (`cargo-cyclonedx` /
  `cargo-sbom`) as release assets. S · low · **security**
- **E2. SLSA build provenance** — GitHub's `attest-build-provenance` reaches
  SLSA Build L2 directly and **L3 via reusable workflows**, which this repo
  already uses throughout. Attest the release artifacts and the SBOM
  predicate. S–M · low · **security, governance**
- **E3. VEX statements** — publish exploitability assessments for advisories
  that cannot be fixed upstream. The fleet has a live example: an advisory
  reachable only through a dependency the crate never called. VEX is how
  you say that formally instead of in a commit message. M · low ·
  **security**
- **E4. OpenSSF Best Practices badge** — passing → silver. S · low ·
  **security, governance**
- ~~**E5. `build.rs` contract**~~ — **done, v0.0.21.** The module comment
  states what it may do (declare cfgs, read `$RUSTC --version`, read one
  env var) and what it must never do; a `build-script-contract` CI job
  asserts the latter by grepping for network and filesystem capability, a
  subprocess count above one, and any `[build-dependencies]`. Grepping for
  capability means a future edit has to defeat CI rather than a reader's
  attention.
- ~~**E6. MSRV & deprecation policy**~~ — **done, v0.0.21.**
  `docs/MSRV-AND-DEPRECATION.md`. The clause that mattered: a dev-dependency
  outrunning the floor is a *decision*, not a mechanical fix — raising a
  user-facing promise to accommodate a test tool should be deliberate, not
  a way to turn a job green.

### EPIC F — Documentation & interop leadership

- **F1. Claim the successor position.** `serde_yaml` is archived and
  `serde_yml` now ships a shim pointing here; noyalib is already named
  alongside `serde-saphyr` and `yaml-rust2` in community "what should I use
  now" threads. Convert that into `MIGRATING-FROM-SERDE-YAML.md` with a
  shim parity table, a one-line `Cargo.toml` swap and a compile-tested
  example. **Highest adoption ROI on this list.** M · low · **interop, docs**
- **F2. Honest competitive comparison** — a page stating where each library
  wins. `serde-saphyr` deliberately has *no* `Value` DOM and deserialises
  straight into typed structs, which is faster and leaner for
  `from_str::<T>` and unable to do what a CST does. noyalib's differentiator
  is the lossless editing surface plus schema validation. Say so, including
  where a reader should pick the other one. S · low · **docs, interop**
- **F3. The Norway problem, stated.** `serde-saphyr` markets its typed-schema
  approach as the answer to `NO → false`. Document noyalib's resolution
  rules and how to get strict behaviour, rather than leaving readers to
  infer it. S · low · **docs, correctness**
- **F4. Task-oriented cookbook** — runnable, doctested recipes. M · low ·
  **docs, DX**
- **F5. docs.rs feature-matrix proof** — CI builds docs under each major
  feature combination. S · low · **docs**

### EPIC G — Agent-era readiness *(new)*

The 2026-07-28 MCP specification is the largest revision since launch: a
stateless protocol core, an Extensions framework, Tasks, MCP Apps,
authorization hardening and a formal deprecation policy. It also lifts
tool `inputSchema`/`outputSchema` to **full JSON Schema 2020-12** —
`oneOf`/`anyOf`/`allOf`, conditionals, `$ref`/`$defs` — and requires that
implementations **not auto-dereference external `$ref` URIs** and **bound
schema depth and validation time**.

noyalib already ships a JSON Schema 2020-12 validator. That makes this an
opportunity rather than a chore — but the hardening the spec requires is
currently absent.

- ~~**G1. Pin the schema-validator hardening**~~ — **done, v0.0.21.**
  Measurement changed the shape of this task: the protections already
  existed, undocumented and untested. `tests/schema_hardening.rs` now pins
  six properties — external `$ref` refused, that refusal being fast enough
  to rule out a network attempt, recursion bounded at depth 500/2 000/10 000
  without stack exhaustion, the bound naming itself in the error, local
  `$ref`/`$defs` still resolving, and ordinary schemas unaffected — and
  `schema_validate.rs` documents them as contract.
  - *Still open:* a **configurable** depth bound and an explicit
    validation-time budget. Today's limit is `jsonschema`'s, not ours, so a
    caller cannot tighten it. S–M · low · **security, interop**
- **G2. `noyalib-mcp` → 2026-07-28 conformance.** Stateless core, Tasks for
  long edits, Extensions, OAuth 2.1. Add an MCP Inspector conformance job.
  L · medium · **satellites, ecosystem**
- **G3. Tool-schema generation.** noyalib already has `schemars`; expose a
  helper that emits an MCP-shaped tool schema from a Rust type, so agent
  authors get validated tool definitions without hand-writing JSON Schema.
  M · low · **interop, DX**
- **G4. Agent-safe editing profile.** Agents edit configuration files
  unattended. Package the existing guarantees — oracle-guarded mutators,
  byte-faithful round-trip, refusal over silent corruption — as a documented
  "safe for unattended edits" profile, with the failure modes stated. The
  v0.0.21 data-loss bug is precisely the class this profile must exclude.
  S–M · low · **docs, DX, correctness**

---

## 4. Implementation plan — satellites

### 4.1 `noya-cli` (noyafmt, noyavalidate)

- **CLI-1.** `noyaedit` exposing the CST mutators as a jq-style path CLI.
- **CLI-2.** Shell completions (`clap_complete`) + man pages (`clap_mangen`).
- **CLI-3.** Prebuilt binaries (`cargo-dist`), `cargo-binstall` metadata,
  Homebrew tap, official `pre-commit-hooks.yaml`.
- **CLI-4.** `--format json|sarif` for `noyavalidate` so it lands in code
  scanning.

### 4.2 `noyalib-mcp`

Now gated on **G2** above — conformance to the 2026-07-28 spec comes before
feature breadth.

- **MCP-1.** Cover the complete CST edit API as it lands.
- **MCP-2.** Resources + prompts, not just tools.
- **MCP-3.** Publish to the MCP registry with a signed release.

### 4.3 `noyalib-lsp`

- **LSP-1.** Duplicate-key diagnostics on `key_span`; code actions backed by
  the mutators — including the new comment mutators.
- **LSP-2.** Semantic tokens, symbols, folding, schema-aware hover.
- **LSP-3.** VS Code extension (Marketplace + OpenVSX); Neovim/Helix/Emacs.

### 4.4 `noyalib-wasm`

- **WASM-1.** npm package with TypeScript types, ESM + CJS, size-tracked.
- **WASM-2.** Expose the full lossless-edit API; a live playground.
- **WASM-3.** `wasm32-wasi` + Deno/Bun smoke tests.

---

## 5. New feature proposals

Ranked value-to-effort; each semver-additive and feature-gated.

1. **`Document::diff` / structural patch** — a minimal comment-preserving
   patch between two documents. Powers CLI/LSP/MCP "apply change" flows and
   3-way merges, and is the natural substrate for agent edits (**G4**).
2. **JSONPath / JMESPath-style query** over `Value` and `Document`.
3. **Deterministic canonical form** (`to_canonical`) for hashing and signing
   payloads — increasingly relevant as config gets signed rather than
   trusted.
4. **YAML↔JSON lossless-where-possible converters**, with anchor/merge
   behaviour documented.
5. **Schema *inference*** — emit a JSON Schema or `struct` skeleton from a
   sample document. Pairs with **G3**: infer, then serve as a tool schema.
6. **Merge-key (`<<`) round-trip policy** as a documented, tested surface.
7. **Editor-grade recovery surface** — expose partial parses as a public
   best-effort API.

---

## 6. Sequencing & milestones

Releasable under the lockstep contract (core + satellites move together).

- **M1 — "Finish the editors" (v0.0.18–0.0.19).** ✅ A2 reorder, A5 key
  spans, B1 coverage gate, F5 docs-matrix groundwork. *A1 slipped to
  v0.0.21.*
- **M2 — "Reach everywhere" (v0.0.20).** ✅ D1 bare-metal (*closed #210*).
- **M3 — "Safe to automate" (v0.0.21).** ✅ A1 comment mutation; the
  `remove` data-loss fix. **Open:** A4 quoting-aware `Emit` — the last
  correctness hazard — and **G1** validator hardening.
- **M4 — "Provably trustworthy" (v0.0.22).** A3 remainder, B2 editor
  fuzzing, E1–E6 (SBOM, SLSA provenance, VEX, badge, `build.rs` contract,
  MSRV policy), D2 embedded CI targets.
- **M5 — "Own the niche" (v0.0.23 → 0.1.0).** F1 succession guide, F2
  comparison, F3 Norway stance, G2–G4 agent readiness, C1–C2 published and
  gated performance, the satellite build-outs. Cut **0.1.0** when the edit
  API is complete, the successor story is documented, and MCP conformance
  is demonstrated.

### Category → milestone it reaches 10

| Category | Reaches 10 at |
|---|---|
| no_std / portability | M2 ✅ (M4 to keep it there) |
| API / functionality | M3–M4 (needs A3 + A4) |
| Correctness / testing | M4 |
| Security / supply-chain | M4 |
| Interop / ecosystem | M5 |
| Documentation | M5 |
| Performance | M5 |
| Satellites | M5 |

---

## 7. Effort summary

Roughly 6–9 focused weeks to M4, and M5 is dominated by satellites rather
than core work. The two items with the best ratio of value to effort are
**F1** (the successor guide — adoption, near-zero risk) and **G1**
(validator hardening — small, and a prerequisite for any MCP claim).

The single highest-risk item remains **A4**: it touches every mutator, and
the failure it prevents is silent.

---

## 8. What I'd *not* do

- **Chase a 100% coverage floor.** Demonstrated flaky at 96; the marginal
  5% is mostly unreachable error arms, and a flaky gate teaches people to
  ignore gates.
- **Re-implement a CSS/JS/general minifier, or any large third-party
  surface, to avoid a dependency** — unless the dependency carries an
  unfixable advisory *and* the replacement is small enough to test
  exhaustively. (The sibling `html-generator` case met that bar; most will
  not.)
- **Chase `serde-saphyr` on no-DOM deserialisation throughput.** Different
  design centre. Compete on lossless editing, schema validation and
  portability, and say so honestly.
- **Add a plugin/scripting surface.** A YAML library that executes user code
  inherits an attack surface disproportionate to the benefit.
- **Support YAML 1.1 semantics by default.** Legacy behaviours stay behind
  explicit flags; the default should be predictable.
