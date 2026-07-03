//! Property-based tests for noyalib using proptest.
//!
//! These tests verify invariants and properties that should hold for all
//! inputs.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Mapping, Number, Value, from_str, from_value, to_string, to_value};
#[cfg(feature = "lossless-u64")]
use noyalib::{ParserConfig, from_str_with_config};
use proptest::prelude::*;

// ============================================================================
// Value Generators
// ============================================================================

/// Generate arbitrary Number values
#[cfg(not(feature = "lossless-u64"))]
fn arb_number() -> impl Strategy<Value = Number> {
    prop_oneof![
        any::<i64>().prop_map(Number::Integer),
        // Use finite floats to avoid NaN comparison issues
        any::<f64>()
            .prop_filter("finite floats only", |f| f.is_finite())
            .prop_map(Number::Float),
    ]
}

/// Generate arbitrary Number values, including canonical unsigned values.
#[cfg(feature = "lossless-u64")]
fn arb_number() -> impl Strategy<Value = Number> {
    prop_oneof![
        any::<i64>().prop_map(Number::Integer),
        ((i64::MAX as u64 + 1)..=u64::MAX).prop_map(Number::Unsigned),
        // Use finite floats to avoid NaN comparison issues
        any::<f64>()
            .prop_filter("finite floats only", |f| f.is_finite())
            .prop_map(Number::Float),
    ]
}

/// Generate arbitrary scalar Value (non-recursive)
fn arb_scalar_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        arb_number().prop_map(Value::Number),
        // Use simple strings without special YAML characters for roundtrip
        "[a-zA-Z0-9_]{0,20}".prop_map(Value::String),
    ]
}

/// Generate arbitrary Value (recursive, with depth limit)
fn arb_value() -> impl Strategy<Value = Value> {
    arb_scalar_value().prop_recursive(
        3,  // depth
        64, // desired size
        10, // items per collection
        |inner| {
            prop_oneof![
                // Sequence of values
                prop::collection::vec(inner.clone(), 0..5).prop_map(Value::Sequence),
                // Mapping with string keys
                prop::collection::vec(
                    ("[a-zA-Z][a-zA-Z0-9_]{0,10}".prop_map(String::from), inner),
                    0..5
                )
                .prop_map(|pairs| {
                    let mut map = Mapping::new();
                    for (k, v) in pairs {
                        let _ = map.insert(k, v);
                    }
                    Value::Mapping(map)
                }),
            ]
        },
    )
}

// ============================================================================
// Roundtrip Properties
// ============================================================================

proptest! {
    /// Serialization followed by deserialization should preserve the value
    #[test]
    fn roundtrip_value(value in arb_value()) {
        let yaml = to_string(&value).expect("serialization should succeed");
        let parsed: Value = from_str(&yaml).expect("deserialization should succeed");

        // Compare structurally (ignore exact float representation)
        prop_assert!(values_equal(&value, &parsed),
            "Roundtrip failed:\nOriginal: {:?}\nYAML: {}\nParsed: {:?}",
            value, yaml, parsed);
    }

    /// to_value followed by from_value should preserve the value
    #[test]
    fn roundtrip_to_from_value(value in arb_value()) {
        let serialized = to_value(&value).expect("to_value should succeed");
        let deserialized: Value = from_value(&serialized).expect("from_value should succeed");

        prop_assert!(values_equal(&value, &deserialized),
            "to_value/from_value roundtrip failed:\nOriginal: {:?}\nSerialized: {:?}\nDeserialized: {:?}",
            value, serialized, deserialized);
    }

    /// Integers should roundtrip exactly
    #[test]
    fn roundtrip_integer(n in any::<i64>()) {
        let value = Value::Number(Number::Integer(n));
        let yaml = to_string(&value).expect("serialization should succeed");
        let parsed: Value = from_str(&yaml).expect("deserialization should succeed");

        prop_assert_eq!(parsed.as_i64(), Some(n));
    }

    /// Unsigned integers should roundtrip exactly when the opt-in is active.
    #[cfg(feature = "lossless-u64")]
    #[test]
    fn roundtrip_unsigned_integer(n in any::<u64>()) {
        let value = Value::Number(Number::from(n));
        let yaml = to_string(&value).expect("serialization should succeed");
        let cfg = ParserConfig::new().lossless_u64_integers(true);
        let parsed: Value = from_str_with_config(&yaml, &cfg).expect("deserialization should succeed");

        prop_assert_eq!(parsed.as_u64(), Some(n));
    }

    /// Booleans should roundtrip exactly
    #[test]
    fn roundtrip_bool(b in any::<bool>()) {
        let value = Value::Bool(b);
        let yaml = to_string(&value).expect("serialization should succeed");
        let parsed: Value = from_str(&yaml).expect("deserialization should succeed");

        prop_assert_eq!(parsed.as_bool(), Some(b));
    }

    /// Simple strings should roundtrip exactly
    #[test]
    fn roundtrip_simple_string(s in "[a-zA-Z0-9_]{1,50}") {
        let value = Value::String(s.clone());
        let yaml = to_string(&value).expect("serialization should succeed");
        let parsed: Value = from_str(&yaml).expect("deserialization should succeed");

        prop_assert_eq!(parsed.as_str(), Some(s.as_str()));
    }

    /// Null should always roundtrip
    #[test]
    fn roundtrip_null(_dummy in Just(())) {
        let value = Value::Null;
        let yaml = to_string(&value).expect("serialization should succeed");
        let parsed: Value = from_str(&yaml).expect("deserialization should succeed");

        prop_assert!(parsed.is_null());
    }
}

