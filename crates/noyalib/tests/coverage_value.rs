//! Value module coverage tests — Mapping utilities, consuming conversions,
//! Number, MappingAny, Tag, etc.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::str::FromStr;

use noyalib::{
    Mapping, MappingAny, MaybeTag, Number, Tag, TaggedValue, Value, check_for_tag, from_str, nobang,
};
#[cfg(feature = "lossless-u64")]
use serde::Deserialize;

// ============================================================================
// Mapping — capacity, reserve, shrink
// ============================================================================

#[test]
fn mapping_with_capacity_and_reserve() {
    let mut m = Mapping::with_capacity(10);
    assert!(m.capacity() >= 10);
    m.reserve(20);
    assert!(m.capacity() >= 20);
    m.shrink_to_fit();
}

#[cfg(feature = "lossless-u64")]
#[test]
fn value_deserialize_visit_u64_preserves_unsigned() {
    let producer = serde_json::Value::from(u64::MAX);
    let value = Value::deserialize(producer).unwrap();
    assert_eq!(value.as_u64(), Some(u64::MAX));
    assert!(matches!(value, Value::Number(Number::Unsigned(n)) if n == u64::MAX));
}

#[test]
fn mapping_clear() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));
    assert_eq!(m.len(), 2);
    m.clear();
    assert!(m.is_empty());
}

// ============================================================================
// Mapping — get_index, get_index_mut
// ============================================================================

#[test]
fn mapping_get_index() {
    let mut m = Mapping::new();
    let _ = m.insert("first", Value::from(1));
    let _ = m.insert("second", Value::from(2));

    let (k, v) = m.get_index(0).unwrap();
    assert_eq!(k, "first");
    assert_eq!(v.as_i64(), Some(1));

    let (k, v) = m.get_index(1).unwrap();
    assert_eq!(k, "second");
    assert_eq!(v.as_i64(), Some(2));

    assert!(m.get_index(5).is_none());
}

#[test]
fn mapping_get_index_mut() {
    let mut m = Mapping::new();
    let _ = m.insert("x", Value::from(10));

    let (_, v) = m.get_index_mut(0).unwrap();
    *v = Value::from(20);
    assert_eq!(m.get("x").unwrap().as_i64(), Some(20));

    assert!(m.get_index_mut(5).is_none());
}

// ============================================================================
// Mapping — remove, remove_entry, swap_remove, shift_remove
// ============================================================================

#[test]
fn mapping_remove_and_remove_entry() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));
    let _ = m.insert("c", Value::from(3));

    let v = m.remove("b").unwrap();
    assert_eq!(v.as_i64(), Some(2));
    assert_eq!(m.len(), 2);

    let (k, v) = m.remove_entry("a").unwrap();
    assert_eq!(k, "a");
    assert_eq!(v.as_i64(), Some(1));

    assert!(m.remove("nonexistent").is_none());
    assert!(m.remove_entry("nonexistent").is_none());
}

#[test]
fn mapping_swap_remove() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));

    let v = m.swap_remove("a").unwrap();
    assert_eq!(v.as_i64(), Some(1));
    assert!(m.swap_remove("nonexistent").is_none());
}

#[test]
fn mapping_shift_remove() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));

    let v = m.shift_remove("a").unwrap();
    assert_eq!(v.as_i64(), Some(1));
    assert!(m.shift_remove("nonexistent").is_none());
}

// ============================================================================
// Mapping — entry, retain, sort_keys, reverse, pop
// ============================================================================

#[test]
fn mapping_entry() {
    let mut m = Mapping::new();
    let _ = m.entry("key").or_insert(Value::from(42));
    assert_eq!(m.get("key").unwrap().as_i64(), Some(42));

    // Entry already exists, don't overwrite
    let _ = m.entry("key").or_insert(Value::from(99));
    assert_eq!(m.get("key").unwrap().as_i64(), Some(42));
}

#[test]
fn mapping_retain() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));
    let _ = m.insert("c", Value::from(3));

    m.retain(|k, _| k != "b");
    assert_eq!(m.len(), 2);
    assert!(!m.contains_key("b"));
}

#[test]
fn mapping_sort_keys() {
    let mut m = Mapping::new();
    let _ = m.insert("c", Value::from(3));
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));

    m.sort_keys();
    let keys: Vec<&String> = m.keys().collect();
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[test]
fn mapping_reverse() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));
    let _ = m.insert("c", Value::from(3));

    m.reverse();
    let keys: Vec<&String> = m.keys().collect();
    assert_eq!(keys, vec!["c", "b", "a"]);
}

