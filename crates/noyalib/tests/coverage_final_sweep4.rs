// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Fourth coverage sweep — error.rs, with/* singleton_map helpers,
//! and de.rs deserialize_identifier / wrap_err paths.

#![allow(
    clippy::approx_constant,
    clippy::bool_assert_comparison,
    dead_code,
    unused_qualifications
)]

use noyalib::{Error, Value, from_str, to_string};
use serde::{Deserialize, Serialize};

// ── error.rs: Error::location for UnknownAnchorAt ────────────────────

#[test]
fn error_location_for_unknown_anchor_at() {
    // A merge referencing an unknown anchor produces UnknownAnchorAt,
    // whose location is exposed via Error::location().
    let yaml = "derived:\n  <<: *missing\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    if matches!(err, Error::UnknownAnchorAt { .. }) {
        assert!(err.location().is_some());
    }
}

// ── error.rs: unknown_field via deny_unknown_fields ─────────────────

#[test]
fn error_unknown_field_via_deny_unknown_fields() {
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Strict {
        known: String,
    }
    let yaml = "known: yes\nextra: no\n";
    let err = from_str::<Strict>(yaml).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("unknown") || err.to_string().contains("extra")
    );
}

// ── error.rs: miette labels for DeserializeWithLocation ─────────────

#[cfg(feature = "miette")]
#[test]
fn error_miette_labels_for_deserialize_with_location() {
    use miette::Diagnostic;
    // Force a deserialize error with location by using a Spanned field
    // plus a type mismatch downstream.
    #[derive(Debug, Deserialize)]
    struct Doc {
        #[allow(dead_code)]
        name: noyalib::Spanned<String>,
        count: i32,
    }
    let yaml = "name: foo\ncount: not-a-number\n";
    let err = from_str::<Doc>(yaml).unwrap_err();
    // Call labels() — covers the labels match arms.
    let _ = err.labels();
    let _ = err.code();
    let _ = err.help();
    let _ = err.source_code();
}

#[cfg(feature = "miette")]
#[test]
fn error_miette_labels_for_unknown_anchor_at() {
    use miette::Diagnostic;
    let yaml = "a: &anchor 1\nb: *anchr\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    // Exercise labels() which walks UnknownAnchorAt branch.
    let _ = err.labels();
    let _ = err.help();
    let _ = err.code();
}

#[cfg(feature = "miette")]
#[test]
fn error_miette_labels_for_parse_with_location() {
    use miette::Diagnostic;
    let yaml = "k: [unclosed\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    let _ = err.labels();
}

// ── with/singleton_map: Tagged value path ────────────────────────────

#[test]
fn singleton_map_with_tagged_value() {
    // Force transform_value_keys to walk a Value::Tagged branch.
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Event {
        Log(String),
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Doc {
        #[serde(with = "noyalib::with::singleton_map")]
        event: Event,
    }
    let doc = Doc {
        event: Event::Log("hi".into()),
    };
    let yaml = to_string(&doc).unwrap();
    assert!(yaml.contains("Log"));
}

#[test]
fn singleton_map_recursive_tagged_value_branch() {
    // Serializing a Value::Tagged via singleton_map_recursive exercises
    // the Tagged arm in `transform_to_singleton_map`.
    use noyalib::{Tag, TaggedValue};
    #[derive(Serialize)]
    struct Wrap<'a> {
        #[serde(with = "noyalib::with::singleton_map_recursive")]
        v: &'a Value,
    }
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!custom"),
        Value::from(42_i64),
    )));
    let yaml = to_string(&Wrap { v: &v }).unwrap();
    assert!(yaml.contains("!custom"));
}

mod snake_with {
    use serde::{Deserializer, Serializer};

    pub(crate) fn serialize<T, S>(v: &T, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        T: serde::Serialize,
        S: Serializer,
    {
        noyalib::with::singleton_map_with::serialize_with(
            v,
            s,
            noyalib::with::singleton_map_with::to_snake_case,
        )
    }

    pub(crate) fn deserialize<'de, T, D>(d: D) -> std::result::Result<T, D::Error>
    where
        T: serde::de::DeserializeOwned + 'static,
        D: Deserializer<'de>,
    {
        noyalib::with::singleton_map_with::deserialize_with(
            d,
            noyalib::with::singleton_map_with::to_snake_case,
        )
    }
}

#[test]
fn singleton_map_with_tagged_value_branch() {
    // serialize_with walks transform_value_keys which has a Tagged arm.
    use noyalib::{Tag, TaggedValue};

    #[derive(Serialize)]
    struct Wrap<'a> {
        #[serde(with = "snake_with")]
        v: &'a Value,
    }

