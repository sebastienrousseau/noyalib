// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Implicit keys are limited to one line and 1024 characters.
//!
//! YAML 1.2.2 §8.2.2 (block mappings) and §7.4.2 (single pairs in a
//! flow sequence) restrict an implicit key to a single line and at most
//! 1024 Unicode characters. Explicit `?` keys and keys inside a flow
//! mapping have no such limit. The scanner relies on the limit to stop
//! holding back tokens for a key that can no longer complete, so a large
//! flow collection at document start is streamed instead of buffered.

use noyalib::{Value, from_str};

fn parses(src: &str) -> bool {
    from_str::<Value>(src).is_ok()
}

fn err(src: &str) -> String {
    match from_str::<Value>(src) {
        Ok(v) => panic!("expected an error, got {v:?}"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn block_key_of_1024_characters_is_accepted() {
    let key = "k".repeat(1024);
    let v: Value = from_str(&format!("{key}: v\n")).expect("1024 characters is the limit");
    assert_eq!(v.get(key.as_str()).and_then(Value::as_str), Some("v"));
}

#[test]
fn block_key_of_1025_characters_is_rejected() {
    let msg = err(&format!("{}: v\n", "k".repeat(1025)));
    assert!(msg.contains("longer than 1024 characters"), "{msg}");
}

#[test]
fn limit_counts_characters_not_bytes() {
    // 1024 two-byte characters: 2048 bytes, still within the limit.
    assert!(parses(&format!("{}: v\n", "é".repeat(1024))));
    assert!(!parses(&format!("{}: v\n", "é".repeat(1025))));
    // Four-byte characters: 4096 bytes is exactly 1024 characters.
    assert!(parses(&format!("{}: v\n", "😀".repeat(1024))));
    assert!(!parses(&format!("{}: v\n", "😀".repeat(1025))));
}

#[test]
fn quoted_and_flow_block_keys_are_limited_too() {
    assert!(parses(&format!("\"{}\": v\n", "k".repeat(1022))));
    assert!(!parses(&format!("\"{}\": v\n", "k".repeat(1023))));
    assert!(!parses(&format!("[{}a]: v\n", "a,".repeat(3000))));
}

#[test]
fn flow_sequence_single_pair_key_is_limited() {
    assert!(parses(&format!("[{}: v]\n", "k".repeat(1024))));
    assert!(!parses(&format!("[{}: v]\n", "k".repeat(1025))));
    assert!(!parses(&format!("[{}: v]\n", "k".repeat(10_000))));
}

#[test]
fn flow_mapping_and_explicit_keys_are_not_limited() {
    let key = "k".repeat(10_000);
    let v: Value = from_str(&format!("{{{key}: v}}\n")).expect("flow mapping key");
    assert_eq!(v.get(key.as_str()).and_then(Value::as_str), Some("v"));
    let v: Value = from_str(&format!("? {key}\n: v\n")).expect("explicit key");
    assert_eq!(v.get(key.as_str()).and_then(Value::as_str), Some("v"));
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
    assert_eq!(v.as_mapping().map(|m| m.len()), Some(3001));
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
