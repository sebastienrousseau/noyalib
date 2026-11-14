//! Reject raw controls in directive fields and unrepresentable tag URIs.
//! Preserve accepted custom tags through serialization, including names
//! resembling built-ins.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::Value;

#[track_caller]
fn refused(input: &str) {
    let r: Result<Value, _> = noyalib::from_str(input);
    assert!(r.is_err(), "{input:?} must be rejected, got {r:?}");
    assert!(noyalib::cst::parse_document(input).is_err(), "{input:?}");
}

#[test]
fn verbatim_tag_with_control_characters_is_rejected() {
    // The fuzz_roundtrip crash input.
    refused("!<\u{7f}\0\0\0\0\0\0\t!");
    refused("!<\u{1}> v");
    // A line break inside the brackets is a control character too —
    // the old loop only stopped at `>`.
    refused("!<a\nb> v");
}

#[test]
fn empty_verbatim_tag_is_rejected() {
    // c-verbatim-tag requires ns-uri-char+ — one or more.
    refused("!<> v");
}

#[test]
fn shorthand_tag_with_control_characters_is_rejected() {
    refused("!\u{1}suffix v");
    refused("!!su\u{7f}ffix v");
    refused("!h!su\u{1}ffix v");
}

#[test]
fn tag_directive_prefixes_must_be_representable() {
    refused("%TAG ! tag:example:>\n---\n!x v\n");
    refused("%TAG !e! tag:example:>\n---\n!e!x v\n");
}

#[test]
fn empty_resolved_tag_uris_are_rejected() {
    refused("%TAG !! \n---\n!! x\n");
    refused("%TAG !e! \n---\n!e! x\n");
}

#[test]
fn directive_fields_reject_raw_controls() {
    for byte in (0u8..32).filter(|b| !matches!(b, 9 | 10 | 13)).chain([127]) {
        let c = char::from(byte);
        for line in [
            format!("%X{c}"),
            format!("%YAML 1.{c}"),
            format!("%TAG !h{c}! tag:example:"),
            format!("%TAG ! tag:exa{c}mple:"),
        ] {
            let source = format!("{line}\n---\nkey: value\n");
            refused(&source);
            let error = noyalib::from_str::<Value>(&source).unwrap_err();
            assert_eq!(error.location().unwrap().index(), source.find(c).unwrap());
        }
    }
    let source = "%TAG\t!h!\ttag:example:\t# ok\n---\nkey: !h!foo v\n";
    assert!(noyalib::from_str::<Value>(source).is_ok());
    assert_eq!(
        noyalib::cst::parse_document(source).unwrap().to_string(),
        source
    );
}

#[test]
fn ordinary_tags_still_parse() {
    let v: Value = noyalib::from_str("!<tag:example.com,2026:x> v").unwrap();
    assert!(matches!(v, Value::Tagged(_)));
    let v: Value = noyalib::from_str("!!str 42").unwrap();
    assert_eq!(v.as_str(), Some("42"));
    let v: Value = noyalib::from_str("!custom v").unwrap();
    assert!(matches!(v, Value::Tagged(_)));
}

#[test]
fn accepted_tags_round_trip_through_emit() {
    for src in [
        "!<tag:example.com,2026:x> v\n",
        "!custom v\n",
        "!!str 42\n",
        "!<!!str> 7\n",
        "!<!!int> x\n",
        "!<!!bool> x\n",
        "!<!!float> x\n",
        "!<!!null> x\n",
        "!<!!seq> [1]\n",
        "!<!!map> {a: 1}\n",
        "%TAG !e! tag:example:%3E\n---\n!e!x v\n",
    ] {
        let v: Value = noyalib::from_str(src).unwrap();
        let emitted = noyalib::to_string(&v).unwrap();
        let back: Value = noyalib::from_str(&emitted)
            .unwrap_or_else(|e| panic!("emit of {src:?} must re-parse: {e} ({emitted:?})"));
        assert_eq!(v, back, "{src:?}");
    }
}

#[test]
fn empty_secondary_looking_uri_keeps_verbatim_spelling() {
    let value: Value = noyalib::from_str("!<!!> x\n").unwrap();
    let emitted = noyalib::to_string(&value).unwrap();
    assert!(
        emitted.contains("!<!!>"),
        "a secondary shorthand needs a suffix: {emitted:?}"
    );
}

