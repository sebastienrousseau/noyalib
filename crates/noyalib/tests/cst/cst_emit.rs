// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Auto-formatting insertion mutators: `insert_entry_value`,
//! `push_back_value`, `insert_after_value`, and the `Emit` trait
//! behind them.
//!
//! The property under test is the one the verbatim fragment mutators
//! cannot offer: a value spliced by these methods **loads back as that
//! value**. A fragment that would restructure the document is quoted,
//! and anything the emitter cannot reproduce is refused with the
//! document left byte-identical — never silently misinterpreted.

use noyalib::cst::{Emit, EmitCtx, parse_document};
use noyalib::{FlowStyle, Mapping, ScalarStyle, Value};

fn seq(items: &[Value]) -> Value {
    Value::Sequence(items.to_vec())
}

fn map(pairs: &[(&str, Value)]) -> Value {
    let mut m = Mapping::new();
    for (k, v) in pairs {
        let _ = m.insert(*k, v.clone());
    }
    Value::Mapping(m)
}

// ── The gap this closes: syntax-shaped data stays data ──────────────

#[test]
fn sequence_item_that_looks_like_a_nested_item_is_quoted() {
    let mut doc = parse_document("items:\n  - one\n").unwrap();
    doc.push_back_value("items", "- two").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - \"- two\"\n");
    assert_eq!(
        doc.as_value()["items"],
        seq(&[Value::from("one"), Value::from("- two")]),
    );
}

#[test]
fn sequence_item_that_looks_like_a_mapping_is_quoted() {
    let mut doc = parse_document("items:\n  - one\n").unwrap();
    doc.push_back_value("items", "two: 2").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - \"two: 2\"\n");
    assert_eq!(doc.as_value()["items"][1], Value::from("two: 2"));
}

#[test]
fn entry_value_that_looks_like_a_mapping_is_quoted() {
    let mut doc = parse_document("labels:\n  app: noyalib\n").unwrap();
    doc.insert_entry_value("labels", "spec", "a: b").unwrap();
    assert_eq!(
        doc.to_string(),
        "labels:\n  app: noyalib\n  spec: \"a: b\"\n",
    );
    assert_eq!(doc.as_value()["labels"]["spec"], Value::from("a: b"));
}

#[test]
fn type_changing_strings_are_quoted() {
    for (input, want) in [
        ("8080", "\"8080\""),
        ("true", "\"true\""),
        ("null", "\"null\""),
        ("~", "\"~\""),
        ("no", "\"no\""),
        ("1.5", "\"1.5\""),
        ("#comment", "\"#comment\""),
        ("*alias", "\"*alias\""),
        ("&anchor", "\"&anchor\""),
        ("[flow]", "\"[flow]\""),
    ] {
        let mut doc = parse_document("m:\n  a: x\n").unwrap();
        doc.insert_entry_value("m", "k", input).unwrap();
        assert_eq!(
            doc.to_string(),
            format!("m:\n  a: x\n  k: {want}\n"),
            "input {input:?}",
        );
        // The point of the quoting: it round-trips as a string.
        assert_eq!(doc.as_value()["m"]["k"], Value::from(input));
    }
}

#[test]
fn keys_are_quoted_too() {
    let mut doc = parse_document("m:\n  a: x\n").unwrap();
    doc.insert_entry_value("m", "a: b", "v").unwrap();
    assert_eq!(doc.to_string(), "m:\n  a: x\n  \"a: b\": v\n");
    assert_eq!(doc.as_value()["m"]["a: b"], Value::from("v"));
}

#[test]
fn numeric_looking_keys_are_quoted() {
    let mut doc = parse_document("m:\n  a: x\n").unwrap();
    doc.insert_entry_value("m", "8080", "http").unwrap();
    assert_eq!(doc.to_string(), "m:\n  a: x\n  \"8080\": http\n");
}

// ── Typed values ───────────────────────────────────────────────────

#[test]
fn primitives_emit_plain() {
    let mut doc = parse_document("m:\n  a: x\n").unwrap();
    doc.insert_entry_value("m", "n", &7_i64).unwrap();
    doc.insert_entry_value("m", "f", &1.5_f64).unwrap();
    doc.insert_entry_value("m", "b", &true).unwrap();
    doc.insert_entry_value("m", "z", &Value::Null).unwrap();
    assert_eq!(
        doc.to_string(),
        "m:\n  a: x\n  n: 7\n  f: 1.5\n  b: true\n  z: null\n",
    );
    let m = &doc.as_value()["m"];
    assert_eq!(m["n"], Value::from(7_i64));
    assert_eq!(m["b"], Value::Bool(true));
    assert_eq!(m["z"], Value::Null);
}

#[test]
fn nested_mapping_is_emitted_as_a_block() {
    let mut doc = parse_document("spec:\n  name: web\n").unwrap();
    let limits = map(&[
        ("image", Value::from("nginx")),
        ("replicas", Value::from(3_i64)),
    ]);
    doc.insert_entry_value("spec", "resources", &limits)
        .unwrap();
    assert_eq!(
        doc.to_string(),
        "spec:\n  name: web\n  resources:\n    image: nginx\n    replicas: 3\n",
    );
    assert_eq!(doc.as_value()["spec"]["resources"], limits);
}

