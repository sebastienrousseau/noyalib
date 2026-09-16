// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Streaming paths that only a shaped document reaches.
//!
//! The streaming deserializer has two clusters of logic that a
//! straightforward `struct` round-trip never touches: the `<<` merge
//! machinery (single alias, a *sequence* of aliases, a duplicate key
//! that the merge must lose to) and serde's four enum-variant shapes.
//! Both are reached only from the wire format, so they are tested here
//! against the format rather than through a helper.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use noyalib::{ParserConfig, Value, from_str, from_str_with_config};
use serde::Deserialize;

// ── merge keys ──────────────────────────────────────────────────

/// A single `<<: *anchor`: the common case, and the one the other
/// merge branches are measured against.
#[test]
fn a_single_alias_merge_pulls_the_anchor_in() {
    let yaml = "\
base: &base\n  a: 1\n  b: 2\ntarget:\n  <<: *base\n  c: 3\n";
    let m: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml).expect("single merge");
    assert_eq!(
        m["target"],
        BTreeMap::from([("a".into(), 1), ("b".into(), 2), ("c".into(), 3),])
    );
}

/// `<<: [*a, *b]` — the sequence form. Earlier sources win over later
/// ones per the merge-key spec, so `b` must keep `one`'s value.
#[test]
fn a_sequence_of_merge_sources_is_resolved_left_to_right() {
    let yaml = "\
one: &one\n  a: 1\n  b: 2\n\
two: &two\n  b: 22\n  c: 33\n\
target:\n  <<: [*one, *two]\n  d: 4\n";
    let m: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml).expect("sequence merge");
    assert_eq!(
        m["target"],
        BTreeMap::from([
            ("a".into(), 1),
            ("b".into(), 2),
            ("c".into(), 33),
            ("d".into(), 4),
        ]),
        "earlier merge sources must win over later ones"
    );
}

/// An explicit key always beats anything the merge supplies, whichever
/// side of the `<<` it is written on.
#[test]
fn an_explicit_key_overrides_the_merged_one_from_either_side() {
    for yaml in [
        "base: &base\n  a: 1\ntarget:\n  <<: *base\n  a: 99\n",
        "base: &base\n  a: 1\ntarget:\n  a: 99\n  <<: *base\n",
    ] {
        let m: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml).expect("override");
        assert_eq!(
            m["target"]["a"], 99,
            "merge overwrote an explicit key in {yaml:?}"
        );
    }
}

/// Three sources deep, with a nested collection carried through, so the
/// injection path walks a subtree rather than a flat scalar list.
#[test]
fn merge_sources_carry_nested_collections() {
    let yaml = "\
one: &one\n  nested:\n    x: 1\n  list:\n    - a\n\
two: &two\n  other: 2\n\
target:\n  <<: [*one, *two]\n";
    #[derive(Deserialize, Debug, PartialEq, Eq)]
    struct Inner {
        x: i64,
    }
    #[derive(Deserialize, Debug, PartialEq, Eq)]
    struct Target {
        nested: Inner,
        list: Vec<String>,
        other: i64,
    }
    #[derive(Deserialize)]
    struct Doc {
        target: Target,
    }
    let d: Doc = from_str(yaml).expect("nested merge");
    assert_eq!(
        d.target,
        Target {
            nested: Inner { x: 1 },
            list: vec!["a".into()],
            other: 2
        }
    );
}

/// A duplicate key inside the *target* mapping: the streaming reader
/// has to skip both the key and its value to stay aligned with the
/// event stream.
#[test]
fn a_duplicate_key_does_not_desynchronise_the_event_stream() {
    let yaml = "target:\n  a: 1\n  a: 2\n  b: 3\n";
    let m: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml).expect("duplicate key");
    assert_eq!(
        m["target"]["b"], 3,
        "the reader lost its place after the duplicate"
    );
    assert!(
        m["target"]["a"] == 1 || m["target"]["a"] == 2,
        "duplicate key produced neither value"
    );
}

// ── enum variants ───────────────────────────────────────────────

#[derive(Deserialize, Debug, PartialEq, Eq)]
enum Shape {
    Unit,
    Newtype(i64),
    Tuple(i64, i64),
    Struct { w: i64, h: i64 },
}

#[test]
fn every_serde_variant_shape_deserializes_from_the_stream() {
    #[derive(Deserialize, Debug, PartialEq, Eq)]
    struct Holder {
        shape: Shape,
    }
    let cases: &[(&str, Shape)] = &[
        ("shape: Unit\n", Shape::Unit),
        ("shape:\n  Newtype: 7\n", Shape::Newtype(7)),
        ("shape:\n  Tuple: [1, 2]\n", Shape::Tuple(1, 2)),
        (
            "shape:\n  Struct:\n    w: 3\n    h: 4\n",
            Shape::Struct { w: 3, h: 4 },
        ),
    ];
    for (yaml, want) in cases {
        let got: Holder = from_str(yaml).unwrap_or_else(|e| panic!("{yaml:?}: {e}"));
        assert_eq!(got.shape, *want, "wrong variant for {yaml:?}");
    }
}

