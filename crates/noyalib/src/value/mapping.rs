//! YAML mapping types (`Mapping`, `MappingAny`).

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use super::Value;
use crate::prelude::*;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::{Index, IndexMut};
use indexmap::IndexMap;
use indexmap::map::{IntoIter, Iter, IterMut, Keys, Values, ValuesMut};
use rustc_hash::FxBuildHasher;
use serde::{Deserialize, Serialize};

/// Fast IndexMap using FxBuildHasher.
type FxIndexMap<K, V> = IndexMap<K, V, FxBuildHasher>;

/// A YAML mapping (dictionary/object).
///
/// This is an ordered map that preserves insertion order, wrapping
/// `IndexMap<String, Value>`. It provides a comprehensive API for working with
/// YAML mappings.
///
/// # Examples
///
/// ```rust
/// use noyalib::{Mapping, Value};
///
/// let mut map = Mapping::new();
/// map.insert("name", Value::from("test"));
/// map.insert("value", Value::from(42));
///
/// assert_eq!(map.len(), 2);
/// assert_eq!(map.get("name").unwrap().as_str(), Some("test"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mapping(FxIndexMap<String, Value>);

impl Mapping {
    /// Creates an empty mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Mapping;
    /// let m = Mapping::new();
    /// assert!(m.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(FxIndexMap::default())
    }

    /// Creates an empty mapping with the specified capacity.
    ///
    /// Pre-allocates room for `capacity` entries to avoid
    /// rehashing during the first inserts.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Mapping;
    /// let m = Mapping::with_capacity(16);
    /// assert!(m.capacity() >= 16);
    /// ```
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(FxIndexMap::with_capacity_and_hasher(
            capacity,
            FxBuildHasher,
        ))
    }

    /// Returns the number of key-value pairs the mapping can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Mapping;
    /// let m = Mapping::with_capacity(8);
    /// assert!(m.capacity() >= 8);
    /// ```
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Reserves capacity for at least `additional` more key-value pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Mapping;
    /// let mut m = Mapping::new();
    /// m.reserve(64);
    /// assert!(m.capacity() >= 64);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Shrinks the capacity of the mapping as much as possible.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Mapping;
    /// let mut m = Mapping::with_capacity(64);
    /// m.shrink_to_fit();
    /// // capacity may now be 0 or any small implementation-defined value.
    /// ```
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Returns the number of key-value pairs in the mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// assert_eq!(m.len(), 1);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the mapping contains no key-value pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Mapping;
    /// assert!(Mapping::new().is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the mapping, removing all key-value pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.clear();
    /// assert!(m.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Inserts a key-value pair into the mapping.
    ///
    /// If the mapping already had this key present, the value is updated,
    /// and the old value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// assert_eq!(m.insert("a", Value::from(1_i64)), None);
    /// assert_eq!(m.insert("a", Value::from(2_i64)).and_then(|v| v.as_i64()), Some(1));
    /// ```
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        self.0.insert(key.into(), value)
    }

    /// Returns `true` if the mapping contains the specified key.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// assert!(m.contains_key("a"));
    /// assert!(!m.contains_key("b"));
    /// ```
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// assert_eq!(m.get("a").and_then(Value::as_i64), Some(1));
    /// assert!(m.get("b").is_none());
    /// ```
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// if let Some(v) = m.get_mut("a") {
    ///     *v = Value::from(2_i64);
    /// }
    /// assert_eq!(m.get("a").and_then(Value::as_i64), Some(2));
    /// ```
    #[must_use]
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.0.get_mut(key)
    }

    /// Returns a reference to the key-value pair at the given index.
    ///
    /// Indexing follows insertion order (this is an `IndexMap`).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("first", Value::from(1_i64));
    /// m.insert("second", Value::from(2_i64));
    /// assert_eq!(m.get_index(0).map(|(k, _)| k.as_str()), Some("first"));
    /// ```
    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<(&String, &Value)> {
        self.0.get_index(index)
    }

    /// Returns a mutable reference to the key-value pair at the given index.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// if let Some((_, v)) = m.get_index_mut(0) {
    ///     *v = Value::from(99_i64);
    /// }
    /// assert_eq!(m.get("a").and_then(Value::as_i64), Some(99));
    /// ```
    #[must_use]
    pub fn get_index_mut(&mut self, index: usize) -> Option<(&String, &mut Value)> {
        self.0.get_index_mut(index)
    }

    /// Returns the index of the given key, if present.
    ///
    /// Indexing follows insertion order (this is an `IndexMap`); a key
    /// keeps its original index when its value is overwritten by a
    /// later [`Mapping::insert`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("first", Value::from(1_i64));
    /// m.insert("second", Value::from(2_i64));
    /// assert_eq!(m.get_index_of("second"), Some(1));
    /// assert_eq!(m.get_index_of("absent"), None);
    /// ```
    #[must_use]
    pub fn get_index_of(&self, key: &str) -> Option<usize> {
        self.0.get_index_of(key)
    }

    /// Removes a key from the mapping, returning the value if the key was
    /// present.
    ///
    /// This operation preserves the order of remaining elements
    /// (uses `shift_remove` semantics, `O(n)`). For order-agnostic
    /// `O(1)` removal, see [`Mapping::swap_remove`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// assert_eq!(m.remove("a").and_then(|v| v.as_i64()), Some(1));
    /// assert!(m.remove("a").is_none());
    /// ```
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.shift_remove(key)
    }

    /// Removes a key from the mapping, returning the key-value pair if present.
    ///
    /// This operation preserves the order of remaining elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// let (k, v) = m.remove_entry("a").unwrap();
    /// assert_eq!(k, "a");
    /// assert_eq!(v.as_i64(), Some(1));
    /// ```
    pub fn remove_entry(&mut self, key: &str) -> Option<(String, Value)> {
        self.0.shift_remove_entry(key)
    }

    /// Removes a key by swapping it with the last element.
    ///
    /// This is `O(1)` but does not preserve order. For
    /// order-preserving removal, see [`Mapping::remove`] or
    /// [`Mapping::shift_remove`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// m.insert("c", Value::from(3_i64));
    /// m.swap_remove("a");
    /// // Order is no longer guaranteed; "c" might now sit where "a" was.
    /// assert_eq!(m.len(), 2);
    /// ```
    pub fn swap_remove(&mut self, key: &str) -> Option<Value> {
        self.0.swap_remove(key)
    }

    /// Removes a key by shifting all elements after it.
    ///
    /// This preserves order but is `O(n)`. Equivalent to
    /// [`Mapping::remove`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// m.shift_remove("a");
    /// assert_eq!(m.iter().next().map(|(k, _)| k.as_str()), Some("b"));
    /// ```
    pub fn shift_remove(&mut self, key: &str) -> Option<Value> {
        self.0.shift_remove(key)
    }

    /// Gets the entry for the given key for in-place manipulation.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.entry("counter").or_insert(Value::from(0_i64));
    /// if let Some(Value::Number(n)) = m.get_mut("counter") {
    ///     if let Some(c) = n.as_i64() { *n = noyalib::Number::Integer(c + 1); }
    /// }
    /// assert_eq!(m.get("counter").and_then(Value::as_i64), Some(1));
    /// ```
    pub fn entry(&mut self, key: impl Into<String>) -> indexmap::map::Entry<'_, String, Value> {
        self.0.entry(key.into())
    }

    /// Retains only the key-value pairs specified by the predicate.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// m.insert("c", Value::from(3_i64));
    /// m.retain(|_k, v| v.as_i64().unwrap_or(0) >= 2);
    /// assert_eq!(m.len(), 2);
    /// assert!(!m.contains_key("a"));
    /// ```
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&String, &mut Value) -> bool,
    {
        self.0.retain(f);
    }

    /// Returns an iterator over the key-value pairs in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// let total: i64 = m.iter().filter_map(|(_, v)| v.as_i64()).sum();
    /// assert_eq!(total, 3);
    /// ```
    #[must_use]
    pub fn iter(&self) -> Iter<'_, String, Value> {
        self.0.iter()
    }

    /// Returns a mutable iterator over the key-value pairs in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Number, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// for (_, v) in m.iter_mut() {
    ///     if let Value::Number(Number::Integer(n)) = v { *n *= 2; }
    /// }
    /// assert_eq!(m.get("a").and_then(Value::as_i64), Some(2));
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<'_, String, Value> {
        self.0.iter_mut()
    }

    /// Returns an iterator over the keys in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// let keys: Vec<&str> = m.keys().map(String::as_str).collect();
    /// assert_eq!(keys, &["a", "b"]);
    /// ```
    #[must_use]
    pub fn keys(&self) -> Keys<'_, String, Value> {
        self.0.keys()
    }

    /// Returns an iterator over the values in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// let sum: i64 = m.values().filter_map(Value::as_i64).sum();
    /// assert_eq!(sum, 3);
    /// ```
    #[must_use]
    pub fn values(&self) -> Values<'_, String, Value> {
        self.0.values()
    }

    /// Returns a mutable iterator over the values in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Number, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(10_i64));
    /// m.insert("b", Value::from(20_i64));
    /// for v in m.values_mut() {
    ///     if let Value::Number(Number::Integer(n)) = v { *n /= 10; }
    /// }
    /// assert_eq!(m.get("a").and_then(Value::as_i64), Some(1));
    /// ```
    pub fn values_mut(&mut self) -> ValuesMut<'_, String, Value> {
        self.0.values_mut()
    }

    /// Returns the first key-value pair in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// assert_eq!(m.first().map(|(k, _)| k.as_str()), Some("a"));
    /// ```
    #[must_use]
    pub fn first(&self) -> Option<(&String, &Value)> {
        self.0.first()
    }

    /// Returns a mutable reference to the first key-value pair.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// if let Some((_, v)) = m.first_mut() { *v = Value::from(99_i64); }
    /// assert_eq!(m.get("a").and_then(Value::as_i64), Some(99));
    /// ```
    #[must_use]
    pub fn first_mut(&mut self) -> Option<(&String, &mut Value)> {
        self.0.first_mut()
    }

    /// Returns the last key-value pair in insertion order.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// assert_eq!(m.last().map(|(k, _)| k.as_str()), Some("b"));
    /// ```
    #[must_use]
    pub fn last(&self) -> Option<(&String, &Value)> {
        self.0.last()
    }

    /// Returns a mutable reference to the last key-value pair.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// if let Some((_, v)) = m.last_mut() { *v = Value::from(99_i64); }
    /// assert_eq!(m.get("b").and_then(Value::as_i64), Some(99));
    /// ```
    #[must_use]
    pub fn last_mut(&mut self) -> Option<(&String, &mut Value)> {
        self.0.last_mut()
    }

    /// Removes and returns the first key-value pair.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// let (k, _) = m.pop_first().unwrap();
    /// assert_eq!(k, "a");
    /// assert_eq!(m.len(), 1);
    /// ```
    pub fn pop_first(&mut self) -> Option<(String, Value)> {
        self.0.shift_remove_index(0)
    }

    /// Removes and returns the last key-value pair.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// let (k, _) = m.pop_last().unwrap();
    /// assert_eq!(k, "b");
    /// ```
    pub fn pop_last(&mut self) -> Option<(String, Value)> {
        self.0.pop()
    }

    /// Sorts the mapping by keys (lexicographic order).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("c", Value::from(3_i64));
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// m.sort_keys();
    /// let keys: Vec<&str> = m.keys().map(String::as_str).collect();
    /// assert_eq!(keys, &["a", "b", "c"]);
    /// ```
    pub fn sort_keys(&mut self) {
        self.0.sort_keys();
    }

    /// Reverses the order of key-value pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// m.insert("b", Value::from(2_i64));
    /// m.reverse();
    /// assert_eq!(m.first().map(|(k, _)| k.as_str()), Some("b"));
    /// ```
    pub fn reverse(&mut self) {
        self.0.reverse();
    }

    /// Extends the mapping with the contents of an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.extend([
    ///     ("a".to_owned(), Value::from(1_i64)),
    ///     ("b".to_owned(), Value::from(2_i64)),
    /// ]);
    /// assert_eq!(m.len(), 2);
    /// ```
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (String, Value)>,
    {
        self.0.extend(iter);
    }

    /// Consumes the mapping and returns its contents as an `IndexMap`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Mapping, Value};
    /// let mut m = Mapping::new();
    /// m.insert("a", Value::from(1_i64));
    /// let inner = m.into_inner();
    /// assert_eq!(inner.len(), 1);
    /// ```
    #[must_use]
    pub fn into_inner(self) -> IndexMap<String, Value> {
        // Convert from FxIndexMap to standard IndexMap for public API stability
        self.0.into_iter().collect()
    }

    /// Creates a mapping from an `IndexMap`.
    ///
    /// # Examples
    ///
    /// ```
    /// use indexmap::IndexMap;
    /// use noyalib::{Mapping, Value};
    /// let mut src = IndexMap::new();
    /// src.insert("a".to_owned(), Value::from(1_i64));
    /// let m = Mapping::from_inner(src);
    /// assert_eq!(m.get("a").and_then(Value::as_i64), Some(1));
    /// ```
    #[must_use]
    pub fn from_inner(map: IndexMap<String, Value>) -> Self {
        Self(map.into_iter().collect())
    }
}

