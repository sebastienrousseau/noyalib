// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 3 — `Validated<T>` garde bridge tests.
//!
//! Verifies that declarative `#[garde(...)]` attributes run after
//! deserialisation and surface field-level failures in the noyalib
//! error path.

#![cfg(feature = "garde")]

use garde::Validate;
use noyalib::{Validated, from_str};
use serde::{Deserialize, Serialize};

// ── Basic: passing validation returns Validated(T) ──────────────────────

#[derive(Debug, Deserialize, Serialize, Validate)]
struct Server {
    #[garde(length(min = 1, max = 64))]
    host: String,
    #[garde(range(min = 1024, max = 65535))]
    port: u16,
}

#[test]
fn valid_input_parses_cleanly() {
    let yaml = "host: db.local\nport: 5432\n";
    let s: Validated<Server> = from_str(yaml).unwrap();
    assert_eq!(s.0.host, "db.local");
    assert_eq!(s.0.port, 5432);
}

#[test]
fn deref_gives_access_to_inner() {
    let yaml = "host: a\nport: 8080\n";
    let s: Validated<Server> = from_str(yaml).unwrap();
    // Deref target is Server.
    assert_eq!(s.host, "a");
    assert_eq!(s.port, 8080);
}

#[test]
fn into_inner_consumes_wrapper() {
    let yaml = "host: a\nport: 8080\n";
    let s: Validated<Server> = from_str(yaml).unwrap();
    let inner: Server = s.into_inner();
    assert_eq!(inner.host, "a");
}

// ── Field-level validation failures ──────────────────────────────────────

#[test]
fn empty_host_rejected_with_field_path() {
    let yaml = "host: \"\"\nport: 8080\n";
    let err = from_str::<Validated<Server>>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("validation failed"), "got: {msg}");
    assert!(msg.contains("host"), "expected field name in: {msg}");
}

#[test]
fn port_below_range_rejected() {
    let yaml = "host: ok\nport: 80\n";
    let err = from_str::<Validated<Server>>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("port"), "expected field name in: {msg}");
}

#[test]
fn multiple_errors_reported_together() {
    let yaml = "host: \"\"\nport: 80\n";
    let err = from_str::<Validated<Server>>(yaml).unwrap_err();
    let msg = err.to_string();
    // Both fields should be mentioned (garde reports all failures).
    assert!(msg.contains("host") && msg.contains("port"), "got: {msg}");
}

// ── Nested validation ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
struct Database {
    #[garde(length(min = 1))]
    name: String,
    #[garde(dive)]
    primary: Server,
}

#[test]
fn nested_validation_reports_dotted_path() {
    let yaml = "name: prod\nprimary:\n  host: \"\"\n  port: 8080\n";
    let err = from_str::<Validated<Database>>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("primary") && msg.contains("host"),
        "expected nested path, got: {msg}"
    );
}

// ── Collection validation ────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
struct Cluster {
    #[garde(length(min = 1, max = 10))]
    nodes: Vec<String>,
}

#[test]
fn empty_collection_rejected() {
    let yaml = "nodes: []\n";
    let err = from_str::<Validated<Cluster>>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("nodes"), "got: {msg}");
}

#[test]
fn valid_collection_parses() {
    let yaml = "nodes:\n  - a\n  - b\n  - c\n";
    let c: Validated<Cluster> = from_str(yaml).unwrap();
    assert_eq!(c.0.nodes.len(), 3);
}

// ── Serialisation is transparent (no validation on way out) ──────────────

#[test]
fn serialize_passes_through_inner() {
    use noyalib::to_string;
    let s: Validated<Server> = from_str("host: a\nport: 8080\n").unwrap();
    let yaml = to_string(&s).unwrap();
    // Output is the same as serialising Server directly.
    assert!(yaml.contains("host: a"));
    assert!(yaml.contains("port: 8080"));
}

// ── From<T> conversion ──────────────────────────────────────────────────

#[test]
fn from_impl_wraps_without_validating() {
    // From<T> is documented as a construction shortcut; it does NOT validate,
    // only Deserialize does. This avoids surprising API: you opt into
    // validation via deserialisation.
    let s = Server {
        host: "".to_string(),
        port: 1,
    };
    let wrapped: Validated<Server> = s.into();
    assert_eq!(wrapped.host, "");
}
