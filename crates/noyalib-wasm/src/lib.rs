// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! noyalib WASM bindings.
//!
//! Exposes YAML parse/serialize and the lossless Document API to JavaScript
//! via wasm-bindgen.
//!
//! The pure-Rust logic lives in [`core`] so it is reachable from
//! `cargo test` on the rlib side; the bindings in this module are
//! the thin JsValue conversion shells.
//!
//! # Cargo features
//!
//! - **`wasm-opt`** *(optional, off by default)* — enables a
//!   post-build pass that re-runs Binaryen's `wasm-opt -O3 -s`
//!   on the produced `.wasm` to shrink the bundle (~17% smaller
//!   on `noyalib-wasm`). Pulls in no extra Rust dependencies; it
//!   is a build-script feature only. Disable for fastest dev
//!   builds.
//!
//! Optional `noyalib` features pulled in by a downstream wasm-pack
//! consumer (`schema`, `parallel` …) compile against the
//! `wasm32-unknown-unknown` target only when their dep tree is
//! WASM-clean — `parallel` (rayon) requires
//! `--target wasm32-wasip1-threads`, not the default browser
//! target. The canonical `noyalib` feature matrix lives in
//! [`crates/noyalib/src/lib.rs`](https://docs.rs/noyalib).
//!
//! # MSRV
//!
//! **Rust 1.85.0** stable. The `wasm-bindgen` 0.2 ecosystem
//! pulls helpers floored at 1.85; the core `noyalib` library
//! still builds on **1.75**. See workspace
//! [`POLICIES.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/POLICIES.md#1-msrv-minimum-supported-rust-version).
//!
//! # Panics
//!
//! Public bindings do not panic on well-formed input. Rust-side
//! parse / serialise errors are converted to `JsError` values
//! that JavaScript receives as a thrown `Error`; the bindings
//! never `unwrap` user-supplied data. The remaining sources of
//! panic on the WebAssembly instance are environmental, not
//! logic, and are documented here for completeness:
//!
//! - **WASM linear-memory exhaustion (OOM).** Allocating a Vec
//!   or String larger than the host's WASM memory limit aborts
//!   the instance. Browsers default to a 4 GiB cap on
//!   `wasm32-unknown-unknown`. Bound `noyalib` resource use
//!   ahead of time via `ParserConfig::strict()` or explicit
//!   `max_*` budgets to fail with a structured `Error::Budget`
//!   instead of an OOM abort.
//! - **Stack overflow.** Pathologically deep YAML (>4096 nested
//!   nodes by default) is rejected with `Error::RecursionLimitExceeded`
//!   long before the WASM stack overflows; deliberately
//!   misconfigured `max_depth` may overflow the host stack.
//! - **Host abort on `panic = abort`.** Every release build
//!   uses `panic = abort` (per `Cargo.toml`), so any logic-level
//!   panic terminates the WebAssembly instance with no chance
//!   of `catch_unwind`. `console_error_panic_hook` is wired in
//!   `examples/` so debug builds surface a JS-readable trace
//!   before the abort.
//!
//! # Errors
//!
//! Every fallible binding returns `Result<JsValue, JsError>`;
//! the `JsError` carries the Rust-side `Display` text from
//! [`noyalib::Error`]. JavaScript callers handle these as
//! standard thrown errors — `try { … } catch (e) { e.message }`.
//!
//! # Concurrency
//!
//! WebAssembly is single-threaded on browsers (without
//! shared-memory + `--enable-experimental-features`). The
//! bindings hold no internal state outside the `WasmDocument`
//! handle; multi-document workloads simply construct multiple
//! handles. Web Workers hosting independent WASM instances are
//! the recommended way to parallelise on the browser.
//!
//! # Platform support
//!
//! Builds against every wasm-pack target: `bundler` (Webpack /
//! Rollup / esbuild / Vite), `web` (native ES module), `nodejs`
//! (CommonJS), `deno`, `no-modules` (plain global). CI verifies
//! the `wasm32-unknown-unknown` target each PR via
//! `wasm-pack test --node`. Cloudflare Workers, Deno, and Bun
//! consume the `bundler` target via their own packaging step;
//! see [`crates/noyalib-wasm/doc/bundling.md`](https://github.com/sebastienrousseau/noyalib/blob/main/crates/noyalib-wasm/doc/bundling.md).
//!
//! # Performance
//!
//! Release bundle size: ~338 KB raw, ~140 KB gzip. With the
//! optional `wasm-opt` post-build pass (`--features wasm-opt`):
//! ~280 KB raw, ~115 KB gzip. Tree-shaking-friendly: the
//! `WasmDocument` API and the plain `parse` / `stringify` API
//! are independent modules; bundlers drop whichever your code
//! does not import.
//!
//! # Security
//!
//! `#![forbid(unsafe_code)]`. No `unsafe` outside `wasm-bindgen`'s
//! own bridge. No network I/O. No filesystem access (browsers
//! sandbox WASM by default; Node and Deno hosts can grant fs
//! access to the host process, but this crate's bindings do
//! not expose it). Resource-limit gates are inherited from
//! `noyalib`'s `ParserConfig` defaults. Full posture:
//! [`SECURITY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/SECURITY.md).
//!
//! # API stability and SemVer
//!
//! Pre-1.0 (`0.0.x`): the JavaScript-facing API (`parse`,
//! `stringify`, `WasmDocument` shape, JSON shape of returned
//! values) is **stable** within a 0.0.x line — bug fixes only.
//! Adding a new method or option is allowed within a 0.0.x
//! bump; removing or renaming an exported binding, or changing
//! the JSON shape of a returned object, is held to a 0.x bump
//! (e.g. 0.0.x → 0.1.0). The Rust library surface (`WasmDocument`,
//! `core::*`) is covered by the workspace SemVer policy in
//! [`POLICIES.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/POLICIES.md#2-semver--api-stability).
//! `cargo-semver-checks` runs in CI on every PR.
//!
//! # Documentation
//!
//! - **Engineering policies** — workspace
//!   [`POLICIES.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/POLICIES.md).
//! - **JS API reference**:
//!   [`doc/js-api.md`](https://github.com/sebastienrousseau/noyalib/blob/main/crates/noyalib-wasm/doc/js-api.md).
//! - **Bundling guide** (Vite, Webpack, Next.js, Cloudflare,
//!   Deno, Bun):
//!   [`doc/bundling.md`](https://github.com/sebastienrousseau/noyalib/blob/main/crates/noyalib-wasm/doc/bundling.md).
//! - **Browser / Node demos**:
//!   [`examples/`](https://github.com/sebastienrousseau/noyalib/tree/main/crates/noyalib-wasm/examples).

