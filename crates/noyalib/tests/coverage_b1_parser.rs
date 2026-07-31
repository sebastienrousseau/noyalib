// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Broad parser/loader coverage: a wide variety of YAML documents,
//! exercising valid edge cases (anchors, aliases, merge keys, tags,
//! every scalar style, flow/block collections, multi-doc, empty docs,
//! comments, explicit/complex keys) and malformed documents that must
//! be rejected (undefined alias, unterminated flow, invalid escapes,
//! bad tag resolution, scalar merge, key collisions, bad indentation).

#![allow(
    missing_docs,
    dead_code,
    unused_results,
    unused_must_use,
    non_snake_case,
    clippy::all
)]

use noyalib::{
    DuplicateKeyPolicy, MergeKeyPolicy, ParserConfig, Value, from_str, from_str_with_config,
};

// ── Helpers ─────────────────────────────────────────────────────────────

fn parse(yaml: &str) -> Value {
    from_str::<Value>(yaml).expect("expected a valid document")
}

// ── Valid: scalar styles ────────────────────────────────────────────────

#[test]
fn plain_scalar_resolves_by_schema() {
    assert_eq!(parse("42").as_i64(), Some(42));
    assert_eq!(parse("3.5").as_f64(), Some(3.5));
    assert_eq!(parse("true").as_bool(), Some(true));
    assert!(parse("~").is_null());
    assert!(parse("null").is_null());
    assert_eq!(parse("hello world").as_str(), Some("hello world"));
}