#[test]
fn nested_mapping_respects_a_four_space_file() {
    let mut doc = parse_document("spec:\n    name: web\n").unwrap();
    let limits = map(&[("image", Value::from("nginx"))]);
    doc.insert_entry_value("spec", "resources", &limits)
        .unwrap();
    assert_eq!(
        doc.to_string(),
        "spec:\n    name: web\n    resources:\n        image: nginx\n",
    );
    assert_eq!(doc.as_value()["spec"]["resources"], limits);
}

#[test]
fn collection_scalars_round_trip_even_when_the_serializer_over_quotes() {
    // The serializer quotes digit-leading strings conservatively
    // (`100m` becomes `"100m"`), which is not minimal but is correct.
    // The contract this tier owes is the round trip, not the spelling.
    let mut doc = parse_document("spec:\n  name: web\n").unwrap();
    let limits = map(&[("cpu", Value::from("100m")), ("memory", Value::from("1Gi"))]);
    doc.insert_entry_value("spec", "resources", &limits)
        .unwrap();
    assert_eq!(doc.as_value()["spec"]["resources"], limits);
}

#[test]
fn mapping_pushed_onto_a_sequence_aligns_under_the_dash() {
    let mut doc = parse_document("containers:\n  - name: web\n").unwrap();
    let item = map(&[
        ("name", Value::from("sidecar")),
        ("port", Value::from(8080_i64)),
    ]);
    doc.push_back_value("containers", &item).unwrap();
    assert_eq!(
        doc.to_string(),
        "containers:\n  - name: web\n  - name: sidecar\n    port: 8080\n",
    );
    assert_eq!(doc.as_value()["containers"][1], item);
}

#[test]
fn multiline_string_becomes_a_block_scalar() {
    let mut doc = parse_document("m:\n  a: x\n").unwrap();
    doc.insert_entry_value("m", "script", "one\ntwo\n").unwrap();
    assert_eq!(
        doc.to_string(),
        "m:\n  a: x\n  script: |\n    one\n    two\n"
    );
    assert_eq!(doc.as_value()["m"]["script"], Value::from("one\ntwo\n"));
}

#[test]
fn multiline_string_in_a_sequence_item() {
    let mut doc = parse_document("items:\n  - one\n").unwrap();
    doc.push_back_value("items", "a\nb\n").unwrap();
    assert_eq!(doc.as_value()["items"][1], Value::from("a\nb\n"));
    // Whatever the indent, the value must survive the round trip.
    let reparsed = parse_document(&doc.to_string()).unwrap();
    assert_eq!(*reparsed.as_value(), *doc.as_value());
}

// ── Style matching ─────────────────────────────────────────────────

#[test]
fn single_quoted_files_get_single_quoted_insertions() {
    let mut doc = parse_document("m:\n  a: 'one'\n  b: 'two'\n").unwrap();
    doc.insert_entry_value("m", "c", "three").unwrap();
    assert_eq!(
        doc.to_string(),
        "m:\n  a: 'one'\n  b: 'two'\n  c: 'three'\n"
    );
}

#[test]
fn double_quoted_files_get_double_quoted_insertions() {
    let mut doc = parse_document("m:\n  a: \"one\"\n  b: \"two\"\n").unwrap();
    doc.insert_entry_value("m", "c", "three").unwrap();
    assert_eq!(
        doc.to_string(),
        "m:\n  a: \"one\"\n  b: \"two\"\n  c: \"three\"\n",
    );
}

#[test]
fn single_quoting_gives_way_to_control_characters() {
    let mut doc = parse_document("m:\n  a: 'one'\n  b: 'two'\n").unwrap();
    doc.insert_entry_value("m", "c", "a\tb").unwrap();
    assert_eq!(
        doc.to_string(),
        "m:\n  a: 'one'\n  b: 'two'\n  c: \"a\\tb\"\n",
    );
    assert_eq!(doc.as_value()["m"]["c"], Value::from("a\tb"));
}

#[test]
fn embedded_quotes_are_escaped_for_their_style() {
    let mut doc = parse_document("m:\n  a: 'one'\n").unwrap();
    doc.insert_entry_value("m", "b", "it's").unwrap();
    assert_eq!(doc.as_value()["m"]["b"], Value::from("it's"));

    let mut doc = parse_document("m:\n  a: \"one\"\n").unwrap();
    doc.insert_entry_value("m", "b", "say \"hi\"").unwrap();
    assert_eq!(doc.as_value()["m"]["b"], Value::from("say \"hi\""));
}

// ── insert_after_value ─────────────────────────────────────────────

#[test]
fn insert_after_places_the_item_at_the_right_index() {
    let mut doc = parse_document("items:\n  - one\n  - three\n").unwrap();
    doc.insert_after_value("items[0]", "two").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - two\n  - three\n");
    assert_eq!(
        doc.as_value()["items"],
        seq(&[Value::from("one"), Value::from("two"), Value::from("three"),]),
    );
}

#[test]
fn insert_after_quotes_syntax_shaped_items() {
    let mut doc = parse_document("items:\n  - one\n  - three\n").unwrap();
    doc.insert_after_value("items[0]", "- two").unwrap();
    assert_eq!(doc.as_value()["items"][1], Value::from("- two"));
}

#[test]
fn insert_after_rejects_a_path_without_an_index() {
    let mut doc = parse_document("items:\n  - one\n").unwrap();
    let err = doc.insert_after_value("items", "two").unwrap_err();
    assert!(
        err.to_string().contains("sequence index"),
        "unexpected error: {err}",
    );
    assert_eq!(doc.to_string(), "items:\n  - one\n");
}

