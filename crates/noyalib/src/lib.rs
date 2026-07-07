// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! # noyalib
//!
//! A YAML 1.2 library for Rust. Pure safe code. Full serde integration.
//!
//! ## Two APIs, one parser
//!
//! noyalib exposes two complementary surfaces over the same scanner
//! and strictness rules. Pick the one that matches your job:
//!
//! - **Data binding** — [`from_str`], [`to_string`], [`Value`],
//!   [`StreamingDeserializer`], [`borrowed::BorrowedValue`]. Read
//!   YAML into typed Rust data, write Rust data back to YAML. The
//!   round-trip travels through a `Value`/struct, so comments,
//!   blank lines, and the original whitespace are not preserved.
//!   Use this for config loaders, RPC payloads, and the 95% of YAML
//!   workloads that just want data.
//!
//! - **Tooling / automation** — [`cst::parse_document`],
//!   [`cst::parse_stream`], [`cst::Document`]. Read YAML into a
//!   side-table CST that reproduces the source byte-for-byte,
//!   targeted edits via `doc.set("path", "fragment")` rewrite only
//!   the touched span — comments, formatting, and sibling entries
//!   are left untouched. Use this when *what the user wrote* matters
//!   (Renovate-style version bumps, Kubernetes manifest patchers,
//!   formatters, schema-driven linters). See `examples/lossless_edit.rs`.
//!
//! ## Quick Start
//!
//! ```rust
//! use noyalib::{from_str, to_string};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Config {
//!     name: String,
//!     port: u16,
//!     features: Vec<String>,
//! }
//!
//! let yaml = "name: myapp\nport: 8080\nfeatures:\n  - auth\n  - api\n";
//! let config: Config = from_str(yaml).unwrap();
//! assert_eq!(config.name, "myapp");
//! assert_eq!(config.port, 8080);
//!
//! let output = to_string(&config).unwrap();
//! let roundtrip: Config = from_str(&output).unwrap();
//! assert_eq!(config, roundtrip);
//! ```
//!
//! ## Deserialization
//!
//! ```rust,no_run
//! # use noyalib::Value;
//! # let yaml = "key: value";
//! # let bytes = b"key: value";
//! # let file = std::io::Cursor::new(yaml);
//! # let value = Value::Null;
//! // From string, byte slice, reader, or Value
//! let v: Value = noyalib::from_str(yaml).unwrap();
//! let v: Value = noyalib::from_slice(bytes).unwrap();
//! let v: Value = noyalib::from_reader(file).unwrap();
//! let v: Value = noyalib::from_value(&value).unwrap();
//!
//! // With security limits
//! let config = noyalib::ParserConfig::strict();
//! let v: Value = noyalib::from_str_with_config(yaml, &config).unwrap();
//! ```
//!
//! ## Serialization
//!
//! ```rust,no_run
//! # use noyalib::Value;
//! # let value = Value::Null;
//! // To string, writer, or fmt::Write
//! let yaml: String = noyalib::to_string(&value).unwrap();
//! let mut buf = Vec::new();
//! noyalib::to_writer(&mut buf, &value).unwrap();
//! let mut s = String::new();
//! noyalib::to_fmt_writer(&mut s, &value).unwrap();
//!
//! // With custom config
//! let config = noyalib::SerializerConfig::new()
//!     .indent(4)
//!     .quote_all(true);
//! let yaml = noyalib::to_string_with_config(&value, &config).unwrap();
//! ```
//!
//! ## Highlights
//!
//! - **Pure Rust** — native YAML 1.2 scanner and parser. No C bindings. No FFI.
//! - **Zero `unsafe`** — `#![forbid(unsafe_code)]` enforced at compile time.
//! - **Fast** — 75% faster serialization, 50% faster deserialization than
//!   serde\_yaml\_ng. Streaming deserializer bypasses the Value AST.
//! - **Serde-native** — serialize and deserialize any `Serialize` /
//!   `Deserialize` type.
//! - **Ordered mappings** — [`IndexMap`](indexmap::IndexMap)-backed. Insertion
//!   order preserved.
//! - **Source spans** — [`Spanned<T>`] tracks exact line, column, and byte
//!   offset.
//! - **Hardened** — configurable depth, size, and alias limits. Billion-laughs
//!   safe.
//! - **100% YAML Test Suite** — 406/406 official test cases pass.
//! - **Zero-copy** — [`borrowed::BorrowedValue`] borrows strings from input.
//! - **Path queries** — `value.query("items[*].name")` with wildcards.
//! - **`no_std`** — works with `alloc` only (`default-features = false`).
//! - **`miette`** — optional rich terminal diagnostics (`--features miette`).
//!
//! ## API stability and SemVer policy
//!
//! noyalib follows [Semantic Versioning 2.0.0]. Pre-`1.0`, the
//! version axis used for breaking changes is the **patch number**
//! during the `0.0.x` series and the **minor number** during the
//! `0.x.y` series — patch bumps within a stable line are
//! source-compatible.
//!
//! - **Public surface** = items reachable from the crate root by an
//!   in-scope `pub use` (this file). Items reachable only via a
//!   `pub` module (e.g. helpers in [`borrowed`], [`cst`],
//!   [`policy`]) are also public; everything in a `pub(crate)` /
//!   private module is internal.
//! - **`#[non_exhaustive]`** is applied to every public
//!   configuration struct ([`ParserConfig`], [`SerializerConfig`],
//!   [`Error`], [`MergeKeyPolicy`], [`DuplicateKeyPolicy`],
//!   [`FlowStyle`], [`ScalarStyle`], [`YamlVersion`]) so adding a
//!   field or variant in a future release is **not** a breaking
//!   change. Construct configuration via the documented
//!   `new` / `default` / `strict` constructors plus the builder
//!   setters; do not use exhaustive struct-literal syntax outside
//!   this crate.
//! - **What we will not break in patch releases:**
//!   - public function signatures (parameter names, types, return
//!     types);
//!   - the [`Value`] enum's variant set;
//!   - re-exported macro names (none today);
//!   - the YAML 1.2 default-strictness contract.
//! - **The deserialise-target bound is `T: for<'de>
//!   Deserialize<'de> + 'static`.** The `'static` half is the
//!   contract every real-world `DeserializeOwned` type already
//!   satisfies (the HRTB itself disallows borrowed lifetimes); it
//!   is documented explicitly because a small number of
//!   externally-defined trait signatures (e.g. `figment`'s
//!   `Format::from_str`) drop the `'static` from their own bound
//!   — for those, noyalib provides feature-gated internal entry
//!   points that bypass the [`Value`]-tag-preserving fast path.
//! - **What may change without a major bump:** non-default
//!   `ParserConfig` semantics under explicit opt-in (e.g. a future
//!   `legacy_*` flag), error *message* wording (variant *names*
//!   are stable), benchmark numbers, internal module layout.
//! - **Deprecations** ship with `#[deprecated(since = "x.y.z",
//!   note = "...")]` for at least one minor release before
//!   removal. CHANGELOG carries the migration recipe.
//! - **API drift checks**: `cargo semver-checks` runs in CI on
//!   every PR.
//!
//! [Semantic Versioning 2.0.0]: https://semver.org/spec/v2.0.0.html
//!
//! ## MSRV policy
//!
//! - **Core library (`noyalib`)** — Rust **1.85.0** stable
//!   (edition 2024, raised in v0.0.5). CI's MSRV gate builds
//!   the `default-features = false` and the standard `default`
//!   set on `rustc 1.85.0` for every PR. The MSRV is treated as
//!   part of the public contract: a bump within `0.0.x` is a
//!   breaking change and ships a major version.
//! - **Optional features** that pull a dep with a higher floor
//!   (`miette`, `garde`, `validate-schema`, `figment`,
//!   `parallel`, `validator`, `tokio`) inherit that dep's MSRV
//!   — typically `1.85`–`1.86` depending on the feature. The
//!   CI matrix runs each one against the dep's declared
//!   `rust-version`.
//! - **Companion crates** ([`noya-cli`], [`noyalib-lsp`]) carry
//!   the same `1.85.0` floor.
//! - **`nightly-simd`** is the only feature that requires nightly
//!   rustc (`#![feature(portable_simd)]`); a `build.rs` cfg-detect
//!   probe means stable builds with `--all-features` still
//!   compile by treating `nightly-simd` as a no-op.
//!
//! [`noya-cli`]: https://crates.io/crates/noya-cli
//! [`noyalib-lsp`]: https://crates.io/crates/noyalib-lsp
//!
//! ## Feature flag matrix
//!
//! All optional integrations are off by default — enable only
//! what your application needs. Default-on flags can be opted out
//! via `default-features = false`.
//!
//! | Feature | Default | Pulls in | Adds | Implies |
//! | :--- | :---: | :--- | :--- | :--- |
//! | `std` | ✅ | — | I/O, [`Spanned<T>`] deserialise, [`cst`] | — |
//! | `fast-int` | ✅ | `itoa` | branchless integer formatting | `std` recommended |
//! | `fast-float` | ✅ | `ryu` | branchless float formatting | `std` recommended |
//! | `strict-deserialise` | ✅ | `serde_ignored` | `from_*_strict` family | `std` |
//! | `minimal` | ⛔ | — | meta-alias for `std` only (drops the three above) | `std` |
//! | `miette` | ⛔ | `miette 7` | rich terminal diagnostics | — |
//! | `schema` | ⛔ | `schemars`, `serde_json` | [`schema_for`] / [`schema_for_yaml`] **+** consumer must also depend on `schemars = "1.2"` to derive [`JsonSchema`] | — |
//! | `validate-schema` | ⛔ | `schema` + `jsonschema` | [`validate_against_schema`], [`coerce_to_schema`] | `schema` |
//! | `figment` | ⛔ | `figment 0.10` | [`figment::Yaml`](crate::figment) Provider | `std` |
//! | `garde` | ⛔ | `garde 0.22` | [`Validated<T>`] | — |
//! | `validator` | ⛔ | `validator 0.19` | [`ValidatedValidator<T>`] | — |
//! | `robotics` | ⛔ | — | `Degrees` / `Radians` / `StrictFloat` newtypes | — |
//! | `parallel` | ⛔ | `rayon 1.10` | [`parallel::parse`], [`parallel::values`] | `std` |
//! | `simd` | ⛔ | — | `noyalib::simd::*` primitives | — |
//! | `nightly-simd` | ⛔ | nightly rustc | 32-byte `StructuralIter` | `simd` |
//! | `compat-serde-yaml` | ⛔ | — | `noyalib::compat::serde_yaml` shim | — |
//! | `compare-saphyr` | ⛔ | `serde-saphyr` | comparison-bench arms (dev-only — do **not** ship in release builds) | — |
//! | `noyavalidate` | ⛔ | — | meta-feature: `validate-schema` + `miette` | `validate-schema`, `miette` |
//! | `wasm-opt` | ⛔ | — | marker consumed by `noyalib-wasm`'s build.rs to opt into a Binaryen post-build pass | — |
//!
//! `docs.rs` builds with `--all-features`; every gated item is
//! tagged with the feature it requires via the `doc(cfg(...))`
//! badge.
//!
//! ## Concurrency guarantees
//!
//! - All public top-level functions ([`from_str`], [`from_slice`],
//!   [`from_reader`], [`to_string`], [`to_writer`], …) are pure
//!   over their inputs and may be called concurrently from any
//!   number of threads.
//! - [`Value`], [`Mapping`], [`Number`], [`Spanned<T>`],
//!   [`Error`] are `Send + Sync`. Cloning a `Value` is `O(n)` in
//!   the value graph; share ownership via
//!   [`Arc`](std::sync::Arc)`<Value>` when that cost matters.
//! - [`policy::Policy`] requires `Send + Sync` so policies can be
//!   shared by reference across threads. Stateful policies should
//!   hold their state behind interior mutability
//!   ([`std::sync::Mutex`] or equivalent).
//! - [`Spanned<T>`] deserialisation uses a thread-local span
//!   context (`std` feature). The TLS guard installs on entry to
//!   [`from_str_with_config`] and clears on return — no leakage
//!   across calls or across threads.
//! - Anchor and alias state lives in the parser stack frame (one
//!   per call); concurrent calls share no mutable state.
//! - The Rayon-backed [`parallel`] module pre-scans document
//!   boundaries on the calling thread, then dispatches each
//!   document to the global Rayon pool — `T: Send` is required.
//! - [`anchors::ArcAnchorRegistry`] / [`anchors::ArcAnchor`] use
//!   `Arc` + `Weak` and are explicitly multi-thread-safe; the
//!   `Rc`-backed siblings are single-thread.
//!
//! ## Security posture
//!
//! - **No `unsafe`** — `#![forbid(unsafe_code)]` enforced at
//!   compile time on every workspace crate.
//! - **No FFI** — pure Rust scanner / parser / serialiser /
//!   CST. Closes the historical `libyaml` C-FFI CVE class.
//! - **No arbitrary object instantiation from tags** — custom
//!   tags surface as [`Value::Tagged`] data; opt-in dispatch via
//!   [`TagRegistry`]. There is no path from a parsed YAML
//!   document to running attacker-chosen code.
//! - **Resource budgets** — seven configurable limits in
//!   [`ParserConfig`] cap depth, document size, alias
//!   expansions, mapping keys, sequence length, duplicate-key
//!   policy, and boolean strictness. [`ParserConfig::strict`]
//!   tightens every budget for untrusted input. Alias-byte
//!   accumulation uses `saturating_add` so a crafted overflow
//!   still trips the cap.
//! - **Pluggable policies** — [`policy::DenyAnchors`],
//!   [`policy::DenyTags`], [`policy::MaxScalarLength`] for
//!   organisational "Safe YAML" enforcement. Custom policies
//!   implement [`policy::Policy`].
//! - **Supply chain** — `cargo audit`, `cargo deny`, `cargo vet`
//!   gate every PR. Releases ship SLSA L3 provenance and
//!   sigstore signatures (verification cookbook in
//!   [`pkg/VERIFY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/pkg/VERIFY.md)).
//!   No archived or unmaintained crate appears in the dependency
//!   graph.
//!
//! Disclosure policy: see
//! [`SECURITY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/SECURITY.md).
//!
//! ## Performance and complexity
//!
//! - **Parser** — single-pass, `O(n)` in input bytes for the
//!   scanner; loader is `O(n)` events. `IndexMap` insert is
//!   amortised `O(1)`; `FxHasher` keeps key hashing cheap on
//!   short keys.
//! - **Streaming deserialise** — bypasses the dynamic `Value`
//!   AST when the caller asks for a typed `T`, eliminating
//!   intermediate allocations. ~30% faster than the
//!   AST-via-`Value` path on real workloads.
//! - **Zero-copy scanner** — string scalars come out as
//!   `Cow::Borrowed` when no escape sequence forces an
//!   allocation. [`borrowed::BorrowedValue`] surfaces this all
//!   the way to the caller.
//! - **SIMD primitives** — [`simd::find_any_of`] dispatches to
//!   `memchr` SSE2/NEON for arity 1/2/3 and SWAR for arity 4+.
//!   With `nightly-simd`, the structural-bitmask scanner widens
//!   to 32-byte lanes — ~9× speedup vs the memchr loop on 1 MiB
//!   inputs.
//! - **SWAR decimal parser** — folds 8 ASCII digits per `u64`
//!   cycle. ~2× faster than `<i64 as FromStr>::from_str` on big
//!   numbers.
//! - **Serialiser** — branchless integer (`itoa`) and float
//!   (`ryu`) formatting in the hot path; falls back to
//!   `core::fmt` under `--no-default-features`.
//! - **Parallel multi-document** — [`parallel::parse`] scales
//!   near-linearly with cores on `---`-separated streams; the
//!   pre-scan is `O(input.len())` on the calling thread.
//! - **`Value::clone`** is `O(n)` over the value graph; share
//!   via `Arc<Value>` when that matters.
//!
//! ## Platform support
//!
//! - **Tier 1**: `x86_64-unknown-linux-gnu`,
//!   `x86_64-apple-darwin`, `aarch64-apple-darwin`,
//!   `x86_64-pc-windows-msvc`, `aarch64-unknown-linux-gnu`. CI
//!   runs on each of these on every PR.
//! - **Tier 2**: musl Linux (`*-musl`),
//!   `i686-pc-windows-msvc`, `aarch64-pc-windows-msvc`. Built
//!   in release CI; not gated on every PR.
//! - **Embedded / `no_std`**: any target supported by `alloc`.
//!   The `std`-only items ([`from_reader`], [`to_writer`],
//!   [`Spanned<T>`] deserialisation via TLS, the [`cst`]
//!   module) are gone; the rest of the surface compiles. CI
//!   enforces `cargo check --no-default-features` on every PR.
//! - **WASM**: `wasm32-unknown-unknown` via the `noyalib-wasm`
//!   companion crate. 338 KB release binary (LTO). Browser
//!   demo in `crates/noyalib/examples/wasm/`.
//! - **Big-endian**: validated under Miri's
//!   `mips64-unknown-linux-gnuabi64` simulation in the weekly
//!   `miri-bigendian` job.
//!
//! ## Error model
//!
//! Every fallible function returns [`Result<T>`](crate::Result)
//! aliasing `core::result::Result<T, Error>`. [`Error`] is
//! `#[non_exhaustive]`, implements `core::fmt::Display`,
//! `core::error::Error` (via `std::error::Error` under the
//! `std` feature), and — with `--features miette` —
//! `miette::Diagnostic` for rich terminal reports.
//!
//! Each entry-point's `# Errors` section enumerates the variant
//! set callers must handle; cross-reference the [`Error`]
//! variants for descriptions.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    all(feature = "nightly-simd", noyalib_nightly),
    allow(unstable_features),
    feature(portable_simd)
)]
// Opt-in coverage annotations. `noyalib_coverage` is set by the
// build script when `NOYALIB_COVERAGE=1` is exported (typically by
// the CI coverage job running on nightly). When active, items
// annotated with `#[cfg_attr(noyalib_coverage, coverage(off))]`
// are excluded from coverage instrumentation. Stable builds and
// regular nightly builds never see the `coverage_attribute`
// feature flag, so the annotations are no-ops there.
#![cfg_attr(noyalib_coverage, allow(unstable_features))]
#![cfg_attr(noyalib_coverage, feature(coverage_attribute))]

