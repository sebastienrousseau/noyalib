// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Tagged values and type refusals, driven through the streaming reader.
//!
//! `noyalib::from_str` dispatches between two readers, so a test that
//! goes through it measures whichever one the dispatcher picked. These
//! name [`StreamingDeserializer`] directly.
//!
//! Two clusters live here. A tagged value deserialized into a map
//! becomes a single entry keyed by the tag, which is how a caller reads
//! `!Colour '#ff8800'` into a `BTreeMap` — the tag has to be rebuilt
//! from its handle and suffix, and the primary handle `!` is spelled
//! differently from the rest. And every `deserialize_*` refusal for a
//! scalar that cannot be the type asked for.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use noyalib::StreamingDeserializer;
use serde::Deserialize;
use serde_bytes::ByteBuf;

#[track_caller]
fn streamed<T>(label: &str, yaml: &str) -> Result<T, noyalib::Error>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    let mut de = StreamingDeserializer::new(yaml);
    let out = T::deserialize(&mut de);
    if let Err(e) = &out {
        assert!(!e.to_string().is_empty(), "{label}: empty error");
    }
    out
}

// ── a tagged value read as a single-entry mapping ───────────────

/// The tag becomes the key. Its handle and suffix are stored apart and
/// rebuilt, and the primary handle (`!`) is spelled differently from a
/// named or secondary one — so each spelling takes its own arm.
#[test]
fn a_tagged_scalar_reads_as_one_entry_keyed_by_its_tag() {
    let cases: &[(&str, &str, &str)] = &[
        ("primary handle", "!Colour '#ff8800'\n", "Colour"),
        ("secondary handle", "!!str hello\n", "str"),
        ("a dotted suffix", "!my.tag value\n", "my.tag"),
    ];
    for (label, yaml, suffix) in cases {
        // Refusing is a defensible contract; a silently wrong value is
        // not, and `streamed` already checked the error is not empty.
        if let Ok(m) = streamed::<BTreeMap<String, String>>(label, yaml) {
            assert_eq!(m.len(), 1, "{label}: not a single entry: {m:?}");
            let (k, _) = m.iter().next().expect("one entry");
            assert!(
                k.contains(suffix),
                "{label}: key {k:?} does not carry the tag suffix {suffix:?}"
            );
            assert!(k.starts_with('!'), "{label}: key {k:?} is not a tag");
        }
    }
}

/// The same, with a collection under the tag rather than a scalar.
#[test]
fn a_tagged_collection_reads_as_one_entry_keyed_by_its_tag() {
    for (label, yaml) in [
        ("a mapping", "!Wrapper\nk: 1\n"),
        ("a sequence", "!Wrapper\n- 1\n- 2\n"),
    ] {
        let _ = streamed::<BTreeMap<String, noyalib::Value>>(label, yaml);
    }
}

// ── refusals ────────────────────────────────────────────────────

/// A string asked for where the scalar resolves to something else, and
/// where the node is not a scalar at all — two different arms.
#[test]
fn a_string_field_refuses_both_a_typed_scalar_and_a_collection() {
    #[derive(Deserialize, Debug)]
    struct HasString {
        #[allow(dead_code)]
        s: String,
    }
    for (label, yaml) in [
        ("an integer", "s: 1\n"),
        ("a float", "s: 1.5\n"),
        ("a boolean", "s: true\n"),
        ("null", "s: ~\n"),
        ("a sequence", "s: [1]\n"),
        ("a mapping", "s: {k: 1}\n"),
    ] {
        assert!(
            streamed::<HasString>(label, yaml).is_err(),
            "{label}: {yaml:?} was accepted as a String"
        );
    }
}

/// `!!binary` whose content is not string-shaped, and bytes asked for
/// where each other scalar kind sits.
#[test]
fn bytes_are_refused_for_every_shape_that_cannot_supply_them() {
    #[derive(Deserialize, Debug)]
    struct HasBytes {
        #[allow(dead_code)]
        b: ByteBuf,
    }
    for (label, yaml) in [
        ("a sequence under !!binary", "b: !!binary [1, 2]\n"),
        ("a mapping under !!binary", "b: !!binary {k: 1}\n"),
        ("invalid base64", "b: !!binary \"not base64!!\"\n"),
        ("a bare integer", "b: 1\n"),
        ("a bare float", "b: 1.5\n"),
        ("a bare boolean", "b: true\n"),
        ("a bare null", "b: ~\n"),
        ("a bare sequence", "b: [1]\n"),
        ("a bare mapping", "b: {k: 1}\n"),
    ] {
        assert!(
            streamed::<HasBytes>(label, yaml).is_err(),
            "{label}: {yaml:?} was accepted as bytes"
        );
    }

    // The control: a well-formed one decodes, so the refusals above are
    // about the shape and not about `!!binary` being broken.
    let ok = streamed::<HasBytes>("valid base64", "b: !!binary \"aGVsbG8=\"\n");
    assert!(ok.is_ok(), "a valid !!binary payload must decode");
}

/// An enum variant name has to come from a scalar; a collection in that
/// position hits the identifier refusal.
#[test]
fn a_variant_name_must_be_a_scalar() {
    #[derive(Deserialize, Debug)]
    enum E {
        A,
        #[allow(dead_code)]
        B(i64),
    }
    for (label, yaml) in [
        ("a sequence as the variant", "k: [1, 2]\n"),
        ("a mapping as the variant", "k: {x: {y: 1}}\n"),
    ] {
        assert!(
            streamed::<BTreeMap<String, E>>(label, yaml).is_err(),
            "{label}: accepted a non-scalar variant name"
        );
    }

    // The control: both real variant shapes work through this reader.
    let m = streamed::<BTreeMap<String, E>>("unit", "k: A\n").expect("unit variant");
    assert!(matches!(m["k"], E::A));
    let m = streamed::<BTreeMap<String, E>>("newtype", "k:\n  B: 7\n").expect("newtype variant");
    assert!(matches!(m["k"], E::B(7)));
}

// ── anchors recorded mid-stream ─────────────────────────────────

/// An alias that appears while an anchor is still being recorded closes
/// the recording — the buffered events become that anchor's definition.
/// It needs an anchor whose own body contains an alias, which no flat
/// fixture produces.
#[test]
fn an_anchor_whose_body_contains_an_alias_is_recorded_correctly() {
    let cases: &[(&str, &str)] = &[
        (
            "an alias inside an anchored mapping",
            "first: &a 1\nsecond: &b\n  inner: *a\nthird:\n  <<: *b\n",
        ),
        (
            "an alias inside an anchored sequence",
            "first: &a 1\nsecond: &b\n  - *a\n  - 2\nthird: *b\n",
        ),
        (
            "an anchored alias at the top level",
            "first: &a 1\nsecond: *a\nthird: *a\n",
        ),
    ];
    for (label, yaml) in cases {
        let v = streamed::<BTreeMap<String, noyalib::Value>>(label, yaml)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        assert!(
            v.contains_key("third"),
            "{label}: lost the last entry: {v:?}"
        );
    }
}