impl Index<&str> for Mapping {
    type Output = Value;

    /// Index into the mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the mapping.
    #[track_caller]
    fn index(&self, key: &str) -> &Self::Output {
        self.0.get(key).expect("key not found in mapping")
    }
}

impl IndexMut<&str> for Mapping {
    /// Mutably index into the mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the mapping.
    #[track_caller]
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.0.get_mut(key).expect("key not found in mapping")
    }
}

impl IntoIterator for Mapping {
    type Item = (String, Value);
    type IntoIter = IntoIter<String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Mapping {
    type Item = (&'a String, &'a Value);
    type IntoIter = Iter<'a, String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Mapping {
    type Item = (&'a String, &'a mut Value);
    type IntoIter = IterMut<'a, String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl FromIterator<(String, Value)> for Mapping {
    fn from_iter<I: IntoIterator<Item = (String, Value)>>(iter: I) -> Self {
        Self(FxIndexMap::from_iter(iter))
    }
}

impl<const N: usize> From<[(String, Value); N]> for Mapping {
    fn from(arr: [(String, Value); N]) -> Self {
        let mut map = FxIndexMap::with_capacity_and_hasher(N, FxBuildHasher);
        for (k, v) in arr {
            let _ = map.insert(k, v);
        }
        Self(map)
    }
}

impl From<IndexMap<String, Value>> for Mapping {
    fn from(map: IndexMap<String, Value>) -> Self {
        Self(map.into_iter().collect())
    }
}

impl From<FxIndexMap<String, Value>> for Mapping {
    fn from(map: FxIndexMap<String, Value>) -> Self {
        Self(map)
    }
}

impl From<Mapping> for IndexMap<String, Value> {
    fn from(map: Mapping) -> Self {
        map.0.into_iter().collect()
    }
}

impl From<Mapping> for FxIndexMap<String, Value> {
    fn from(map: Mapping) -> Self {
        map.0
    }
}

impl From<Vec<(String, Value)>> for Mapping {
    fn from(v: Vec<(String, Value)>) -> Self {
        Self(v.into_iter().collect())
    }
}

impl Hash for Mapping {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for (k, v) in &self.0 {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl PartialOrd for Mapping {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Mapping {
    fn cmp(&self, other: &Self) -> Ordering {
        self.len().cmp(&other.len()).then_with(|| {
            for ((ak, av), (bk, bv)) in self.iter().zip(other.iter()) {
                match ak.cmp(bk) {
                    Ordering::Equal => {}
                    ord => return ord,
                }
                match av.cmp(bv) {
                    Ordering::Equal => continue,
                    ord => return ord,
                }
            }
            Ordering::Equal
        })
    }
}

impl fmt::Display for Mapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, (k, v)) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{k}: {v}")?;
        }
        write!(f, "}}")
    }
}

