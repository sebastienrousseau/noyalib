// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Regression tests for source spelling on typed AST fallback (#494).

#![allow(missing_docs)]

#[cfg(feature = "std")]
use noyalib::Spanned;
use noyalib::{ParserConfig, from_str_with_config, load_all_as_with_config};
use serde::Deserialize;
use std::collections::HashMap;

fn config() -> ParserConfig {
    ParserConfig::new().plain_scalar_strings(true)
}

fn ast_config() -> ParserConfig {
    config().ignore_binary_tag_for_string(true)
}

#[test]
fn an_unrelated_tag_keeps_plain_scalar_spelling() {
    for literal in ["3.10", "0x1F", "007", "True", "NULL", "~", ".NaN", "+.INF"] {
        let source = format!("v: {literal}\nw: !!int 7\n");
        let value: HashMap<String, String> = from_str_with_config(&source, &config()).unwrap();
        assert_eq!(value["v"], literal, "{source:?}");
        assert_eq!(value["w"], "7");
    }
}

#[test]
fn forced_ast_matches_streaming_for_plain_scalars() {
    for literal in ["3.10", "0x1F", "007", "True", "NULL", "~", ".NaN", "+.INF"] {
        let source = format!("v: {literal}\n");
        let streaming: HashMap<String, String> = from_str_with_config(&source, &config()).unwrap();
        let ast: HashMap<String, String> = from_str_with_config(&source, &ast_config()).unwrap();
        assert_eq!(streaming["v"], literal);
        assert_eq!(ast, streaming, "{source:?}");
    }
}

#[test]
#[cfg(feature = "std")]
fn spanned_strings_keep_the_source_spelling_and_location() {
    let value: Spanned<String> = from_str_with_config("0x1F", &config()).unwrap();
    assert_eq!(value.value, "0x1F");
    assert_eq!(value.start.index(), 0);
    assert_eq!(value.end.index(), 4);
}

#[test]
fn nested_sequence_strings_keep_the_source_spelling() {
    #[derive(Deserialize)]
    struct Row {
        v: String,
    }
    let rows: Vec<Row> = from_str_with_config("- v: 3.10\n- v: 0x1F\n", &ast_config()).unwrap();
    assert_eq!(rows[0].v, "3.10");
    assert_eq!(rows[1].v, "0x1F");
}

#[test]
fn aliases_read_the_anchors_scalar_spelling() {
    let source = "base: &n 0x1F\nv: *n\n";
    let value: HashMap<String, String> = from_str_with_config(source, &ast_config()).unwrap();
    assert_eq!(value["base"], "0x1F");
    assert_eq!(value["v"], "0x1F");
}

#[test]
fn a_multi_character_source_does_not_become_a_single_char() {
    assert!(from_str_with_config::<char>("007", &ast_config()).is_err());
}

#[test]
fn typed_multi_document_loading_preserves_each_scalar() {
    let docs: Vec<HashMap<String, String>> =
        load_all_as_with_config("v: 3.10\n---\nv: 0x1F\n", &config()).unwrap();
    assert_eq!(docs[0]["v"], "3.10");
    assert_eq!(docs[1]["v"], "0x1F");
}

#[test]
#[cfg(feature = "std")]
fn configured_reader_preserves_each_scalar() {
    let docs: Vec<HashMap<String, String>> =
        noyalib::read_with_config(std::io::Cursor::new("v: 3.10\n---\nv: 0x1F\n"), &config())
            .unwrap()
            .collect::<noyalib::Result<_>>()
            .unwrap();
    assert_eq!(docs[0]["v"], "3.10");
    assert_eq!(docs[1]["v"], "0x1F");
}

#[test]
fn the_option_keeps_numeric_targets_typed() {
    #[derive(Deserialize)]
    struct Mixed {
        text: String,
        count: u32,
        enabled: bool,
    }
    let value: Mixed =
        from_str_with_config("text: 007\ncount: 0x1F\nenabled: true\n", &ast_config()).unwrap();
    assert_eq!(value.text, "007");
    assert_eq!(value.count, 31);
    assert!(value.enabled);
}

#[test]
fn quoted_strings_are_decoded_and_default_coercion_stays_off() {
    let value: HashMap<String, String> =
        from_str_with_config("v: \"line\\nbreak\"\n", &ast_config()).unwrap();
    assert_eq!(value["v"], "line\nbreak");
    assert!(
        from_str_with_config::<String>(
            "0x1F",
            &ParserConfig::new().ignore_binary_tag_for_string(true)
        )
        .is_err()
    );
}

fn assert_source_alias_budget<T>(fragment: &str, limit: usize)
where
    T: for<'de> Deserialize<'de> + 'static,
{
    let source = format!("base: &n {fragment}\na: *n\nb: *n\nc: *n\n");
    assert!(source.len() < limit);
    for cfg in [config(), ast_config()] {
        let err = from_str_with_config::<T>(&source, &cfg.max_document_length(limit))
            .err()
            .expect("aliases must charge the source text restored into typed strings");
        assert!(
            matches!(err, noyalib::Error::RepetitionLimitExceeded),
            "{err}"
        );
    }
}

#[test]
fn scalar_aliases_charge_the_restored_source_bytes() {
    assert_source_alias_budget::<HashMap<String, String>>(&"0".repeat(200), 512);
}

#[test]
fn sequence_aliases_charge_the_restored_source_bytes() {
    assert_source_alias_budget::<HashMap<String, Vec<String>>>(
        &format!("[{}]", "0".repeat(200)),
        512,
    );
}

#[test]
fn mapping_aliases_charge_the_restored_source_bytes() {
    assert_source_alias_budget::<HashMap<String, HashMap<String, String>>>(
        &format!("{{v: {}}}", "0".repeat(400)),
        1024,
    );
}

#[test]
fn fields_inside_a_tagged_collection_keep_source_spelling() {
    let value: HashMap<String, String> =
        from_str_with_config("!Row\nv: 0x1F\n", &ast_config()).unwrap();
    assert_eq!(value["v"], "0x1F");
}

#[test]
fn merged_fields_keep_their_original_scalar_spelling() {
    let source = "base: &b {v: 0x1F}\nobj: {<<: *b}\n";
    let value: HashMap<String, HashMap<String, String>> =
        from_str_with_config(source, &ast_config()).unwrap();
    assert_eq!(value["base"]["v"], "0x1F");
    assert_eq!(value["obj"]["v"], "0x1F");
}

#[test]
fn merge_order_and_explicit_overrides_keep_their_source_spelling() {
    let source =
        "a: &a {v: 0x1F, w: 007}\nb: &b {v: 3.10, x: True}\nobj: {<<: [*a, *b], w: 0008}\n";
    let value: HashMap<String, HashMap<String, String>> =
        from_str_with_config(source, &ast_config()).unwrap();
    assert_eq!(value["obj"]["v"], "0x1F");
    assert_eq!(value["obj"]["w"], "0008");
    assert_eq!(value["obj"]["x"], "True");
}

#[test]
#[cfg(feature = "std")]
fn cst_inherited_fields_keep_no_owned_source_span() {
    let source = "base: &b {v: 0x1F}\nobj: {<<: *b}\n";
    let doc = noyalib::cst::parse_document_with_config(source, &config()).unwrap();
    assert!(doc.span_at("obj.v").is_none());
    assert_eq!(doc.source(), source);
}
