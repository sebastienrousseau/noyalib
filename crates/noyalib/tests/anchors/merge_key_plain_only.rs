//! Only plain `<<` scalars are merge keys. Quoted spellings and aliases
//! resolving to the same text remain ordinary keys. Single-value and
//! whole-stream entry points must agree.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Value, from_str, load_all};

/// Load through the AST loader.
#[track_caller]
fn ast(yaml: &str) -> Result<Value, noyalib::Error> {
    load_all(yaml)?
        .next()
        .unwrap_or_else(|| panic!("no document in:\n{yaml}"))
}

/// Both paths must produce the same `Value`.
#[track_caller]
fn both(yaml: &str) -> Value {
    let streamed: Value = from_str(yaml).unwrap_or_else(|e| panic!("streaming: {e}\n{yaml}"));
    let loaded = ast(yaml).unwrap_or_else(|e| panic!("ast: {e}\n{yaml}"));
    assert_eq!(streamed, loaded, "paths disagree on:\n{yaml}");
    streamed
}

/// Both paths must refuse.
#[track_caller]
fn both_err(yaml: &str) {
    assert!(
        from_str::<Value>(yaml).is_err(),
        "streaming accepted:\n{yaml}"
    );
    assert!(ast(yaml).is_err(), "ast accepted:\n{yaml}");
}

// ── a plain `<<` is a merge key (unchanged) ──────────────────────────

#[test]
fn plain_merge_key_still_merges() {
    let v = both("base: &b\n  x: 1\n  y: 2\nout:\n  <<: *b\n");
    assert_eq!(v["out"]["x"], Value::from(1));
    assert_eq!(v["out"]["y"], Value::from(2));
    assert!(v["out"].get("<<").is_none(), "the key must not survive");
}

#[test]
fn plain_merge_key_with_a_local_override() {
    let v = both("base: &b\n  x: 1\n  y: 2\nout:\n  <<: *b\n  y: 9\n");
    assert_eq!(v["out"]["y"], Value::from(9), "local wins");
}

#[test]
fn plain_merge_key_from_a_sequence_of_anchors() {
    let v = both("a: &a\n  x: 1\nb: &b\n  y: 2\nout:\n  <<: [*a, *b]\n");
    assert_eq!(v["out"]["x"], Value::from(1));
    assert_eq!(v["out"]["y"], Value::from(2));
}

// ── a quoted `<<` is an ordinary key ─────────────────────────────────

#[test]
fn double_quoted_merge_spelling_is_an_ordinary_key() {
    let v = both("out:\n  \"<<\": 1\n  y: 2\n");
    assert_eq!(v["out"]["<<"], Value::from(1), "kept as a literal key");
    assert_eq!(v["out"]["y"], Value::from(2));
}

#[test]
fn single_quoted_merge_spelling_is_an_ordinary_key() {
    let v = both("out:\n  '<<': 1\n  y: 2\n");
    assert_eq!(v["out"]["<<"], Value::from(1));
}

#[test]
fn a_quoted_merge_spelling_may_take_any_value() {
    // As an ordinary key it has no constraint on its value — a scalar here
    // would be `ScalarInMergeElement` if it were still read as a merge.
    let v = both("out:\n  \"<<\": [1, 2]\n");
    assert_eq!(v["out"]["<<"][0], Value::from(1));
    let v = both("out:\n  \"<<\": {a: 1}\n");
    assert_eq!(v["out"]["<<"]["a"], Value::from(1));
}

#[test]
fn a_quoted_merge_spelling_beside_a_real_merge() {
    let v = both("base: &b\n  x: 1\nout:\n  <<: *b\n  \"<<\": 2\n");
    assert_eq!(v["out"]["x"], Value::from(1), "the plain one merged");
    assert_eq!(v["out"]["<<"], Value::from(2), "the quoted one is a key");
}

// ── an alias that resolves to "<<" is an ordinary key ────────────────

#[test]
fn an_alias_resolving_to_the_merge_spelling_is_an_ordinary_key() {
    let v = both("k: &k \"<<\"\nout:\n  *k : 1\n  y: 2\n");
    assert_eq!(v["out"]["<<"], Value::from(1), "not a merge instruction");
    assert_eq!(v["out"]["y"], Value::from(2));
}

#[test]
fn an_alias_to_a_plain_merge_spelling_is_still_an_ordinary_key() {
    // The anchored scalar is a *plain* `<<`, but the merge tag attaches to
    // the scalar's own presentation at the key position, and this key is
    // an alias — not a plain scalar.
    let v = both("k: &k <<\nout:\n  *k : 1\n  y: 2\n");
    assert_eq!(v["out"]["<<"], Value::from(1));
    assert_eq!(v["out"]["y"], Value::from(2));
}

// ── malformed merges must still be refused ───────────────────────────

#[test]
fn a_plain_merge_key_with_a_scalar_target_is_refused() {
    both_err("out:\n  <<: 1\n");
}

#[test]
fn a_plain_merge_key_with_an_unknown_anchor_is_refused() {
    both_err("out:\n  <<: *nope\n");
}

#[test]
fn a_plain_merge_key_with_a_scalar_inside_a_sequence_is_refused() {
    both_err("a: &a\n  x: 1\nout:\n  <<: [*a, 1]\n");
}