impl Serialize for Mapping {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Mapping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};

        struct MappingVisitor;

        impl<'de> Visitor<'de> for MappingVisitor {
            type Value = Mapping;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a YAML mapping")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Mapping, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut mapping = Mapping::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    let _ = mapping.insert(key, value);
                }
                Ok(mapping)
            }
        }

        deserializer.deserialize_map(MappingVisitor)
    }
}

/// A YAML mapping with `Value` keys.
///
/// Unlike [`Mapping`] which only supports `String` keys, `MappingAny` allows
/// any [`Value`] as a key. This is useful for representing YAML mappings where
/// keys might be numbers, booleans, or even nested structures.
///
/// # Examples
///
/// ```rust
/// use noyalib::{MappingAny, Value};
///
/// let mut map = MappingAny::new();
/// map.insert(Value::from(1), Value::from("one"));
/// map.insert(Value::from("two"), Value::from(2));
/// map.insert(Value::Bool(true), Value::from("yes"));
///
/// assert_eq!(map.len(), 3);
/// assert_eq!(map.get(&Value::from(1)).unwrap().as_str(), Some("one"));
/// ```
///
/// # YAML Example
///
/// This type can represent YAML like:
///
/// ```yaml
/// 1: one
/// "two": 2
/// true: yes
/// [1, 2]: nested key
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MappingAny(FxIndexMap<Value, Value>);

