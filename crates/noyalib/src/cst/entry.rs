// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Path-shaped mutable handle to a CST node — the `Entry` "pro"
//! interface that complements the functional `Document::set` /
//! `remove` / `push_back` / `insert_after` methods.
//!
//! The functional methods stay first-class: when the caller knows
//! the operation up front, `doc.set("server.port", "9090")?` is the
//! shortest path. `Entry` exists for the *idiomatic* / *composable*
//! cases:
//!
//! - chaining nested mutations without re-typing the prefix
//!   (`doc.entry("server").set_value(...)?`),
//! - reading-then-writing a field by the same handle
//!   (`if entry.exists() { entry.set("...")?; }`),
//! - inserting into a child mapping by name without manual
//!   path-string concatenation
//!   (`doc.entry("metadata.labels").insert("env", "prod")?`),
//! - keeping a single mutable borrow across a sequence of edits.
//!
//! Every operation routes through the same lossless CST splice path
//! the functional methods use — comments, blank lines, sibling
//! entries, and indentation are preserved byte-for-byte across
//! every Entry-driven edit.

use crate::cst::Document;
use crate::error::{Error, Result};
use crate::prelude::*;
use crate::value::Value;

/// A path-shaped mutable handle to a node in a [`Document`].
///
/// Created by [`Document::entry`]. Every method routes through the
/// same CST splice the functional API uses, so edits are byte-faithful
/// outside the touched span.
///
/// # Examples
///
/// Single-level edit, equivalent to `Document::set`:
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let mut doc = parse_document("port: 8080\n").unwrap();
/// doc.entry("port").set("9090").unwrap();
/// assert_eq!(doc.to_string(), "port: 9090\n");
/// ```
///
/// Nested edit by chained drill-down — no string concatenation:
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let mut doc = parse_document(
///     "server:\n  host: localhost\n  port: 8080\n",
/// ).unwrap();
/// doc.entry("server").entry("port").set("9090").unwrap();
/// assert!(doc.to_string().contains("port: 9090"));
/// ```
///
/// Insert into a mapping by key without manual path strings:
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let mut doc = parse_document(
///     "metadata:\n  labels:\n    app: noyalib\n",
/// ).unwrap();
/// doc.entry("metadata.labels").insert("env", "prod").unwrap();
/// let out = doc.to_string();
/// assert!(out.contains("app: noyalib"));
/// assert!(out.contains("env: prod"));
/// ```
#[derive(Debug)]
pub struct Entry<'a> {
    doc: &'a mut Document,
    path: String,
}

impl<'a> Entry<'a> {
    pub(crate) fn new(doc: &'a mut Document, path: String) -> Self {
        Self { doc, path }
    }

    /// The path this entry represents, as a dotted/indexed string.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// `true` if a node currently exists at this path.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.doc.span_at(&self.path).is_some()
    }

    /// The current source slice at this path, or `None` if the path
    /// does not resolve. Forwards to [`Document::get`].
    #[must_use]
    pub fn get(&self) -> Option<&str> {
        self.doc.get(&self.path)
    }

    /// The byte range `(start, end)` at this path, or `None` if the
    /// path does not resolve. Forwards to [`Document::span_at`].
    #[must_use]
    pub fn span_at(&self) -> Option<(usize, usize)> {
        self.doc.span_at(&self.path)
    }

    /// Comments that decorate this node. Forwards to
    /// [`Document::comments_at`].
    #[must_use]
    pub fn comments(&self) -> crate::cst::CommentBundle {
        self.doc.comments_at(&self.path)
    }

    /// Replace the value at this path with `fragment`. Forwards to
    /// [`Document::set`].
    ///
    /// Consumes the entry. Returns the [`Document`]'s reference back
    /// via the borrow expiring; chain a fresh `doc.entry(...)` for
    /// the next edit.
    ///
    /// # Errors
    ///
    /// Same as [`Document::set`] — invalid path, parse-after-splice
    /// failure, etc.
    pub fn set(self, fragment: &str) -> Result<()> {
        self.doc.set(&self.path, fragment)
    }

    /// Replace the value at this path with the YAML emission of
    /// `value`. Forwards to [`Document::set_value`].
    ///
    /// # Errors
    ///
    /// Same as [`Document::set_value`].
    pub fn set_value(self, value: &Value) -> Result<()> {
        self.doc.set_value(&self.path, value)
    }

    /// Remove the entry at this path. Forwards to
    /// [`Document::remove`].
    ///
    /// # Errors
    ///
    /// Same as [`Document::remove`].
    pub fn remove(self) -> Result<()> {
        self.doc.remove(&self.path)
    }

    /// Insert a `key: value` pair into the mapping at this path.
    /// If the key already exists, the value is replaced losslessly;
    /// if it is new, a sibling line is spliced after the last
    /// existing entry with matching indent. Forwards to
    /// [`Document::insert_entry`].
    ///
    /// # Errors
    ///
    /// - The mapping at `self.path` does not exist.
    /// - The mapping is empty (no anchor for indent — use
    ///   [`Self::set`] with a fragment to give the mapping its first
    ///   entry).
    /// - The same parse-after-splice errors as the underlying
    ///   `replace_span`.
    pub fn insert(self, key: &str, fragment: &str) -> Result<()> {
        self.doc.insert_entry(&self.path, key, fragment)
    }

    /// Insert a `key: value` pair where the value is a typed
    /// [`Value`], auto-formatting both halves.
    ///
    /// Forwards to [`Document::insert_entry_value`]: the value is
    /// emitted so it re-parses to exactly `value` — quoted when the
    /// plain spelling would not — the key is quoted when it needs to
    /// be, indentation follows the file's detected unit, and the
    /// splice is held to a re-parse plus typed-value check that rolls
    /// back on any mismatch.
    ///
    /// # Errors
    ///
    /// As [`Document::insert_entry_value`].
    pub fn insert_value(self, key: &str, value: &Value) -> Result<()> {
        self.doc.insert_entry_value(&self.path, key, value)
    }

    /// Append a typed [`Value`] to the sequence at this path.
    /// Forwards to [`Document::push_back_value`].
    ///
    /// # Errors
    ///
    /// As [`Document::push_back_value`] — the path must point to a
    /// non-empty block sequence.
    pub fn push_back_value(self, value: &Value) -> Result<()> {
        self.doc.push_back_value(&self.path, value)
    }

    /// Insert a typed [`Value`] as a new sequence item immediately
    /// after the item at this path. Forwards to
    /// [`Document::insert_after_value`].
    ///
    /// # Errors
    ///
    /// As [`Document::insert_after_value`] — the path must end in an
    /// index and resolve to an item of a block sequence.
    pub fn insert_after_value(self, value: &Value) -> Result<()> {
        self.doc.insert_after_value(&self.path, value)
    }

    /// Append `fragment` as a new item to the sequence at this path.
    /// Forwards to [`Document::push_back`].
    ///
    /// # Errors
    ///
    /// Same as [`Document::push_back`] — the path must point to a
    /// non-empty block sequence.
    pub fn push_back(self, fragment: &str) -> Result<()> {
        self.doc.push_back(&self.path, fragment)
    }

    /// Insert `fragment` as a new sequence item immediately after
    /// the item at this path. Forwards to [`Document::insert_after`].
    ///
    /// # Errors
    ///
    /// Same as [`Document::insert_after`] — the path must end in an
    /// index and resolve to a sequence item in a block sequence.
    pub fn insert_after(self, fragment: &str) -> Result<()> {
        self.doc.insert_after(&self.path, fragment)
    }

    /// Drill down to a child path. The returned [`Entry`] represents
    /// `self.path + "." + child`, holding the same mutable borrow. A
    /// child that begins with `[` — an index, or the quoted segment
    /// [`crate::path::quote_key`] spells for a key holding `.`, `[`,
    /// `]`, or `*` — is appended without the separator.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document(
    ///     "a:\n  b:\n    c: 1\n",
    /// ).unwrap();
    /// doc.entry("a").entry("b").entry("c").set("2").unwrap();
    /// assert!(doc.to_string().contains("c: 2"));
    /// ```
    #[must_use]
    pub fn entry(self, child: &str) -> Self {
        let combined = compose_path(&self.path, child);
        Entry::new(self.doc, combined)
    }

    /// Ensure a value exists at this path: if the entry already
    /// resolves, leave it untouched; if it does not, splice
    /// `default` as the new value.
    ///
    /// Mirrors the [`std::collections::hash_map::Entry::or_insert`]
    /// pattern adapted to the CST: the path is *resolved through the
    /// green tree first*, then the splice runs only on the vacant
    /// branch. The resolution is byte-faithful and skips the typed
    /// `Value` cache, so the call is cheap on already-occupied paths.
    ///
    /// Returns `Ok(true)` when the splice ran (the entry was
    /// vacant), `Ok(false)` when the entry was already occupied
    /// (no-op).
    ///
    /// # Errors
    ///
    /// - For mapping inserts: as [`Self::insert`] — the parent
    ///   mapping must exist at the dotted-path prefix and must not
    ///   be empty.
    /// - For top-level inserts (no `.` in path): rejected with a
    ///   clear message; use [`Document::set`] with a fresh document
    ///   for that case.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document(
    ///     "metadata:\n  labels:\n    app: noyalib\n",
    /// ).unwrap();
    ///
    /// // First call: the path was vacant, so the splice ran.
    /// let inserted = doc.entry("metadata.labels.env")
    ///     .or_insert("prod")
    ///     .unwrap();
    /// assert!(inserted);
    /// assert!(doc.to_string().contains("env: prod"));
    ///
    /// // Second call on the same path: the entry is now occupied,
    /// // so it returns `false` and leaves the value untouched.
    /// let inserted = doc.entry("metadata.labels.env")
    ///     .or_insert("staging")
    ///     .unwrap();
    /// assert!(!inserted);
    /// assert!(doc.to_string().contains("env: prod"));
    /// ```
    pub fn or_insert(self, default: &str) -> Result<bool> {
        if self.exists() {
            return Ok(false);
        }
        self.insert_at_path(default)?;
        Ok(true)
    }

    /// Like [`Self::or_insert`] but constructs the default lazily —
    /// the closure runs only on the vacant branch.
    ///
    /// # Errors
    ///
    /// As [`Self::or_insert`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document(
    ///     "metadata:\n  labels:\n    app: noyalib\n",
    /// ).unwrap();
    /// let _ = doc.entry("metadata.labels.env")
    ///     .or_insert_with(|| "prod".to_owned())
    ///     .unwrap();
    /// assert!(doc.to_string().contains("env: prod"));
    /// ```
    pub fn or_insert_with<F>(self, default: F) -> Result<bool>
    where
        F: FnOnce() -> String,
    {
        if self.exists() {
            return Ok(false);
        }
        let frag = default();
        self.insert_at_path(&frag)?;
        Ok(true)
    }

    /// Like [`Self::or_insert`] but takes a typed [`Value`]. Honours
    /// the file's detected indent unit
    /// ([`Document::indent_unit`]) when emitting the value.
    ///
    /// # Errors
    ///
    /// As [`Self::or_insert`], plus serializer errors when the value
    /// cannot be emitted as YAML.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    /// use noyalib::Value;
    ///
    /// let mut doc = parse_document(
    ///     "metadata:\n  labels:\n    app: noyalib\n",
    /// ).unwrap();
    /// let _ = doc.entry("metadata.labels.replicas")
    ///     .or_insert_value(&Value::Number(noyalib::Number::Integer(3)))
    ///     .unwrap();
    /// assert!(doc.to_string().contains("replicas: 3"));
    /// ```
    pub fn or_insert_value(self, default: &Value) -> Result<bool> {
        if self.exists() {
            return Ok(false);
        }
        // Splits the leaf key off the path and goes through the
        // auto-formatting insert, so the value is quoted to re-parse
        // as itself and the splice is guarded — the same contract
        // `insert_value` offers.
        self.insert_value_at_path(default)?;
        Ok(true)
    }

    /// If the entry is occupied, run `f` and let it apply
    /// arbitrary mutations through a fresh sub-borrow of the
    /// document. No-op when the entry is vacant. The closure
    /// receives a [`Document`] reference so it can touch any path,
    /// not just `self.path` — useful for "if this exists, also
    /// update its sibling" patterns.
    ///
    /// Returns `self` for further chaining (typically with
    /// [`Self::or_insert`]) — mirrors
    /// [`std::collections::hash_map::Entry::and_modify`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document(
    ///     "service:\n  port: 8080\n",
    /// ).unwrap();
    ///
    /// // Increment-style update: read, mutate, write — all under
    /// // the same Entry handle. The closure only runs because the
    /// // path is occupied.
    /// let _ = doc.entry("service.port")
    ///     .and_modify(|d| {
    ///         let _ = d.set("service.port", "9090");
    ///     })
    ///     .or_insert("8080") // no-op now: path exists
    ///     .unwrap();
    /// assert!(doc.to_string().contains("port: 9090"));
    /// ```
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut Document),
    {
        if self.doc.span_at(&self.path).is_some() {
            f(self.doc);
        }
        self
    }

    /// Helper: split `self.path` into `(parent, leaf)` and call
    /// `Document::insert_entry`. Top-level paths (no `.`) and
    /// sequence-index paths are rejected with an actionable error.
    fn insert_at_path(self, fragment: &str) -> Result<()> {
        let path = self.path;
        // Sequence indexes can't be inserted via the mapping path —
        // detect either a leading `[` or a final `[index]` segment
        // anywhere in the dotted path so the error is actionable.
        if path.contains('[') {
            return Err(Error::Parse(format!(
                "or_insert: cannot insert at sequence index `{path}`; \
                 use Entry::push_back or Entry::insert_after instead"
            )));
        }
        match path.rfind('.') {
            Some(idx) => {
                let (parent, key_with_dot) = path.split_at(idx);
                // Skip the leading `.` to get the leaf key.
                let key = &key_with_dot[1..];
                self.doc.insert_entry(parent, key, fragment)
            }
            None => Err(Error::Parse(format!(
                "or_insert: cannot insert at top-level key `{path}` \
                 on an existing document — use Document::set on a \
                 non-existent path or insert through a parent mapping"
            ))),
        }
    }

    /// The typed analogue of [`Self::insert_at_path`]: split the leaf
    /// key off `self.path` and insert `value` under it with
    /// auto-formatting and the post-splice guard.
    fn insert_value_at_path(self, value: &Value) -> Result<()> {
        let path = self.path;
        if path.contains('[') {
            return Err(Error::Parse(format!(
                "or_insert: cannot insert at sequence index `{path}`; \
                 use Entry::push_back_value or Entry::insert_after_value instead"
            )));
        }
        match path.rfind('.') {
            Some(idx) => {
                let (parent, key_with_dot) = path.split_at(idx);
                let key = &key_with_dot[1..];
                self.doc.insert_entry_value(parent, key, value)
            }
            None => Err(Error::Parse(format!(
                "or_insert: cannot insert at top-level key `{path}` \
                 on an existing document — use Document::set on a \
                 non-existent path or insert through a parent mapping"
            ))),
        }
    }
}

