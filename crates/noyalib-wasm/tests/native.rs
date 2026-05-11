// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Native rlib tests for `noyalib-wasm`. The crate is built as
//! both `cdylib` (for `wasm-pack`) and `rlib` (for `cargo test`),
//! so the JsValue-free portions of the binding surface — the
//! `WasmDocument` constructor, `to_string`, `replace_span`, `set`,
//! and the `core` module — are reachable from regular Rust tests.
//!
//! Together with the per-function tests in `core.rs`, this file
//! pins every code path inside `noyalib-wasm` that does not
//! require a wasm-bindgen runtime.

use noyalib_wasm::WasmDocument;
use noyalib_wasm::core::{
    document_comments_at, document_get_source, document_get_value, document_span_at, merge_yaml,
    parse_yaml_to_value, validate_yaml_json, value_to_yaml, yaml_get_path, yaml_round_trip,
};

// ── WasmDocument lifecycle (JsValue-free surface) ──────────────────────

#[test]
fn wasm_document_round_trips_unedited_input() {
    let yaml = "name: noyalib\nversion: 1\n";
    let doc = WasmDocument::new(yaml).unwrap();
    assert_eq!(doc.to_string(), yaml);
}

#[test]
fn wasm_document_replace_span_edits_source_in_place() {
    let mut doc = WasmDocument::new("name: noyalib\nversion: 1\n").unwrap();
    // Replace `noyalib` (offset 6..13).
    doc.replace_span(6, 13, "fast-yaml").unwrap();
    assert_eq!(doc.to_string(), "name: fast-yaml\nversion: 1\n");
}

#[test]
fn wasm_document_set_replaces_value() {
    let mut doc = WasmDocument::new("name: noyalib\nversion: 1\n").unwrap();
    doc.set("version", "2").unwrap();
    assert_eq!(doc.to_string(), "name: noyalib\nversion: 2\n");
}

// NOTE: Error-path tests for `WasmDocument::new`, `replace_span`, and
// `set` cannot run as native rlib tests — those methods build a
// `JsError`, and `JsError::new` is a wasm-bindgen import that
// panics on non-wasm targets ("cannot call wasm-bindgen imported
// functions on non-wasm targets"). Equivalent error coverage is
// provided by `core::tests::*` against the underlying noyalib API.

#[test]
fn wasm_document_as_document_exposes_inner() {
    // Native accessor lets callers reach the underlying CST
    // without going through wasm-bindgen marshalling.
    let doc = WasmDocument::new("a: 1\n").unwrap();
    let inner = doc.as_document();
    assert!(inner.as_value().get_path("a").is_some());
}

// ── core module re-validation ──────────────────────────────────────────

#[test]
fn core_parse_yaml_to_value_smoke() {
    let v = parse_yaml_to_value("k: 42\n").unwrap();
    assert_eq!(v["k"].as_i64(), Some(42));
}

#[test]
fn core_value_to_yaml_smoke() {
    let v = parse_yaml_to_value("k: 42\n").unwrap();
    let s = value_to_yaml(&v).unwrap();
    assert!(s.contains("k:"));
}

#[test]
fn core_yaml_round_trip_smoke() {
    assert!(yaml_round_trip("k: 42\n").unwrap().contains("42"));
}

#[test]
fn core_validate_yaml_json_smoke() {
    assert!(validate_yaml_json("k: 1\n").unwrap());
}

#[test]
fn core_yaml_get_path_smoke() {
    let v = yaml_get_path("a:\n  b: 1\n", "a.b").unwrap();
    assert_eq!(v.unwrap().as_i64(), Some(1));
}

#[test]
fn core_merge_yaml_smoke() {
    let merged = merge_yaml("a: 1\n", "b: 2\n").unwrap();
    let v = parse_yaml_to_value(&merged).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
    assert_eq!(v["b"].as_i64(), Some(2));
}

#[test]
fn core_document_helpers_against_real_doc() {
    let yaml = "# top\nname: noyalib # inline\nversion: 1\n";
    let doc = noyalib::cst::parse_document(yaml).unwrap();

    // span_at + the slice it addresses.
    let (start, end) = document_span_at(&doc, "name").unwrap();
    assert_eq!(&yaml[start..end], "noyalib");

    // get_value resolves the path into the Value tree.
    let v = document_get_value(&doc, "version").unwrap();
    assert_eq!(v.as_i64(), Some(1));

    // get_source returns the raw source fragment.
    assert_eq!(document_get_source(&doc, "name").unwrap(), "noyalib");

    // comments_at surfaces both before and inline comment views.
    let (before, inline) = document_comments_at(&doc, "name");
    assert!(!before.is_empty());
    assert!(inline.is_some());
}
