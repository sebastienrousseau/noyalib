//! Tests for MappingAny type.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use noyalib::{Mapping, MappingAny, Value, from_str, to_string};

// ============================================================================
// Basic Operations
// ============================================================================

#[test]
fn test_mapping_any_new() {
    let map = MappingAny::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[test]
fn test_mapping_any_with_capacity() {
    let map = MappingAny::with_capacity(10);
    assert!(map.is_empty());
    assert!(map.capacity() >= 10);
}

#[test]
fn test_mapping_any_insert_and_get() {
    let mut map = MappingAny::new();

    // Insert with various key types
    let _ = map.insert(Value::from("string_key"), Value::from(1));
    let _ = map.insert(Value::from(42), Value::from("int_key"));
    let _ = map.insert(Value::Bool(true), Value::from("bool_key"));
    let _ = map.insert(Value::Null, Value::from("null_key"));

    assert_eq!(map.len(), 4);
    assert_eq!(
        map.get(&Value::from("string_key")).unwrap().as_i64(),
        Some(1)
    );
    assert_eq!(map.get(&Value::from(42)).unwrap().as_str(), Some("int_key"));
    assert_eq!(
        map.get(&Value::Bool(true)).unwrap().as_str(),
        Some("bool_key")
    );
    assert_eq!(map.get(&Value::Null).unwrap().as_str(), Some("null_key"));
}

#[test]
fn test_mapping_any_insert_overwrite() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from(1), Value::from("first"));
    let old = map.insert(Value::from(1), Value::from("second"));

    assert_eq!(old.unwrap().as_str(), Some("first"));
    assert_eq!(map.get(&Value::from(1)).unwrap().as_str(), Some("second"));
}

#[test]
fn test_mapping_any_contains_key() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from("value"));

    assert!(map.contains_key(&Value::from("key")));
    assert!(!map.contains_key(&Value::from("nonexistent")));
}

#[test]
fn test_mapping_any_get_mut() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from(1));

    if let Some(v) = map.get_mut(&Value::from("key")) {
        *v = Value::from(2);
    }

    assert_eq!(map.get(&Value::from("key")).unwrap().as_i64(), Some(2));
}

#[test]
fn test_mapping_any_remove() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("a"), Value::from(1));
    let _ = map.insert(Value::from("b"), Value::from(2));
    let _ = map.insert(Value::from("c"), Value::from(3));

    let removed = map.remove(&Value::from("b"));
    assert_eq!(removed.unwrap().as_i64(), Some(2));
    assert_eq!(map.len(), 2);

    // Verify order is preserved
    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys[0].as_str(), Some("a"));
    assert_eq!(keys[1].as_str(), Some("c"));
}

#[test]
fn test_mapping_any_remove_entry() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from("value"));

    let (k, v) = map.remove_entry(&Value::from("key")).unwrap();
    assert_eq!(k.as_str(), Some("key"));
    assert_eq!(v.as_str(), Some("value"));
    assert!(map.is_empty());
}

#[test]
fn test_mapping_any_swap_remove() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("a"), Value::from(1));
    let _ = map.insert(Value::from("b"), Value::from(2));
    let _ = map.insert(Value::from("c"), Value::from(3));

    let removed = map.swap_remove(&Value::from("a"));
    assert_eq!(removed.unwrap().as_i64(), Some(1));
    // After swap_remove, order may change (last element moves to removed position)
    assert_eq!(map.len(), 2);
}

#[test]
fn test_mapping_any_clear() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("a"), Value::from(1));
    let _ = map.insert(Value::from("b"), Value::from(2));

    map.clear();
    assert!(map.is_empty());
}

// ============================================================================
// Index Operations
// ============================================================================

#[test]
fn test_mapping_any_get_index() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("first"), Value::from(1));
    let _ = map.insert(Value::from("second"), Value::from(2));

    let (k, v) = map.get_index(0).unwrap();
    assert_eq!(k.as_str(), Some("first"));
    assert_eq!(v.as_i64(), Some(1));

    let (k, v) = map.get_index(1).unwrap();
    assert_eq!(k.as_str(), Some("second"));
    assert_eq!(v.as_i64(), Some(2));

    assert!(map.get_index(2).is_none());
}