#[test]
fn root_sequence_is_addressable() {
    let mut doc = parse_document("- one\n- two\n").unwrap();
    doc.insert_after_value("[0]", "mid").unwrap();
    assert_eq!(doc.to_string(), "- one\n- mid\n- two\n");
    doc.push_back_value("", "last").unwrap();
    assert_eq!(doc.to_string(), "- one\n- mid\n- two\n- last\n");
}

// ── Existing keys ──────────────────────────────────────────────────

#[test]
fn an_existing_key_is_replaced_in_place() {
    let mut doc = parse_document("m:\n  a: x  # keep me\n  b: y\n").unwrap();
    doc.insert_entry_value("m", "a", "z").unwrap();
    assert_eq!(doc.to_string(), "m:\n  a: z  # keep me\n  b: y\n");
    assert_eq!(doc.as_value()["m"]["a"], Value::from("z"));
}

#[test]
fn replacing_an_existing_key_quotes_too() {
    let mut doc = parse_document("m:\n  a: x\n").unwrap();
    doc.insert_entry_value("m", "a", "true").unwrap();
    assert_eq!(doc.to_string(), "m:\n  a: \"true\"\n");
    assert_eq!(doc.as_value()["m"]["a"], Value::from("true"));
}

// ── Preservation ───────────────────────────────────────────────────

#[test]
fn untouched_bytes_survive_verbatim() {
    let src = "# header\n\nspec:  # trailing\n  a: 1\n\n  # a note\n  b: 'two'\n\nother: keep\n";
    let mut doc = parse_document(src).unwrap();
    doc.insert_entry_value("spec", "c", "three").unwrap();
    let out = doc.to_string();
    assert!(out.starts_with("# header\n\nspec:  # trailing\n  a: 1\n"));
    assert!(out.contains("  # a note\n  b: 'two'\n"));
    assert!(out.ends_with("other: keep\n"));
    assert!(out.contains("c: 'three'"));
}

#[test]
fn crlf_documents_are_not_reflowed() {
    let src = "m:\r\n  a: 1\r\n";
    let mut doc = parse_document(src).unwrap();
    doc.insert_entry_value("m", "b", &2_i64).unwrap();
    assert!(
        doc.to_string().starts_with("m:\r\n  a: 1\r\n"),
        "existing CRLF lines must be untouched: {:?}",
        doc.to_string(),
    );
    assert_eq!(doc.as_value()["m"]["b"], Value::from(2_i64));
}

// ── Refusals, each leaving the document unchanged ──────────────────

#[test]
fn merge_key_spelling_is_refused() {
    let mut doc = parse_document("m:\n  a: 1\n").unwrap();
    let before = doc.to_string();
    let err = doc.insert_entry_value("m", "<<", "v").unwrap_err();
    assert!(err.to_string().contains("merge directive"), "{err}");
    assert_eq!(doc.to_string(), before);
}

#[test]
fn non_printable_keys_are_refused() {
    let mut doc = parse_document("m:\n  a: 1\n").unwrap();
    let before = doc.to_string();
    let err = doc.insert_entry_value("m", "a\u{7}b", "v").unwrap_err();
    assert!(err.to_string().contains("non-printable"), "{err}");
    assert_eq!(doc.to_string(), before);
}

#[test]
fn inserting_inside_an_aliased_anchor_names_the_anchor() {
    let src = "base: &base\n  a: 1\nuse: *base\n";
    let mut doc = parse_document(src).unwrap();
    let before = doc.to_string();
    let err = doc.insert_entry_value("base", "b", &2_i64).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("&base"), "must name the anchor: {msg}");
    assert!(
        msg.contains("materialise_aliases_of"),
        "must suggest the fix: {msg}",
    );
    assert_eq!(doc.to_string(), before);
}

// ── Dotted keys (`app.kubernetes.io/name` and friends) ─────────────

#[test]
fn a_dotted_key_can_be_inserted() {
    let mut doc = parse_document("labels:\n  app: web\n").unwrap();
    doc.insert_entry_value("labels", "app.kubernetes.io/name", "web")
        .unwrap();
    assert_eq!(
        doc.to_string(),
        "labels:\n  app: web\n  app.kubernetes.io/name: web\n",
    );
    assert_eq!(
        doc.as_value()["labels"]["app.kubernetes.io/name"],
        Value::from("web"),
    );
}

#[test]
fn a_dotted_key_does_not_collide_with_the_path_it_looks_like() {
    // `m.a.b` addresses the nested `b`; the key `a.b` is a different
    // thing, and inserting it must not rewrite the nested value.
    let mut doc = parse_document("m:\n  a:\n    b: 1\n").unwrap();
    doc.insert_entry_value("m", "a.b", "v").unwrap();
    assert_eq!(doc.to_string(), "m:\n  a:\n    b: 1\n  a.b: v\n");
    assert_eq!(doc.as_value()["m"]["a"]["b"], Value::from(1_i64));
    assert_eq!(doc.as_value()["m"]["a.b"], Value::from("v"));
}