#[test]
fn a_sequence_of_mixed_variants_round_trips() {
    let yaml = "\
- Unit\n- Newtype: 1\n- Tuple: [2, 3]\n- Struct:\n    w: 4\n    h: 5\n";
    let got: Vec<Shape> = from_str(yaml).expect("mixed variants");
    assert_eq!(
        got,
        vec![
            Shape::Unit,
            Shape::Newtype(1),
            Shape::Tuple(2, 3),
            Shape::Struct { w: 4, h: 5 },
        ]
    );
}

#[test]
fn an_unknown_variant_is_rejected_by_name() {
    let err = from_str::<Shape>("Nope\n").expect_err("unknown variant must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("Nope"),
        "error does not name the variant: {msg}"
    );
}

// ── scalar surface ──────────────────────────────────────────────

/// Every primitive serde asks for, in one struct, so the per-type
/// `deserialize_*` entry points are all driven from the stream.
#[test]
fn the_whole_primitive_surface_deserializes_from_the_stream() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Prims {
        b: bool,
        i8v: i8,
        i16v: i16,
        i32v: i32,
        i64v: i64,
        u8v: u8,
        u16v: u16,
        u32v: u32,
        u64v: u64,
        f32v: f32,
        f64v: f64,
        ch: char,
        s: String,
        unit: (),
        opt_some: Option<i64>,
        opt_none: Option<i64>,
    }
    let yaml = "\
b: true\ni8v: -8\ni16v: -16\ni32v: -32\ni64v: -64\n\
u8v: 8\nu16v: 16\nu32v: 32\nu64v: 64\n\
f32v: 1.5\nf64v: 2.5\nch: x\ns: hello\nunit: ~\n\
opt_some: 5\nopt_none: ~\n";
    let p: Prims = from_str(yaml).expect("primitives");
    assert_eq!(
        p,
        Prims {
            b: true,
            i8v: -8,
            i16v: -16,
            i32v: -32,
            i64v: -64,
            u8v: 8,
            u16v: 16,
            u32v: 32,
            u64v: 64,
            f32v: 1.5,
            f64v: 2.5,
            ch: 'x',
            s: "hello".into(),
            unit: (),
            opt_some: Some(5),
            opt_none: None,
        }
    );
}

/// The scalar resolver's legacy branches are all behind `ParserConfig`
/// flags, so a default parse never reaches them. Each pair below is the
/// same input read twice — once with the flag off, once on — because the
/// flag's whole purpose is the difference between the two.
#[test]
fn the_legacy_scalar_flags_change_what_a_plain_scalar_resolves_to() {
    // (input, flag setter, value with the flag off, value with it on)
    type Setter = fn(&mut ParserConfig);
    let cases: &[(&str, Setter, &str, &str)] = &[
        // YAML 1.1 sexagesimal: `1:30` is ninety seconds, `1:00:00` an hour.
        ("1:30", |c| c.legacy_sexagesimal = true, "1:30", "90"),
        (
            "1:00:00",
            |c| c.legacy_sexagesimal = true,
            "1:00:00",
            "3600",
        ),
        // YAML 1.1 octal: a bare leading zero means base 8.
        ("0777", |c| c.legacy_octal_numbers = true, "777", "511"),
        // YAML 1.1 booleans beyond true/false.
        ("yes", |c| c.legacy_booleans = true, "yes", "true"),
        ("off", |c| c.legacy_booleans = true, "off", "false"),
    ];
    for (input, set, want_off, want_on) in cases {
        let yaml = format!("k: {input}\n");

        let off: Value = from_str_with_config(&yaml, &ParserConfig::new())
            .unwrap_or_else(|e| panic!("{input:?} with the flag off: {e}"));
        let mut cfg = ParserConfig::new();
        set(&mut cfg);
        let on: Value = from_str_with_config(&yaml, &cfg)
            .unwrap_or_else(|e| panic!("{input:?} with the flag on: {e}"));

        let render = |v: &Value| match v.get("k") {
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => format!("{other:?}")
                .trim_start_matches("Number(")
                .trim_end_matches(')')
                .to_string(),
            None => "<missing>".to_string(),
        };
        let (got_off, got_on) = (render(&off), render(&on));
        assert!(
            got_off.contains(want_off),
            "{input:?} with the flag off: wanted {want_off:?}, got {got_off:?}"
        );
        assert!(
            got_on.contains(want_on),
            "{input:?} with the flag on: wanted {want_on:?}, got {got_on:?}"
        );
        assert_ne!(
            got_off, got_on,
            "{input:?}: the flag made no difference, so it is not being read"
        );
    }
}

/// Without the legacy flag, `1:30` is a plain string — the YAML 1.2
/// behaviour, and the reason the branch above has to be opt-in.
#[test]
fn a_colon_scalar_is_a_string_by_default() {
    let m: BTreeMap<String, String> =
        from_str("a: 1:30\nb: 1:00:00\n").expect("colon scalars are strings");
    assert_eq!(m["a"], "1:30");
    assert_eq!(m["b"], "1:00:00");
}

/// A quoted scalar that *looks* like an integer must stay a string —
/// the branch that distinguishes them is otherwise unreached.
#[test]
fn a_quoted_integer_like_scalar_stays_a_string() {
    let m: BTreeMap<String, String> =
        from_str("a: \"12\"\nb: '0x1f'\nc: \"1:30\"\n").expect("quoted scalars");
    assert_eq!(m["a"], "12");
    assert_eq!(m["b"], "0x1f");
    assert_eq!(m["c"], "1:30");
}