impl MappingAny {
    /// Creates an empty mapping.
    #[must_use]
    pub fn new() -> Self {
        Self(FxIndexMap::default())
    }

    /// Creates an empty mapping with the specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(FxIndexMap::with_capacity_and_hasher(
            capacity,
            FxBuildHasher,
        ))
    }

    /// Returns the number of key-value pairs the mapping can hold without
    /// reallocating.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Reserves capacity for at least `additional` more key-value pairs.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Shrinks the capacity of the mapping as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Returns the number of key-value pairs in the mapping.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the mapping contains no key-value pairs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the mapping, removing all key-value pairs.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Inserts a key-value pair into the mapping.
    ///
    /// If the mapping already had this key present, the value is updated,
    /// and the old value is returned.
    pub fn insert(&mut self, key: Value, value: Value) -> Option<Value> {
        self.0.insert(key, value)
    }

    /// Returns `true` if the mapping contains the specified key.
    #[must_use]
    pub fn contains_key(&self, key: &Value) -> bool {
        self.0.contains_key(key)
    }

    /// Returns a reference to the value corresponding to the key.
    #[must_use]
    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.0.get(key)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[must_use]
    pub fn get_mut(&mut self, key: &Value) -> Option<&mut Value> {
        self.0.get_mut(key)
    }

    /// Returns a reference to the key-value pair at the given index.
    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<(&Value, &Value)> {
        self.0.get_index(index)
    }

    /// Returns a mutable reference to the key-value pair at the given index.
    #[must_use]
    pub fn get_index_mut(&mut self, index: usize) -> Option<(&Value, &mut Value)> {
        self.0.get_index_mut(index)
    }

    /// Removes a key from the mapping, returning the value if the key was
    /// present.
    ///
    /// This operation preserves the order of remaining elements.
    pub fn remove(&mut self, key: &Value) -> Option<Value> {
        self.0.shift_remove(key)
    }

    /// Removes a key from the mapping, returning the key-value pair if present.
    ///
    /// This operation preserves the order of remaining elements.
    pub fn remove_entry(&mut self, key: &Value) -> Option<(Value, Value)> {
        self.0.shift_remove_entry(key)
    }

    /// Removes a key by swapping it with the last element.
    ///
    /// This is faster than `remove` but does not preserve order.
    pub fn swap_remove(&mut self, key: &Value) -> Option<Value> {
        self.0.swap_remove(key)
    }

    /// Removes a key by shifting all elements after it.
    ///
    /// This preserves order but is slower than `swap_remove`.
    pub fn shift_remove(&mut self, key: &Value) -> Option<Value> {
        self.0.shift_remove(key)
    }

    /// Gets the entry for the given key for in-place manipulation.
    pub fn entry(&mut self, key: Value) -> indexmap::map::Entry<'_, Value, Value> {
        self.0.entry(key)
    }

    /// Retains only the key-value pairs specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Value, &mut Value) -> bool,
    {
        self.0.retain(f);
    }

    /// Returns an iterator over the key-value pairs.
    #[must_use]
    pub fn iter(&self) -> Iter<'_, Value, Value> {
        self.0.iter()
    }

    /// Returns a mutable iterator over the key-value pairs.
    pub fn iter_mut(&mut self) -> IterMut<'_, Value, Value> {
        self.0.iter_mut()
    }

    /// Returns an iterator over the keys.
    #[must_use]
    pub fn keys(&self) -> Keys<'_, Value, Value> {
        self.0.keys()
    }

    /// Returns an iterator over the values.
    #[must_use]
    pub fn values(&self) -> Values<'_, Value, Value> {
        self.0.values()
    }

    /// Returns a mutable iterator over the values.
    pub fn values_mut(&mut self) -> ValuesMut<'_, Value, Value> {
        self.0.values_mut()
    }

    /// Returns the first key-value pair.
    #[must_use]
    pub fn first(&self) -> Option<(&Value, &Value)> {
        self.0.first()
    }

    /// Returns a mutable reference to the first key-value pair.
    #[must_use]
    pub fn first_mut(&mut self) -> Option<(&Value, &mut Value)> {
        self.0.first_mut()
    }

    /// Returns the last key-value pair.
    #[must_use]
    pub fn last(&self) -> Option<(&Value, &Value)> {
        self.0.last()
    }

    /// Returns a mutable reference to the last key-value pair.
    #[must_use]
    pub fn last_mut(&mut self) -> Option<(&Value, &mut Value)> {
        self.0.last_mut()
    }

    /// Removes and returns the first key-value pair.
    pub fn pop_first(&mut self) -> Option<(Value, Value)> {
        self.0.shift_remove_index(0)
    }

    /// Removes and returns the last key-value pair.
    pub fn pop_last(&mut self) -> Option<(Value, Value)> {
        self.0.pop()
    }

    /// Sorts the mapping by keys.
    pub fn sort_keys(&mut self) {
        self.0.sort_keys();
    }

    /// Reverses the order of key-value pairs.
    pub fn reverse(&mut self) {
        self.0.reverse();
    }

    /// Extends the mapping with the contents of an iterator.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (Value, Value)>,
    {
        self.0.extend(iter);
    }

    /// Returns the inner `IndexMap`.
    #[must_use]
    pub fn into_inner(self) -> IndexMap<Value, Value> {
        self.0.into_iter().collect()
    }

    /// Creates a mapping from an `IndexMap`.
    #[must_use]
    pub fn from_inner(map: IndexMap<Value, Value>) -> Self {
        Self(map.into_iter().collect())
    }

    /// Converts this `MappingAny` to a `Mapping` if all keys are strings.
    ///
    /// Returns `None` if any key is not a string value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{Mapping, MappingAny, Value};
    ///
    /// let mut map = MappingAny::new();
    /// map.insert(Value::from("key1"), Value::from(1));
    /// map.insert(Value::from("key2"), Value::from(2));
    ///
    /// let mapping = map.into_mapping().unwrap();
    /// assert_eq!(mapping.len(), 2);
    /// ```
    #[must_use]
    pub fn into_mapping(self) -> Option<Mapping> {
        let mut mapping = Mapping::with_capacity(self.len());
        for (k, v) in self.0 {
            if let Value::String(s) = k {
                let _ = mapping.insert(s, v);
            } else {
                return None;
            }
        }
        Some(mapping)
    }
}

