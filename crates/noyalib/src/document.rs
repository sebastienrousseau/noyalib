//! Multi-document YAML loading.
//!
//! This module provides functionality for parsing YAML documents that contain
//! multiple documents separated by `---`.
//!
//! # Examples
//!
//! ```rust
//! use noyalib::document::load_all;
//!
//! let yaml = "---
//! name: doc1
//! ---
//! name: doc2
//! ";
//!
//! let docs: Vec<_> = load_all(yaml).unwrap().collect();
//! assert_eq!(docs.len(), 2);
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::de::ParserConfig;
use crate::error::{Error, Result};
use crate::parser;
use crate::prelude::*;
#[cfg(feature = "std")]
use crate::span_context::{self, SpanTree};
use crate::value::Value;
#[cfg(not(feature = "std"))]
use alloc::vec::IntoIter;
use core::marker::PhantomData;
#[cfg(feature = "std")]
use std::vec::IntoIter;

/// An iterator over YAML documents in a string.
///
/// Created by the [`load_all`] function.
///
/// # Examples
///
/// ```
/// use noyalib::document::load_all;
/// let iter = load_all("---\na: 1\n---\nb: 2\n").unwrap();
/// assert_eq!(iter.len(), 2);
/// ```
#[derive(Debug)]
pub struct DocumentIterator {
    docs: IntoIter<Value>,
    #[cfg(feature = "std")]
    _span_trees: Vec<SpanTree>,
    total: usize,
}

impl DocumentIterator {
    /// Returns the total number of documents parsed.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::document::load_all;
    /// let iter = load_all("a: 1\n").unwrap();
    /// assert_eq!(iter.len(), 1);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.total
    }

    /// Returns true if there are no documents.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::document::load_all;
    /// let iter = load_all("a: 1\n").unwrap();
    /// assert!(!iter.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }
}

impl Iterator for DocumentIterator {
    type Item = Result<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        self.docs.next().map(Ok)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.docs.size_hint()
    }
}

impl ExactSizeIterator for DocumentIterator {}

/// Load all YAML documents from a string.
///
/// This function parses a YAML string that may contain multiple documents
/// separated by `---` markers. Default security limits are applied.
///
/// # Examples
///
/// ```rust
/// use noyalib::document::load_all;
///
/// let yaml = "---
/// first: 1
/// ---
/// second: 2
/// ";
///
/// let docs: Vec<_> = load_all(yaml).unwrap().filter_map(Result::ok).collect();
/// assert_eq!(docs.len(), 2);
/// ```
///
/// # Errors
///
/// Returns an error if the YAML syntax is invalid.
pub fn load_all(input: &str) -> Result<DocumentIterator> {
    load_all_with_config(input, &ParserConfig::default())
}

/// Load all YAML documents from a string with custom security limits.
///
/// # Errors
///
/// Returns an error if the YAML syntax is invalid or the document
/// exceeds the configured limits.
///
/// # Examples
///
/// ```
/// use noyalib::{document::load_all_with_config, ParserConfig};
/// let cfg = ParserConfig::new();
/// let iter = load_all_with_config("a: 1\n---\nb: 2\n", &cfg).unwrap();
/// assert_eq!(iter.len(), 2);
/// ```
pub fn load_all_with_config(input: &str, config: &ParserConfig) -> Result<DocumentIterator> {
    if input.len() > config.max_document_length {
        return Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            config.max_document_length
        )));
    }
    let parse_config = parser::ParseConfig::from(config);

    #[cfg(feature = "std")]
    {
        let pairs = parser::parse(input, &parse_config)?;
        let (docs, span_trees): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
        let total = docs.len();
        Ok(DocumentIterator {
            docs: docs.into_iter(),
            _span_trees: span_trees,
            total,
        })
    }

    #[cfg(not(feature = "std"))]
    {
        let docs = parser::parse_all_values(input, &parse_config)?;
        let total = docs.len();
        Ok(DocumentIterator {
            docs: docs.into_iter(),
            total,
        })
    }
}

/// Load all YAML documents from a string, returning an error if parsing fails.
///
/// This is an alias for [`load_all`] which also returns errors on invalid
/// syntax.
///
/// # Examples
///
/// ```rust
/// use noyalib::document::try_load_all;
///
/// let yaml = "---
/// first: 1
/// ---
/// second: 2
/// ";
///
/// let iter = try_load_all(yaml).unwrap();
/// assert_eq!(iter.len(), 2);
/// ```
///
/// # Errors
///
/// Returns an error if the YAML syntax is invalid.
pub fn try_load_all(input: &str) -> Result<DocumentIterator> {
    load_all(input)
}

/// Load all YAML documents and deserialize them into a typed vector.
///
/// # Examples
///
/// ```rust
/// use noyalib::document::load_all_as;
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, PartialEq)]
/// struct Doc {
///     name: String,
/// }
///
/// let yaml = "---
/// name: first
/// ---
/// name: second
/// ";
///
/// let docs: Vec<Doc> = load_all_as(yaml).unwrap();
/// assert_eq!(docs.len(), 2);
/// assert_eq!(docs[0].name, "first");
/// assert_eq!(docs[1].name, "second");
/// ```
///
/// # Errors
///
/// Returns an error if parsing fails or if any document cannot be
/// deserialized into the target type.
pub fn load_all_as<T>(input: &str) -> Result<Vec<T>>
where
    T: for<'de> serde::Deserialize<'de> + 'static,
{
    let parse_config = parser::ParseConfig::from(&ParserConfig::default());

    #[cfg(feature = "std")]
    {
        let pairs = parser::parse(input, &parse_config)?;
        let mut results = Vec::with_capacity(pairs.len());
        let source: Arc<str> = input.into();

        for (value, span_tree) in &pairs {
            let spans = span_context::build_span_map(value, span_tree);
            let ctx = span_context::SpanContext {
                spans,
                source: source.clone(),
            };
            let _guard = span_context::set_span_context(ctx);
            let typed: T = crate::from_value(value)?;
            results.push(typed);
        }

        Ok(results)
    }

    #[cfg(not(feature = "std"))]
    {
        let docs = parser::parse_all_values(input, &parse_config)?;
        let mut results = Vec::with_capacity(docs.len());
        for value in &docs {
            let typed: T = crate::from_value(value)?;
            results.push(typed);
        }
        Ok(results)
    }
}

