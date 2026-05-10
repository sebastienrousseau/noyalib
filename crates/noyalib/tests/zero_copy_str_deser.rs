// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Zero-copy `Deserialize<'de>` for `&'de str` and `Cow<'de, str>`
//! from YAML input.
//!
//! Verifies that the streaming deserialiser surfaces plain-scalar
//! string content via `visit_borrowed_str` when the parser produced a
//! `Cow::Borrowed` event, allowing callers that target `&'de str` to
//! obtain a slice into the input buffer with no intermediate
//! allocation. Scalars that the parser materialised into an owned
//! buffer (line-folded plain scalars, double-quoted scalars with
//! escapes, alias replays, transformed tags) correctly fall back to
//! `visit_string`.
//!
//! Plain-scalar borrowing currently fires reliably for the *terminal*
//! scalar of an input — the parser's slow-path always allocates an
//! owned buffer when it crosses a line break, even when no folding is
//! actually applied. Tightening the slow path to emit
//! `Cow::Borrowed` when the scalar does not continue on the next line
//! is tracked as a follow-up parser optimisation; until then,
//! `Cow<'a, str>` is the recommended target for callers who want
//! borrow-when-possible semantics.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::borrowed::TransformReason;
use noyalib::from_str_borrowing;
use serde::Deserialize;
use std::borrow::Cow;

#[test]
fn cow_str_target_handles_plain_and_escaped() {
    #[derive(Debug, Deserialize)]
    struct CowDoc<'a> {
        #[serde(borrow)]
        plain: Cow<'a, str>,
        #[serde(borrow)]
        escaped: Cow<'a, str>,
    }
    let yaml = "plain: hello\nescaped: \"line\\nbreak\"\n";
    let v: CowDoc<'_> = from_str_borrowing(yaml).unwrap();
    assert_eq!(v.plain, "hello");
    assert_eq!(v.escaped, "line\nbreak");
}

#[test]
fn terminal_scalar_borrows_zero_copy() {
    // A scalar at the very end of input (no trailing newline) hits
    // the parser's fast path and yields a `Cow::Borrowed` event,
    // allowing the streaming deserialiser to call
    // `visit_borrowed_str` and satisfy `Deserialize<'de> for &'de str`.
    let yaml = "value: terminal";
    #[derive(Debug, Deserialize)]
    struct One<'a> {
        value: &'a str,
    }
    let v: One<'_> = from_str_borrowing(yaml).unwrap();
    assert_eq!(v.value, "terminal");
    let yaml_range = yaml.as_ptr() as usize..(yaml.as_ptr() as usize + yaml.len());
    assert!(
        yaml_range.contains(&(v.value.as_ptr() as usize)),
        "scalar should borrow from input slice"
    );
}

#[test]
fn typical_yaml_borrows_zero_copy() {
    // After the parser fast-path extension (`scalar_terminates_on_line`
    // covers `key: value\n`), the typical YAML shape now produces
    // `Cow::Borrowed` events. `&'a str` deserialise should succeed
    // and point back into the input buffer.
    #[derive(Debug, Deserialize)]
    struct Doc<'a> {
        name: &'a str,
        role: &'a str,
    }
    let yaml = "name: noyalib\nrole: parser\n";
    let v: Doc<'_> = from_str_borrowing(yaml).unwrap();
    assert_eq!(v.name, "noyalib");
    assert_eq!(v.role, "parser");

    let yaml_range = yaml.as_ptr() as usize..(yaml.as_ptr() as usize + yaml.len());
    assert!(yaml_range.contains(&(v.name.as_ptr() as usize)));
    assert!(yaml_range.contains(&(v.role.as_ptr() as usize)));
}

#[test]
fn cow_target_works_with_typical_yaml() {
    // The common `key: value\n` shape produces an owned scalar event
    // (parser slow-path), so `&str` targets fail. `Cow<'a, str>`
    // accepts both forms — the recommended target shape today.
    #[derive(Debug, Deserialize)]
    struct One<'a> {
        #[serde(borrow)]
        value: Cow<'a, str>,
    }
    let v: One<'_> = from_str_borrowing("value: hello\n").unwrap();
    assert_eq!(v.value, "hello");
}

#[test]
fn strict_str_target_errors_clearly_when_owned() {
    // For inputs where the parser allocated, `&'de str` deserialise
    // fails with serde's "expected a borrowed string" — a clean
    // error rather than a silent allocation.
    #[derive(Debug, Deserialize)]
    struct One<'a> {
        #[allow(dead_code)]
        s: &'a str,
    }
    let yaml = "s: \"with\\nescape\"\n";
    let res: Result<One<'_>, _> = from_str_borrowing(yaml);
    assert!(res.is_err(), "expected borrow failure on escaped scalar");
}

#[test]
fn transform_reason_messages_are_stable() {
    assert!(TransformReason::EscapeSequence.as_str().contains("escape"));
    assert!(TransformReason::LineFold.as_str().contains("line"));
    assert!(TransformReason::TagResolution.as_str().contains("tag"));
    assert!(TransformReason::QuotedScalar.as_str().contains("quoted"));
    assert!(TransformReason::AliasExpansion.as_str().contains("alias"));
}

#[test]
fn transform_reason_implements_display() {
    use core::fmt::Write;
    let mut s = String::new();
    write!(&mut s, "{}", TransformReason::LineFold).unwrap();
    assert!(s.contains("line"));
}

#[test]
fn transform_reason_traits() {
    let a = TransformReason::EscapeSequence;
    let b = a;
    assert_eq!(a, b);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use core::hash::Hasher;
    use std::hash::Hash;
    a.hash(&mut hasher);
    let _ = hasher.finish();
    let dbg = format!("{a:?}");
    assert!(dbg.contains("Escape"));
}

#[test]
fn from_str_borrowing_with_config_respects_strict_mode() {
    use noyalib::ParserConfig;
    let cfg = ParserConfig::strict();
    let s: Cow<'_, str> = noyalib::from_str_borrowing_with_config("hello\n", &cfg).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn from_str_borrowing_rejects_oversize_input() {
    use noyalib::ParserConfig;
    let cfg = ParserConfig::new().max_document_length(4);
    let res: Result<Cow<'_, str>, _> =
        noyalib::from_str_borrowing_with_config("hello world\n", &cfg);
    assert!(res.is_err(), "oversize input must error");
}