// ============================================================================
// Number Properties
// ============================================================================

proptest! {
    /// Number::Integer should always return the same value via as_i64
    #[test]
    fn number_integer_as_i64(n in any::<i64>()) {
        let num = Number::Integer(n);
        prop_assert_eq!(num.as_i64(), Some(n));
    }

    /// Number::Integer with non-negative values should work with as_u64
    #[test]
    fn number_integer_as_u64(n in 0i64..=i64::MAX) {
        let num = Number::Integer(n);
        prop_assert_eq!(num.as_u64(), Some(n as u64));
    }

    /// Number::Float should always return the same value via as_f64
    #[test]
    fn number_float_as_f64(f in any::<f64>().prop_filter("finite", |f| f.is_finite())) {
        let num = Number::Float(f);
        prop_assert!((num.as_f64() - f).abs() < f64::EPSILON || num.as_f64() == f);
    }

    /// Number comparison should be reflexive
    #[test]
    fn number_cmp_reflexive(n in arb_number()) {
        prop_assert_eq!(n.cmp(&n), std::cmp::Ordering::Equal);
    }

    /// Number hash should be consistent with equality (for integers)
    #[test]
    fn number_hash_consistent_integers(a in any::<i64>(), b in any::<i64>()) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let na = Number::Integer(a);
        let nb = Number::Integer(b);

        if na == nb {
            let mut ha = DefaultHasher::new();
            let mut hb = DefaultHasher::new();
            na.hash(&mut ha);
            nb.hash(&mut hb);
            prop_assert_eq!(ha.finish(), hb.finish());
        }
    }

    /// Number hash for identical floats should be consistent
    #[test]
    fn number_hash_identical_floats(f in any::<f64>().prop_filter("finite", |f| f.is_finite())) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let n1 = Number::Float(f);
        let n2 = Number::Float(f);

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        n1.hash(&mut h1);
        n2.hash(&mut h2);

        // Same float value should hash the same
        prop_assert_eq!(h1.finish(), h2.finish());
    }
}

// ============================================================================
// Value Properties
// ============================================================================

proptest! {
    /// Value equality should be reflexive
    #[test]
    fn value_eq_reflexive(value in arb_value()) {
        let cloned = value.clone();
        prop_assert_eq!(value, cloned);
    }

    /// Value comparison should be reflexive
    #[test]
    fn value_cmp_reflexive(value in arb_value()) {
        prop_assert_eq!(value.cmp(&value), std::cmp::Ordering::Equal);
    }

    /// Value hash should be consistent with equality
    #[test]
    fn value_hash_consistent(a in arb_value(), b in arb_value()) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        if a == b {
            let mut ha = DefaultHasher::new();
            let mut hb = DefaultHasher::new();
            a.hash(&mut ha);
            b.hash(&mut hb);
            prop_assert_eq!(ha.finish(), hb.finish());
        }
    }

    /// Cloning a Value should produce an equal value
    #[test]
    fn value_clone_equals(value in arb_value()) {
        let cloned = value.clone();
        prop_assert_eq!(value, cloned);
    }
}

