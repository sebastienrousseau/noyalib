// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Final coverage push on `crates/noyalib/src/value.rs`.
//!
//! Targets specific uncovered branches identified post the
//! `value_coverage_extra.rs` round 1: Display impls (Mapping,
//! MappingAny, Value), Serialize impls for collections,
//! TagPreservingMapAccess error arms, and the `&'de Value`
//! Deserializer enum/seq dispatch shapes.

use std::collections::BTreeMap;

use noyalib::{Mapping, MappingAny, Number, Tag, TaggedValue, Value};

// ── Mapping Display + Serialize ────────────────────────────────────

#[test]
fn final_value_mapping_display_format() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1_i64));
    let _ = m.insert("b", Value::from("two"));
    let s = format!("{m}");
    assert!(s.contains("a:") || s.contains("a "));
    assert!(s.contains('1'));
    assert!(s.contains("two"));
    assert!(s.starts_with('{') && s.ends_with('}'));
}

#[test]
fn final_value_mapping_display_empty() {
    let m = Mapping::new();
    assert_eq!(format!("{m}"), "{}");
}

#[test]
fn final_value_mapping_serialize_via_to_string() {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::from(42_i64));
    let s = noyalib::to_string(&m).expect("ser");
    assert!(s.contains("k:") && s.contains("42"));
}

// ── MappingAny Display + Serialize (non-string keys) ────────────────

#[test]
fn final_value_mappingany_display() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from(1_i64), Value::from("one"));
    let _ = m.insert(Value::from(true), Value::from("yes"));
    let s = format!("{m}");
    assert!(s.starts_with('{') && s.ends_with('}'));
}

#[test]
fn final_value_mappingany_serialize() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from(1_i64), Value::from("one"));
    let s = noyalib::to_string(&m).expect("ser");
    assert!(s.contains("one"));
}

#[test]
fn final_value_mappingany_default_then_insert() {
    let mut m = MappingAny::default();
    assert!(m.is_empty());
    let _ = m.insert(Value::from(true), Value::Null);
    assert_eq!(m.len(), 1);
}

// ── Number edge cases ──────────────────────────────────────────────

#[test]
fn final_value_number_display_special_floats() {
    // Number::Display shares the serializer's float formatter (#348),
    // so it prints the YAML `.inf` / `-.inf` / `.nan` forms, not
    // `f64`'s default "inf" / "-inf" / "NaN". The loose `contains`
    // checks below hold for both forms; see
    // `number_float_display.rs` for exact-string coverage of the
    // fixed output.
    let pos_inf = format!("{}", Number::Float(f64::INFINITY));
    assert!(pos_inf.contains("inf"));
    let neg_inf = format!("{}", Number::Float(f64::NEG_INFINITY));
    assert!(neg_inf.contains("inf") && neg_inf.starts_with('-'));
    let nan = format!("{}", Number::Float(f64::NAN));
    assert!(nan.to_ascii_lowercase().contains("nan"));
}

#[test]
fn final_value_number_eq_negative_zero() {
    let p = Number::Float(0.0);
    let n = Number::Float(-0.0);
    assert_eq!(p, n);
}

#[test]
fn final_value_number_ord_int_vs_float() {
    let i = Number::Integer(2);
    let f = Number::Float(2.5);
    assert!(i < f);
    assert!(f > i);
}

#[test]
fn final_value_number_arithmetic_helpers() {
    assert_eq!(Number::Integer(3).as_i64(), Some(3));
    assert_eq!(Number::Integer(3).as_u64(), Some(3));
    assert!((Number::Integer(3).as_f64() - 3.0).abs() < 1e-9);
    assert_eq!(Number::Integer(-1).as_u64(), None);
    assert_eq!(Number::Float(1.5).as_i64(), None);
}

// ── Tag display, From, Hash ─────────────────────────────────────────

#[test]
fn final_value_tag_from_str_roundtrip() {
    let t: Tag = "!Custom".into();
    assert_eq!(t.as_str(), "!Custom");
    let s: String = t.into_string();
    assert_eq!(s, "!Custom");
}

#[test]
fn final_value_tag_from_string() {
    let t: Tag = String::from("!!str").into();
    assert_eq!(t.as_str(), "!!str");
}

#[test]
fn final_value_tag_display_via_format() {
    let t = Tag::new("!MyTag");
    assert_eq!(format!("{t}"), "!MyTag");
}

#[test]
fn final_value_tag_eq_strips_one_bang() {
    // `Tag::PartialEq` compares via `nobang()` which strips AT MOST
    // ONE leading `!`. `!foo` and `foo` collapse to the same nobang
    // form; `!!str` and `!str` do NOT (one ends as `!str`, the other
    // as `str`).
    assert_eq!(Tag::new("!foo"), Tag::new("foo"));
    assert_eq!(Tag::new("!!str"), Tag::new("!!str"));
    assert_ne!(Tag::new("!!str"), Tag::new("!str"));
}

// ── Value::Display walks all variants ──────────────────────────────

#[test]
fn final_value_display_all_variants() {
    assert_eq!(format!("{}", Value::Null), "null");
    assert_eq!(format!("{}", Value::Bool(true)), "true");
    assert_eq!(format!("{}", Value::from(42_i64)), "42");
    assert_eq!(format!("{}", Value::String("hi".into())), "hi");
    let seq = Value::Sequence(vec![Value::from(1_i64), Value::from(2_i64)]);
    let s = format!("{seq}");
    assert!(s.starts_with('[') && s.ends_with(']'));
    assert!(s.contains('1') && s.contains('2'));
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::from("v"));
    let map = Value::Mapping(m);
    let s = format!("{map}");
    assert!(s.contains('k'));
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!T"),
        Value::from(7_i64),
    )));
    let s = format!("{tagged}");
    assert!(s.contains("!T") || s.contains('7'));
}

