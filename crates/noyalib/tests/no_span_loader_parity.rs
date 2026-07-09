//! Fast-path `Value` loader parity with the span-full loader.
//!
//! `from_str::<Value>` routes through the span-free loader
//! (`NoSpanLoader`) for the same reason the streaming path exists:
//! the caller doesn't need span data, so building it is waste. That
//! fast path used to lack the collision guard and the three DoS
//! budgets the span-full loader enforces — a distinct-typed key
//! (`1: a` then `"1": b`) would silently collapse, and an over-limit
//! sequence, mapping, or `<<` chain would run to completion.
//!
//! This suite pins the parity: every check the span-full loader
//! applies must also fire on the fast `Value` path.
//!
//! Regression tests for the review of `feat/v0.0.14`.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{
    BudgetBreach, DuplicateKeyPolicy, Error, MergeKeyPolicy, ParserConfig, Value, from_str,
    from_str_with_config,
};

// ── Distinct-typed collisions on the Value fast path ──────────────

#[test]
fn from_str_value_refuses_distinct_typed_key_collision() {
    // Reproduces the pre-fix silent-collapse: `1: a` then `"1": b`
    // both stringify to key `"1"`, so the second insert would drop
    // the first. The fast path must refuse.
    for src in [
        "1: a\n\"1\": b\n",
        "true: yes\n\"true\": no\n",
        "~: a\n\"null\": b\n",
    ] {
        let err = from_str::<Value>(src)
            .expect_err("distinct-typed collision must error on the Value fast path");
        assert!(
            matches!(err, Error::KeyCollision(_)),
            "expected KeyCollision for {src:?}, got {err:?}"
        );
    }
}

#[test]
fn from_str_value_accepts_genuine_duplicate_keys() {
    // Same typed key twice is a duplicate, not a collision: default
    // `DuplicateKeyPolicy::Last` keeps one entry with no error.
    let v: Value = from_str("1: a\n1: b\n").unwrap();
    let m = v.as_mapping().unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m.get("1"), Some(&Value::String("b".into())));
}

#[test]
fn from_str_value_accepts_lone_scalar_key() {
    // A lone numeric or bool key must stringify without tripping
    // the collision check.
    assert!(from_str::<Value>("8080: service\n").is_ok());
    assert!(from_str::<Value>("true: on\n").is_ok());
    assert!(from_str::<Value>("a: 1\nb: 2\n").is_ok());
}

// ── DoS budgets on the Value fast path ────────────────────────────

fn small_limits() -> ParserConfig {
    let mut cfg = ParserConfig::default();
    cfg.max_sequence_length = 4;
    cfg.max_mapping_keys = 4;
    cfg.max_merge_keys = 2;
    cfg
}

#[test]
fn from_str_value_enforces_max_sequence_length() {
    let cfg = small_limits();
    let src = "[1, 2, 3, 4, 5]\n"; // 5 items > 4
    let err = from_str_with_config::<Value>(src, &cfg)
        .expect_err("sequence-length budget must fire on the Value fast path");
    // Mirrors the span-full loader's spelling.
    let msg = format!("{err}");
    assert!(
        msg.contains("sequence length limit"),
        "expected sequence-length budget error, got {msg:?}"
    );
}

#[test]
fn from_str_value_enforces_max_mapping_keys() {
    let cfg = small_limits();
    let src = "a: 1\nb: 2\nc: 3\nd: 4\ne: 5\n"; // 5 keys > 4
    let err = from_str_with_config::<Value>(src, &cfg)
        .expect_err("mapping-key budget must fire on the Value fast path");
    let msg = format!("{err}");
    assert!(
        msg.contains("mapping key limit"),
        "expected mapping-key budget error, got {msg:?}"
    );
}

#[test]
fn from_str_value_enforces_max_merge_keys() {
    let cfg = small_limits();
    // Three `<<:` merges > 2. Anchor-and-alias each so the merge
    // event actually fires.
    let src = "\
a: &a
  x: 1
b: &b
  y: 2
c: &c
  z: 3
one:
  <<: *a
  <<: *b
  <<: *c
";
    let err = from_str_with_config::<Value>(src, &cfg)
        .expect_err("max-merge-keys budget must fire on the Value fast path");
    assert!(
        matches!(err, Error::Budget(BudgetBreach::MaxMergeKeys { .. })),
        "expected MaxMergeKeys, got {err:?}"
    );
}

#[test]
fn from_str_value_merge_key_policy_error_refused_on_value_path() {
    // `MergeKeyPolicy::Error` must reject `<<` on the Value fast
    // path exactly like on the span-full path.
    let mut cfg = ParserConfig::default();
    cfg.merge_key_policy = MergeKeyPolicy::Error;
    let src = "base: &b\n  x: 1\nservice:\n  <<: *b\n";
    let err = from_str_with_config::<Value>(src, &cfg)
        .expect_err("MergeKeyPolicy::Error must reject `<<`");
    let msg = format!("{err}");
    assert!(
        msg.contains("merge key") || msg.contains("<<"),
        "unexpected message: {msg:?}"
    );
}

// ── Merge keys don't clone or count against collision ─────────────

#[test]
fn merge_keys_do_not_trip_collision_on_value_path() {
    // `<<` merge values are buffered and never run through the
    // collision check; the hot-path clone should be skipped, but
    // the visible behaviour is: no error, merged fields present.
    let src = "\
defaults: &d
  timeout: 30
service:
  <<: *d
  name: web
";
    let v: Value = from_str(src).unwrap();
    let svc = v.as_mapping().unwrap().get("service").unwrap();
    let svc = svc.as_mapping().unwrap();
    assert_eq!(svc.get("timeout"), Some(&Value::Number(30i64.into())));
    assert_eq!(svc.get("name"), Some(&Value::String("web".into())));
}

#[test]
fn duplicate_policy_first_still_wins_on_value_path() {
    // Verify the parity fix preserves the existing
    // `DuplicateKeyPolicy` behaviour on the fast path.
    let mut cfg = ParserConfig::default();
    cfg.duplicate_key_policy = DuplicateKeyPolicy::First;
    let v: Value = from_str_with_config("k: one\nk: two\n", &cfg).unwrap();
    let m = v.as_mapping().unwrap();
    // Default fast path is last-wins, but per test above the
    // policy is overridden; both loaders honour it.
    assert_eq!(m.get("k"), Some(&Value::String("one".into())));
}

// ── max_documents parity on the Value fast path (ultrareview BUG006) ──

#[test]
fn value_fast_path_enforces_max_documents() {
    // `from_str::<Value>` routes through NoSpanLoader, which lacked the
    // `max_documents` guard the span-full loader enforces — so a caller
    // using max_documents as a DoS mitigation got no enforcement on the
    // fast path, and the whole stream was materialised.
    let cfg = ParserConfig::new().max_documents(1);
    let src = "a: 1\n---\nb: 2\n---\nc: 3\n";
    let err = from_str_with_config::<Value>(src, &cfg)
        .expect_err("max_documents must be enforced on the Value fast path");
    assert!(
        matches!(
            err,
            Error::Budget(BudgetBreach::MaxDocuments { limit: 1, .. })
        ),
        "expected MaxDocuments budget error, got {err:?}"
    );
}
