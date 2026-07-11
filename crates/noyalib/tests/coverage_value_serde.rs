// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for `value/serde_impl.rs` — the enum-from-string
//! deserializer and the lossless-u64 visitor arm.
//!
//! (An earlier revision also tested a tag-preserving magic-map
//! deserialize path; that path was genuinely orphaned — `Value::Tagged`
//! serialises to a foreign serde format as `{tag: value}`, never as the
//! `$__noyalib_*` magic map, so nothing produces that input — and was
//! removed as dead code in #172, so the test is gone with it.)

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::{Value, from_value, to_value};
use serde::Deserialize;

#[test]
fn enum_deserializes_from_bare_string() {
    // A unit enum variant deserialized from a bare `Value::String`
    // exercises the string-enum deserializer arm.
    #[derive(Debug, Deserialize, PartialEq)]
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
