// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Typed rejections from `from_str` carry the field path (#353).
//!
//! `server:\n  port: "x"` must report `server.port: invalid type: …`,
//! not just a line and column — the path is what an operator searches
//! the config for, and it survives includes and environment expansion
//! that make positions approximate. The path prefixes the message;
//! `location()` is unchanged. Errors at the document root, and errors
//! from `from_value` (no source, no spans), stay exactly as they were.

#![cfg(feature = "std")]

use noyalib::{Value, from_str, from_value};

#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct Server {
    port: u16,
}

#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct Cfg {
    server: Server,
}

#[test]
fn nested_field_error_names_the_dotted_path() {
    let err = from_str::<Cfg>("server:\n  port: \"x\"\n").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("server.port: "), "path missing: {msg}");
    assert!(msg.contains("invalid type"), "wording lost: {msg}");
    // The location still points at the offending scalar.
    let loc = err.location().expect("location survives the prefix");
    assert_eq!((loc.line(), loc.column()), (2, 9), "got: {msg}");
}

#[test]
fn sequence_index_is_bracketed() {
    #[derive(serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct Doc {
        items: Vec<i32>,
    }
    let err = from_str::<Doc>("items:\n  - 1\n  - x\n").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("items[1]: "), "path missing: {msg}");
}

#[test]
fn root_sequence_leads_with_the_index() {
    let err = from_str::<Vec<i32>>("- 1\n- x\n").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("[1]: "), "path missing: {msg}");
}

#[test]
fn deeply_nested_mixed_path() {
    #[derive(serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct Inner {
        name: String,
        count: u8,
    }
    #[derive(serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct Outer {
        groups: Vec<Inner>,
    }
    #[derive(serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct Doc {
        a: Outer,
    }
    let yaml =
        "a:\n  groups:\n    - name: ok\n      count: 1\n    - name: bad\n      count: many\n";
    let err = from_str::<Doc>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("a.groups[1].count: "), "path missing: {msg}");
}

#[test]
fn root_error_stays_unprefixed() {
    let err = from_str::<i32>("hello").unwrap_err();
    let msg = err.to_string();
    // Message text begins right after the location preamble — no
    // path, no stray colon.
    assert!(
        msg.contains(": invalid type") || msg.contains(": type mismatch"),
        "got: {msg}"
    );
    assert!(!msg.contains(": : "), "double prefix: {msg}");
}

#[test]
fn from_value_errors_are_untouched() {
    // No source text, no spans: the path machinery must not fire.
    let v: Value = from_str("port: \"x\"\n").unwrap();
    let err = from_value::<Server>(&v).unwrap_err();
    assert!(err.location().is_none());
    assert!(
        !err.to_string().contains("port: invalid"),
        "unexpected prefix without a source: {err}"
    );
}

#[test]
fn swallowed_probe_error_does_not_leak_into_the_next_parse() {
    // An untagged enum tries a failing variant (which records a
    // failing node) and then succeeds; the recorded node must not
    // decorate an unrelated later error.
    #[derive(serde::Deserialize, Debug)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum Loose {
        Num(u64),
        Text(String),
    }
    #[derive(serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct Probe {
        field: Loose,
    }
    let ok: Probe = from_str("field: definitely-text\n").unwrap();
    drop(ok);
    // Next parse fails at the root: no path prefix expected.
    let err = from_str::<i32>("hello").unwrap_err();
    assert!(
        !err.to_string().contains("field"),
        "leaked path from the previous parse: {err}"
    );
}

#[test]
fn streaming_wording_keeps_the_path_prefix() {
    // The default path tries the streaming walker first; its typed
    // rejection (serde's own wording) falls through to the AST pass
    // that finds both the location and the path. The final message
    // carries all three: path, streaming wording, location.
    let err = from_str::<Cfg>("server:\n  port: \"x\"\n").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("server.port: "), "path: {msg}");
    assert!(msg.contains("expected u16"), "streaming wording: {msg}");
    assert!(msg.contains("line 2"), "location: {msg}");
}