#[test]
fn replacing_an_existing_dotted_key_is_an_upsert() {
    // Refused before #388, when the path syntax could not address
    // `app.io/name`; the key is now addressed as `labels["app.io/name"]`
    // and rewritten in place like any other existing key.
    let mut doc = parse_document("labels:\n  app.io/name: web\n").unwrap();
    doc.insert_entry_value("labels", "app.io/name", "api")
        .unwrap();
    assert_eq!(doc.to_string(), "labels:\n  app.io/name: api\n");
}

// ── Documents whose last line has no terminator ────────────────────

#[test]
fn a_file_without_a_trailing_newline_still_takes_an_entry() {
    let mut doc = parse_document("m:\n  a: 1").unwrap();
    doc.insert_entry_value("m", "b", &2_i64).unwrap();
    assert_eq!(doc.to_string(), "m:\n  a: 1\n  b: 2\n");
    assert_eq!(doc.as_value()["m"]["b"], Value::from(2_i64));
}

#[test]
fn a_file_without_a_trailing_newline_still_takes_an_item() {
    let mut doc = parse_document("items:\n  - one").unwrap();
    doc.push_back_value("items", "two").unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - two\n");
    assert_eq!(doc.as_value()["items"][1], Value::from("two"));
}

#[test]
fn tagged_values_are_refused() {
    let mut doc = parse_document("m:\n  a: 1\n").unwrap();
    let before = doc.to_string();
    let tagged: Value = noyalib::from_str("!custom 1").unwrap();
    let err = doc.insert_entry_value("m", "t", &tagged).unwrap_err();
    assert!(err.to_string().contains("tagged"), "{err}");
    assert_eq!(doc.to_string(), before);
}

#[test]
fn an_empty_mapping_takes_its_first_flow_entry() {
    // `{}` is a flow mapping; since #338 it receives its first entry
    // in place instead of refusing for want of an indent anchor.
    let mut doc = parse_document("m: {}\n").unwrap();
    doc.insert_entry_value("m", "a", "1").unwrap();
    assert_eq!(doc.to_string(), "m: {a: \"1\"}\n");
}

#[test]
fn an_empty_sequence_takes_its_first_flow_member() {
    // `[]` is a flow sequence; since #338 it receives its first
    // member in place instead of refusing for want of an anchor.
    let mut doc = parse_document("items: []\n").unwrap();
    doc.push_back_value("items", "one").unwrap();
    assert_eq!(doc.to_string(), "items: [one]\n");
}

#[test]
fn a_flow_sequence_takes_the_member_inline() {
    // Since #338 a single-line flow sequence splices `, member`
    // before its closing bracket; only the multi-line form refuses.
    let mut doc = parse_document("items: [one, two]\n").unwrap();
    doc.push_back_value("items", "three").unwrap();
    assert_eq!(doc.to_string(), "items: [one, two, three]\n");

    let mut doc = parse_document("items: [one,\n  two]\n").unwrap();
    let before = doc.to_string();
    assert!(doc.push_back_value("items", "three").is_err());
    assert_eq!(doc.to_string(), before);
}

#[test]
fn a_non_sequence_path_is_refused() {
    let mut doc = parse_document("m:\n  a: 1\n").unwrap();
    let before = doc.to_string();
    assert!(doc.push_back_value("m", "x").is_err());
    assert_eq!(doc.to_string(), before);
}

#[test]
fn a_missing_path_is_refused() {
    let mut doc = parse_document("m:\n  a: 1\n").unwrap();
    let before = doc.to_string();
    assert!(doc.insert_entry_value("nope", "k", "v").is_err());
    assert!(doc.push_back_value("nope", "v").is_err());
    assert!(doc.insert_after_value("nope[0]", "v").is_err());
    assert_eq!(doc.to_string(), before);
}

#[test]
fn a_merge_inherited_key_can_be_overridden() {
    // `<<: *base` puts `a` in the typed view with no entry of its own,
    // and the loader orders it last — so the indent anchor has to
    // ignore it and settle on `b`, the last key that owns bytes.
    // Inserting `a` explicitly then overrides the inherited value,
    // which is what YAML says an explicit key does.
    let src = "base: &base\n  a: 1\nuse:\n  <<: *base\n  b: 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.insert_entry_value("use", "a", &9_i64).unwrap();
    assert_eq!(
        doc.to_string(),
        "base: &base\n  a: 1\nuse:\n  <<: *base\n  b: 2\n  a: 9\n",
    );
    assert_eq!(doc.as_value()["use"]["a"], Value::from(9_i64));
    // The anchor definition is untouched.
    assert_eq!(doc.as_value()["base"]["a"], Value::from(1_i64));
}

#[test]
fn a_mapping_of_only_merged_keys_has_no_indent_anchor() {
    let src = "base: &base\n  a: 1\nuse:\n  <<: *base\n";
    let mut doc = parse_document(src).unwrap();
    let before = doc.to_string();
    let err = doc.insert_entry_value("use", "b", &2_i64).unwrap_err();
    assert!(err.to_string().contains("merge"), "unexpected error: {err}");
    assert_eq!(doc.to_string(), before, "refusal must not touch the source");
}

#[test]
fn growing_an_existing_scalar_into_a_collection_is_refused() {
    let mut doc = parse_document("m:\n  a: 1\n").unwrap();
    let before = doc.to_string();
    let nested = map(&[("x", Value::from(1_i64))]);
    let err = doc.insert_entry_value("m", "a", &nested).unwrap_err();
    assert!(err.to_string().contains("already exists"), "{err}");
    assert_eq!(doc.to_string(), before);
}

