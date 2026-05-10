//! YAML value types.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::{Index, IndexMut};
use core::str::FromStr;

use indexmap::map::{IntoIter, Iter, IterMut, Keys, Values, ValuesMut};
use indexmap::IndexMap;
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

/// A YAML sequence (array/list).
pub type Sequence = Vec<Value>;

/// Represents a YAML number.
#[derive(Debug, Clone, Copy)]
pub enum Number {
    /// A signed integer.
    Integer(i64),
    /// A floating-point number.
    Float(f64),
}

impl Number {
    /// Returns the number as an `i64` if it is an integer.
    ///
    /// Floats return `None` even when their value happens to be a
    /// whole number; the type tag is part of the test.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert_eq!(Number::Integer(42).as_i64(), Some(42));
    /// assert_eq!(Number::Float(1.0).as_i64(), None);
    /// ```
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Number::Integer(n) => Some(*n),
            Number::Float(_) => None,
        }
    }

    /// Returns the number as a `u64` if it is a non-negative integer.
    ///
    /// Negative integers and floats return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert_eq!(Number::Integer(42).as_u64(), Some(42));
    /// assert_eq!(Number::Integer(-1).as_u64(), None);
    /// assert_eq!(Number::Float(1.0).as_u64(), None);
    /// ```
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Number::Integer(n) if *n >= 0 => Some(*n as u64),
            _ => None,
        }
    }

    /// Returns the number as an `f64`.
    ///
    /// Always succeeds — integers are widened to `f64` (with the
    /// usual `i64 → f64` precision loss for magnitudes above
    /// 2^53), floats pass through unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert_eq!(Number::Integer(42).as_f64(), 42.0);
    /// assert_eq!(Number::Float(0.5).as_f64(), 0.5);
    /// ```
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        match self {
            Number::Integer(n) => *n as f64,
            Number::Float(n) => *n,
        }
    }

    /// Returns `true` if the number is an integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Integer(42).is_integer());
    /// assert!(!Number::Float(1.0).is_integer());
    /// ```
    #[must_use]
    pub fn is_integer(&self) -> bool {
        matches!(self, Number::Integer(_))
    }

    /// Returns `true` if the number is a float.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Float(1.0).is_float());
    /// assert!(!Number::Integer(42).is_float());
    /// ```
    #[must_use]
    pub fn is_float(&self) -> bool {
        matches!(self, Number::Float(_))
    }

    /// Returns `true` if the number can be represented as an `i64`.
    ///
    /// True for all integer values, false for floats.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Integer(42).is_i64());
    /// assert!(!Number::Float(42.0).is_i64());
    /// ```
    #[must_use]
    pub fn is_i64(&self) -> bool {
        matches!(self, Number::Integer(_))
    }

    /// Returns `true` if the number can be represented as a `u64`.
    ///
    /// True for non-negative integers, false otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Integer(42).is_u64());
    /// assert!(!Number::Integer(-1).is_u64());
    /// assert!(!Number::Float(1.0).is_u64());
    /// ```
    #[must_use]
    pub fn is_u64(&self) -> bool {
        matches!(self, Number::Integer(n) if *n >= 0)
    }

    /// Returns `true` if the number can be represented as an `f64`.
    ///
    /// Always true — both integers and floats convert to `f64`
    /// (with the usual precision caveats for very large
    /// integers).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Integer(42).is_f64());
    /// assert!(Number::Float(1.0).is_f64());
    /// ```
    #[must_use]
    pub fn is_f64(&self) -> bool {
        true
    }

    /// Returns `true` if the number is `NaN` (Not a Number).
    ///
    /// Integers are never `NaN` — only floats with the IEEE 754
    /// NaN bit pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Float(f64::NAN).is_nan());
    /// assert!(!Number::Float(0.0).is_nan());
    /// assert!(!Number::Integer(0).is_nan());
    /// ```
    #[must_use]
    pub fn is_nan(&self) -> bool {
        match self {
            Number::Float(n) => n.is_nan(),
            Number::Integer(_) => false,
        }
    }

    /// Returns `true` if the number is positive or negative infinity.
    ///
    /// Integers are always finite — only `Number::Float(±inf)`
    /// returns true.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Float(f64::INFINITY).is_infinite());
    /// assert!(Number::Float(f64::NEG_INFINITY).is_infinite());
    /// assert!(!Number::Integer(i64::MAX).is_infinite());
    /// ```
    #[must_use]
    pub fn is_infinite(&self) -> bool {
        match self {
            Number::Float(n) => n.is_infinite(),
            Number::Integer(_) => false,
        }
    }

    /// Returns `true` if the number is neither infinite nor `NaN`.
    ///
    /// Integers are always finite; floats are finite when neither
    /// `±∞` nor `NaN`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Integer(0).is_finite());
    /// assert!(Number::Float(0.5).is_finite());
    /// assert!(!Number::Float(f64::NAN).is_finite());
    /// assert!(!Number::Float(f64::INFINITY).is_finite());
    /// ```
    #[must_use]
    pub fn is_finite(&self) -> bool {
        match self {
            Number::Float(n) => n.is_finite(),
            Number::Integer(_) => true,
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::Integer(n) => write!(f, "{n}"),
            Number::Float(n) => write!(f, "{n}"),
        }
    }
}

/// Error returned when parsing a number from a string fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseNumberError {
    _private: (),
}

impl fmt::Display for ParseNumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid number")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseNumberError {}

impl FromStr for Number {
    type Err = ParseNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Handle special float values
        match s {
            ".nan" | ".NaN" | ".NAN" => return Ok(Number::Float(f64::NAN)),
            ".inf" | ".Inf" | ".INF" => return Ok(Number::Float(f64::INFINITY)),
            "+.inf" | "+.Inf" | "+.INF" => return Ok(Number::Float(f64::INFINITY)),
            "-.inf" | "-.Inf" | "-.INF" => return Ok(Number::Float(f64::NEG_INFINITY)),
            _ => {}
        }

        // Try parsing as integer first
        if let Ok(n) = s.parse::<i64>() {
            return Ok(Number::Integer(n));
        }

        // Handle hex (0x), octal (0o), and binary (0b) integers
        if s.len() > 2 {
            let (prefix, rest) = s.split_at(2);
            match prefix {
                "0x" | "0X" => {
                    if let Ok(n) = i64::from_str_radix(rest, 16) {
                        return Ok(Number::Integer(n));
                    }
                }
                "0o" | "0O" => {
                    if let Ok(n) = i64::from_str_radix(rest, 8) {
                        return Ok(Number::Integer(n));
                    }
                }
                "0b" | "0B" => {
                    if let Ok(n) = i64::from_str_radix(rest, 2) {
                        return Ok(Number::Integer(n));
                    }
                }
                _ => {}
            }
        }

        // Try parsing as float
        if let Ok(n) = s.parse::<f64>() {
            return Ok(Number::Float(n));
        }

        Err(ParseNumberError { _private: () })
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::Integer(a), Number::Integer(b)) => a == b,
            (Number::Float(a), Number::Float(b)) => {
                // Treat NaN == NaN to satisfy the Eq contract (reflexivity)
                (a.is_nan() && b.is_nan()) || a == b
            }
            _ => false,
        }
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Number::Integer(n) => {
                0u8.hash(state);
                n.hash(state);
            }
            Number::Float(n) => {
                1u8.hash(state);
                // Eq/Hash contract: equal values must hash equal. Two
                // edge cases break naive `to_bits()` hashing:
                //   - `+0.0 == -0.0` is true under IEEE 754 (and our
                //     PartialEq), but `to_bits()` gives 0x0000… vs
                //     0x8000…. Normalise zeros to a single bit pattern.
                //   - PartialEq treats NaN == NaN as true (so `Eq` is
                //     reflexive), but distinct NaN payloads have
                //     distinct bits. Hash a fixed sentinel for NaN.
                let bits = if n.is_nan() {
                    0x7FF8_0000_0000_0001
                } else if *n == 0.0 {
                    0
                } else {
                    n.to_bits()
                };
                bits.hash(state);
            }
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Number::Integer(a), Number::Integer(b)) => a.cmp(b),
            (Number::Float(a), Number::Float(b)) => {
                // Handle NaN: treat all NaN as equal and greater than any non-NaN
                match (a.is_nan(), b.is_nan()) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    (false, false) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                }
            }
            (Number::Integer(a), Number::Float(b)) => {
                if b.is_nan() {
                    Ordering::Less
                } else if *a > (1_i64 << 53) || *a < -(1_i64 << 53) {
                    // Large integer outside f64 safe range — compare via string
                    // to avoid precision loss from i64→f64 cast.
                    let a_f = *a as f64;
                    if (a_f as i64) == *a {
                        a_f.partial_cmp(b).unwrap_or(Ordering::Equal)
                    } else {
                        // Precision lost — compare integer magnitude vs float
                        if *a > 0 {
                            if *b < (1_i64 << 53) as f64 {
                                Ordering::Greater
                            } else {
                                (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal)
                            }
                        } else if *b > -(1_i64 << 53) as f64 {
                            Ordering::Less
                        } else {
                            (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal)
                        }
                    }
                } else {
                    (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal)
                }
            }
            (Number::Float(a), Number::Integer(b)) => {
                // Delegate to the Integer-Float case and invert.
                match Number::Integer(*b).cmp(&Number::Float(*a)) {
                    Ordering::Less => Ordering::Greater,
                    Ordering::Greater => Ordering::Less,
                    Ordering::Equal => Ordering::Equal,
                }
            }
        }
    }
}

