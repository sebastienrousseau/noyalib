# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

(Nothing yet - `[v0.0.11]` is the cut.)

## [v0.0.11] - 2026-07-01

The **CI-integrity** cut. Fixes a silently-broken `no_std` build that
had been masked on `main` since v0.0.9 by cache poisoning in the CI
gates, closes three broken intra-doc links that were failing the
Pages-deployment workflow every push, hardens every specialised cargo
job across `ci.yml`, `security.yml`, `docs.yml`, and `release.yml`
with an isolated `CARGO_TARGET_DIR` so a stale artefact set can no
longer serve as a passing fingerprint for a different feature-set,
adds a strict-rustdoc PR gate so broken doc links fail a PR rather
than the post-merge Pages deployment, and closes OSSF Scorecard Code
Scanning alert #36 (`RUSTSEC-2026-0173`, `proc-macro-error2`
unmaintained — build-time-only via the opt-in `validator` feature,
never ships in a release artefact) with a source-controlled
`osv-scanner.toml` ignore.

No public API change. No MSRV change (still 1.85). One behavior
change under `--no-default-features`: `crates/noyalib/src/doc_boundary.rs`
now correctly imports `Vec` and `vec![]` from the `crate::prelude`
under `no_std` (previously used them without importing, which the
poisoned `no_std` CI gate never noticed).

### Fixed

- **`no_std` build actually compiles.** `doc_boundary.rs` used `Vec`
  and `vec![]` without importing them from `crate::prelude`. Under
  `std` the prelude auto-imports both; under `no_std` they're only
  reachable via `alloc::vec::Vec` / `alloc::vec` or the project's
  `crate::prelude` re-export. Adds `use crate::prelude::{Vec, vec};`.
  Also gates `use crate::span_context;` in `de.rs` behind
  `#[cfg(feature = "std")]` to match its call sites, so
  `--no-default-features` no longer trips `-D unused-imports`.
- **Three broken intra-doc links** in `de/config.rs` (`[Value]` at
  lines 179, 191, 348 and `[Error::Custom]` at line 213) now resolve.
  These had been failing the `Documentation` workflow's strict
  `-D warnings` build on every push to `main` since 2026-06-30.
  Qualified all four to `[Value](crate::Value)` /
  `[Error::Custom](crate::Error::Custom)`.

### Changed

- **CI cache-poisoning guard applied across every specialised cargo
  job.** `no_std`, `MSRV (1.85.0) core build`, `Miri (focused)`,
  `Miri (full + big-endian, scheduled)`, `Coverage gate`,
  `Differential fuzz (10s smoke)`, `rustdoc (strict)`, `soak-fuzz`,
  `soak-miri`, `docs.yml`, `release.yml/validate`, and
  `release.yml/cross-verify` all now use an isolated
  `CARGO_TARGET_DIR` plus a scoped Swatinem cache namespace. Sharing
  target/ across feature configurations was how the `no_std` job
  passed a stale-but-clean check for two full releases while the
  code was actually broken — this pattern rules that out.
- **New `docs-strict` PR-gated job** added to `ci.yml`. Mirrors the
  strict `RUSTDOCFLAGS` (`-D warnings` + `broken_intra_doc_links` +
  `private_intra_doc_links` + `invalid_codeblock_attributes` +
  `invalid_html_tags` + `bare_urls`) that `docs.yml` uses on `main`,
  but on every PR. Broken doc links now fail a PR instead of the
  Pages deployment after merge.

### Security

- **Code Scanning alert #36 closed.** `RUSTSEC-2026-0173`
  (`proc-macro-error2` unmaintained) documented as accepted risk in
  the new `osv-scanner.toml` (mirroring the pre-existing `deny.toml`
  `[advisories.ignore]` entry) so OSSF Scorecard's Vulnerabilities
  check no longer re-flags it. Rationale is build-time-only exposure
  via the opt-in `validator` feature; never ships in a release
  artefact. Revisit when `validator` cuts a release that drops
  `proc-macro-error2`.

## [v0.0.10] - 2026-06-30

