//! Coverage for the `lossless-u64` `Number::Unsigned` arms of
//! `de/deserializer.rs` and the flow-collection / token paths of
//! `cst/format.rs`.
//!
//! These branches existed but were unexercised: the crate's `Unsigned`
//! variant was only ever deserialized *into* `u64`, so the widening
//! (`Unsigned` -> `i64`/`f64`) and the overflow rejection were never
//! driven. Likewise `cst::format` was only tested on block collections,
//! leaving the verbatim flow-mapping / flow-sequence arms cold.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

// Used on both feature legs: the `Unsigned` tests are `lossless-u64`
// only, but the negative-integer mirror test below exercises
// `Number::Integer` and runs with default features too. Keeping the
// import ungated is what lets every test use short paths — the crate
// denies `unnecessary_qualification`, so a `cfg`-gated import would
// force `noyalib::`-prefixed paths that then become redundant the
// moment the feature is on.
use noyalib::{Number, Value, from_value};

// ---------------------------------------------------------------------
// de/deserializer.rs — `Number::Unsigned` arms
// ---------------------------------------------------------------------

/// `deserialize_i64` on an `Unsigned` that *fits* in `i64` takes the
/// `Ok(n) => visit_i64` arm.
#[cfg(feature = "lossless-u64")]
#[test]
fn unsigned_within_i64_range_deserializes_to_i64() {
    let v = Value::Number(Number::Unsigned(42));
    let got: i64 = from_value(&v).unwrap();
    assert_eq!(got, 42);
}

/// `deserialize_i64` on an `Unsigned` too large for `i64` takes the
/// `Err(_) => TypeMismatch` arm rather than wrapping around.
///
/// The message is asserted precisely, not by substring: reporting both
/// sides of the mismatch as "integer" (which `type_name` would do, since
/// it collapses `Integer` and `Unsigned` onto one label) produces
/// "expected integer, found integer" and tells the caller nothing. The
/// value itself must appear so the error is actionable.
#[cfg(feature = "lossless-u64")]
#[test]
fn unsigned_above_i64_max_rejected_for_i64() {
    let v = Value::Number(Number::Unsigned(u64::MAX));
    let err = from_value::<i64>(&v).unwrap_err();

    assert!(
        matches!(err, noyalib::Error::TypeMismatch { .. }),
        "expected TypeMismatch, got: {err:?}"
    );
    assert_eq!(
        err.to_string(),
        format!(
            "type mismatch: expected signed integer (i64), \
             found unsigned integer {}, above i64::MAX",
            u64::MAX
        )
    );
}

/// The mirror case: a *negative* `Integer` deserialised into `u64` must
/// say the sign is the problem, not merely "found integer".
#[test]
fn negative_integer_rejected_for_u64_names_the_sign() {
    let v = Value::Number(Number::Integer(-1));
    let err = from_value::<u64>(&v).unwrap_err();

    assert_eq!(
        err.to_string(),
        "type mismatch: expected unsigned integer, found negative integer -1"
    );
}

/// The boundary itself: `i64::MAX` as an `Unsigned` must still convert.
#[cfg(feature = "lossless-u64")]
#[test]
fn unsigned_at_i64_max_boundary_converts() {
    let v = Value::Number(Number::Unsigned(i64::MAX as u64));
    let got: i64 = from_value(&v).unwrap();
    assert_eq!(got, i64::MAX);
}

/// `deserialize_f64` widens an `Unsigned` through the `as f64` arm.
#[cfg(feature = "lossless-u64")]
#[test]
fn unsigned_widens_to_f64() {
    let v = Value::Number(Number::Unsigned(u64::MAX));
    let got: f64 = from_value(&v).unwrap();
    assert_eq!(got, u64::MAX as f64);
}

/// `deserialize_any` (driven by an untyped `Value` target) reaches the
/// `Unsigned` arm of the any-dispatcher.
#[cfg(feature = "lossless-u64")]
#[test]
fn unsigned_round_trips_through_deserialize_any() {
    let v = Value::Number(Number::Unsigned(u64::MAX));
    let got: Value = from_value(&v).unwrap();
    assert_eq!(got, v);
}

/// A newtype struct wrapping an unsigned drives
/// `deserialize_newtype_struct` -> `visit_newtype_struct`.
#[cfg(feature = "lossless-u64")]
#[test]
fn unsigned_through_newtype_struct() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Wrapper(u64);

    let v = Value::Number(Number::Unsigned(u64::MAX));
    let got: Wrapper = from_value(&v).unwrap();
    assert_eq!(got, Wrapper(u64::MAX));
}

// ---------------------------------------------------------------------
// cst/format.rs — flow collections and token passthrough
// ---------------------------------------------------------------------

/// A flow mapping takes the `FlowMapping => write_verbatim` arm: the
/// formatter must not reflow it into block style.
///
/// "Verbatim" is asserted as byte equality, not as "the braces are still
/// in there somewhere" — a formatter that rewrote `{x: 1, y: 2}` into
/// `{ x: 1 , y: 2 }` would satisfy a containment check while breaking
/// exactly the property this arm exists to guarantee.
#[test]
fn format_preserves_flow_mapping_verbatim() {
    let src = "a: {x: 1, y: 2}\n";
    assert_eq!(noyalib::cst::format(src).unwrap(), src);
}

/// A flow sequence takes the `FlowSequence => write_verbatim` arm.
#[test]
fn format_preserves_flow_sequence_verbatim() {
    let src = "a: [1, 2, 3]\n";
    assert_eq!(noyalib::cst::format(src).unwrap(), src);
}

/// Verbatim means verbatim: interior spacing a block formatter would
/// normalise is carried through a flow collection untouched.
#[test]
fn format_does_not_normalise_inside_a_flow_collection() {
    let src = "a: {x:   1}\n";
    assert_eq!(
        noyalib::cst::format(src).unwrap(),
        src,
        "flow interior must not be re-spaced"
    );
}

/// A block sequence whose items are nested nodes drives the
/// `GreenChild::Node` recursion inside `format_sequence_item`, while the
/// scalar items drive `GreenChild::Token`.
#[test]
fn format_sequence_items_with_nested_nodes_and_tokens() {
    let src = "list:\n  - plain\n  - nested:\n      inner: 1\n  - [1, 2]\n";
    let out = noyalib::cst::format(src).unwrap();
    // Already-canonical input must survive a format pass unchanged —
    // stronger than checking that individual substrings survived, which
    // would pass even if the nesting were flattened.
    assert_eq!(out, src);
}

/// Formatting must be idempotent — a second pass over already-formatted
/// output re-walks the same node/token arms and changes nothing.
#[test]
fn format_is_idempotent_across_flow_and_block() {
    let src = "a: {x: 1}\nb:\n  - 1\n  - [2, 3]\nc: plain\n";
    let once = noyalib::cst::format(src).unwrap();
    let twice = noyalib::cst::format(&once).unwrap();
    assert_eq!(once, twice, "format should be idempotent");
}