// ============================================================================
// Number From impls
// ============================================================================

impl From<i8> for Number {
    fn from(v: i8) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<i16> for Number {
    fn from(v: i16) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<i32> for Number {
    fn from(v: i32) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<i64> for Number {
    fn from(v: i64) -> Self {
        Number::Integer(v)
    }
}

impl From<isize> for Number {
    fn from(v: isize) -> Self {
        Number::Integer(v as i64)
    }
}

impl From<u8> for Number {
    fn from(v: u8) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<u16> for Number {
    fn from(v: u16) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<u32> for Number {
    fn from(v: u32) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<u64> for Number {
    fn from(v: u64) -> Self {
        if v <= i64::MAX as u64 {
            Number::Integer(v as i64)
        } else {
            Number::Float(v as f64)
        }
    }
}

impl From<usize> for Number {
    fn from(v: usize) -> Self {
        Number::from(v as u64)
    }
}

impl From<f32> for Number {
    fn from(v: f32) -> Self {
        Number::Float(f64::from(v))
    }
}

impl From<f64> for Number {
    fn from(v: f64) -> Self {
        Number::Float(v)
    }
}

// ============================================================================
// Tag utilities
// ============================================================================

/// Strips a leading `!` from a string, if present.
///
/// # Examples
///
/// ```rust
/// use noyalib::nobang;
///
/// assert_eq!(nobang("!foo"), "foo");
/// assert_eq!(nobang("foo"), "foo");
/// assert_eq!(nobang("!!int"), "!int");
/// ```
#[must_use]
pub fn nobang(s: &str) -> &str {
    s.strip_prefix('!').unwrap_or(s)
}

/// Result of checking whether a value looks like a YAML tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaybeTag<T> {
    /// The value is a tag (starts with `!`).
    Tag(String),
    /// The value is not a tag.
    NotTag(T),
}

/// Checks whether a value's display representation looks like a YAML tag.
///
/// A value is considered a tag if its string representation starts with `!`.
///
/// # Examples
///
/// ```rust
/// use noyalib::{check_for_tag, MaybeTag};
///
/// let result = check_for_tag(&"!mytag");
/// assert!(matches!(result, MaybeTag::Tag(_)));
///
/// let result = check_for_tag(&"plain");
/// assert!(matches!(result, MaybeTag::NotTag(_)));
/// ```
pub fn check_for_tag<T: fmt::Display>(value: &T) -> MaybeTag<String> {
    let s = value.to_string();
    if s.starts_with('!') {
        MaybeTag::Tag(s)
    } else {
        MaybeTag::NotTag(s)
    }
}

/// Magic key in the [`TagPreservingMapAccess`] map shape that
/// signals "the next entry is the tag string". Recognised by
/// `Value::deserialize`'s visitor on the tag-preserving path
/// driven by [`crate::de::Deserializer::preserve_tags`].
pub(crate) const TAGGED_VALUE_FIELD_TAG: &str = "$__noyalib_tag";

/// Magic key in the [`TagPreservingMapAccess`] map shape that
/// signals "the next entry is the inner [`Value`]".
pub(crate) const TAGGED_VALUE_FIELD_VALUE: &str = "$__noyalib_value";

/// A YAML tag.
///
/// Tags are used in YAML to denote the type of a value.
/// For example, `!custom_type value` has the tag `!custom_type`.
///
/// Tag comparison ignores a leading `!` prefix, so `Tag::new("!foo") ==
/// Tag::new("foo")`.
///
/// # Examples
///
/// ```rust
/// use noyalib::Tag;
///
/// let tag = Tag::new("!custom");
/// assert_eq!(tag.as_str(), "!custom");
/// assert_eq!(Tag::new("!foo"), Tag::new("foo"));
/// ```
#[derive(Debug, Clone)]
pub struct Tag(String);

impl Tag {
    /// Creates a new tag from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Tag;
    /// let t = Tag::new("!Custom");
    /// assert_eq!(t.as_str(), "!Custom");
    /// ```
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    /// Returns the tag as a string slice.
    ///
    /// The leading `!` (or `!!`) is included; use [`Tag::nobang`]
    /// for the unprefixed form.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Tag;
    /// assert_eq!(Tag::new("!Custom").as_str(), "!Custom");
    /// assert_eq!(Tag::new("!!str").as_str(), "!!str");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the tag and returns the inner string.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Tag;
    /// let s: String = Tag::new("!Custom").into_string();
    /// assert_eq!(s, "!Custom");
    /// ```
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Returns the tag string with a single leading `!` stripped.
    ///
    /// Strips at most one `!` — the YAML 1.2 *primary* tag
    /// handle. The secondary `!!` handle keeps one `!` after the
    /// strip (`!!str` → `!str`); use `Tag::as_str().trim_start_matches('!')`
    /// if you want every `!` removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Tag;
    /// assert_eq!(Tag::new("!Custom").nobang(), "Custom");
    /// assert_eq!(Tag::new("!!str").nobang(), "!str");
    /// assert_eq!(Tag::new("plain").nobang(), "plain");
    /// ```
    #[must_use]
    pub fn nobang(&self) -> &str {
        nobang(&self.0)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Tag {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Tag {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        nobang(&self.0) == nobang(&other.0)
    }
}

impl Eq for Tag {}

impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        nobang(&self.0).hash(state);
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tag {
    fn cmp(&self, other: &Self) -> Ordering {
        nobang(&self.0).cmp(nobang(&other.0))
    }
}

impl TryFrom<&[u8]> for Tag {
    type Error = core::str::Utf8Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        core::str::from_utf8(bytes).map(Tag::new)
    }
}

/// A tagged YAML value.
///
/// Represents a value with an explicit YAML tag, such as `!custom_type value`.
/// Tags are used to specify the type or interpretation of a value.
///
/// # Examples
///
/// ```rust
/// use noyalib::{Tag, TaggedValue, Value};
///
/// let tagged = TaggedValue::new(
///     Tag::new("!timestamp"),
///     Value::String("2024-01-01".to_string()),
/// );
/// assert_eq!(tagged.tag().as_str(), "!timestamp");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedValue {
    /// The tag.
    tag: Tag,
    /// The value.
    value: Box<Value>,
}

impl TaggedValue {
    /// Creates a new tagged value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Tag, TaggedValue, Value};
    /// let tv = TaggedValue::new(Tag::new("!Custom"), Value::from("hello"));
    /// assert_eq!(tv.tag().as_str(), "!Custom");
    /// assert_eq!(tv.value().as_str(), Some("hello"));
    /// ```
    #[must_use]
    pub fn new(tag: Tag, value: Value) -> Self {
        Self {
            tag,
            value: Box::new(value),
        }
    }

    /// Returns a reference to the tag.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Tag, TaggedValue, Value};
    /// let tv = TaggedValue::new(Tag::new("!Color"), Value::from("#ff8800"));
    /// assert_eq!(tv.tag().as_str(), "!Color");
    /// ```
    #[must_use]
    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    /// Returns a reference to the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Tag, TaggedValue, Value};
    /// let tv = TaggedValue::new(Tag::new("!Color"), Value::from("#ff8800"));
    /// assert_eq!(tv.value().as_str(), Some("#ff8800"));
    /// ```
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Returns a mutable reference to the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Tag, TaggedValue, Value};
    /// let mut tv = TaggedValue::new(Tag::new("!Color"), Value::from("#000"));
    /// *tv.value_mut() = Value::from("#ff8800");
    /// assert_eq!(tv.value().as_str(), Some("#ff8800"));
    /// ```
    #[must_use]
    pub fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    /// Consumes the tagged value and returns the tag and value
    /// as separate owned components.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Tag, TaggedValue, Value};
    /// let tv = TaggedValue::new(Tag::new("!Custom"), Value::from(42_i64));
    /// let (tag, value) = tv.into_parts();
    /// assert_eq!(tag.as_str(), "!Custom");
    /// assert_eq!(value.as_i64(), Some(42));
    /// ```
    #[must_use]
    pub fn into_parts(self) -> (Tag, Value) {
        (self.tag, *self.value)
    }
}

impl fmt::Display for TaggedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.tag, self.value)
    }
}

impl Serialize for TaggedValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(self.tag.as_str(), self.value())?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for TaggedValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};

        struct TaggedValueVisitor;

        impl<'de> Visitor<'de> for TaggedValueVisitor {
            type Value = TaggedValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a single-entry map representing a tagged value")
            }

            fn visit_map<A>(self, mut map: A) -> Result<TaggedValue, A::Error>
            where
                A: MapAccess<'de>,
            {
                let (tag, value): (String, Value) = map
                    .next_entry()?
                    .ok_or_else(|| serde::de::Error::custom("expected a single-entry map"))?;
                Ok(TaggedValue::new(Tag::new(tag), value))
            }
        }

        deserializer.deserialize_map(TaggedValueVisitor)
    }
}

impl<'de> serde::Deserializer<'de> for &'de TaggedValue {
    type Error = crate::Error;

    fn deserialize_any<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(TaggedValueMapAccess {
            tag: Some(self.tag.as_str()),
            value: Some(self.value()),
        })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_enum(TaggedValueEnumAccess { tagged: self })
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

struct TaggedValueMapAccess<'de> {
    tag: Option<&'de str>,
    value: Option<&'de Value>,
}