#[test]
fn unspellable_tag_bodies_emit_as_quoted_key_mappings() {
    // In the serde data model a tagged value and a single-entry
    // mapping keyed by its `!`-leading spelling are the same thing.
    // When the tag body holds a character no tag spelling can carry
    // (a tab; `>`), the emitter resolves the ambiguity toward the
    // mapping with a quoted key, and the output round-trips.
    let v: Value = noyalib::from_str("\"!\t\":").unwrap();
    let emitted = noyalib::to_string(&v).unwrap();
    let back: Value =
        noyalib::from_str(&emitted).unwrap_or_else(|e| panic!("must re-parse: {e} ({emitted:?})"));
    assert_eq!(v, back, "{emitted:?}");
}

#[test]
fn block_scalar_content_rejects_raw_control_characters() {
    // Found by the serde_yaml parity fuzzer (`>-\x07`): §5.1
    // c-printable governs block scalar content too.
    refused(">-\n \u{7}");
    refused("a: |\n  x\u{1}y\n");
    // Tabs inside content stay legal.
    let v: Value = noyalib::from_str("a: |\n  x\ty\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("x\ty\n"));
}

#[test]
fn double_quoted_rejects_raw_controls_but_keeps_escapes() {
    refused("a: \"x\u{7}y\"\n");
    refused("\"\u{1}\"");
    // The escape spellings remain the way to carry controls.
    let v: Value = noyalib::from_str("a: \"x\\x07y\\u0001\"\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("x\u{7}y\u{1}"));
}

#[test]
fn block_scalar_header_line_carries_no_content() {
    // §8.1.1: after `|`/`>` and its indicators, only blanks and a
    // comment may precede the line break — a literal `\n` (or any
    // text) on the header line is malformed, not content (found by
    // the serde_yaml parity fuzzer on `>-\n`).
    refused(">-\\n");
    refused("a: |x\n  y\n");
    refused("a: >2x\n  y\n");
    // Comments and the indicators themselves remain fine.
    let v: Value = noyalib::from_str("a: |- # note\n  x\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("x"));
}

fn unicode_source_contexts(ch: char) -> Vec<String> {
    vec![
        format!("a{ch}z"),
        format!("key: a{ch}z\n"),
        format!("# a{ch}z\nok\n"),
        format!("|\n  a{ch}z\n"),
        format!(">\n  a{ch}z\n"),
        format!("| # a{ch}z\n  ok\n"),
        format!("&a{ch}z ok\n"),
        format!("!a{ch}z ok\n"),
        format!("!<tag:example:a{ch}z> ok\n"),
        format!("%UNUSED a{ch}z\n---\nok\n"),
        format!("%TAG !e! tag:example:a{ch}z\n---\n!e!x ok\n"),
    ]
}

#[test]
fn raw_nonprintable_unicode_is_rejected_outside_quoted_content() {
    let mut accepted = Vec::new();
    for ch in [
        '\u{80}', '\u{84}', '\u{86}', '\u{9f}', '\u{fffe}', '\u{ffff}',
    ] {
        for source in unicode_source_contexts(ch) {
            if noyalib::from_str::<Value>(&source).is_ok() {
                accepted.push(format!("Value: {source:?}"));
            }
            for mode in [
                noyalib::MergeKeyPolicy::Auto,
                noyalib::MergeKeyPolicy::AsOrdinary,
            ] {
                let cfg = noyalib::ParserConfig::new().merge_key_policy(mode);
                if noyalib::from_str_with_config::<serde::de::IgnoredAny>(&source, &cfg).is_ok() {
                    accepted.push(format!("ignored {mode:?}: {source:?}"));
                }
            }
            if noyalib::cst::parse_document(&source).is_ok() {
                accepted.push(format!("CST: {source:?}"));
            }
        }
    }
    assert!(accepted.is_empty(), "{}", accepted.join("\n"));
    let original = include_str!("../fixtures/raw-nonprintable-unicode.bin");
    refused(original);
}

#[test]
fn unicode_printable_edges_and_quoted_non_c0_content_remain_supported() {
    for ch in [
        '\u{85}',
        '\u{a0}',
        '\u{d7ff}',
        '\u{e000}',
        '\u{fdd0}',
        '\u{fffd}',
        '\u{10000}',
        '\u{1fffe}',
        '\u{10ffff}',
    ] {
        for source in unicode_source_contexts(ch) {
            let parsed = noyalib::from_str::<Value>(&source);
            assert!(parsed.is_ok(), "{source:?}: {parsed:?}");
        }
    }
    for ch in ['\u{80}', '\u{9f}', '\u{fffe}', '\u{ffff}'] {
        for source in [
            format!("'a{ch}z'"),
            format!("\"a{ch}z\""),
            format!("\"\\u{:04x}\"", ch as u32),
        ] {
            let value: String = noyalib::from_str(&source).unwrap();
            let serialized = noyalib::to_string(&value).unwrap();
            assert_eq!(noyalib::from_str::<String>(&serialized).unwrap(), value);
        }
    }
}