#[test]
fn mapping_pop_first_and_pop_last() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));
    let _ = m.insert("c", Value::from(3));

    let (k, v) = m.pop_first().unwrap();
    assert_eq!(k, "a");
    assert_eq!(v.as_i64(), Some(1));

    let (k, v) = m.pop_last().unwrap();
    assert_eq!(k, "c");
    assert_eq!(v.as_i64(), Some(3));

    assert_eq!(m.len(), 1);

    // Empty mapping
    let mut empty = Mapping::new();
    assert!(empty.pop_first().is_none());
    assert!(empty.pop_last().is_none());
}

// ============================================================================
// Mapping — first, last, first_mut, last_mut
// ============================================================================

#[test]
fn mapping_first_last() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("z", Value::from(26));

    let (k, v) = m.first().unwrap();
    assert_eq!(k, "a");
    assert_eq!(v.as_i64(), Some(1));

    let (k, v) = m.last().unwrap();
    assert_eq!(k, "z");
    assert_eq!(v.as_i64(), Some(26));
}

#[test]
fn mapping_first_last_mut() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("z", Value::from(26));

    let (_, v) = m.first_mut().unwrap();
    *v = Value::from(100);
    assert_eq!(m.get("a").unwrap().as_i64(), Some(100));

    let (_, v) = m.last_mut().unwrap();
    *v = Value::from(200);
    assert_eq!(m.get("z").unwrap().as_i64(), Some(200));
}

// ============================================================================
// Mapping — iterators, extend, conversions
// ============================================================================

#[test]
fn mapping_iterators() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));

    assert_eq!(m.iter().count(), 2);
    assert_eq!(m.keys().count(), 2);
    assert_eq!(m.values().count(), 2);

    for (_, v) in m.iter_mut() {
        *v = Value::from(0);
    }
    assert_eq!(m.get("a").unwrap().as_i64(), Some(0));
}

#[test]
fn mapping_values_mut() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    for v in m.values_mut() {
        *v = Value::from(99);
    }
    assert_eq!(m.get("a").unwrap().as_i64(), Some(99));
}

#[test]
fn mapping_extend() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));

    let extra = vec![
        ("b".to_string(), Value::from(2)),
        ("c".to_string(), Value::from(3)),
    ];
    m.extend(extra);
    assert_eq!(m.len(), 3);
}

#[test]
fn mapping_into_inner_and_from_inner() {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::from(1));

    let inner = m.into_inner();
    assert_eq!(inner.len(), 1);

    let m2 = Mapping::from_inner(inner);
    assert_eq!(m2.len(), 1);
}

#[test]
fn mapping_from_vec() {
    let v = vec![
        ("a".to_string(), Value::from(1)),
        ("b".to_string(), Value::from(2)),
    ];
    let m = Mapping::from(v);
    assert_eq!(m.len(), 2);
}

#[test]
fn mapping_from_array() {
    let m = Mapping::from([
        ("x".to_string(), Value::from(10)),
        ("y".to_string(), Value::from(20)),
    ]);
    assert_eq!(m.len(), 2);
}

#[test]
fn mapping_from_iter() {
    let m: Mapping = vec![
        ("a".to_string(), Value::from(1)),
        ("b".to_string(), Value::from(2)),
    ]
    .into_iter()
    .collect();
    assert_eq!(m.len(), 2);
}

#[test]
fn mapping_into_indexmap() {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::from(1));
    let im: indexmap::IndexMap<String, Value> = m.into();
    assert_eq!(im.len(), 1);
}

#[test]
fn mapping_from_indexmap() {
    let mut im = indexmap::IndexMap::new();
    let _ = im.insert("k".to_string(), Value::from(1));
    let m = Mapping::from(im);
    assert_eq!(m.len(), 1);
}

#[test]
fn mapping_into_iterator() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));

    // owned iterator
    let items: Vec<_> = m.into_iter().collect();
    assert_eq!(items.len(), 2);
}

#[test]
fn mapping_ref_into_iterator() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));

    let items: Vec<_> = (&m).into_iter().collect();
    assert_eq!(items.len(), 1);
}

#[test]
fn mapping_mut_into_iterator() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));

    for (_, v) in &mut m {
        *v = Value::from(0);
    }
    assert_eq!(m.get("a").unwrap().as_i64(), Some(0));
}