// ============================================================================
// Eq/Hash invariant — explicit regressions
// ============================================================================
//
// These pin the historical edge cases for `Number`'s Eq/Hash contract
// so they are always replayed regardless of proptest seed:
//   - `+0.0 == -0.0` (IEEE 754) — must hash to the same value.
//   - `NaN == NaN` (we make `Eq` reflexive) — must hash to the same value.
//   - `Float(1.0)` and `Integer(1)` are NOT equal — they may hash to
//     different values, but the variant tag in the hasher must keep
//     them apart in practice.

fn hash_one<T: std::hash::Hash>(t: &T) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}

#[test]
fn number_zero_negative_zero_eq_hash() {
    let p = Number::Float(0.0);
    let n = Number::Float(-0.0);
    assert_eq!(p, n, "+0.0 == -0.0 by IEEE 754");
    assert_eq!(hash_one(&p), hash_one(&n), "Eq/Hash invariant");
}

#[test]
fn number_nan_eq_hash() {
    let a = Number::Float(f64::NAN);
    let b = Number::Float(f64::from_bits(0x7FF8_0000_0000_0042));
    assert_eq!(a, b, "all NaNs compare equal under our reflexive-Eq policy");
    assert_eq!(hash_one(&a), hash_one(&b), "Eq/Hash invariant");
}

#[test]
fn value_zero_negative_zero_eq_hash() {
    let p = Value::Number(Number::Float(0.0));
    let n = Value::Number(Number::Float(-0.0));
    assert_eq!(p, n);
    assert_eq!(hash_one(&p), hash_one(&n));
}

// ============================================================================
// Mapping Properties
// ============================================================================

proptest! {
    /// Inserting a key should make it retrievable
    #[test]
    fn mapping_insert_get(
        key in "[a-zA-Z][a-zA-Z0-9_]{0,10}",
        value in arb_scalar_value()
    ) {
        let mut map = Mapping::new();
        let _ = map.insert(key.clone(), value.clone());

        prop_assert_eq!(map.get(&key), Some(&value));
    }

    /// Removing a key should make it unretrievable
    #[test]
    fn mapping_remove(
        key in "[a-zA-Z][a-zA-Z0-9_]{0,10}",
        value in arb_scalar_value()
    ) {
        let mut map = Mapping::new();
        let _ = map.insert(key.clone(), value);
        let _ = map.shift_remove(&key);

        prop_assert!(map.get(&key).is_none());
    }

    /// Mapping length should match number of unique keys inserted
    #[test]
    fn mapping_length(
        pairs in prop::collection::vec(
            ("[a-zA-Z][a-zA-Z0-9]{0,5}".prop_map(String::from), arb_scalar_value()),
            0..10
        )
    ) {
        let mut map = Mapping::new();
        let mut unique_keys = std::collections::HashSet::new();

        for (k, v) in pairs {
            let _ = map.insert(k.clone(), v);
            let _ = unique_keys.insert(k);
        }

        prop_assert_eq!(map.len(), unique_keys.len());
    }
}

// ============================================================================
// Merge Properties
// ============================================================================

proptest! {
    /// Merging with an empty mapping should not change the base
    #[test]
    fn merge_with_empty(base in arb_value()) {
        let mut merged = base.clone();
        merged.merge(Value::Mapping(Mapping::new()));

        // If base was a mapping, it should be unchanged
        if base.is_mapping() {
            prop_assert!(values_equal(&merged, &base));
        }
    }

    /// Merging into an empty mapping should produce the other mapping
    #[test]
    fn merge_into_empty(
        pairs in prop::collection::vec(
            ("[a-zA-Z][a-zA-Z0-9]{0,5}".prop_map(String::from), arb_scalar_value()),
            1..5
        )
    ) {
        let mut base = Value::Mapping(Mapping::new());

        let mut other = Mapping::new();
        for (k, v) in pairs.iter() {
            let _ = other.insert(k.clone(), v.clone());
        }

        let other_clone = other.clone();
        base.merge(Value::Mapping(other));

        // Base should now contain all keys from other with their final values
        for (k, v) in other_clone.iter() {
            prop_assert_eq!(base.get(k.as_str()), Some(v));
        }
    }
}

// ============================================================================
// Path Access Properties
// ============================================================================

