// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed deserialization errors carry the source location when the input
//! came from text (`from_str`), and stay unlocated when it did not
//! (`from_value`). Covers the two error shapes that reach
//! `Deserializer::wrap_err` besides `Error::Deserialize`: the typed
//! `TypeMismatch` arms and serde visitor errors (`Error::Custom`) on the
//! `deserialize_any` path that `#[serde(flatten)]` takes.

use noyalib::{Error, Value, from_str, from_value};

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Server {
    port: u16,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Config {
    server: Server,
}

/// A `u16` field fed a string takes `deserialize_u64`'s catch-all arm
/// (`TypeMismatch`); from text, the error must name where.
#[test]
fn type_mismatch_from_str_carries_the_value_location() {
    let err = from_str::<Config>("server:\n  port: \"not-a-number\"\n").unwrap_err();
    let location = err
        .location()
        .unwrap_or_else(|| panic!("typed error from text must carry a location, got: {err:?}"));
    assert_eq!(
        location.line(),
        2,
        "the offending scalar is on line 2: {err}"
    );
    let text = err.to_string();
    assert!(
        text.contains("line 2") && text.contains("column"),
        "Display must include the position: {text}"
    );
    // The streaming path's wording is serde's own (`invalid type: string
    // "not-a-number", expected u16`); the AST path, which supplied the
    // location, would have said `type mismatch: expected unsigned integer,
    // found string`. The message must not depend on which path ran.
    assert!(
        text.contains("invalid type") && text.contains("expected u16"),
        "serde's wording from the streaming path is kept: {text}"
    );
    assert!(
        !text.contains("type mismatch"),
        "the AST path's wording must not replace the streaming message: {text}"
    );
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Inner {
    port: u16,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Outer {
    #[serde(flatten)]
    inner: Inner,
}

/// Through `#[serde(flatten)]` serde buffers the map and re-dispatches via
/// `deserialize_any`, so the failure is raised by serde's own visitor as
/// `Error::Custom("invalid type: ...")`. It must be located too. The
/// position is the enclosing mapping's start (line 1 here), not the
/// scalar's: serde buffers flattened content into `Content` before
/// re-dispatching it, so the deserializer that sees the error is the one
/// for the mapping being flattened.
#[test]
fn visitor_error_on_the_any_path_carries_the_enclosing_mapping_location() {
    let err = from_str::<Outer>("name: x\nport: \"not-a-number\"\n").unwrap_err();
    let location = err
        .location()
        .unwrap_or_else(|| panic!("visitor error from text must carry a location, got: {err:?}"));
    assert_eq!(
        location.line(),
        1,
        "the flattened mapping starts on line 1: {err}"
    );
    let text = err.to_string();
    assert!(
        text.contains("line 1") && text.contains("column"),
        "Display must include the position: {text}"
    );
    assert!(
        text.contains("invalid type") && text.contains("expected u16"),
        "serde's own message is kept: {text}"
    );
}

/// Without a span context there is nothing to attach, and the variant is
/// unchanged, so callers matching on `TypeMismatch` after `from_value`
/// keep working.
#[test]
fn from_value_errors_stay_unlocated_and_keep_their_variant() {
    let err = from_value::<u16>(&Value::String("not-a-number".into())).unwrap_err();
    assert!(
        matches!(err, Error::TypeMismatch { .. }),
        "expected TypeMismatch, got: {err:?}"
    );
    assert!(err.location().is_none());
}