// README doctest coverage: every ```rust block in
// crates/noyalib/README.md is exercised by `cargo test --doc`.
// The hidden module exists only when doctesting so the README
// content does not leak into the docs.rs page (the lib's own
// crate-level docs above are the canonical surface there).
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_doctests {}

// The workspace-root README (GitHub landing page) is doctested by
// `scripts/check-readme-examples.sh` — every ```rust block there
// is extracted and compiled against `noyalib` from a scratch
// project. Cannot `include_str!("../../../README.md")` here
// because the workspace-root README lives outside the crate's
// package layout, so `cargo publish --dry-run` would fail
// verification. The script-based path preserves the invariant
// (broken root-README examples fail CI) without breaking publish.

#[cfg(not(feature = "std"))]
extern crate alloc;

/// Internal prelude for no_std compatibility.
/// Provides String, Vec, Box, etc. from alloc when std is absent.
#[cfg(not(feature = "std"))]
pub(crate) mod prelude {
    pub(crate) use alloc::borrow::{Cow, ToOwned};
    pub(crate) use alloc::boxed::Box;
    pub(crate) use alloc::format;
    pub(crate) use alloc::string::{String, ToString};
    pub(crate) use alloc::sync::Arc;
    pub(crate) use alloc::vec;
    pub(crate) use alloc::vec::Vec;
    pub(crate) use core::fmt;
}