/// Lazy iterator that yields `Result<T>` per YAML document parsed
/// from a reader.
///
/// Created by [`read`] / [`read_with_config`]. Deserialisation
/// errors on individual documents are surfaced as `Err` values; the
/// iterator continues so callers can recover and process subsequent
/// documents. Syntax errors during the initial parse are returned
/// from [`read`] / [`read_with_config`] before iteration starts.
///
/// # Memory
///
/// Today the reader is fully drained into a `String` before the
/// underlying parser runs, so memory is `O(input_len)`. True
/// `O(1)`-document streaming requires a parser-level rewrite that
/// can accept incremental byte chunks; that work is tracked
/// separately.
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct DocumentReadIterator<T> {
    docs: IntoIter<Value>,
    _phantom: PhantomData<fn() -> T>,
}

#[cfg(feature = "std")]
impl<T> DocumentReadIterator<T> {
    /// Total number of documents pending iteration.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// let yaml = "a: 1\n---\nb: 2\n";
    /// let iter: noyalib::DocumentReadIterator<noyalib::Value> =
    ///     noyalib::read(Cursor::new(yaml)).unwrap();
    /// assert_eq!(iter.len(), 2);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Whether the iterator has no further documents.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// let iter: noyalib::DocumentReadIterator<noyalib::Value> =
    ///     noyalib::read(Cursor::new("")).unwrap();
    /// assert!(iter.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.docs.len() == 0
    }
}

#[cfg(feature = "std")]
impl<T> Iterator for DocumentReadIterator<T>
where
    T: for<'de> serde::Deserialize<'de> + 'static,
{
    type Item = Result<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.docs.next()?;
        Some(crate::from_value(&value))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.docs.size_hint()
    }
}

#[cfg(feature = "std")]
impl<T> ExactSizeIterator for DocumentReadIterator<T> where
    T: for<'de> serde::Deserialize<'de> + 'static
{
}

/// Stream-decode every YAML document from a reader into typed
/// values, yielding one `Result<T>` per document.
///
/// The reader is drained eagerly (see [`DocumentReadIterator`] for
/// the memory caveat); document-by-document deserialisation is then
/// produced lazily on demand. Per-document deserialisation errors
/// surface as `Err` values inside the iterator so callers can
/// recover and continue. A syntax error in the underlying YAML is
/// returned synchronously from this function before any iteration
/// happens.
///
/// # Errors
///
/// Returns an error if the reader fails, the YAML cannot be parsed,
/// or any document exceeds the default security limits. Per-document
/// deserialisation errors are *not* surfaced here; they appear
/// inside the iterator.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, PartialEq)]
/// struct Doc { id: u32 }
///
/// let yaml = "id: 1\n---\nid: 2\n---\nid: 3\n";
/// let docs: Vec<Doc> = noyalib::read::<_, Doc>(Cursor::new(yaml))
///     .unwrap()
///     .filter_map(Result::ok)
///     .collect();
/// assert_eq!(docs, vec![Doc { id: 1 }, Doc { id: 2 }, Doc { id: 3 }]);
/// ```
#[cfg(feature = "std")]
pub fn read<R, T>(reader: R) -> Result<DocumentReadIterator<T>>
where
    R: std::io::Read,
    T: for<'de> serde::Deserialize<'de> + 'static,
{
    read_with_config(reader, &ParserConfig::default())
}

/// [`read`] with a custom [`ParserConfig`] for tightened security
/// limits.
///
/// # Errors
///
/// Same as [`read`].
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
/// use noyalib::{read_with_config, ParserConfig, Value};
///
/// let cfg = ParserConfig::strict();
/// let yaml = "a: 1\n---\nb: 2\n";
/// let count = read_with_config::<_, Value>(Cursor::new(yaml), &cfg)
///     .unwrap()
///     .count();
/// assert_eq!(count, 2);
/// ```
#[cfg(feature = "std")]
pub fn read_with_config<R, T>(
    mut reader: R,
    config: &ParserConfig,
) -> Result<DocumentReadIterator<T>>
where
    R: std::io::Read,
    T: for<'de> serde::Deserialize<'de> + 'static,
{
    let mut buf = String::new();
    let _read_bytes = reader
        .read_to_string(&mut buf)
        .map_err(|e| Error::Parse(format!("reader I/O failed: {e}")))?;
    if buf.len() > config.max_document_length.saturating_mul(64) {
        // Soft cap on the *aggregated* multi-document buffer to
        // bound memory regardless of per-document caps.
        return Err(Error::Parse(format!(
            "reader payload exceeds 64× max_document_length ({} bytes)",
            config.max_document_length
        )));
    }
    let parse_config = parser::ParseConfig::from(config);
    let pairs = parser::parse(&buf, &parse_config)?;
    let docs: Vec<Value> = pairs.into_iter().map(|(value, _)| value).collect();
    Ok(DocumentReadIterator {
        docs: docs.into_iter(),
        _phantom: PhantomData,
    })
}
