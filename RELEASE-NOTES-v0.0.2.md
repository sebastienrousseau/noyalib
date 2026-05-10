<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.2 — Release Notes

A point release that lands the implementation half of the
**v0.0.3 milestone** (zero-copy borrowed deserialisation, lazy
multi-document reader iterator) alongside three v0.0.4 ergonomics
adapters (property interpolation, ariadne, validator → miette
bridge). Closes **16 milestone issues** in a single cut — 9
fully implemented, 7 confirmed already shipped in v0.0.1 with
file-and-line evidence comments.

## Highlights

- **Zero-copy `&'de str` deserialisation.** `from_str_borrowing`
  routes plain scalars through `visit_borrowed_str`; the parser
  fast-path is extended so the typical `key: value\n` shape
  borrows directly from the input slice. `Cow<'a, str>` works
  uniformly across borrowed / owned cases. New
  `TransformReason` enum catalogues the five reasons a scalar
  may fail to borrow.
- **Lazy multi-document `Read` iterator.** `noyalib::read`
  yields `Result<T>` per YAML document with per-document error
  recovery (deserialise errors don't halt iteration; syntax
  errors return synchronously).
- **`${KEY}` substitution during parse.**
  `ParserConfig::properties(map)` plus `${KEY:-default}`
  fallbacks, `$$` / `${{` escapes, and a `strict_properties`
  toggle.
- **`ariadne` adapter.** `noyalib::ariadne_adapter::error_to_ariadne_report`
  renders `Error` with full source-context labels in ariadne's
  terminal-friendly style. Pairs with the existing
  `miette::Diagnostic` impl.
- **garde / validator → miette bridge.**
  `validated_miette::garde_errors_to_miette` /
  `validator_errors_to_miette` walk a validation tree and emit a
  single `miette::Report` labelled at the `Spanned<T>`'s byte
  range.

## Issues closed

### Implemented in v0.0.2 (9)

| Issue | Title | API |
| :--- | :--- | :--- |
| **#7** | Streaming multi-document iterator | `read`, `read_with_config`, `DocumentReadIterator<T>` |
| **#8** | Zero-copy borrowed string deserialisation | `from_str_borrowing`, `from_str_borrowing_with_config`, `borrowed::TransformReason` |
| **#11** | `${VAR}` property interpolation | `ParserConfig::properties`, `ParserConfig::strict_properties` |
| **#23** | `ariadne` diagnostic adapter | `ariadne_adapter::error_to_ariadne_report` |
| **#32** | `Spanned<T>` + garde/validator → miette bridge | `validated_miette::garde_errors_to_miette`, `validator_errors_to_miette` |

### Confirmed already shipped in v0.0.1 (closed with evidence) (7)

| Issue | Title | Evidence |
| :--- | :--- | :--- |
| **#9** | Event-based streaming deserialisation | `from_str_streaming` fast-path inside `from_str` (30% faster vs AST) |
| **#12** | Figment config-framework integration | `figment.rs` + tests/figment_provider.rs |
| **#13** | Miette structured diagnostics integration | `diagnostic.rs` + `impl miette::Diagnostic for Error` |
| **#14** | garde / validator validation integration | `validated.rs` (`Validated<T>`, `ValidatedValidator<T>`) |
| **#16** | `#[non_exhaustive]` on public config types | Confirmed across `ParserConfig`, `SerializerConfig`, `FlowStyle`, `ScalarStyle`, `MergeKeyPolicy`, `DuplicateKeyPolicy`, `YamlVersion`, `RequireIndent`, `Error`, `TransformReason` |
| **#20** | Format-preserving CST round-trip editing | `cst/` module (anchor, annotated, builder, coerce, document, style) |
| **#21** | `#![no_std]` + alloc | `#![cfg_attr(not(feature = "std"), no_std)]` + CI `no_std (alloc-only) build` gate |
| **#27** | Path query API (`.query()`) | `Value::query` + `BorrowedValue::query` with `*` / `..` |
| **#28** | Zero-copy `Value<'a>` AST | Parallel `BorrowedValue<'a>` with `Cow<'a, str>` keys + values (18% faster) |
| **#30** | Shared-memory DAGs via Rc/Arc anchor registry | `AnchorRegistry` + `ArcAnchorRegistry` + `RcRecursive` / `ArcRecursive` |
| **#31** | Robotics / scientific numeric profile | `robotics.rs` (`Degrees` / `Radians`, strict-f64 deser, custom-tag dispatch) |

## What ships

### New `noyalib` APIs

- `from_str_borrowing` / `from_str_borrowing_with_config` —
  zero-copy deserialise into targets that borrow from the
  input slice. The streaming deserialiser routes plain-scalar
  string events through `visit_borrowed_str` whenever the
  parser produced a `Cow::Borrowed` event.
- Plain-scalar parser fast-path now also fires when the scalar
  terminates before the next newline (`scalar_terminates_on_line`)
  and the slow-path emits `Cow::Borrowed(input_slice)` whenever
  the scalar is a single contiguous run of input bytes (no
  folded line breaks). Result: the typical `key: value\n` shape
  borrows zero-copy on the streaming path.
- `borrowed::TransformReason` — `#[non_exhaustive]` public enum
  cataloguing why a scalar can fail to borrow:
  `EscapeSequence`, `LineFold`, `TagResolution`,
  `QuotedScalar`, `AliasExpansion`. `Display` and `as_str()`
  provide stable messages.