// ============================================================================
// Mapping — Index, IndexMut, Display, Ord, Hash
// ============================================================================

#[test]
fn mapping_index() {
    let mut m = Mapping::new();
    let _ = m.insert("key", Value::from(42));

    assert_eq!(m["key"].as_i64(), Some(42));
}

#[test]
#[should_panic(expected = "key not found")]
fn mapping_index_missing_panics() {
    let m = Mapping::new();
    let _ = &m["missing"];
}

#[test]
fn mapping_index_mut() {
    let mut m = Mapping::new();
    let _ = m.insert("key", Value::from(42));

    m["key"] = Value::from(99);
    assert_eq!(m["key"].as_i64(), Some(99));
}

#[test]
fn mapping_display() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let _ = m.insert("b", Value::from(2));
    let s = format!("{m}");
    assert!(s.contains("a: 1"));
    assert!(s.contains("b: 2"));
}

#[test]
fn mapping_ord() {
    let mut m1 = Mapping::new();
    let _ = m1.insert("a", Value::from(1));
    let mut m2 = Mapping::new();
    let _ = m2.insert("b", Value::from(1));

    // They have the same length, so comparison is by key
    assert!(m1 < m2); // "a" < "b"
}

#[test]
fn mapping_hash() {
    use std::collections::HashSet;
    let mut m1 = Mapping::new();
    let _ = m1.insert("a", Value::from(1));
    let m2 = m1.clone();

    let mut set = HashSet::new();
    let _ = set.insert(m1);
    let _ = set.insert(m2);
    assert_eq!(set.len(), 1);
}

// ============================================================================
// MappingAny — basic operations
// ============================================================================

#[test]
fn mapping_any_basic() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from(1), Value::from("one"));
    let _ = m.insert(Value::Bool(true), Value::from("yes"));

    assert_eq!(m.len(), 2);
    assert!(!m.is_empty());
    assert!(m.contains_key(&Value::from(1)));
    assert_eq!(m.get(&Value::from(1)).unwrap().as_str(), Some("one"));
}

#[test]
fn mapping_any_get_mut() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("k"), Value::from(1));
    *m.get_mut(&Value::from("k")).unwrap() = Value::from(2);
    assert_eq!(m.get(&Value::from("k")).unwrap().as_i64(), Some(2));
}

#[test]
fn mapping_any_get_index() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let (k, v) = m.get_index(0).unwrap();
    assert_eq!(k.as_str(), Some("a"));
    assert_eq!(v.as_i64(), Some(1));
    assert!(m.get_index(5).is_none());
}

#[test]
fn mapping_any_get_index_mut() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let (_, v) = m.get_index_mut(0).unwrap();
    *v = Value::from(99);
    assert_eq!(m.get(&Value::from("a")).unwrap().as_i64(), Some(99));
    assert!(m.get_index_mut(5).is_none());
}

#[test]
fn mapping_any_remove_and_swap() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let _ = m.insert(Value::from("b"), Value::from(2));

    let v = m.remove(&Value::from("a")).unwrap();
    assert_eq!(v.as_i64(), Some(1));

    let _ = m.insert(Value::from("c"), Value::from(3));
    let v = m.swap_remove(&Value::from("b")).unwrap();
    assert_eq!(v.as_i64(), Some(2));

    let v = m.shift_remove(&Value::from("c")).unwrap();
    assert_eq!(v.as_i64(), Some(3));
}

#[test]
fn mapping_any_remove_entry() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("k"), Value::from(42));
    let (k, v) = m.remove_entry(&Value::from("k")).unwrap();
    assert_eq!(k.as_str(), Some("k"));
    assert_eq!(v.as_i64(), Some(42));
}

#[test]
fn mapping_any_capacity_and_shrink() {
    let mut m = MappingAny::with_capacity(10);
    assert!(m.capacity() >= 10);
    m.reserve(20);
    m.shrink_to_fit();
}

#[test]
fn mapping_any_clear() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    m.clear();
    assert!(m.is_empty());
}

#[test]
fn mapping_any_entry_retain_sort_reverse() {
    let mut m = MappingAny::new();
    let _ = m.entry(Value::from("a")).or_insert(Value::from(1));
    let _ = m.insert(Value::from("c"), Value::from(3));
    let _ = m.insert(Value::from("b"), Value::from(2));

    m.retain(|_, v| v.as_i64() != Some(2));
    assert_eq!(m.len(), 2);

    m.sort_keys();
    m.reverse();
}

