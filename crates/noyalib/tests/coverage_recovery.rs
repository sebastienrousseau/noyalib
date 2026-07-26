// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for the lenient-recovery branch arms in `recovery.rs`:
//! the zero-budget short-circuit, the Pass-2 (duplicate-key) error
//! collection, and the line-truncation whitespace-candidate skip.

#![cfg(feature = "recovery")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::recovery::{LenientConfig, parse_lenient_with};
use noyalib::{DuplicateKeyPolicy, ParserConfig};

#[test]
fn zero_error_budget_short_circuits_to_null() {
    // `budget == 0` (max_errors: 0) makes recover_one bail before even
    // the strict pass, returning Null with no diagnostics.
    let cfg = LenientConfig {
        max_errors: 0,
        ..LenientConfig::default()
    };
    let r = parse_lenient_with("a: [unclosed\n", &cfg);
    assert!(r.value.is_null(), "zero budget must yield Null");
    assert!(r.errors.is_empty(), "zero budget collects no errors");
}

#[test]
fn pass2_duplicate_key_error_is_collected() {
    // With a non-`Last` base policy, a duplicate key fails the strict
    // pass; the Pass-2 retry switches to `Last` but the *other* error
    // (an unterminated flow sequence) still fails — so the Pass-2 error
    // is pushed alongside the strict one.
    let base = ParserConfig::default().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let cfg = LenientConfig {
        base_config: base,
        line_truncation: false, // isolate the Pass-2 behaviour
        ..LenientConfig::default()
    };
    let r = parse_lenient_with("a: 1\na: 2\nb: [unclosed\n", &cfg);
    assert!(
        r.errors.len() >= 2,
        "strict + pass-2 errors both collected, got {:?}",
        r.errors
    );
}

#[test]
fn line_truncation_skips_whitespace_only_candidates() {
    // Leading blank lines before unrecoverable garbage force the
    // truncation loop to encounter whitespace-only prefixes, which it
    // skips (`continue`) rather than re-parsing.
    let cfg = LenientConfig::default();
    let r = parse_lenient_with("\n\n\n{[garbage", &cfg);
    // Best-effort recovery must not panic and reports the failure.
    assert!(!r.errors.is_empty(), "unrecoverable garbage yields errors");
    assert!(!r.is_complete);
}