// ── The same contract through `Entry` ──────────────────────────────

#[test]
fn entry_insert_value_quotes_its_key_and_value() {
    let mut doc = parse_document("m:\n  a: x\n").unwrap();
    doc.entry("m")
        .insert_value("8080", &Value::from("true"))
        .unwrap();
    assert_eq!(doc.to_string(), "m:\n  a: x\n  \"8080\": \"true\"\n");
    assert_eq!(doc.as_value()["m"]["8080"], Value::from("true"));
}

#[test]
fn entry_push_back_value_quotes_syntax_shaped_items() {
    let mut doc = parse_document("items:\n  - one\n").unwrap();
    doc.entry("items")
        .push_back_value(&Value::from("- two"))
        .unwrap();
    assert_eq!(doc.as_value()["items"][1], Value::from("- two"));
}

#[test]
fn entry_insert_after_value_places_the_item() {
    let mut doc = parse_document("items:\n  - one\n  - three\n").unwrap();
    doc.entry("items[0]")
        .insert_after_value(&Value::from("two"))
        .unwrap();
    assert_eq!(doc.to_string(), "items:\n  - one\n  - two\n  - three\n");
}

#[test]
fn entry_or_insert_value_quotes_too() {
    let mut doc = parse_document("m:\n  a: x\n").unwrap();
    let inserted = doc
        .entry("m.b")
        .or_insert_value(&Value::from("1.5"))
        .unwrap();
    assert!(inserted);
    assert_eq!(doc.to_string(), "m:\n  a: x\n  b: \"1.5\"\n");
    assert_eq!(doc.as_value()["m"]["b"], Value::from("1.5"));

    // Occupied entries are left alone.
    let inserted = doc.entry("m.b").or_insert_value(&Value::from("9")).unwrap();
    assert!(!inserted);
    assert_eq!(doc.as_value()["m"]["b"], Value::from("1.5"));
}

// ── The trait itself ───────────────────────────────────────────────

#[test]
fn emit_and_expected_value_agree() {
    let ctx = EmitCtx::new(ScalarStyle::Plain, FlowStyle::Block, 2, 0);
    assert_eq!("plain".emit(&ctx).unwrap(), "plain");
    assert_eq!("plain".expected_value().unwrap(), Value::from("plain"),);
    assert_eq!(String::from("8080").emit(&ctx).unwrap(), "\"8080\"");
    assert_eq!(false.emit(&ctx).unwrap(), "false");
    assert_eq!(3_u32.emit(&ctx).unwrap(), "3");
    assert_eq!(Value::Null.emit(&ctx).unwrap(), "null");
}

#[test]
fn emit_ctx_reports_its_site() {
    let ctx = EmitCtx::new(ScalarStyle::DoubleQuoted, FlowStyle::Auto, 4, 6);
    assert_eq!(ctx.quote_style(), ScalarStyle::DoubleQuoted);
    assert_eq!(ctx.flow_style(), FlowStyle::Auto);
    assert_eq!(ctx.indent_unit(), 4);
    assert_eq!(ctx.column(), 6);
}

#[test]
fn every_emitted_value_reloads_as_itself() {
    // The contract, swept over the shapes the emitter handles.
    let cases: Vec<Value> = vec![
        Value::from("plain"),
        Value::from("8080"),
        Value::from("true"),
        Value::from("- dash"),
        Value::from("key: value"),
        Value::from("  padded  "),
        Value::from("multi\nline\n"),
        Value::from(""),
        Value::from(0_i64),
        Value::from(-12_i64),
        Value::from(2.5_f64),
        Value::Bool(false),
        Value::Null,
        seq(&[Value::from(1_i64), Value::from("two")]),
        map(&[("k", Value::from("v"))]),
    ];
    for case in cases {
        let mut doc = parse_document("m:\n  seed: 0\n").unwrap();
        doc.insert_entry_value("m", "x", &case)
            .unwrap_or_else(|e| panic!("insert of {case:?} failed: {e}"));
        assert_eq!(doc.as_value()["m"]["x"], case, "round trip of {case:?}");

        let mut doc = parse_document("items:\n  - seed\n").unwrap();
        doc.push_back_value("items", &case)
            .unwrap_or_else(|e| panic!("push of {case:?} failed: {e}"));
        assert_eq!(doc.as_value()["items"][1], case, "round trip of {case:?}");
    }
}

// ── Floats: the spelling must stay a float, not collapse to an int ──

#[test]
fn whole_valued_float_round_trips_as_float_not_int() {
    // `1.0` must emit as `1.0` (not `1`, which would load as an integer
    // and fail the oracle). Regression for the `Display`-based number
    // path that refused every whole-valued float.
    let mut doc = parse_document("m:\n  a: 1\n").unwrap();
    doc.insert_entry_value("m", "ratio", &1.0_f64).unwrap();
    assert!(
        doc.to_string().contains("ratio: 1.0"),
        "got {:?}",
        doc.to_string()
    );
    assert_eq!(doc.as_value()["m"]["ratio"], Value::from(1.0_f64));
}