#[test]
fn mapping_any_first_last() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let _ = m.insert(Value::from("z"), Value::from(26));

    assert!(m.first().is_some());
    assert!(m.last().is_some());
    assert!(m.first_mut().is_some());
    assert!(m.last_mut().is_some());
}

#[test]
fn mapping_any_pop_first_and_pop_last() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let _ = m.insert(Value::from("b"), Value::from(2));

    let (k, _) = m.pop_first().unwrap();
    assert_eq!(k.as_str(), Some("a"));

    let (k, _) = m.pop_last().unwrap();
    assert_eq!(k.as_str(), Some("b"));

    assert!(m.pop_first().is_none());
    assert!(m.pop_last().is_none());
}

#[test]
fn mapping_any_iterators() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    assert_eq!(m.iter().count(), 1);
    assert_eq!(m.keys().count(), 1);
    assert_eq!(m.values().count(), 1);
    for (_, v) in m.iter_mut() {
        *v = Value::from(0);
    }
    for v in m.values_mut() {
        *v = Value::from(1);
    }
}

#[test]
fn mapping_any_extend() {
    let mut m = MappingAny::new();
    m.extend(vec![(Value::from("a"), Value::from(1))]);
    assert_eq!(m.len(), 1);
}

#[test]
fn mapping_any_into_inner_and_from_inner() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("k"), Value::from(1));
    let inner = m.into_inner();
    let m2 = MappingAny::from_inner(inner);
    assert_eq!(m2.len(), 1);
}

#[test]
fn mapping_any_into_mapping() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let _ = m.insert(Value::from("b"), Value::from(2));
    let mapping = m.into_mapping().unwrap();
    assert_eq!(mapping.len(), 2);
}

#[test]
fn mapping_any_into_mapping_fails_non_string_key() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from(42), Value::from(1));
    assert!(m.into_mapping().is_none());
}

#[test]
fn mapping_any_from_mapping() {
    let mut m = Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let any = MappingAny::from(m);
    assert_eq!(any.len(), 1);
}

#[test]
fn mapping_any_from_array() {
    let m = MappingAny::from([(Value::from("a"), Value::from(1))]);
    assert_eq!(m.len(), 1);
}

#[test]
fn mapping_any_from_indexmap() {
    let mut im = indexmap::IndexMap::new();
    let _ = im.insert(Value::from("a"), Value::from(1));
    let m = MappingAny::from(im);
    assert_eq!(m.len(), 1);
}

#[test]
fn mapping_any_into_indexmap() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let im: indexmap::IndexMap<Value, Value> = m.into();
    assert_eq!(im.len(), 1);
}

#[test]
fn mapping_any_from_iter() {
    let m: MappingAny = vec![(Value::from("a"), Value::from(1))]
        .into_iter()
        .collect();
    assert_eq!(m.len(), 1);
}

#[test]
fn mapping_any_into_iterator() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let items: Vec<_> = m.into_iter().collect();
    assert_eq!(items.len(), 1);
}

#[test]
fn mapping_any_ref_into_iter() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let items: Vec<_> = (&m).into_iter().collect();
    assert_eq!(items.len(), 1);
}

#[test]
fn mapping_any_mut_into_iter() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    for (_, v) in &mut m {
        *v = Value::from(0);
    }
}

#[test]
fn mapping_any_index() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("k"), Value::from(42));
    assert_eq!(m[&Value::from("k")].as_i64(), Some(42));
}

#[test]
#[should_panic(expected = "key not found")]
fn mapping_any_index_missing_panics() {
    let m = MappingAny::new();
    let _ = &m[&Value::from("missing")];
}

#[test]
fn mapping_any_index_mut() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("k"), Value::from(1));
    m[&Value::from("k")] = Value::from(99);
    assert_eq!(m[&Value::from("k")].as_i64(), Some(99));
}

#[test]
fn mapping_any_display() {
    let mut m = MappingAny::new();
    let _ = m.insert(Value::from("a"), Value::from(1));
    let s = format!("{m}");
    assert!(s.contains("a"));
}

#[test]
fn mapping_any_ord_and_hash() {
    use std::collections::HashSet;
    let m1 = MappingAny::new();
    let m2 = MappingAny::new();
    assert_eq!(m1, m2);
    assert!(m1 <= m2);

    let mut set = HashSet::new();
    let _ = set.insert(m1);
    let _ = set.insert(m2);
    assert_eq!(set.len(), 1);
}