impl<'de> serde::de::MapAccess<'de> for TaggedValueMapAccess<'de> {
    type Error = crate::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> crate::Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.tag.take() {
            Some(tag) => seed
                .deserialize(serde::de::value::BorrowedStrDeserializer::new(tag))
                .map(Some),
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> crate::Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }
}

/// MapAccess emitted on the [`TAGGED_VALUE_TYPE_NAME`] code path.
///
/// Surfaces a tagged scalar as a two-entry map with magic keys
/// (`$__noyalib_tag` → tag string; `$__noyalib_value` → inner
/// `Value`) so [`Value`]'s own visitor can pattern-match the
/// shape and reconstruct `Value::Tagged(...)` on the
/// data-binding return path. Distinct from the existing
/// [`TaggedValueMapAccess`] (which uses the *real* tag as the
/// map key for typed-enum deserialise) to avoid colliding with
/// user data that legitimately has a key of the same name.
pub(crate) struct TagPreservingMapAccess<'de> {
    state: TagPreservingState<'de>,
}

#[derive(Clone, Copy)]
enum TagPreservingState<'de> {
    EmitTagKey { tag: &'de str, value: &'de Value },
    EmitTagValue { tag: &'de str, value: &'de Value },
    EmitValueKey { value: &'de Value },
    EmitValueValue { value: &'de Value },
    Done,
}

impl<'de> TagPreservingMapAccess<'de> {
    pub(crate) fn new(tag: &'de str, value: &'de Value) -> Self {
        Self {
            state: TagPreservingState::EmitTagKey { tag, value },
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for TagPreservingMapAccess<'de> {
    type Error = crate::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> crate::Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.state {
            TagPreservingState::EmitTagKey { tag, value } => {
                self.state = TagPreservingState::EmitTagValue { tag, value };
                seed.deserialize(
                    serde::de::value::BorrowedStrDeserializer::<crate::Error>::new(
                        TAGGED_VALUE_FIELD_TAG,
                    ),
                )
                .map(Some)
            }
            TagPreservingState::EmitValueKey { value } => {
                self.state = TagPreservingState::EmitValueValue { value };
                seed.deserialize(
                    serde::de::value::BorrowedStrDeserializer::<crate::Error>::new(
                        TAGGED_VALUE_FIELD_VALUE,
                    ),
                )
                .map(Some)
            }
            TagPreservingState::Done => Ok(None),
            // Calling next_key without consuming the previous value
            // is a serde misuse — surface as a custom error rather
            // than panicking.
            TagPreservingState::EmitTagValue { .. } | TagPreservingState::EmitValueValue { .. } => {
                Err(serde::de::Error::custom(
                    "TagPreservingMapAccess: next_key called before next_value",
                ))
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> crate::Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        match self.state {
            TagPreservingState::EmitTagValue { tag, value } => {
                self.state = TagPreservingState::EmitValueKey { value };
                seed.deserialize(
                    serde::de::value::BorrowedStrDeserializer::<crate::Error>::new(tag),
                )
            }
            TagPreservingState::EmitValueValue { value } => {
                self.state = TagPreservingState::Done;
                // Route through the preserve-tags-aware Deserializer
                // so any nested `Value::Tagged` inside `value` also
                // survives the round-trip — without this wrapping,
                // a tagged scalar inside a tagged collection would
                // collapse to the single-key `Mapping{"!tag": …}`
                // shape that the standard `&'de Value` Deserializer
                // produces for `Value::Tagged` (BUG: noyalib v0.0.1
                // C4HZ regression — global tags inside a tagged
                // sequence).
                seed.deserialize(crate::de::Deserializer::with_options_preserving_tags(
                    value, None, false,
                ))
            }
            _ => Err(serde::de::Error::custom(
                "TagPreservingMapAccess: next_value called out of order",
            )),
        }
    }
}

struct TaggedValueEnumAccess<'de> {
    tagged: &'de TaggedValue,
}

impl<'de> serde::de::EnumAccess<'de> for TaggedValueEnumAccess<'de> {
    type Error = crate::Error;
    type Variant = TaggedValueVariantAccess<'de>;

    fn variant_seed<V>(self, seed: V) -> crate::Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(
            serde::de::value::BorrowedStrDeserializer::<crate::Error>::new(
                self.tagged.tag.nobang(),
            ),
        )?;
        Ok((
            variant,
            TaggedValueVariantAccess {
                value: self.tagged.value(),
            },
        ))
    }
}

struct TaggedValueVariantAccess<'de> {
    value: &'de Value,
}

impl<'de> serde::de::VariantAccess<'de> for TaggedValueVariantAccess<'de> {
    type Error = crate::Error;

    fn unit_variant(self) -> crate::Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> crate::Result<T::Value>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.value)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_seq(self.value, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_map(self.value, visitor)
    }
}

/// Represents any valid YAML value.
#[derive(Debug, Clone, Default)]
pub enum Value {
    /// Represents a YAML null value.
    #[default]
    Null,
    /// Represents a YAML boolean.
    Bool(bool),
    /// Represents a YAML number (integer or float).
    Number(Number),
    /// Represents a YAML string.
    String(String),
    /// Represents a YAML sequence (array).
    Sequence(Sequence),
    /// Represents a YAML mapping (object).
    Mapping(Mapping),
    /// Represents a tagged YAML value.
    Tagged(Box<TaggedValue>),
}