#[test]
fn special_floats_round_trip() {
    for (v, spelling) in [(f64::INFINITY, ".inf"), (f64::NEG_INFINITY, "-.inf")] {
        let mut doc = parse_document("m:\n  a: 1\n").unwrap();
        doc.insert_entry_value("m", "x", &v).unwrap();
        assert!(
            doc.to_string().contains(spelling),
            "want {spelling}, got {:?}",
            doc.to_string()
        );
        assert_eq!(doc.as_value()["m"]["x"], Value::from(v));
    }
    // `Number` treats NaN == NaN, so `.nan` clears the oracle too.
    let mut doc = parse_document("m:\n  a: 1\n").unwrap();
    doc.insert_entry_value("m", "x", &f64::NAN).unwrap();
    assert!(
        doc.to_string().contains(".nan"),
        "got {:?}",
        doc.to_string()
    );
    assert_eq!(doc.as_value()["m"]["x"], Value::from(f64::NAN));
}

#[test]
fn fractional_float_via_push_back_round_trips() {
    let mut doc = parse_document("nums:\n  - 1\n").unwrap();
    doc.push_back_value("nums", &2.5_f64).unwrap();
    assert_eq!(
        doc.as_value()["nums"],
        seq(&[Value::from(1_i64), Value::from(2.5_f64)]),
    );
}

// ── The post-splice integrity guard: exercise the rollback arm ──────

/// A deliberately inconsistent `Emit`: `emit` splices the integer `1`
/// while `expected_value` claims a string. The spliced fragment
/// re-parses fine, so only the typed-value oracle can catch the
/// disagreement — driving the integrity-check rollback that an honest
/// value never reaches.
struct Contradiction;

impl Emit for Contradiction {
    fn emit(&self, _ctx: &EmitCtx) -> noyalib::Result<String> {
        Ok("1".to_owned())
    }
    fn expected_value(&self) -> noyalib::Result<Value> {
        Ok(Value::from("sentinel"))
    }
}

#[test]
fn integrity_mismatch_rolls_back_insert_entry_value() {
    let mut doc = parse_document("m:\n  a: 1\n").unwrap();
    let before = doc.to_string();
    let err = doc
        .insert_entry_value("m", "k", &Contradiction)
        .unwrap_err();
    assert!(
        err.to_string().contains("integrity check"),
        "expected an integrity-check refusal, got {err}"
    );
    assert_eq!(doc.to_string(), before, "must roll back byte-for-byte");
}

#[test]
fn integrity_mismatch_rolls_back_push_back_value() {
    let mut doc = parse_document("items:\n  - one\n").unwrap();
    let before = doc.to_string();
    let err = doc.push_back_value("items", &Contradiction).unwrap_err();
    assert!(
        err.to_string().contains("integrity check"),
        "expected an integrity-check refusal, got {err}"
    );
    assert_eq!(doc.to_string(), before, "must roll back byte-for-byte");
}

#[test]
fn integrity_mismatch_rolls_back_insert_after_value() {
    let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    let before = doc.to_string();
    let err = doc
        .insert_after_value("items[0]", &Contradiction)
        .unwrap_err();
    assert!(
        err.to_string().contains("integrity check"),
        "expected an integrity-check refusal, got {err}"
    );
    assert_eq!(doc.to_string(), before, "must roll back byte-for-byte");
}

// ── Cover the full Emit surface: every primitive + the accessors ────

#[test]
fn emit_ctx_accessors_report_their_fields() {
    let ctx = EmitCtx::new(ScalarStyle::SingleQuoted, FlowStyle::Flow, 4, 6);
    assert_eq!(ctx.quote_style(), ScalarStyle::SingleQuoted);
    assert_eq!(ctx.flow_style(), FlowStyle::Flow);
    assert_eq!(ctx.indent_unit(), 4);
    assert_eq!(ctx.column(), 6);
}

#[test]
fn every_primitive_type_emits_and_reports_its_value() {
    // Exercise `emit` + `expected_value` for each `impl Emit` — the
    // macro-generated integer/float impls, bool, str/String, Value, and
    // the `&T` blanket — so no per-type impl is left uncovered.
    let ctx = EmitCtx::new(ScalarStyle::Plain, FlowStyle::Block, 2, 0);
    macro_rules! check {
        ($($v:expr),* $(,)?) => {$({
            let v = $v;
            assert!(!v.emit(&ctx).unwrap().is_empty(), "empty emit for {:?}", v.expected_value());
            assert!(v.expected_value().is_ok());
        })*};
    }
    check!(
        0_i8, 1_i16, 2_i32, 3_i64, 4_isize, 5_u8, 6_u16, 7_u32, 8_u64, 9_usize, 1.5_f32, 2.5_f64,
        true, false
    );
    // str / String / Value scalar impls.
    assert_eq!("plain".emit(&ctx).unwrap(), "plain");
    let owned = String::from("owned");
    assert_eq!(owned.emit(&ctx).unwrap(), "owned");
    assert_eq!(Value::from(7_i64).emit(&ctx).unwrap(), "7");
    assert_eq!(Value::Null.emit(&ctx).unwrap(), "null");
}

#[test]
fn or_insert_value_covers_success_noop_and_index_refusal() {
    // Success: a fresh nested key routes through insert_value_at_path.
    let mut doc = parse_document("cfg:\n  a: 1\n").unwrap();
    assert!(
        doc.entry("cfg.b")
            .or_insert_value(&Value::from("8080"))
            .unwrap(),
        "a new key must be inserted"
    );
    assert_eq!(doc.as_value()["cfg"]["b"], Value::from("8080"));
    // No-op: an existing key is left untouched and reports false.
    assert!(
        !doc.entry("cfg.a")
            .or_insert_value(&Value::from("9"))
            .unwrap(),
        "an existing key must not be overwritten"
    );
    assert_eq!(doc.as_value()["cfg"]["a"], Value::from(1_i64));
    // A sequence-index target is refused.
    assert!(doc.entry("cfg[0]").or_insert_value(&Value::Null).is_err());
}

