<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib ecosystem — road to 10/10

An evidence-based analysis of the noyalib core crate and its four satellites
(`noyalib-wasm`, `noyalib-mcp`, `noyalib-lsp`, `noya-cli`), the gaps that
stand between "excellent" and "10/10 in every category", and a phased,
reviewable implementation plan.

> Honest framing: this is already a top-decile Rust library. The core clears
> a CI gate most crates never attempt — coverage ≥95%, Miri, MSRV 1.86,
> `no_std`, differential fuzzing, `cargo-vet`/`deny`/`machete`,
> `cargo-semver-checks`, REUSE, and a full 3-OS × stable/nightly matrix.
> The work below is about closing the *last* margins and adding strategic
> capability, not fixing something broken.

---

## 1. Current-state scorecard

Grades are current, evidence-based. "→ 10" is what this plan targets.

| Category | Now | Evidence | Gap to 10/10 |
|---|---|---|---|
| **API / functionality** | 8.5 | 467 public fns; lossless CST editors (`set`/`insert`/`remove`/`rename_key`/`rename_anchor`); streaming; async | CST edit API incomplete (#221): comment mutation, sequence reorder, extended `remove`, quoting-aware `Emit` |
| **Correctness / testing** | 9.5 | 142 test files, coverage ≥95% gate, Miri, differential fuzz vs saphyr | Coverage floor is 95% not 100%; fuzz is a 10 s smoke, not continuous; property-test breadth uneven |
| **Performance** | 9 | 16 benches, SIMD, `fast-int`/`fast-float`, `parallel` (rayon) | No published, tracked benchmark numbers; no CI regression gate; no criterion baselines |
| **Security / supply-chain** | 9.5 | cargo-vet, cargo-deny, CodeQL, OSSF scorecard action, REUSE, `unsafe` forbidden except `simd` | No published SBOM artifact per release; no OpenSSF Best Practices badge; `simd` `unsafe` needs a documented safety-invariant audit |
| **Documentation** | 9 | rustdoc-strict, `USER-GUIDE.md`, ADRs, 77 examples, README | No docs.rs feature-matrix build proof; no task-oriented cookbook; no per-satellite docs site |
| **no_std / portability** | 8 | `no_std` + alloc; wasm32 bare-metal builds | #210: does not build for `*-none` bare-metal; no `thumbv*`/embedded target in CI |
| **DX / ergonomics** | 8.5 | miette/ariadne diagnostics, typed path API, recovery | No derive helpers/builders for common flows; error taxonomy could surface fix-hints uniformly |
| **Interop / ecosystem** | 9 | serde, `serde_yaml` compat shim, figment, schemars, garde/validator, tokio, sval | `serde_yaml` is unmaintained upstream — noyalib should *claim the successor position* explicitly, with a migration guide + shim parity table |
| **Satellites** | 7.5 | wasm/mcp/lsp/cli all shipping | Each is v0.0.x with obvious next features (below); no shared docs, no published npm/registry packaging for wasm/mcp |
| **Release / governance** | 9.5 | ADR-0005 strict lockstep, signed commits, Keep-a-Changelog | MSRV/deprecation policy not written down as a doc; release is partly manual |

**Weighted read:** core ≈ **9.0/10**, satellites ≈ **7.5/10**. The two
project-tracked issues (#221, #210) plus the satellite build-out are ~80% of
the distance to a clean 10 everywhere.

---

## 2. Gaps & issues (grounded)

### 2.1 Project-tracked
- **#221 — CST edit API remaining gaps.** `rename_key` (gap 2) just landed via
  #222. Still open: **(1)** comment mutation (`set_comment`/`insert_comment`/
  `remove_comment`), **(3)** sequence reorder (`swap_items`/`move_item`),
  **(4)** extended `remove` (multi-line/nested/sole-entry/flow), **(5)**
  quoting-aware fragment emit (`set`/`insert`/`push_back` synthesise indent
  but not quoting — a fragment with `:` or leading `-` can restructure a doc
  the re-parse guard can't catch because the result is *valid* YAML).
  Read-only key spans for duplicate-key diagnostics is a bonus ask.
- **#210 — `no_std` does not build for bare-metal `*-none` targets.** wasm32
  builds; true embedded (`thumbv7em-none-eabihf`, etc.) does not.

### 2.2 Analysis-surfaced (not yet tracked)
- **Coverage floor at 95%.** For a data-format library, the uncovered 5% is
  exactly where edge-case bugs live (error branches, recovery paths, SIMD
  fallbacks). A 100% region floor with justified `#[coverage(off)]` on
  genuinely-unreachable arms is achievable and higher-signal.
- **Fuzzing is a 10 s PR smoke.** No corpus accretion, no continuous/OSS-Fuzz
  integration, no structured-input fuzzers for the *editors* (only the
  parser is differentially fuzzed vs saphyr).
- **No performance regression gate.** Benches exist but nothing fails CI on a
  regression, and there are no published numbers to anchor claims like
  "SIMD" / "fast-float".
- **SIMD `unsafe` audit.** `simd`/`nightly-simd` opt out of the
  `unsafe_code = forbid` invariant. There's no published soundness note /
  Miri-under-SIMD coverage for those paths.
- **`serde_yaml` succession.** `serde_yaml` is archived upstream; noyalib has
  a `compat-serde-yaml` shim but doesn't *market or document* itself as the
  drop-in successor — a large, free adoption lever.
- **Satellite feature depth** (see §4).
- **No published SBOM per release**, no OpenSSF Best Practices badge, no
  MSRV/deprecation policy doc.

---

## 3. Implementation plan — core crate

Format: **Epic → tasks → acceptance criteria → effort (S≤1d / M≤1wk / L>1wk)
→ risk → categories moved.** Sequenced into milestones in §6.

### EPIC A — Complete the CST surgical-edit API (closes #221)
The single biggest functionality lever; the design pattern (resolve span →
splice → re-parse-and-oracle guard → rollback) is already proven by
`rename_key`.

- **A1. Comment mutation** — `set_comment(path, position, text)`,
  `insert_comment(path, position, text)`, `remove_comment(path, position)`
  where `position ∈ {Leading, Inline, Trailing}`, built on `comments_at`.
  - *Accept:* byte-identity everywhere except the comment span; `#`-prefix and
    surrounding whitespace synthesised; guard + rollback; ≥20 tests incl.
    multi-line leading blocks, inline-after-value, blank-line interactions.
  - M · low risk · **API, DX**
- **A2. Sequence reorder** — `swap_items(seq, i, j)`, `move_item(seq, from, to)`.
  - *Accept:* single computed splice plan (no offset-invalidation bugs);
    preserves each item's comments/anchors/formatting; refuses flow seqs in
    v1 (mirror `remove`); ≥18 tests.
  - M · medium risk (multi-span offset math) · **API**
- **A3. Extended `remove`** — multi-line / nested / sole-entry / block-flow.
  Port the reference `replace_span` fallback from the #221 reporter's `yqr`,
  keeping the parse-differently guard.
  - *Accept:* removing a nested block map/seq leaves siblings byte-identical;
    sole-entry removal yields a valid empty collection; ≥25 tests + a fuzz
    target that removes a random path and asserts `value == original − path`.
  - L · medium risk · **API, correctness**
- **A4. Quoting-aware `Emit`** — the deferred auto-formatting follow-up:
  fragment inserters quote/escape scalars so a value containing `:` / leading
  `-` / `#` cannot restructure the document.
  - *Accept:* a property test — for arbitrary scalar `s`, `set(path, s)` then
    read-back returns exactly `s`; the "valid-but-misinterpreted" class is
    closed; semver-additive (new `EmitOptions`, existing behaviour default).
  - L · higher risk (touches every mutator) · **API, correctness**
- **A5. Read-only key spans** — `Document::key_span(path)` / duplicate-key
  reporting with positions.
  - *Accept:* powers diagnostics without walking the green tree by hand; used
    by lsp (§4.3). S · low · **API, DX**

### EPIC B — Testing to a defensible 10
- **B1. Raise coverage floor 95% → 100% regions**, with justified
  `#[coverage(off)]` on unreachable arms only. Accept: gate at 100%; every
  suppression carries a one-line why. M · low · **testing**
- **B2. Editor fuzzing** — structured `arbitrary`-driven fuzz targets for each
  mutator (set/insert/remove/rename/reorder/comment): apply a random edit,
  assert the guard's invariant (re-parse equals the typed oracle) or a clean
  refusal. Accept: 6 new fuzz targets; corpus committed; runs nightly, not
  just 10 s. M · low · **testing, correctness**
- **B3. Continuous fuzzing** — submit to OSS-Fuzz (or a nightly long-run job)
  with corpus persistence. Accept: OSS-Fuzz project merged, or a nightly
  ≥30 min job green. M · low · **testing, security**
- **B4. Property-test parity** — ensure every public codec path
  (parse↔serialise↔CST-round-trip) has a `proptest`/`arbitrary` round-trip.
  Accept: a checklist test that fails if a public entry point lacks one. M ·
  low · **testing**

### EPIC C — Performance, measured and defended
- **C1. Published benchmark suite** — criterion baselines checked in; a
  `BENCHMARKS.md` with numbers vs `serde_yaml`, `saphyr`, `yaml-rust2` on a
  fixed corpus and machine spec. S–M · low · **performance, docs**
- **C2. CI regression gate** — `criterion` + `critcmp` (or `cargo-codspeed`)
  failing the build on >X% regression on a canonical set. M · medium
  (runner noise) · **performance**
- **C3. SIMD soundness note** — document the `unsafe` invariants; run Miri /
  sanitizers over the scalar-fallback equivalence tests; add a
  `simd == scalar` differential property. M · medium · **performance, security**

### EPIC D — no_std / portability (closes #210)
- **D1. Bare-metal `*-none` build** — audit for hidden `std`/`alloc`-assuming
  paths; gate `thumbv7em-none-eabihf` (+ `riscv32imac-unknown-none-elf`) in
  CI with a `#![no_std]`/`#![no_main]` smoke crate. M · medium · **no_std**
- **D2. `alloc`-free surface audit** — document exactly which APIs need
  `alloc` vs pure `core`; consider a `core`-only reader for the
  streaming/event API. M · medium · **no_std, docs**

### EPIC E — Security / supply-chain to 10
- **E1. Per-release SBOM** — emit CycloneDX (`cargo-cyclonedx`) as a release
  asset; link it from the README. S · low · **security**
- **E2. OpenSSF Best Practices badge** — complete the passing (→ silver)
  questionnaire; add the badge. S · low · **security, governance**
- **E3. Provenance** — cargo publish with build provenance / attestations;
  document the signing story (SSH-signed tags + verified releases). S–M ·
  low · **security, governance**

### EPIC F — Documentation & interop leadership
- **F1. `serde_yaml` successor positioning** — a `MIGRATING-FROM-SERDE-YAML.md`
  with a shim parity table (what matches, what intentionally differs, how to
  flip), a one-line `Cargo.toml` swap, and a compile-tested example. High
  adoption ROI. M · low · **interop, docs**
- **F2. Task-oriented cookbook** — `doc/COOKBOOK.md`: "edit a value keeping
  comments", "merge two docs", "validate against a JSON Schema", "stream a
  huge file", each a runnable, doctested snippet. M · low · **docs, DX**
- **F3. docs.rs feature-matrix proof** — CI job building docs with each major
  feature set so the rustdoc never silently breaks under a feature combo. S ·
  low · **docs**

---

## 4. Implementation plan — satellites

### 4.1 `noya-cli` (noyafmt, noyavalidate) → 10
- **CLI-1.** `noyaedit` subcommand exposing the CST mutators (set/get/remove/
  rename/reorder) as a jq-style path CLI — the natural home for Epic A.
- **CLI-2.** Shell completions (bash/zsh/fish/pwsh) via `clap_complete`; man
  pages via `clap_mangen`; ship in releases.
- **CLI-3.** Prebuilt binaries per-platform (`cargo-dist`) + `cargo-binstall`
  metadata; Homebrew tap; a `pre-commit` hook entry (doc/pre-commit.md exists
  — wire an official `pre-commit-hooks.yaml`).
- **CLI-4.** `--format json|sarif` for `noyavalidate` so it drops into CI
  annotations and code scanning.
- *Effort:* M each · **satellites, DX, ecosystem**

### 4.2 `noyalib-mcp` → 10
- **MCP-1.** Expand the tool set to cover the *complete* CST edit API as it
  lands (comment mutation, reorder, extended remove) — keep every tool
  read-only-by-contract with the guard.
- **MCP-2.** Resources + prompts (not just tools): expose the parsed doc as an
  MCP resource; ship prompt templates for common edit intents.
- **MCP-3.** Publish to the MCP registry + a signed release; add a conformance
  test against the MCP Inspector in CI (pacs008-mcp already does this — reuse
  the pattern).
- *Effort:* M each · **satellites, ecosystem**

### 4.3 `noyalib-lsp` → 10
- **LSP-1.** Consume Epic A5 (key spans) for duplicate-key diagnostics with
  ranges; add code actions ("rename key", "sort keys", "remove entry") backed
  by the CST mutators — surgical, comment-preserving edits from the editor.
- **LSP-2.** Semantic tokens, document symbols/outline, folding ranges, hover
  with schema docs when a `validate-schema` schema is attached.
- **LSP-3.** A VS Code extension (thin client) published to the Marketplace +
  OpenVSX; document Neovim/Helix/Emacs setup.
- *Effort:* M–L · **satellites, DX**

### 4.4 `noyalib-wasm` → 10
- **WASM-1.** Ship a proper **npm package** with generated **TypeScript
  types**, ESM + CJS, and a size-tracked bundle (`wasm-opt` on in release).
- **WASM-2.** Expose the full lossless-edit API (not just parse/serialise) so
  browser tools get the same guarantees; a live playground page.
- **WASM-3.** `wasm32-wasi` target + a Deno/Bun smoke test in CI.
- *Effort:* M · **satellites, ecosystem**

---

## 5. New feature proposals (beyond gaps)

Ranked by value-to-effort. Each is semver-additive and optional-feature-gated.

1. **JSONPath / JMESPath-style query** over `Value` and `Document` (read side
   of the jq-story that `yqr` is building on top) — a `query` feature.
2. **YAML→JSON / JSON→YAML** lossless-where-possible converters as first-class
   APIs (+ CLI), with anchor/merge handling documented.
3. **Schema *inference*** — generate a JSON Schema or Rust `struct` skeleton
   from a sample document (pairs with the existing `schema` feature).
4. **Merge-key (`<<`) materialisation & round-trip policy** as a documented,
   testable surface (it already trips several edge cases in #221).
5. **`Document::diff` / structural patch** — produce a minimal comment-
   preserving patch between two documents (powers the CLI/LSP/MCP "apply
   change" flows and 3-way merges).
6. **Deterministic canonical form** (`to_canonical`) for hashing/signing YAML
   payloads — useful to the finance/config audiences the sibling projects
   target.
7. **Editor-grade error recovery surface** — expose the `recovery` module's
   partial parse as a public "best-effort" API for tooling.

---

## 6. Sequencing & milestones

Milestones are releasable under the lockstep contract (core + satellites move
together). Rough order, dependency-aware:

- **M1 — "Finish the editors" (v0.0.18–0.0.19).** Epic A (A1 comment, A2
  reorder, A5 key spans), B1 coverage-100, F3 docs-matrix. Unblocks LSP/MCP/
  CLI edit features. *Closes most of #221.*
- **M2 — "Prove it fast & safe" (v0.0.20).** A3 extended remove, B2 editor
  fuzzing, C1 published benches, E1 SBOM, E2 OpenSSF badge.
- **M3 — "Reach everywhere" (v0.0.21).** A4 quoting-aware Emit (the hard one),
  D1 bare-metal (*closes #210*), C2 perf gate, C3 SIMD soundness.
- **M4 — "Own the niche" (v0.0.22 → 0.1.0).** F1 serde_yaml succession, F2
  cookbook, the satellite build-outs (CLI-1..4, MCP-1..3, LSP-1..3,
  WASM-1..3), and the §5 features that survive triage. Cut **0.1.0** when the
  edit API is complete and the successor story is documented.

### Category → milestone it reaches 10

| Category | Reaches 10 at |
|---|---|
| API / functionality | M3 (Emit) / M4 (features) |
| Correctness / testing | M2 (fuzz + 100% cov) |
| Performance | M3 (gate + numbers) |
| Security / supply-chain | M2 (SBOM + badge + SIMD note) |
| Documentation | M4 (cookbook + migration) |
| no_std / portability | M3 (#210) |
| DX / ergonomics | M3–M4 (Emit + CLI/LSP actions) |
| Interop / ecosystem | M4 (serde_yaml succession) |
| Satellites | M4 |
| Release / governance | M2 (policy docs + SBOM) |

---

## 7. Effort summary

- **Core:** ~6 epics, ~20 tasks. The three L-items (A3, A4, plus D1) are the
  critical path; everything else is S/M and parallelisable.
- **Satellites:** ~13 tasks, mostly M, gated on Epic A landing in core.
- **Risk hotspots:** A4 (quoting-aware Emit — touches every mutator; mitigate
  with a property test and an opt-in `EmitOptions` default-off flip), C2
  (bench-gate flakiness on shared runners — use codspeed or a noise budget),
  D1 (hidden `alloc` assumptions).

## 8. What I'd *not* do (scope discipline)

- Don't chase 100% coverage by testing genuinely-unreachable arms — mark them
  `#[coverage(off)]` with a reason instead.
- Don't add a plugin/DSL layer; the value is a rock-solid lossless core +
  thin, honest satellites.
- Don't break the `unsafe`-forbid invariant beyond the audited `simd` seam.
- Keep the lockstep contract — no feature ships in a satellite ahead of the
  core capability it needs.