impl Document {
    /// Return a path-shaped mutable handle to the node at `path`.
    ///
    /// The handle is the "pro" mutation interface — chainable,
    /// composable, ergonomic for nested edits — that complements the
    /// functional [`Document::set`] / [`Document::remove`] /
    /// [`Document::push_back`] / [`Document::insert_after`] methods
    /// (all of which remain available for direct one-shot operations).
    ///
    /// `entry` itself is infallible — the path is recorded but not
    /// resolved at this point. Operations on the returned entry
    /// surface path-resolution and splice-failure errors via their
    /// own `Result`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document(
    ///     "metadata:\n  labels:\n    app: noyalib\n",
    /// ).unwrap();
    /// doc.entry("metadata.labels").insert("env", "prod").unwrap();
    ///
    /// let out = doc.to_string();
    /// assert!(out.contains("app: noyalib"));
    /// assert!(out.contains("env: prod"));
    /// ```
    pub fn entry<'a>(&'a mut self, path: &str) -> Entry<'a> {
        Entry::new(self, path.to_owned())
    }
}

/// Concatenate a parent path and a child segment with a dot
/// separator, except when the parent is empty (in which case the
/// child is the whole path) or the child already begins with `[`
/// (an index — no separator needed).
fn compose_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        return child.to_owned();
    }
    if child.starts_with('[') {
        return format!("{parent}{child}");
    }
    format!("{parent}.{child}")
}

