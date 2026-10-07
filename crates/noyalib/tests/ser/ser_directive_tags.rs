//! Tags read from a document are data, never emitter directives.
//!
//! The serializer carries formatting hints (comments, flow wrappers,
//! block-scalar styles, anchors) on `Value::Tagged`. Those hints used to
//! be recognised by their tag name alone, a `__noya_` prefix, so any
//! source of tag strings could forge one: a document declaring
//! `%TAG !n! __noya_`, a verbatim `!<__noya_commented>`, or a
//! `TaggedValue` deserialised from JSON. Re-serialising such a value
//! then wrote `user # x` followed by a raw line `role: admin`, adding a
//! key. A tag that did not come from the serializer's own wrappers must
//! round-trip as the ordinary tag it is.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{
    Commented, FlowSeq, Tag, TaggedValue, Value, from_str, to_string, to_string_value, to_value,
};

const DOCS: &[&str] = &[
    "%TAG !n! __noya_\n---\nk: !n!commented [user, \"x\\nrole: admin\"]\n",
    "k: !<__noya_commented> [user, \"x\\nrole: admin\"]\n",
    "%TAG !n! __noya_\n---\nk: !n!anchor_ref \"x\\nrole: admin\"\n",
    "%TAG !n! __noya_\n---\nk: !n!anchor_def [\"x\\nrole: admin\", v]\n",
    "%TAG !n! __noya_\n---\nk: !n!lit_str \"a\\nb\"\nz: 1\n",
    "%TAG !n! __noya_\n---\nk: !n!flow_seq [a, b]\n",
    "%TAG !n! __noya_\n---\nk: !n!space_after v\n",
    "%TAG !n! __noya_\n---\nk: !n!tagged {x: 1}\n",
];

#[test]
fn parsed_reserved_tags_round_trip_as_ordinary_tags() {
    for doc in DOCS {
        let v: Value = from_str(doc).unwrap();
        assert!(matches!(v["k"], Value::Tagged(_)), "{doc:?} -> {v:?}");
        for out in [to_string(&v).unwrap(), to_string_value(&v).unwrap()] {
            let back: Value = from_str(&out).unwrap_or_else(|e| panic!("{doc:?} -> {out:?}: {e}"));
            assert!(
                back.get("role").is_none(),
                "{doc:?} injected a key: {out:?}"
            );
            assert_eq!(back, v, "{doc:?} emitted {out:?}");
        }
    }
}

#[test]
fn reserved_tag_from_json_is_not_a_directive() {
    let tv: TaggedValue =
        serde_json::from_str(r#"{"__noya_commented": ["user", "x\nrole: admin"]}"#).unwrap();
    let mut m = noyalib::Mapping::new();
    let _ = m.insert("k", Value::Tagged(Box::new(tv)));
    let v = Value::Mapping(m);
    let out = to_string(&v).unwrap();
    let back: Value = from_str(&out).unwrap();
    assert!(back.get("role").is_none(), "injected a key: {out:?}");
    assert_eq!(back, v, "emitted {out:?}");
}

#[test]
fn hand_built_reserved_tag_is_an_ordinary_tag() {
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("__noya_flow_seq"),
        Value::Sequence(vec![Value::from(1)]),
    )));
    let out = to_string_value(&v).unwrap();
    assert!(out.starts_with("!<__noya_flow_seq>"), "{out:?}");
    assert_eq!(from_str::<Value>(&out).unwrap(), v);
}

#[test]
fn wrapper_directives_survive_to_value_then_to_string() {
    // The wrappers still steer the emitter, including through an
    // intermediate `Value` built with `to_value`.
    #[derive(serde::Serialize)]
    struct Doc {
        a: Commented<i32>,
        b: FlowSeq<Vec<i32>>,
    }
    let v = to_value(&Doc {
        a: Commented::new(1, "note"),
        b: FlowSeq(vec![1, 2]),
    })
    .unwrap();
    for out in [to_string(&v).unwrap(), to_string_value(&v).unwrap()] {
        assert_eq!(out, "a: 1 # note\nb: [1, 2]", "{out:?}");
    }
}