#[test]
fn typed_inserts_of_collections_via_every_mutator() {
    // insert_entry_value with a nested mapping and a sequence.
    let mut doc = parse_document("root:\n  a: 1\n").unwrap();
    doc.insert_entry_value("root", "m", &map(&[("k", Value::from("v"))]))
        .unwrap();
    doc.insert_entry_value("root", "s", &seq(&[Value::from(1_i64), Value::from(2_i64)]))
        .unwrap();
    assert_eq!(doc.as_value()["root"]["m"]["k"], Value::from("v"));
    assert_eq!(doc.as_value()["root"]["s"][1], Value::from(2_i64));

    // push_back_value and insert_after_value with collection items.
    let mut doc2 = parse_document("list:\n  - 1\n").unwrap();
    doc2.push_back_value("list", &map(&[("k", Value::from("v"))]))
        .unwrap();
    doc2.insert_after_value("list[0]", &seq(&[Value::from(9_i64)]))
        .unwrap();
    assert_eq!(doc2.as_value()["list"][1], seq(&[Value::from(9_i64)]));
    assert_eq!(doc2.as_value()["list"][2]["k"], Value::from("v"));
}

// ── Cover the remaining untested forwarders / trait impls ───────────
// These are the functions the coverage gate flagged as never entered:
// `String::expected_value`, the `&T` blanket, `Entry::span_at` /
// `Entry::set_value`, and the (dead) `Error::entry_not_found` helper.

#[test]
fn string_value_round_trips_through_emit_and_oracle() {
    // Passing an owned `String` (not a `&str`) exercises `String`'s own
    // `Emit::emit` + `Emit::expected_value`.
    let mut doc = parse_document("m:\n  a: 1\n").unwrap();
    doc.insert_entry_value("m", "k", &String::from("hello"))
        .unwrap();
    assert_eq!(doc.as_value()["m"]["k"], Value::from("hello"));
    // And the oracle half directly.
    assert_eq!(
        String::from("x").expected_value().unwrap(),
        Value::from("x")
    );
}

#[test]
fn reference_blanket_impl_forwards_both_halves() {
    // `impl<T: Emit + ?Sized> Emit for &T` — reached by naming `&String`
    // as the Emit type explicitly.
    let ctx = EmitCtx::new(ScalarStyle::Plain, FlowStyle::Block, 2, 0);
    let owned = String::from("val");
    let by_ref: &String = &owned;
    assert_eq!(<&String as Emit>::emit(&by_ref, &ctx).unwrap(), "val");
    assert_eq!(
        <&String as Emit>::expected_value(&by_ref).unwrap(),
        Value::from("val")
    );
}

#[test]
fn entry_span_at_and_set_value_forward_to_document() {
    let mut doc = parse_document("m:\n  a: hello\n").unwrap();
    // Entry::span_at
    let span = doc.entry("m.a").span_at().expect("path resolves");
    assert_eq!(&doc.to_string()[span.0..span.1], "hello");
    // Entry::set_value
    doc.entry("m.a").set_value(&Value::from("world")).unwrap();
    assert_eq!(doc.as_value()["m"]["a"], Value::from("world"));
}

#[test]
fn error_entry_not_found_reports_the_path() {
    // A public (doc-hidden) constructor with no internal caller — assert
    // it renders the path so the helper is exercised, not dead-carried.
    let err = noyalib::Error::entry_not_found("a.b.c");
    assert!(err.to_string().contains("a.b.c"), "got {err}");
}

#[test]
fn double_quoted_key_and_top_level_or_insert_refusal() {
    // A key needing quotes in a non-single-quoted document forces
    // emit_key's double-quote branch.
    let mut doc = parse_document("plain: 1\n").unwrap();
    doc.insert_entry_value("", "a: b", &Value::from("v"))
        .unwrap();
    assert!(
        doc.to_string().contains("\"a: b\": v"),
        "got {:?}",
        doc.to_string()
    );

    // or_insert_value on a bare top-level key is refused (the None arm of
    // insert_value_at_path).
    let mut doc2 = parse_document("x: 1\n").unwrap();
    let err = doc2
        .entry("newtop")
        .or_insert_value(&Value::from(1_i64))
        .unwrap_err();
    assert!(err.to_string().contains("top-level key"), "got {err}");
}

// ── Symmetric matrix: every Emit type through every mutator ─────────
// insert_entry_value / push_back_value / insert_after_value are generic
// over the Emit type; each concrete type is a separate monomorphization
// (with its own guard closure). Driving all three with the same diverse
// set exercises every instantiation and its rollback closure, and is a
// thorough regression net for the typed-insert surface.