// ============================================================================
// Number
// ============================================================================

#[test]
fn number_as_accessors() {
    let n = Number::Integer(42);
    assert_eq!(n.as_i64(), Some(42));
    assert_eq!(n.as_u64(), Some(42));
    assert!((n.as_f64() - 42.0).abs() < f64::EPSILON);
    assert!(n.is_integer());
    assert!(!n.is_float());
    assert!(n.is_i64());
    assert!(n.is_u64());
    assert!(n.is_f64());
    assert!(!n.is_nan());
    assert!(!n.is_infinite());
    assert!(n.is_finite());
}

#[test]
fn number_negative_not_u64() {
    let n = Number::Integer(-1);
    assert!(n.as_u64().is_none());
    assert!(!n.is_u64());
}

#[test]
fn number_float_accessors() {
    let n = Number::Float(2.75);
    assert!(n.as_i64().is_none());
    assert!(n.as_u64().is_none());
    assert!(!n.is_integer());
    assert!(n.is_float());
    assert!(!n.is_i64());
    assert!(!n.is_u64());
    assert!(n.is_f64());
    assert!(!n.is_nan());
    assert!(!n.is_infinite());
    assert!(n.is_finite());
}

#[test]
fn number_nan_predicates() {
    let n = Number::Float(f64::NAN);
    assert!(n.is_nan());
    assert!(!n.is_infinite());
    assert!(!n.is_finite());
}

#[test]
fn number_infinity_predicates() {
    let n = Number::Float(f64::INFINITY);
    assert!(!n.is_nan());
    assert!(n.is_infinite());
    assert!(!n.is_finite());
}

#[test]
fn number_display_integer() {
    assert_eq!(Number::Integer(42).to_string(), "42");
}

#[test]
fn number_display_float() {
    let s = Number::Float(2.75).to_string();
    assert!(s.contains("2.75"));
}

#[test]
fn number_display_negative_zero() {
    let s = Number::Float(-0.0).to_string();
    // -0.0 displays as "-0" or "0" depending on platform
    assert!(s.contains("0"));
}

#[test]
fn number_from_str_integer() {
    assert_eq!(Number::from_str("42").unwrap().as_i64(), Some(42));
    assert_eq!(Number::from_str("-17").unwrap().as_i64(), Some(-17));
}

#[test]
fn number_from_str_hex_octal_binary() {
    assert_eq!(Number::from_str("0x2A").unwrap().as_i64(), Some(42));
    assert_eq!(Number::from_str("0o52").unwrap().as_i64(), Some(42));
    assert_eq!(Number::from_str("0b101010").unwrap().as_i64(), Some(42));
    assert_eq!(Number::from_str("0X2A").unwrap().as_i64(), Some(42));
    assert_eq!(Number::from_str("0O52").unwrap().as_i64(), Some(42));
    assert_eq!(Number::from_str("0B101010").unwrap().as_i64(), Some(42));
}

#[test]
fn number_from_str_float() {
    let n = Number::from_str("2.75").unwrap();
    assert!((n.as_f64() - 2.75).abs() < 0.001);
}

#[test]
fn number_from_str_special_floats() {
    assert!(Number::from_str(".nan").unwrap().is_nan());
    assert!(Number::from_str(".NaN").unwrap().is_nan());
    assert!(Number::from_str(".NAN").unwrap().is_nan());
    assert!(Number::from_str(".inf").unwrap().as_f64().is_infinite());
    assert!(Number::from_str("-.inf").unwrap().as_f64().is_infinite());
    assert!(Number::from_str("+.inf").unwrap().as_f64().is_infinite());
    assert!(Number::from_str(".Inf").unwrap().as_f64().is_infinite());
    assert!(Number::from_str("-.Inf").unwrap().as_f64().is_infinite());
    assert!(Number::from_str("+.INF").unwrap().as_f64().is_infinite());
}

#[test]
fn number_from_str_invalid() {
    assert!(Number::from_str("not_a_number").is_err());
    let err = Number::from_str("xyz").unwrap_err();
    assert_eq!(err.to_string(), "invalid number");
}

#[test]
fn number_eq_nan() {
    let a = Number::Float(f64::NAN);
    let b = Number::Float(f64::NAN);
    assert_eq!(a, b); // NaN == NaN by design
}

