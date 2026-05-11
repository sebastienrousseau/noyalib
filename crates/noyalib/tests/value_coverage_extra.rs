// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Targeted line/region coverage for `crates/noyalib/src/value.rs`.
//!
//! Each test exercises a specific uncovered branch identified by
//! `cargo llvm-cov`. The grouping follows the file's logical
//! structure: `Mapping` / `MappingAny` Display + Serialize +
//! Deserialize, `Number` arithmetic edge cases (NaN / inf, large
//! integer comparisons), `TaggedValue` Serialize / Deserialize,
//! `TagPreservingMapAccess` state machine, `Value::query` /
//! `apply_merge` / `Display` walks, `ValueIndex` impls, and the
//! Serde Deserializer for `&Value`.

use std::collections::HashMap;

use noyalib::{
    Mapping, MappingAny, Number, Tag, TaggedValue, Value, from_str, from_value, to_string,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// Mapping — Display via multi-entry walk + comma branch (L790, L793, L795)
// ============================================================================

#[test]
fn mapping_display_multi_entry() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));
    let _ = m.insert("c", Value::from(3));
    let s = format!("{m}");
    assert!(s.starts_with('{'));
    assert!(s.ends_with('}'));
    // Two ", " separators on three entries — exercises L793.
    assert_eq!(s.matches(", ").count(), 2);
    assert!(s.contains("a: 1"));
    assert!(s.contains("c: 3"));
}

// ============================================================================
// Mapping — Serialize impl (L807, L809) — round-trip via to_string
// ============================================================================

#[test]
fn mapping_serialize_roundtrip() {
    let mut m = Mapping::new();
    let _ = m.insert("k1", Value::from("v1"));
    let _ = m.insert("k2", Value::from(2));
    let yaml = to_string(&m).expect("serialize Mapping");
    assert!(yaml.contains("k1"));
    assert!(yaml.contains("k2"));
}

// ============================================================================
// Mapping — Deserialize impl (L836)
// ============================================================================

#[test]
fn mapping_deserialize_directly() {
    // Direct Deserialize<Mapping> path — distinct from from_str::<Value>.
    let yaml = "alpha: 1\nbeta: 2\n";
    let m: Mapping = from_str(yaml).expect("deserialize Mapping");
    assert_eq!(m.get("alpha").unwrap().as_i64(), Some(1));
    assert_eq!(m.get("beta").unwrap().as_i64(), Some(2));
}

// ============================================================================
// MappingAny — Display, Serialize, Deserialize (L1272, L1275, L1277, L1289,
// L1291, L1318)
// ============================================================================

#[test]
fn mapping_any_display_multi_entry() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from(1), Value::from("one"));
    let _ = m.insert(Value::from(2), Value::from("two"));
    let _ = m.insert(Value::from(3), Value::from("three"));
    let s = format!("{m}");
    assert!(s.starts_with('{'));
    assert!(s.ends_with('}'));
    assert_eq!(s.matches(", ").count(), 2);
}

#[test]
fn mapping_any_serialize_roundtrip() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let _ = m.insert(Value::from("b"), Value::from(2));
    // Mapping with string keys can serialize to YAML; this hits the
    // MappingAny Serialize impl.
    let yaml = to_string(&m).expect("serialize MappingAny");
    assert!(yaml.contains("a"));
    assert!(yaml.contains("b"));
}

#[test]
fn mapping_any_deserialize_directly() {
    let yaml = "alpha: 1\nbeta: 2\n";
    let m: MappingAny = from_str(yaml).expect("deserialize MappingAny");
    assert_eq!(m.len(), 2);
}

// ============================================================================
// Number — large integer vs float comparison (L1702, L1707)
// ============================================================================

#[test]
fn number_cmp_large_integer_vs_float_round_trip() {
    // 1 << 54 — outside the i64→f64 safe range (>2^53), but the
    // value itself is power-of-two so the cast is lossless.
    let big = 1_i64 << 54;
    let i = Number::Integer(big);
    let f = Number::Float(big as f64);
    // Equal under IEEE semantics — exercises the (a_f as i64) == *a
    // branch (L1693-L1695).
    assert_eq!(i.cmp(&f), std::cmp::Ordering::Equal);
}

#[test]
fn number_cmp_large_negative_integer_vs_float() {
    // Negative side of L1704-1707.
    let big = -(1_i64 << 60) - 1;
    let i = Number::Integer(big);
    let f = Number::Float(-1.0e30);
    let _ = i.cmp(&f);
    let _ = f.cmp(&i);
}

#[test]
fn number_cmp_large_pos_integer_with_small_float() {
    // a > 0 and b < (1<<53) as f64 → Ordering::Greater (L1700).
    let big = (1_i64 << 60) + 1;
    let i = Number::Integer(big);
    let f = Number::Float(1.0);
    assert!(i > f);
    assert!(f < i);
}