#[test]
fn test_mapping_any_get_index_mut() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from(1));

    if let Some((_, v)) = map.get_index_mut(0) {
        *v = Value::from(99);
    }

    assert_eq!(map.get(&Value::from("key")).unwrap().as_i64(), Some(99));
}

#[test]
fn test_mapping_any_first_last() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("a"), Value::from(1));
    let _ = map.insert(Value::from("b"), Value::from(2));
    let _ = map.insert(Value::from("c"), Value::from(3));

    let (k, v) = map.first().unwrap();
    assert_eq!(k.as_str(), Some("a"));
    assert_eq!(v.as_i64(), Some(1));

    let (k, v) = map.last().unwrap();
    assert_eq!(k.as_str(), Some("c"));
    assert_eq!(v.as_i64(), Some(3));
}

#[test]
fn test_mapping_any_pop_first_last() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("a"), Value::from(1));
    let _ = map.insert(Value::from("b"), Value::from(2));
    let _ = map.insert(Value::from("c"), Value::from(3));

    let (k, v) = map.pop_first().unwrap();
    assert_eq!(k.as_str(), Some("a"));
    assert_eq!(v.as_i64(), Some(1));

    let (k, v) = map.pop_last().unwrap();
    assert_eq!(k.as_str(), Some("c"));
    assert_eq!(v.as_i64(), Some(3));

    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&Value::from("b")).unwrap().as_i64(), Some(2));
}

// ============================================================================
// Entry API
// ============================================================================

#[test]
fn test_mapping_any_entry() {
    let mut map = MappingAny::new();

    // Insert via entry
    let _ = map.entry(Value::from("key")).or_insert(Value::from(1));
    assert_eq!(map.get(&Value::from("key")).unwrap().as_i64(), Some(1));

    // Entry doesn't overwrite existing
    let _ = map.entry(Value::from("key")).or_insert(Value::from(999));
    assert_eq!(map.get(&Value::from("key")).unwrap().as_i64(), Some(1));
}

// ============================================================================
// Iteration
// ============================================================================

#[test]
fn test_mapping_any_iter() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from(1), Value::from("one"));
    let _ = map.insert(Value::from(2), Value::from("two"));

    let items: Vec<_> = map.iter().collect();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].0.as_i64(), Some(1));
    assert_eq!(items[0].1.as_str(), Some("one"));
}

#[test]
fn test_mapping_any_keys_values() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("a"), Value::from(1));
    let _ = map.insert(Value::from("b"), Value::from(2));

    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys.len(), 2);

    let values: Vec<_> = map.values().collect();
    assert_eq!(values.len(), 2);
}

#[test]
fn test_mapping_any_into_iter() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from("value"));

    for (k, v) in map {
        assert_eq!(k.as_str(), Some("key"));
        assert_eq!(v.as_str(), Some("value"));
    }
}

#[test]
fn test_mapping_any_retain() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from(1), Value::from("one"));
    let _ = map.insert(Value::from(2), Value::from("two"));
    let _ = map.insert(Value::from(3), Value::from("three"));

    // Keep only values that end with "e"
    map.retain(|_, v| v.as_str().map(|s| s.ends_with('e')).unwrap_or(false));

    assert_eq!(map.len(), 2);
    assert!(map.contains_key(&Value::from(1))); // "one"
    assert!(map.contains_key(&Value::from(3))); // "three"
    assert!(!map.contains_key(&Value::from(2))); // "two" removed
}

// ============================================================================
// Non-String Keys
// ============================================================================

#[test]
fn test_mapping_any_integer_keys() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from(1), Value::from("one"));
    let _ = map.insert(Value::from(2), Value::from("two"));
    let _ = map.insert(Value::from(-1), Value::from("negative one"));

    assert_eq!(map.get(&Value::from(1)).unwrap().as_str(), Some("one"));
    assert_eq!(
        map.get(&Value::from(-1)).unwrap().as_str(),
        Some("negative one")
    );
}

#[test]
fn test_mapping_any_bool_keys() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::Bool(true), Value::from("yes"));
    let _ = map.insert(Value::Bool(false), Value::from("no"));

    assert_eq!(map.get(&Value::Bool(true)).unwrap().as_str(), Some("yes"));
    assert_eq!(map.get(&Value::Bool(false)).unwrap().as_str(), Some("no"));
}

#[test]
fn test_mapping_any_null_key() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::Null, Value::from("null value"));

    assert_eq!(map.get(&Value::Null).unwrap().as_str(), Some("null value"));
}

