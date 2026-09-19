//! `Number::Display` float formatting, tested against the serializer.
//!
//! Refs #348: `Display` used to route whole floats through `f64`'s
//! default formatting, which drops the trailing `.0` (`4.0` printed as
//! `"4"`) and disagrees with the emitter (`noyalib::to_string`), which
//! already wrote `4.0`, `.inf`, `-.inf`, `.nan`. `Display` now shares
//! the serializer's float formatter, so the two always agree.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Number, Value, from_str};

#[test]
fn whole_float_keeps_trailing_zero() {
    assert_eq!(Number::from(4.0_f64).to_string(), "4.0");
}

#[test]
fn positive_infinity_prints_yaml_form() {
    assert_eq!(Number::from(f64::INFINITY).to_string(), ".inf");
}

#[test]
fn negative_infinity_prints_yaml_form() {
    assert_eq!(Number::from(f64::NEG_INFINITY).to_string(), "-.inf");
}

#[test]
fn nan_prints_yaml_form() {
    assert_eq!(Number::from(f64::NAN).to_string(), ".nan");
}

#[test]
fn integer_display_is_unchanged() {
    assert_eq!(Number::from(7_i64).to_string(), "7");
}

#[test]
fn non_whole_float_display_is_unchanged() {
    assert_eq!(Number::from(1.5_f64).to_string(), "1.5");
}

/// `Display` output must re-parse as a `Number` of the same variant and
/// value — the whole point of matching the serializer's formatter is
/// that `Display` output round-trips through the parser too.
fn assert_round_trips(n: Number) {
    let printed = n.to_string();
    let reparsed = from_str::<Value>(&printed)
        .unwrap_or_else(|e| panic!("`{printed}` (from {n:?}) failed to re-parse as a Value: {e}"));
    match (n, reparsed) {
        (Number::Integer(a), Value::Number(Number::Integer(b))) => assert_eq!(a, b),
        (Number::Float(a), Value::Number(Number::Float(b))) => {
            if a.is_nan() {
                assert!(b.is_nan(), "expected NaN, got {b} (printed {printed:?})");
            } else {
                assert_eq!(a, b, "printed {printed:?}");
            }
        }
        (n, reparsed) => {
            panic!("variant mismatch: {n:?} printed as {printed:?}, reparsed as {reparsed:?}")
        }
    }
}

#[test]
fn round_trip_whole_float() {
    assert_round_trips(Number::from(4.0_f64));
}

#[test]
fn round_trip_positive_infinity() {
    assert_round_trips(Number::from(f64::INFINITY));
}

#[test]
fn round_trip_negative_infinity() {
    assert_round_trips(Number::from(f64::NEG_INFINITY));
}

#[test]
fn round_trip_nan() {
    assert_round_trips(Number::from(f64::NAN));
}

#[test]
fn round_trip_integer() {
    assert_round_trips(Number::from(7_i64));
}

#[test]
fn round_trip_non_whole_float() {
    assert_round_trips(Number::from(1.5_f64));
}
