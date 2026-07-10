// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Integration tests for `TagRegistry` — custom-tag pass-through on the
//! streaming deserialization path.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use noyalib::{Error, ParserConfig, TagRegistry, Value, from_str_with_config};
use serde::Deserialize;

fn cfg(tags: &[&str]) -> ParserConfig {
    let mut reg = TagRegistry::new();
    for t in tags {
        let _ = reg.register(*t);
    }
    ParserConfig::new().tag_registry(Arc::new(reg))
}

// ── Scalar newtype pass-through ──────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq)]
struct Celsius(f64);

#[test]
fn scalar_tag_registered_deserializes_to_newtype() {
    let cfg = cfg(&["!Celsius"]);
    let c: Celsius = from_str_with_config("!Celsius 42.5", &cfg).unwrap();
    assert_eq!(c, Celsius(42.5));
}

#[test]
fn scalar_tag_not_registered_errors_on_newtype_without_registry() {
    // `Celsius(f64)` is a newtype — without the registry, the streaming
    // path wraps the tagged value as `{tag: "!Celsius", value: ...}`
    // which a newtype visitor rejects. This is the exact frustration
    // the registry is designed to fix.
    let cfg = ParserConfig::new();
    let res: Result<Celsius, _> = from_str_with_config("!Celsius 42.5", &cfg);
    assert!(res.is_err(), "expected error without registry, got {res:?}");
}

#[test]
fn multiple_tags_in_registry() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Meters(f64);
    #[derive(Debug, Deserialize, PartialEq)]
    struct Seconds(f64);

    let cfg = cfg(&["!Meters", "!Seconds"]);
    let m: Meters = from_str_with_config("!Meters 10.0", &cfg).unwrap();
    let s: Seconds = from_str_with_config("!Seconds 2.5", &cfg).unwrap();
    assert_eq!(m, Meters(10.0));
    assert_eq!(s, Seconds(2.5));
}

// ── Registered tag on a sequence target ──────────────────────────────

#[test]
fn registered_tag_on_sequence_strips_and_deserializes() {
    let cfg = cfg(&["!MyVec"]);
    let v: Vec<i32> = from_str_with_config("!MyVec [1, 2, 3]", &cfg).unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

// ── Registered tag on a mapping target ───────────────────────────────

#[test]
fn registered_tag_on_mapping_strips_and_deserializes() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }
    let cfg = cfg(&["!Point"]);
    let p: Point = from_str_with_config("!Point {x: 1, y: 2}", &cfg).unwrap();
    assert_eq!(p, Point { x: 1, y: 2 });
}

// ── Tag not in registry still works — just routes via AST ───────────

#[test]
fn unregistered_tag_on_scalar_falls_back_to_string() {
    // Without the registry, the AST resolver surfaces unknown
    // custom tags as `Value::Tagged(tag, String(<inner>))` so the
    // tag is queryable downstream. The registry opts *out of*
    // that wrapper and lets the typed target see through the
    // tag — the path the strongly-typed `Celsius` test uses.
    let cfg = ParserConfig::new();
    let v: Value = from_str_with_config("!Other 42", &cfg).unwrap();
    let Value::Tagged(t) = &v else {
        panic!("expected Tagged, got {v:?}");
    };
    assert_eq!(t.tag().as_str(), "!Other");
    assert_eq!(t.value().as_str(), Some("42"));
}

// ── Registry does not affect core YAML tags ──────────────────────────

#[test]
fn core_tags_unaffected_by_registry() {
    // Registering `!!str` is a no-op — core tags carry semantic
    // information the AST resolver must see (forces `!!str 42` to
    // deserialize as the string "42", not the integer 42).
    let cfg = cfg(&["!!str", "!!int"]);
    let s: String = from_str_with_config("!!str 42", &cfg).unwrap();
    assert_eq!(s, "42");
}

// ── Empty registry == no registry ───────────────────────────────────

#[test]
fn empty_registry_is_no_op() {
    let cfg = ParserConfig::new().tag_registry(Arc::new(TagRegistry::new()));
    // Empty registry is equivalent to no registry — same AST
    // routing + tag-preserving deserialise. The custom tag
    // surfaces as `Value::Tagged`; opt out by registering it.
    let v: Value = from_str_with_config("!Other 42", &cfg).unwrap();
    let Value::Tagged(t) = &v else {
        panic!("expected Tagged, got {v:?}");
    };
    assert_eq!(t.tag().as_str(), "!Other");
    assert_eq!(t.value().as_str(), Some("42"));
}

// ── Shared registry across configs ───────────────────────────────────

#[test]
fn arc_registry_shares_across_configs() {
    let reg = Arc::new(TagRegistry::new().with("!Celsius"));
    let cfg_a = ParserConfig::new().tag_registry(Arc::clone(&reg));
    let cfg_b = ParserConfig::strict().tag_registry(Arc::clone(&reg));

    let a: Celsius = from_str_with_config("!Celsius 1.0", &cfg_a).unwrap();
    let b: Celsius = from_str_with_config("!Celsius 2.0", &cfg_b).unwrap();
    assert_eq!(a, Celsius(1.0));
    assert_eq!(b, Celsius(2.0));
}

