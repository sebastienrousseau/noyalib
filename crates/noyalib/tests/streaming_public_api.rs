// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 1b — `StreamingDeserializer` public API.
//!
//! Verifies that users can construct a `StreamingDeserializer` directly
//! and drive `serde::Deserialize` on it without going through
//! [`noyalib::from_str`]. Also documents (via tests) the subset of YAML
//! it supports and the shape of the errors it produces for unsupported
//! constructs.

use noyalib::{ParserConfig, StreamingDeserializer, Value};
use serde_core::Deserialize as _;
use std::collections::BTreeMap;

// ── Construction ─────────────────────────────────────────────────────────

#[test]
fn new_constructs_from_borrowed_str() {
    let yaml = "key: value\n";
    let mut de = StreamingDeserializer::new(yaml);
    let m: BTreeMap<String, String> = serde_core::Deserialize::deserialize(&mut de).unwrap();
    assert_eq!(m["key"], "value");
}

#[test]
fn with_config_accepts_custom_parser_settings() {
    let yaml = "k: 1\n";
    let config = ParserConfig::strict();
    let mut de = StreamingDeserializer::with_config(yaml, &config);
    let m: BTreeMap<String, i32> = serde_core::Deserialize::deserialize(&mut de).unwrap();
    assert_eq!(m["k"], 1);
}

// ── Typed deserialisation — no Value AST allocation path ─────────────────

#[test]
fn struct_deserialisation_skips_ast() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Server {
        host: String,
        port: u16,
        features: Vec<String>,
    }
    let yaml = "host: localhost\nport: 8080\nfeatures:\n  - auth\n  - api\n";
    let mut de = StreamingDeserializer::new(yaml);
    let s = Server::deserialize(&mut de).unwrap();
    assert_eq!(s.host, "localhost");
    assert_eq!(s.port, 8080);
    assert_eq!(s.features, vec!["auth", "api"]);
}

#[test]
fn nested_collections_work() {
    let yaml = "a:\n  b:\n    c: 42\n";
    let mut de = StreamingDeserializer::new(yaml);
    let m: BTreeMap<String, BTreeMap<String, BTreeMap<String, i32>>> =
        serde_core::Deserialize::deserialize(&mut de).unwrap();
    assert_eq!(m["a"]["b"]["c"], 42);
}

#[test]
fn anchor_alias_handled_natively() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Endpoint {
        host: String,
        port: u16,
    }
    #[derive(Debug, serde::Deserialize)]
    struct Doc {
        primary: Endpoint,
        replica: Endpoint,
    }
    let yaml = r"
primary: &p
  host: db.local
  port: 5432
replica: *p
";
    let mut de = StreamingDeserializer::new(yaml);
    let d = Doc::deserialize(&mut de).unwrap();
    assert_eq!(d.primary, d.replica);
    assert_eq!(d.primary.host, "db.local");
}

#[test]
fn native_merge_key_expansion() {
    // Deserialise into a fully-consuming type (BTreeMap<String, BTreeMap<...>>)
    // so every key is kept — using a struct that ignores `base` would skip
    // an anchored value and trigger the AST fallback.
    let yaml = r"
base: &b
  a: 1
  b: 2
target:
  <<: *b
  c: 3
";
    let mut de = StreamingDeserializer::new(yaml);
    let outer: BTreeMap<String, BTreeMap<String, i64>> =
        serde_core::Deserialize::deserialize(&mut de).unwrap();
    let target = &outer["target"];
    assert_eq!(target["a"], 1);
    assert_eq!(target["b"], 2);
    assert_eq!(target["c"], 3);
}

#[test]
fn native_multi_merge_key_expansion() {
    let yaml = r"
defaults: &d
  host: localhost
  port: 8080
overrides: &o
  port: 9090
  timeout: 30
target:
  <<: [*o, *d]
  debug: true
";
    let mut de = StreamingDeserializer::new(yaml);
    let outer: BTreeMap<String, BTreeMap<String, Value>> =
        serde_core::Deserialize::deserialize(&mut de).unwrap();
    let target = &outer["target"];
    // overrides (*o) comes FIRST in sequence, so it takes precedence for `port`.
    assert_eq!(target["host"].as_str().unwrap(), "localhost");
    assert_eq!(target["port"].as_i64().unwrap(), 9090);
    assert_eq!(target["timeout"].as_i64().unwrap(), 30);
    assert!(target["debug"].as_bool().unwrap());
}

// ── Error cases: unsupported constructs produce errors ──────────────────

#[test]
fn unknown_anchor_produces_error() {
    let yaml = "foo: *missing\n";
    let mut de = StreamingDeserializer::new(yaml);
    let err = <BTreeMap<String, String>>::deserialize(&mut de).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("missing") || msg.contains("unknown"));
}

#[test]
fn invalid_syntax_produces_parse_error() {
    let yaml = "key: [unclosed\nnext: v\n";
    let mut de = StreamingDeserializer::new(yaml);
    let err = <BTreeMap<String, String>>::deserialize(&mut de).unwrap_err();
    // A parse error surfaces — exact wording depends on the parser.
    assert!(!err.to_string().is_empty());
}

// ── Config plumbing: security limits apply ─────────────────────────────

#[test]
fn config_is_plumbed_through_with_config_constructor() {
    // Verify ParserConfig flows through to the scanner by using strict
    // boolean parsing: the streaming path must treat `True` as a string,
    // not a bool, when the strict flag is enabled.
    let yaml = "val: True\n";
    let config = ParserConfig::strict();
    let mut de = StreamingDeserializer::with_config(yaml, &config);
    let m: BTreeMap<String, String> = serde_core::Deserialize::deserialize(&mut de).unwrap();
    assert_eq!(m["val"], "True", "strict mode must pass `True` as string");
}

// ── Debug impl ──────────────────────────────────────────────────────────

#[test]
fn debug_impl_does_not_panic() {
    let de = StreamingDeserializer::new("a: 1");
    let s = format!("{de:?}");
    assert!(s.contains("StreamingDeserializer"));
}

// ── Equivalence: same result as from_str for supported inputs ──────────

#[test]
fn equivalent_to_from_str_for_simple_inputs() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Cfg {
        name: String,
        count: u32,
        items: Vec<String>,
    }
    let yaml = "name: x\ncount: 5\nitems:\n  - a\n  - b\n";

    let via_fromstr: Cfg = noyalib::from_str(yaml).unwrap();
    let mut de = StreamingDeserializer::new(yaml);
    let via_streaming = Cfg::deserialize(&mut de).unwrap();
    assert_eq!(via_fromstr, via_streaming);
}
