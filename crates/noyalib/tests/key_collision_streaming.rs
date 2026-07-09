// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Distinct-typed key-collision parity on the STREAMING path (v0.0.15).
//!
//! v0.0.14 gave `from_str::<Value>` (the AST fast path) the KeyCollision
//! guard, but the streaming path — used for typed/struct/map targets and the
//! borrowing API — still stringified keys and silently collapsed a distinct-
//! typed collision (`1` vs `"1"`). This suite pins the fix: the streaming
//! path now detects collisions in parity with the loader.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use noyalib::{Error, Value, from_str, from_str_borrowing};

fn assert_collision<T: std::fmt::Debug>(r: Result<T, Error>, ctx: &str) {
    assert!(
        matches!(r, Err(Error::KeyCollision(_))),
        "{ctx}: expected KeyCollision, got {r:?}"
    );
}

#[test]
fn typed_map_target_detects_distinct_typed_collision() {
    // The typed/map target routes through streaming; `1` (int) and `"1"`
    // (string) both stringify to "1" and used to silently collapse.
    assert_collision(
        from_str::<BTreeMap<String, String>>("1: a\n\"1\": b\n"),
        "BTreeMap 1 vs \"1\"",
    );
    assert_collision(
        from_str::<BTreeMap<String, String>>("true: a\n\"true\": b\n"),
        "BTreeMap true vs \"true\"",
    );
    assert_collision(
        from_str::<BTreeMap<String, String>>("~: a\n\"null\": b\n"),
        "BTreeMap ~ vs \"null\"",
    );
}

#[test]
fn borrowing_value_target_detects_distinct_typed_collision() {
    // BUG004: the zero-copy entry point built a StreamingDeserializer
    // unconditionally, so `Value` collisions collapsed silently there.
    assert_collision(
        from_str_borrowing::<Value>("1: a\n\"1\": b\n"),
        "from_str_borrowing::<Value>",
    );
}

#[test]
fn streaming_collision_does_not_over_fire() {
    // Distinct keys and genuine same-typed duplicates are NOT collisions.
    let ok: BTreeMap<String, i64> = from_str("a: 1\nb: 2\n").unwrap();
    assert_eq!(ok.len(), 2);
    let ints: BTreeMap<String, i64> = from_str("1: 10\n2: 20\n").unwrap();
    assert_eq!(ints.len(), 2);
    // Same-typed duplicate key resolves under DuplicateKeyPolicy (last wins),
    // not a collision.
    let dup: BTreeMap<String, i64> = from_str("a: 1\na: 2\n").unwrap();
    assert_eq!(dup.get("a"), Some(&2));
}