#[test]
fn number_cmp_nan_int() {
    let i = Number::Integer(0);
    let nan = Number::Float(f64::NAN);
    // Integer < NaN always (L1689).
    assert!(i < nan);
    assert!(nan > i);
}

// ============================================================================
// TaggedValue — Serialize (L2130-L2131) + Deserialize (L2157-L2158)
// ============================================================================

#[test]
fn tagged_value_serialize_direct() {
    let tv = TaggedValue::new(Tag::new("!Custom"), Value::from(42));
    let s = to_string(&tv).expect("serialize TaggedValue");
    assert!(s.contains("Custom"));
    assert!(s.contains("42"));
}

#[test]
fn tagged_value_deserialize_direct() {
    // Single-entry map shape — TaggedValueVisitor consumes it.
    // Use plain mapping syntax so the YAML loader produces a
    // mapping rather than a tagged scalar.
    let yaml = "myTag: 42\n";
    let tv: TaggedValue = from_str(yaml).expect("deserialize TaggedValue");
    // The tag is whatever the visitor surfaced as the first key.
    assert_eq!(tv.value().as_i64(), Some(42));
    assert_eq!(tv.tag().as_str(), "myTag");
}

#[test]
fn tagged_value_deserialize_empty_map_errors() {
    // Zero-entry map → expected-single-entry error path.
    let yaml = "{}\n";
    let r: Result<TaggedValue, _> = from_str(yaml);
    assert!(r.is_err());
}

// ============================================================================
// TagPreservingMapAccess — round-trip a Tagged value via Value
// (covers L2225, L2254-L2258, L2264, L2268-L2273, L2276, L2278-L2282, L2285,
//  L2287, L2292, L2297, L2299, L2303-L2307, L2310-L2311, L2321-L2322, L2325,
//  L2329)
// ============================================================================

#[test]
fn tag_preserving_round_trip_via_from_value() {
    // `from_value::<Value>` short-circuits via clone, so build a
    // tagged tree and round-trip through from_str instead — that
    // engages the TagPreservingMapAccess state machine.
    let yaml = "!MyTag\nfoo\n";
    let v: Value = from_str(yaml).expect("from_str tagged scalar");
    assert!(v.is_tagged());
    let tv = v.as_tagged().expect("Value::Tagged");
    assert_eq!(tv.tag().as_str(), "!MyTag");
    assert_eq!(tv.value().as_str(), Some("foo"));
}

#[test]
fn tag_preserving_round_trip_nested_tagged_sequence() {
    // Tagged sequence containing a tagged scalar — exercises the
    // recursive `with_options_preserving_tags` descent inside
    // `TagPreservingMapAccess::next_value_seed` (L2310-L2323).
    let yaml = "!Outer\n- !Inner foo\n- !Inner bar\n";
    let v: Value = from_str(yaml).expect("nested tagged");
    assert!(v.is_tagged());
}

#[test]
fn tag_preserving_round_trip_through_from_value_value() {
    // `from_value::<Value>` triggers the clone short-circuit, but
    // the nested re-serialization through Deserialize still
    // exercises `Value::Tagged` reconstruction.
    let inner = Value::String("payload".into());
    let tag = Tag::new("!My");
    let tagged = Value::Tagged(Box::new(TaggedValue::new(tag, inner)));
    let v: Value = from_value(&tagged).expect("from_value clone path");
    assert!(v.is_tagged());
}

// ============================================================================
// Value::apply_merge — sequence + tagged recursion (L2937-L2939, L3201, L3206)
// ============================================================================

#[test]
fn apply_merge_inside_sequence() {
    let mut v: Value =
        from_str("- a: &x\n    k: 1\n- <<: *x\n  m: 2\n").expect("parse merge in seq");
    v.apply_merge().expect("apply_merge over Sequence");
    let seq = v.as_sequence().unwrap();
    assert_eq!(seq[1].get("k").unwrap().as_i64(), Some(1));
}

#[test]
fn apply_merge_inside_tagged() {
    let mut v: Value = from_str("!Cfg\na: &x {k: 1}\nb:\n  <<: *x\n").expect("parse tagged merge");
    v.apply_merge().expect("apply_merge over Tagged");
    // Walk past the tag.
    let inner = v.untag_ref();
    let mapping = inner.as_mapping().unwrap();
    let b = mapping.get("b").unwrap();
    assert_eq!(b.get("k").unwrap().as_i64(), Some(1));
}

#[test]
fn get_path_mut_wildcard_returns_none() {
    // L3166: get_path_mut with a wildcard is documented to return None.
    let mut v: Value = from_str("a: 1\n").expect("parse");
    assert!(v.get_path_mut("a.*").is_none());
    assert!(v.get_path_mut("**.a").is_none());
}