#[test]
fn test_mapping_any_sequence_key() {
    let mut map = MappingAny::new();
    let key = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let _ = map.insert(key.clone(), Value::from("sequence key"));

    assert_eq!(map.get(&key).unwrap().as_str(), Some("sequence key"));
}

#[test]
fn test_mapping_any_nested_mapping_key() {
    let mut inner = Mapping::new();
    let _ = inner.insert("nested", Value::from("value"));
    let key = Value::Mapping(inner);

    let mut map = MappingAny::new();
    let _ = map.insert(key.clone(), Value::from("mapping key"));

    assert_eq!(map.get(&key).unwrap().as_str(), Some("mapping key"));
}

// ============================================================================
// Conversion
// ============================================================================

#[test]
fn test_mapping_any_into_mapping_success() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key1"), Value::from(1));
    let _ = map.insert(Value::from("key2"), Value::from(2));

    let mapping = map.into_mapping().unwrap();
    assert_eq!(mapping.len(), 2);
    assert_eq!(mapping.get("key1").unwrap().as_i64(), Some(1));
}

#[test]
fn test_mapping_any_into_mapping_fail() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("string_key"), Value::from(1));
    let _ = map.insert(Value::from(42), Value::from(2)); // Non-string key

    assert!(map.into_mapping().is_none());
}

#[test]
fn test_mapping_any_from_mapping() {
    let mut mapping = Mapping::new();
    let _ = mapping.insert("key1", Value::from(1));
    let _ = mapping.insert("key2", Value::from(2));

    let map = MappingAny::from(mapping);
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&Value::from("key1")).unwrap().as_i64(), Some(1));
}

#[test]
fn test_mapping_any_from_index_map() {
    use indexmap::IndexMap;

    let mut inner = IndexMap::new();
    let _ = inner.insert(Value::from("key"), Value::from("value"));

    let map = MappingAny::from(inner);
    assert_eq!(map.len(), 1);
}

#[test]
fn test_mapping_any_into_index_map() {
    use indexmap::IndexMap;

    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from("value"));

    let inner: IndexMap<Value, Value> = map.into();
    assert_eq!(inner.len(), 1);
}

#[test]
fn test_mapping_any_from_array() {
    let map = MappingAny::from([
        (Value::from("a"), Value::from(1)),
        (Value::from("b"), Value::from(2)),
    ]);

    assert_eq!(map.len(), 2);
}

#[test]
fn test_mapping_any_from_iter() {
    let pairs = vec![
        (Value::from("a"), Value::from(1)),
        (Value::from("b"), Value::from(2)),
    ];

    let map: MappingAny = pairs.into_iter().collect();
    assert_eq!(map.len(), 2);
}

// ============================================================================
// Serde
// ============================================================================

#[test]
fn test_mapping_any_serialize() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from("value"));

    let yaml = to_string(&map).unwrap();
    assert!(yaml.contains("key: value"));
}

#[test]
fn test_mapping_any_deserialize() {
    let yaml = "key: value\n";
    let map: MappingAny = from_str(yaml).unwrap();

    assert_eq!(map.len(), 1);
    assert_eq!(
        map.get(&Value::from("key")).unwrap().as_str(),
        Some("value")
    );
}

#[test]
fn test_mapping_any_roundtrip() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("string"), Value::from("text"));
    let _ = map.insert(Value::from("number"), Value::from(42));
    let _ = map.insert(Value::from("bool"), Value::Bool(true));

    let yaml = to_string(&map).unwrap();
    let parsed: MappingAny = from_str(&yaml).unwrap();

    assert_eq!(parsed.len(), 3);
    assert_eq!(
        parsed.get(&Value::from("string")).unwrap().as_str(),
        Some("text")
    );
}

#[test]
fn test_mapping_any_deserialize_integer_key() {
    // YAML allows integer keys
    let yaml = "1: one\n2: two\n";
    let map: MappingAny = from_str(yaml).unwrap();

    assert_eq!(map.len(), 2);
    // Note: YAML parser may parse "1" as integer or string depending on config
    // We test that it deserializes without error
}

// ============================================================================
// Ordering and Comparison
// ============================================================================