// ── StreamingDeserializer direct-use path ────────────────────────────

#[test]
fn streaming_deserializer_with_tag_registry_direct() {
    use noyalib::StreamingDeserializer;
    let reg = Arc::new(TagRegistry::new().with("!Celsius"));
    let mut de = StreamingDeserializer::new("!Celsius 98.6").with_tag_registry(reg);
    let c = Celsius::deserialize(&mut de).unwrap();
    assert_eq!(c, Celsius(98.6));
}

// ── Registry accessors round-trip ────────────────────────────────────

#[test]
fn registry_contains_matches_registered() {
    let reg = TagRegistry::new().with("!a").with("!b");
    assert!(reg.contains("!a"));
    assert!(reg.contains("!b"));
    assert!(!reg.contains("!c"));
    assert_eq!(reg.len(), 2);
    assert!(!reg.is_empty());
}

// ── Value target + registry: KeyCollision guard + streaming parity ───
//
// v0.0.14 review regression. `from_str::<Value>` WITH a tag registry
// active used to route through the streaming path, which stringifies
// keys before the distinct-typed `KeyCollision` guard can run — so `1`
// and `"1"` silently collapsed. `Value`+registry now uses the AST loader
// (which owns the guard) and the loader strips registered tags itself, so
// its output matches what streaming produced.

#[test]
fn value_target_with_registry_detects_distinct_typed_key_collision() {
    let cfg = cfg(&["!Celsius"]);
    let res: Result<Value, _> = from_str_with_config("1: a\n\"1\": b\n", &cfg);
    assert!(
        matches!(res, Err(Error::KeyCollision(_))),
        "Value+registry must detect the 1 vs \"1\" collision, got {res:?}"
    );
    // Parity with the no-registry Value path, which already errored.
    let res2: Result<Value, _> = from_str_with_config("1: a\n\"1\": b\n", &ParserConfig::new());
    assert!(matches!(res2, Err(Error::KeyCollision(_))));
}

#[test]
fn value_target_registry_strip_is_style_correct() {
    let cfg = cfg(&["!Celsius"]);
    // Plain scalar under a registered tag: stripped, then schema-resolved
    // to a number — exactly what streaming yields.
    let v: Value = from_str_with_config("!Celsius 42", &cfg).unwrap();
    assert_eq!(v.as_i64(), Some(42));
    // Quoted scalar under the same tag: stripped, but the quoted STYLE is
    // honoured so it stays a string. A post-parse Value walk could not know
    // this (the AST tagged node discards style) — proving the strip must
    // happen at parse time.
    let v: Value = from_str_with_config("!Celsius \"42\"", &cfg).unwrap();
    assert_eq!(v.as_str(), Some("42"));
    // Without the registry the tag is preserved as Value::Tagged.
    let v: Value = from_str_with_config("!Celsius 42", &ParserConfig::new()).unwrap();
    assert!(
        v.as_tagged().is_some(),
        "unregistered tag must stay tagged, got {v:?}"
    );
}

#[test]
fn value_target_registry_strips_collection_tag() {
    // A registered tag on a collection is stripped too (streaming parity),
    // exposing the bare mapping rather than a Value::Tagged wrapper.
    let cfg = cfg(&["!Config"]);
    let v: Value = from_str_with_config("!Config\na: 1\nb: 2\n", &cfg).unwrap();
    assert!(
        v.as_tagged().is_none(),
        "registered collection tag must be stripped, got {v:?}"
    );
    assert_eq!(v["a"].as_i64(), Some(1));
    assert_eq!(v["b"].as_i64(), Some(2));
}

// ── Borrowing entry honours the registry ─────────────────────────────

#[test]
fn borrowing_entry_attaches_tag_registry() {
    // Every other registry test drives the *owning*
    // `from_str_with_config`. This exercises the *borrowing* entry
    // (`from_str_borrowing_with_config`), whose registry-attach branch
    // (`de.rs` line 153) is otherwise never taken. A registered
    // `!Celsius` must be stripped so the inner scalar deserialises into
    // the newtype exactly as on the owning path.
    let cfg = cfg(&["!Celsius"]);
    let c: Celsius = noyalib::from_str_borrowing_with_config("!Celsius 42.5", &cfg).unwrap();
    assert_eq!(c, Celsius(42.5));
}

#[test]
fn borrowing_entry_without_registry_keeps_tag_opaque() {
    // Control for the test above: with no registry installed the
    // borrowing entry must *not* strip the tag, so the newtype visitor
    // rejects the wrapped `{tag, value}` shape.
    let res: Result<Celsius, Error> =
        noyalib::from_str_borrowing_with_config("!Celsius 42.5", &ParserConfig::new());
    assert!(res.is_err(), "unregistered tag must not strip: {res:?}");
}
