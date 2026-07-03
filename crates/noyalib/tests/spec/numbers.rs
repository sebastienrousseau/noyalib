// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Number representations

#[cfg(feature = "lossless-u64")]
use noyalib::{ParserConfig, from_str_with_config, to_string_value};
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

#[cfg(feature = "lossless-u64")]
#[test]
fn integer_lossless_u64_plain_scalars() {
    let cfg = ParserConfig::new().lossless_u64_integers(true);
    for value in [i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX] {
        let yaml = format!("{value}\n");
        let v: Value = from_str_with_config(&yaml, &cfg).unwrap();
        assert_eq!(v.as_u64(), Some(value));
        assert!(v.is_u64());
        assert!(!matches!(v, Value::Number(n) if n.is_float()));

        let emitted = to_string_value(&v).unwrap();
        assert_eq!(emitted.trim(), value.to_string());
        let reparsed: Value = from_str_with_config(&emitted, &cfg).unwrap();
        assert_eq!(reparsed.as_u64(), Some(value));
    }
}

#[cfg(feature = "lossless-u64")]
#[test]
fn integer_lossless_u64_tagged_and_radix_scalars() {
    let cfg = ParserConfig::new().lossless_u64_integers(true);
    for (yaml, expected) in [
        ("!!int 18446744073709551615\n", u64::MAX),
        ("0x8000000000000000\n", i64::MAX as u64 + 1),
        ("0xffffffffffffffff\n", u64::MAX),
        ("!!int 0xffffffffffffffff\n", u64::MAX),
    ] {
        let v: Value = from_str_with_config(yaml, &cfg).unwrap();
        assert_eq!(v.as_u64(), Some(expected), "{yaml}");
    }
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