#![forbid(unsafe_code)]

pub mod core;

use noyalib::cst::{Document, parse_document};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Convert a Rust value to a `JsValue` using the JS-friendly
/// serializer config:
///
/// - **Mappings** become plain JS Objects (`{ name: 'foo' }`),
///   not the `Map` instance `serde_wasm_bindgen`'s default
///   produces. End users overwhelmingly expect dot-property
///   access (`value.name`) rather than `.get('name')`.
/// - Other defaults are preserved.
fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value.serialize(&serializer)
}

#[derive(Serialize)]
struct WasmSpan {
    start: usize,
    end: usize,
}

/// A YAML document with byte-faithful source preservation and path-targeted edits.
#[wasm_bindgen]
pub struct WasmDocument {
    inner: Document,
}

#[wasm_bindgen]
impl WasmDocument {
    /// Parse a YAML string into a lossless Document.
    #[wasm_bindgen(constructor)]
    pub fn new(yaml: &str) -> Result<WasmDocument, JsError> {
        let doc = parse_document(yaml).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WasmDocument { inner: doc })
    }

    /// Re-emit the document as a string. Byte-identical to original if no edits.
    ///
    /// Bound on the JS side as `toString()` (the conventional camelCase
    /// name) so callers can write `doc.toString()` and override the
    /// default `Object.prototype.toString` rather than coexist with it.
    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    /// Replace the bytes at `start..end` with `replacement`.
    ///
    /// Bound on the JS side as `replaceSpan(start, end, replacement)`
    /// per JS naming conventions.
    #[wasm_bindgen(js_name = replaceSpan)]
    pub fn replace_span(
        &mut self,
        start: usize,
        end: usize,
        replacement: &str,
    ) -> Result<(), JsError> {
        self.inner
            .replace_span(start, end, replacement)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get the parsed value at a dotted path.
    pub fn get(&self, path: &str) -> Result<JsValue, JsError> {
        match core::document_get_value(&self.inner, path) {
            Some(v) => to_js(&v).map_err(|e| JsError::new(&e.to_string())),
            None => Ok(JsValue::NULL),
        }
    }

    /// Get the raw source fragment at a dotted path.
    ///
    /// Bound on the JS side as `getSource(path)` per JS naming
    /// conventions.
    #[wasm_bindgen(js_name = getSource)]
    pub fn get_source(&self, path: &str) -> JsValue {
        match core::document_get_source(&self.inner, path) {
            Some(s) => JsValue::from_str(s),
            None => JsValue::NULL,
        }
    }

    /// Get the byte range `{ start, end }` for the value at a dotted path.
    ///
    /// Bound on the JS side as `spanAt(path)` per JS naming
    /// conventions.
    #[wasm_bindgen(js_name = spanAt)]
    pub fn span_at(&self, path: &str) -> Result<JsValue, JsError> {
        match core::document_span_at(&self.inner, path) {
            Some((start, end)) => {
                to_js(&WasmSpan { start, end }).map_err(|e| JsError::new(&e.to_string()))
            }
            None => Ok(JsValue::NULL),
        }
    }

    /// Set a value at a dotted path using a JS object.
    ///
    /// Bound on the JS side as `setValue(path, value)` per JS naming
    /// conventions.
    #[wasm_bindgen(js_name = setValue)]
    pub fn set_value(&mut self, path: &str, value: JsValue) -> Result<(), JsError> {
        let v: noyalib::Value =
            serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))?;
        self.inner
            .set_value(path, &v)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set a value at a dotted path using a YAML fragment string.
    pub fn set(&mut self, path: &str, fragment: &str) -> Result<(), JsError> {
        self.inner
            .set(path, fragment)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Read the YAML comments associated with the node at `path`.
    /// Returns `{ before: string[], inline: string | null }` so the
    /// caller can surface human-authored doc-comments alongside
    /// values — the demo that motivates the entire CST architecture.
    ///
    /// Bound on the JS side as `commentsAt(path)` per JS naming
    /// conventions.
    #[wasm_bindgen(js_name = commentsAt)]
    pub fn comments_at(&self, path: &str) -> Result<JsValue, JsError> {
        let (before, inline) = core::document_comments_at(&self.inner, path);
        #[derive(Serialize)]
        struct Bundle {
            before: Vec<String>,
            inline: Option<String>,
        }
        to_js(&Bundle { before, inline }).map_err(|e| JsError::new(&e.to_string()))
    }
}

