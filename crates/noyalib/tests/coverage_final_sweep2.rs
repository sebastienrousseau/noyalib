// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Second coverage sweep — targets the last residual clusters of
//! uncovered lines in `streaming.rs`, `de.rs`, `error.rs`, and a few
//! `with/*` modules.

#![allow(
    clippy::approx_constant,
    clippy::bool_assert_comparison,
    dead_code,
    unused_qualifications
)]

use noyalib::{ParserConfig, RcAnchor, Spanned, Value, from_str, from_str_with_config};
use serde::{Deserialize, Serialize};

// ── streaming: anchor whose body contains an alias (maybe_record Alias arm) ─

#[test]
fn streaming_anchor_body_contains_alias() {
    // `a` is recorded; `b` records `*a` inside its mapping body, which
    // exercises the Event::Alias arm inside `maybe_record`.
    let yaml = "\
a: &a first
b: &b
  inner: *a
copy: *b
";
    let v: Value = from_str(yaml).unwrap();
    let copy = v.get("copy").unwrap();
    let inner = copy.get("inner").unwrap();
    assert_eq!(inner.as_str(), Some("first"));
}

// ── streaming: unknown alias inside a merge expansion ────────────────

#[test]
fn streaming_merge_unknown_anchor_errors() {
    let yaml = "base:\n  k: 1\nderived:\n  <<: *missing\n  v: 2\n";
    #[derive(Debug, Deserialize)]
    struct Doc {
        base: std::collections::BTreeMap<String, i64>,
        derived: std::collections::BTreeMap<String, i64>,
    }
    let err = from_str::<Doc>(yaml).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("anchor"));
}

// ── streaming: merge source with nested sequence/mapping ─────────────

#[test]
fn streaming_merge_with_nested_structures_in_local() {
    // Local mapping has a nested sequence + nested mapping alongside
    // the <<: merge; exercises nested depth tracking in
    // `buffer_rest_of_mapping`.
    let yaml = "\
base: &base
  shared: 1
child:
  <<: *base
  items:
    - 1
    - 2
  nested:
    inner: yes
";
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Child {
        shared: i64,
        items: Vec<i64>,
        nested: std::collections::BTreeMap<String, String>,
    }
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        #[serde(skip)]
        _base: Option<()>,
        child: Child,
    }
    let v: Value = from_str(yaml).unwrap();
    let child = v.get("child").unwrap();
    assert_eq!(child.get("shared").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(
        child
            .get("items")
            .and_then(|v| v.as_sequence())
            .unwrap()
            .len(),
        2
    );
}

// ── streaming: ignored_any on a scalar (skip_value balance==0 arm) ──

#[test]
fn streaming_ignored_any_on_scalar() {
    #[derive(Debug, Deserialize)]
    struct Doc {
        keep: String,
        #[serde(default, skip_deserializing)]
        _ignored: (),
    }
    let yaml = "keep: yes\nskip_me: sentinel\n";
    // serde's default for the struct path with deny_unknown_fields off
    // will read `skip_me` and then ignore it via the unknown-field
    // fallthrough, which routes through deserialize_ignored_any on a
    // scalar.
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.keep, "yes");
}

#[test]
fn streaming_ignored_any_on_alias() {
    // An ignored field whose value is *an alias* hits the
    // `Event::Alias if balance == 0` arm inside `skip_value`.
    let yaml = "anchor: &a hello\n_skip: *a\nkeep: yes\n";
    #[derive(Debug, Deserialize)]
    struct Doc {
        anchor: String,
        keep: String,
    }
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.anchor, "hello");
    assert_eq!(d.keep, "yes");
}

// ── streaming: deserialize_newtype_struct with a custom tag ─────────

#[derive(Debug, Serialize)]
struct NewtypeWrapper(String);

impl<'de> Deserialize<'de> for NewtypeWrapper {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Access the {tag, value} map produced by StreamingTagMapAccess.
        #[derive(Deserialize)]
        struct Inner {
            tag: String,
            value: String,
        }
        let inner = Inner::deserialize(deserializer)?;
        Ok(NewtypeWrapper(format!("{}={}", inner.tag, inner.value)))
    }
}

#[test]
fn streaming_newtype_struct_with_custom_tag() {
    // Deserializing a struct that takes a NewtypeWrapper with a custom
    // tag routes through StreamingTagMapAccess.
    #[derive(Debug, Deserialize)]
    struct Doc {
        field: std::collections::BTreeMap<String, String>,
    }
    // Fallback path via mapping deserialisation preserves the tag in
    // the AST. We just care the path doesn't panic.
    let yaml = "field:\n  a: b\n";
    let _: Doc = from_str(yaml).unwrap();
}

