// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Implicit keys holding a flow collection are limited to 1024
//! characters; implicit keys stay limited to one line.
//!
//! YAML 1.2.2 §8.2.2 (block mappings) and §7.4.2 (single pairs in a
//! flow sequence) restrict an implicit key to a single line and at most
//! 1024 Unicode characters. The scanner relies on the limit to stop
//! holding back the tokens of a flow collection that can no longer be a
//! key, so a large flow collection at document start is streamed
//! instead of buffered.
//!
//! Scalar keys keep no length limit: a scalar is one token however long
//! it is, so waiting for its `:` costs nothing, and noyalib's serializer
//! writes long string keys as implicit keys. Explicit `?` keys and keys
//! inside a flow mapping have no limit in the spec.

use noyalib::{Mapping, Value, from_str, to_string};

fn parses(src: &str) -> bool {
    from_str::<Value>(src).is_ok()
}

fn err(src: &str) -> String {
    match from_str::<Value>(src) {
        Ok(v) => panic!("expected an error, got {v:?}"),
        Err(e) => e.to_string(),
    }
}

/// A flow-sequence key of exactly `chars` characters: `[a, a, …, a]`.
fn seq_key(chars: usize) -> String {
    assert!(chars >= 3 && chars % 3 == 0);
    format!("[{}a]", "a, ".repeat(chars / 3 - 1))
}

#[test]
fn flow_collection_block_key_of_1024_characters_is_accepted() {
    // 1023 characters of key plus the `:` position is the limit.
    let key = format!("{}{}", seq_key(1020), " ".repeat(4));
    assert!(parses(&format!("{key}: v\n")));
}

#[test]
fn flow_collection_block_key_over_1024_characters_is_rejected() {
    let key = format!("{}{}", seq_key(1020), " ".repeat(5));
    assert!(!parses(&format!("{key}: v\n")));
    let msg = err(&format!("{}: v\n", seq_key(1035)));
    assert!(msg.contains("longer than 1024 characters"), "{msg}");
    let msg = err(&format!("{}: v\n", seq_key(30_000)));
    assert!(msg.contains("longer than 1024 characters"), "{msg}");
}

#[test]
fn limit_counts_characters_not_bytes() {
    // `[a, "…"]`: 7 characters around the quoted text.
    let key = |n: usize, c: &str| format!("[a, \"{}\"]", c.repeat(n));
    assert!(parses(&format!("{}: v\n", key(1017, "é"))));
    assert!(!parses(&format!("{}: v\n", key(1018, "é"))));
    assert!(parses(&format!("{}: v\n", key(1017, "😀"))));
    assert!(!parses(&format!("{}: v\n", key(1018, "😀"))));
}

#[test]
fn flow_sequence_single_pair_collection_key_is_limited() {
    assert!(parses(&format!("[{}: v]\n", seq_key(900))));
    assert!(!parses(&format!("[{}: v]\n", seq_key(1050))));
    assert!(!parses(&format!("[{}: v]\n", seq_key(30_000))));
}

#[test]
fn long_scalar_keys_are_still_accepted() {
    for key in [
        "k".repeat(5000),
        format!("\"{}\"", "k".repeat(5000)),
        format!("&a !!str {}", "k".repeat(5000)),
    ] {
        assert!(
            parses(&format!("{key}: v\n")),
            "block key of {} bytes",
            key.len()
        );
        assert!(
            parses(&format!("[{key}: v]\n")),
            "flow pair of {} bytes",
            key.len()
        );
    }
}

#[test]
fn serialized_long_string_keys_round_trip() {
    let mut map = Mapping::new();
    let _ = map.insert("k".repeat(5000), Value::from("v"));
    let _ = map.insert(format!("[{}]", "a,".repeat(3000)), Value::from(1));
    let value = Value::Mapping(map);
    let yaml = to_string(&value).expect("serialize");
    let back: Value = from_str(&yaml).expect("re-parse serialized long keys");
    assert_eq!(back, value);
}

#[test]
fn flow_mapping_and_explicit_keys_are_not_limited() {
    let key = seq_key(30_000);
    let v: Value = from_str(&format!("{{{key}: v}}\n")).expect("flow mapping key");
    assert_eq!(v.as_mapping().map(Mapping::len), Some(1));
    let v: Value = from_str(&format!("? {key}\n: v\n")).expect("explicit key");
    assert_eq!(v.as_mapping().map(Mapping::len), Some(1));
}

#[test]
fn multi_line_block_key_keeps_its_error() {
    // Long enough that the queue has handed out and compacted the key's
    // tokens before the `:` arrives.
    let src = format!("[{}a]: v\n", "a,\n".repeat(2000));
    let msg = err(&src);
    assert!(msg.contains("cannot span multiple lines"), "{msg}");
}

#[test]
fn long_collections_without_a_colon_are_values() {
    let v: Value = from_str(&format!("[{}a]\n", "a,".repeat(5000))).expect("one line");
    assert_eq!(v.as_sequence().map(Vec::len), Some(5001));
    let v: Value = from_str(&format!("[{}a]\n", "a,\n".repeat(5000))).expect("many lines");
    assert_eq!(v.as_sequence().map(Vec::len), Some(5001));
    let json = format!(
        "{{{}\"z\": 0}}\n",
        (0..3000)
            .map(|i| format!("\"k{i}\": {i},\n"))
            .collect::<String>()
    );
    let v: Value = from_str(&json).expect("multi-line JSON object");
    assert_eq!(v.as_mapping().map(Mapping::len), Some(3001));
}

#[test]
fn colon_on_the_next_line_still_starts_a_new_pair() {
    // 6M2F with a value longer than any implicit key: `: *a` on the next
    // line is a new pair with an empty key, not a key for the long value.
    let long = "b".repeat(5000);
    let v: Value = from_str(&format!("? &a a\n: &b {long}\n: *a\n")).expect("6M2F shape");
    let map = v.as_mapping().expect("mapping");
    assert_eq!(map.len(), 2);
    assert_eq!(v.get("a").and_then(Value::as_str), Some(long.as_str()));
}
