//! `Error::kind()` classifier smoke tests.
//!
//! Pins the mapping from an [`Error`] variant to its coarse
//! [`ErrorKind`]. Downstream consumers routing errors by kind
//! (structured logging, HTTP status mapping, retry policies)
//! rely on this being stable — new variants must land under an
//! existing kind whenever possible.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{
    BudgetBreach, DuplicateKeyPolicy, Error, ErrorKind, ParserConfig, Value, from_str,
    from_str_with_config,
};

#[test]
fn syntax_error_kinds_as_syntax() {
    let err = from_str::<Value>("a: [unclosed").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Syntax);
}

#[test]
fn key_collision_kinds_as_key_collision() {
    let err = from_str::<Value>("1: a\n\"1\": b\n").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::KeyCollision);
}

#[test]
fn duplicate_key_error_policy_kinds_as_duplicate_key() {
    let mut cfg = ParserConfig::default();
    cfg.duplicate_key_policy = DuplicateKeyPolicy::Error;
    let err = from_str_with_config::<Value>("k: 1\nk: 2\n", &cfg).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::DuplicateKey);
}

#[test]
fn budget_errors_kind_as_budget() {
    // Uses a `BudgetBreach` variant so the mapping is unambiguous.
    // (Historical caveat: `max_sequence_length` / `max_mapping_keys`
    // still surface as `Error::Serialize("… limit exceeded")` rather
    // than `Error::Budget(…)`, so those two land in
    // [`ErrorKind::Data`] — worth unifying in a follow-up.)
    let mut cfg = ParserConfig::default();
    cfg.max_merge_keys = 1;
    let src = "\
a: &a
  x: 1
b: &b
  y: 2
c:
  <<: *a
  <<: *b
";
    let err = from_str_with_config::<Value>(src, &cfg).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Budget);
}

#[test]
fn recursion_limit_kinds_as_budget() {
    let e = Error::RecursionLimitExceeded { depth: 999 };
    assert_eq!(e.kind(), ErrorKind::Budget);
}

#[test]
fn budget_breach_kinds_as_budget() {
    let e = Error::Budget(BudgetBreach::MaxEvents {
        limit: 1,
        observed: 2,
    });
    assert_eq!(e.kind(), ErrorKind::Budget);
}

#[test]
fn end_of_stream_kinds_as_end_of_stream() {
    assert_eq!(Error::EndOfStream.kind(), ErrorKind::EndOfStream);
}

#[test]
fn missing_field_kinds_as_data() {
    assert_eq!(Error::MissingField("x".into()).kind(), ErrorKind::Data);
    assert_eq!(Error::UnknownField("y".into()).kind(), ErrorKind::Data);
    assert_eq!(
        Error::TypeMismatch {
            expected: "int",
            found: "str".into()
        }
        .kind(),
        ErrorKind::Data
    );
}

#[test]
fn custom_kinds_as_other() {
    assert_eq!(Error::Custom("oops".into()).kind(), ErrorKind::Other);
    assert_eq!(Error::Message("m".into(), None).kind(), ErrorKind::Other);
}

#[test]
fn merge_shape_kinds_as_policy() {
    assert_eq!(Error::ScalarInMergeElement.kind(), ErrorKind::Policy);
    assert_eq!(Error::SequenceInMergeElement.kind(), ErrorKind::Policy);
    assert_eq!(Error::TaggedInMerge.kind(), ErrorKind::Policy);
}

#[test]
fn shared_delegates_to_inner_kind() {
    use std::sync::Arc;
    let inner = Error::KeyCollision("1".into());
    let shared = Error::Shared(Arc::new(inner));
    assert_eq!(shared.kind(), ErrorKind::KeyCollision);
}

#[test]
fn error_kind_traits_are_present() {
    // Trait bounds are surfaced in the tests so a future change
    // that drops one shows up here as a compile failure.
    fn assert_traits<T: Copy + PartialEq + Eq + core::hash::Hash + core::fmt::Debug>() {}
    assert_traits::<ErrorKind>();
    let a = ErrorKind::Syntax;
    let b = a;
    assert_eq!(a, b);
}
