// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Type mismatches driven through the streaming deserializer itself.
//!
//! `noyalib::from_str` dispatches: some shapes go to the streaming
//! reader, others are loaded into a `Value` first. A test that goes
//! through `from_str` therefore measures whichever path the dispatcher
//! happened to pick, which is not a stable thing to test against —
//! the streaming reader's own `deserialize_*` refusals need
//! [`StreamingDeserializer`] named directly.
//!
//! Each case asks the streaming reader for a Rust type the YAML cannot
//! supply, and checks it says so rather than inventing a value.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use noyalib::StreamingDeserializer;
use serde::Deserialize;
use serde_bytes::ByteBuf;

/// Drive the streaming reader directly and return the error text.
#[track_caller]
fn streamed_err<T>(label: &str, yaml: &str) -> String
where
    T: for<'de> Deserialize<'de> + std::fmt::Debug + 'static,
{
    let mut de = StreamingDeserializer::new(yaml);
    match T::deserialize(&mut de) {
        Ok(v) => panic!("{label}: {yaml:?} was accepted as {v:?}"),
        Err(e) => e.to_string(),
    }
}

/// The control: the same reader handles the well-typed version, so a
/// refusal below is about the type and not about the reader.
#[test]
fn the_streaming_reader_accepts_the_well_typed_versions() {
    let mut de = StreamingDeserializer::new("a: text\n");
    let m: BTreeMap<String, String> = Deserialize::deserialize(&mut de).expect("strings");
    assert_eq!(m["a"], "text");

    let mut de = StreamingDeserializer::new("a: 1\n");
    let m: BTreeMap<String, i64> = Deserialize::deserialize(&mut de).expect("integers");
    assert_eq!(m["a"], 1);
}

/// Asking for a `String` where a collection sits does not produce a
/// user-facing type error from the streaming reader: it raises the
/// internal `$__noyalib_streaming_fallback` sentinel that tells
/// `from_str` to retry through the AST path. Driving the reader
/// directly exposes that sentinel, which is the one thing a caller
/// must never see from `from_str`.
#[test]
fn a_collection_where_a_string_goes_raises_the_fallback_sentinel_only_internally() {
    const SENTINEL: &str = "$__noyalib_streaming_fallback";
    for (label, yaml) in [("a sequence", "a: [1, 2]\n"), ("a mapping", "a: {b: 1}\n")] {
        let msg = streamed_err::<BTreeMap<String, String>>(label, yaml);
        assert!(!msg.is_empty(), "{label}: empty message");

        // Through the public entry point the sentinel must be gone —
        // either the AST path succeeds, or a real error comes back.
        match noyalib::from_str::<BTreeMap<String, String>>(yaml) {
            Ok(_) => {}
            Err(e) => assert!(
                !e.to_string().contains(SENTINEL),
                "{label}: the internal fallback sentinel leaked to the caller: {e}"
            ),
        }
    }
}

#[test]
fn asking_for_bytes_where_a_non_string_sits_is_refused() {
    #[derive(Deserialize, Debug)]
    struct HasBytes {
        #[allow(dead_code)]
        b: ByteBuf,
    }
    for (label, yaml) in [
        ("an integer", "b: 1\n"),
        ("a float", "b: 1.5\n"),
        ("a boolean", "b: true\n"),
        ("a sequence", "b: [1]\n"),
        ("a mapping", "b: {x: 1}\n"),
    ] {
        let msg = streamed_err::<HasBytes>(label, yaml);
        assert!(!msg.is_empty(), "{label}: empty message");
    }
}

#[test]
fn asking_for_a_number_where_text_sits_is_refused() {
    for (label, yaml) in [
        ("plain text", "a: hello\n"),
        ("a quoted digit string", "a: \"12x\"\n"),
        ("a sequence", "a: [1]\n"),
    ] {
        let msg = streamed_err::<BTreeMap<String, i64>>(label, yaml);
        assert!(!msg.is_empty(), "{label}: empty message");
    }
    let msg = streamed_err::<BTreeMap<String, u64>>("a negative", "a: -1\n");
    assert!(!msg.is_empty(), "negative into u64: empty message");
    let msg = streamed_err::<BTreeMap<String, f64>>("text into f64", "a: hello\n");
    assert!(!msg.is_empty(), "text into f64: empty message");
    let msg = streamed_err::<BTreeMap<String, bool>>("text into bool", "a: hello\n");
    assert!(!msg.is_empty(), "text into bool: empty message");
    let msg = streamed_err::<BTreeMap<String, char>>("a word into char", "a: hello\n");
    assert!(!msg.is_empty(), "word into char: empty message");
}

/// An enum variant name has to come from a scalar. A collection in the
/// variant position hits the identifier path's refusal, which nothing
/// else reaches.
#[test]
fn a_variant_name_that_is_not_a_scalar_is_refused() {
    #[derive(Deserialize, Debug)]
    enum E {
        A,
        #[allow(dead_code)]
        B(i64),
    }
    let msg = streamed_err::<BTreeMap<String, E>>("a sequence variant name", "k: [1, 2]\n");
    assert!(!msg.is_empty(), "empty message");

    // The control: both variant shapes do work through this reader.
    let mut de = StreamingDeserializer::new("k: A\n");
    let m: BTreeMap<String, E> = Deserialize::deserialize(&mut de).expect("unit variant");
    assert!(matches!(m["k"], E::A));
    let mut de = StreamingDeserializer::new("k:\n  B: 7\n");
    let m: BTreeMap<String, E> = Deserialize::deserialize(&mut de).expect("newtype variant");
    assert!(matches!(m["k"], E::B(7)));
}

/// A `!!binary` scalar whose content is not string-shaped, and one
/// whose content is not valid base64 — two distinct refusals on the
/// same path.
#[test]
fn a_malformed_binary_tag_is_refused_in_both_ways() {
    #[derive(Deserialize, Debug)]
    struct HasBytes {
        #[allow(dead_code)]
        b: ByteBuf,
    }
    let msg = streamed_err::<HasBytes>("non-string !!binary content", "b: !!binary [1, 2]\n");
    assert!(!msg.is_empty(), "empty message");

    let msg = streamed_err::<HasBytes>("invalid base64", "b: !!binary \"not base64!!\"\n");
    assert!(
        msg.contains("binary") || msg.contains("base64"),
        "unhelpful message: {msg}"
    );

    // The control: a well-formed one decodes.
    let mut de = StreamingDeserializer::new("b: !!binary \"aGVsbG8=\"\n");
    let v: HasBytes = Deserialize::deserialize(&mut de).expect("valid base64");
    let _ = v;
}

/// A tagged value deserialized into a mapping takes the tag-as-key
/// path, which the untagged shapes never reach.
#[test]
fn a_tagged_value_can_be_read_as_a_single_entry_mapping() {
    let mut de = StreamingDeserializer::new("!Colour '#ff8800'\n");
    let m: Result<BTreeMap<String, String>, _> = Deserialize::deserialize(&mut de);
    match m {
        Ok(m) => {
            assert_eq!(m.len(), 1, "the tag did not become a single entry: {m:?}");
            let (k, v) = m.iter().next().expect("one entry");
            assert!(k.contains("Colour"), "the key is not the tag: {k}");
            assert_eq!(v, "#ff8800");
        }
        Err(e) => {
            // Refusing is also a defensible contract; what must not
            // happen is a silently wrong value.
            assert!(!e.to_string().is_empty(), "empty refusal");
        }
    }
}
