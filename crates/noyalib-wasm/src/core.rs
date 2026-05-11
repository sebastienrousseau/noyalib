// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Pure-Rust core for the noyalib WASM bindings.
//!
//! Every JS-facing entry point in `lib.rs` reduces to a thin
//! conversion layer over a function in this module. The split lets
//! `cargo test` exercise the entire logical surface of the WASM
//! crate natively — JsValue plumbing aside, the binding code that
//! *can* fail in non-trivial ways is here, fully reachable from
//! native unit tests.
//!
//! All functions take `&str` / native Rust types and return
//! `Result<T, noyalib::Error>` so tests can pattern-match on the
//! error variant without dragging in `wasm-bindgen` types.

use noyalib::Value;
use noyalib::cst::Document;

/// Parse a YAML string into a [`Value`] tree.
///
/// # Examples
///
/// ```
/// let v = noyalib_wasm::core::parse_yaml_to_value("k: 1\n").unwrap();
/// assert_eq!(v["k"].as_i64(), Some(1));
/// ```
pub fn parse_yaml_to_value(yaml: &str) -> noyalib::Result<Value> {
    noyalib::from_str(yaml)
}

/// Serialise a [`Value`] tree back to YAML text.
///
/// # Examples
///
/// ```
/// use noyalib_wasm::core::{parse_yaml_to_value, value_to_yaml};
/// let v = parse_yaml_to_value("k: 1\n").unwrap();
/// let s = value_to_yaml(&v).unwrap();
/// assert!(s.contains("k:"));
/// ```
pub fn value_to_yaml(value: &Value) -> noyalib::Result<String> {
    noyalib::to_string(value)
}

/// Round-trip a YAML string through parsing + serialisation. Useful
/// as a smoke test that the input is parseable and re-emittable.
///
/// # Examples
///
/// ```
/// let s = noyalib_wasm::core::yaml_round_trip("k: 42\n").unwrap();
/// assert!(s.contains("42"));
/// ```
pub fn yaml_round_trip(yaml: &str) -> noyalib::Result<String> {
    let v = parse_yaml_to_value(yaml)?;
    value_to_yaml(&v)
}

/// Validate a YAML string against the embedded JSON schema. Returns
/// `Ok(true)` on success — the binary form is what the WASM API
/// returns to JS callers.
///
/// # Examples
///
/// ```
/// assert!(noyalib_wasm::core::validate_yaml_json("k: 1\n").unwrap());
/// ```
pub fn validate_yaml_json(yaml: &str) -> noyalib::Result<bool> {
    let value: Value = noyalib::from_str(yaml)?;
    noyalib::validate_yaml_json_schema(&value).map(|()| true)
}

/// Resolve a dotted path inside a YAML document, returning the
/// resolved [`Value`] when the path exists.
///
/// # Examples
///
/// ```
/// let v = noyalib_wasm::core::yaml_get_path("a:\n  b: 1\n", "a.b")
///     .unwrap()
///     .unwrap();
/// assert_eq!(v.as_i64(), Some(1));
/// ```
pub fn yaml_get_path(yaml: &str, path: &str) -> noyalib::Result<Option<Value>> {
    let value: Value = noyalib::from_str(yaml)?;
    Ok(value.get_path(path).cloned())
}

/// Merge `override_yaml` into `base_yaml` and re-emit the combined
/// document. The override has precedence, mirroring `Value::merge`.
///
/// # Examples
///
/// ```
/// let merged = noyalib_wasm::core::merge_yaml("a: 1\n", "b: 2\n").unwrap();
/// assert!(merged.contains("a:"));
/// assert!(merged.contains("b:"));
/// ```
pub fn merge_yaml(base_yaml: &str, override_yaml: &str) -> noyalib::Result<String> {
    let mut base: Value = noyalib::from_str(base_yaml)?;
    let overrides: Value = noyalib::from_str(override_yaml)?;
    base.merge(overrides);
    noyalib::to_string(&base)
}

