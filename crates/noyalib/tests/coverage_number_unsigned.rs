// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for `Number::Unsigned` arms in `value/number.rs` — the
//! predicates (`as_f64` / `is_i64` / `is_nan` / `is_infinite` /
//! `is_finite`), the full cross-type `Ord` matrix, and parsing an
//! integer above `i64::MAX`. All of these live behind `lossless-u64`,
//! which is why the arms stay uncovered without the feature.

#![cfg(feature = "lossless-u64")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use core::cmp::Ordering;
use core::str::FromStr;

use noyalib::Number;

#[test]
fn unsigned_predicates_and_as_f64() {
    let big = Number::Unsigned(u64::MAX);
    let small = Number::Unsigned(5);
    assert_eq!(small.as_f64(), 5.0);
    assert!(small.is_i64(), "5 fits in i64");
    assert!(!big.is_i64(), "u64::MAX does not fit in i64");
    assert!(!big.is_nan());
    assert!(!big.is_infinite());
    assert!(big.is_finite());
}

#[test]
fn unsigned_cross_type_ordering() {
    let u = Number::Unsigned(10);
    let u_big = Number::Unsigned(20);
    let i_neg = Number::Integer(-3);
    let i_pos = Number::Integer(15);
    let f_lt = Number::Float(2.5);
    let f_gt = Number::Float(99.0);
    let nan = Number::Float(f64::NAN);

    // Unsigned vs Unsigned.
    assert_eq!(u.cmp(&u_big), Ordering::Less);
    assert_eq!(u_big.cmp(&u), Ordering::Greater);
    // Integer vs Unsigned (negative integer is always the smaller).
    assert_eq!(i_neg.cmp(&u), Ordering::Less);
    assert_eq!(i_pos.cmp(&u), Ordering::Greater);
    // Unsigned vs Integer (mirror).
    assert_eq!(u.cmp(&i_neg), Ordering::Greater);
    assert_eq!(u.cmp(&i_pos), Ordering::Less);
    // Unsigned vs Float.
    assert_eq!(u.cmp(&f_lt), Ordering::Greater);
    assert_eq!(u.cmp(&f_gt), Ordering::Less);
    // Float vs Unsigned (mirror).
    assert_eq!(f_gt.cmp(&u), Ordering::Greater);
    assert_eq!(f_lt.cmp(&u), Ordering::Less);
    // NaN arms: must not panic and must be antisymmetric.
    let a = u.cmp(&nan);
    let b = nan.cmp(&u);
    assert_ne!(a, Ordering::Equal);
    assert_eq!(a, b.reverse(), "cmp must be a consistent total order");
}

#[test]
fn number_from_str_above_i64_max_is_unsigned() {
    // `Number`'s `FromStr` falls back to a u64 parse when the value
    // overflows i64 — decimal, hex, and octal all have their own
    // lossless `Unsigned` arms.
    let dec = Number::from_str("18446744073709551615").unwrap();
    assert!(matches!(dec, Number::Unsigned(u64::MAX)), "got {dec:?}");
    let hex = Number::from_str("0xFFFFFFFFFFFFFFFF").unwrap();
    assert!(matches!(hex, Number::Unsigned(u64::MAX)), "got {hex:?}");
    let oct = Number::from_str("0o1777777777777777777777").unwrap();
    assert!(matches!(oct, Number::Unsigned(u64::MAX)), "got {oct:?}");
}

#[test]
fn from_u64_constructs_number() {
    let small = Number::from(5_u64);
    assert_eq!(small.as_f64(), 5.0);
    let big = Number::from(u64::MAX);
    assert!(big.as_f64() > 0.0);
}
