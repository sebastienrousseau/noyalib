// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Key interning for memory-efficient repeated-key workloads.
//!
//! YAML mappings frequently repeat the same key string across
//! many records — `metadata`, `labels`, `name`, `version`,
//! `apiVersion` in Kubernetes-shaped documents, `level`, `time`,
//! `service` in structured logs. Each fresh parse allocates a
//! brand-new `String` for every key. For a 10 000-record stream
//! that's tens of thousands of identical heap allocations.
//!
//! [`KeyInterner`](crate::interner::KeyInterner) is the primitive
//! that lets a caller dedupe
//! those allocations. The intern table maps each unique key text
//! to a shared `Arc<str>`; subsequent calls with the same key
//! return a clone of the existing `Arc` instead of allocating.
//!
//! Memory model: `Arc<str>` is two pointers (16 bytes on a 64-bit
//! target) plus the actual string bytes shared via the `Arc`.
//! For a key that occurs `N` times, the interner's footprint is
//! `O(unique_keys * key_bytes) + N * 16` versus the
//! non-interned baseline of `O(N * key_bytes)`. For repeated keys
//! the win is substantial — 20-byte keys repeated 10 000× drop
//! from ~200 KB to ~20 bytes + 160 KB of `Arc` pointers.
//!
//! # Example: deduplicating keys across a parsed stream
//!
//! ```
//! use noyalib::interner::KeyInterner;
//! let mut interner = KeyInterner::new();
//! let a = interner.intern("metadata");
//! let b = interner.intern("metadata");
//! // Same `Arc` — second call returns a clone of the cached
//! // entry. `Arc::ptr_eq` confirms the underlying allocation
//! // is shared.
//! assert!(std::sync::Arc::ptr_eq(&a, &b));
//! assert_eq!(&*a, "metadata");
//! ```
//!
//! # Future direction
//!
//! v0.0.1 ships the primitive without changing
//! [`crate::Mapping`]'s storage type — the `Mapping`'s public API
//! is `String`-keyed and that contract is preserved across the
//! initial release. A future major version may switch the
//! internal key representation to `Arc<str>` and use the
//! interner transparently during parse; the public surface would
//! continue to behave like a `String`-keyed map via deref
//! semantics.

use crate::prelude::*;
use crate::prelude::{FxBuildHasher, FxHashMap};

/// Interner for `&str` → `Arc<str>` deduplication.
///
/// Each call to [`KeyInterner::intern`] returns a shared
/// `Arc<str>` for the supplied key. The first call allocates the
/// string; every subsequent call with the same key bytes returns
/// a clone of the cached `Arc`, sharing the underlying
/// allocation.
///
/// # Examples
///
/// ```
/// use noyalib::interner::KeyInterner;
/// let mut interner = KeyInterner::new();
/// let key = interner.intern("port");
/// assert_eq!(&*key, "port");
/// assert_eq!(interner.len(), 1);
///
/// let key2 = interner.intern("port");
/// // Re-intern returns the same Arc — no new allocation.
/// assert!(std::sync::Arc::ptr_eq(&key, &key2));
/// assert_eq!(interner.len(), 1);
/// ```
#[derive(Debug, Default)]
pub struct KeyInterner {
    // The value side of the map is `()` — we use `FxHashMap` as a
    // hash table keyed by `Arc<str>`. `get_key_value` lets us
    // return the existing `Arc<str>` clone without allocating a
    // fresh one when a key is already present.
    table: FxHashMap<Arc<str>, ()>,
}