/// Look up the byte span of the value at `path` inside the parsed
/// document. The pair `(start, end)` is half-open — `end` is the
/// first byte past the value.
///
/// # Examples
///
/// ```
/// let doc = noyalib::cst::parse_document("name: noyalib\n").unwrap();
/// let (s, e) = noyalib_wasm::core::document_span_at(&doc, "name").unwrap();
/// assert_eq!(&"name: noyalib\n"[s..e], "noyalib");
/// ```
pub fn document_span_at(doc: &Document, path: &str) -> Option<(usize, usize)> {
    doc.span_at(path)
}

/// Resolve a dotted path inside the [`Document`]'s `Value` view.
///
/// Returns an owned [`Value`] clone — the underlying tree lives
/// behind a `RefCell` on the [`Document`], so the lifetime of a
/// borrow is tied to the temporary `Ref`. Cloning is cheap for
/// scalar leaves and an acceptable fixed cost on the WASM
/// boundary, where the value is about to be re-encoded as a
/// JsValue anyway.
///
/// # Examples
///
/// ```
/// let doc = noyalib::cst::parse_document("a:\n  b: 1\n").unwrap();
/// let v = noyalib_wasm::core::document_get_value(&doc, "a.b").unwrap();
/// assert_eq!(v.as_i64(), Some(1));
/// ```
pub fn document_get_value(doc: &Document, path: &str) -> Option<Value> {
    doc.as_value().get_path(path).cloned()
}

/// Read the raw source fragment at the given path.
///
/// # Examples
///
/// ```
/// let doc = noyalib::cst::parse_document("name: noyalib\n").unwrap();
/// let s = noyalib_wasm::core::document_get_source(&doc, "name").unwrap();
/// assert_eq!(s, "noyalib");
/// ```
pub fn document_get_source<'a>(doc: &'a Document, path: &str) -> Option<&'a str> {
    doc.get(path)
}