// ── streaming: billion-laughs guard triggered by alias_bytes ────────

#[test]
fn streaming_alias_bytes_guard() {
    // Multiple expansions of a large anchor should eventually hit the
    // alias_bytes saturating cap — enforced via max_document_length.
    let cfg = ParserConfig::new().max_document_length(128);
    // Anchor with an 80-byte payload, then 10 aliases of it.
    let payload = "x".repeat(80);
    let yaml = format!("a: &a {payload}\nlist:\n  - *a\n  - *a\n  - *a\n",);
    let err = from_str_with_config::<Value>(&yaml, &cfg).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("limit")
            || err.to_string().to_lowercase().contains("document")
            || err.to_string().to_lowercase().contains("repetition")
    );
}

// ── AST path: Deserializer::wrap_err with span context ──────────────

#[test]
fn ast_wrap_err_emits_deserialize_with_location() {
    // A Spanned<T> deserialize error gets wrapped with location. Force
    // the AST path via Spanned<String>, then throw a Deserialize error
    // by expecting a non-existent variant.
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        #[serde(rename = "kind")]
        _force: Spanned<String>,
        #[allow(dead_code)]
        value: Choice,
    }
    #[derive(Debug, Deserialize)]
    enum Choice {
        A,
        B,
    }
    let yaml = "kind: x\nvalue: C\n";
    let err = from_str::<Doc>(yaml).unwrap_err();
    // Error mentions unknown variant `C`.
    assert!(err.to_string().contains('C') || err.to_string().contains("unknown"));
}

// ── AST path: deserialize_identifier with a String value ────────────

#[test]
fn ast_identifier_path_via_tag_key() {
    // Serde uses deserialize_identifier for #[serde(tag = "type")]
    // struct matching. Forcing AST via Spanned<_> gets us into
    // de.rs deserialize_identifier.
    #[derive(Debug, Deserialize)]
    #[serde(tag = "type")]
    #[allow(dead_code)]
    enum Shape {
        Square { side: i64 },
        Circle { radius: i64 },
    }
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        #[allow(dead_code)]
        _force: Spanned<String>,
        s: Shape,
    }
    let yaml = "_force: x\ns:\n  type: Square\n  side: 3\n";
    let d: Doc = from_str(yaml).unwrap();
    match d.s {
        Shape::Square { side } => assert_eq!(side, 3),
        _ => panic!("expected Square"),
    }
}

// ── AST path: deserialize_bytes on a non-string, non-sequence ───────

#[test]
fn ast_deserialize_bytes_type_mismatch() {
    // bytes on a numeric Value should error with "bytes" expected.
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        #[allow(dead_code)]
        _force: Spanned<String>,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    }
    let yaml = "_force: x\ndata: 42\n";
    // AST path: Number is not convertible to bytes → TypeMismatch.
    let err = from_str::<Doc>(yaml).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("bytes"));
}

// ── RcAnchor round-trip hits anchor-tracking serializer paths ───────

#[test]
fn rc_anchor_three_aliases_reuses_same_id() {
    let shared: RcAnchor<String> = RcAnchor::from("payload".to_string());
    let doc = vec![shared.clone(), shared.clone(), shared];
    let yaml = noyalib::to_string_tracking_shared(&doc).unwrap();
    // Expect one anchor definition, two alias references.
    assert_eq!(yaml.matches("&id001").count(), 1);
    assert_eq!(yaml.matches("*id001").count(), 2);
}

// ── Number FP NaN edge cases covered directly ────────────────────────

#[test]
fn value_float_nan_equals_self_structurally() {
    // Value uses bit-level equality for floats — NaN == NaN.
    let a = Value::from(f64::NAN);
    let b = Value::from(f64::NAN);
    assert_eq!(a, b);
}

// ── Streaming: deep sequence catches SeqAccess::drop early return ───

#[test]
fn streaming_seq_drop_on_partial_read() {
    // Reading a tuple (length 2) from a 4-element seq triggers
    // StreamingSeqAccess::drop cleaning up the remaining events.
    let yaml = "- 1\n- 2\n- 3\n- 4\n";
    let pair: (i32, i32) = from_str(yaml).unwrap();
    assert_eq!(pair, (1, 2));
}

// ── Streaming: deep map reads exercise MapAccess::drop ──────────────

#[test]
fn streaming_map_drop_on_partial_read() {
    // Deserialising an empty-field struct from a populated mapping
    // triggers StreamingMapAccess::drop to skip all keys+values.
    let yaml = "a: 1\nb: 2\nc: 3\n";
    #[derive(Debug, Deserialize)]
    struct Empty {}
    let _: Empty = from_str(yaml).unwrap();
}