impl Value {
    /// Returns `true` if the value is `Value::Null`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert!(Value::Null.is_null());
    /// assert!(!Value::from(false).is_null());
    /// ```
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns `true` if the value is a boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert!(Value::from(true).is_bool());
    /// assert!(!Value::from(1_i64).is_bool());
    /// ```
    #[must_use]
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Returns `true` if the value is a number (integer or float).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert!(Value::from(42_i64).is_number());
    /// assert!(Value::from(1.5).is_number());
    /// assert!(!Value::from("42").is_number());
    /// ```
    #[must_use]
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// Returns `true` if the value is a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert!(Value::from("hello").is_string());
    /// assert!(!Value::from(42_i64).is_string());
    /// ```
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Returns `true` if the value is a sequence (YAML list).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let v: Value = from_str("[1, 2, 3]").unwrap();
    /// assert!(v.is_sequence());
    /// ```
    #[must_use]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Value::Sequence(_))
    }

    /// Returns `true` if the value is a mapping (YAML map).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let v: Value = from_str("a: 1\nb: 2\n").unwrap();
    /// assert!(v.is_mapping());
    /// ```
    #[must_use]
    pub fn is_mapping(&self) -> bool {
        matches!(self, Value::Mapping(_))
    }

    /// Returns `true` if the value is tagged (custom YAML tag).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let v: Value = from_str("!Custom 'hello'\n").unwrap();
    /// assert!(v.is_tagged());
    /// ```
    #[must_use]
    pub fn is_tagged(&self) -> bool {
        matches!(self, Value::Tagged(_))
    }

    /// Returns the value as a boolean if it is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert_eq!(Value::from(true).as_bool(), Some(true));
    /// assert_eq!(Value::from("true").as_bool(), None);
    /// ```
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns `Some(())` if the value is null, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert_eq!(Value::Null.as_null(), Some(()));
    /// assert_eq!(Value::from(0_i64).as_null(), None);
    /// ```
    #[must_use]
    pub fn as_null(&self) -> Option<()> {
        match self {
            Value::Null => Some(()),
            _ => None,
        }
    }

    /// Returns the value as an `i64` if it is an integer.
    ///
    /// Floats return `None` even when the underlying value is a
    /// whole number; the type tag is part of the test.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert_eq!(Value::from(42_i64).as_i64(), Some(42));
    /// assert_eq!(Value::from(1.5).as_i64(), None);
    /// ```
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// Returns the value as a `u64` if it is a non-negative integer.
    ///
    /// Negative integers return `None`. Floats also return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert_eq!(Value::from(42_i64).as_u64(), Some(42));
    /// assert_eq!(Value::from(-1_i64).as_u64(), None);
    /// ```
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(n) => n.as_u64(),
            _ => None,
        }
    }

    /// Returns the value as an `f64` if it is a number.
    ///
    /// Integers are widened to `f64` (with the usual `i64 → f64`
    /// precision loss for magnitudes above 2^53).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert_eq!(Value::from(42_i64).as_f64(), Some(42.0));
    /// assert_eq!(Value::from(1.5).as_f64(), Some(1.5));
    /// assert_eq!(Value::from("42").as_f64(), None);
    /// ```
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(n.as_f64()),
            _ => None,
        }
    }

    /// Returns `true` if the value is an integer that fits in `i64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert!(Value::from(42_i64).is_i64());
    /// assert!(!Value::from(1.5).is_i64());
    /// ```
    #[must_use]
    pub fn is_i64(&self) -> bool {
        match self {
            Value::Number(n) => n.is_i64(),
            _ => false,
        }
    }

    /// Returns `true` if the value is a non-negative integer that fits in `u64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert!(Value::from(42_i64).is_u64());
    /// assert!(!Value::from(-1_i64).is_u64());
    /// ```
    #[must_use]
    pub fn is_u64(&self) -> bool {
        match self {
            Value::Number(n) => n.is_u64(),
            _ => false,
        }
    }

    /// Returns `true` if the value is a number (always convertible to `f64`).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert!(Value::from(42_i64).is_f64());
    /// assert!(Value::from(1.5).is_f64());
    /// ```
    #[must_use]
    pub fn is_f64(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// Returns the value as a string slice if it is a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Value;
    /// assert_eq!(Value::from("hello").as_str(), Some("hello"));
    /// assert_eq!(Value::from(42_i64).as_str(), None);
    /// ```
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as a sequence if it is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let v: Value = from_str("[1, 2, 3]").unwrap();
    /// assert_eq!(v.as_sequence().map(|s| s.len()), Some(3));
    /// ```
    #[must_use]
    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self {
            Value::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as a mutable sequence if it is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let mut v: Value = from_str("[1, 2]").unwrap();
    /// if let Some(seq) = v.as_sequence_mut() {
    ///     seq.push(Value::from(3_i64));
    /// }
    /// assert_eq!(v.as_sequence().unwrap().len(), 3);
    /// ```
    #[must_use]
    pub fn as_sequence_mut(&mut self) -> Option<&mut Sequence> {
        match self {
            Value::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as a mapping if it is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let v: Value = from_str("a: 1\nb: 2\n").unwrap();
    /// let m = v.as_mapping().unwrap();
    /// assert_eq!(m.get("a").and_then(Value::as_i64), Some(1));
    /// ```
    #[must_use]
    pub fn as_mapping(&self) -> Option<&Mapping> {
        match self {
            Value::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the value as a mutable mapping if it is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let mut v: Value = from_str("a: 1\n").unwrap();
    /// if let Some(m) = v.as_mapping_mut() {
    ///     m.insert("b", Value::from(2_i64));
    /// }
    /// assert_eq!(v.as_mapping().unwrap().len(), 2);
    /// ```
    #[must_use]
    pub fn as_mapping_mut(&mut self) -> Option<&mut Mapping> {
        match self {
            Value::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the value as a tagged value if it is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let v: Value = from_str("!Custom 'hi'\n").unwrap();
    /// let tv = v.as_tagged().unwrap();
    /// assert_eq!(tv.tag().as_str(), "!Custom");
    /// ```
    #[must_use]
    pub fn as_tagged(&self) -> Option<&TaggedValue> {
        match self {
            Value::Tagged(t) => Some(t),
            _ => None,
        }
    }

    /// Returns the value as a mutable tagged value if it is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Tag, Value};
    /// let mut v: Value = from_str("!A 'x'\n").unwrap();
    /// if let Some(tv) = v.as_tagged_mut() {
    ///     // mutate inner via value_mut
    ///     *tv.value_mut() = Value::from("y");
    /// }
    /// assert_eq!(v.as_tagged().unwrap().value().as_str(), Some("y"));
    /// # let _ = Tag::new("!A");
    /// ```
    #[must_use]
    pub fn as_tagged_mut(&mut self) -> Option<&mut TaggedValue> {
        match self {
            Value::Tagged(t) => Some(t),
            _ => None,
        }
    }

    /// Index into a sequence or mapping.
    ///
    /// Accepts a string key (for mappings) or a `usize` index
    /// (for sequences).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let v: Value = from_str("a: [1, 2, 3]\n").unwrap();
    /// assert_eq!(v.get("a").unwrap().get(0).and_then(Value::as_i64), Some(1));
    /// ```
    #[must_use]
    pub fn get<I: ValueIndex>(&self, index: I) -> Option<&Value> {
        index.index_into(self)
    }

    /// Mutably index into a sequence or mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let mut v: Value = from_str("a: 1\n").unwrap();
    /// if let Some(slot) = v.get_mut("a") {
    ///     *slot = Value::from(2_i64);
    /// }
    /// assert_eq!(v.get("a").and_then(Value::as_i64), Some(2));
    /// ```
    #[must_use]
    pub fn get_mut<I: ValueIndex>(&mut self, index: I) -> Option<&mut Value> {
        index.index_into_mut(self)
    }

    /// Access a nested value using a path string.
    ///
    /// Supports dot notation for mappings and bracket notation for sequences:
    /// - `"foo.bar"` - access key "bar" in mapping "foo"
    /// - `"items[0]"` - access index 0 in sequence "items"
    /// - `"items[0].name"` - access key "name" in first element of sequence
    ///   "items"
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = r#"
    /// server:
    ///   host: localhost
    ///   port: 8080
    /// items:
    ///   - name: first
    ///   - name: second
    /// "#;
    ///
    /// let value: Value = from_str(yaml).unwrap();
    ///
    /// assert_eq!(
    ///     value.get_path("server.host").unwrap().as_str(),
    ///     Some("localhost")
    /// );
    /// assert_eq!(value.get_path("server.port").unwrap().as_i64(), Some(8080));
    /// assert_eq!(
    ///     value.get_path("items[0].name").unwrap().as_str(),
    ///     Some("first")
    /// );
    /// assert_eq!(
    ///     value.get_path("items[1].name").unwrap().as_str(),
    ///     Some("second")
    /// );
    /// ```
    #[must_use]
    pub fn get_path(&self, path: &str) -> Option<&Value> {
        let segments = parse_path(path);
        let mut current = self;

        for segment in segments {
            current = match segment {
                QuerySegment::Key(key) => current.get(key.as_str())?,
                QuerySegment::Index(idx) => current.get(idx)?,
                QuerySegment::Wildcard | QuerySegment::RecursiveDescent => {
                    // For get_path, return the first match
                    return self.query(path).into_iter().next();
                }
            };
        }

        Some(current)
    }

    /// Query nested values using an extended path expression.
    ///
    /// Returns all matching values. Supports:
    /// - Dot notation: `"foo.bar.baz"`
    /// - Bracket indexing: `"items[0]"`
    /// - Wildcard: `"items[*]"` or `"items.*"` — matches all children
    /// - Recursive descent: `"..name"` — finds `name` at any depth
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = "items:\n  - name: a\n    v: 1\n  - name: b\n    v: 2\n";
    /// let value: Value = from_str(yaml).unwrap();
    ///
    /// // Wildcard: all items
    /// let all = value.query("items[*].name");
    /// assert_eq!(all.len(), 2);
    ///
    /// // Recursive descent: find "name" at any depth
    /// let names = value.query("..name");
    /// assert_eq!(names.len(), 2);
    /// ```
    #[must_use]
    pub fn query(&self, path: &str) -> Vec<&Value> {
        let segments = parse_path(path);
        let mut results = Vec::new();
        query_recursive(self, &segments, 0, &mut results);
        results
    }

    /// Mutably access a nested value using a path string.
    ///
    /// See [`get_path`](Self::get_path) for path syntax documentation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = "server:\n  port: 8080\n";
    /// let mut value: Value = from_str(yaml).unwrap();
    ///
    /// if let Some(port) = value.get_path_mut("server.port") {
    ///     *port = Value::from(9090);
    /// }
    ///
    /// assert_eq!(value.get_path("server.port").unwrap().as_i64(), Some(9090));
    /// ```
    #[must_use]
    pub fn get_path_mut(&mut self, path: &str) -> Option<&mut Value> {
        let segments = parse_path(path);
        let mut current = self;

        for segment in segments {
            current = match segment {
                QuerySegment::Key(key) => current.get_mut(key.as_str())?,
                QuerySegment::Index(idx) => current.get_mut(idx)?,
                QuerySegment::Wildcard | QuerySegment::RecursiveDescent => return None,
            };
        }

        Some(current)
    }

    /// Deep merge another value into this one.
    ///
    /// Merge behavior:
    /// - Mappings: keys from `other` are merged recursively; `other` keys
    ///   override `self` keys
    /// - Sequences: `other` sequence replaces `self` sequence (use
    ///   `merge_concat` for concatenation)
    /// - Scalars: `other` value replaces `self` value
    /// - Null in `other`: replaces `self` value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let mut base: Value = from_str(
    ///     "
    /// server:
    ///   host: localhost
    ///   port: 8080
    /// ",
    /// )
    /// .unwrap();
    ///
    /// let override_val: Value = from_str(
    ///     "
    /// server:
    ///   port: 9090
    ///   ssl: true
    /// ",
    /// )
    /// .unwrap();
    ///
    /// base.merge(override_val);
    ///
    /// assert_eq!(
    ///     base.get_path("server.host").unwrap().as_str(),
    ///     Some("localhost")
    /// );
    /// assert_eq!(base.get_path("server.port").unwrap().as_i64(), Some(9090));
    /// assert_eq!(base.get_path("server.ssl").unwrap().as_bool(), Some(true));
    /// ```
    pub fn merge(&mut self, other: Value) {
        match (self, other) {
            (Value::Mapping(base), Value::Mapping(other)) => {
                for (key, other_value) in other {
                    match base.get_mut(&key) {
                        Some(base_value) => {
                            base_value.merge(other_value);
                        }
                        None => {
                            let _ = base.insert(key, other_value);
                        }
                    }
                }
            }
            (this, other) => {
                *this = other;
            }
        }
    }

    /// Deep merge with sequence concatenation.
    ///
    /// Similar to [`merge`](Self::merge), but sequences are concatenated
    /// instead of replaced.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let mut base: Value = from_str("items:\n  - a\n  - b\n").unwrap();
    /// let other: Value = from_str("items:\n  - c\n  - d\n").unwrap();
    ///
    /// base.merge_concat(other);
    ///
    /// let items = base.get("items").unwrap().as_sequence().unwrap();
    /// assert_eq!(items.len(), 4);
    /// ```
    pub fn merge_concat(&mut self, other: Value) {
        match (self, other) {
            (Value::Mapping(base), Value::Mapping(other)) => {
                for (key, other_value) in other {
                    match base.get_mut(&key) {
                        Some(base_value) => {
                            base_value.merge_concat(other_value);
                        }
                        None => {
                            let _ = base.insert(key, other_value);
                        }
                    }
                }
            }
            (Value::Sequence(base), Value::Sequence(other)) => {
                base.extend(other);
            }
            (this, other) => {
                *this = other;
            }
        }
    }

    /// Remove a key from a mapping.
    ///
    /// Returns the removed value if the key existed and this is a mapping.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let mut value: Value = from_str("a: 1\nb: 2\n").unwrap();
    /// let removed = value.remove("a");
    ///
    /// assert_eq!(removed.unwrap().as_i64(), Some(1));
    /// assert!(value.get("a").is_none());
    /// ```
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        match self {
            Value::Mapping(map) => map.shift_remove(key),
            _ => None,
        }
    }

    /// Insert a key-value pair into a mapping.
    ///
    /// Returns the previous value if the key existed. Returns `None` if this is
    /// not a mapping.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let mut value: Value = from_str("a: 1\n").unwrap();
    /// value.insert("b", Value::from(2));
    ///
    /// assert_eq!(value.get("b").unwrap().as_i64(), Some(2));
    /// ```
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        match self {
            Value::Mapping(map) => map.insert(key.into(), value),
            _ => None,
        }
    }

    /// Performs merging of `<<` keys into the surrounding mapping.
    ///
    /// This implements YAML's merge key functionality as described in
    /// <https://yaml.org/type/merge.html>.
    ///
    /// The merge key `<<` is used to indicate that all the keys of one or more
    /// specified mappings should be inserted into the current mapping. If a key
    /// already exists in the current mapping, its value is NOT overridden.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let config = r#"
    /// defaults: &defaults
    ///   timeout: 30
    ///   retries: 3
    ///
    /// server:
    ///   <<: *defaults
    ///   host: localhost
    ///   timeout: 60
    /// "#;
    ///
    /// let mut value: Value = from_str(config).unwrap();
    /// value.apply_merge().unwrap();
    ///
    /// // The server mapping now has merged values from defaults
    /// assert_eq!(value["server"]["host"].as_str(), Some("localhost"));
    /// assert_eq!(value["server"]["timeout"].as_i64(), Some(60)); // Not overridden
    /// assert_eq!(value["server"]["retries"].as_i64(), Some(3));  // Merged from defaults
    /// ```
    ///
    /// # Multiple Merge Sources
    ///
    /// When `<<` is followed by a sequence of mappings, they are merged in
    /// order. Earlier mappings in the sequence take precedence for
    /// duplicate keys.
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = r#"
    /// a: &a
    ///   x: 1
    /// b: &b
    ///   x: 2
    ///   y: 2
    /// merged:
    ///   <<: [*a, *b]
    ///   z: 3
    /// "#;
    ///
    /// let mut value: Value = from_str(yaml).unwrap();
    /// value.apply_merge().unwrap();
    ///
    /// assert_eq!(value["merged"]["x"].as_i64(), Some(1)); // From *a (first)
    /// assert_eq!(value["merged"]["y"].as_i64(), Some(2)); // From *b
    /// assert_eq!(value["merged"]["z"].as_i64(), Some(3)); // Direct value
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A merge key value is a scalar (not a mapping or sequence of mappings)
    /// - A merge key value is a tagged value
    /// - A sequence in a merge key contains non-mapping values
    pub fn apply_merge(&mut self) -> crate::Result<()> {
        match self {
            Value::Mapping(mapping) => {
                // First, recursively apply merge to all values
                for value in mapping.values_mut() {
                    value.apply_merge()?;
                }

                // Then process the << key if present
                let merge_value = mapping.remove("<<");
                let merge_sequence = match merge_value {
                    Some(Value::Sequence(seq)) => seq,
                    Some(value) => vec![value],
                    None => vec![],
                };

                // Process each merge source
                for value in merge_sequence {
                    match value {
                        Value::Mapping(merge_map) => {
                            // Merge keys from source, but don't override existing keys
                            for (k, v) in merge_map {
                                let _ = mapping.entry(k).or_insert(v);
                            }
                        }
                        Value::Sequence(_) => {
                            return Err(crate::Error::SequenceInMergeElement);
                        }
                        Value::Tagged(_) => {
                            return Err(crate::Error::TaggedInMerge);
                        }
                        _ => {
                            return Err(crate::Error::ScalarInMergeElement);
                        }
                    }
                }
            }
            Value::Sequence(seq) => {
                // Recursively apply merge to sequence elements
                for value in seq {
                    value.apply_merge()?;
                }
            }
            Value::Tagged(tagged) => {
                // Recursively apply merge to tagged value
                tagged.value_mut().apply_merge()?;
            }
            // Scalars don't need merge processing
            _ => {}
        }

        Ok(())
    }

    /// Substitute every `${name}` reference inside string scalars
    /// with the corresponding entry from `properties`. The walk is
    /// recursive — strings nested inside sequences, mappings, and
    /// tagged values are all visited.
    ///
    /// String keys in mappings are treated as opaque and never
    /// interpolated; only string *values* are touched. This avoids
    /// surprising key-rename interactions and keeps the schema
    /// stable.
    ///
    /// `${{` and `}}` escape sequences let users include literal
    /// `${` and `}` in a scalar that should not be interpreted as
    /// an interpolation site.
    ///
    /// # Errors
    ///
    /// Returns `Error::Custom` with the offending placeholder name
    /// when a `${name}` reference is not present in `properties`.
    /// Use [`Value::interpolate_properties_lossy`] to substitute an
    /// empty string for unknown placeholders without erroring.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// use std::collections::HashMap;
    ///
    /// let mut value: Value = from_str("\
    /// service:
    ///   name: ${APP_NAME}
    ///   port: ${BIND_PORT}
    /// ").unwrap();
    ///
    /// let mut props = HashMap::new();
    /// props.insert("APP_NAME".to_string(), "noyalib".to_string());
    /// props.insert("BIND_PORT".to_string(), "8080".to_string());
    ///
    /// value.interpolate_properties(&props).unwrap();
    /// assert_eq!(value["service"]["name"].as_str(), Some("noyalib"));
    /// // The numeric value stays as a string — re-deserialize the
    /// // tree if you need typed coercion.
    /// assert_eq!(value["service"]["port"].as_str(), Some("8080"));
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn interpolate_properties<S>(
        &mut self,
        properties: &std::collections::HashMap<String, S>,
    ) -> crate::Result<()>
    where
        S: AsRef<str>,
    {
        self.interpolate_inner(
            &|name| match properties.get(name) {
                Some(v) => ResolveOutcome::Found(v.as_ref().to_owned()),
                None => ResolveOutcome::Missing,
            },
            MissingAction::Error(false),
        )
    }

    /// Like [`Value::interpolate_properties`] but redacts the
    /// placeholder name from any error surfaced when an unknown
    /// `${name}` is encountered — useful when the placeholder
    /// name itself is sensitive (e.g. it carries an audit-trail
    /// secret identifier, or it's used in a context where logs
    /// are externally indexed).
    ///
    /// On success this method is identical to
    /// `interpolate_properties`. On failure the error reads
    /// `interpolate_properties: unknown placeholder
    /// ${"<redacted>"}` instead of including the original name.
    /// Substituted *values* (the contents of the property map)
    /// are never echoed to errors — that's the responsibility of
    /// the caller's downstream validators.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// use std::collections::HashMap;
    ///
    /// let mut value: Value = from_str("token: ${SECRET_TOKEN_NAME}").unwrap();
    /// // Empty property map — substitution fails. With the
    /// // redacting variant the placeholder name does not leak.
    /// let props: HashMap<String, String> = HashMap::new();
    /// let err = value.interpolate_properties_redacted(&props).unwrap_err();
    /// let msg = err.to_string();
    /// assert!(msg.contains("<redacted>"));
    /// assert!(!msg.contains("SECRET_TOKEN_NAME"));
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn interpolate_properties_redacted<S>(
        &mut self,
        properties: &std::collections::HashMap<String, S>,
    ) -> crate::Result<()>
    where
        S: AsRef<str>,
    {
        self.interpolate_inner(
            &|name| match properties.get(name) {
                Some(v) => ResolveOutcome::Found(v.as_ref().to_owned()),
                None => ResolveOutcome::Missing,
            },
            MissingAction::Error(true),
        )
    }

    /// Like [`Value::interpolate_properties`] but never errors —
    /// unknown placeholders are replaced with an empty string. The
    /// motivating use case is environment-variable expansion where
    /// missing variables should silently degrade rather than abort
    /// the load.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// use std::collections::HashMap;
    ///
    /// let mut value: Value = from_str("greeting: hello ${WHO}, hello ${MISSING}").unwrap();
    /// let mut props: HashMap<String, String> = HashMap::new();
    /// props.insert("WHO".into(), "world".into());
    ///
    /// value.interpolate_properties_lossy(&props);
    /// assert_eq!(value["greeting"].as_str(), Some("hello world, hello "));
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn interpolate_properties_lossy<S>(
        &mut self,
        properties: &std::collections::HashMap<String, S>,
    ) where
        S: AsRef<str>,
    {
        // Missing entries fall back to the empty string (and
        // honour any `${name:-default}` syntax along the way).
        // The walk never errors so the outer call is total.
        let _ = self.interpolate_inner(
            &|name| match properties.get(name) {
                Some(v) => ResolveOutcome::Found(v.as_ref().to_owned()),
                None => ResolveOutcome::Missing,
            },
            MissingAction::Empty,
        );
    }

    /// Internal interpolation driver — `resolve` returns a
    /// [`ResolveOutcome`] tri-state so the walker can distinguish
    /// found / missing / errored. `missing_action` decides what
    /// happens when a placeholder is missing *and* has no
    /// `:-default` fallback.
    #[cfg(feature = "std")]
    pub(crate) fn interpolate_inner(
        &mut self,
        resolve: &dyn Fn(&str) -> ResolveOutcome,
        missing_action: MissingAction,
    ) -> crate::Result<()> {
        match self {
            Value::String(s) => {
                if let Some(updated) = expand_placeholders(s, resolve, missing_action)? {
                    *s = updated;
                }
            }
            Value::Sequence(seq) => {
                for v in seq {
                    v.interpolate_inner(resolve, missing_action)?;
                }
            }
            Value::Mapping(map) => {
                for v in map.values_mut() {
                    v.interpolate_inner(resolve, missing_action)?;
                }
            }
            Value::Tagged(tagged) => {
                tagged
                    .value_mut()
                    .interpolate_inner(resolve, missing_action)?;
            }
            // Null / Bool / Number have no string content; nothing to do.
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
        Ok(())
    }

    /// Recursively strips tags from this value, returning the untagged value.
    ///
    /// If the value is `Value::Tagged`, the inner value is returned
    /// (recursively untagged). Sequences and mappings have their elements
    /// recursively untagged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let v: Value = from_str("!Custom 'hi'\n").unwrap();
    /// assert!(v.is_tagged());
    /// let untagged = v.untag();
    /// assert_eq!(untagged.as_str(), Some("hi"));
    /// ```
    #[must_use]
    pub fn untag(self) -> Self {
        match self {
            Value::Tagged(tagged) => tagged.value.untag(),
            Value::Sequence(seq) => Value::Sequence(seq.into_iter().map(Value::untag).collect()),
            Value::Mapping(map) => {
                let untagged: Mapping = map.into_iter().map(|(k, v)| (k, v.untag())).collect();
                Value::Mapping(untagged)
            }
            other => other,
        }
    }

    /// Returns a reference to the innermost untagged value.
    ///
    /// If the value is `Value::Tagged`, returns a reference to the inner value
    /// (recursively following tags). Does not recurse into sequences or
    /// mappings.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let v: Value = from_str("!Custom 'hi'\n").unwrap();
    /// assert_eq!(v.untag_ref().as_str(), Some("hi"));
    /// ```
    #[must_use]
    pub fn untag_ref(&self) -> &Self {
        match self {
            Value::Tagged(tagged) => tagged.value.untag_ref(),
            other => other,
        }
    }

    /// Returns a mutable reference to the innermost untagged value.
    ///
    /// If the value is `Value::Tagged`, returns a mutable reference to the
    /// inner value (recursively following tags). Does not recurse into
    /// sequences or mappings.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str, Value};
    /// let mut v: Value = from_str("!Custom 'hi'\n").unwrap();
    /// *v.untag_mut() = Value::from("bye");
    /// assert_eq!(v.untag_ref().as_str(), Some("bye"));
    /// ```
    #[must_use]
    pub fn untag_mut(&mut self) -> &mut Self {
        match self {
            Value::Tagged(tagged) => tagged.value.untag_mut(),
            other => other,
        }
    }
}

// Use shared path parsing from the path module.
use crate::path::{parse_query_path, QuerySegment};

/// Backwards-compatible alias.
fn parse_path(path: &str) -> Vec<QuerySegment> {
    parse_query_path(path)
}

/// Recursively query a Value tree against path segments.
fn query_recursive<'a>(
    value: &'a Value,
    segments: &[QuerySegment],
    depth: usize,
    results: &mut Vec<&'a Value>,
) {
    if depth >= segments.len() {
        results.push(value);
        return;
    }

    match &segments[depth] {
        QuerySegment::Key(key) => {
            if let Some(child) = value.get(key.as_str()) {
                query_recursive(child, segments, depth + 1, results);
            }
        }
        QuerySegment::Index(idx) => {
            if let Some(child) = value.get(*idx) {
                query_recursive(child, segments, depth + 1, results);
            }
        }
        QuerySegment::Wildcard => match value {
            Value::Sequence(seq) => {
                for item in seq {
                    query_recursive(item, segments, depth + 1, results);
                }
            }
            Value::Mapping(map) => {
                for (_, v) in map.iter() {
                    query_recursive(v, segments, depth + 1, results);
                }
            }
            _ => {}
        },
        QuerySegment::RecursiveDescent => {
            // Match the remaining path at this level and all descendants
            let remaining = &segments[depth + 1..];
            if !remaining.is_empty() {
                // Try matching the rest of the path at this node
                query_recursive(value, segments, depth + 1, results);
                // Recurse into all children
                match value {
                    Value::Sequence(seq) => {
                        for item in seq {
                            query_recursive(item, segments, depth, results);
                        }
                    }
                    Value::Mapping(map) => {
                        for (_, v) in map.iter() {
                            query_recursive(v, segments, depth, results);
                        }
                    }
                    Value::Tagged(t) => {
                        query_recursive(t.value(), segments, depth, results);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Sequence(a), Value::Sequence(b)) => a == b,
            (Value::Mapping(a), Value::Mapping(b)) => a == b,
            (Value::Tagged(a), Value::Tagged(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Discriminant for variant type
        core::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Bool(b) => b.hash(state),
            Value::Number(n) => n.hash(state),
            Value::String(s) => s.hash(state),
            Value::Sequence(seq) => {
                seq.len().hash(state);
                for v in seq {
                    v.hash(state);
                }
            }
            Value::Mapping(map) => {
                map.len().hash(state);
                for (k, v) in map {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Value::Tagged(tagged) => {
                tagged.tag().hash(state);
                tagged.value().hash(state);
            }
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        // Order: Null < Bool < Number < String < Sequence < Mapping < Tagged
        fn type_order(v: &Value) -> u8 {
            match v {
                Value::Null => 0,
                Value::Bool(_) => 1,
                Value::Number(_) => 2,
                Value::String(_) => 3,
                Value::Sequence(_) => 4,
                Value::Mapping(_) => 5,
                Value::Tagged(_) => 6,
            }
        }

        match type_order(self).cmp(&type_order(other)) {
            Ordering::Equal => {}
            ord => return ord,
        }

        match (self, other) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Number(a), Value::Number(b)) => a.cmp(b),
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Sequence(a), Value::Sequence(b)) => a.len().cmp(&b.len()).then_with(|| {
                for (av, bv) in a.iter().zip(b.iter()) {
                    match av.cmp(bv) {
                        Ordering::Equal => continue,
                        ord => return ord,
                    }
                }
                Ordering::Equal
            }),
            (Value::Mapping(a), Value::Mapping(b)) => a.len().cmp(&b.len()).then_with(|| {
                for ((ak, av), (bk, bv)) in a.iter().zip(b.iter()) {
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
            }),
            (Value::Tagged(a), Value::Tagged(b)) => a
                .tag()
                .as_str()
                .cmp(b.tag().as_str())
                .then_with(|| a.value().cmp(b.value())),
            _ => unreachable!("type_order check ensures same variants"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Sequence(s) => {
                write!(f, "[")?;
                for (i, v) in s.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Mapping(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            Value::Tagged(t) => write!(f, "{t}"),
        }
    }
}

/// A type that can be used to index into a `Value`.
///
/// This trait provides methods for accessing elements within a [`Value`] by
/// index. It is implemented for:
/// - `usize` - for indexing into sequences
/// - `&str` - for indexing into mappings by string key
/// - `String` - for indexing into mappings by owned string
/// - `&String` - for indexing into mappings by string reference
///
/// # Examples
///
/// ```rust
/// use noyalib::{from_str, Value, ValueIndex};
///
/// let yaml = r#"
/// items:
///   - name: first
///   - name: second
/// config:
///   host: localhost
/// "#;
///
/// let value: Value = from_str(yaml).unwrap();
///
/// // Using usize to index into sequences
/// assert_eq!(
///     value
///         .get("items")
///         .unwrap()
///         .get(0)
///         .unwrap()
///         .get("name")
///         .unwrap()
///         .as_str(),
///     Some("first")
/// );
///
/// // Using &str to index into mappings
/// assert_eq!(
///     value.get("config").unwrap().get("host").unwrap().as_str(),
///     Some("localhost")
/// );
/// ```
pub trait ValueIndex {
    /// Index into a value, returning a reference to the element if found.
    ///
    /// Returns `None` if:
    /// - The value is not the appropriate type for this index (e.g., indexing a
    ///   mapping with `usize`)
    /// - The index/key doesn't exist
    fn index_into(self, value: &Value) -> Option<&Value>;

    /// Mutably index into a value, returning a mutable reference to the element
    /// if found.
    ///
    /// Returns `None` if:
    /// - The value is not the appropriate type for this index
    /// - The index/key doesn't exist
    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value>;

    /// Index into a value, inserting a default value if the key doesn't exist.
    ///
    /// This method is useful for building nested structures or ensuring a key
    /// exists.
    ///
    /// # Behavior
    ///
    /// - For sequences: panics if the index is out of bounds
    /// - For mappings: creates a null entry if the key doesn't exist
    /// - For null values: converts to an empty mapping (for string keys only)
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The value is not the appropriate type for this index
    /// - Indexing a sequence with an out-of-bounds index
    fn index_or_insert(self, value: &mut Value) -> &mut Value;
}

impl ValueIndex for usize {
    fn index_into(self, value: &Value) -> Option<&Value> {
        match value {
            Value::Sequence(s) => s.get(self),
            Value::Tagged(t) => self.index_into(t.value()),
            _ => None,
        }
    }

    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value> {
        match value {
            Value::Sequence(s) => s.get_mut(self),
            Value::Tagged(t) => self.index_into_mut(t.value_mut()),
            _ => None,
        }
    }

    #[track_caller]
    fn index_or_insert(self, value: &mut Value) -> &mut Value {
        match value {
            Value::Sequence(s) => {
                let len = s.len();
                s.get_mut(self).unwrap_or_else(|| {
                    panic!(
                        "cannot access index {} of YAML sequence of length {}",
                        self, len
                    )
                })
            }
            Value::Tagged(t) => self.index_or_insert(t.value_mut()),
            _ => panic!(
                "cannot access index {} of YAML {}",
                self,
                value_type_name(value)
            ),
        }
    }
}

impl ValueIndex for &str {
    fn index_into(self, value: &Value) -> Option<&Value> {
        match value {
            Value::Mapping(m) => m.get(self),
            Value::Tagged(t) => self.index_into(t.value()),
            _ => None,
        }
    }

    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value> {
        match value {
            Value::Mapping(m) => m.get_mut(self),
            Value::Tagged(t) => self.index_into_mut(t.value_mut()),
            _ => None,
        }
    }

    #[track_caller]
    fn index_or_insert(self, value: &mut Value) -> &mut Value {
        // If the value is null, convert it to an empty mapping
        if let Value::Null = value {
            *value = Value::Mapping(Mapping::new());
        }

        match value {
            Value::Mapping(m) => {
                let _ = m.entry(self.to_owned()).or_insert(Value::Null);
                m.get_mut(self).unwrap()
            }
            Value::Tagged(t) => self.index_or_insert(t.value_mut()),
            _ => panic!(
                "cannot access key {:?} in YAML {}",
                self,
                value_type_name(value)
            ),
        }
    }
}

impl ValueIndex for String {
    fn index_into(self, value: &Value) -> Option<&Value> {
        self.as_str().index_into(value)
    }

    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value> {
        self.as_str().index_into_mut(value)
    }

    #[track_caller]
    fn index_or_insert(self, value: &mut Value) -> &mut Value {
        self.as_str().index_or_insert(value)
    }
}

impl ValueIndex for &String {
    fn index_into(self, value: &Value) -> Option<&Value> {
        self.as_str().index_into(value)
    }

    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value> {
        self.as_str().index_into_mut(value)
    }

    #[track_caller]
    fn index_or_insert(self, value: &mut Value) -> &mut Value {
        self.as_str().index_or_insert(value)
    }
}

impl ValueIndex for &Value {
    fn index_into(self, value: &Value) -> Option<&Value> {
        match self {
            Value::String(s) => s.as_str().index_into(value),
            Value::Number(Number::Integer(n)) if *n >= 0 => {
                usize::try_from(*n).ok()?.index_into(value)
            }
            _ => None,
        }
    }

    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value> {
        match self {
            Value::String(s) => s.as_str().index_into_mut(value),
            Value::Number(Number::Integer(n)) if *n >= 0 => {
                usize::try_from(*n).ok()?.index_into_mut(value)
            }
            _ => None,
        }
    }

    #[track_caller]
    fn index_or_insert(self, value: &mut Value) -> &mut Value {
        match self {
            Value::String(s) => s.as_str().index_or_insert(value),
            Value::Number(Number::Integer(n)) if *n >= 0 => {
                let idx =
                    usize::try_from(*n).unwrap_or_else(|_| panic!("index {} overflows usize", n));
                idx.index_or_insert(value)
            }
            _ => panic!("cannot index with {:?}", self),
        }
    }
}

/// Expand `${name}` placeholders in `s` using the supplied
/// resolver. Returns:
///
/// - `Ok(None)` when no placeholders are present (caller can avoid
///   allocating a fresh `String`),
/// - `Ok(Some(expanded))` when at least one placeholder was found,
/// - `Err(_)` when the resolver returned an error for a
///   placeholder.
///
/// Escape sequences:
/// - `${{` produces a literal `${` (placeholder NOT recognised),
/// - `}}` produces a literal `}`.
///
/// Placeholder names match `[A-Za-z_][A-Za-z0-9_.]*` — letters,
/// digits, underscore, dot. The dot allows hierarchical names like
/// `${db.host}` for users who want to namespace their property
/// maps. Anything that does not match is a parse error.
#[cfg(feature = "std")]
/// Outcome of resolving a placeholder name.
///
/// Three-way return so [`expand_placeholders`] can distinguish a
/// genuinely missing key (which may then defer to a `:-default`
/// fallback) from a key whose own resolution errored.
pub(crate) enum ResolveOutcome {
    /// Name resolved cleanly.
    Found(String),
    /// Name not in the resolver's lookup table.
    Missing,
    /// Resolution failed for a non-missing reason. Reserved for
    /// future resolver impls that may surface I/O or permissions
    /// errors (e.g. environment-variable-backed resolvers); not
    /// produced by the in-memory `properties` map path today.
    #[allow(dead_code)]
    Error(crate::Error),
}

/// What to do when a placeholder has no map entry and no
/// `:-default` fallback.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MissingAction {
    /// Substitute the empty string. Lossy/non-strict mode.
    Empty,
    /// Surface an error. Boolean is `true` to redact the
    /// placeholder name from the error message.
    Error(bool),
}

fn expand_placeholders(
    s: &str,
    resolve: &dyn Fn(&str) -> ResolveOutcome,
    missing_action: MissingAction,
) -> crate::Result<Option<String>> {
    let bytes = s.as_bytes();
    // Fast path: no `$` and no `}` → no allocation, no walk.
    if !bytes.contains(&b'$') && !bytes.contains(&b'}') {
        return Ok(None);
    }
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let mut touched = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'$' {
            // Escape: `$$` → literal `$`.
            out.push('$');
            i += 2;
            touched = true;
            continue;
        }
        if b == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            // Escape: `${{` → literal `${`.
            if i + 2 < bytes.len() && bytes[i + 2] == b'{' {
                out.push_str("${");
                i += 3;
                touched = true;
                continue;
            }
            // Scan the placeholder name. Stop at the first `}` (close)
            // or `:` (default-value sentinel for `${name:-default}`).
            let name_start = i + 2;
            let mut j = name_start;
            while j < bytes.len() && bytes[j] != b'}' && bytes[j] != b':' {
                let c = bytes[j];
                let ok = c.is_ascii_alphanumeric() || c == b'_' || c == b'.';
                if !ok {
                    return Err(crate::Error::Custom(format!(
                        "interpolate_properties: invalid character {:?} in placeholder",
                        c as char
                    )));
                }
                j += 1;
            }
            if j >= bytes.len() {
                return Err(crate::Error::Custom(
                    "interpolate_properties: unterminated `${...}` placeholder".into(),
                ));
            }
            if name_start == j {
                return Err(crate::Error::Custom(
                    "interpolate_properties: empty placeholder `${}`".into(),
                ));
            }
            let name = &s[name_start..j];
            // Optional `:-default` fallback: `${name:-default}`.
            let mut default: Option<&str> = None;
            let mut close = j;
            if bytes[j] == b':' {
                if j + 1 >= bytes.len() || bytes[j + 1] != b'-' {
                    return Err(crate::Error::Custom(
                        "interpolate_properties: expected `:-default` after `${name:`".into(),
                    ));
                }
                let default_start = j + 2;
                let mut k = default_start;
                while k < bytes.len() && bytes[k] != b'}' {
                    k += 1;
                }
                if k >= bytes.len() {
                    return Err(crate::Error::Custom(
                        "interpolate_properties: unterminated `${name:-default}`".into(),
                    ));
                }
                default = Some(&s[default_start..k]);
                close = k;
            }
            let value = match resolve(name) {
                ResolveOutcome::Found(v) => v,
                ResolveOutcome::Missing => match default {
                    Some(d) => d.to_owned(),
                    None => match missing_action {
                        MissingAction::Empty => String::new(),
                        MissingAction::Error(redact) => {
                            return Err(crate::Error::Custom(if redact {
                                "interpolate_properties: unknown placeholder `${<redacted>}`".into()
                            } else {
                                format!("interpolate_properties: unknown placeholder `${{{name}}}`")
                            }));
                        }
                    },
                },
                ResolveOutcome::Error(e) => return Err(e),
            };
            out.push_str(&value);
            i = close + 1;
            touched = true;
            continue;
        }
        if b == b'}' && i + 1 < bytes.len() && bytes[i + 1] == b'}' {
            // Escape: `}}` → literal `}`.
            out.push('}');
            i += 2;
            touched = true;
            continue;
        }
        // Multi-byte UTF-8 — push the leading byte's char in one go.
        // SAFETY-equivalent: `s` is a valid &str so byte boundaries
        // align with char boundaries; we walk byte-by-byte but only
        // act on ASCII bytes that we know cannot be inside a
        // multi-byte sequence.
        let c = s[i..].chars().next().expect("char at boundary");
        out.push(c);
        i += c.len_utf8();
    }
    if touched {
        Ok(Some(out))
    } else {
        Ok(None)
    }
}

/// Returns the type name of a value for error messages.
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Sequence(_) => "sequence",
        Value::Mapping(_) => "mapping",
        Value::Tagged(_) => "tagged value",
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, SeqAccess, Visitor};

        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("any valid YAML value")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
                Ok(Value::Number(Number::Integer(v)))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
                Ok(Value::Number(Number::Integer(v as i64)))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
                Ok(Value::Number(Number::Float(v)))
            }

            fn visit_str<E>(self, v: &str) -> Result<Value, E> {
                Ok(Value::String(v.to_owned()))
            }

            fn visit_string<E>(self, v: String) -> Result<Value, E> {
                Ok(Value::String(v))
            }

            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                // Pre-size from the SeqAccess size_hint when
                // available — saves up to ~11 reallocations on a
                // 2 000-element sequence (Vec doubles on each
                // grow). Falls back to the default growth strategy
                // when the hint isn't reliable.
                let mut vec = match seq.size_hint() {
                    Some(n) if n > 0 && n < 1 << 20 => Vec::with_capacity(n),
                    _ => Vec::new(),
                };
                while let Some(elem) = seq.next_element()? {
                    vec.push(elem);
                }
                Ok(Value::Sequence(vec))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // Tag-preserving fast path: noyalib's own
                // [`crate::de::Deserializer::deserialize_any`]
                // routes `Value::Tagged(...)` through a
                // [`TagPreservingMapAccess`] when its
                // `preserve_tags` flag is on (set automatically
                // for `from_str::<Value>` / `from_value::<Value>`
                // — see [`crate::de::is_value_target`]). The first
                // key of that map is the [`TAGGED_VALUE_FIELD_TAG`]
                // sentinel; detect it and reconstruct
                // `Value::Tagged` so the tag survives the
                // data-binding return path.
                //
                // Other Deserializers (serde_json, FlatMap, …)
                // never see this magic shape, so this branch is
                // strictly additive.
                let first_key: Option<String> = map.next_key()?;
                if let Some(k) = first_key.as_deref() {
                    if k == TAGGED_VALUE_FIELD_TAG {
                        let tag_str: String = map.next_value()?;
                        let second_key: String = map.next_key()?.ok_or_else(|| {
                            <A::Error as serde::de::Error>::custom(
                                "tag-preserving map missing $__noyalib_value entry",
                            )
                        })?;
                        if second_key != TAGGED_VALUE_FIELD_VALUE {
                            return Err(<A::Error as serde::de::Error>::custom(format!(
                                "tag-preserving map: expected `{}`, got `{}`",
                                TAGGED_VALUE_FIELD_VALUE, second_key
                            )));
                        }
                        let inner: Value = map.next_value()?;
                        return Ok(Value::Tagged(Box::new(TaggedValue::new(
                            Tag::new(tag_str),
                            inner,
                        ))));
                    }
                }
                // Regular mapping path — collect every (k, v) pair
                // including the (k, v) we already consumed.
                // Pre-size when the MapAccess provides a usable
                // hint; saves ~10 IndexMap rehashes on large
                // mappings (capacity grows by ~doubling).
                let mut mapping = match map.size_hint() {
                    Some(n) if n > 0 && n < 1 << 20 => Mapping::with_capacity(n),
                    _ => Mapping::new(),
                };
                if let Some(k) = first_key {
                    let v: Value = map.next_value()?;
                    let _ = mapping.insert(k, v);
                }
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    let _ = mapping.insert(key, value);
                }
                Ok(Value::Mapping(mapping))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_none(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Number(Number::Integer(n)) => serializer.serialize_i64(*n),
            Value::Number(Number::Float(n)) => serializer.serialize_f64(*n),
            Value::String(s) => serializer.serialize_str(s),
            Value::Sequence(s) => s.serialize(serializer),
            Value::Mapping(m) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in m {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            Value::Tagged(tagged) => {
                // Serialize as a single-entry map with tag as key
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(tagged.tag().as_str(), tagged.value())?;
                map.end()
            }
        }
    }
}

// ============================================================================
// Deserializer implementation for &Value
// ============================================================================

impl<'de> serde::de::IntoDeserializer<'de, crate::Error> for &'de Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

struct ValueSeqAccess<'de> {
    iter: core::slice::Iter<'de, Value>,
}

impl<'de> serde::de::SeqAccess<'de> for ValueSeqAccess<'de> {
    type Error = crate::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> crate::Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }
}

struct ValueMapAccess<'de> {
    iter: Iter<'de, String, Value>,
    value: Option<&'de Value>,
}

impl<'de> serde::de::MapAccess<'de> for ValueMapAccess<'de> {
    type Error = crate::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> crate::Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(serde::de::value::BorrowedStrDeserializer::new(key))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> crate::Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }
}

