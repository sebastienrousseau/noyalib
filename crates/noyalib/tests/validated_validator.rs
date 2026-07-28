// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Tests for the `validator` crate bridge (`ValidatedValidator<T>`).
//!
//! Mirrors the garde bridge tests but targets the `validator` feature.

#![cfg(feature = "validator")]

use noyalib::{from_str, to_string, validated::ValidatedValidator};
use std::collections::BTreeMap;
use validator::Validate;

#[derive(Debug, serde::Deserialize, serde::Serialize, Validate)]
struct Server {
    #[validate(length(min = 1, max = 64))]
    host: String,
    #[validate(range(min = 1024, max = 65535))]
    port: u16,
}

// ── Valid input parses cleanly ──────────────────────────────────────────

#[test]
fn valid_input_parses_cleanly() {
    let yaml = "host: db.local\nport: 5432\n";
    let s: ValidatedValidator<Server> = from_str(yaml).unwrap();
    assert_eq!(s.0.host, "db.local");
    assert_eq!(s.0.port, 5432);
}

#[test]
fn deref_gives_access_to_inner() {
    let yaml = "host: a\nport: 8080\n";
    let s: ValidatedValidator<Server> = from_str(yaml).unwrap();
    // Deref target is Server.
    assert_eq!(s.host, "a");
    assert_eq!(s.port, 8080);
}

#[test]
fn into_inner_consumes_wrapper() {
    let yaml = "host: a\nport: 8080\n";
    let s: ValidatedValidator<Server> = from_str(yaml).unwrap();
    let inner: Server = s.into_inner();
    assert_eq!(inner.host, "a");
    assert_eq!(inner.port, 8080);
}

// ── Field-level validation failures surface with field names ────────────

#[test]
fn empty_host_rejected_with_field_path() {
    let yaml = "host: \"\"\nport: 8080\n";
    let err = from_str::<ValidatedValidator<Server>>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("validation failed"), "got: {msg}");
    assert!(msg.contains("host"), "expected field name in: {msg}");
}

#[test]
fn port_below_range_rejected() {
    let yaml = "host: ok\nport: 80\n";
    let err = from_str::<ValidatedValidator<Server>>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("port"), "expected field name in: {msg}");
}

#[test]
fn port_above_range_rejected() {
    let yaml = "host: ok\nport: 70000\n";
    let err = from_str::<ValidatedValidator<Server>>(yaml);
    // port is u16 so 70000 may fail serde's own range check first,
    // but either deserialisation or validation must fail.
    assert!(err.is_err());
}

#[test]
fn multiple_errors_reported_together() {
    let yaml = "host: \"\"\nport: 80\n";
    let err = from_str::<ValidatedValidator<Server>>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("host") && msg.contains("port"), "got: {msg}");
}

// ── Collection validation ────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, serde::Serialize, Validate)]
struct Cluster {
    #[validate(length(min = 1, max = 10))]
    nodes: Vec<String>,
}

#[test]
fn empty_collection_rejected() {
    let yaml = "nodes: []\n";
    let err = from_str::<ValidatedValidator<Cluster>>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("nodes"), "got: {msg}");
}

#[test]
fn valid_collection_parses() {
    let yaml = "nodes:\n  - a\n  - b\n  - c\n";
    let c: ValidatedValidator<Cluster> = from_str(yaml).unwrap();
    assert_eq!(c.0.nodes.len(), 3);
}

// ── Serialisation is transparent (no validation on way out) ─────────────

#[test]
fn serialize_passes_through_inner() {
    let s: ValidatedValidator<Server> = from_str("host: a\nport: 8080\n").unwrap();
    let yaml = to_string(&s).unwrap();
    assert!(yaml.contains("host: a"));
    assert!(yaml.contains("port: 8080"));
}

// ── From<T> conversion (opt-in construction without validation) ─────────

#[test]
fn from_impl_wraps_without_validating() {
    // `From<T>` is a shortcut that does NOT validate — only `Deserialize`
    // runs the validator. Mirrors the garde wrapper's policy.
    let s = Server {
        host: String::new(),
        port: 1,
    };
    let wrapped: ValidatedValidator<Server> = s.into();
    assert_eq!(wrapped.host, "");
}

// ── Nested struct with Validate-derived inner ──────────────────────────

#[test]
fn roundtrip_value_equivalence() {
    // Read -> modify -> write -> read loop preserves the values.
    let yaml = "host: db.local\nport: 5432\n";
    let s: ValidatedValidator<Server> = from_str(yaml).unwrap();
    let out = to_string(&s).unwrap();
    let s2: ValidatedValidator<Server> = from_str(&out).unwrap();
    assert_eq!(s.0.host, s2.0.host);
    assert_eq!(s.0.port, s2.0.port);
}

// ── Does not interfere with non-validator types ─────────────────────────

#[test]
fn sibling_type_without_wrapper_works() {
    // Non-validator Server variant must still parse normally.
    #[derive(serde::Deserialize)]
    struct Plain {
        #[allow(dead_code)]
        host: String,
    }
    let yaml = "host: ok\n";
    let p: Plain = from_str(yaml).unwrap();
    let _ = p.host;
    // And a BTreeMap still works too.
    let m: BTreeMap<String, String> = from_str("a: b\n").unwrap();
    assert_eq!(m["a"], "b");
}