proptest! {
    /// get_path on a simple mapping should work like get
    #[test]
    fn get_path_simple(
        key in "[a-zA-Z][a-zA-Z0-9_]{0,10}",
        value in arb_scalar_value()
    ) {
        let mut map = Mapping::new();
        let _ = map.insert(key.clone(), value.clone());
        let v = Value::Mapping(map);

        prop_assert_eq!(v.get_path(&key), Some(&value));
    }

    /// get_path with invalid path should return None
    #[test]
    fn get_path_nonexistent(
        key in "[a-zA-Z][a-zA-Z0-9_]{0,10}",
        value in arb_scalar_value()
    ) {
        let mut map = Mapping::new();
        let _ = map.insert(key, value);
        let v = Value::Mapping(map);

        prop_assert!(v.get_path("nonexistent_key_xyz").is_none());
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Compare two Values for structural equality, handling float comparison
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(Number::Integer(a)), Value::Number(Number::Integer(b))) => a == b,
        #[cfg(feature = "lossless-u64")]
        (Value::Number(Number::Unsigned(a)), Value::Number(Number::Unsigned(b))) => a == b,
        (Value::Number(Number::Float(a)), Value::Number(Number::Float(b))) => {
            (a - b).abs() < 1e-10 || (a.is_nan() && b.is_nan())
        }
        (Value::Number(Number::Integer(a)), Value::Number(Number::Float(b))) => {
            (*a as f64 - b).abs() < 1e-10
        }
        (Value::Number(Number::Float(a)), Value::Number(Number::Integer(b))) => {
            (a - *b as f64).abs() < 1e-10
        }
        #[cfg(feature = "lossless-u64")]
        (Value::Number(Number::Unsigned(a)), Value::Number(Number::Float(b))) => {
            (*a as f64 - b).abs() < 1e-10
        }
        #[cfg(feature = "lossless-u64")]
        (Value::Number(Number::Float(a)), Value::Number(Number::Unsigned(b))) => {
            (a - *b as f64).abs() < 1e-10
        }
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Sequence(a), Value::Sequence(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Mapping(a), Value::Mapping(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, v)| b.get(k).is_some_and(|bv| values_equal(v, bv)))
        }
        (Value::Tagged(a), Value::Tagged(b)) => {
            a.tag() == b.tag() && values_equal(a.value(), b.value())
        }
        _ => false,
    }
}

// ============================================================================
// DoS-limit × lossless-u64 interaction
// ============================================================================

/// The `lossless-u64` opt-in must not weaken any of the parser's
/// hardening budgets (`max_depth`, `max_document_length`,
/// `max_alias_expansions`, `max_mapping_keys`,
/// `max_sequence_length`). Every scalar-resolution path — including
/// the new `Number::Unsigned` route — sits inside the loader loop
/// that increments those counters, so the budget check MUST fire
/// before the scalar resolver runs.
///
/// The tests below feed adversarial documents that pair a
/// budget-triggering shape with a `u64::MAX` (or MAX-adjacent)
/// value. The library MUST return a `LimitExceeded` (or moral
/// equivalent) `Error` — never `Ok`, never a panic. The variant
/// matters less than the mechanical fact that the limit fires.
#[cfg(feature = "lossless-u64")]
mod dos_limits_lossless_u64 {
    use super::*;