impl<'de> serde::Deserializer<'de> for &'de Value {
    type Error = crate::Error;

    fn deserialize_any<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Number(Number::Integer(n)) => visitor.visit_i64(*n),
            Value::Number(Number::Float(n)) => visitor.visit_f64(*n),
            Value::String(s) => visitor.visit_borrowed_str(s),
            Value::Sequence(seq) => visitor.visit_seq(ValueSeqAccess { iter: seq.iter() }),
            Value::Mapping(map) => visitor.visit_map(ValueMapAccess {
                iter: map.iter(),
                value: None,
            }),
            Value::Tagged(tagged) => {
                let tagged_ref: &'de TaggedValue = tagged;
                serde::Deserializer::deserialize_any(tagged_ref, visitor)
            }
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Tagged(tagged) => {
                let tagged_ref: &'de TaggedValue = tagged;
                serde::Deserializer::deserialize_enum(tagged_ref, name, variants, visitor)
            }
            Value::String(s) => visitor
                .visit_enum(serde::de::value::BorrowedStrDeserializer::<crate::Error>::new(s)),
            _ => serde::Deserializer::deserialize_any(self, visitor),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Sequence(seq) => visitor.visit_seq(ValueSeqAccess { iter: seq.iter() }),
            _ => serde::Deserializer::deserialize_any(self, visitor),
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Mapping(map) => visitor.visit_map(ValueMapAccess {
                iter: map.iter(),
                value: None,
            }),
            _ => serde::Deserializer::deserialize_any(self, visitor),
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return visitor.visit_map(crate::de::SpannedMapAccess::new(self, None));
        }
        serde::Deserializer::deserialize_map(self, visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct tuple
        tuple_struct identifier ignored_any
    }
}