/// Internal prelude for std compatibility.
#[cfg(feature = "std")]
pub(crate) mod prelude {
    pub(crate) use std::borrow::{Cow, ToOwned};
    pub(crate) use std::boxed::Box;
    pub(crate) use std::fmt;
    pub(crate) use std::format;
    pub(crate) use std::string::{String, ToString};
    pub(crate) use std::sync::Arc;
    pub(crate) use std::vec;
    pub(crate) use std::vec::Vec;
}

mod anchors;
/// Zero-copy YAML values that borrow from the input.
/// [`ariadne`] adapter — render `crate::Error` as an
/// `ariadne::Report` with the offending byte range labelled.
/// Behind the `ariadne` Cargo feature.
#[cfg(feature = "ariadne")]
#[cfg_attr(docsrs, doc(cfg(feature = "ariadne")))]
pub mod ariadne_adapter;

/// Internal RFC 4648 base64 codec for `!!binary` scalars.
mod base64;
/// `!include` directive — resolver types
/// (`IncludeResolver`, `IncludeRequest`, `InputSource`,
/// `SymlinkPolicy`, `SafeFileResolver`). Wired into
/// `ParserConfig::include_resolver`.
#[cfg(feature = "include")]
#[cfg_attr(docsrs, doc(cfg(feature = "include")))]
pub mod include;

