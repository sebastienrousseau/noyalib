// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Panic-free property tests.
//!
//! Formalises the contract from
//! [`POLICIES.md` §8 — Panic policy](../../../doc/POLICIES.md#8-panic-policy)
//! that the public `from_*` family **never panics on input**,
//! regardless of how malformed. Every entry point handles
//! malformed input by returning `Err(Error)`, never by
//! unwinding.
//!
//! These property tests use `proptest` to throw arbitrary byte
//! sequences (UTF-8 and non-UTF-8) at every public parse entry
//! and assert that whatever the parser does, it does not panic.

#![allow(missing_docs)]

use noyalib::{Value, from_slice, from_str};
use proptest::prelude::*;

// Default to 256 random cases per property; CI may override
// via `PROPTEST_CASES`.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(256)
    ))]

    /// `from_str::<Value>` on any valid UTF-8 string never panics.
    /// It returns either `Ok(Value)` or `Err(Error)` — both fine.
    #[test]
    fn from_str_value_never_panics_on_arbitrary_utf8(s in ".{0,1024}") {
        std::panic::catch_unwind(|| {
            let _: Result<Value, _> = from_str(&s);
        }).expect("from_str must not panic on arbitrary UTF-8");
    }

    /// `from_slice::<Value>` on any byte sequence never panics
    /// — even non-UTF-8 input. Non-UTF-8 should error cleanly.
    #[test]
    fn from_slice_value_never_panics_on_arbitrary_bytes(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        std::panic::catch_unwind(|| {
            let _: Result<Value, _> = from_slice(&bytes);
        }).expect("from_slice must not panic on arbitrary bytes");
    }

    /// `from_str::<i64>` on any UTF-8 string never panics.
    #[test]
    fn from_str_typed_never_panics(s in ".{0,256}") {
        std::panic::catch_unwind(|| {
            let _: Result<i64, _> = from_str(&s);
        }).expect("typed from_str must not panic");
    }

    /// `from_str::<String>` on arbitrary input never panics.
    #[test]
    fn from_str_string_target_never_panics(s in ".{0,256}") {
        std::panic::catch_unwind(|| {
            let _: Result<String, _> = from_str(&s);
        }).expect("String-target from_str must not panic");
    }

    /// `load_all` on arbitrary input never panics.
    #[test]
    fn load_all_never_panics(s in ".{0,512}") {
        std::panic::catch_unwind(|| {
            let _: Result<Vec<Value>, _> = noyalib::load_all_as(&s);
        }).expect("load_all_as must not panic");
    }

    /// `cst::parse_document` never panics on arbitrary UTF-8.
    #[test]
    fn cst_parse_document_never_panics(s in ".{0,512}") {
        std::panic::catch_unwind(|| {
            let _ = noyalib::cst::parse_document(&s);
        }).expect("cst::parse_document must not panic");
    }
}

// ── Targeted regression-style panic-free tests ───────────────────

/// Inputs known to have caused panics in earlier YAML parsers.
const HISTORICAL_PANIC_INPUTS: &[&[u8]] = &[
    // Empty
    b"",
    // BOM only
    b"\xef\xbb\xbf",
    // Just a directive
    b"%YAML 1.2",
    // Unclosed flow
    b"[",
    b"{",
    b"[[[[",
    b"{{{{",
    // Empty alias
    b"*",
    // Empty anchor
    b"&",
    // Empty tag
    b"!",
    // Just a colon
    b":",
    // Just a dash
    b"-",
    // Mixed
    b"---\n...\n---\n...\n",
    // Quote bombs
    b"'\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\",
    b"\"\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\",
    // Deep nesting
    b"[[[[[[[[[[[[[[[[[[[[",
    // Bad UTF-8
    b"\xff\xff\xff",
    b"\x80\x81\x82",
    // CRLF mix
    b"a:\r\n  b:\r\n    c",
    // Tabs in indent (should error)
    b"\ta:\n\t\tb: 1",
];

#[test]
fn historical_panic_inputs_do_not_panic() {
    for (i, bytes) in HISTORICAL_PANIC_INPUTS.iter().enumerate() {
        let res = std::panic::catch_unwind(|| {
            let _: Result<Value, _> = from_slice(bytes);
        });
        assert!(
            res.is_ok(),
            "panic on historical input #{i}: {:?}",
            String::from_utf8_lossy(bytes)
        );
    }
}

#[test]
fn historical_panic_inputs_via_str_when_utf8() {
    for (i, bytes) in HISTORICAL_PANIC_INPUTS.iter().enumerate() {
        if let Ok(s) = std::str::from_utf8(bytes) {
            let res = std::panic::catch_unwind(|| {
                let _: Result<Value, _> = from_str(s);
            });
            assert!(res.is_ok(), "panic on str-cast historical #{i}: {s:?}");
        }
    }
}