// ============================================================================
// query_recursive — wildcard + recursive descent into Tagged (L3501,
// L3504-L3506, L3514, L3524-L3526, L3534-L3535)
// ============================================================================

#[test]
fn query_wildcard_into_sequence() {
    let v: Value = from_str("- 1\n- 2\n- 3\n").expect("parse seq");
    let r = v.query("*");
    assert_eq!(r.len(), 3);
}

#[test]
fn query_wildcard_into_mapping() {
    let v: Value = from_str("a: 1\nb: 2\n").expect("parse map");
    let r = v.query("*");
    assert_eq!(r.len(), 2);
}

#[test]
fn query_wildcard_no_match_on_scalar() {
    let v = Value::from(42_i64);
    let r = v.query("*");
    assert_eq!(r.len(), 0);
}

#[test]
fn query_recursive_descent_into_sequence() {
    let v: Value = from_str("data:\n  - name: alpha\n  - name: beta\n").expect("parse seq of map");
    let r = v.query("..name");
    assert_eq!(r.len(), 2);
}

#[test]
fn query_recursive_descent_into_tagged() {
    let v: Value = from_str("!Cfg\nname: alpha\n").expect("parse tagged");
    let r = v.query("..name");
    assert!(!r.is_empty());
}

// ============================================================================
// Value::Display — Sequence walk (L3662, L3665, L3667), Mapping walk
// (L3672, L3675, L3677), Tagged path
// ============================================================================

#[test]
fn value_display_sequence_multi() {
    let v: Value = from_str("- 1\n- 2\n- 3\n").expect("parse seq");
    let s = format!("{v}");
    assert!(s.starts_with('['));
    assert!(s.ends_with(']'));
    assert_eq!(s.matches(", ").count(), 2);
}

#[test]
fn value_display_mapping_multi() {
    let v: Value = from_str("a: 1\nb: 2\nc: 3\n").expect("parse map");
    let s = format!("{v}");
    assert!(s.starts_with('{'));
    assert!(s.ends_with('}'));
    assert_eq!(s.matches(", ").count(), 2);
}

// ============================================================================
// ValueIndex — usize/&str index_or_insert panic-free happy paths (L3794,
// L3878, L3888, L3900) and &Value index (L3878 etc.)
// ============================================================================

#[test]
fn value_index_or_insert_into_sequence_via_tagged() {
    // Tagged(Sequence) → recursing into the inner sequence (L3794).
    let inner = Value::Sequence(vec![Value::from(10), Value::from(20)]);
    let mut v = Value::Tagged(Box::new(TaggedValue::new(Tag::new("!Wrap"), inner)));
    let elem = &mut v[0];
    assert_eq!(elem.as_i64(), Some(10));
}

#[test]
fn value_index_into_via_value_string_key() {
    // L3878-L3879: &Value as index
    let v: Value = from_str("a: 1\n").expect("parse");
    let key = Value::from("a");
    assert_eq!(v.get(&key).and_then(Value::as_i64), Some(1));
}

#[test]
fn value_index_into_via_value_int_key_for_seq() {
    // L3878 (mut), L3888 path
    let v: Value = from_str("- 10\n- 20\n").expect("parse");
    let key = Value::from(1_i64);
    assert_eq!(v.get(&key).and_then(Value::as_i64), Some(20));
}

#[test]
fn value_index_into_mut_via_value_int_key() {
    let mut v: Value = from_str("- 10\n- 20\n").expect("parse");
    let key = Value::from(0_i64);
    if let Some(elem) = v.get_mut(&key) {
        *elem = Value::from(99);
    }
    assert_eq!(v[0].as_i64(), Some(99));
}

#[test]
fn value_index_into_via_value_negative_int_returns_none() {
    let v: Value = from_str("- 1\n").expect("parse");
    let key = Value::from(-1_i64);
    assert!(v.get(&key).is_none());
}

// ============================================================================
// Value::Deserialize — visit_seq / visit_map with the fast-path through
// from_value (L4010-L4011, L4027-L4029, L4051-L4053)
// ============================================================================

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Wrapper {
    data: Value,
}

#[test]
fn value_deserialize_sequence_via_serde_json() {
    // Serde JSON drives the Value visitor through visit_seq /
    // visit_map without any tag-preserving sentinels.
    let json = r#"{"data": [1, 2, 3]}"#;
    let w: Wrapper = serde_json::from_str(json).expect("from_json");
    assert!(w.data.is_sequence());
    assert_eq!(w.data.as_sequence().unwrap().len(), 3);
}

#[test]
fn value_deserialize_string_via_serde_json() {
    let json = r#"{"data": "hello"}"#;
    let w: Wrapper = serde_json::from_str(json).expect("from_json string");
    assert_eq!(w.data.as_str(), Some("hello"));
}