// ── Trait impls ──────────────────────────────────────────────────────

impl Error {
    /// Internal — surface a generic "no entry" message used by the
    /// upcoming `or_insert` machinery (v0.0.2). Defined here so the
    /// public Error API does not grow a new variant for the in-flight
    /// work yet.
    #[doc(hidden)]
    #[must_use]
    pub fn entry_not_found(path: &str) -> Self {
        Self::Parse(format!("entry not found at path: {path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parse_document;

    #[test]
    fn entry_set_replaces_value_losslessly() {
        let mut doc = parse_document("# top comment\nport: 8080  # inline comment\n").unwrap();
        doc.entry("port").set("9090").unwrap();
        let out = doc.to_string();
        assert!(out.contains("port: 9090"));
        assert!(out.contains("# top comment"));
        assert!(out.contains("# inline comment"));
    }

    #[test]
    fn entry_chains_through_dotted_path() {
        let mut doc = parse_document("server:\n  host: localhost\n  port: 8080\n").unwrap();
        doc.entry("server").entry("port").set("9090").unwrap();
        assert!(doc.to_string().contains("port: 9090"));
    }

    #[test]
    fn entry_insert_into_mapping() {
        let mut doc = parse_document("metadata:\n  labels:\n    app: noyalib\n").unwrap();
        doc.entry("metadata.labels").insert("env", "prod").unwrap();
        let out = doc.to_string();
        assert!(out.contains("app: noyalib"));
        assert!(out.contains("env: prod"));
    }

    #[test]
    fn entry_insert_value_typed() {
        let mut doc = parse_document("metadata:\n  labels:\n    app: noyalib\n").unwrap();
        let v = Value::Number(crate::Number::Integer(3));
        doc.entry("metadata.labels")
            .insert_value("replicas", &v)
            .unwrap();
        let out = doc.to_string();
        assert!(out.contains("replicas: 3"));
    }

    #[test]
    fn entry_remove() {
        let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
        doc.entry("b").remove().unwrap();
        let out = doc.to_string();
        assert!(out.contains("a: 1"));
        assert!(!out.contains("b:"));
        assert!(out.contains("c: 3"));
    }

    #[test]
    fn entry_push_back_to_sequence() {
        let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
        doc.entry("items").push_back("three").unwrap();
        let out = doc.to_string();
        assert!(out.contains("- one"));
        assert!(out.contains("- two"));
        assert!(out.contains("- three"));
    }

    #[test]
    fn entry_insert_after_in_sequence() {
        let mut doc = parse_document("items:\n  - one\n  - three\n").unwrap();
        doc.entry("items[0]").insert_after("two").unwrap();
        let out = doc.to_string();
        let one_pos = out.find("one").unwrap();
        let two_pos = out.find("two").unwrap();
        let three_pos = out.find("three").unwrap();
        assert!(one_pos < two_pos);
        assert!(two_pos < three_pos);
    }

    #[test]
    fn entry_get_reads_source_slice() {
        let doc = parse_document("port: 8080\n").unwrap();
        // `entry` needs `&mut self`, so reads via Entry require a
        // throwaway doc — but the `get` method is a pure read.
        let mut doc = doc;
        let e = doc.entry("port");
        assert_eq!(e.get(), Some("8080"));
    }

    #[test]
    fn entry_exists_distinguishes_present_and_absent() {
        let mut doc = parse_document("a: 1\n").unwrap();
        assert!(doc.entry("a").exists());
        assert!(!doc.entry("b").exists());
    }

    #[test]
    fn entry_path_returns_recorded_string() {
        let mut doc = parse_document("a:\n  b: 1\n").unwrap();
        let e = doc.entry("a").entry("b");
        assert_eq!(e.path(), "a.b");
    }

    #[test]
    fn entry_with_index_uses_no_dot_separator() {
        let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
        // `entry("items").entry("[0]")` should produce `items[0]`,
        // not `items.[0]`.
        let e = doc.entry("items").entry("[0]");
        assert_eq!(e.path(), "items[0]");
    }

    #[test]
    fn entry_comments_forwards_to_document() {
        let mut doc = parse_document("# decorator\nport: 8080  # inline\n").unwrap();
        let bundle = doc.entry("port").comments();
        assert_eq!(bundle.before.len(), 1);
        assert_eq!(bundle.before[0].text, " decorator");
        assert_eq!(bundle.inline.as_ref().unwrap().text, " inline");
    }

    #[test]
    fn entry_set_on_nonexistent_path_errors() {
        let mut doc = parse_document("a: 1\n").unwrap();
        let err = doc.entry("nonexistent").set("2").unwrap_err();
        assert!(err.to_string().contains("path not found"));
    }

    #[test]
    fn entry_repeated_edits_compose() {
        // Multiple Entry-driven edits in sequence — each releases
        // its borrow at the end of its statement, the next can
        // reborrow doc cleanly.
        let mut doc = parse_document("name: noyalib\nversion: 0.0.1\n").unwrap();
        doc.entry("name").set("renamed").unwrap();
        doc.entry("version").set("0.0.2").unwrap();
        let out = doc.to_string();
        assert!(out.contains("name: renamed"));
        assert!(out.contains("version: 0.0.2"));
    }
}
