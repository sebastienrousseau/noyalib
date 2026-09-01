//! A BOM is a stream prefix, not content.
//!
//! Found by `fuzz_roundtrip` on `\u{feff}\n\u{feff}*'`: the scanner
//! treated a mid-document BOM as the first character of a plain
//! scalar, and the serializer wrote that string back unquoted — on
//! re-parse the leading BOM was stream-skipped and the rest of the
//! scalar reinterpreted as markup (`*'` became an alias to an anchor
//! that does not exist). Per §5.2 a BOM must not appear inside a
//! document, and a `Value::String` holding one must emit
//! double-quoted with `﻿`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::Value;

#[test]
fn leading_bom_is_skipped() {
    let v: Value = noyalib::from_str("\u{feff}a: 1\n").unwrap();
    assert_eq!(v["a"].as_i64(), Some(1));
}

#[test]
fn bom_inside_the_stream_is_rejected() {
    for input in [
        "\u{feff}\n\u{feff}*'",
        "a: \u{feff}b\n",
        "a: 1\n\u{feff}b: 2\n",
    ] {
        let r: Result<Value, _> = noyalib::from_str(input);
        assert!(r.is_err(), "{input:?} must be rejected, got {r:?}");
    }
}

#[test]
fn strings_holding_a_bom_emit_double_quoted_and_round_trip() {
    for s in ["\u{feff}*'", "a\u{feff}b", "\u{feff}"] {
        let v = Value::String(s.to_owned());
        let emitted = noyalib::to_string(&v).unwrap();
        assert!(
            emitted.contains("\\uFEFF"),
            "{s:?} must emit the escape, got {emitted:?}"
        );
        let back: Value = noyalib::from_str(&emitted)
            .unwrap_or_else(|e| panic!("emit of {s:?} must re-parse: {e} ({emitted:?})"));
        assert_eq!(back.as_str(), Some(s), "{emitted:?}");
    }
}