    proptest! {
        /// A deeply-nested document ending in a `u64` scalar must
        /// error on the depth limit — never succeed, never panic.
        #[test]
        fn depth_limit_fires_before_u64_scalar(
            depth in 200usize..1_000,
            n in (i64::MAX as u64 + 1)..=u64::MAX,
        ) {
            // Build "[ [ [ ... [ N ] ... ] ] ]" nested `depth` levels.
            let mut yaml = String::with_capacity(depth * 2 + 32);
            for _ in 0..depth {
                yaml.push('[');
            }
            yaml.push_str(&n.to_string());
            for _ in 0..depth {
                yaml.push(']');
            }
            yaml.push('\n');

            let cfg = ParserConfig::new()
                .max_depth(128)
                .lossless_u64_integers(true);
            let res: Result<Value, _> = from_str_with_config(&yaml, &cfg);
            prop_assert!(res.is_err(),
                "depth {} vs max_depth 128 should fail, got Ok({:?})",
                depth,
                res.ok());
        }

        /// A document *at* the depth limit ending in a `u64::MAX`
        /// scalar must round-trip cleanly (the budget check should
        /// not fire before a legal document finishes).
        #[test]
        fn depth_limit_permits_legal_u64_scalar(
            n in (i64::MAX as u64 + 1)..=u64::MAX,
        ) {
            let depth = 32; // well within default max_depth 128
            let mut yaml = String::with_capacity(depth * 2 + 32);
            for _ in 0..depth { yaml.push('['); }
            yaml.push_str(&n.to_string());
            for _ in 0..depth { yaml.push(']'); }
            yaml.push('\n');

            let cfg = ParserConfig::new()
                .max_depth(128)
                .lossless_u64_integers(true);
            let v: Value = from_str_with_config(&yaml, &cfg)
                .expect("legal-depth u64 doc should parse");
            // Walk down to the scalar and confirm it kept its value.
            let mut cursor = &v;
            for _ in 0..depth {
                match cursor {
                    Value::Sequence(s) => cursor = &s[0],
                    other => panic!("expected sequence, got {other:?}"),
                }
            }
            match cursor {
                Value::Number(Number::Unsigned(m)) => prop_assert_eq!(*m, n),
                other => panic!("expected Unsigned({n}), got {other:?}"),
            }
        }

        /// A document larger than `max_document_length` must error
        /// even when its payload is a plain `u64` scalar.
        #[test]
        fn document_length_limit_fires_before_u64_scalar(
            padding in 2_000usize..8_000,
            n in (i64::MAX as u64 + 1)..=u64::MAX,
        ) {
            // "# " + N spaces + "id: <n>\n" — padding drives past
            // `max_document_length = 1024`.
            let mut yaml = String::from("# ");
            yaml.extend(core::iter::repeat_n(' ', padding));
            yaml.push_str(&format!("\nid: {n}\n"));

            let cfg = ParserConfig::new()
                .max_document_length(1024)
                .lossless_u64_integers(true);
            let res: Result<Value, _> = from_str_with_config(&yaml, &cfg);
            prop_assert!(res.is_err(),
                "doc-length {} vs max 1024 should fail, got Ok",
                yaml.len());
        }

        /// A mapping with more keys than `max_mapping_keys` must
        /// error — the u64 value in each entry must NOT provide a
        /// bypass.
        #[test]
        fn mapping_keys_limit_fires_before_u64_scalar(
            keys in 65usize..256,
            n in (i64::MAX as u64 + 1)..=u64::MAX,
        ) {
            let mut yaml = String::new();
            for i in 0..keys {
                yaml.push_str(&format!("k{i}: {n}\n"));
            }

            let cfg = ParserConfig::new()
                .max_mapping_keys(64)
                .lossless_u64_integers(true);
            let res: Result<Value, _> = from_str_with_config(&yaml, &cfg);
            prop_assert!(res.is_err(),
                "keys {} vs max_mapping_keys 64 should fail, got Ok",
                keys);
        }

        /// A sequence longer than `max_sequence_length` must error
        /// even when every element is a lossless `u64`.
        #[test]
        fn sequence_length_limit_fires_before_u64_scalar(
            elems in 65usize..256,
            n in (i64::MAX as u64 + 1)..=u64::MAX,
        ) {
            let mut yaml = String::from("[");
            for i in 0..elems {
                if i > 0 { yaml.push_str(", "); }
                yaml.push_str(&n.to_string());
            }
            yaml.push_str("]\n");

            let cfg = ParserConfig::new()
                .max_sequence_length(64)
                .lossless_u64_integers(true);
            let res: Result<Value, _> = from_str_with_config(&yaml, &cfg);
            prop_assert!(res.is_err(),
                "elems {} vs max_sequence_length 64 should fail, got Ok",
                elems);
        }

        /// No `u64::MAX` scalar can be silently interpreted as an
        /// `i64` (wrap-to-negative). Under the opt-in it MUST come
        /// back as `Number::Unsigned` (or fail cleanly); the
        /// `as_i64()` accessor MUST NOT return `Some(-1)`.
        #[test]
        fn no_signed_wrap_at_u64_max(n in (i64::MAX as u64 + 1)..=u64::MAX) {
            let yaml = format!("id: {n}\n");
            let cfg = ParserConfig::new().lossless_u64_integers(true);
            let v: Value = from_str_with_config(&yaml, &cfg)
                .expect("legal-shape u64 doc should parse");
            let scalar = &v["id"];
            prop_assert!(
                !matches!(scalar, Value::Number(Number::Integer(_))),
                "u64::MAX-adjacent scalar {n} incorrectly parsed as Integer: {scalar:?}"
            );
        }
    }
}
