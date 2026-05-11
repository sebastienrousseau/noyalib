<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.5 — Release Notes

The **Polish & Stabilization** cut. Lands all four open issues
from the v0.0.5 milestone: Edition 2024 + MSRV 1.85 (#15),
declarative config-builder macros (#17), pluggable error-message
formatters (#18), and the pre-release API stabilisation audit
(#19).

## Highlights

* **Edition 2024 + MSRV 1.85.** Whole workspace moves to the new
  edition; the indexmap / hashbrown / rustc-hash lockfile pins
  that were keeping MSRV-1.75 alive are dropped.
* **Declarative config macros.** `parser_config! { … }` and
  `serializer_config! { … }` expand to the existing chained
  builders — zero runtime overhead, terser call sites.
* **`MessageFormatter` trait.** New `noyalib::i18n` module with
  bundled `DefaultFormatter` (developer-facing) +
  `UserFormatter` (plain-language). Custom localisation tables
  plug in by impl-ing the trait.
* **API stabilisation checkpoint.** All public configuration
  types carry `#[non_exhaustive]`, every public function has
  doc-comments with examples (strict-doc gate enforces it),
  `Error` enum's variant set is comprehensive and actionable.

## What ships

### Edition 2024 + MSRV 1.85 (issue #15)

All six workspace crates move to `edition = "2024"` /
`rust-version = "1.85.0"`. CI's MSRV gate retargeted in the same
cut.

Edition-2024 idiom fixes applied:

* `streaming.rs` — `ref mut anchor` / `ref value` bindings
  inferred under the new match ergonomics.
* `error.rs::format_with_source` — `repeat(' ').take(n)` →
  `repeat_n(' ', n)`.
* `roundtrip_edge_cases.rs` — `Value::Mapping(ref m)` inside a
  `matches!` pattern no longer requires the explicit `ref`.
* `examples/figment.rs` — env-overlay refactored to use
  `figment::Env::raw` + `Serialized` instead of `std::env::set_var`
  (which edition 2024 marks `unsafe`).

Lockfile pins dropped: `indexmap 2.10 → 2.14`, `rustc-hash 2.0 →
2.1.2`, `hashbrown 0.15 → 0.17`.

### Declarative `parser_config!` / `serializer_config!` (issue #17)

```rust
use noyalib::parser_config;
let cfg = parser_config! {
    max_depth: 64,
    strict_booleans: true,
};
```

Pure expansion to the existing chained-setter builders. Empty
form (`parser_config! {}`) returns the `new()` baseline.
Trailing comma after the last entry is permitted. `serializer_config!`
targets `SerializerConfig`.

### Error-message formatters (issue #18)

New `noyalib::i18n` module:

| API | Behaviour |
| :--- | :--- |
| `MessageFormatter` trait | `Send + Sync` strategy for rendering `Error` as a user-visible message |
| `DefaultFormatter` | Preserves the developer-facing message verbatim (`Display`-equivalent) |
| `UserFormatter` | Short plain-language sentences ("The configuration file has a syntax error on line 5.") |
| `Error::render_with_formatter(&dyn MessageFormatter)` | Dispatch entry point |

`UserFormatter` collapses noyalib's diagnostic vocabulary —
`!!binary`, "merge key", "alias expansion limit" — into
audience-appropriate templates. Line numbers are included when
the source location is available; field names are stripped so
sensitive data doesn't leak into GUI alerts.

### API audit (issue #19)

Pre-1.0 stabilisation checkpoint. Confirmed:

* Every public configuration type carries `#[non_exhaustive]`.
* Every public function has doc-comments with examples
  (strict-doc gate enforces it on every PR).
* The `Error` enum's variant set is comprehensive — 14 active
  variants cover every internal failure path.
* No unintended public API surface — every `pub` item is either
  re-exported from the crate root or lives in a documented
  `pub mod`.

Stable 1.0.0 is deferred to post-production hardening (target:
2028+). v0.0.5 is the stabilisation *checkpoint*.

## Compatibility

* **Public API:** additive only (excepting the MSRV bump).
* **MSRV:** Rust **1.85.0** stable (was 1.75.0). This is the
  one breaking change in v0.0.5 — downstream consumers on
  Rust < 1.85 will need to stay on noyalib 0.0.4.
* **Wire format:** unchanged.

## Migration from v0.0.4

```toml
[dependencies]
noyalib = "0.0.5"
```

If your toolchain is on Rust ≥ 1.85, no other changes needed.
The new `parser_config!` / `serializer_config!` macros are
opt-in — existing chained-builder code keeps working unchanged.
`Error::render_with_formatter` is opt-in — existing `Display` /
`format_with_source` paths keep working unchanged.

## Headline numbers

Unchanged shape from v0.0.4: 100% strict YAML Test Suite
(406/406), zero `unsafe`, 4 000+ workspace tests +
30 new tests for the v0.0.5 APIs (9 macro tests + 12 i18n
formatter tests + the migration's idiom-fix coverage).

## Verification

```bash
cargo install noya-cli --version 0.0.5
noyafmt --version
noyavalidate --version

# Cosign-verify any release artefact
cosign verify-blob \
  --certificate "noyalib-0.0.5.crate.pem" \
  --signature   "noyalib-0.0.5.crate.sig" \
  --certificate-identity-regexp \
    "^https://github.com/sebastienrousseau/noyalib/" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  noyalib-0.0.5.crate
```
