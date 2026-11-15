// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Cover enum-from-string deserialization and the lossless-u64 visitor.
//!
//! Tagged values serialize as `{tag: value}`. The unused magic-map
//! deserialization path was removed in #172.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::{Value, from_value};
// Only the `lossless-u64` test below uses `to_value`; gate the import with the
// same cfg so the default-feature build does not trip `-D unused-imports`.
#[cfg(feature = "lossless-u64")]
use noyalib::to_value;

#[test]
fn enum_deserializes_from_bare_string() {
    // A unit enum variant deserialized from a bare `Value::String`
    // exercises the string-enum deserializer arm.
    #[derive(Debug, serde::Deserialize, PartialEq)]
    enum Level {
        Info,
        Warn,
    }
    let v = Value::String("Warn".into());
    let got: Level = from_value(&v).unwrap();
    assert_eq!(got, Level::Warn);
}

#[cfg(feature = "lossless-u64")]
#[test]
fn unsigned_value_visits_u64() {
    // A `Value` holding an `Unsigned` number deserialized into a u64
    // drives the lossless-u64 `visit_u64` arm.
    let v = to_value(&u64::MAX).unwrap();
    let n: u64 = from_value(&v).unwrap();
    assert_eq!(n, u64::MAX);
}