#[test]
fn number_eq_cross_type() {
    let a = Number::Integer(42);
    let b = Number::Float(42.0);
    assert_ne!(a, b); // Different variants
}

#[test]
fn number_ord() {
    let a = Number::Integer(1);
    let b = Number::Integer(2);
    assert!(a < b);

    let c = Number::Float(1.0);
    let d = Number::Float(2.0);
    assert!(c < d);

    // Cross-type comparison
    let e = Number::Integer(1);
    let f = Number::Float(2.0);
    assert!(e < f);

    // NaN ordering
    let nan = Number::Float(f64::NAN);
    assert!(a < nan); // NaN is greater than non-NaN
}

#[test]
fn number_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let _ = set.insert(Number::Integer(42));
    let _ = set.insert(Number::Integer(42));
    assert_eq!(set.len(), 1);
}

#[test]
fn number_from_types() {
    let _ = Number::from(1i8);
    let _ = Number::from(1i16);
    let _ = Number::from(1i32);
    let _ = Number::from(1i64);
    let _ = Number::from(1isize);
    let _ = Number::from(1u8);
    let _ = Number::from(1u16);
    let _ = Number::from(1u32);
    let _ = Number::from(1u64);
    let _ = Number::from(1usize);
    let _ = Number::from(1.0f32);
    let _ = Number::from(1.0f64);
}

#[test]
fn number_u64_max_becomes_float() {
    let n = Number::from(u64::MAX);
    #[cfg(not(feature = "lossless-u64"))]
    assert!(n.is_float());
    #[cfg(feature = "lossless-u64")]
    assert_eq!(n.as_u64(), Some(u64::MAX));
}

// ============================================================================
// Tag
// ============================================================================

#[test]
fn tag_new_and_as_str() {
    let t = Tag::new("!custom");
    assert_eq!(t.as_str(), "!custom");
}

#[test]
fn tag_into_string() {
    let t = Tag::new("!tag");
    assert_eq!(t.into_string(), "!tag");
}

#[test]
fn tag_nobang() {
    let t = Tag::new("!foo");
    assert_eq!(t.nobang(), "foo");

    let t2 = Tag::new("bar");
    assert_eq!(t2.nobang(), "bar");
}

#[test]
fn tag_display() {
    let t = Tag::new("!custom");
    assert_eq!(format!("{t}"), "!custom");
}

#[test]
fn tag_bang_ignoring_equality() {
    assert_eq!(Tag::new("!foo"), Tag::new("foo"));
    assert_ne!(Tag::new("!!int"), Tag::new("!int")); // nobang("!!int")="!int" vs nobang("!int")="int"
    assert_ne!(Tag::new("foo"), Tag::new("bar"));
}

#[test]
fn tag_ord() {
    let a = Tag::new("!alpha");
    let b = Tag::new("!beta");
    assert!(a < b);
}

#[test]
fn tag_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let _ = set.insert(Tag::new("!foo"));
    let _ = set.insert(Tag::new("foo")); // Same after nobang
    assert_eq!(set.len(), 1);
}

#[test]
fn tag_from_str_and_string() {
    let t1 = Tag::from("!tag");
    let t2 = Tag::from("!tag".to_string());
    assert_eq!(t1, t2);
}

#[test]
fn tag_as_ref_str() {
    let t = Tag::new("!tag");
    let s: &str = t.as_ref();
    assert_eq!(s, "!tag");
}

#[test]
fn tag_try_from_bytes() {
    let t = Tag::try_from(b"!foo".as_slice()).unwrap();
    assert_eq!(t.as_str(), "!foo");

    assert!(Tag::try_from(&[0xFF, 0xFE][..]).is_err());
}

// ============================================================================
// nobang and check_for_tag
// ============================================================================

#[test]
fn nobang_function() {
    assert_eq!(nobang("!foo"), "foo");
    assert_eq!(nobang("foo"), "foo");
    assert_eq!(nobang("!!int"), "!int");
    assert_eq!(nobang(""), "");
}

#[test]
fn check_for_tag_function() {
    match check_for_tag(&"!mytag") {
        MaybeTag::Tag(s) => assert_eq!(s, "!mytag"),
        _ => panic!("expected Tag"),
    }

    match check_for_tag(&"plain") {
        MaybeTag::NotTag(s) => assert_eq!(s, "plain"),
        _ => panic!("expected NotTag"),
    }
}

// ============================================================================
// TaggedValue
// ============================================================================