impl Index<&Value> for MappingAny {
    type Output = Value;

    /// Index into the mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the mapping.
    #[track_caller]
    fn index(&self, key: &Value) -> &Self::Output {
        self.0.get(key).expect("key not found in mapping")
    }
}

impl IndexMut<&Value> for MappingAny {
    /// Mutably index into the mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the mapping.
    #[track_caller]
    fn index_mut(&mut self, key: &Value) -> &mut Self::Output {
        self.0.get_mut(key).expect("key not found in mapping")
    }
}

impl IntoIterator for MappingAny {
    type Item = (Value, Value);
    type IntoIter = IntoIter<Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a MappingAny {
    type Item = (&'a Value, &'a Value);
    type IntoIter = Iter<'a, Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut MappingAny {
    type Item = (&'a Value, &'a mut Value);
    type IntoIter = IterMut<'a, Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl FromIterator<(Value, Value)> for MappingAny {
    fn from_iter<I: IntoIterator<Item = (Value, Value)>>(iter: I) -> Self {
        Self(IndexMap::from_iter(iter))
    }
}

impl<const N: usize> From<[(Value, Value); N]> for MappingAny {
    fn from(arr: [(Value, Value); N]) -> Self {
        let mut map = FxIndexMap::with_capacity_and_hasher(N, FxBuildHasher);
        for (k, v) in arr {
            let _ = map.insert(k, v);
        }
        Self(map)
    }
}

impl From<IndexMap<Value, Value>> for MappingAny {
    fn from(map: IndexMap<Value, Value>) -> Self {
        Self(map.into_iter().collect())
    }
}

impl From<FxIndexMap<Value, Value>> for MappingAny {
    fn from(map: FxIndexMap<Value, Value>) -> Self {
        Self(map)
    }
}

impl From<MappingAny> for IndexMap<Value, Value> {
    fn from(map: MappingAny) -> Self {
        map.0.into_iter().collect()
    }
}

impl From<MappingAny> for FxIndexMap<Value, Value> {
    fn from(map: MappingAny) -> Self {
        map.0
    }
}

impl From<Mapping> for MappingAny {
    /// Converts a `Mapping` (with `String` keys) into a `MappingAny`.
    fn from(map: Mapping) -> Self {
        let mut any = MappingAny::with_capacity(map.len());
        for (k, v) in map {
            let _ = any.insert(Value::String(k), v);
        }
        any
    }
}

impl Hash for MappingAny {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for (k, v) in &self.0 {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl PartialOrd for MappingAny {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MappingAny {
    fn cmp(&self, other: &Self) -> Ordering {
        self.len().cmp(&other.len()).then_with(|| {
            for ((ak, av), (bk, bv)) in self.iter().zip(other.iter()) {
                match ak.cmp(bk) {
                    Ordering::Equal => {}
                    ord => return ord,
                }
                match av.cmp(bv) {
                    Ordering::Equal => continue,
                    ord => return ord,
                }
            }
            Ordering::Equal
        })
    }
}

impl fmt::Display for MappingAny {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, (k, v)) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{k}: {v}")?;
        }
        write!(f, "}}")
    }
}

impl Serialize for MappingAny {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for MappingAny {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};

        struct MappingAnyVisitor;

        impl<'de> Visitor<'de> for MappingAnyVisitor {
            type Value = MappingAny;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a YAML mapping")
            }

            fn visit_map<A>(self, mut map: A) -> Result<MappingAny, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut mapping = MappingAny::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((key, value)) = map.next_entry::<Value, Value>()? {
                    let _ = mapping.insert(key, value);
                }
                Ok(mapping)
            }
        }

        deserializer.deserialize_map(MappingAnyVisitor)
    }
}
