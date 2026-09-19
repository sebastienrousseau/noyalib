// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Property tests for the two pure decision procedures that the v0.0.33
//! bug reports (#383, #385, #387, #388) showed the example-based suites
//! could not enumerate: the block-scalar layout the emitter chooses for
//! a string, and the path grammar's quoting of arbitrary keys.
//!
//! Both properties are total: for every generated input the identity
//! must hold, with no carve-outs.

#![allow(clippy::unwrap_used, missing_docs)]

use noyalib::{Mapping, Value, from_str, path, to_string};
use proptest::prelude::*;

/// Strings drawn from the alphabet that drives every block-scalar
/// decision: spaces and tabs (indentation indicators), newlines
/// (chomping), and a few letters so content lines exist.
fn hostile_string() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            4 => Just('a'),
            2 => Just('b'),
            3 => Just(' '),
            1 => Just('\t'),
            4 => Just('\n'),
            1 => Just('#'),
            1 => Just(':'),
            1 => Just('-'),
        ],
        0..14,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

/// Keys that exercise every branch of `path::quote_key`: the four
/// structural characters, quotes, backslashes, and plain letters.
fn hostile_key() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            4 => Just('k'),
            2 => Just('.'),
            2 => Just('['),
            2 => Just(']'),
            1 => Just('*'),
            1 => Just('"'),
            1 => Just('\''),
            1 => Just('\\'),
            1 => Just(' '),
            1 => Just('/'),
        ],
        1..8,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn map(entries: Vec<(String, Value)>) -> Value {
    let mut m = Mapping::new();
    for (k, v) in entries {
        let _previous = m.insert(k, v);
    }
    Value::Mapping(m)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// Whatever layout the emitter picks for a string (plain, quoted,
    /// literal block with any chomping or indentation indicator), the
    /// parser reads the identical string back, and emitting that value
    /// again yields the identical text.
    #[test]
    fn strings_survive_emit_then_parse(s in hostile_string()) {
        let v = map(vec![("k".into(), Value::String(s.clone()))]);
        let out = to_string(&v).unwrap();
        let back: Value = from_str(&out).unwrap();
        prop_assert_eq!(back["k"].as_str(), Some(s.as_str()), "emitted:\n{}", out);
        let again = to_string(&back).unwrap();
        prop_assert_eq!(&again, &out, "to_string is not idempotent");
    }

    /// A block scalar followed by a sibling entry keeps its exact value
    /// across repeated round trips (the #385 class: keep chomping used to
    /// grow the value by one newline per trip when a sibling followed).
    #[test]
    fn block_scalar_with_sibling_is_stable(s in hostile_string()) {
        let v = map(vec![
            ("off".into(), Value::String(s)),
            ("b".into(), Value::String("x".into())),
        ]);
        let out1 = to_string(&v).unwrap();
        let back1: Value = from_str(&out1).unwrap();
        prop_assert_eq!(&back1, &v, "first trip:\n{}", out1);
        let out2 = to_string(&back1).unwrap();
        prop_assert_eq!(&out2, &out1, "second trip changed the text");
    }

    /// A block scalar as a bare sequence item reads back exactly (the
    /// #387 class: the indentation indicator counted from the wrong
    /// column).
    #[test]
    fn sequence_item_block_scalar_round_trips(s in hostile_string()) {
        let v = map(vec![("a".into(), Value::Sequence(vec![Value::String(s)]))]);
        let out = to_string(&v).unwrap();
        let back: Value = from_str(&out).unwrap();
        prop_assert_eq!(&back, &v, "emitted:\n{}", out);
    }

    /// `quote_key` spells any key so that every path-taking API reads it
    /// back as that key; `push_key` composes such segments; `join_keys`
    /// builds the whole path.
    #[test]
    fn quoted_keys_address_their_entry(k in hostile_key(), inner in hostile_key()) {
        let leaf = Value::String("v".into());
        let v = map(vec![(k.clone(), map(vec![(inner.clone(), leaf.clone())]))]);
        let one = path::quote_key(&k);
        prop_assert!(v.get_path(&one).is_some(), "quote_key({:?}) = {:?} did not resolve", k, one);
        let mut p = String::new();
        path::push_key(&mut p, &k);
        path::push_key(&mut p, &inner);
        prop_assert_eq!(v.get_path(&p), Some(&leaf), "push_key path {:?}", p);
        let joined = path::join_keys([k.as_str(), inner.as_str()]);
        prop_assert_eq!(v.get_path(&joined), Some(&leaf), "join_keys path {:?}", joined);
    }
}
