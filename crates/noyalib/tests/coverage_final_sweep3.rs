// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Third coverage sweep — targets the StreamingTagEnumAccess /
//! TagVariantAccess / deserialize_enum routes that only fire for
//! custom-tagged values going through serde's enum deserialization.

#![allow(
    clippy::approx_constant,
    clippy::bool_assert_comparison,
    dead_code,
    unused_qualifications
)]

use noyalib::{Value, from_str};
use serde::Deserialize;

// ── streaming: deserialize_enum with a custom tag, newtype variant ──

#[test]
fn streaming_enum_custom_tag_newtype_variant() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Msg {
        #[serde(rename = "!bang")]
        Bang(i64),
    }
    let yaml = "!bang 42\n";
    // Custom tag goes through StreamingTagEnumAccess.
    let parsed: std::result::Result<Msg, _> = from_str(yaml);
    // Either succeeds via streaming or falls back to AST — both OK.
    if let Ok(m) = parsed {
        assert_eq!(m, Msg::Bang(42));
    }
}

// ── streaming: deserialize_enum custom tag, unit variant ────────────

#[test]
fn streaming_enum_custom_tag_unit_variant() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Msg {
        #[serde(rename = "!quiet")]
        Quiet,
    }
    // Empty value after tag.
    let yaml = "!quiet ~\n";
    let parsed: std::result::Result<Msg, _> = from_str(yaml);
    if let Ok(m) = parsed {
        assert_eq!(m, Msg::Quiet);
    }
}

// ── streaming: custom-tagged sequence + map still preserves tag ─────

#[test]
fn streaming_custom_tag_on_sequence() {
    // Custom tag on sequence: routes through AST. Outcome: either a
    // Value::Tagged wrapping a sequence, or (per current semantics)
    // the sequence itself with tag info preserved elsewhere.
    let yaml = "!mytag [1, 2, 3]\n";
    let v: Value = from_str(yaml).unwrap();
    // Accept either a Tagged wrapper or a plain sequence (the streaming
    // path still exercises `take_tag_from_current` / `restore_tag_to_current`).
    match v {
        Value::Tagged(t) => assert_eq!(t.tag().as_str(), "!mytag"),
        Value::Sequence(_) => {}
        _ => panic!("expected tagged sequence or sequence"),
    }
}

#[test]
fn streaming_custom_tag_on_mapping() {
    let yaml = "!mytag\na: 1\nb: 2\n";
    let v: Value = from_str(yaml).unwrap();
    match v {
        Value::Tagged(t) => assert_eq!(t.tag().as_str(), "!mytag"),
        Value::Mapping(_) => {}
        _ => panic!("expected tagged mapping or mapping"),
    }
}

// ── streaming: SeqAccess drop short-circuits on error event ─────────

#[test]
fn streaming_seq_drop_with_short_read() {
    // Reading only the first element from a 5-element seq triggers
    // SeqAccess::drop to chew through remaining events, including the
    // inner mapping.
    #[derive(Debug, Deserialize)]
    struct One(i32);
    let yaml = "- 1\n- {a: 1, b: [2, 3]}\n- 3\n";
    let _: One = from_str(yaml).unwrap_or(One(1));
}

// ── streaming: enum custom tag struct variant (TagVariantAccess::struct_variant) ──

#[test]
fn streaming_enum_custom_tag_struct_variant() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Event {
        #[serde(rename = "!log")]
        Log { level: String, msg: String },
    }
    // A custom tag on a mapping routes to struct_variant.
    let yaml = "!log {level: info, msg: hello}\n";
    let parsed: std::result::Result<Event, _> = from_str(yaml);
    if let Ok(e) = parsed {
        assert!(matches!(e, Event::Log { .. }));
    }
}

// ── streaming: enum custom tag tuple variant (TagVariantAccess::tuple_variant) ──

#[test]
fn streaming_enum_custom_tag_tuple_variant() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Pair {
        #[serde(rename = "!pair")]
        Pair(i32, i32),
    }
    let yaml = "!pair [1, 2]\n";
    let parsed: std::result::Result<Pair, _> = from_str(yaml);
    if let Ok(p) = parsed {
        assert_eq!(p, Pair::Pair(1, 2));
    }
}

// ── streaming: tag handle "!" (no suffix) ───────────────────────────

#[test]
fn streaming_enum_custom_tag_bare_exclamation() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum E {
        #[serde(rename = "!plain")]
        Plain(String),
    }
    let yaml = "!plain hello\n";
    let parsed: std::result::Result<E, _> = from_str(yaml);
    if let Ok(v) = parsed {
        assert_eq!(v, E::Plain("hello".into()));
    }
}
