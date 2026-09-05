// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Bracket-quoted key segments in the path grammar (issue #388): a
//! mapping key holding `.`, `[`, `]`, or `*` is addressed as `["key"]`
//! by every path-taking API, so the reads find it and the CST mutators
//! edit that entry rather than whatever the plain spelling resolves to.

#![allow(missing_docs)]

use noyalib::cst::parse_document;
use noyalib::path::{join_keys, quote_key};
use noyalib::{Value, from_str};

fn s(v: &str) -> Value {
    Value::String(v.into())
}

/// Keys the grammar reads as an index (`a[x]`), a wildcard (`*`), and a
/// nested path (`app.io/name`), beside the plain keys those readings
/// would resolve to.
const DOC: &str = "a: 1\nab: 2\n'a[x]': 3  # kept\n'*': star\napp.io/name: web\nlast: z\n";

#[test]
fn reads_find_a_quoted_key() {
    let doc = parse_document(DOC).unwrap();
    assert_eq!(doc.get(r#"["*"]"#), Some("star"));
    let value = doc.as_value();
    assert_eq!(
        value.get_path(r#"["a[x]"]"#).and_then(Value::as_i64),
        Some(3)
    );
    assert_eq!(
        value
            .get_path(&quote_key("app.io/name"))
            .and_then(Value::as_str),
        Some("web")
    );

    let v: Value = from_str(DOC).unwrap();
    assert_eq!(v.get_path(r#"["a[x]"]"#).and_then(Value::as_i64), Some(3));
    assert_eq!(v.query(r#"["*"]"#).len(), 1);
    assert_eq!(v.query(r"['a[x]']").len(), 1);
}

#[test]
fn borrowed_reads_take_a_quoted_key() {
    let v = noyalib::borrowed::from_str_borrowed("a.b: 1\n'*': 2\n").unwrap();
    assert_eq!(v.get_path(r#"["a.b"]"#).and_then(|x| x.as_i64()), Some(1));
    assert_eq!(v.query(r#"["*"]"#).len(), 1);
}

#[test]
fn set_value_writes_the_quoted_key_and_leaves_the_plain_neighbours() {
    let mut doc = parse_document(DOC).unwrap();
    doc.set_value(r#"["a[x]"]"#, &s("changed")).unwrap();
    doc.set_value(r#"["*"]"#, &s("moon")).unwrap();
    doc.set_value(r#"["app.io/name"]"#, &s("api")).unwrap();
    assert_eq!(
        doc.to_string(),
        "a: 1\nab: 2\n'a[x]': changed  # kept\n'*': moon\napp.io/name: api\nlast: z\n"
    );
}

#[test]
fn a_double_quoted_source_key_resolves_through_the_typed_view() {
    // The green-tree walk decodes plain and single-quoted keys only; a
    // double-quoted one falls back to the typed cache, which sees every
    // key decoded.
    let mut doc = parse_document("\"a.b\": 1\nc: 2\n").unwrap();
    doc.set_value(r#"["a.b"]"#, &Value::from(5_i64)).unwrap();
    assert_eq!(doc.to_string(), "\"a.b\": 5\nc: 2\n");
}

#[test]
fn set_path_upserts_creates_and_appends_under_quoted_segments() {
    // An existing leaf is an upsert.
    let mut doc = parse_document("'*': old\n").unwrap();
    doc.set_path(r#"["*"]"#, &s("new")).unwrap();
    assert_eq!(doc.to_string(), "'*': new\n");

    // A missing level under a quoted ancestor: the ancestor prefix is
    // re-spelled quoted when it is handed on to the inserter.
    let mut doc = parse_document("'a.b':\n  c: 1\n").unwrap();
    doc.set_path(r#"["a.b"].d"#, &Value::Bool(true)).unwrap();
    assert_eq!(doc.to_string(), "'a.b':\n  c: 1\n  d: true\n");

    // Missing levels whose keys the emitter has to quote.
    let mut doc = parse_document("title: x\n").unwrap();
    let path = join_keys(["menu", "a[0]", "*"]);
    doc.set_path(&path, &s("v")).unwrap();
    let value = doc.as_value();
    assert_eq!(value.get_path(&path).and_then(Value::as_str), Some("v"));
    assert_eq!(value.get_path("title").and_then(Value::as_str), Some("x"));
    drop(value);

    // The first key of an empty document, under a comment header.
    let mut doc = parse_document("# note\n").unwrap();
    doc.set_path(r#"["*"]"#, &s("v")).unwrap();
    assert!(doc.to_string().starts_with("# note\n"), "{doc}");
    let value = doc.as_value();
    assert_eq!(
        value.get_path(r#"["*"]"#).and_then(Value::as_str),
        Some("v")
    );
}

#[test]
fn remove_and_rename_address_the_quoted_key_not_its_plain_prefix() {
    // At v0.0.32 `remove("a[x]")` removed `a`, and `remove("*")` was
    // refused as a wildcard.
    let mut doc = parse_document(DOC).unwrap();
    doc.remove(r#"["a[x]"]"#).unwrap();
    assert_eq!(
        doc.to_string(),
        "a: 1\nab: 2\n'*': star\napp.io/name: web\nlast: z\n"
    );
    // The renamed key keeps the entry's quote style, as every rename does.
    doc.rename_key(r#"["*"]"#, "star").unwrap();
    assert_eq!(
        doc.to_string(),
        "a: 1\nab: 2\n'star': star\napp.io/name: web\nlast: z\n"
    );
    doc.rename_key(r#"["app.io/name"]"#, "app.io/component")
        .unwrap();
    assert_eq!(
        doc.to_string(),
        "a: 1\nab: 2\n'star': star\napp.io/component: web\nlast: z\n"
    );
    doc.remove(r#"["app.io/component"]"#).unwrap();
    assert_eq!(doc.to_string(), "a: 1\nab: 2\n'star': star\nlast: z\n");
}

#[test]
fn rename_key_still_refuses_an_unquoted_non_index_bracket_segment() {
    let mut doc = parse_document("servers:\n  web: 1\n").unwrap();
    let err = doc.rename_key("servers[web]", "x").unwrap_err().to_string();
    assert!(
        err.contains("neither a sequence index nor a quoted key"),
        "{err}"
    );
    assert!(err.contains(r#"["web"]"#), "{err}");
    assert_eq!(doc.to_string(), "servers:\n  web: 1\n");

    doc.rename_key(r#"servers["web"]"#, "api").unwrap();
    assert_eq!(doc.to_string(), "servers:\n  api: 1\n");

    // A `]` inside the quotes does not end the segment.
    let mut doc = parse_document("'a]b': 1\n").unwrap();
    doc.rename_key("['a]b']", "c").unwrap();
    assert_eq!(doc.to_string(), "'c': 1\n");
}

#[test]
fn span_at_and_key_span_locate_a_quoted_key() {
    let doc = parse_document(DOC).unwrap();
    let (start, end) = doc.span_at(r#"["*"]"#).unwrap();
    assert_eq!(&DOC[start..end], "star");
    let (start, end) = doc.key_span(r#"["a[x]"]"#).unwrap();
    assert!(DOC[start..end].contains("a[x]"), "{}", &DOC[start..end]);
}

#[test]
fn insert_entry_value_upserts_a_key_the_grammar_would_misread() {
    // At v0.0.32 the upsert of an existing `*` appended a second `*`
    // entry (the existing-key guard covered `.` and `[` only), and a
    // key holding `.` or `[` was refused outright.
    let mut doc = parse_document("'*': old\napp.io/name: web\n").unwrap();
    doc.insert_entry_value("", "*", &s("new")).unwrap();
    doc.insert_entry_value("", "app.io/name", &s("api"))
        .unwrap();
    assert_eq!(doc.to_string(), "'*': new\napp.io/name: api\n");

    let mut doc = parse_document("labels:\n  app.io/name: web\n  a[0]: 1\n").unwrap();
    doc.insert_entry("labels", "app.io/name", "api").unwrap();
    doc.insert_entry_value("labels", "a[0]", &Value::from(2_i64))
        .unwrap();
    assert_eq!(doc.to_string(), "labels:\n  app.io/name: api\n  a[0]: 2\n");
}

#[test]
fn entry_handle_composes_a_quoted_child_without_a_separator() {
    let mut doc = parse_document("labels:\n  app.io/name: web\n").unwrap();
    doc.entry("labels")
        .entry(&quote_key("app.io/name"))
        .set("api")
        .unwrap();
    assert_eq!(doc.to_string(), "labels:\n  app.io/name: api\n");
}

#[test]
fn comment_editors_take_a_quoted_key() {
    let mut doc = parse_document("'*': star\n").unwrap();
    doc.set_inline_comment(r#"["*"]"#, "wild").unwrap();
    assert!(doc.to_string().contains("# wild"), "{doc}");
    assert!(!doc.comments_at(r#"["*"]"#).is_empty());
}

#[test]
fn a_plain_key_reads_the_same_quoted_or_not() {
    let mut doc = parse_document("server:\n  port: 8080\n").unwrap();
    doc.set_value(r#"server["port"]"#, &Value::from(9090_i64))
        .unwrap();
    assert_eq!(doc.to_string(), "server:\n  port: 9090\n");
    assert_eq!(doc.get("server.port"), Some("9090"));
    assert_eq!(doc.get(&join_keys(["server", "port"])), Some("9090"));
}
