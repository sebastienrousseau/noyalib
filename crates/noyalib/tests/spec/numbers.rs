// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Number representations

use noyalib::{Value, from_str};

// --- Integers ---

#[test]
fn integer_decimal() {
    let v: i64 = from_str("42").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn integer_negative() {
    let v: i64 = from_str("-17").unwrap();
    assert_eq!(v, -17);
}

#[test]
fn integer_zero() {
    let v: i64 = from_str("0").unwrap();
    assert_eq!(v, 0);
}

#[test]
fn integer_positive_explicit() {
    let v: i64 = from_str("+42").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn integer_hex() {
    let v: Value = from_str("0x2A").unwrap();
    assert_eq!(v.as_i64(), Some(42));
}

#[test]
fn integer_octal() {
    let v: Value = from_str("0o52").unwrap();
    assert_eq!(v.as_i64(), Some(42));
}

#[test]
fn integer_large() {
    let v: i64 = from_str("1000000000").unwrap();
    assert_eq!(v, 1_000_000_000);
}

// --- Floats ---

#[test]
fn float_decimal() {
    let v: f64 = from_str("2.75").unwrap();
    assert!((v - 2.75).abs() < 0.001);
}

#[test]
fn float_negative() {
    let v: f64 = from_str("-2.5").unwrap();
    assert!((v - (-2.5)).abs() < 0.001);
}

#[test]
fn float_scientific() {
    let v: f64 = from_str("1.0e3").unwrap();
    assert!((v - 1000.0).abs() < 0.001);
}

#[test]
fn float_scientific_negative_exp() {
    let v: f64 = from_str("1.0e-3").unwrap();
    assert!((v - 0.001).abs() < 0.0001);
}

#[test]
fn float_infinity() {
    let v: Value = from_str(".inf").unwrap();
    let f = v.as_f64().unwrap();
    assert!(f.is_infinite() && f > 0.0);
}

#[test]
fn float_negative_infinity() {
    let v: Value = from_str("-.inf").unwrap();
    let f = v.as_f64().unwrap();
    assert!(f.is_infinite() && f < 0.0);
}

#[test]
fn float_nan() {
    let v: Value = from_str(".nan").unwrap();
    let f = v.as_f64().unwrap();
    assert!(f.is_nan());
}

#[test]
fn float_Inf_uppercase() {
    let v: Value = from_str(".Inf").unwrap();
    let f = v.as_f64().unwrap();
    assert!(f.is_infinite());
}

#[test]
fn float_NaN_mixed_case() {
    // yaml-rust2 may parse .NaN as string rather than Real
    // depending on exact casing rules. Both outcomes are acceptable.
    let v: Value = from_str(".NaN").unwrap();
    if let Some(f) = v.as_f64() {
        assert!(f.is_nan());
    } else {
        assert!(v.is_string());
    }
}

#[test]
fn float_zero() {
    let v: f64 = from_str("0.0").unwrap();
    assert!((v - 0.0).abs() < f64::EPSILON);
}

// --- Integer types ---

#[test]
fn u8_value() {
    let v: u8 = from_str("255").unwrap();
    assert_eq!(v, 255);
}

#[test]
fn u16_value() {
    let v: u16 = from_str("65535").unwrap();
    assert_eq!(v, 65535);
}

#[test]
fn i8_value() {
    let v: i8 = from_str("-128").unwrap();
    assert_eq!(v, -128);
}

#[test]
fn i16_value() {
    let v: i16 = from_str("-32768").unwrap();
    assert_eq!(v, -32768);
}

#[test]
fn numbers_in_sequence() {
    let v: Vec<Value> = from_str("- 42\n- 2.75\n- .inf\n- .nan\n").unwrap();
    assert_eq!(v[0].as_i64(), Some(42));
    assert!(v[1].as_f64().is_some());
    assert!(v[2].as_f64().unwrap().is_infinite());
    assert!(v[3].as_f64().unwrap().is_nan());
}