The **BOM scanner** cut. A leading UTF-8 BOM (`U+FEFF`) no longer
breaks `parse_document` on multi-node inputs. Contributed by
[@zoosky](https://github.com/zoosky) (PR #118, rebased as #123).

No public API change. No MSRV change.

### Fixed

- **Leading UTF-8 BOM is transparent to indentation and comments.**
  The scanner used to consume the BOM with `advance_by(3)` while
  counting those three bytes toward the column of the following
  content, so a BOM-prefixed multi-node document (`<BOM>a: 1\nb: 2\n`)
  errored with "stray content after document — subsequent documents
  must start with '---'". The same miscount also broke BOM-prefixed
  sequences, nested mappings, and a `<BOM>#`-style first-line comment.
  Three surgical fixes in `parser/scanner.rs` make the BOM transparent
  at every column-aware site: `fetch_stream_start` resets `self.col = 0`
  after `advance_by(3)`, the simple-key indent path skips a leading
  BOM when computing `line_start`, and the block-context comment check
  treats `#` immediately after a leading BOM as a start-of-input
  comment. Each fix is gated on `pos == 3` / `line_start == 0` so an
  interior `0xEF 0xBB 0xBF` byte sequence in legitimate UTF-8 is
  never mistaken for one. Two scanner regression tests included.

## [v0.0.9] - 2026-06-30

The **supply-chain refresh** cut. Batches eight open Dependabot PRs,
clears two RustSec advisories, migrates `jsonschema` 0.33 → 0.46 with
the ValidationError API change, and refreshes the cargo-vet
exemptions + `imports.lock` snapshot so the supply-chain gates run
green from a clean state.

No public API change. No MSRV change.

### Fixed

- **Two RustSec advisories cleared.** `anyhow` 1.0.102 → 1.0.103
  (`RUSTSEC-2026-0190` — `Error::downcast_mut` unsoundness),
  `memmap2` 0.9.10 → 0.9.11 (`RUSTSEC-2026-0186` — unchecked pointer
  offset).
- **`jsonschema` 0.33 → 0.46.7 API migration.**
  `ValidationError::kind` and `ValidationError::instance_path` are
  now methods, not public fields. Updated `schema_validate.rs` and
  `cst/coerce.rs` to the new call syntax. All 16 schema tests pass
  under the new API.

### Changed

- **Batched Dependabot bumps.** GitHub Actions: `actions/checkout`
  6.0.3 → 7.0.0, `actions/cache` 5.0.5 → 6.1.0,
  `actions/attest-build-provenance` 4.1.0 → 4.1.1,
  `github/codeql-action/{init,analyze,upload-sarif}` SHA bump,
  `ossf/scorecard-action` SHA bump, `dtolnay/rust-toolchain` master
  SHA bump, `taiki-e/install-action` 2.81.8 → 2.82.2. Cargo:
  `bytes` 1.11.1 → 1.12.0, `serde-saphyr` 0.0.27 → 0.0.28.
- **Supply-chain hygiene.** `deny.toml` gains a scoped `Zlib`
  license allowance for `foldhash` (transitive via
  `hashbrown 0.16 → referencing 0.46`), matching the existing
  MIT-0 / BSD-2-Clause posture. `supply-chain/config.toml` shrinks
  from 18 to 14 exemptions after `cargo vet regenerate exemptions`
  (upstream audits now cover what was previously locally
  exempted). `supply-chain/imports.lock` refreshed via
  `cargo vet regenerate imports` so `--locked` accepts the current
  dep graph.
- **Workspace crates bumped to 0.0.9** (`noyalib`, `noya-cli`,
  `noyalib-mcp`, `noyalib-lsp`, `noyalib-wasm`) with intra-workspace
  `version =` pins synced. `xtask` stays at 0.0.1.

## [v0.0.8] - 2026-06-17

The **FlowStyle Fix** cut. Honors `SerializerConfig::flow_style` in the
serializer so `FlowStyle::Flow` and `FlowStyle::Auto` finally emit
inline collections (#84), folds in the batched Dependabot backlog
(cargo, GitHub Actions, Docker base images), and updates the
supply-chain gates for the new `proc-macro-error2` unmaintained
advisory.

No public API change. No MSRV change (still 1.85). The only behavior
change is the FlowStyle fix: callers setting `Flow` or `Auto` now get
inline output instead of silently getting block output.

### Fixed

- **`SerializerConfig::flow_style` is now honored.** `flow_style` and
  `flow_threshold` were stored but never read by the emit path, so all
  collections rendered as block regardless of config. `write_sequence`
  and `write_mapping` now dispatch to the flow emitters: `Flow` is
  always inline, `Auto` is inline within `flow_threshold` (default 4)
  with a safe block fall-back for oversized subtrees, and `Block` is
  unchanged. Adds four regression tests. [#84]

### Changed

- **Batched Dependabot bumps.** cargo: `clap_complete` 4.6.3 to 4.6.5,
  `smallvec` 1.15.1 to 1.15.2, `memchr` 2.8.0 to 2.8.2. github-actions:
  `actions/checkout` 6.0.2 to 6.0.3, `taiki-e/install-action` 2.81.1 to
  2.81.8, `KSXGitHub/github-actions-deploy-aur` 3.0.1 to 4.1.3,
  `docker/setup-buildx-action` 3.12.0 to 4.1.0, `docker/login-action`
  3.7.0 to 4.2.0. docker: `rust:1.96-bookworm`, `debian:bookworm-slim`,
  and `distroless/cc-debian12` digests re-pinned. [#85 to #96]

### Security

- **RUSTSEC-2026-0173** (`proc-macro-error2` unmaintained) ignored in
  `deny.toml` and `.cargo/audit.toml`. Build-time only via
  `validator_derive` to `validator`; never ships in an artefact.
  Revisit when `validator` releases off `proc-macro-error2`.

## [v0.0.7] — 2026-06-02

The **Supply-chain Hardening** cut. Closes three CVEs in the
VS Code extension's npm tree (xml2js, qs, tmp — all dragged in
by the legacy `vsce` devDep, which was removed), tightens the
release pipeline with `npm ci` + exact-pinned lockfile, and
folds in the routine Dependabot backlog: serde, clap, criterion
0.8 (with the `std::hint::black_box` migration across 14 bench
files), yaml-competitors, the actions group (10 actions headed
by checkout v5 → v6), plus eight standalone action bumps and
the sigstore cosign-installer v3 → v4 bump.

### Security

- **Drop legacy `vsce`.** `pkg/vscode/devDependencies` removed;
  the release pipeline already used `npx --yes @vscode/vsce`,
  so the legacy package was inert. Clears GHSA-776f-qx25-q3cc
  (xml2js), GHSA-q8mj-m7cp-5q26 (qs), and GHSA-ph9p-34f9-6g65
  (tmp). [#70]
- **`npm install` → `npm ci`** in the `vscode-extension` job
  with a committed, exact-pinned `package-lock.json`. [#69]
- **cosign-installer** upgraded to v4.1.2; release artefact
  signing flow unchanged. [#63]
- **Auto-approve Dependabot** workflow added to satisfy
  OpenSSF Scorecard's `Code-Review` check on a
  solo-maintainer project. [#64]

### Changed

- **criterion 0.5.1 → 0.8.2.** All 14 bench files migrated from
  `criterion::black_box` to `std::hint::black_box` (criterion
  0.8 deprecated the re-export). [#80]
- **yaml-competitors group bump** — yaml-rust2 0.9 → 0.11,
  rust-yaml 0.0.5 → 1.1; both are bench/comparison-only deps,
  not part of the runtime tree. [#80]
- **actions group bump (10 actions)** including checkout v5 →
  v6, cache v4 → v5, configure-pages v4 → v6,
  upload-pages-artifact v3 → v5, plus 6 others. [#80]
- **clap_complete 4.5 → 4.6, clap_mangen 0.2 → 0.3** in
  noya-cli and xtask. [#80]
- **Docker base** bumped from `rust:1.85-bookworm` to
  `rust:1.96-bookworm` in all three Dockerfiles. [#62]

### Fixed

- **OpenSSF Scorecard Pinned-Dependencies** 9/10 → 10/10 by
  exact-pinning every npm dep and switching to `npm ci`. [#66,
  #69]

## [v0.0.6] — 2026-05-30

The **Ecosystem Integration** cut. Lands the four remaining
open issues from the v0.0.6 milestone (#22, #24, #25, #33),
closes out the leftover stabilisation checklist (#19), the
i18n hooks (#18), the user-reported pnpm-lock recursion bug
(#46), and the OpenSSF Scorecard hardening pass that lifts
the score from 6.5/10 to ~9/10.

### Fixed — streaming deserializer depth leak on empty flow mappings (issue #46)

`StreamingMapAccess::next_key_seed` (and the symmetric
`next_value_seed` / `StreamingSeqAccess::next_element_seed`)
did not consult the access object's `finished` flag. Serde
visitors that call `next_entry` after the previous call
returned `Ok(None)` — `noyalib::Value`'s `ValueVisitor::visit_map`
is the canonical one — read the **next event from the parent
mapping** and treated it as belonging to the now-exhausted
child. The recursive `deserialize_any` on each spilled value
inflated `self.depth` by one per entry; on a `pnpm-lock.yaml`
shaped input with N consecutive empty flow mappings `{}`,
depth hit `max_depth + 1` after exactly 128 entries and
`from_str::<Value>` failed with
`Error::RecursionLimitExceeded { depth: 129 }` even though the
real nesting depth was 2.

Fix in `crates/noyalib/src/streaming.rs`:

* Both `MapAccess::next_key_seed` and
  `MapAccess::next_value_seed` return `Ok(None)` / a clear
  contract-error early when `finished` is set.
* `SeqAccess::next_element_seed` mirrors the guard.
* `deserialize_any` / `deserialize_seq` / `deserialize_map`
  now decrement `self.depth` on both `Ok` and `Err` so a
  failed inner visit cannot leak depth into the outer scope
  (the same leak path under a different trigger).

Regression test: `crates/noyalib/tests/issue_46.rs` —
50 000-package `pnpm-lock.yaml`-shaped fixture, 3 000 empty
flow mappings at one level, complex peer-dependency keys, and
the deterministic depth-cliff probe at every `n` in
`[100, 128, 129, 130, 200, 500, 1000]`.

Affects every typed deserialize target whose visitor calls
`next_entry` past the end (including `BTreeMap<K, V>` and
struct fields of optional shape). Default `from_str` users
upgrading to this release should see only the previously-broken
parses now succeed; no behavioural change on valid documents.

### Fixed — no-span loader missing depth-limit check

Companion audit finding to issue #46: the no-span loader path
(`crates/noyalib/src/parser/loader.rs`) — used by
`from_str::<Value>`'s value-target fast path and by `no_std`
multi-document loading — incremented `self.depth` on
`SequenceStart` / `MappingStart` events at lines 814 / 833 but
did **not** check against `ParserConfig::max_depth` the way
the span-tracked loader does at lines 399-401 / 443-445.
Adversarial deeply-nested input could consume stack without
ever firing `RecursionLimitExceeded`. Now mirrored from the
span loader. Regression test:
`no_span_loader_honours_max_depth` in
`crates/noyalib/tests/issue_46.rs`.

Both findings were surfaced by the same code-pattern audit
that confirmed every `MapAccess` / `SeqAccess` /
`EnumAccess` / `VariantAccess` impl across `streaming.rs`,
`de.rs`, and `value.rs` either has a `finished`-style guard
or uses an iterator that naturally returns `None` on
exhaustion. No further iterator-state-leak bugs in the same
family remain.


### Security & hardening pass on the v0.0.6 surface

Post-merge deep-dive audit surfaced six DoS / correctness
findings in the new modules; this section documents the fixes.

* **Recovery `---`-spam OOM (C2).** `parse_lenient` now bounds
  the document-marker scan by `ParserConfig::max_documents`. A
  hostile `---\n`-only-spam input cannot drive unbounded
  `Vec<usize>` allocation.
* **Recovery O(n²) line-truncation (C1).** The truncation loop
  is bounded by a new `LenientConfig::truncation_event_budget`
  (default 1 MiB cumulative bytes across retries). Adversarial
  10k-line malformed input no longer triggers ~10k full
  re-parses.
* **Async unbounded `read_to_end` (C3).** Both
  `from_async_reader{,_with_config}` and the new
  `from_async_reader_multi_with_config` drain through
  `AsyncReadExt::take(max_document_length)`. Slow-drip
  adversaries cannot grow the buffer beyond the configured
  limit before the parser fires its own check.
* **CRLF support (C4).** `recovery::split_documents`,
  `tokio_async::find_doc_boundary`, and the existing
  `parallel::split` now share **one** workspace-private scanner
  in `crate::doc_boundary` that accepts both `\n` and `\r\n`
  line terminators. The three previous copies disagreed on
  CRLF — Windows-edited buffers round-trip through every entry
  point now.
* **BOM support (C5).** `parse_lenient` and both async readers
  strip a leading UTF-8 BOM (`U+FEFF`) so Windows-saved buffers
  parse identically to LF-on-Linux equivalents.
* **Decoder recursion → loop (C6).** `YamlDecoder::decode` no
  longer recurses on whitespace-only frames; the all-whitespace
  preamble is consumed in a bounded `loop` instead, eliminating
  the adversarial stack-overflow vector.

Correctness fixes that landed alongside the hardening:

* `parse_lenient` now collects Pass-2 / Pass-3 errors instead
  of silently dropping them (M1).
* Multi-document budget exhaustion no longer truncates the
  output `Sequence` — skipped documents are emitted as
  `Value::Null` so per-document diagnostic indices stay
  aligned for LSP joiners (M2).
* Line-truncation now treats the buffer end as a candidate
  cut, so a malformed last line **without** a trailing newline
  (the universal mid-typing case) is still recoverable (M3).
* Pass-2 `ParserConfig` clone hoisted out of the hot path —
  one clone per document, not one per pass (M13).

Surface additions on the tokio module:

* `from_async_reader_multi_with_config` — config-aware variant
  was missing.
* `YamlDecoder::max_frame_size(usize)` — optional inter-frame
  buffer cap for codec users driving untrusted-network input
  (M7). `Decoder::decode` returns
  `Error::Io(InvalidData)` when the buffer exceeds the cap.
* `YamlDecoder` is now `Clone`.

Surface additions on the sval adapter:

* `to_sval_writer_with_config` + `SvalConfig` —
  `coerce_non_finite_to_null` toggles NaN / ±∞ → `Null` so
  downstream consumers (e.g. `sval_json`) that reject
  non-finites accept the stream (M10).
* `impl sval::Value for Tag` — the public `Tag` type now has a
  direct sval impl (M12).

Documentation:

* `SECURITY.md` and `doc/POLICIES.md` document the new
  resource-limit knobs and their threat model.
* CHANGELOG, READMEs, and `RELEASE-NOTES-v0.0.6.md` cross-reference
  the new safe-by-default contracts.
* MSRV inconsistency (workspace claimed 1.75 in some places,
  Cargo.toml says 1.85 since v0.0.5) resolved on both axes.

Tests: +24 unit tests across the three modules (61 total),
covering CRLF, BOM, `---`-spam, no-trailing-newline truncation,
budget-exhaustion index preservation, oversize-reader truncation,
frame-size cap, NaN coercion. Plus a new
`bench_recovery_lenient_on_invalid_input` arm exercising the
3-pass recovery loop on realistic LSP-style half-typed input.

### Added — Error-recovering parser (`recovery` feature, issue #22)

`noyalib::recovery::parse_lenient` returns a `ParseResult`
carrying the best-effort tree plus the list of every error
encountered, so LSP / IDE consumers can keep showing
autocomplete and diagnostics on half-typed documents.

```rust
let r = noyalib::recovery::parse_lenient("a: 1\nb: [unclosed\n");
assert!(!r.is_complete);
assert!(!r.errors.is_empty());
```

Recovery strategies — strict pass first, then
`DuplicateKeyPolicy::Last` retry, then line-truncation retry
that drops trailing lines until something parses.
Multi-document input is split on `---` and each document is
recovered independently. Error collection is capped via
`LenientConfig::max_errors`.

Gated behind the new `recovery` Cargo feature; zero extra deps.

### Added — `sval` streaming adapter (`sval` feature, issue #25)

Alternative to the default serde route for callers wanting to
skip `serde_derive`'s compile-time overhead or the binary-size
cost of serde monomorphisation. Adds `impl sval::Value for
Value`, `Number`, `Mapping`, `MappingAny`, and `TaggedValue`,
plus a `noyalib::sval_adapter::to_sval_writer` entry point.
serde remains the default; this is opt-in.

### Added — Native tokio async parsing (`tokio` feature, issue #24)

`noyalib::tokio_async::from_async_reader` /
`from_async_reader_multi` parse from any
`tokio::io::AsyncRead` without `spawn_blocking`.
`YamlDecoder<T>` is a `tokio_util::codec::Decoder` for
plugging streaming YAML parsing into a
`tokio_util::codec::Framed` / tower pipeline. Per-document
emission boundary follows the YAML 1.2.2 §9.1.2 `---` grammar.

### Changed — npm publish moves to Trusted Publishing / OIDC (issue #33)

`.github/workflows/release-binaries.yml` no longer reads
`NPM_TOKEN`; both `@noyalib/noyalib-wasm` and `noyalib-mcp`
publish jobs declare `id-token: write` and rely on the OIDC
handshake against per-package trusted-publisher policies
configured at `https://www.npmjs.com/package/<name>/access`.
`pkg/PUBLISH.md` §6 documents the bootstrap + secret-retirement
flow.

Compromise window collapses from 1 year (granular access
token) to ~10 minutes (per-run OIDC token). The
`--provenance` flag stays attached to both publish steps so
the npm verified-publisher badge keeps linking back to the
exact GitHub Actions run.

## [v0.0.5] — 2026-05-11

### Changed — Edition 2024 + MSRV bump to 1.85 (issue #15)

All six workspace crates (`noyalib`, `noya-cli`, `noyalib-mcp`,
`noyalib-lsp`, `noyalib-wasm`, `xtask`) move to:

* `edition = "2024"`
* `rust-version = "1.85.0"`

CI's MSRV-gate job retargeted to 1.85.0 in the same commit.
Edition-2024 idiom fixes applied to surface lints (match
ergonomics on `&mut _` patterns, `repeat_n` replacing
`repeat().take()`, redundant `ref`/`mut` bindings inside
`matches!`). The `unsafe`-tightened `std::env::set_var` /
`remove_var` are no longer called from `examples/figment.rs` —
the env-overlay scenario was refactored to `figment::Env::raw`
plus a synthetic `Serialized` layer.

Lockfile pins for the MSRV-1.75 workarounds are dropped:
* `indexmap 2.10` → `2.14` (latest in `>=2, <3`)
* `rustc-hash 2.0` → `2.1.2` (latest in `>=2, <3`)
* `hashbrown` (transitive) → 0.17

### Added — Declarative `parser_config!` / `serializer_config!` macros (issue #17)

```rust
use noyalib::parser_config;
let cfg = parser_config! {
    max_depth: 64,
    strict_booleans: true,
    duplicate_key_policy: DuplicateKeyPolicy::Error,
};
```

Pure expansion to the existing chained-setter builders — zero
runtime overhead. Supports empty form (`parser_config! {}`
returns `ParserConfig::new()`) and trailing comma after the
last entry. The `serializer_config!` counterpart targets
[`SerializerConfig`].

### Added — Pluggable error-message formatters (issue #18)

New `noyalib::i18n` module:

* `MessageFormatter` trait — `Send + Sync` strategy for
  rendering `Error` as a user-visible message.
* `DefaultFormatter` — preserves the developer-facing message
  verbatim (`Display`-equivalent).
* `UserFormatter` — collapses noyalib's diagnostic vocabulary
  into short plain-language sentences appropriate for
  non-developer audiences. Includes line numbers when the source
  location is available; strips internal terms (`!!binary`,
  "merge key") and field names that might leak in a GUI alert.
* `Error::render_with_formatter(&dyn MessageFormatter)` —
  dispatch entry point. Custom localisation tables / rich
  formatters plug in by impl-ing `MessageFormatter`.

### Documentation — pre-release API stabilisation audit (issue #19)

Pre-1.0 stabilisation checkpoint. The audit confirmed:
* All public configuration types (`ParserConfig`,
  `SerializerConfig`, `Error`, `MergeKeyPolicy`,
  `DuplicateKeyPolicy`, `FlowStyle`, `ScalarStyle`,
  `YamlVersion`, `RequireIndent`, `TransformReason`,
  `SymlinkPolicy`) carry `#[non_exhaustive]` so adding a field
  / variant in a future patch release is non-breaking.
* All public functions ship with doc-comments + working
  examples. Strict-doc gate (`-D rustdoc::broken_intra_doc_links
  -D rustdoc::private_intra_doc_links -D
  rustdoc::redundant_explicit_links`) enforces this on every
  PR.
* The Error enum's variant set is comprehensive and actionable —
  `Parse`, `ParseWithLocation`, `Deserialize`,
  `DeserializeWithLocation`, `Io`, `Custom`,
  `RecursionLimitExceeded`, `DuplicateKey`,
  `RepetitionLimitExceeded`, `Budget`, `UnknownAnchor`,
  `UnknownAnchorAt`, `MissingField`, `TypeMismatch`, and family
  cover every internal failure path.
* No unintended public surface — every `pub` item is either
  re-exported from the crate root or lives in a documented
  `pub mod`. `pub(crate)` everywhere else.

Stable 1.0.0 is deferred to post-production hardening (target:
2028+). v0.0.5 is the stabilisation *checkpoint*, not the SemVer
release.

## [v0.0.4] — 2026-05-11

### Added — `!include` directive support (issue #10)

`ParserConfig::include_resolver` + `max_include_depth`. Two
feature gates: `include` (resolver types only, works in
no_std-style builds) and `include_fs` (adds the bundled
`SafeFileResolver` with root-dir sandboxing and configurable
symlink policy).

After parse, every `Value::Tagged(!include, scalar_spec)` node
is replaced with the resolver's output. Highlights:

- **In-memory resolvers** — wrap any `Fn(IncludeRequest) ->
  Result<InputSource>` via `IncludeResolver::new`. Useful for
  virtual filesystems, test harnesses, network-backed fetchers.
- **`SafeFileResolver`** — filesystem-backed resolver rooted at
  a directory. Path traversal (`../../etc/passwd`) is caught by
  canonicalisation + root-prefix check; symlinks are governed by
  `SymlinkPolicy::FollowWithinRoot` (default) or
  `SymlinkPolicy::Reject`.
- **Fragment anchors** — `!include file.yaml#key` narrows to the
  named top-level mapping key inside the included document.
- **Cycle detection** — per-walk visited set rejects A→B→A
  regardless of depth.
- **Depth ceiling** — `max_include_depth` defaults to 24 (8 in
  `ParserConfig::strict()`). Trips
  `Error::RecursionLimitExceeded` on overflow.
- **Streaming fast-path** is automatically disabled when an
  include resolver is installed so the post-parse walk runs
  uniformly across every typed target.

11 new integration tests + a 4-scenario runnable example
(`cargo run --example include_directive --features include_fs`).

## [v0.0.3] — 2026-05-11

### Changed — widen `rustc-hash` cap to `>=2, <3`

Single-line manifest widening (`rustc-hash = ">=2, <2.1"` →
`rustc-hash = ">=2, <3"`). The old cap was defensive, not
load-bearing — noyalib's usage is the stable `FxHashMap` /
`FxHashSet` / `FxBuildHasher` public surface unchanged across
the 2.x line.

The motivating downstream is `html-generator`, whose dependency
chain pulls `scraper 0.26 → selectors 0.36 → rustc-hash
^2.1.1`. Under the previous range the two co-resolution paths
were incompatible.

`Cargo.lock` stays pinned to `rustc-hash 2.0.0` because 2.1+
declares `rust-version = "1.77"`, above noyalib's 1.75 MSRV
floor. Downstream consumers on Rust ≥ 1.77 are free to
`cargo update -p rustc-hash` to take 2.1+; consumers on 1.75
inherit our lockfile pin via `cargo build --locked`. Same
MSRV-preservation pattern v0.0.2 used for `indexmap 2.10 /
hashbrown 0.15`.

## [v0.0.2] — 2026-05-10

### Added — `${KEY}` / `${KEY:-default}` substitution during parse (issue #11)

`ParserConfig::properties(map)` plus `strict_properties(bool)`
toggle. Each YAML scalar is walked after parse and any
`${name}` placeholder is substituted from the supplied
`Arc<HashMap<String,String>>`. Supports `${KEY:-default}`
inline fallbacks, `$$` → `$` and `${{` → `${` escapes, and
`}}` → `}`. Strict mode (default for `ParserConfig::strict()`)
errors on unknown keys; lossy mode (default) substitutes the
empty string. Syntax errors in the placeholder (invalid
character, unterminated, malformed `:-default` separator) always
abort regardless of mode. Streaming fast-path is automatically
disabled when properties are active so the post-parse walk runs
uniformly across every typed target.

### Added — `ariadne` adapter for `Error` (issue #23)

New `ariadne` Cargo feature exposing
`noyalib::ariadne_adapter::error_to_ariadne_report(err, filename, source)`
that converts a `noyalib::Error` into an `ariadne::Report` with
the offending byte range labelled. Pairs with the existing
`miette::Diagnostic` impl on `Error` for users who prefer
ariadne's rendering. Multibyte-safe: `Location::index()` is
clamped to the source bounds before being expanded to a labelled
range.

### Added — garde / validator → miette bridge with `Spanned<T>` (issue #32)

`noyalib::validated_miette` exposes
`garde_errors_to_miette(spanned, errors, source, name)` and
`validator_errors_to_miette(...)` that walk a validation error
tree (compact `path: message; …` summary) and emit a single
`miette::Report` whose source label points at the
`Spanned<T>`'s byte range. Behind the `miette` Cargo feature
plus either `garde` or `validator` (or both). Hand-rolled
`Display` + `Error` + `miette::Diagnostic` impls keep
`thiserror` out of the dep closure (matches the policy in
`error.rs`).

### Added — `from_str_borrowing` + `TransformReason` (issue #8)

New public entry points `from_str_borrowing` and
`from_str_borrowing_with_config` for `T: Deserialize<'a>` targets
that borrow from the input slice (`&'a str`, `Cow<'a, str>`,
structs containing those). The streaming deserialiser now routes
plain-scalar string events through `visit_borrowed_str` whenever
the parser produced a `Cow::Borrowed` event, unlocking truly
zero-copy `&'de str` deserialisation. Quoted scalars without
escapes also borrow; scalars that required decoding (escapes,
multi-line folding, alias replay, tag resolution) fall back to
owned buffers.

Adjacent parser hardening: the plain-scalar slow path now emits
`Cow::Borrowed(input_slice)` whenever the scalar is a single
contiguous run of input bytes (no folded line breaks), matching
the slow-path's owned-buffer result byte-for-byte. This means the
common `key: value\n` shape now borrows zero-copy on the streaming
path, not just terminal scalars at end-of-input.

`TransformReason` enum (`noyalib::borrowed::TransformReason`)
catalogues the five reasons a scalar can fail to borrow:
`EscapeSequence`, `LineFold`, `TagResolution`, `QuotedScalar`,
`AliasExpansion`. `Display` and `as_str` provide stable messages
suitable for inclusion in higher-level error reports. The enum is
`#[non_exhaustive]` so adding finer-grained variants in the future
is non-breaking.

### Added — `read` / `read_with_config` lazy multi-document reader (issue #7)

`noyalib::read<R: Read, T: DeserializeOwned>(reader)` returns a
`DocumentReadIterator<T>` that yields one `Result<T>` per YAML
document. Per-document deserialisation errors surface as `Err`
items so callers can recover and continue across document
boundaries; YAML *syntax* errors return synchronously from
`read` / `read_with_config` before iteration starts. The
implementation drains the reader into a `String` first
(`O(input_len)` peak memory); a future v0.0.3+ pass will tighten
this to `O(1-document)` once the parser learns to accept
incremental byte chunks.

### Confirmed shipped in v0.0.1 (closes issues #9, #27, #28, #30)

- **#9 — Event-based streaming deserialisation.**
  `StreamingDeserializer` is the fast path inside `from_str` /
  `from_str_with_config`; falls back to the AST loader only when
  the caller's config disables streaming-eligible features.
  Measured **30% faster** than the AST path (14.0 vs 19.4 µs;
  see [`doc/BENCHMARKS.md`](doc/BENCHMARKS.md#architecture-validation)).
- **#27 — Path query API.** `Value::query` /
  `BorrowedValue::query` ship dot notation, array indexing,
  wildcards (`*`), and recursive descent (`..`). Filter
  expressions (`[?field==value]`) remain optional and tracked
  separately.
- **#28 — Zero-copy `Value<'a>` AST.** Implemented via the
  parallel `BorrowedValue<'a>` type with `Cow<'a, str>` keys and
  values, shipped in v0.0.1 to avoid a breaking change to
  `Value`. Measured **18% faster** than the owned `Value` path.
- **#30 — Shared-memory DAGs via Rc/Arc anchor registry.**
  `AnchorRegistry<T>` (`Rc`) and `ArcAnchorRegistry<T>` (`Arc`)
  expose `register` / `resolve` returning shared pointers to the
  same heap allocation (verified by `Rc::ptr_eq` / `Arc::ptr_eq`
  in the type's doctests). Cyclic graphs use
  `RcRecursive`/`ArcRecursive` with their `Weak` partners.

### Changed — relax `indexmap` upper bound

Bumped `indexmap` requirement from `>=2, <2.11` to `>=2, <3`. The
old cap was defensive (we hadn't tested against 2.11+ at release
time), not load-bearing — noyalib only uses `IndexMap`,
`map::Iter`, `map::Entry`, and other stable public surface that
hasn't changed across the 2.x line. indexmap 2.10 and 2.11 share
MSRV 1.63, well below noyalib's own 1.75 floor.

The motivating downstream is `html-generator`, which pulls
`toml = "1.1"` (which depends on `indexmap ^2.11.4`); the previous
`<2.11` cap made the two co-resolution paths incompatible.

## [v0.0.1] — 2026-05-10

### Fixed — `Eq`/`Hash` invariant for `Number` floats (signed zero, NaN)

`PartialEq for Number` deliberately treats `+0.0 == -0.0` (per IEEE
754) and `NaN == NaN` (to satisfy `Eq` reflexivity). The `Hash for
Number` impl was hashing `f64::to_bits()` directly, which gives
distinct bit patterns for those equal values — surfaced by the
ubuntu-nightly `value_hash_consistent` proptest. Normalised both
edges in the hasher: zero hashes as `0u64` regardless of sign, NaN
hashes as a fixed quiet-NaN sentinel. Three explicit regression
tests pinned in `tests/proptest.rs`.

### Performance — bulk-copy quoted-scalar interior runs via SIMD prefix scan

The single- and double-quoted scalar fast paths used to read one
UTF-8 character at a time inside a `match self.peek()` loop —
`slice_str + push_str` per char. Replaced with `simd::clean_prefix_len`
over the appropriate ASCII needle set; semantics are bit-exact, ~30%
end-to-end on a worst-case 100KB single-quoted ASCII string. All
needles are ASCII, so slicing on a needle hit is char-boundary safe.

### Performance — Profile-Guided Optimization (PGO) infrastructure

New `scripts/pgo.sh` drives the full LLVM PGO pipeline:
instrumented build → train against `bench_corpus/` and the YAML
test suite → `llvm-profdata merge` → optimised rebuild. Documented
in `doc/PGO.md` and surfaced in `doc/POLICIES.md` §4 as an opt-in
5–15% extra speedup path on top of the default `cargo build
--release` numbers. Loader Vec/Mapping pre-sizing via
`Value::deserialize`'s `SeqAccess`/`MapAccess` `size_hint()` cuts
the first reallocation on the AST fallback path.

### CI — panic-free contract + unused-dep gate

- `tests/panic_free.rs`: 8 `proptest` properties + 19 historical-input
  regression cases verify that `from_str`, `from_slice`, `load_all`,
  and `cst::parse_document` never panic on arbitrary input. CI runs
  the proptest at the default seed; nightly stress-runs at
  `PROPTEST_CASES=16384`.
- `cargo-machete (unused-dep gate)` is now a required CI job —
  blocks PRs that add a dependency to `Cargo.toml` without using
  it. Catches accidental fat-tree imports.

### Fixed — defensive char-boundary clamp in `Scanner::slice_str`

Adversarial mixed-quote input (`"A:\r*aa {\"\\¡"`) could land
`slice_str` mid-codepoint and panic. Added a stable polyfill of
`str::floor_char_boundary` and clamp both `start` and `end` to
char boundaries before slicing. Three new fuzz targets in
`fuzz/fuzz_targets/` cover the new code paths.

### Fixed — Windows-only MCP atomic-write flake

`tool_call_set_preserves_comments` flaked on Windows when a
concurrent reader observed a half-written file. The `noyalib_set`
write helper now uses `MoveFileExW(MOVEFILE_REPLACE_EXISTING |
MOVEFILE_WRITE_THROUGH)` semantics on Windows so concurrent
readers see either the old or the new contents — never a
half-write or a stale-page-cache observation.

### Docs — satellite-crate enterprise-readiness sections

`noya-cli`, `noyalib-lsp`, `noyalib-mcp`, `noyalib-wasm` lib.rs
crate-level doc blocks now match the noyalib core's 12-dimension
template. Added: `# Cargo features`, `# Performance`, `# API
stability and SemVer` sections. WASM `# Panics` expanded to
enumerate WASM-specific abort sources (linear-memory OOM, stack
overflow on misconfigured `max_depth`, `panic = abort` on the
host). `noyalib-mcp` and `noyalib-wasm` READMEs now state the
explicit MSRV (1.75.0 and 1.85.0 respectively) and tier-1
platform list — bringing them into alignment with the
`noyalib-lsp` and `noya-cli` READMEs.

### Docs — diagnostic feature-gate fix

`tests/cst_schema_tag_audit.rs` referenced `validate_against_schema`
unconditionally but the symbol is gated behind
`feature = "validate-schema"`. Test-crate now compiles cleanly
under default features as well as `--all-features`.

### v0.0.2 milestone — implemented in v0.0.1

The seven open issues on the v0.0.2 milestone are closed inside
v0.0.1 per the "don't pre-emptively phase a bang launch"
principle. Public API additions:

- **`noyalib::Error::Budget(BudgetBreach)`** + the
  `BudgetBreach` enum (#3). Six new `ParserConfig` budgets:
  `max_events`, `max_nodes`, `max_total_scalar_bytes`,
  `max_documents`, `max_merge_keys`, `alias_anchor_ratio`.
  Each has a builder method on `ParserConfig`; `strict()`
  uses tighter caps. Enforced in `Loader::process_event`.
- **`noyalib::Error::render(source) -> String`** +
  `render_with_options(source, &RenderOptions)` (#2). New
  public types `RenderOptions { crop_radius, color }` and
  `CroppedRegion<'a>` for caller-facing diagnostic
  rendering. `format_with_source` / `format_with_source_radius`
  remain for backwards compatibility.
- **`RcRecursive<T>` / `ArcRecursive<T>` / `RcRecursion<T>` /
  `ArcRecursion<T>`** (#5). Late-init / cyclic-graph anchor
  wrappers in `noyalib::anchors`. Access via `.borrow()` /
  `.lock()`; `Serialize` / `Deserialize` impls delegate to the
  inner `T`.
- **`noyalib::RequireIndent`** + `ParserConfig::require_indent`
  (#6). API surface for indentation-validation modes
  (`Unchecked`, `Even`, `Divisible(N)`, `Uniform(Option<N>)`).
  Scanner-side enforcement is a follow-up per the issue's
  own "Blast Radius" note.

Already-implemented issues confirmed and closed: `!!binary`
support (#4 — `src/base64.rs`), yaml-test-suite compliance
runner (#26 — 406/406 strict), streaming anchor event replay
(#29 — `streaming.rs::anchor_events` + `replay_stack`).

Test coverage: 38 new regression tests across
`tests/{budget_breach,error_render,require_indent,recursive_anchors}.rs`.
Coverage gates: **95.63% functions / 93.16% lines / 92.31%
regions** (all above CI thresholds).

### Docs — README refactor: extracted deep weeds, grouped tooling cluster

The workspace README had grown to a 1 593-line full-doc website.
Two refinements:

- **Extracted** the full Benchmarks tables (deserialise /
  serialise / SIMD / SWAR / parallel / architecture-validation /
  project-metrics) into [`doc/BENCHMARKS.md`](doc/BENCHMARKS.md),
  and the full Ecosystem-comparison feature matrix into
  [`doc/COMPARISON.md`](doc/COMPARISON.md). The README keeps a
  ~10-line summary table for each, with a link to the full
  doc. Reading-the-table notes and the SWAR pipeline
  walkthrough live in the extracted files.
- **Re-grouped the tooling cluster.** The "Tooling" section
  was at line 391; now an ecosystem table sits right after
  Quick Start (line 213) under "The noyalib ecosystem"
  covering all five crates (`noyalib`, `noya-cli`,
  `noyalib-lsp`, `noyalib-mcp`, `noyalib-wasm`) with
  per-crate install commands and per-host quick-link entries
  pointing at the editor / MCP / ecosystem-gate config
  examples. The library-only deep-dive sections (Features,
  Custom tags, Governance, Policy, etc.) follow below in
  one block, so the library docs and the tooling docs are
  cleanly separated.

README size: **1 499 lines** (was 1 593). Two new doc files
absorbing 238 lines of detail.

### Docs — per-crate migration guides for the wider YAML ecosystem

Each non-`serde_yaml` Rust YAML crate now has its own dedicated
migration guide with the same shape as the original
`MIGRATION-FROM-SERDE-YAML.md` (TL;DR diff, function table,
behavioural notes, checklist). Crates.io state verified
**2026-05-08**:

- [`MIGRATION-FROM-SERDE-YML.md`](doc/MIGRATION-FROM-SERDE-YML.md) — `serde_yml` 0.0.12 (archived 2025-09)
- [`MIGRATION-FROM-YAML-SERDE.md`](doc/MIGRATION-FROM-YAML-SERDE.md) — `yaml_serde` 0.10.4 (active fork)
- [`MIGRATION-FROM-SERDE-YAML-NG.md`](doc/MIGRATION-FROM-SERDE-YAML-NG.md) — `serde-yaml-ng` 0.10.0 (active drop-in fork)
- [`MIGRATION-FROM-SERDE-NORWAY.md`](doc/MIGRATION-FROM-SERDE-NORWAY.md) — `serde-norway` 0.9.42 (hard-fork)
- [`MIGRATION-FROM-SERDE-YAML-BW.md`](doc/MIGRATION-FROM-SERDE-YAML-BW.md) — `serde-yaml-bw` 2.5.6 (non-drop-in 2.x)
- [`MIGRATION-FROM-SERDE-SAPHYR.md`](doc/MIGRATION-FROM-SERDE-SAPHYR.md) — `serde-saphyr` 0.0.26 (no `Value` DOM)
- [`MIGRATION-FROM-YAML-SPANNED.md`](doc/MIGRATION-FROM-YAML-SPANNED.md) — `yaml-spanned` 0.0.3 (parser-only)

The umbrella index lives at
[`doc/MIGRATION.md`](doc/MIGRATION.md) and points at all eight
guides via a compatibility matrix. The workspace README and the
`noyalib` crate README both link into the per-crate guides.

### YAML Test Suite — 100% strict (406/406, 0 skip)

The historical 18-case `SKIP_LIST` (2JQS, 6WLZ, 6CK3, P76L, 6VJK,
UT92, WZ62, 4ABK, M7A3, K527, 9WXW, V9D5, CFD4, KK5P, M2N8, M5DY,
RZP5, XW4D) is gone — the parser now passes every active YAML
1.2 Test Suite case under strict comparison. The skip list was a
historical artefact of an earlier parser state; under the
current scanner + loader those cases produce values that match
the suite's expected JSON. The lenient `official_suite.rs`
runner additionally needed a tag-stripping JSON projection
(`yaml_value_to_json`) to align with the suite's tag-less
expected shape after the `Value::Tagged` preservation work.

**Both runners now report 406/406 = 100.0% strict, 0 skip, 0 fail.**

### Added — Stress / load test battery (`tests/stress_load.rs`)

13 new regression tests pinning the parser's behaviour under
pathological input:

- 1 MB single block-scalar document.
- 10 000-entry mapping / 10 000-item sequence.
- 1 000-document multi-document stream.
- 100-level deep nesting + recursion-limit DoS guard at 10 000.
- Billion-laughs-style alias amplification rejection.
- 1 MB long plain scalar.
- 100-iteration parse-emit-reparse stability.
- Unicode-heavy document (emoji / CJK / RTL).
- Custom `ParserConfig` low `max_depth` enforcement.
- 1 000 anchors + aliases within budget.

### Performance — release profile tuned for speed

`[profile.release]` `opt-level` flipped from `"s"` (size) to `3`
(speed) for the workspace. The library's per-byte scanner
dispatch inlines and vectorizes meaningfully better at `3`. WASM
bundle size is managed separately by the `wasm-pack` post-build
`wasm-opt -Os` pass (see `crates/noyalib-wasm/README.md`),
keeping the published `.wasm` at its target ~338 KB.
`overflow-checks = true` is preserved on the security-vs-speed
trade-off — the parser handles untrusted input and cannot afford
silent wraparound on indent / depth / size counters.

### Fixed — Windows-only MCP test flake (`tool_call_set_preserves_comments`)

`noyalib_set` previously called `fs::write(file, …)` directly.
On Windows the test's `read_to_string` could observe stale
contents because the kernel page-cache hadn't flushed by the
time the spawned MCP child exited. Replaced with an
*atomic-write* helper: write to a sibling temp file, `sync_all`,
then `rename` over the target. The rename is atomic on POSIX
and on Windows under `MoveFileExW(MOVEFILE_REPLACE_EXISTING |
MOVEFILE_WRITE_THROUGH)` semantics, so concurrent readers see
either the old or the new contents — never a half-write or a
stale cache.

### Fixed — nested `Value::Tagged` inside a tagged container (C4HZ regression)

`from_str::<Value>("!shape\n- !circle 1\n")` previously collapsed
the inner `Tagged(circle, "1")` into a single-key
`Mapping{"!circle": "1"}` because
`TagPreservingMapAccess::next_value_seed` handed the inner
`Value` to a tag-blind `&'de Value` Deserializer. Fixed by
re-wrapping the inner value in
`crate::de::Deserializer::with_options_preserving_tags(...)` so
nested `Value::Tagged` survives every layer of the data-binding
return path. Restores YAML test suite C4HZ ("Spec Example 2.24
Global Tags") to the strict-pass set — strict compliance back
to **100.0% (387/387)**.

### Added — `to_string_value` / `to_writer_value` for lossless `Value::Tagged` emit

- **`noyalib::to_string_value(&Value) -> Result<String>`** and
  the `_with_config` variant emit a `Value` directly via the
  YAML-tag-aware writer, skipping the `Serialize` pipeline.
  Required when the input may contain `Value::Tagged(...)` and
  the caller wants the YAML-tag wire form to survive on emit.
- **`noyalib::to_writer_value<W: io::Write>(W, &Value) -> Result<()>`**
  and the `_with_config` variant — same contract, writing into
  any `io::Write`.
- **Why these are separate from `to_string` / `to_writer`**: the
  generic family routes `Value::Tagged` through
  `Serializer::serialize_map` (which is the right shape for
  `serde_json` and other serde-bridge consumers) and that
  flattens the tag into a single-entry map on emit. Exposing the
  YAML-tag-aware path under a distinct name keeps the
  `Serialize`-trait contract clean while giving `Value` users a
  lossless emit option.

### Migration notice (pre-launch — applies before v0.0.1 is tagged)

Two source-level changes ship in `[Unreleased]` that downstream
crates touching the published `from_*` family will see. Both are
non-breaking for typed deserialise; they affect only the
`from_str::<Value>` and `from_value::<Value>` shapes.

1. **Tag preservation**: a `from_str::<Value>("!Custom 'hi'\n")`
   that previously returned `Value::String("hi")` now returns
   `Value::Tagged(Tag("!Custom"), Value::String("hi"))`. Code
   that read tagged scalars via `as_str` / `as_i64` / etc. needs
   either a wrapper unwrap (`value.untag_ref().as_str()`), a
   typed deserialise (`#[derive(Deserialize)] struct Foo`), or a
   tag-aware `match`. See the migration recipe in
   [`doc/MIGRATION-FROM-SERDE-YAML.md`](doc/MIGRATION-FROM-SERDE-YAML.md#1-valuetagged-is-a-7th-variant--and-noyalib-preserves-scalar-tags-too).
2. **`T: 'static` bound** on the public `from_str` /
   `from_str_with_config` / `from_slice*` / `from_reader*` /
   `from_value` family. Every real-world `DeserializeOwned` type
   already satisfies it (the HRTB on its own already disallows
   borrowed lifetimes); the `'static` is what lets noyalib detect
   at the call site whether `T == Value` and engage the
   tag-preserving fast path. Add `+ 'static` to bound expressions
   in any wrapper functions you wrote on top of noyalib's
   `from_*`. Trait signatures from external crates (e.g.
   `figment::Format::from_str`) that drop `'static` are handled
   by a private internal entry point — your existing
   `impl Format for ...` keeps compiling.

### Added — Custom-tag scalar `Value::Tagged` surfacing on the default deserialise path

- **`from_str::<Value>("!Custom 'hi'")`** now returns
  `Value::Tagged(Tag("!Custom"), Value::String("hi"))` instead
  of unwrapping to the inner `Value::String("hi")`. The tag
  survives the data-binding return path so downstream consumers
  can dispatch on it. Tagged sequences and tagged mappings
  already worked via the AST loader; this closes the gap for
  scalars.
- **Typed targets are unchanged.** A `#[derive(Deserialize)]
  struct Foo { x: u8 }` against `!Foo {x: 1}` still sees through
  the tag — that's the correct behaviour for the typed path and
  the only one that lets schema-tagged inputs deserialise into
  bare structs.
- **Mechanism**: the `from_str_with_config` / `from_value` entry
  points detect `T == Value` via [`std::any::TypeId`] and engage
  a `preserve_tags` flag on the noyalib `Deserializer`. When the
  flag is on, tagged values are surfaced through a magic-key
  MapAccess that `Value::deserialize`'s visitor recognises and
  reconstructs as `Value::Tagged`. Other Deserializers
  (`serde_json`, `figment`, FlatMap-shaped flatten extras) never
  see the magic shape.
- **API change**: the public `from_str` / `from_str_with_config`
  / `from_slice*` / `from_reader*` / `from_value` family now
  carries a `T: 'static` bound (in addition to the existing
  `for<'de> Deserialize<'de>`). This is a soft constraint that
  every real-world `DeserializeOwned` type already satisfies —
  the HRTB itself disallows borrowed lifetimes — and unlocks the
  TypeId-driven dispatch above. `figment` integration uses a
  private non-`'static` typed entry-point so its `Format::from_str`
  signature stays compatible.
- 4 regression tests retargeted from the old transparent-unwrap
  behaviour to the new tag-preserving contract:
  `tests/de.rs::test_deserialize_tagged_value`,
  `tests/coverage_100.rs::loader_tag_primary_empty_suffix` and
  `loader_custom_tag_with_inner_resolution`,
  `tests/coverage_boost.rs::loader_span_custom_tag_empty_suffix`,
  `tests/tag_registry.rs::unregistered_tag_on_scalar_falls_back_to_string`
  and `empty_registry_is_no_op`.

### Added — Truncated error formatters

- **`Error::format_with_source_truncated(source, max_chars)`**
  and **`Error::format_with_source_radius_truncated(source,
  radius, max_chars)`** — bridge-channel-friendly variants of
  the existing snippet renderers. Cap rendered diagnostics at a
  caller-supplied character budget, truncating on a UTF-8
  character boundary and appending an ASCII `...` ellipsis. Use
  for log lines, Slack messages, Sentry tags, or any sink with a
  hard length budget.
- Truncation contract: `<= max_chars` characters in the
  output; UTF-8-aligned cut; `...` appended unless `max_chars <
  3` (in which case the prefix that fits is returned without an
  ellipsis).
- Four unit tests cover the under-budget passthrough, the
  over-budget ellipsis, the tiny-budget ellipsis-drop, and
  multi-byte character alignment.

### Fixed — JSON-style UTF-16 surrogate pair pairing in `\uXXXX` escapes

- Double-quoted YAML scalars now accept `𝄞` (high + low
  surrogate) and combine the two halves into the corresponding
  supplementary-plane code point via the UTF-16 algorithm.
  Previously these escapes errored as "invalid Unicode code point
  U+D834" because `char::from_u32` rejects surrogate halves
  outright. JSON-emitting YAML producers commonly emit pair form,
  so the rejection was a real interop hit.
- Lone, reversed, and truncated surrogates remain rejected with
  the same error shape — the change is additive, not relaxed.
- 13 integration tests in `tests/json_surrogate_escape.rs` cover
  musical G clef (U+1D11E), grinning face emoji (U+1F600),
  multiple pairs in sequence, BMP-escape interleaving, and every
  rejection path.

### Added — Borrowed-path alias resolution

- **`BorrowedValue<'a>`** now eagerly resolves YAML anchors
  (`&name`) and aliases (`*name`). The anchored value is stored
  in a side-table; each alias clones it into the tree. String
  fields stay `Cow::Borrowed` so the clone is mostly free —
  only sequences and mappings actually duplicate, matching the
  owned `Value` path's behaviour.
- Alias-bomb defence: total expansions are capped by
  `ParserConfig::max_alias_expansions`, the same limit the owned
  path enforces.
- Aliases used as mapping keys coerce scalars to `Cow<'a, str>`
  (string / bool / number / null); non-scalar key aliases error
  rather than silently coercing — keeps the `Mapping` key type
  honest.
- Anchor namespace resets on `DocumentEnd` per spec.
- Previously the borrowed path errored with "aliases not
  supported in borrowed mode"; that message is gone. API surface
  is unchanged — the `BorrowedValue` enum and constructors are
  byte-identical.
- 12 integration tests in `tests/borrowed_alias_resolution.rs`
  cover scalar / sequence / mapping anchors, alias-as-key,
  multi-doc namespace isolation, unknown-anchor errors,
  expansion-cap defence, and round-trip parity with the owned
  path.

### Added — YAML 1.1 mode toggle

- **`ParserConfig::version(YamlVersion::V1_1)`** — single-call
  preset that flips the three resolver-table differences between
  YAML 1.2 (default) and 1.1 on as a bundle: `yes`/`no`/`on`/`off`
  booleans, bare-`0` octal `0644`, sexagesimal `60:00`. Selecting
  `V1_2` resets the trio so a config can be reverted without
  rebuilding from scratch.
- The fine-grained `legacy_booleans` / `legacy_octal_numbers` /
  `legacy_sexagesimal` flags remain available for callers who
  want to mix and match (e.g. "1.1 booleans but reject octal
  `0644`"). `version()` sets the preset; individual flags refine.
- New public type **`noyalib::YamlVersion`** with `V1_1` /
  `V1_2` variants. `Default::default()` is `V1_2`.
- 11 integration tests in `tests/yaml_version.rs` cover default
  behaviour, the 1.1 preset, the 1.2 reset, override-after-preset
  composability, and a Kubernetes-flavoured mixed-1.1-isms
  document round-trip.

### Added — `compat-serde-yaml` symbol parity

- **`Deserializer`** and **`Serializer`** types now re-export
  under `noyalib::compat::serde_yaml`. Existing
  `serde_yaml::Deserializer` / `Serializer` references compile
  unchanged after the prefix swap.
- New **`compat::serde_yaml::{value, mapping, with}`**
  sub-modules mirror the upstream layout. Migrating code that
  imports via the path form (`serde_yaml::value::Tag`,
  `#[serde(with = "serde_yaml::with::singleton_map")]`) only
  needs a search-and-replace on the prefix.
- The `with` sub-module re-exports all four
  `singleton_map_*` helpers + `nested_singleton_map`.
- 5 new tests in `compat/serde_yaml.rs` — the compat suite is
  now 13/13 green.

### Added — Lean / minimal dependency profile

- New **`fast-int`**, **`fast-float`**, and
  **`strict-deserialise`** Cargo features make `itoa`, `ryu`,
  and `serde_ignored` optional. All three are on by default —
  the lean profile is opt-out.
- New **`minimal`** meta-feature alias — equivalent to
  `default-features = false, features = ["std"]` — drops the
  three deps for FIPS / embedded / audit-heavy environments.
  Numeric formatting falls back to `core::fmt` (slower; output
  remains valid YAML); the `from_str_strict` /
  `from_slice_strict` / `from_reader_strict` typo-detection
  helpers are absent.
- Default profile: 8 runtime deps. Lean profile: 5 — drops
  `itoa`, `ryu`, `serde_ignored`. Verified via `cargo tree`.
- README's Install section documents the trade-off.

### Added — Strict deserialise on every input shape

- **`noyalib::from_slice_strict<T>`** and
  **`noyalib::from_reader_strict<R, T>`** — same unknown-field
  detection semantics as `from_str_strict`, but accepting `&[u8]`
  and `impl io::Read` directly so callers already holding bytes
  or a reader don't have to round-trip through `String` to opt
  in. Both gated behind `#[cfg(feature = "std")]` to match the
  existing string-input variant; both re-exported from the crate
  root.
- Five new integration tests in `tests/ux_diagnostics.rs` cover
  happy path + typo detection on both new helpers, plus
  invalid-UTF-8 rejection on the slice path. Doc-tests on each
  helper give an executable usage example.
- README "Strict deserialise" section gains an input-shape × API
  matrix (`&str` / `&[u8]` / `impl io::Read` × lenient / strict).

### Added — Ecosystem-citizen examples

- Six new examples that show noyalib slotting into the standard
  Rust configuration / validation / diagnostics toolbox without
  custom glue:
  - `include` — `$include`-key modular configs (Argo CD / JSON
    Schema `$ref`-style cross-file references) with cycle detection.
  - `figment` — layered defaults / YAML / env composition through
    the `figment::Provider` we already ship under the `figment`
    feature; demonstrates per-environment overlay chains.
  - `validation_garde` — declarative logic validation via the
    `garde` crate paired with `Validated<T>`.
  - `validation_validator` — same scenario through the
    `validator` crate (Actix / Axum / Rocket idiom) paired with
    `ValidatedValidator<T>`.
  - `diagnostic_path` — `serde_path_to_error` integration that
    pinpoints the offending nested key (including sequence indices
    such as `server.replicas[1].weight`) in deeply structured
    documents.
  - `robotics_polymorphism` — tagged-enum dispatch + the
    `Degrees` / `Radians` / `StrictFloat` newtypes from the
    `robotics` feature, illustrating unit-aware parsing on a
    Tree-Planting Robot mission plan.
- The `figment` Cargo dep now activates its `env` feature so the
  example chain (`Yaml::string` → `Env::prefixed`) compiles
  without consumers having to opt into it themselves.

### Added — Key interner

- **`noyalib::interner::KeyInterner`** — `&str` → `Arc<str>`
  deduplication primitive for memory-efficient repeated-key
  workloads. Each call to `intern(key)` returns a shared
  `Arc<str>`; the first call allocates, every subsequent call
  with the same key bytes returns a clone of the cached entry.
- Targets the Kubernetes-shaped use case where keys like
  `metadata`, `labels`, `name`, `apiVersion`, `selector` repeat
  thousands of times across a stream. For 20-byte keys repeated
  10 000 times, footprint drops from ~200 KB to ~20 bytes +
  `Arc` pointers.
- Public surface: `KeyInterner::new`, `with_capacity(n)`,
  `intern(&str) -> Arc<str>`, `get(&str) -> Option<Arc<str>>`,
  `len`, `is_empty`, `clear`.
- The `Mapping` public API is **unchanged** — `Mapping<String,
  Value>` is preserved so existing call sites compile clean. A
  future major version may swap the internal storage to
  `Arc<str>` and use the interner transparently during parse;
  v0.0.1 ships the primitive without that breaking change.
- 7 unit tests covering basic intern semantics, distinct-key
  separation, empty-string handling, `get` lookup,
  `clear` semantics, and a Kubernetes-key-set dedup smoke test.

### Changed — CST short-pointer compression

- `GreenChild::Token { len }` is now `u32` (was `usize`). YAML
  documents are bounded at 4 GiB by the parser's
  `max_document_length` cap, so a `u32` is sufficient. The
  narrower field drops `GreenChild::Token` from 24 bytes to 8
  bytes on a 64-bit target — meaningfully better L1/L2 cache
  locality on tree traversals.
- `GreenNode.text_len` is similarly narrowed from `usize` to
  `u32` (private field; public `text_len()` accessor still
  returns `usize` for ergonomic call-site arithmetic).
- Public `text_len()` accessors on `GreenChild` and `GreenNode`
  preserve their `usize` return type — the narrower storage is
  widened at the API boundary so existing callers continue to
  compile.
- 387/387 strict YAML 1.2 test-suite pass preserved (0 failures,
  19 deliberate skips out of 406 variant assertions); full test
  suite + doctest sweep green.

### Added — Parallel multi-document parsing

- **`noyalib::parallel::parse<T>(input) -> Result<Vec<T>>`** and
  **`noyalib::parallel::values(input) -> Result<Vec<Value>>`** —
  pre-scan `---` document boundaries on a single thread, then
  deserialise each document in parallel via Rayon. Targets
  multi-document streams (telemetry logs, audit exports,
  Kubernetes-resource snapshots) where single-thread parsing is
  CPU-bound.
- **`noyalib::parallel::split(input) -> Vec<&str>`** — the
  standalone document-boundary pre-scanner. Useful when the caller
  wants to drive their own concurrency primitives (async tasks,
  custom thread pools).
- Gated behind the `parallel` Cargo feature (off by default —
  pulls in `rayon` only when the user asks for it).
- 10 unit tests covering boundary detection edge cases (no
  separators, empty input, implicit first document, mid-line
  `---`, dashes followed by non-whitespace) and end-to-end
  correctness against `load_all_as`.

### Added — SWAR decimal-integer parser

- **`noyalib::simd::parse_decimal_u64` / `parse_decimal_i64`** —
  branch-free 8-digits-per-cycle SWAR pipeline replacing the
  stdlib byte-by-byte loop. Three pair-wise multiply-add phases
  fold a `u64` chunk of ASCII digits into the parsed value with
  no per-byte branch.
- Plumbed into the streaming integer resolver
  (`crate::streaming::parse_integer`); base-10 plain scalars now
  flow through the SWAR path. Hex / octal / sign-prefixed paths
  retain stdlib for spec-correct overflow semantics.
- Bench results (`benches/numeric_parse.rs`):
  - 3-digit input: parity with stdlib (SWAR doesn't engage).
  - **8 digits: 2.17× faster** (8.12 ns → 3.74 ns).
  - **19 digits: 2.38× faster** (22 ns → 9.25 ns).
  - **i64::MAX / i64::MIN: 2.5× faster.**
  - **Bulk parse of 1000 integers: 47 % faster.**
- Validation: every byte checked in `b'0'..=b'9'` before
  arithmetic; `wrapping_mul` is intentional in the SWAR pipeline
  (high bits discarded by downstream shift-and-mask) and the
  validator rejects malformed input. Overflow returns `None`.
- 11 unit tests including baseline equivalence against
  `<u64 as FromStr>::from_str` across 19 representative values
  (covers `i64::MIN`, `i64::MAX`, `u64::MAX`, sign handling).
- 387/387 strict YAML 1.2 test-suite pass preserved (0 failures,
  19 deliberate skips out of 406 variant assertions).

### Added — Canonical scanner needle constants

- **`simd::BLOCK_PLAIN_NEEDLES`** / **`simd::FLOW_PLAIN_NEEDLES`** /
  **`simd::LINE_BREAK_NEEDLES`** — public `&[u8]` constants
  documenting the YAML 1.2 plain-scalar boundary candidate sets
  per parser context. Future scanner refactors can reach the
  canonical set via these names without re-deriving them.

### Added — Structural bitmask discovery (`simdjson`-style)

- **`SimdScanner::structural_bitmask_32(&[u8; 32]) -> u32`** — load
  a 32-byte chunk and produce a dense bitmask where bit `i` is set
  iff `chunk[i]` is in the scanner's needle set. The building
  block of `simdjson`-style structural discovery: instead of
  walking the haystack and stopping at every delimiter, callers
  drain the mask via `mask.trailing_zeros()` + `mask & (mask - 1)`
  and advance the parser state machine directly from one delimiter
  to the next.
- **`StructuralIter`** — iterator that walks every structural-byte
  position in a haystack of arbitrary length. Handles the chunk
  loop, the partial-chunk tail, and the cached-bit drain
  internally so callers see one stream of byte offsets in order.
- Bench results (`benches/structural_bitmask.rs`, real YAML-shaped
  input):
  - **Stable Rust**: 4.2× faster than the existing memchr-loop
    structural-discovery path across 4 KiB / 64 KiB / 1 MiB.
  - **Nightly with `nightly-simd`**: 9.2× faster than the same
    baseline (single `Simd<u8, 32>` chunk + branchless
    `to_bitmask()` per 32-byte window).
- Five unit tests + cross-needle-set baseline equivalence check
  (every YAML-relevant arity 1 / 2 / 3 / 7 / 10) and four
  `StructuralIter` correctness tests covering chunk-boundary
  straddles, partial tails, and 2 KiB adversarial inputs against a
  scalar baseline.

### Removed — `thiserror` runtime dependency

- noyalib's `Error` enum no longer derives `thiserror::Error`. The
  `Display` and `std::error::Error` impls are now hand-written
  (matching the previous `#[error(...)]` format strings byte-for-
  byte, so all `Display` output is stable across the migration).
- Drops the proc-macro from every downstream crate's compile graph
  — meaningful for downstream build times in big workspaces.
- The runtime dep list is now: `serde`, `indexmap`, `rustc-hash`,
  `itoa`, `ryu`, `memchr`, `smallvec`. Every other dep is feature-
  gated and off by default.

### Changed — `serde` defaults synchronised with our `std` feature

- `serde = { default-features = false, features = ["derive",
  "alloc"] }` plus `std = ["serde/std"]` so `cargo build
  --no-default-features` actually compiles in no_std mode (serde's
  `de::Error` super-trait `StdError` resolves to a no_std-friendly
  bound when serde is itself in no_std mode).

### Removed — `serde_yaml` 0.9 upstream dependency

- The `compat-serde-yaml` shim **no longer pulls in the
  unmaintained `serde_yaml` 0.9 crate**. Every type the shim
  exposes (`Value`, `Mapping`, `Number`, `Sequence`, `Tag`,
  `TaggedValue`, `Error`, `Location`) is a noyalib-native type
  re-exported under the `serde_yaml` name; downstream
  `cargo audit` / `cargo deny` runs no longer pick up the
  archived advisory chain.
- The previous direct `From<noyalib::Value> for ::serde_yaml::Value`
  / `TryFrom<::serde_yaml::Value> for noyalib::Value` impls are
  removed. Mid-migration codebases route in-flight upstream values
  through the Serde data model instead — the universal-translator
  path the Serde ecosystem already provides for every JSON-shaped
  AST pair: `noyalib::to_value(&upstream_serde_yaml_value)?`.

### Added — Release-candidate examples and benches

- **`examples/entry_api.rs`** — surgical Kubernetes manifest
  patching via the `Document::entry` proxy API. Demonstrates
  `or_insert` / `insert_value` / `set` chained edits with every
  comment, indent, and sibling preserved byte-for-byte.
- **`examples/flattened.rs`** — `Flattened<T>` capture pattern:
  typed view + raw metadata view from one parse pass.
- **`examples/schema_validation.rs`** — library-level
  `schema_for` + `validate_against_schema` + `coerce_to_schema`
  pipeline. Mirrors what `noyavalidate --fix` does on the CLI.
- **`benches/streaming_vs_value.rs`** — head-to-head throughput
  comparison between `StreamingDeserializer` and
  `from_str::<Value>` across small / medium / large workloads,
  plus a dedicated `BTreeMap` MapAccess scenario.
- **`benches/large_doc_soak.rs`** — 1 MiB / 10 MiB / 50 MiB soak
  benchmark catching quadratic regressions and SIMD hot-path
  regressions on long-input workloads.

### Changed — MSRV-1.75 hardening

- Pinned `indexmap` to `2.10.0` and `rustc-hash` to `2.0.0` so the
  resolver does not pull manifests requiring Rust 2024 edition
  (Cargo 1.85+) and breaking the MSRV-1.75 check.
- Removed unused `yaml_lib` dev-dependency (its manifest also
  required edition 2024).
- Promoted `serde-saphyr` (optional `compare-saphyr` feature):
  the saphyr lineage adopted edition 2024 across all available
  versions; gating it lets the comparison benchmarks still run on
  newer toolchains while keeping the default 1.75 build path
  clean.
- Demoted `pub` → `pub(crate)` on internal `Span`, `Token`,
  `ScanError`, `ParsedDocument`, `SubtreeContext` fields to
  satisfy the workspace `unreachable_pub = "forbid"` lint on
  Rust 1.75 (the lint behaviour tightened between 1.75 and the
  current stable, so the existing `pub` declarations on
  `pub(crate)` parents only failed under the older toolchain).

### Added — Streaming `!!binary`

- **`StreamingDeserializer` honours `!!binary` natively** — `serde_bytes`
  byte targets (`Vec<u8>` with `#[serde(with = "serde_bytes")]`,
  `serde_bytes::ByteBuf`) now decode RFC 4648 base64 directly inside
  the streaming path without falling back to the AST. Mirrors the
  AST-path type contract: untagged plain scalars that resolve to
  int / float / bool / null produce a `TypeMismatch` rather than
  silently coercing their UTF-8 representation to bytes.

### Added — Schema-driven type coercion (surgical `--fix`)

- **`noyalib::coerce_to_schema(value, schema) -> Result<usize>`** —
  walks JSON Schema 2020-12 type-mismatch errors against an
  in-memory `Value` and coerces string-shaped values into the
  schema's expected type when the parse succeeds. Targets the most
  common hand-written-YAML failure mode: `port: "8080"` gets
  rewritten to `port: 8080` automatically when the schema says
  `port: integer`.
- Handles three coercions: `String → Integer`, `String → Number`,
  `String → Boolean`. Unparseable inputs are left in place so the
  caller can surface the residue via a follow-up
  `validate_against_schema` call.
- Iterative fix-loop (capped at 1024 passes) re-runs validation
  after each coercion so cascading errors converge cleanly.
- 8 integration tests in `tests/coerce_to_schema.rs` cover the
  three target types, nested objects, sequence items, mixed
  valid / fixable / unfixable inputs, and the no-op case.

### Added — Portable-SIMD structural scanner

- **`SimdScanner` type** in `noyalib::simd` — build-once,
  scan-many byte-set finder optimised for parser inner loops.
  Stable Rust uses the existing memchr / SWAR / bitmap path; the
  new `nightly-simd` Cargo feature widens the inner loop to a
  32-byte `Simd<u8, 32>` chunk via `core::simd` portable SIMD,
  broadcasting each needle and OR-ing equality masks for
  branch-free structural detection.
- **`build.rs` toolchain probe** — emits `cfg(noyalib_nightly)`
  when `rustc --version` reports a nightly channel, so the
  `feature(portable_simd)` attribute is gated on both the user's
  feature flag and the actual compiler — `--all-features` on
  stable continues to compile cleanly.
- Both code paths are exhaustively cross-checked against a scalar
  baseline across needle widths 2 / 4 / 8 / 10 and haystack lengths
  spanning the SIMD chunk boundary (31 / 32 / 33 / 64 / 128 / 1024).

### Added — Pluggable parser policies

- **`noyalib::policy` module** — `Policy` trait with
  `check_event(&PolicyEvent)` and `check_value(&Value)` hooks for
  enforcing organisational "Safe YAML" constraints during parsing.
- **Built-in policies**: `DenyAnchors` (rejects `&name` definitions
  and `*name` aliases — covers the billion-laughs vector and
  audit-readability concerns), `DenyTags` (rejects custom tags
  while permitting YAML 1.2 core tags), `MaxScalarLength(n)` (caps
  individual scalar size in bytes).
- **`ParserConfig::with_policy(p)`** — register one or more
  policies; they run in registration order during the AST loader's
  event walk. The streaming fast-path is bypassed automatically
  when any policy is registered, ensuring uniform enforcement.
- 11 integration tests in `tests/policy.rs` cover each built-in
  policy, custom-policy composition, short-circuit-on-first-error
  semantics, and streaming-path bypass.

## [0.0.1] - 2026-05-04

The launch release. Sections below catalogue every capability the
library ships at launch, grouped by theme. See
[`doc/design/`](doc/design/) for the architecture rationale and
the commit history on `main` for per-change context.

### Added — Property interpolation

- **`Value::interpolate_properties(&map)`** — substitute `${name}`
  references inside string scalars from a property map. Walks
  recursively into sequences, mappings, and tagged values; map
  keys are left unchanged so the schema stays stable. `${{` and
  `}}` escapes preserve literal `${` / `}`. Returns
  `Error::Custom` on unknown placeholders.
- **`Value::interpolate_properties_lossy(&map)`** — same walk,
  but unknown placeholders substitute the empty string instead of
  erroring. Suitable for env-var expansion where missing
  variables should silently degrade.
- Placeholder names match `[A-Za-z_][A-Za-z0-9_.]*` so dotted
  hierarchies like `${db.host}` work.

### Added — serde-ecosystem interop

- **`serde_path_to_error` interop** — verified by
  `tests/serde_ecosystem.rs`; the path through nested structures
  and sequences is reported correctly when wrapping noyalib's
  `Deserializer`.
- **`serde_ignored` interop** — same test file confirms unknown
  fields at the top level and at any depth are surfaced through
  the standard wrapper without noyalib-specific integration.

### Added — `figment` provider

- **`figment` Cargo feature** — pulls in `figment` 0.10 and
  exposes `noyalib::figment::Yaml`, a drop-in `Format` + `Provider`
  that plugs into `Figment::merge` / `Figment::join` chains the
  same way `figment::providers::Toml` and
  `figment::providers::Json` do.
- 8 integration tests in `tests/figment_provider.rs` cover
  string/file extraction, layered merge / join semantics, parse-
  and missing-field error propagation, nested struct round-trip,
  and YAML 1.2 anchor + alias resolution through the provider.

### Added — `ParserConfig` knobs

Four additive `ParserConfig` toggles, all defaulting to YAML 1.2
spec behaviour (zero impact on existing callers):

- **`merge_key_policy`** with [`crate::MergeKeyPolicy`] —
  `Auto` (default) preserves YAML 1.2 §10.2 merge semantics;
  `AsOrdinary` keeps `<<` as a literal key in the resulting
  mapping; `Error` rejects any document containing a `<<` key.
  When set to non-`Auto`, the deserializer routes through the
  AST loader (the streaming path hard-wires the YAML 1.2
  semantics).
- **`no_schema`** — when `true`, every plain scalar surfaces as
  a `Value::String` regardless of whether it would normally
  resolve to `null` / `bool` / int / float. The "Norway problem"
  fix: schema strictness is opt-in. Quoted scalars and explicit
  tags (`!!int`, `!!bool`) are unaffected.
- **`legacy_octal_numbers`** — when `true`, accepts YAML
  1.1-style bare `0`-prefix octal literals (`0644` → 420) in
  addition to the YAML 1.2 `0o644` form. Numerics with `8` or
  `9` digits fall through to decimal even with the toggle on.
- **`ignore_binary_tag_for_string`** — when `true`,
  deserializing `!!binary "ABCD"` into a `String` target yields
  the literal base64 source string rather than rejecting on tag
  mismatch. The canonical bytes path (`Vec<u8>`,
  `serde_bytes::ByteBuf`) is unaffected — it always decodes the
  base64 payload. Useful for migrations from Python pyyaml-style
  applications that treat the tag as advisory.

### Added — `Flattened<T>` capture wrapper

- **`noyalib::Flattened<T>`** — pairs a typed deserialization of
  `T` with the underlying [`Value`] tree captured from the
  source. Solves the "I want `#[serde(flatten)]` plus the dynamic
  view for span lookup / unknown-field detection / schema
  validation" use case that the built-in residue types
  (`HashMap<String, Value>` etc.) erase. Deserializes by
  capturing the input as a [`Value`] first, then re-running
  `T::deserialize` against the captured tree via
  [`crate::from_value`]. Both `flattened.value: T` and
  `flattened.raw: Value` are exposed; `Deref<Target = T>` makes
  the typed view ergonomic. Round-trip transparency on
  serialize: only the typed view is emitted, mirroring
  `Spanned<T>`.

### Added — `legacy_sexagesimal` ParserConfig toggle

- **`ParserConfig::legacy_sexagesimal(true)`** — accept YAML
  1.1-style colon-separated base-60 numbers (`60:00` → 3 600,
  `1:30:00` → 5 400, `-1:30:00` → -5 400) as integers.
  Fractional last-component variant (`1:30:00.5` → 5 400.5)
  resolves to a float. Off by default; YAML 1.2 dropped the
  sexagesimal schema. Robust against false positives:
  components other than the first are clamped to 0..=59 and
  ISO-8601 timestamps with embedded `:` colons are correctly
  classified as strings, not as sexagesimal.

### Added — `JsonSchema` for `noyalib::Value`

- **`impl JsonSchema for noyalib::Value`** (gated by the
  `schema` feature) — emits the JSON Schema 2020-12 idiom for
  "any JSON-expressible value": a `oneOf` union of null,
  boolean, number, string, array, and object, with the array /
  object cases referencing the same `YamlValue` definition
  recursively. Lets users derive [`schemars::JsonSchema`] on a
  struct that has a `Value` field (e.g. an envelope type whose
  `payload` is "any user-supplied YAML") without writing a
  custom impl.

### Added — Mutable-Value experience for the CST

- **`Entry::or_insert(default)`** / **`or_insert_with(f)`** /
  **`or_insert_value(default)`** — std-collections-style
  ergonomics on top of the existing path-shaped Entry handle.
  Returns `Ok(true)` when the splice ran (path was vacant),
  `Ok(false)` when the path was already occupied. Top-level
  keys and sequence-index paths get actionable errors that
  redirect to `Document::set` and `push_back`/`insert_after`
  respectively.
- **`Entry::and_modify(f)`** — closure runs only when the path
  resolves; receives a `&mut Document` for arbitrary
  cross-path mutations. Returns `self` so the standard
  `and_modify(...).or_insert(...)` pattern composes.
- **`Document::rename_anchor(old, new)`** — atomic rename of
  every `&old` declaration and every `*old` reference in one
  operation. Returns the count of touched sites. The whole
  rename is performed as a single `replace_span` over the
  document so intermediate states with mismatched anchor /
  alias names are never observed. Validates `new` against YAML
  1.2 §6.9.2 (no flow indicators or whitespace).

### Added — Style heuristics for CST inserts

- **`Document::dominant_quote_style()`** returns the file's
  preferred scalar quote style (`Plain`, `SingleQuoted`, or
  `DoubleQuoted`) by tallying every quoted scalar in the green
  tree and breaking ties in favour of the simpler form. Plain
  mapping keys are deliberately ignored — the question is
  "when the user *did* quote a value, what did they reach
  for?".
- **`Document::dominant_flow_style()`** returns the dominant
  collection layout (`FlowStyle::Block` or `FlowStyle::Auto`)
  by counting Block vs Flow mappings and sequences.
- **`Entry::insert_value`** now consumes both heuristics: a new
  `Value::String` value gets the file's dominant quote style
  applied to the spliced fragment (manual quoting since the
  serializer's `scalar_style` config does not affect top-level
  scalars); collections continue to splice in block form for
  multi-line emissions. The `dominant_flow_style()` accessor
  is exposed for callers who want to wrap typed collections in
  `fmt::FlowMap` / `fmt::FlowSeq` before serializing.

### Added — Multi-line error snippets

- **`Error::format_with_source_radius(source, radius)`** —
  rustc-style error rendering with `radius` lines of context
  above and below the offending line. Output uses a fixed-width
  gutter (line numbers right-aligned to the widest), a `|` rule,
  and a caret line under the offending column. Falls back to
  plain `Display` when the error has no location or the location
  is past EOF.
- The original [`crate::Error::format_with_source`] is preserved
  byte-for-byte; the radius variant is purely additive.

### Added — Spec compliance

- **Native YAML 1.2 scanner and parser**, written entirely in safe
  Rust — `#![forbid(unsafe_code)]` at the crate root.
- **100% YAML Test Suite strict compliance**: 387/387 attempted
  variant assertions pass, 0 failures, 19 deliberate skips out
  of 406 total. The skip list is tracked alongside the harness in
  `tests/yaml_compliance_report.rs` so the gap is explicit and
  audit-friendly; each new correctness fix lands with the
  corresponding suite case unblocked.
- Full serde `Serialize` and `Deserialize` support including
  `#[serde(flatten)]`, `#[serde(default)]`, `#[serde(rename)]`,
  enum representations (externally-tagged, internally-tagged,
  adjacently-tagged, untagged).
- **Multi-document streams**: `load_all`, `load_all_as`,
  `to_string_multi`, `to_writer_multi`, `from_str_multi` (under
  the `compat-serde-yaml` feature) — `---` / `...` separators
  honoured, byte-faithful concatenation when paired with the
  CST.
- **YAML 1.1 compatibility** via `ParserConfig::legacy_booleans`:
  resolves `yes`/`no`/`on`/`off`/`y`/`n` as booleans (the
  "Norway problem" — opt-in, never silent).
- **Strict-mode hardening**: `ParserConfig::strict_booleans`,
  depth limits, document-length cap, alias-expansion cap,
  duplicate-key policy, recursion-depth probe.

### Added — Frictionless migration from `serde_yaml`

- **Comment-aware reads** (`load_comments`, `Comment`,
  `CommentKind`) — extract leading / trailing / standalone
  comments without touching the typed `Value` path.
- **`noyafmt` CLI**: lossless YAML formatter that round-trips
  through the CST, normalising whitespace and quoting without
  changing semantics.
- **`noyalib-mcp`**: Model Context Protocol server exposing
  `parse`, `format`, `get`, `set`, `validate` tools — drop-in
  for any LLM agent that needs YAML manipulation.
- **WASM playground** (`noyalib-wasm`): 201 KB
  `wasm32-unknown-unknown` build with browser demo.

#### Added — `serde_yaml` compat shim

- **`compat-serde-yaml` feature**: drop-in surface for the
  unmaintained `serde_yaml` 0.9 crate.
- Type-level parity with `serde_yaml::Value`,
  `serde_yaml::Mapping`, `serde_yaml::Number` via `From` /
  `TryFrom` conversions both directions, with
  `SerdeYamlConversionError { NonStringKey, UnrepresentableNumber }`
  for the lossy edges.
- `noyalib::compat::serde_yaml::Error` re-export wrapping
  `noyalib::Error` with location parity.
- **`Document::validate`**: non-panicking sibling of `ensure_cache`
  for callers that want to surface invalid-source errors as
  `Result` rather than via lazy panic.

#### Added — `!!binary` first-class support

- **`!!binary` tag** with RFC 4648 base64 codec
  (`src/base64.rs`, hand-rolled, whitespace-tolerant decoder).
- `serde_bytes::Bytes` / `ByteBuf` round-trip including
  multi-line block-scalar form, inline form, quoted form, and
  the full 0..=255 byte range.
- `Value::Tagged` carries `Tag::new("!!binary")` for callers
  that walk the typed tree.

#### Added — `Spanned<Value>` flatten guard

- Bare `Value` as the target of `#[serde(flatten)]` collects
  unmatched keys into a `Value::Mapping` exactly as
  `serde_yaml` / `serde_json` users expect.
- `Spanned<Value>` in a `#[serde(flatten)]` position now errors
  with a clear, actionable message pointing at the working
  alternative (bare `Value` + `Document::span_at`) instead of
  the bare `missing_field` gibberish that resulted from serde's
  `FlatStructAccess` filtering.

### Added — Lossless editing API

- **Side-table CST** (`noyalib::cst`) for byte-faithful
  round-tripping: `parse_document(s)?.to_string() == s` for any
  input the parser accepts.
- `Document::source`, `Document::span_at`, `Document::get`,
  `Document::comments_at`, `Document::syntax`,
  `Document::as_value` for read access by path.
- `Document::set`, `Document::set_value`, `Document::remove`,
  `Document::push_back`, `Document::insert_after`,
  `Document::replace_span` for mutation — every edit is
  byte-faithful outside the spliced region; comments, blank
  lines, and sibling formatting survive verbatim.
- **Incremental repair**: localised `replace_span` re-parses the
  smallest enclosing block; Document-scope re-parse only on
  shape inversion.
- **Lazy `Value` / `SpanTree`**: typed cache invalidated rather
  than re-parsed eagerly — successive edits in a batch don't
  pay the parser cost; the deferred parse runs once on the
  first read (~6× single edit).
- **Green-tree path resolution**: walks the structural CST
  directly, skipping the typed cache for the common
  set-then-set pattern (~7.6× batch).
- **Relative-len leaves**: O(log N) splice — the green node only
  stores child lengths, not absolute byte ranges (~37× over
  baseline).

#### Added — `Entry` API

- **`Document::entry(path) -> Entry<'_>`** path-shaped mutable
  handle, complementing the functional `set` / `remove` /
  `push_back` / `insert_after` methods (both stay first-class).
- 12 methods on `Entry`: `path`, `exists`, `get`, `span_at`,
  `comments`, `set`, `set_value`, `remove`, `insert`,
  `insert_value`, `push_back`, `insert_after`, plus chained
  drill-down via `Entry::entry(child)` with smart path
  composition (`items[0]` not `items.[0]`).
- New primitive `Document::insert_entry` — mapping-side
  analogue of `push_back` for sequences.

#### Added — automatic indent detection

- **`Document::indent_unit()`**: detects 2- / 3- / 4-space block
  indents from non-empty/non-comment line deltas; defaults to 2
  when undetectable. Tab-indented lines short-circuit.
- `Entry::insert_value` and `Document::insert_entry` plumb the
  detected unit into the serializer so inserts conform to the
  surrounding file's convention.
- Bug fix bundled: `column_of_key_at` now walks back to the
  actual key line (not the value's first byte), so a sibling
  insert under a parent whose last value is a nested block
  lands at the correct column.

#### Added — anchor management

- **`Document::anchors()`**, **`aliases()`**, **`aliases_of(name)`**:
  every `&name` / `*name` lexeme in source order with byte spans.
- **`Document::materialise_alias_at(byte_pos)`**: replace `*name`
  with the source bytes of `&name`'s scalar value, leaving the
  alias's site independent of any future edits to the anchor.
- **`Document::materialise_aliases_of(name)`**: bulk; reverse
  source-order so each splice's offsets stay valid.
- Propagation contract documented: edits to anchored values are
  visible at every alias site after the next load (because
  aliases are pointers in YAML's data model).
- Multi-line block-valued anchors return a clear "follow-up"
  error pointing at `Document::anchors()` + `replace_span()`
  for manual splicing — out of scope for v0.0.1.

### Added — Schema contracts

#### Added — JSON Schema codegen

- **`schema` Cargo feature** (off by default).
- **`pub use schemars::JsonSchema`** — derive imported via
  `noyalib`, no second crate dep for users.
- **`schema_for::<T>() -> Result<Value>`**: schema as a
  `noyalib::Value` tree.
- **`schema_for_yaml::<T>() -> Result<String>`**: schema as YAML
  text for sharing / version control.
- Honours `#[doc]` (→ `description`), `#[serde(default)]` (drops
  from `required`), `#[serde(rename)]` (renames property), and
  emits `minimum`/`maximum` for fixed-width integers.

#### Added — Schema validation and enhanced CLI

- **`validate-schema` Cargo feature** (implies `schema`).
- **`validate_against_schema(value, schema) -> Result<()>`**:
  enforce a JSON Schema 2020-12 contract against parsed YAML.
  Multiple violations aggregated with RFC 6901 JSON-pointer
  paths.
- **`validate_against_schema_str(yaml, schema_yaml)`**:
  convenience for raw text.
- **`noyavalidate -s/--schema PATH`**: validate each parsed
  document against the schema (YAML or JSON; both parse).
  Multi-doc streams prefix each failing document with
  `[document N]`.
- **`noyavalidate --fix`**: in-place lossless reformat via the
  CST formatter. Stdin + `--fix` keeps stdout clean for piping.
- **Critical guard**: `--fix` does NOT run when `--schema`
  rejects the input — otherwise a buggy file would be silently
  rewritten with the violation in place.

### Added — SIMD primitives and hot-path integration

- **`noyalib::simd` module**: pure-safe Rust multi-byte search
  primitives.
- `find_any_of(haystack, needles) -> Option<usize>` — dispatches
  to `memchr` for arity 1/2/3, SWAR (8-byte-stride packed
  membership lookup) for arity 4+.
- `clean_prefix_len(haystack, needles)` — length of the leading
  no-needle run; the "skip-clean-prefix" call shape.
- `ByteBitmap` + `bitmap_for(needles)` + `find_byte_in_bitmap` —
  256-bit bitmap surface for callers amortising bitmap
  construction across many calls with the same needle set.
- **Hot-path integration**: the plain-scalar inner loop in
  `fetch_plain_scalar` skips ahead via `clean_prefix_len`
  before applying the state-dependent boundary rules.
  Equivalence-tested against the byte-by-byte baseline; YAML
  1.2 official suite stays at 100% with the integration on.
- **Throughput** (Apple M1, criterion --quick, 64 KiB sparse
  haystack): arity-3 memchr 29 GiB/s vs scalar 509 MiB/s
  (~58×); arity-8 SWAR 1.45 GiB/s vs scalar 270 MiB/s (~5.4×).
- **`unsafe_code = "forbid"` invariant preserved** — no
  `core::arch::*` intrinsics, no platform-specific deps.

### Performance

Benchmarked on Apple M4, Rust 1.94 stable:

| Benchmark | noyalib | serde\_yaml\_ng | Improvement |
|---|---|---|---|
| Serialize (simple) | 358 ns | 1.41 us | **75% faster** |
| Serialize (nested) | 2.80 us | 8.32 us | **66% faster** |
| Deserialize (simple) | 1.39 us | 2.79 us | **50% faster** |
| Deserialize (nested) | 9.16 us | 17.3 us | **47% faster** |
| Deserialize (large) | 0.83 ms | 1.49 ms | **44% faster** |

CST-only metrics (Apple M1, criterion --quick, batch of 500
single-key edits):

| Optimisation | Speedup |
|---|---|
| Incremental repair | baseline |
| Lazy `Value`/`SpanTree` | ~6× single edit |
| Green-tree path resolution | ~7.6× batch |
| Relative-len leaves | ~37× over baseline |

### Added — API surface (foundation)

- `Value`, `Mapping`, `MappingAny`, `Sequence`, `Number`, `Tag`,
  `TaggedValue` types.
- `from_str`, `from_slice`, `from_reader`, `from_value`
  deserialization functions.
- `to_string`, `to_writer`, `to_fmt_writer`, `to_value`
  serialization functions.
- All functions available with `_with_config` variants for
  custom security / formatting limits.
- `SerializerConfig` with indent, flow style, scalar style,
  block scalars, document markers, `quote_all`,
  `compact_list_indent`, `folded_wrap_chars`, `min_fold_chars`.
- `ParserConfig` with depth limits, document-length limits,
  alias-expansion caps, duplicate-key policy,
  `strict_booleans`, `legacy_booleans`.
- **`Streaming` deserializer** (`StreamingDeserializer`):
  bypasses the `Value` AST for typed deserialization (50%
  faster than the Value-based path).
- **`BorrowedValue<'a>`**: zero-copy AST that borrows strings
  from input — 18% faster than the owned `Value`.
- **Path queries**: `value.query("items[*].name")` with
  wildcards (`*`) and recursive descent (`..`).
- **`Spanned<T>`** for tracking source line, column, and byte
  offset of deserialized values.
- **`apply_merge()`** for YAML merge key (`<<`) expansion.
- **`Path` type** for structured error location tracking.
- **Anchor & alias support**: `RcAnchor`, `ArcAnchor`,
  `RcWeakAnchor`, `ArcWeakAnchor`, `AnchorRegistry`,
  `ArcAnchorRegistry`.
- **`fmt` module**: `FlowSeq`, `FlowMap`, `LitStr`, `FoldStr`,
  `Commented`, `SpaceAfter`.
- **`with` module**: `singleton_map`, `singleton_map_optional`,
  `singleton_map_recursive`, `singleton_map_with`.
- **YAML 1.2 spec-schemas**: `validate_yaml_core_schema`,
  `validate_yaml_json_schema`, `validate_yaml_failsafe_schema`,
  `is_yaml_failsafe_compatible`, `is_yaml_json_compatible`.
- **`miette` diagnostic integration** (`miette` feature): rich
  terminal diagnostics with error codes, help text, source
  spans.
- **`garde` / `validator` integration** (`garde` / `validator`
  features): declarative post-deserialise validation via
  `Validated<T>` / `ValidatedValidator<T>`.
- **`#[non_exhaustive]`** on `ParserConfig`, `SerializerConfig`,
  `FlowStyle`, `ScalarStyle`.
- **`#[must_use]`** on 83 query methods.

### Added — Tooling & CLIs

- **`noyavalidate`**: validate YAML syntax (and optional JSON
  Schema) with rich `miette` diagnostics; supports `--schema
  PATH` (enforces a JSON Schema 2020-12 contract) and `--fix`
  (in-place lossless reformat through the CST).
- **`noyafmt`**: lossless CST-driven formatter.
- **`noyalib-mcp`**: Model Context Protocol server (separate
  workspace member).
- **`noyalib-wasm`**: WASM bindings + browser playground
  (separate workspace member).

### Added — Examples

- **60 branded examples** under `crates/noyalib/examples/`, each
  with the animated spinner UI from `examples/support.rs`.
- Categorised into Core, Spec, Logic & Security, DX, Advanced,
  Future-Proof, Deep Rust, Final, Platform, and Competitive
  Features.

### Added — Testing

- **2,200+ tests** including YAML spec compliance,
  property-based tests (`proptest`), competitor parity tests
  (`yaml-rust2`, `serde-saphyr`, `yaml_lib`, `rust-yaml`,
  `serde_yaml_ng`), and edge cases.
- **9 fuzz targets** (`cargo fuzz`) — five generic
  (`fuzz_parse`, `fuzz_roundtrip`, `fuzz_from_value`,
  `fuzz_multi_doc`, `fuzz_strict`) plus four targeted regression
  fuzzers (`fuzz_borrowed_alias`, `fuzz_diff`,
  `fuzz_double_quoted`, `fuzz_yaml_v1_1`). Seed corpus committed
  under `fuzz/corpus/seed/`.
- **Differential fuzz smoke** in CI (10 s per push).
- **Soak fuzz** (weekly, 1 hour per target) under
  `.github/workflows/security.yml`.
- **YAML 1.2 official suite vendored** under
  `tests/yaml-test-suite/` (MIT, upstream).
- **Cross-platform CI**: Linux, macOS, Windows × stable,
  1.75.0 (MSRV), nightly. Nightly is `continue-on-error`.

### Added — Supply chain & governance

- **`#![forbid(unsafe_code)]`** at the crate root.
- **`unreachable_pub = "forbid"`**, `non_ascii_idents = "forbid"`,
  full `clippy::all + pedantic + cargo + nursery` policy.
- **MSRV pinned at 1.75.0** with a dedicated CI job.
- **`cargo-deny`** licenses + advisories + bans + sources.
- **`cargo-vet`** with the Mozilla, Google, Bytecode Alliance,
  Embark, ISRG audit imports plus a bootstrap exemption list.
- **`cargo-semver-checks`** on every PR (gated against
  pre-publication state until the first crates.io release).
- **OpenSSF Scorecard** badge.
- **CodeQL** static analysis.
- **REUSE.software 3.3 compliance** — every file has SPDX
  copyright + license headers, blanket `REUSE.toml`
  annotations cover meta / CI / docs / fixtures.
- **SLSA L3 provenance** + **sigstore** signing in the
  release workflow.
- **SHA256 / SHA512 checksums** + **SBOM** generated per
  release.
- **`Assisted-by:` trailer** auto-injected on every commit per
  the Linux kernel coding-assistants standard.
- **Signed commits** (SSH ed25519) verified by CI.

### Added — `no_std` posture

- Full `#![no_std]` support: `default-features = false` keeps
  the `alloc`-only build working. Core parsing / serialization
  (`from_str`, `to_string`, `Value`, schemas) and the streaming
  deserializer all run without `std`.
- I/O functions (`from_reader`, `to_writer`),
  `Spanned<T>` deserialization (thread-local storage), the
  `cst` module, and the `noyavalidate` / `noyafmt` CLIs require
  the `std` feature.
- **CI enforces `cargo check --no-default-features` on every
  push.**

### Added — Cargo feature matrix

The full `[features]` block of `crates/noyalib/Cargo.toml`. Three
default-on optional features (`fast-int`, `fast-float`,
`strict-deserialise`) opt out via `default-features = false`; all
other optional features opt in.

| Feature | Default | Pulls in |
|---|---|---|
| `std` | yes | (none — gates std-only items) |
| `fast-int` | yes | `itoa` 1 (branchless integer formatting) |
| `fast-float` | yes | `ryu` 1 (branchless float formatting) |
| `strict-deserialise` | yes | `serde_ignored` 0.1 (`from_*_strict`) |
| `minimal` | no | meta-alias for `std` only — drops the three above |
| `miette` | no | `miette` 7 rich diagnostics |
| `garde` | no | `garde` 0.22 derive-based validation |
| `validator` | no | `validator` 0.19 derive-based validation |
| `compat-serde-yaml` | no | name-for-name shim (no upstream dep) |
| `schema` | no | `schemars` 1.2 + `serde_json` (codegen) |
| `validate-schema` | no | implies `schema` + `jsonschema` 0.33 |
| `figment` | no | `figment` 0.10 `Yaml` Provider |
| `parallel` | no | `rayon` 1.10 (`parallel::parse` / `values` / `split`) |
| `simd` | no | `noyalib::simd::*` primitives + parser hot path |
| `nightly-simd` | no | nightly rustc — 32-byte `StructuralIter` (implies `simd`) |
| `compare-saphyr` | no | dev-only — `serde-saphyr` for cross-library benches |
| `robotics` | no | `Degrees` / `Radians` / `StrictFloat` newtypes |
| `noyavalidate` | no | binary feature: `std` + `miette` + `validate-schema` |
| `wasm-opt` | no | size-tuned WASM build profile |

[Unreleased]: https://github.com/sebastienrousseau/noyalib/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/sebastienrousseau/noyalib/releases/tag/v0.0.1