- `read` / `read_with_config` + `DocumentReadIterator<T>` —
  lazy multi-document iterator over `R: Read`. Per-document
  deserialisation errors surface as `Err` items so iteration
  continues across document boundaries; YAML syntax errors
  return synchronously.
- `ParserConfig::properties` + `ParserConfig::strict_properties`
  — `${KEY}` / `${KEY:-default}` substitution during parse,
  with `$$` / `${{` / `}}` escapes. `ParserConfig::strict()`
  defaults `strict_properties` to `true` so untrusted-input
  pipelines abort on unknown placeholders.

### New crate features

- `ariadne` — pulls `ariadne 0.5`. Exposes
  `noyalib::ariadne_adapter::error_to_ariadne_report` to
  render an `Error` as an `ariadne::Report`.
- `validated_miette` module (always-present, but the gated
  bridge functions require `miette + garde` or
  `miette + validator`).

### `noya-cli`, `noyalib-mcp`, `noyalib-lsp`, `noyalib-wasm`

All four satellite crates pick up v0.0.2 in lockstep — no
behaviour changes of their own, but they re-pin to the v0.0.2
library and benefit from the underlying improvements.

## What changed (besides the new APIs)

- **`indexmap` upper bound relaxed to `<3`** (was `<2.11`). The
  old cap was defensive; noyalib's usage covers stable
  `IndexMap`, `map::Iter`, `map::Entry` surface that hasn't
  changed across the 2.x line.
- **`Cargo.lock` pins `indexmap 2.10.0`** to keep `hashbrown
  0.15.x` resolved transitively — `hashbrown 0.17` declares
  `edition = "2024"` in its manifest, which requires Cargo
  1.85+, above noyalib's 1.75 MSRV. Downstream consumers on
  Rust ≥ 1.85 can `cargo update` to newer if desired.
- **Parser slow-path borrow fix:** trailing inline whitespace
  before a flow indicator (`,]}`) is trimmed at fast-path emit
  so the borrowed slice matches the slow-path owned-buffer
  result byte-for-byte. Fixes a latent flow-mode
  emit-with-trailing-space bug the slow path was masking.
- **`expand_placeholders` refactored** from `Result<String>`
  resolver to a tri-state `ResolveOutcome` (Found / Missing /
  Error) so the `:-default` path can distinguish missing-key
  from resolution-error without a sentinel. Existing
  `Value::interpolate_properties*` public callers unchanged.

## Headline numbers

- **YAML 1.2 spec compliance: 100% strict** — 406/406 official
  YAML Test Suite cases pass, 0 fail, 0 skip.
- **Zero `unsafe`** workspace-wide
  (`#![forbid(unsafe_code)]`).
- **No transitive C dependency** (no libyaml).
- **4 000+ workspace tests + 495+ doctests + 38 new
  integration tests** for the v0.0.2 APIs.
- **96.22% function / 94.30% line / 93.44% region** coverage
  (CI-gated). New code's coverage is higher: `borrowed.rs`
  96.37% region, `document.rs` 98.61% region.
- **270 fully audited / 0 partially audited / 5 exempted
  (bench-only competitor + transitive)** per `cargo vet`.
- **Five publishable crates** — `noyalib`, `noya-cli`,
  `noyalib-mcp`, `noyalib-lsp`, `noyalib-wasm` — all in
  lockstep at v0.0.2.

## Compatibility

- **MSRV** — Rust **1.75.0** stable for the core library,
  unchanged from v0.0.1. Optional features (`miette`, `garde`,
  `validator`, `validate-schema`, `figment`, `ariadne`) pull
  deps with higher floors (1.80–1.86) and ship with their own
  per-feature MSRV gates in CI.
- **Public API** — additive only. Every new entry point is
  behind its own function name or feature gate; no existing
  signature changed. Internal refactor of
  `interpolate_inner` keeps the public
  `Value::interpolate_properties` family signature-compatible.
- **Lockfile** — pinned `indexmap 2.10.0` to keep MSRV 1.75
  alive. Downstream consumers on newer toolchains are free to
  `cargo update -p indexmap` to take 2.11+ / 2.14+.

## Migration from v0.0.1

No breaking changes. Drop-in upgrade. To use the new APIs:

```toml
[dependencies]
noyalib = { version = "0.0.2", features = ["miette", "garde", "ariadne"] }
```

`from_str_borrowing` works on any `T: Deserialize<'a>` with a
borrowed lifetime — `&'a str`, `Cow<'a, str>`, structs
containing those. `from_str` continues to work for owned
targets without change. The streaming deserialiser
automatically prefers borrowed delivery whenever the parser
produced a borrowed scalar.

`${KEY}` interpolation is opt-in via `ParserConfig::properties`
— no behaviour change for callers who don't install a property
map.

## Acknowledgements

- The `serde-saphyr`, `yaml-rust2`, `serde_yaml_ng`,
  `serde_yml`, and `yaml-spanned` projects provided the
  reference points for the head-to-head benchmark numbers.
- The `miette` and `ariadne` projects power the new
  diagnostic adapters; the `garde` and `validator` projects
  power the validation bridge.

## Verification

```bash
# Install + check version
cargo install noya-cli --version 0.0.2
noyafmt --version
noyavalidate --version

# Cosign-verify any release artefact
cosign verify-blob \
  --certificate "noyalib-0.0.2.crate.pem" \
  --signature   "noyalib-0.0.2.crate.sig" \
  --certificate-identity-regexp \
    "^https://github.com/sebastienrousseau/noyalib/" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  noyalib-0.0.2.crate
```

Full verification recipes in [`pkg/VERIFY.md`](pkg/VERIFY.md).