#[test]
fn tagged_value_new_and_accessors() {
    let tv = TaggedValue::new(Tag::new("!t"), Value::from(42));
    assert_eq!(tv.tag().as_str(), "!t");
    assert_eq!(tv.value().as_i64(), Some(42));
}

#[test]
fn tagged_value_value_mut() {
    let mut tv = TaggedValue::new(Tag::new("!t"), Value::from(1));
    *tv.value_mut() = Value::from(2);
    assert_eq!(tv.value().as_i64(), Some(2));
}

#[test]
fn tagged_value_into_parts() {
    let tv = TaggedValue::new(Tag::new("!t"), Value::from("hello"));
    let (tag, val) = tv.into_parts();
    assert_eq!(tag.as_str(), "!t");
    assert_eq!(val.as_str(), Some("hello"));
}

#[test]
fn tagged_value_display() {
    let tv = TaggedValue::new(Tag::new("!custom"), Value::from(42));
    let s = format!("{tv}");
    assert!(s.contains("!custom"));
    assert!(s.contains("42"));
}

// ============================================================================
// Value — consuming conversions
// ============================================================================

#[test]
fn value_into_string() {
    let v: Value = from_str("hello").unwrap();
    match v {
        Value::String(s) => assert_eq!(s, "hello"),
        _ => panic!("expected string"),
    }
}

#[test]
fn value_into_sequence() {
    let v: Value = from_str("- 1\n- 2\n").unwrap();
    match v {
        Value::Sequence(s) => assert_eq!(s.len(), 2),
        _ => panic!("expected sequence"),
    }
}

#[test]
fn value_into_mapping() {
    let v: Value = from_str("a: 1\nb: 2\n").unwrap();
    match v {
        Value::Mapping(m) => assert_eq!(m.len(), 2),
        _ => panic!("expected mapping"),
    }
}

// ============================================================================
// Value — as_* accessors
// ============================================================================

#[test]
fn value_as_null() {
    let v = Value::Null;
    assert_eq!(v.as_null(), Some(()));
    assert!(v.as_bool().is_none());
}

#[test]
fn value_as_tagged() {
    let tv = TaggedValue::new(Tag::new("!t"), Value::from(1));
    let v = Value::Tagged(Box::new(tv));
    assert!(v.as_tagged().is_some());
    assert!(v.is_tagged());
}

#[test]
fn value_as_tagged_mut() {
    let tv = TaggedValue::new(Tag::new("!t"), Value::from(1));
    let mut v = Value::Tagged(Box::new(tv));
    let t = v.as_tagged_mut().unwrap();
    *t.value_mut() = Value::from(2);
    assert_eq!(v.as_tagged().unwrap().value().as_i64(), Some(2));
}

// ============================================================================
// Value — type predicates
// ============================================================================

#[test]
fn value_type_predicates() {
    assert!(Value::Null.is_null());
    assert!(Value::Bool(true).is_bool());
    assert!(Value::Number(Number::Integer(1)).is_number());
    assert!(Value::String("s".into()).is_string());
    assert!(Value::Sequence(vec![]).is_sequence());
    assert!(Value::Mapping(Mapping::new()).is_mapping());

    let tv = TaggedValue::new(Tag::new("!t"), Value::Null);
    assert!(Value::Tagged(Box::new(tv)).is_tagged());
}

#[test]
fn value_is_i64_u64_f64() {
    let v = Value::Number(Number::Integer(42));
    assert!(v.is_i64());
    assert!(v.is_u64());
    assert!(v.is_f64());

    let v = Value::Number(Number::Integer(-1));
    assert!(v.is_i64());
    assert!(!v.is_u64());

    let v = Value::Number(Number::Float(2.75));
    assert!(!v.is_i64());
    assert!(v.is_f64());

    assert!(!Value::Null.is_i64());
    assert!(!Value::Null.is_u64());
    assert!(!Value::Null.is_f64());
}

// ============================================================================
// Value — merge and merge_concat
// ============================================================================