#[test]
fn every_emit_type_through_every_mutator() {
    fn entry<E: Emit + ?Sized>(v: &E) {
        let mut d = parse_document("m:\n  seed: 0\n").unwrap();
        d.insert_entry_value("m", "k", v).unwrap();
        assert!(d.to_string().contains("k:"));
    }
    fn push<E: Emit + ?Sized>(v: &E) {
        let mut d = parse_document("s:\n  - 0\n").unwrap();
        d.push_back_value("s", v).unwrap();
        assert_eq!(d.as_value()["s"].as_sequence().unwrap().len(), 2);
    }
    fn after<E: Emit + ?Sized>(v: &E) {
        let mut d = parse_document("s:\n  - 0\n  - 9\n").unwrap();
        d.insert_after_value("s[0]", v).unwrap();
        assert_eq!(d.as_value()["s"].as_sequence().unwrap().len(), 3);
    }
    // Drive each mutator's rejection arms too, so the per-type
    // monomorphization's error diagnostics (not just its happy path)
    // are exercised for every Emit type.
    fn errors<E: Emit + ?Sized>(v: &E) {
        let mut d = parse_document("m:\n  a: 1\nseq:\n  - x\n").unwrap();
        let before = d.to_string();
        assert!(d.insert_entry_value("nope", "k", v).is_err());
        assert!(d.insert_entry_value("seq", "k", v).is_err());
        assert!(d.push_back_value("nope", v).is_err());
        assert!(d.push_back_value("m", v).is_err());
        assert!(d.insert_after_value("m", v).is_err());
        assert!(d.insert_after_value("seq[9]", v).is_err());
        assert_eq!(d.to_string(), before, "refused edits must not mutate");
    }
    fn all<E: Emit + ?Sized>(v: &E) {
        entry(v);
        push(v);
        after(v);
        errors(v);
    }

    all(&true);
    all(&7_i64);
    all(&1.5_f64);
    all(&8_u64);
    all(&String::from("text"));
    all("borrowed");
    all(&Value::Null);
    all(&Value::from(42_i64));
    all(&Value::Bool(false));
    all(&map(&[("nested", Value::from("v"))]));
    all(&seq(&[Value::from(1_i64), Value::from(2_i64)]));
}

// ── #290: the quote vote is scoped to the edit site ─────────────────
//
// The document-wide vote counts quoted scalars against each other and
// ignores plain ones, so one quoted line anywhere decided the spelling of
// every later insertion. On a Kubernetes manifest that meant `value: "30"`
// in a container's env block dictating the spelling of a label at the top
// of the file. Insertion now learns from the collection it lands in.

#[test]
fn an_unrelated_quoted_line_does_not_quote_the_insertion() {
    for src in [
        "quoted: \"30\"\nlabels:\n  app: web\n",
        "quoted: 'x'\nlabels:\n  app: web\n",
    ] {
        let mut doc = parse_document(src).unwrap();
        doc.insert_entry_value("labels", "tier", "frontend")
            .unwrap();
        assert!(
            doc.to_string().ends_with("  app: web\n  tier: frontend\n"),
            "the sibling is plain, so the insertion should be: {:?}",
            doc.to_string()
        );
    }
}

#[test]
fn the_site_majority_decides_not_a_single_distant_quote() {
    let mut doc =
        parse_document("m:\n  a: one\n  b: two\n  c: three\n  d: four\n  e: \"5\"\n").unwrap();
    doc.insert_entry_value("m", "j", "w").unwrap();
    assert!(
        doc.to_string().ends_with("  j: w\n"),
        "{:?}",
        doc.to_string()
    );
}

#[test]
fn a_genuinely_quoted_site_still_gets_quoted_insertions() {
    let mut doc = parse_document("m:\n  a: \"one\"\n  b: \"two\"\n").unwrap();
    doc.insert_entry_value("m", "j", "w").unwrap();
    assert!(
        doc.to_string().ends_with("  j: \"w\"\n"),
        "{:?}",
        doc.to_string()
    );

    let mut doc = parse_document("m:\n  a: 'one'\n  b: 'two'\n").unwrap();
    doc.insert_entry_value("m", "j", "w").unwrap();
    assert!(
        doc.to_string().ends_with("  j: 'w'\n"),
        "{:?}",
        doc.to_string()
    );
}

#[test]
fn keys_do_not_vote_only_values_do() {
    // Mapping keys are almost always plain, so counting scalar tokens in
    // the site's byte range would tie two plain keys against two quoted
    // values and wrongly pick plain.
    let mut doc = parse_document("m:\n  alpha: \"one\"\n  beta: \"two\"\n").unwrap();
    doc.insert_entry_value("m", "gamma", "w").unwrap();
    assert!(
        doc.to_string().ends_with("  gamma: \"w\"\n"),
        "{:?}",
        doc.to_string()
    );
}

#[test]
fn a_mixed_site_keeps_the_quoting_it_has() {
    // A tie is not evidence for stripping quotes; #290 is about unrelated
    // lines deciding, not about mixed sites.
    let mut doc = parse_document("m:\n  a: 1\n  b: 'two'\n").unwrap();
    doc.insert_entry_value("m", "c", "three").unwrap();
    assert!(
        doc.to_string().contains("c: 'three'"),
        "{:?}",
        doc.to_string()
    );
}

#[test]
fn sequences_learn_from_their_own_items() {
    let mut doc = parse_document("quoted: \"30\"\nxs:\n  - alpha\n  - beta\n").unwrap();
    doc.push_back_value("xs", "billing").unwrap();
    assert!(
        doc.to_string().ends_with("  - beta\n  - billing\n"),
        "{:?}",
        doc.to_string()
    );
}