impl KeyInterner {
    /// Construct a fresh interner with no entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::interner::KeyInterner;
    /// let interner = KeyInterner::new();
    /// assert_eq!(interner.len(), 0);
    /// assert!(interner.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            table: FxHashMap::default(),
        }
    }

    /// Construct an interner with capacity for at least
    /// `capacity` distinct keys. Useful when the caller knows the
    /// upper-bound on unique keys (e.g. a schema's field count).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::interner::KeyInterner;
    /// let interner = KeyInterner::with_capacity(64);
    /// assert!(interner.is_empty());
    /// ```
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            table: FxHashMap::with_capacity_and_hasher(capacity, FxBuildHasher),
        }
    }

    /// Intern a key. The first call with a given byte sequence
    /// allocates an `Arc<str>`; subsequent calls return a clone
    /// of the cached entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::interner::KeyInterner;
    /// let mut interner = KeyInterner::new();
    /// let one = interner.intern("metadata");
    /// let two = interner.intern("metadata");
    /// assert!(std::sync::Arc::ptr_eq(&one, &two));
    /// ```
    pub fn intern(&mut self, key: &str) -> Arc<str> {
        if let Some((existing, ())) = self.table.get_key_value(key) {
            return Arc::clone(existing);
        }
        let arc: Arc<str> = Arc::from(key);
        let _ = self.table.insert(Arc::clone(&arc), ());
        arc
    }

    /// Look up a key without inserting. Returns `Some(arc)` when
    /// the key is already interned, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::interner::KeyInterner;
    /// let mut interner = KeyInterner::new();
    /// assert!(interner.get("port").is_none());
    /// let _ = interner.intern("port");
    /// assert!(interner.get("port").is_some());
    /// ```
    #[must_use]
    pub fn get(&self, key: &str) -> Option<Arc<str>> {
        self.table
            .get_key_value(key)
            .map(|(arc, ())| Arc::clone(arc))
    }

    /// Number of distinct keys interned so far.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::interner::KeyInterner;
    /// let mut interner = KeyInterner::new();
    /// let _ = interner.intern("a");
    /// let _ = interner.intern("b");
    /// let _ = interner.intern("a"); // not a new key
    /// assert_eq!(interner.len(), 2);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns `true` if no keys have been interned.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::interner::KeyInterner;
    /// let interner = KeyInterner::new();
    /// assert!(interner.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Clear every interned key. The underlying `Arc`s remain
    /// valid for any caller that still holds a clone.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::interner::KeyInterner;
    /// let mut interner = KeyInterner::new();
    /// let _ = interner.intern("k");
    /// interner.clear();
    /// assert!(interner.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.table.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_returns_same_arc_for_repeated_key() {
        let mut interner = KeyInterner::new();
        let a = interner.intern("metadata");
        let b = interner.intern("metadata");
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn intern_distinct_keys_get_distinct_arcs() {
        let mut interner = KeyInterner::new();
        let a = interner.intern("port");
        let b = interner.intern("host");
        assert!(!Arc::ptr_eq(&a, &b));
        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn empty_string_is_interned() {
        let mut interner = KeyInterner::new();
        let a = interner.intern("");
        let b = interner.intern("");
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(&*a, "");
    }

    #[test]
    fn get_returns_existing_arc_without_inserting() {
        let mut interner = KeyInterner::new();
        assert!(interner.get("k").is_none());
        assert_eq!(interner.len(), 0);
        let inserted = interner.intern("k");
        let fetched = interner.get("k").unwrap();
        assert!(Arc::ptr_eq(&inserted, &fetched));
    }

    #[test]
    fn clear_resets_the_intern_table() {
        let mut interner = KeyInterner::new();
        let a = interner.intern("k");
        interner.clear();
        assert!(interner.is_empty());
        // The previously-issued Arc is still valid.
        assert_eq!(&*a, "k");
        // Re-interning the same key gets a *new* Arc since the
        // table has been cleared.
        let b = interner.intern("k");
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn with_capacity_constructs_empty_interner() {
        let interner = KeyInterner::with_capacity(128);
        assert!(interner.is_empty());
        assert_eq!(interner.len(), 0);
    }

    #[test]
    fn dedupes_real_kubernetes_key_set() {
        // Keys that occur many times in a typical Kubernetes
        // manifest. After interning, every duplicate `metadata`
        // / `labels` / `name` / `selector` shares one allocation.
        let mut interner = KeyInterner::new();
        let keys = [
            "apiVersion",
            "kind",
            "metadata",
            "name",
            "labels",
            "spec",
            "selector",
            "matchLabels",
            "template",
            "containers",
            "image",
            "ports",
            "containerPort",
        ];
        // Intern every key 100 times to simulate a stream of 100
        // similar manifests.
        for _ in 0..100 {
            for &k in &keys {
                let _ = interner.intern(k);
            }
        }
        // The table only has one entry per distinct key.
        assert_eq!(interner.len(), keys.len());
    }
}
