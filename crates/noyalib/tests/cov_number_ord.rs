// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage: `Number::cmp` cross-type arms (Integer / Unsigned / Float),
//! reachable because `Value::cmp` delegates to `Number::cmp` for two numeric
//! values. The `Unsigned` arms are `#[cfg(feature = "lossless-u64")]`.
//!
//! NB: values are built with `Value::from(u64/i64/f64)` — `from_str` with the
//! default config never yields `Number::Unsigned` (lossless-u64 is a config
//! toggle, not just a feature), so it would not reach the `Unsigned` arms.

#![cfg(feature = "lossless-u64")]
#![allow(clippy::unwrap_used)]

use std::cmp::Ordering;

use noyalib::Value;

#[test]
fn number_ord_integer_vs_unsigned() {
    let umax = Value::from(u64::MAX); // Number::Unsigned
    let neg = Value::from(-5_i64); // Number::Integer(-5)
    let pos = Value::from(5_i64); // Number::Integer(5)

    // Integer vs Unsigned: a negative integer is always Less.
    assert_eq!(neg.cmp(&umax), Ordering::Less);
    // A non-negative integer widens to u64 and compares.
    assert_eq!(pos.cmp(&umax), Ordering::Less);
    // Unsigned vs Integer (the symmetric arm).
    assert_eq!(umax.cmp(&neg), Ordering::Greater);
    assert_eq!(umax.cmp(&pos), Ordering::Greater);
    // Unsigned vs Unsigned.
    assert_eq!(umax.cmp(&Value::from(u64::MAX)), Ordering::Equal);
}

#[test]
fn number_ord_unsigned_vs_float_and_nan() {
    let umax = Value::from(u64::MAX);
    let flt = Value::from(2.5_f64);
    let nan = Value::from(f64::NAN);

    // Unsigned vs Float and the symmetric Float vs Unsigned.
    assert_eq!(umax.cmp(&flt), Ordering::Greater);
    assert_eq!(flt.cmp(&umax), Ordering::Less);
    // NaN arms must not panic and must yield a total order.
    let _ = umax.cmp(&nan);
    let _ = nan.cmp(&umax);
}

#[test]
fn number_ord_integer_vs_float() {
    let neg = Value::from(-5_i64);
    let flt = Value::from(2.5_f64);
    assert_eq!(neg.cmp(&flt), Ordering::Less);
    assert_eq!(flt.cmp(&neg), Ordering::Greater);
}

#[test]
fn number_ord_sort_mixed_types() {
    // A sort exercises many cross-type comparisons at once.
    let mut xs = [
        Value::from(u64::MAX),
        Value::from(-5_i64),
        Value::from(2.5_f64),
        Value::from(5_i64),
        Value::from(0_i64),
    ];
    xs.sort();
    assert_eq!(xs.first(), Some(&Value::from(-5_i64)));
    assert_eq!(xs.last(), Some(&Value::from(u64::MAX)));
}