#[test]
fn value_deserialize_unit_via_serde_json_null() {
    let json = r#"{"data": null}"#;
    let w: Wrapper = serde_json::from_str(json).expect("from_json null");
    assert!(w.data.is_null());
}

#[test]
fn value_deserialize_float_via_serde_json() {
    let json = r#"{"data": 3.14}"#;
    let w: Wrapper = serde_json::from_str(json).expect("from_json float");
    assert!(w.data.as_f64().unwrap() > 3.0);
}

#[test]
fn value_deserialize_bool_via_serde_json() {
    let json = r#"{"data": true}"#;
    let w: Wrapper = serde_json::from_str(json).expect("from_json bool");
    assert_eq!(w.data.as_bool(), Some(true));
}

#[test]
fn value_deserialize_nested_map_via_serde_json() {
    let json = r#"{"data": {"k": 1, "k2": 2}}"#;
    let w: Wrapper = serde_json::from_str(json).expect("from_json map");
    assert!(w.data.is_mapping());
    assert_eq!(w.data.as_mapping().unwrap().len(), 2);
}

// ============================================================================
// Value visit_map — magic-key path engages tag-preserving reconstruction
// (L4104-L4106, L4109-L4111, L4115-L4118)
// ============================================================================

#[test]
fn value_visit_map_tag_preserving_path() {
    // YAML with a global tag → the from_str::<Value> path goes
    // through TagPreservingMapAccess and the magic-key visit_map
    // branch reconstructs Value::Tagged.
    let yaml = "!Custom\nname: alpha\nport: 8080\n";
    let v: Value = from_str(yaml).expect("from_str tagged map");
    assert!(v.is_tagged());
    let inner = v.untag();
    assert!(inner.is_mapping());
}

#[test]
fn value_visit_map_tag_preserving_with_complex_inner() {
    // Tagged scalar that survives the round-trip.
    let yaml = "!T 42\n";
    let v: Value = from_str(yaml).expect("from_str tagged scalar");
    assert!(v.is_tagged());
}

// ============================================================================
// &Value Deserializer — deserialize_struct with SPANNED_TYPE_NAME and
// deserialize_enum string variant (L4282)
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
enum E {
    A,
    B,
    Tup(i32, i32),
}

#[test]
fn value_ref_deserialize_enum_string_variant() {
    let v = Value::from("A");
    let e: E = from_value(&v).expect("string enum variant");
    assert_eq!(e, E::A);
}

#[test]
fn value_ref_deserialize_enum_via_mapping() {
    // Single-entry mapping form `{Tup: [1, 2]}` for tuple variant.
    let mut m = Mapping::new();
    let _ = m.insert("Tup", Value::Sequence(vec![Value::from(1), Value::from(2)]));
    let v = Value::Mapping(m);
    let e: E = from_value(&v).expect("tuple enum");
    assert_eq!(e, E::Tup(1, 2));
}

// ============================================================================
// Value::interpolate_properties — Tagged + Sequence walk (L3392)
// ============================================================================

#[test]
fn interpolate_properties_walks_tagged() {
    let mut v: Value = from_str("!Cfg\n  name: ${who}\n").expect("parse");
    let mut props: HashMap<String, String> = HashMap::new();
    let _ = props.insert("who".into(), "world".into());
    v.interpolate_properties(&props).expect("interpolate");
    let inner = v.untag_ref();
    let m = inner.as_mapping().unwrap();
    assert_eq!(m.get("name").unwrap().as_str(), Some("world"));
}

#[test]
fn interpolate_properties_walks_sequence() {
    let mut v: Value = from_str("- ${a}\n- ${b}\n").expect("parse seq");
    let mut props: HashMap<String, String> = HashMap::new();
    let _ = props.insert("a".into(), "alpha".into());
    let _ = props.insert("b".into(), "beta".into());
    v.interpolate_properties(&props).expect("interpolate seq");
    let s = v.as_sequence().unwrap();
    assert_eq!(s[0].as_str(), Some("alpha"));
    assert_eq!(s[1].as_str(), Some("beta"));
}

// ============================================================================
// Mapping::cmp / MappingAny::cmp — value-mismatch arm (L780, L1262)
// ============================================================================

#[test]
fn mapping_cmp_value_difference() {
    let mut m1 = Mapping::new();
    let _ = m1.insert("k", Value::from(1));
    let mut m2 = Mapping::new();
    let _ = m2.insert("k", Value::from(2));
    // Same key, different values — value `cmp` arm fires.
    assert!(m1 < m2);
}

#[test]
fn mapping_any_cmp_value_difference() {
    let mut m1 = MappingAny::new();
    let _ = m1.insert(Value::from("k"), Value::from(1));
    let mut m2 = MappingAny::new();
    let _ = m2.insert(Value::from("k"), Value::from(2));
    assert!(m1 < m2);
}