// ============================================================================
// From implementations for Value
// ============================================================================

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Number(Number::Integer(v))
    }
}

impl From<u8> for Value {
    fn from(v: u8) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<u16> for Value {
    fn from(v: u16) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Number(Number::Float(f64::from(v)))
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Number(Number::Float(v))
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_owned())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Sequence(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

impl From<Number> for Value {
    fn from(v: Number) -> Self {
        Value::Number(v)
    }
}

impl From<Mapping> for Value {
    fn from(v: Mapping) -> Self {
        Value::Mapping(v)
    }
}

// Note: From<Sequence> is covered by From<Vec<T>> since Sequence = Vec<Value>

impl From<TaggedValue> for Value {
    fn from(v: TaggedValue) -> Self {
        Value::Tagged(Box::new(v))
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::Number(Number::from(v))
    }
}

impl From<isize> for Value {
    fn from(v: isize) -> Self {
        Value::Number(Number::from(v))
    }
}

impl From<usize> for Value {
    fn from(v: usize) -> Self {
        Value::Number(Number::from(v))
    }
}

impl<'a> From<Cow<'a, str>> for Value {
    fn from(v: Cow<'a, str>) -> Self {
        Value::String(v.into_owned())
    }
}

impl<T: Clone + Into<Value>> From<&[T]> for Value {
    fn from(v: &[T]) -> Self {
        Value::Sequence(v.iter().cloned().map(Into::into).collect())
    }
}

impl<T: Into<Value>> FromIterator<T> for Value {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Value::Sequence(iter.into_iter().map(Into::into).collect())
    }
}

// ============================================================================
// Index trait implementations for Value
// ============================================================================

impl Index<usize> for Value {
    type Output = Value;

    /// Index into a YAML sequence.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a sequence or if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = "- a\n- b\n- c\n";
    /// let value: Value = from_str(yaml).unwrap();
    /// assert_eq!(value[0].as_str(), Some("a"));
    /// assert_eq!(value[1].as_str(), Some("b"));
    /// ```
    #[track_caller]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .expect("index out of bounds or not a sequence")
    }
}

impl IndexMut<usize> for Value {
    /// Mutably index into a YAML sequence.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a sequence or if the index is out of bounds.
    #[track_caller]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
            .expect("index out of bounds or not a sequence")
    }
}

impl Index<&str> for Value {
    type Output = Value;

    /// Index into a YAML mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a mapping or if the key is not found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = "name: test\nversion: 1\n";
    /// let value: Value = from_str(yaml).unwrap();
    /// assert_eq!(value["name"].as_str(), Some("test"));
    /// assert_eq!(value["version"].as_i64(), Some(1));
    /// ```
    #[track_caller]
    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).expect("key not found or not a mapping")
    }
}

impl IndexMut<&str> for Value {
    /// Mutably index into a YAML mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a mapping or if the key is not found.
    #[track_caller]
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.get_mut(key).expect("key not found or not a mapping")
    }
}