#[test]
fn test_mapping_any_sort_keys() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("c"), Value::from(3));
    let _ = map.insert(Value::from("a"), Value::from(1));
    let _ = map.insert(Value::from("b"), Value::from(2));

    map.sort_keys();

    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys[0].as_str(), Some("a"));
    assert_eq!(keys[1].as_str(), Some("b"));
    assert_eq!(keys[2].as_str(), Some("c"));
}

#[test]
fn test_mapping_any_reverse() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("a"), Value::from(1));
    let _ = map.insert(Value::from("b"), Value::from(2));
    let _ = map.insert(Value::from("c"), Value::from(3));

    map.reverse();

    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys[0].as_str(), Some("c"));
    assert_eq!(keys[1].as_str(), Some("b"));
    assert_eq!(keys[2].as_str(), Some("a"));
}

#[test]
fn test_mapping_any_eq() {
    let mut map1 = MappingAny::new();
    let _ = map1.insert(Value::from("a"), Value::from(1));

    let mut map2 = MappingAny::new();
    let _ = map2.insert(Value::from("a"), Value::from(1));

    let mut map3 = MappingAny::new();
    let _ = map3.insert(Value::from("a"), Value::from(2));

    assert_eq!(map1, map2);
    assert_ne!(map1, map3);
}

#[test]
fn test_mapping_any_ord() {
    let mut map1 = MappingAny::new();
    let _ = map1.insert(Value::from("a"), Value::from(1));

    let mut map2 = MappingAny::new();
    let _ = map2.insert(Value::from("a"), Value::from(1));
    let _ = map2.insert(Value::from("b"), Value::from(2));

    // map1 < map2 because map2 has more elements
    assert!(map1 < map2);
}

#[test]
fn test_mapping_any_hash() {
    let mut map1 = MappingAny::new();
    let _ = map1.insert(Value::from("key"), Value::from("value"));

    let mut map2 = MappingAny::new();
    let _ = map2.insert(Value::from("key"), Value::from("value"));

    fn hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    assert_eq!(hash(&map1), hash(&map2));
}

// ============================================================================
// Display
// ============================================================================

#[test]
fn test_mapping_any_display() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("a"), Value::from(1));
    let _ = map.insert(Value::from("b"), Value::from(2));

    let s = map.to_string();
    assert!(s.contains("a: 1"));
    assert!(s.contains("b: 2"));
}

// ============================================================================
// Index Operators
// ============================================================================

#[test]
fn test_mapping_any_index() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from("value"));

    let key = Value::from("key");
    assert_eq!(map[&key].as_str(), Some("value"));
}

#[test]
fn test_mapping_any_index_mut() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from(1));

    let key = Value::from("key");
    map[&key] = Value::from(2);

    assert_eq!(map.get(&key).unwrap().as_i64(), Some(2));
}

#[test]
#[should_panic(expected = "key not found")]
fn test_mapping_any_index_panic() {
    let map = MappingAny::new();
    let key = Value::from("nonexistent");
    let _ = &map[&key];
}

// ============================================================================
// Memory Management
// ============================================================================

#[test]
fn test_mapping_any_reserve_shrink() {
    let mut map = MappingAny::with_capacity(100);
    let _ = map.insert(Value::from("a"), Value::from(1));

    assert!(map.capacity() >= 100);

    map.shrink_to_fit();
    assert!(map.capacity() < 100);
}

#[test]
fn test_mapping_any_extend() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("a"), Value::from(1));

    let extra = vec![
        (Value::from("b"), Value::from(2)),
        (Value::from("c"), Value::from(3)),
    ];

    map.extend(extra);

    assert_eq!(map.len(), 3);
}

#[test]
fn test_mapping_any_into_inner() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from("value"));

    let inner = map.into_inner();
    assert_eq!(inner.len(), 1);
}

#[test]
fn test_mapping_any_from_inner() {
    use indexmap::IndexMap;

    let mut inner = IndexMap::new();
    let _ = inner.insert(Value::from("key"), Value::from("value"));

    let map = MappingAny::from_inner(inner);
    assert_eq!(map.len(), 1);
}

// ============================================================================
// Clone and Default
// ============================================================================

#[test]
fn test_mapping_any_clone() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("key"), Value::from("value"));

    let cloned = map.clone();
    assert_eq!(map, cloned);
}

#[test]
fn test_mapping_any_default() {
    let map = MappingAny::default();
    assert!(map.is_empty());
}
