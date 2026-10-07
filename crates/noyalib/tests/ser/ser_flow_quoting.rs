//! Plain scalars inside flow collections.
//!
//! Inside `[...]` and `{...}` the flow indicators `,` `[` `]` `{` `}`
//! end or nest a collection wherever they appear in a plain scalar
//! (YAML 1.2 §7.3.3, `ns-plain-safe(c)`), not only at its first byte.
//! The serializer used to apply the block-context rule there, so
//! `["viewer, admin"]` was written as `[viewer, admin]` and read back as
//! two items, and a flow-mapping value `bob, admin` added a key. These
//! cases pin every flow entry point: `FlowStyle::Flow`, `FlowStyle::Auto`
//! and the `FlowSeq` / `FlowMap` wrappers under the default block style.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{
    FlowMap, FlowSeq, FlowStyle, Mapping, SerializerConfig, Value, from_str, to_string,
    to_string_with_config,
};
use std::collections::BTreeMap;

fn s(x: &str) -> Value {
    Value::String(x.to_owned())
}

fn map(pairs: &[(&str, Value)]) -> Value {
    let mut m = Mapping::new();
    for (k, v) in pairs {
        let _ = m.insert(*k, v.clone());
    }
    Value::Mapping(m)
}

fn flow_configs() -> [SerializerConfig; 2] {
    [
        SerializerConfig::new().flow_style(FlowStyle::Flow),
        SerializerConfig::new().flow_style(FlowStyle::Auto),
    ]
}

const HOSTILE: &[&str] = &[
    "viewer, admin",
    "bob, admin",
    "x,",
    ",x",
    "x]",
    "viewer], [admin",
    "x}",
    "x{y}",
    "a[0]",
    "x, b: y",
    "a: b",
    "k:,",
    "a #b",
];

#[track_caller]
fn assert_round_trip(v: &Value, cfg: &SerializerConfig) {
    let out = to_string_with_config(v, cfg).unwrap();
    let back: Value = from_str(&out).unwrap_or_else(|e| panic!("{out:?} failed to parse: {e}"));
    assert_eq!(&back, v, "emitted {out:?}");
}

#[test]
fn flow_sequence_items_with_flow_indicators_round_trip() {
    for cfg in &flow_configs() {
        for x in HOSTILE {
            let v = map(&[("groups", Value::Sequence(vec![s(x), s("z")]))]);
            assert_round_trip(&v, cfg);
        }
    }
}

#[test]
fn flow_mapping_values_and_keys_with_flow_indicators_round_trip() {
    for cfg in &flow_configs() {
        for x in HOSTILE {
            assert_round_trip(&map(&[("user", s(x)), ("role", s("user"))]), cfg);
            assert_round_trip(&map(&[(x, s("v")), ("role", s("user"))]), cfg);
        }
    }
}

#[test]
fn flow_value_cannot_add_a_key() {
    let cfg = SerializerConfig::new().flow_style(FlowStyle::Flow);
    let v = map(&[("user", s("bob, admin: true"))]);
    let out = to_string_with_config(&v, &cfg).unwrap();
    let back: Value = from_str(&out).unwrap();
    assert!(back.get("admin").is_none(), "emitted {out:?}");
    assert_eq!(back, v);
}

#[test]
fn flow_multi_line_strings_round_trip() {
    for cfg in &flow_configs() {
        for x in ["a\nb", "a\n", "\n", "a\n\nb\n\n", "  lead\nx"] {
            assert_round_trip(&Value::Sequence(vec![s(x)]), cfg);
            assert_round_trip(&map(&[("k", s(x))]), cfg);
        }
    }
}

#[test]
fn flow_wrappers_quote_flow_indicators() {
    #[derive(serde::Serialize)]
    struct Acl {
        groups: FlowSeq<Vec<String>>,
        attrs: FlowMap<BTreeMap<String, String>>,
    }
    let mut attrs = BTreeMap::new();
    let _ = attrs.insert("a, b".to_owned(), "bob, admin".to_owned());
    let acl = Acl {
        groups: FlowSeq(vec!["viewer, admin".into(), "x]".into()]),
        attrs: FlowMap(attrs),
    };
    let out = to_string(&acl).unwrap();
    let back: Value = from_str(&out).unwrap();
    assert_eq!(
        back["groups"],
        Value::Sequence(vec![s("viewer, admin"), s("x]")]),
        "emitted {out:?}"
    );
    assert_eq!(back["attrs"]["a, b"], s("bob, admin"), "emitted {out:?}");
}

#[test]
fn flow_wrapper_holding_block_shapes_stays_valid() {
    // A mapping, a multi-line string or a nested sequence inside a
    // `FlowSeq` must be written in flow form too: block layout inside
    // `[...]` does not parse.
    #[derive(serde::Serialize)]
    struct Doc {
        v: FlowSeq<Vec<Value>>,
    }
    let doc = Doc {
        v: FlowSeq(vec![
            map(&[("a", s("1")), ("b", Value::Sequence(vec![s("x, y")]))]),
            s("line1\nline2"),
        ]),
    };
    let out = to_string(&doc).unwrap();
    let back: Value = from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
    assert_eq!(back["v"], Value::Sequence(doc.v.0), "emitted {out:?}");
}

#[test]
fn plain_strings_stay_plain_in_flow() {
    let cfg = SerializerConfig::new().flow_style(FlowStyle::Flow);
    let v = map(&[("k", Value::Sequence(vec![s("a-b"), s("x.y"), s("a:b")]))]);
    assert_eq!(
        to_string_with_config(&v, &cfg).unwrap(),
        "{k: [a-b, x.y, a:b]}"
    );
}