#[test]
fn double_quoted_scalar_with_escapes() {
    let v = parse(r#""line\tbreak\n""#);
    assert_eq!(v.as_str(), Some("line\tbreak\n"));
}

#[test]
fn literal_block_scalar_preserves_newlines() {
    let v = parse("text: |\n  one\n  two\n");
    assert_eq!(v["text"].as_str(), Some("one\ntwo\n"));
}

#[test]
fn folded_block_scalar_folds_newlines() {
    let v = parse("text: >\n  one\n  two\n");
    assert_eq!(v["text"].as_str(), Some("one two\n"));
}

#[test]
fn special_float_scalars() {
    assert!(parse(".inf").as_f64().unwrap().is_infinite());
    assert!(parse("-.inf").as_f64().unwrap().is_sign_negative());
    assert!(parse(".nan").as_f64().unwrap().is_nan());
}

// ── Valid: collections ──────────────────────────────────────────────────

#[test]
fn flow_sequence() {
    let v = parse("[1, 2, 3]");
    assert_eq!(v.as_sequence().unwrap().len(), 3);
    assert_eq!(v[0].as_i64(), Some(1));
}

#[test]
fn block_mapping_nested() {
    let v = parse("a:\n  b:\n    c: 1\n");
    assert_eq!(v["a"]["b"]["c"].as_i64(), Some(1));
}

#[test]
fn indentless_sequence_under_key() {
    let v = parse("items:\n- a\n- b\n");
    assert_eq!(v["items"].as_sequence().unwrap().len(), 2);
}

#[test]
fn empty_flow_collections() {
    assert!(parse("[]").is_sequence());
    assert!(parse("{}").is_mapping());
}

#[test]
fn empty_block_mapping_value_is_null() {
    let v = parse("key:\n");
    assert!(v["key"].is_null());
}

// ── Valid: anchors, aliases, merge keys ─────────────────────────────────

#[test]
fn anchor_and_alias_scalar() {
    let v = parse("a: &x 7\nb: *x\n");
    assert_eq!(v["a"].as_i64(), Some(7));
    assert_eq!(v["b"].as_i64(), Some(7));
}

#[test]
fn anchor_on_mapping_reused_by_alias() {
    let v = parse("base: &b {p: 1, q: 2}\ncopy: *b\n");
    assert_eq!(v["copy"]["p"].as_i64(), Some(1));
    assert_eq!(v["copy"]["q"].as_i64(), Some(2));
}

#[test]
fn anchor_on_sequence_reused_by_alias() {
    let v = parse("base: &b [1, 2]\ncopy: *b\n");
    assert_eq!(v["copy"].as_sequence().unwrap().len(), 2);
}

#[test]
fn merge_key_splices_mapping() {
    let yaml = "defaults: &d\n  port: 8080\n  host: localhost\nservice:\n  <<: *d\n  host: api\n";
    let v = parse(yaml);
    assert_eq!(v["service"]["port"].as_i64(), Some(8080));
    assert_eq!(v["service"]["host"].as_str(), Some("api"));
}

#[test]
fn merge_key_with_sequence_of_aliases() {
    let yaml = "a: &a {x: 1}\nb: &b {y: 2}\nc:\n  <<: [*a, *b]\n  z: 3\n";
    let v = parse(yaml);
    assert_eq!(v["c"]["x"].as_i64(), Some(1));
    assert_eq!(v["c"]["y"].as_i64(), Some(2));
    assert_eq!(v["c"]["z"].as_i64(), Some(3));
}

#[test]
fn merge_key_null_is_noop() {
    let v = parse("m:\n  <<: ~\n  x: 1\n");
    assert_eq!(v["m"]["x"].as_i64(), Some(1));
}

// ── Valid: tags ─────────────────────────────────────────────────────────

#[test]
fn core_tag_int() {
    assert_eq!(parse("!!int 42").as_i64(), Some(42));
    assert_eq!(parse("!!int 0x1F").as_i64(), Some(31));
    assert_eq!(parse("!!int 0o17").as_i64(), Some(15));
}

#[test]
fn core_tag_float_and_specials() {
    assert_eq!(parse("!!float 1.5").as_f64(), Some(1.5));
    assert!(parse("!!float .inf").as_f64().unwrap().is_infinite());
    assert!(parse("!!float .nan").as_f64().unwrap().is_nan());
}

#[test]
fn core_tag_bool_null_str() {
    assert_eq!(parse("!!bool true").as_bool(), Some(true));
    assert!(parse("!!null ~").is_null());
    assert_eq!(parse("!!str 123").as_str(), Some("123"));
}

#[test]
fn custom_tag_wraps_scalar() {
    assert!(parse("!mytag hello").is_tagged());
}

#[test]
fn custom_tag_wraps_mapping() {
    assert!(parse("!custom {a: 1}\n").is_tagged());
}

// ── Valid: documents ────────────────────────────────────────────────────

#[test]
fn multi_document_stream_via_load_all() {
    let docs: Vec<Value> = noyalib::document::load_all("---\na: 1\n---\nb: 2\n")
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(docs.len(), 2);
}

#[test]
fn empty_document_is_null() {
    assert!(parse("---\n").is_null());
}

#[test]
fn comments_are_ignored() {
    let v = parse("# leading comment\na: 1 # trailing\n# another\nb: 2\n");
    assert_eq!(v["a"].as_i64(), Some(1));
    assert_eq!(v["b"].as_i64(), Some(2));
}

#[test]
fn bom_is_accepted() {
    let v = parse("\u{FEFF}key: value\n");
    assert_eq!(v["key"].as_str(), Some("value"));
}

// ── Valid: explicit and complex keys ────────────────────────────────────

#[test]
fn explicit_key_indicator() {
    let v = parse("? key\n: value\n");
    assert_eq!(v["key"].as_str(), Some("value"));
}

#[test]
fn sequence_as_complex_key() {
    let v = parse("? [a, b]\n: value\n");
    assert!(v.as_mapping().unwrap().contains_key("[a, b]"));
}

#[test]
fn mapping_as_complex_key() {
    let v = parse("? {a: 1}\n: value\n");
    assert!(v.as_mapping().unwrap().contains_key("{a: 1}"));
}

#[test]
fn special_float_keys_canonicalise() {
    let v = parse(".inf: a\n.nan: b\n");
    let m = v.as_mapping().unwrap();
    assert!(m.contains_key("inf"));
    assert!(m.contains_key("nan"));
}

#[test]
fn float_and_bool_keys_stringify() {
    let v = parse("1.5: x\ntrue: y\n");
    let m = v.as_mapping().unwrap();
    assert!(m.contains_key("1.5"));
    assert!(m.contains_key("true"));
}

// ── Valid: duplicate-key policies ───────────────────────────────────────

#[test]
fn duplicate_key_last_wins_by_default() {
    assert_eq!(parse("a: 1\na: 2\n")["a"].as_i64(), Some(2));
}

#[test]
fn duplicate_key_first_policy() {
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let v: Value = from_str_with_config("a: 1\na: 2\n", &cfg).unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

// ── Malformed: must be rejected ─────────────────────────────────────────

#[test]
fn undefined_alias_is_error() {
    assert!(from_str::<Value>("a: *missing\n").is_err());
}

#[test]
fn unterminated_flow_sequence_is_error() {
    assert!(from_str::<Value>("[1, 2, 3").is_err());
}

#[test]
fn invalid_double_quote_escape_is_error() {
    assert!(from_str::<Value>(r#""bad \q escape""#).is_err());
}

#[test]
fn bad_int_tag_value_is_error() {
    assert!(from_str::<Value>("!!int notanumber\n").is_err());
}

#[test]
fn bad_bool_tag_value_is_error() {
    assert!(from_str::<Value>("!!bool maybe\n").is_err());
}

#[test]
fn scalar_in_merge_element_is_error() {
    // `<<` merging a plain scalar (not a mapping/sequence) is rejected.
    assert!(from_str::<Value>("m:\n  <<: notamap\n  x: 1\n").is_err());
}

#[test]
fn distinct_typed_key_collision_is_error() {
    // `1` (int) then `"1"` (string) collapse to the same string key but
    // carry different typed keys — refused to avoid silently dropping one.
    assert!(from_str::<Value>("1: a\n\"1\": b\n").is_err());
}

#[test]
fn duplicate_key_error_policy_rejects() {
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    assert!(from_str_with_config::<Value>("a: 1\na: 2\n", &cfg).is_err());
}

#[test]
fn merge_key_error_policy_rejects() {
    let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::Error);
    assert!(from_str_with_config::<Value>("base: &b {x: 1}\nm:\n  <<: *b\n", &cfg).is_err());
}

#[test]
fn tab_indentation_is_error() {
    // A tab used as indentation before a structural `-` is rejected.
    assert!(from_str::<Value>("a:\n\t- 1\n").is_err());
}

#[test]
fn bad_block_indentation_is_error() {
    // `c` dedents to indent 1 — matching neither the root mapping (0)
    // nor the nested `b` (2), an inconsistent-indentation error.
    assert!(from_str::<Value>("a:\n  b: 1\n c: 2\n").is_err());
}

#[test]
fn max_depth_limit_is_error() {
    let cfg = ParserConfig::new().max_depth(2);
    assert!(from_str_with_config::<Value>("a:\n  b:\n    c:\n      d: 1\n", &cfg).is_err());
}

#[test]
fn max_document_length_limit_is_error() {
    let cfg = ParserConfig::new().max_document_length(4);
    assert!(from_str_with_config::<Value>("key: a long value here\n", &cfg).is_err());
}