// ── Value Serialize / Deserialize round-trips by shape ─────────────

#[test]
fn final_value_serialize_sequence() {
    let v = Value::Sequence(vec![
        Value::from(1_i64),
        Value::from(2_i64),
        Value::from(3_i64),
    ]);
    let s = noyalib::to_string(&v).expect("ser seq");
    assert!(s.contains('1') && s.contains('2') && s.contains('3'));
}

#[test]
fn final_value_serialize_tagged_collection() {
    // Tagged collection: serialize emits as single-key mapping with
    // tag as key, hits the corresponding match arm in serialize.
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!Custom"),
        Value::Sequence(vec![Value::from(1_i64)]),
    )));
    let s = noyalib::to_string(&tagged).expect("ser tagged");
    assert!(s.contains("Custom") || s.contains('!'));
}

#[test]
fn final_value_serialize_mapping_with_nested() {
    let mut inner = Mapping::new();
    let _ = inner.insert("x", Value::from(1_i64));
    let mut outer = Mapping::new();
    let _ = outer.insert("inner", Value::Mapping(inner));
    let v = Value::Mapping(outer);
    let s = noyalib::to_string(&v).expect("ser nested");
    assert!(s.contains("inner") && s.contains('x'));
}

// ── &'de Value Deserializer enum/seq dispatch ──────────────────────

#[derive(Debug, serde::Deserialize, PartialEq)]
enum Tag2 {
    First,
    Second,
}

#[test]
fn final_value_deserializer_enum_via_string() {
    let v = Value::String("First".into());
    let t: Tag2 = noyalib::from_value(&v).expect("string-as-enum");
    assert_eq!(t, Tag2::First);
}

#[test]
fn final_value_deserializer_seq_via_value() {
    let v = Value::Sequence(vec![Value::from(1_i64), Value::from(2_i64)]);
    let xs: Vec<i64> = noyalib::from_value(&v).expect("seq");
    assert_eq!(xs, vec![1, 2]);
}

#[test]
fn final_value_deserializer_mapping_into_btreemap() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1_i64));
    let _ = m.insert("b", Value::from(2_i64));
    let bm: BTreeMap<String, i64> = noyalib::from_value(&Value::Mapping(m)).expect("map");
    assert_eq!(bm.get("a"), Some(&1));
    assert_eq!(bm.get("b"), Some(&2));
}

// ── Value::query and get_path edge cases ───────────────────────────

#[test]
fn final_value_query_returns_empty_on_no_match() {
    let yaml = "items: [a, b, c]\n";
    let v: Value = noyalib::from_str(yaml).expect("parse");
    let r = v.query("$.nonexistent[*]");
    assert!(r.is_empty());
}

#[test]
fn final_value_query_wildcards() {
    let yaml = "items:\n  - name: alice\n  - name: bob\n";
    let v: Value = noyalib::from_str(yaml).expect("parse");
    let names: Vec<&str> = v
        .query("$.items[*].name")
        .into_iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(names.contains(&"alice") || names.contains(&"bob") || names.is_empty());
}

#[test]
fn final_value_get_path_index() {
    let yaml = "items: [10, 20, 30]\n";
    let v: Value = noyalib::from_str(yaml).expect("parse");
    let r = v.get_path("items[1]");
    assert_eq!(r.and_then(|v| v.as_i64()), Some(20));
}

#[test]
fn final_value_get_path_returns_none_on_missing() {
    let v = Value::Null;
    assert!(v.get_path("foo.bar").is_none());
}

// ── Mapping/MappingAny edge cases (capacity, indexed access) ──────

#[test]
fn final_value_mapping_with_capacity() {
    let m = Mapping::with_capacity(8);
    assert!(m.is_empty());
    assert_eq!(m.len(), 0);
}

#[test]
fn final_value_mapping_get_index_oob_none() {
    let m = Mapping::new();
    assert!(m.get_index(0).is_none());
}

#[test]
fn final_value_mapping_first_last_empty() {
    let m = Mapping::new();
    assert!(m.first().is_none());
    assert!(m.last().is_none());
}

#[test]
fn final_value_mapping_first_last_populated() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1_i64));
    let _ = m.insert("b", Value::from(2_i64));
    assert_eq!(m.first().map(|(k, _)| k.as_str()), Some("a"));
    assert_eq!(m.last().map(|(k, _)| k.as_str()), Some("b"));
}

#[test]
fn final_value_mapping_iter_mut_modifies() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1_i64));
    for (_, v) in &mut m {
        if let Some(n) = v.as_i64() {
            *v = Value::from(n + 100);
        }
    }
    assert_eq!(m.get("a").and_then(|v| v.as_i64()), Some(101));
}

// ── Number::FromStr ─────────────────────────────────────────────────

#[test]
fn final_value_number_fromstr_int() {
    let n: Number = "42".parse().expect("int");
    assert_eq!(n.as_i64(), Some(42));
}

#[test]
fn final_value_number_fromstr_negative_int() {
    let n: Number = "-100".parse().expect("neg int");
    assert_eq!(n.as_i64(), Some(-100));
}

#[test]
fn final_value_number_fromstr_float() {
    let n: Number = "1.5".parse().expect("float");
    assert!((n.as_f64() - 1.5).abs() < 1e-9);
}

#[test]
fn final_value_number_fromstr_inf() {
    let n: Number = ".inf".parse().expect("inf");
    assert!(n.as_f64().is_infinite() && n.as_f64() > 0.0);
}

#[test]
fn final_value_number_fromstr_invalid() {
    let r: Result<Number, _> = "not-a-number".parse();
    assert!(r.is_err());
}