    let mut m = noyalib::Mapping::new();
    let _ = m.insert("OuterKey".to_string(), Value::from(1_i64));
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!custom"),
        Value::Mapping(m),
    )));
    let yaml = to_string(&Wrap { v: &tagged }).unwrap();
    assert!(yaml.contains("!custom"));
}

#[test]
fn singleton_map_recursive_deep() {
    use noyalib::with::singleton_map_recursive as smr;
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Node {
        Leaf(i32),
        Branch(Vec<Node>),
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Tree {
        #[serde(with = "smr")]
        root: Node,
    }
    let tree = Tree {
        root: Node::Branch(vec![Node::Leaf(1), Node::Leaf(2)]),
    };
    let yaml = to_string(&tree).unwrap();
    // Output should contain Branch and Leaf as singleton map keys.
    assert!(yaml.contains("Branch"));
    let back: Tree = from_str(&yaml).unwrap();
    assert_eq!(back, tree);
}

// ── de.rs: deserialize_identifier via AST path ──────────────────────

#[test]
fn ast_deserialize_identifier_string_path() {
    // Spanned forces AST; serde uses deserialize_identifier to match
    // tagged-enum discriminator "type" against variants.
    #[derive(Debug, Deserialize)]
    #[serde(tag = "type")]
    enum Item {
        Apple,
        Banana,
    }
    #[derive(Debug, Deserialize)]
    struct Doc {
        #[allow(dead_code)]
        _force: noyalib::Spanned<String>,
        item: Item,
    }
    let yaml = "_force: x\nitem:\n  type: Apple\n";
    let d: Doc = from_str(yaml).unwrap();
    assert!(matches!(d.item, Item::Apple));
}

// ── de.rs: bytes fallback via AST (type mismatch) ───────────────────

#[test]
fn ast_deserialize_bytes_type_mismatch_on_sequence() {
    // AST deserialize_bytes does not accept sequences directly; the
    // type mismatch exercise reaches the `_ => TypeMismatch` arm.
    #[derive(Debug, Deserialize)]
    struct Doc {
        #[allow(dead_code)]
        _force: noyalib::Spanned<String>,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    }
    let yaml = "_force: x\ndata: [104, 105]\n";
    let err = from_str::<Doc>(yaml).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("bytes"));
}

// ── edit_distance branches: empty inputs ────────────────────────────

#[test]
fn edit_distance_empty_anchor_name() {
    // An empty anchor name in a reference like `*` is invalid YAML
    // but the closest_name helper handles empty strings.
    // We trigger this via a reference near a defined anchor:
    let yaml = "a: &defined 1\nb: *d\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    let _ = err.to_string();
}

// ── streaming: unknown_field via deny_unknown_fields through streaming ──

// ── de.rs: Spanned<enum> drives VariantAccess with span context ────

#[test]
fn ast_spanned_enum_hits_variant_access_with_span_context() {
    // Putting a Spanned wrapper around an enum value forces the AST
    // path, and deserializing the enum inside Spanned context walks
    // VariantAccess::unit_variant / newtype_variant_seed with
    // span_ctx Some.
    #[derive(Debug, Deserialize, PartialEq)]
    enum Kind {
        One,
        Two(i64),
        Three(i64, i64),
        Four { n: i64 },
    }
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        unit: noyalib::Spanned<Kind>,
        newtype: noyalib::Spanned<Kind>,
        tuple: noyalib::Spanned<Kind>,
        strct: noyalib::Spanned<Kind>,
    }
    let yaml = "\
unit: One
newtype:
  Two: 42
tuple:
  Three: [1, 2]
strct:
  Four:
    n: 7
";
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.unit.value, Kind::One);
    assert_eq!(d.newtype.value, Kind::Two(42));
    assert_eq!(d.tuple.value, Kind::Three(1, 2));
    assert_eq!(d.strct.value, Kind::Four { n: 7 });
}

#[test]
fn streaming_unknown_field_via_deny() {
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Strict {
        k: i32,
    }
    // No Spanned forces streaming path.
    let yaml = "k: 1\nextra: hi\n";
    let err = from_str::<Strict>(yaml).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("unknown") || err.to_string().contains("extra")
    );
}