/// Declarative `parser_config!` / `serializer_config!` builder
/// macros. Pure expansion to the existing chained-setter
/// builders — zero runtime overhead.
mod macros;

/// Pluggable error-message formatters: [`i18n::MessageFormatter`]
/// trait plus [`i18n::DefaultFormatter`] (developer-facing,
/// verbatim) and [`i18n::UserFormatter`] (user-facing,
/// simplified language). Use
/// [`crate::Error::render_with_formatter`] to plug in
/// localisation tables or custom rendering.
pub mod i18n;

/// `Spanned<T>` + garde / validator → `miette::Report` bridge.
/// Behind the `miette` Cargo feature; the actual conversion
/// functions are gated on `miette + garde` or `miette + validator`.
#[cfg(feature = "miette")]
#[cfg_attr(docsrs, doc(cfg(feature = "miette")))]
pub mod validated_miette;

pub mod borrowed;
mod comments;
/// Drop-in compatibility shims for upstream YAML crates. Each shim
/// is gated behind its own feature flag so unused migration paths
/// add zero compile cost. See [`compat::serde_yaml`] for the
/// `serde_yaml` 0.9 surface.
pub mod compat;
/// Side-table CST for byte-faithful round-tripping with typed
/// path-targeted edits.
///
/// See `docs/design/green-tree.md` for the architectural plan. The
/// `Document` API depends on the parser's `SpanTree`, which lives
/// under the `std` feature.
#[cfg(feature = "std")]
pub mod cst;
mod de;
/// Spanned-to-miette diagnostic bridge (requires `miette` feature).
#[cfg(feature = "miette")]
#[cfg_attr(docsrs, doc(cfg(feature = "miette")))]
pub mod diagnostic;
/// Workspace-private `---` document-boundary scanner shared by
/// `parallel::split`, `recovery::split_documents`, and
/// `tokio_async::find_doc_boundary` — one CRLF/BOM-aware scanner,
/// one DoS cap.
pub(crate) mod doc_boundary;
/// Multi-document loading and iteration.
pub mod document;
mod error;
/// [`figment`] provider integration. Pulls in `figment` 0.10
/// when the `figment` Cargo feature is enabled.
#[cfg(feature = "figment")]
#[cfg_attr(docsrs, doc(cfg(feature = "figment")))]
pub mod figment;
mod flattened;
/// Formatting wrappers for per-value YAML output style control.
pub mod fmt;
/// Key interning for memory-efficient repeated-key workloads.
pub mod interner;
/// Parallel multi-document YAML parsing via Rayon. Gated by the
/// `parallel` feature.
#[cfg(feature = "parallel")]
#[cfg_attr(docsrs, doc(cfg(feature = "parallel")))]
pub mod parallel;
mod parser;
mod path;
/// Pluggable parser policies for "Safe YAML" enforcement.
pub mod policy;
/// Error-recovering parser for LSP / IDE partial parsing. Gated
/// by the `recovery` feature.
#[cfg(feature = "recovery")]
#[cfg_attr(docsrs, doc(cfg(feature = "recovery")))]
pub mod recovery;
/// Robotics and scientific numeric types (requires `robotics` feature).
#[cfg(feature = "robotics")]
#[cfg_attr(docsrs, doc(cfg(feature = "robotics")))]
pub mod robotics;
mod schema;
/// JSON Schema codegen via [`schemars`] — derive
/// [`schemars::JsonSchema`] for a Rust type and call
/// [`schema_for`] / [`schema_for_yaml`] to obtain the schema as a
/// [`crate::Value`] or as YAML text. Requires the `schema` feature.
#[cfg(feature = "schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
mod schema_codegen;
/// Schema *validation* — enforce a JSON Schema 2020-12 contract
/// against a parsed [`Value`]. Pairs with [`schema_codegen`].
/// Requires the `validate-schema` feature (which implies `schema`).
#[cfg(feature = "validate-schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "validate-schema")))]
mod schema_validate;
mod ser;
/// `sval` zero-allocation streaming serialization adapter.
/// Gated by the `sval` feature.
#[cfg(feature = "sval")]
#[cfg_attr(docsrs, doc(cfg(feature = "sval")))]
pub mod sval_adapter;
/// Native async YAML parsing for tokio runtimes —
/// `from_async_reader` plus a `tokio_util::codec::Decoder`.
/// Gated by the `tokio` feature.
#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio_async;