/// Read the comments associated with the node at `path`. Returns
/// `(before_comments, inline_comment)` — both views the WASM API
/// surfaces verbatim to JS callers.
///
/// # Examples
///
/// ```
/// let doc = noyalib::cst::parse_document("# top\nname: noyalib\n").unwrap();
/// let (before, inline) = noyalib_wasm::core::document_comments_at(&doc, "name");
/// assert!(!before.is_empty());
/// assert!(inline.is_none());
/// ```
pub fn document_comments_at(doc: &Document, path: &str) -> (Vec<String>, Option<String>) {
    let bundle = doc.comments_at(path);
    let before: Vec<String> = bundle.before.iter().map(|c| c.text.clone()).collect();
    let inline: Option<String> = bundle.inline.map(|c| c.text);
    (before, inline)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_yaml_to_value_basic() {
        let v = parse_yaml_to_value("a: 1\nb: hello\n").unwrap();
        assert_eq!(v["a"].as_i64(), Some(1));
        assert_eq!(v["b"].as_str(), Some("hello"));
    }

    #[test]
    fn parse_yaml_to_value_returns_parse_error() {
        let res = parse_yaml_to_value("a: [\n");
        assert!(res.is_err(), "unterminated flow sequence must error");
    }

    #[test]
    fn value_to_yaml_round_trips() {
        let yaml = "a: 1\nb: 2\n";
        let v = parse_yaml_to_value(yaml).unwrap();
        let out = value_to_yaml(&v).unwrap();
        // Re-parse the output — content must round-trip even if
        // whitespace is reformatted.
        let v2 = parse_yaml_to_value(&out).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn yaml_round_trip_smoke() {
        let out = yaml_round_trip("k: 42\n").unwrap();
        assert!(out.contains("k:"));
        assert!(out.contains("42"));
    }

    #[test]
    fn validate_yaml_json_succeeds_on_json_compatible_input() {
        let ok = validate_yaml_json("a: 1\nb: hello\n").unwrap();
        assert!(ok);
    }

    #[test]
    fn validate_yaml_json_rejects_unparseable() {
        let res = validate_yaml_json("a: [\n");
        assert!(res.is_err());
    }

    #[test]
    fn yaml_get_path_finds_existing_key() {
        let v = yaml_get_path("a:\n  b: 42\n", "a.b").unwrap();
        assert_eq!(v.unwrap().as_i64(), Some(42));
    }

    #[test]
    fn yaml_get_path_returns_none_for_missing_key() {
        let v = yaml_get_path("a: 1\n", "missing.path").unwrap();
        assert!(v.is_none());
    }

    #[test]
    fn yaml_get_path_propagates_parse_error() {
        let res = yaml_get_path("a: [\n", "a");
        assert!(res.is_err());
    }

    #[test]
    fn merge_yaml_combines_disjoint_keys() {
        let merged = merge_yaml("a: 1\n", "b: 2\n").unwrap();
        let v = parse_yaml_to_value(&merged).unwrap();
        assert_eq!(v["a"].as_i64(), Some(1));
        assert_eq!(v["b"].as_i64(), Some(2));
    }

    #[test]
    fn merge_yaml_override_wins_on_conflict() {
        let merged = merge_yaml("port: 80\n", "port: 8080\n").unwrap();
        let v = parse_yaml_to_value(&merged).unwrap();
        assert_eq!(v["port"].as_i64(), Some(8080));
    }

    #[test]
    fn merge_yaml_propagates_base_parse_error() {
        let res = merge_yaml("a: [\n", "b: 1\n");
        assert!(res.is_err());
    }

    #[test]
    fn merge_yaml_propagates_override_parse_error() {
        let res = merge_yaml("a: 1\n", "b: [\n");
        assert!(res.is_err());
    }

    #[test]
    fn document_span_at_returns_value_range() {
        let doc = noyalib::cst::parse_document("name: noyalib\n").unwrap();
        let (start, end) = document_span_at(&doc, "name").unwrap();
        assert!(start < end);
        // The slice should match the value text, not the key.
        assert_eq!(&"name: noyalib\n"[start..end], "noyalib");
    }

    #[test]
    fn document_span_at_missing_path_is_none() {
        let doc = noyalib::cst::parse_document("a: 1\n").unwrap();
        assert!(document_span_at(&doc, "missing").is_none());
    }

    #[test]
    fn document_get_value_resolves_nested_path() {
        let doc = noyalib::cst::parse_document("a:\n  b: 42\n").unwrap();
        let v = document_get_value(&doc, "a.b").unwrap();
        assert_eq!(v.as_i64(), Some(42));
    }

    #[test]
    fn document_get_value_missing_returns_none() {
        let doc = noyalib::cst::parse_document("a: 1\n").unwrap();
        assert!(document_get_value(&doc, "x.y").is_none());
    }

    #[test]
    fn document_get_source_returns_raw_fragment() {
        let doc = noyalib::cst::parse_document("name: noyalib\n").unwrap();
        let s = document_get_source(&doc, "name").unwrap();
        assert_eq!(s, "noyalib");
    }

    #[test]
    fn document_get_source_missing_is_none() {
        let doc = noyalib::cst::parse_document("a: 1\n").unwrap();
        assert!(document_get_source(&doc, "missing").is_none());
    }

    #[test]
    fn document_comments_at_returns_before_and_inline() {
        let yaml = "# top-level\nname: noyalib # inline\n";
        let doc = noyalib::cst::parse_document(yaml).unwrap();
        let (before, inline) = document_comments_at(&doc, "name");
        assert_eq!(before.len(), 1);
        assert!(before[0].contains("top-level"));
        assert!(inline.is_some());
        assert!(inline.unwrap().contains("inline"));
    }

    #[test]
    fn document_comments_at_no_comments_returns_empty() {
        let doc = noyalib::cst::parse_document("name: noyalib\n").unwrap();
        let (before, inline) = document_comments_at(&doc, "name");
        assert!(before.is_empty());
        assert!(inline.is_none());
    }
}