#[test]
fn value_merge_mappings() {
    let mut base: Value = from_str("a: 1\nb: 2\n").unwrap();
    let other: Value = from_str("b: 3\nc: 4\n").unwrap();
    base.merge(other);
    assert_eq!(base.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(base.get("b").unwrap().as_i64(), Some(3)); // overwritten
    assert_eq!(base.get("c").unwrap().as_i64(), Some(4));
}

#[test]
fn value_merge_scalars() {
    let mut base = Value::from(1);
    let other = Value::from(2);
    base.merge(other);
    assert_eq!(base.as_i64(), Some(2));
}

#[test]
fn value_merge_concat_sequences() {
    let mut base: Value = from_str("items:\n  - a\n  - b\n").unwrap();
    let other: Value = from_str("items:\n  - c\n  - d\n").unwrap();
    base.merge_concat(other);
    let items = base.get("items").unwrap().as_sequence().unwrap();
    assert_eq!(items.len(), 4);
}

#[test]
fn value_merge_concat_scalars() {
    let mut base = Value::from("old");
    base.merge_concat(Value::from("new"));
    assert_eq!(base.as_str(), Some("new"));
}

// ============================================================================
// Value — remove and insert
// ============================================================================

#[test]
fn value_remove() {
    let mut v: Value = from_str("a: 1\nb: 2\n").unwrap();
    let removed = v.remove("a");
    assert_eq!(removed.unwrap().as_i64(), Some(1));
    assert!(v.get("a").is_none());

    // Remove from non-mapping
    let mut v = Value::from(42);
    assert!(v.remove("key").is_none());
}

#[test]
fn value_insert() {
    let mut v: Value = from_str("a: 1\n").unwrap();
    let prev = v.insert("b", Value::from(2));
    assert!(prev.is_none());
    assert_eq!(v.get("b").unwrap().as_i64(), Some(2));

    // Insert into non-mapping
    let mut v = Value::from(42);
    assert!(v.insert("key", Value::Null).is_none());
}

// ============================================================================
// ValueIndex for &Value
// ============================================================================

#[test]
fn value_index_by_value_string_key() {
    let v: Value = from_str("a: 1\nb: 2\n").unwrap();
    let key = Value::from("a");
    assert_eq!(v.get(&key).unwrap().as_i64(), Some(1));
}

#[test]
fn value_index_by_value_integer_key() {
    let v: Value = from_str("- 10\n- 20\n- 30\n").unwrap();
    let key = Value::Number(Number::Integer(1));
    assert_eq!(v.get(&key).unwrap().as_i64(), Some(20));
}

#[test]
fn value_index_by_value_negative_integer() {
    let v: Value = from_str("- 10\n- 20\n").unwrap();
    let key = Value::Number(Number::Integer(-1));
    assert!(v.get(&key).is_none());
}

#[test]
fn value_index_by_value_non_indexable() {
    let v: Value = from_str("a: 1\n").unwrap();
    let key = Value::Bool(true);
    assert!(v.get(&key).is_none());
}

// ============================================================================
// Value — Display, Ord, Hash, Default, FromStr
// ============================================================================

#[test]
fn value_display() {
    assert_eq!(Value::Null.to_string(), "null");
    assert_eq!(Value::Bool(true).to_string(), "true");
    assert_eq!(Value::from(42).to_string(), "42");
    assert_eq!(Value::from("hello").to_string(), "hello");
}

#[test]
fn value_default_is_null() {
    let v = Value::default();
    assert!(v.is_null());
}

#[test]
fn value_from_types() {
    let _ = Value::from(true);
    let _ = Value::from(42i64);
    let _ = Value::from(42u64);
    let _ = Value::from(2.75f64);
    let _ = Value::from("hello");
    let _ = Value::from("hello".to_string());
    let _ = Value::from(Number::Integer(1));
}

// ============================================================================
// Value — get_path
// ============================================================================

#[test]
fn value_get_path_nested() {
    let v: Value = from_str("a:\n  b:\n    c: 42\n").unwrap();
    assert_eq!(v.get_path("a.b.c").unwrap().as_i64(), Some(42));
}

#[test]
fn value_get_path_with_index() {
    let v: Value = from_str("items:\n  - name: first\n  - name: second\n").unwrap();
    assert_eq!(v.get_path("items[0].name").unwrap().as_str(), Some("first"));
    assert_eq!(
        v.get_path("items[1].name").unwrap().as_str(),
        Some("second")
    );
}

#[test]
fn value_get_path_missing() {
    let v: Value = from_str("a: 1\n").unwrap();
    assert!(v.get_path("b.c").is_none());
}

#[test]
fn value_get_path_mut() {
    let mut v: Value = from_str("a:\n  b: 1\n").unwrap();
    if let Some(val) = v.get_path_mut("a.b") {
        *val = Value::from(2);
    }
    assert_eq!(v.get_path("a.b").unwrap().as_i64(), Some(2));
}