/// SIMD-friendly multi-byte search primitives.
///
/// Pure-safe Rust (no `unsafe`, no platform intrinsics, no
/// hardware-specific deps). The vectorisation comes from
/// `memchr`'s SSE2 / NEON dispatch for arity 1/2/3 and SWAR
/// (SIMD-Within-A-Register) for arity 4+. The parser hot path
/// uses these primitives unconditionally; the `simd` Cargo
/// feature is retained as a no-op for forward compatibility.
pub mod simd;
pub(crate) mod span_context;
pub(crate) mod spanned;
mod streaming;
pub mod tag_registry;
/// Declarative post-deserialise validation via [`garde`] or [`validator`]
/// (requires the corresponding feature).
#[cfg(any(feature = "garde", feature = "validator"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "garde", feature = "validator"))))]
pub mod validated;
mod value;
pub mod with;

pub use anchors::{
    AnchorRegistry, ArcAnchor, ArcAnchorRegistry, ArcWeakAnchor, RcAnchor, RcWeakAnchor,
};
// Recursive anchor wrappers depend on `Rc<RefCell<…>>` /
// `Arc<Mutex<…>>` which require `std` (RefCell+Mutex live under
// `core::cell` / `std::sync` — the latter only available with std).
#[cfg(feature = "std")]
pub use anchors::{ArcRecursion, ArcRecursive, RcRecursion, RcRecursive};
pub use comments::{Comment, CommentKind, load_comments};
pub use de::RequireIndent;
pub use de::{
    Deserializer, DuplicateKeyPolicy, MergeKeyPolicy, ParserConfig, YamlVersion, from_slice,
    from_slice_with_config, from_str, from_str_borrowing, from_str_borrowing_with_config,
    from_str_with_config, from_value,
};
#[cfg(feature = "std")]
pub use de::{from_reader, from_reader_with_config};
#[cfg(all(feature = "std", feature = "strict-deserialise"))]
pub use de::{from_reader_strict, from_slice_strict, from_str_strict};
#[cfg(feature = "std")]
pub use document::{DocumentReadIterator, read, read_with_config};
pub use document::{load_all, load_all_as, load_all_with_config, try_load_all};
pub use error::{BudgetBreach, CroppedRegion, Error, ErrorKind, Location, RenderOptions, Result};
pub use flattened::Flattened;
pub use fmt::{Commented, FlowMap, FlowSeq, FoldStr, FoldString, LitStr, LitString, SpaceAfter};
pub use path::Path;
pub use schema::{
    is_yaml_failsafe_compatible, is_yaml_json_compatible, validate_yaml_core_schema,
    validate_yaml_failsafe_schema, validate_yaml_json_schema,
};
#[cfg(feature = "schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
pub use schema_codegen::{JsonSchema, schema_for, schema_for_yaml};
#[cfg(feature = "validate-schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "validate-schema")))]
pub use schema_validate::{coerce_to_schema, validate_against_schema, validate_against_schema_str};
pub use ser::{
    FlowStyle, ScalarStyle, Serializer, SerializerConfig, to_fmt_writer, to_fmt_writer_with_config,
    to_string, to_string_multi, to_string_multi_with_config, to_string_value,
    to_string_value_with_config, to_string_with_config, to_value,
};
#[cfg(feature = "std")]
pub use ser::{
    to_string_tracking_shared, to_string_tracking_shared_with_config, to_writer_tracking_shared,
    to_writer_tracking_shared_with_config,
};
#[cfg(feature = "std")]
pub use ser::{
    to_writer, to_writer_multi, to_writer_multi_with_config, to_writer_value,
    to_writer_value_with_config, to_writer_with_config,
};
pub use spanned::Spanned;
pub use streaming::StreamingDeserializer;
pub use tag_registry::TagRegistry;
#[cfg(feature = "garde")]
#[cfg_attr(docsrs, doc(cfg(feature = "garde")))]
pub use validated::Validated;
#[cfg(feature = "validator")]
#[cfg_attr(docsrs, doc(cfg(feature = "validator")))]
pub use validated::ValidatedValidator;
pub use value::{
    Mapping, MappingAny, MaybeTag, Number, ParseNumberError, Sequence, Tag, TaggedValue, Value,
    ValueIndex, check_for_tag, nobang,
};