impl WasmDocument {
    /// Native (rlib) accessor for the inner [`Document`]. Lets
    /// `cargo test` exercise the underlying state transitions
    /// without going through a JS shell.
    pub fn as_document(&self) -> &Document {
        &self.inner
    }
}

// ── Legacy / Simple API ──────────────────────────────────────────────────────

/// Parse a YAML string and return a JS object.
#[wasm_bindgen]
pub fn parse(yaml: &str) -> Result<JsValue, JsError> {
    let value = core::parse_yaml_to_value(yaml).map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&value).map_err(|e| JsError::new(&e.to_string()))
}

/// Serialize a JS object to a YAML string.
#[wasm_bindgen]
pub fn stringify(value: JsValue) -> Result<String, JsError> {
    let v: noyalib::Value =
        serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))?;
    core::value_to_yaml(&v).map_err(|e| JsError::new(&e.to_string()))
}

/// Validate YAML against the JSON-compatible schema. Bound on the
/// JS side as `validateJson(yaml)` per JS naming conventions.
#[wasm_bindgen(js_name = validateJson)]
pub fn validate_json(yaml: &str) -> Result<bool, JsError> {
    core::validate_yaml_json(yaml).map_err(|e| JsError::new(&e.to_string()))
}

/// Get a value at a dotted path from a YAML string. Bound on the
/// JS side as `getPath(yaml, path)` per JS naming conventions.
#[wasm_bindgen(js_name = getPath)]
pub fn get_path(yaml: &str, path: &str) -> Result<JsValue, JsError> {
    match core::yaml_get_path(yaml, path).map_err(|e| JsError::new(&e.to_string()))? {
        Some(v) => to_js(&v).map_err(|e| JsError::new(&e.to_string())),
        None => Ok(JsValue::NULL),
    }
}

/// Merge two YAML documents.
#[wasm_bindgen]
pub fn merge(base_yaml: &str, override_yaml: &str) -> Result<String, JsError> {
    core::merge_yaml(base_yaml, override_yaml).map_err(|e| JsError::new(&e.to_string()))
}
