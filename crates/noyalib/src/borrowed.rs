// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Zero-copy YAML values that borrow strings from the input.
//!
//! [`BorrowedValue`] is the zero-copy counterpart of [`Value`](crate::Value).
//! String scalars and mapping keys use `Cow<'a, str>`, borrowing directly from
//! the input buffer when no escape processing was needed. This eliminates heap
//! allocations for the majority of YAML content.
//!
//! # Examples
//!
//! ```rust
//! use noyalib::borrowed::{from_str_borrowed, BorrowedValue};
//!
//! let yaml = "name: noyalib\nversion: 1\n";
//! let value: BorrowedValue<'_> = from_str_borrowed(yaml).unwrap();
//! assert_eq!(value.as_mapping().unwrap().get("name").unwrap().as_str(), Some("noyalib"));
//! ```

use crate::error::{Error, Result};
use crate::parser::meter::{AliasCost, CostTally, Meter};
use crate::parser::{
    Event, InternalDuplicateKeyPolicy, InternalMergeKeyPolicy, ParseConfig, Parser, ScalarStyle,
    budget,
};
use crate::path::{QueryPath, QuerySegment, parse_query_path};
use crate::prelude::IndexMap;
use crate::prelude::*;
use crate::prelude::{FxBuildHasher, FxHashMap};
use core::hash::{Hash, Hasher};

/// Why a YAML scalar could not be borrowed directly from the input
/// buffer and had to be materialised into an owned `String`.
///
/// Surfaced for users introspecting [`BorrowedValue`] / streaming
/// deserialisation paths: when a `Cow<'a, str>` resolves to
/// [`Cow::Owned`] instead of [`Cow::Borrowed`], one of these reasons
/// applies. Useful in benchmarks and allocation-budget audits.
///
/// # Examples
///
/// ```
/// use noyalib::borrowed::TransformReason;
/// assert_eq!(TransformReason::EscapeSequence.as_str(),
///            "scalar contained escape sequences that required decoding");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TransformReason {
    /// The scalar contained `\n`, `\t`, `\xNN`, `\uNNNN`, `\UNNNNNNNN`,
    /// or other escape sequences that required decoding into a fresh
    /// allocation.
    EscapeSequence,
    /// The scalar spans multiple physical lines and required line
    /// folding (block scalar `>` or `|`, or a multi-line flow scalar).
    LineFold,
    /// Tag resolution materialised a fresh representation (`!!binary`
    /// base64 decode, custom-tag dispatch via [`crate::TagRegistry`]).
    TagResolution,
    /// The scalar was double-quoted and contained at least one escape,
    /// so the parser produced an owned post-escape buffer.
    QuotedScalar,
    /// The scalar arrived via alias replay (`*anchor`) and the replayed
    /// buffer is owned by the alias-expansion machinery, not the input
    /// slice.
    AliasExpansion,
}

impl TransformReason {
    /// A human-readable explanation suitable for inclusion in error
    /// messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::TransformReason;
    /// assert_eq!(TransformReason::LineFold.as_str(),
    ///            "scalar spans multiple lines and required line folding");
    /// ```
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EscapeSequence => "scalar contained escape sequences that required decoding",
            Self::LineFold => "scalar spans multiple lines and required line folding",
            Self::TagResolution => "tag resolution materialised a fresh representation",
            Self::QuotedScalar => "double-quoted scalar with escapes produced an owned buffer",
            Self::AliasExpansion => "scalar arrived via alias replay (`*anchor`)",
        }
    }
}

impl fmt::Display for TransformReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A zero-copy YAML value that borrows strings from the input.
///
/// # Examples
///
/// ```
/// use noyalib::borrowed::{from_str_borrowed, BorrowedValue};
/// let v: BorrowedValue<'_> = from_str_borrowed("k: 1\n").unwrap();
/// assert!(v.as_mapping().is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorrowedValue<'a> {
    /// YAML null.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// assert!(BorrowedValue::Null.is_null());
    /// ```
    Null,
    /// YAML boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// let v = BorrowedValue::Bool(true);
    /// assert_eq!(v.as_bool(), Some(true));
    /// ```
    Bool(bool),
    /// YAML number.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{borrowed::BorrowedValue, Number};
    /// let v = BorrowedValue::Number(Number::Integer(42));
    /// assert_eq!(v.as_i64(), Some(42));
    /// ```
    Number(crate::value::Number),
    /// YAML string — borrows from input when possible.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use noyalib::borrowed::BorrowedValue;
    /// let v = BorrowedValue::String(Cow::Borrowed("hi"));
    /// assert_eq!(v.as_str(), Some("hi"));
    /// ```
    String(Cow<'a, str>),
    /// YAML sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// let v = BorrowedValue::Sequence(vec![BorrowedValue::Null]);
    /// assert_eq!(v.as_sequence().unwrap().len(), 1);
    /// ```
    Sequence(Vec<Self>),
    /// YAML mapping with borrowed keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::{from_str_borrowed, BorrowedValue};
    /// let v = from_str_borrowed("k: 1\n").unwrap();
    /// assert!(matches!(v, BorrowedValue::Mapping(_)));
    /// ```
    Mapping(IndexMap<Cow<'a, str>, Self, FxBuildHasher>),
}

impl<'a> BorrowedValue<'a> {
    /// Returns `true` if this is a null value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// assert!(BorrowedValue::Null.is_null());
    /// ```
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns the string value if this is a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use noyalib::borrowed::BorrowedValue;
    /// let v = BorrowedValue::String(Cow::Borrowed("hi"));
    /// assert_eq!(v.as_str(), Some("hi"));
    /// ```
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the i64 value if this is an integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{borrowed::BorrowedValue, Number};
    /// let v = BorrowedValue::Number(Number::Integer(42));
    /// assert_eq!(v.as_i64(), Some(42));
    /// ```
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// Returns the bool value if this is a boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// assert_eq!(BorrowedValue::Bool(true).as_bool(), Some(true));
    /// ```
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the sequence if this is a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::BorrowedValue;
    /// let v = BorrowedValue::Sequence(vec![BorrowedValue::Null]);
    /// assert_eq!(v.as_sequence().unwrap().len(), 1);
    /// ```
    #[must_use]
    pub fn as_sequence(&self) -> Option<&[Self]> {
        match self {
            Self::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the mapping if this is a mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::from_str_borrowed;
    /// let v = from_str_borrowed("k: 1\n").unwrap();
    /// assert!(v.as_mapping().is_some());
    /// ```
    #[must_use]
    pub fn as_mapping(&self) -> Option<&IndexMap<Cow<'a, str>, Self, FxBuildHasher>> {
        match self {
            Self::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Query nested values through a prevalidated [`QueryPath`].
    #[must_use]
    pub fn query_path(&self, path: &QueryPath) -> Vec<&Self> {
        let mut results = Vec::new();
        borrowed_query_recursive(self, path.segments(), 0, &mut results);
        results
    }

    /// Query nested values using an extended path expression.
    ///
    /// Returns all matching values. Supports dot notation, bracket indexing,
    /// bracket-quoted keys (`labels["app.kubernetes.io/name"]`, see
    /// [`crate::path`]), wildcards (`*`), and recursive descent (`..`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::borrowed::from_str_borrowed;
    ///
    /// let yaml = "items:\n  - name: a\n  - name: b\n";
    /// let v = from_str_borrowed(yaml).unwrap();
    /// let names = v.query("items[*].name");
    /// assert_eq!(names.len(), 2);
    /// ```
    #[must_use]
    pub fn query(&self, path: &str) -> Vec<&Self> {
        let segments = parse_query_path(path);
        let mut results = Vec::new();
        borrowed_query_recursive(self, &segments, 0, &mut results);
        results
    }

    /// Access a nested value through a prevalidated [`QueryPath`].
    #[must_use]
    pub fn get_query_path(&self, path: &QueryPath) -> Option<&Self> {
        let mut current = self;
        for seg in path.segments() {
            current = match seg {
                QuerySegment::Key(key) => {
                    if let Self::Mapping(m) = current {
                        m.get(key.as_str())?
                    } else {
                        return None;
                    }
                }
                QuerySegment::Index(idx) => {
                    if let Self::Sequence(s) = current {
                        s.get(*idx)?
                    } else {
                        return None;
                    }
                }
                QuerySegment::Wildcard | QuerySegment::RecursiveDescent => {
                    return self.query_path(path).into_iter().next();
                }
            };
        }
        Some(current)
    }

    /// Access a nested value via a dotted path.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::from_str_borrowed;
    /// let v = from_str_borrowed("a:\n  b: 2\n").unwrap();
    /// assert_eq!(v.get_path("a.b").unwrap().as_i64(), Some(2));
    /// ```
    #[must_use]
    pub fn get_path(&self, path: &str) -> Option<&Self> {
        let segments = parse_query_path(path);
        let mut current = self;
        for seg in &segments {
            current = match seg {
                QuerySegment::Key(key) => {
                    if let Self::Mapping(m) = current {
                        m.get(key.as_str())?
                    } else {
                        return None;
                    }
                }
                QuerySegment::Index(idx) => {
                    if let Self::Sequence(s) = current {
                        s.get(*idx)?
                    } else {
                        return None;
                    }
                }
                QuerySegment::Wildcard | QuerySegment::RecursiveDescent => {
                    return self.query(path).into_iter().next();
                }
            };
        }
        Some(current)
    }

    /// Convert to an owned `Value`, cloning all borrowed strings.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::from_str_borrowed;
    /// let v = from_str_borrowed("k: 1\n").unwrap();
    /// let owned = v.into_owned();
    /// assert!(owned.as_mapping().is_some());
    /// ```
    #[must_use]
    pub fn into_owned(self) -> crate::Value {
        match self {
            Self::Null => crate::Value::Null,
            Self::Bool(b) => crate::Value::Bool(b),
            Self::Number(n) => crate::Value::Number(n),
            Self::String(s) => crate::Value::String(s.into_owned()),
            Self::Sequence(seq) => {
                crate::Value::Sequence(seq.into_iter().map(|v| v.into_owned()).collect())
            }
            Self::Mapping(map) => {
                let mut m = crate::Mapping::with_capacity(map.len());
                for (k, v) in map {
                    let _ = m.insert(k.into_owned(), v.into_owned());
                }
                crate::Value::Mapping(m)
            }
        }
    }
}

impl Hash for BorrowedValue<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Null => {}
            Self::Bool(b) => b.hash(state),
            Self::Number(n) => n.hash(state),
            Self::String(s) => s.hash(state),
            Self::Sequence(seq) => seq.hash(state),
            Self::Mapping(map) => {
                state.write_usize(map.len());
                for (k, v) in map {
                    k.hash(state);
                    v.hash(state);
                }
            }
        }
    }
}

impl serde_core::Serialize for BorrowedValue<'_> {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Number(n) => match n {
                crate::value::Number::Integer(i) => serializer.serialize_i64(*i),
                #[cfg(feature = "lossless-u64")]
                crate::value::Number::Unsigned(u) => serializer.serialize_u64(*u),
                crate::value::Number::Float(f) => serializer.serialize_f64(*f),
            },
            Self::String(s) => serializer.serialize_str(s),
            Self::Sequence(seq) => seq.serialize(serializer),
            Self::Mapping(map) => {
                use serde_core::ser::SerializeMap as _;
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    m.serialize_entry(k.as_ref(), v)?;
                }
                m.end()
            }
        }
    }
}

impl PartialOrd for BorrowedValue<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BorrowedValue<'_> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering;
        let rank = |v: &Self| -> u8 {
            match v {
                Self::Null => 0,
                Self::Bool(_) => 1,
                Self::Number(_) => 2,
                Self::String(_) => 3,
                Self::Sequence(_) => 4,
                Self::Mapping(_) => 5,
            }
        };
        let r = rank(self).cmp(&rank(other));
        if r != Ordering::Equal {
            return r;
        }
        match (self, other) {
            (Self::Null, Self::Null) => Ordering::Equal,
            (Self::Bool(a), Self::Bool(b)) => a.cmp(b),
            (Self::Number(a), Self::Number(b)) => a.cmp(b),
            (Self::String(a), Self::String(b)) => a.cmp(b),
            (Self::Sequence(a), Self::Sequence(b)) => a.cmp(b),
            (Self::Mapping(a), Self::Mapping(b)) => a.len().cmp(&b.len()),
            _ => Ordering::Equal,
        }
    }
}

/// Recursively query a `BorrowedValue` tree.
fn borrowed_query_recursive<'a, 'b>(
    value: &'b BorrowedValue<'a>,
    segments: &[QuerySegment],
    depth: usize,
    results: &mut Vec<&'b BorrowedValue<'a>>,
) {
    if depth >= segments.len() {
        results.push(value);
        return;
    }
    match &segments[depth] {
        QuerySegment::Key(key) => {
            if let BorrowedValue::Mapping(m) = value {
                if let Some(child) = m.get(key.as_str()) {
                    borrowed_query_recursive(child, segments, depth + 1, results);
                }
            }
        }
        QuerySegment::Index(idx) => {
            if let BorrowedValue::Sequence(s) = value {
                if let Some(child) = s.get(*idx) {
                    borrowed_query_recursive(child, segments, depth + 1, results);
                }
            }
        }
        QuerySegment::Wildcard => match value {
            BorrowedValue::Sequence(seq) => {
                for item in seq {
                    borrowed_query_recursive(item, segments, depth + 1, results);
                }
            }
            BorrowedValue::Mapping(map) => {
                for (_, v) in map {
                    borrowed_query_recursive(v, segments, depth + 1, results);
                }
            }
            _ => {}
        },
        QuerySegment::RecursiveDescent => {
            let remaining = &segments[depth + 1..];
            if !remaining.is_empty() {
                borrowed_query_recursive(value, segments, depth + 1, results);
                match value {
                    BorrowedValue::Sequence(seq) => {
                        for item in seq {
                            borrowed_query_recursive(item, segments, depth, results);
                        }
                    }
                    BorrowedValue::Mapping(map) => {
                        for (_, v) in map {
                            borrowed_query_recursive(v, segments, depth, results);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Parse YAML into a zero-copy `BorrowedValue` that borrows from the input.
///
/// This is significantly faster than `from_str::<Value>` for large documents
/// because string scalars and mapping keys borrow directly from the input
/// buffer instead of allocating on the heap.
///
/// # Examples
///
/// ```rust
/// use noyalib::borrowed::{from_str_borrowed, BorrowedValue};
///
/// let yaml = "host: localhost\nport: 8080\n";
/// let value = from_str_borrowed(yaml).unwrap();
/// assert_eq!(value.as_mapping().unwrap().get("host").unwrap().as_str(), Some("localhost"));
/// ```
pub fn from_str_borrowed(input: &str) -> Result<BorrowedValue<'_>> {
    from_str_borrowed_with_config(input, &crate::ParserConfig::default())
}

/// Parse YAML into a zero-copy `BorrowedValue` with custom security limits.
///
/// Same as [`from_str_borrowed`] but accepts a [`crate::ParserConfig`]
/// so callers can tighten `max_document_length`, `max_depth`, and other
/// limits for untrusted input.
///
/// # Errors
///
/// Returns an error when the input exceeds `max_document_length`, when
/// the parser encounters invalid YAML, or when alias expansion exceeds
/// [`ParserConfig::max_alias_expansions`](crate::ParserConfig::max_alias_expansions).
///
/// # Aliases
///
/// Anchors (`&name`) and aliases (`*name`) are eagerly resolved on the
/// borrowed path. The anchored value is stored in a side-table keyed
/// by name; each alias clones the value into the tree (string fields
/// stay `Cow::Borrowed`, so the clone is mostly free — only sequences
/// and mappings actually duplicate). Total expansions are bounded by
/// `max_alias_expansions` to neutralise YAML bombs.
///
/// # Examples
///
/// ```
/// use noyalib::{borrowed::from_str_borrowed_with_config, ParserConfig};
/// let cfg = ParserConfig::strict();
/// let v = from_str_borrowed_with_config("k: 1\n", &cfg).unwrap();
/// assert!(v.as_mapping().is_some());
/// ```
pub fn from_str_borrowed_with_config<'a>(
    input: &'a str,
    user_config: &crate::ParserConfig,
) -> Result<BorrowedValue<'a>> {
    let config = ParseConfig::from(user_config);
    if input.len() > config.max_document_length {
        return Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            config.max_document_length
        )));
    }

    let mut parser = Parser::new(input);
    let mut builder = BorrowedBuilder::new(&config);

    loop {
        let event = parser
            .next_event()
            .map_err(|e| Error::parse_at(&*e.message, input, e.index))?;
        if matches!(event, Event::StreamEnd) {
            break;
        }
        builder.process(event)?;
    }

    Ok(builder.into_value())
}

enum Frame<'a> {
    Sequence(Vec<BorrowedValue<'a>>, Option<String>),
    MappingKey(
        IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>,
        Option<String>,
    ),
    MappingValue(
        IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>,
        Cow<'a, str>,
        Option<String>,
    ),
}

/// The plain `<<` key that the owned loaders read as a merge.
const MERGE_KEY: &str = "<<";

struct BorrowedBuilder<'a, 'c> {
    stack: Vec<Frame<'a>>,
    result: Option<BorrowedValue<'a>>,
    config: &'c ParseConfig,
    depth: usize,
    /// Anchor → value table, with the measured cost of expanding each.
    /// Eager resolution: an `Alias` event clones the anchored value into
    /// the tree (string fields stay `Cow::Borrowed`, so only sequences
    /// and mappings duplicate), after the cost is charged.
    anchors: FxHashMap<String, (BorrowedValue<'a>, AliasCost)>,
    /// The budget meter the owned loaders and the streaming
    /// deserializer charge, so every limit holds here too.
    meter: Meter,
}

impl<'a, 'c> BorrowedBuilder<'a, 'c> {
    fn new(config: &'c ParseConfig) -> Self {
        Self {
            stack: Vec::new(),
            result: None,
            config,
            depth: 0,
            anchors: FxHashMap::default(),
            meter: Meter::default(),
        }
    }

    fn into_value(self) -> BorrowedValue<'a> {
        self.result.unwrap_or(BorrowedValue::Null)
    }

    fn resolve_scalar(
        &self,
        value: Cow<'a, str>,
        style: ScalarStyle,
        tag: Option<&(String, String)>,
    ) -> BorrowedValue<'a> {
        // `!!str` is a resolution tag, not a decoration: it says the
        // scalar *is* a string, so plain resolution must not run and
        // turn `!!str 1` into the integer 1. The owned graph honours it
        // (`resolve_tagged_scalar`'s "str" arm); this one used to
        // discard the tag entirely and disagree.
        //
        // The other core tags (`int`, `float`, `bool`, `null`) already
        // land on the same answer through plain resolution for every
        // well-formed payload, and `BorrowedValue` has no `Tagged`
        // variant for a custom tag to live in — both are recorded in
        // `differential_readers.rs` rather than silently differing.
        if is_core_string_tag(tag) || style != ScalarStyle::Plain {
            return BorrowedValue::String(value);
        }
        let c = self.config;
        match crate::streaming::resolve_plain_ext(
            &value,
            c.strict_booleans,
            c.legacy_booleans,
            c.no_schema,
            c.legacy_octal_numbers,
            c.legacy_sexagesimal,
            c.lossless_u64_integers(),
        ) {
            crate::streaming::Scalar::Null => BorrowedValue::Null,
            crate::streaming::Scalar::Bool(b) => BorrowedValue::Bool(b),
            crate::streaming::Scalar::Int(i) => {
                BorrowedValue::Number(crate::value::Number::Integer(i))
            }
            #[cfg(feature = "lossless-u64")]
            crate::streaming::Scalar::Uint(u) => {
                BorrowedValue::Number(crate::value::Number::Unsigned(u))
            }
            crate::streaming::Scalar::Float(f) => {
                BorrowedValue::Number(crate::value::Number::Float(f))
            }
            crate::streaming::Scalar::Str(_) => BorrowedValue::String(value),
        }
    }

    /// Place a finished value: into the open sequence or mapping, under
    /// the same length, key-count and duplicate-key rules the owned
    /// loaders apply, or as the document's root.
    fn push_value(&mut self, value: BorrowedValue<'a>) -> Result<()> {
        match self.stack.pop() {
            Some(Frame::Sequence(mut seq, anchor)) => {
                if seq.len() >= self.config.max_sequence_length {
                    return Err(Error::Budget(crate::BudgetBreach::MaxSequenceLength {
                        limit: self.config.max_sequence_length,
                        observed: seq.len() + 1,
                    }));
                }
                seq.push(value);
                self.stack.push(Frame::Sequence(seq, anchor));
            }
            Some(Frame::MappingValue(mut map, key, anchor)) => {
                self.insert_entry(&mut map, key, value)?;
                self.stack.push(Frame::MappingKey(map, anchor));
            }
            // A key arrives through `set_key`, never here; keep the frame.
            Some(frame @ Frame::MappingKey(..)) => self.stack.push(frame),
            None => self.result = Some(value),
        }
        Ok(())
    }

    fn insert_entry(
        &self,
        map: &mut IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>,
        key: Cow<'a, str>,
        value: BorrowedValue<'a>,
    ) -> Result<()> {
        if map.len() >= self.config.max_mapping_keys {
            return Err(Error::Budget(crate::BudgetBreach::MaxMappingKeys {
                limit: self.config.max_mapping_keys,
                observed: map.len() + 1,
            }));
        }
        match self.config.duplicate_key_policy {
            InternalDuplicateKeyPolicy::First if map.contains_key(&key) => {}
            InternalDuplicateKeyPolicy::Error if map.contains_key(&key) => {
                return Err(Error::DuplicateKey(key.into_owned()));
            }
            _ => {
                let _ = map.insert(key, value);
            }
        }
        Ok(())
    }

    /// Turn the open mapping's key state into its value state.
    fn set_key(&mut self, key: Cow<'a, str>) {
        if let Some(Frame::MappingKey(map, anchor)) = self.stack.pop() {
            self.stack.push(Frame::MappingValue(map, key, anchor));
        }
    }

    /// Register `value` under `anchor` so a later `*anchor` event can
    /// resolve to a clone of it. No-op when `anchor` is `None`.
    fn record_anchor(&mut self, anchor: Option<String>, value: &BorrowedValue<'a>) {
        if let Some(name) = anchor {
            let cost = borrowed_cost(value);
            let _ = self.anchors.insert(name, (value.clone(), cost));
        }
    }

    fn process(&mut self, event: Event<'a>) -> Result<()> {
        self.meter.charge_event(&event, self.config)?;
        match event {
            Event::DocumentEnd => {
                // Per YAML spec each document has its own anchor
                // namespace. Reset between documents to match.
                self.anchors.clear();
                Ok(())
            }
            Event::Scalar {
                value,
                style,
                anchor,
                tag,
                ..
            } => self.on_scalar(value, style, anchor, tag.as_ref()),
            Event::SequenceStart { anchor, .. } => {
                self.open_collection()?;
                self.stack
                    .push(Frame::Sequence(Vec::with_capacity(4), anchor));
                Ok(())
            }
            Event::MappingStart { anchor, .. } => {
                self.open_collection()?;
                self.stack.push(Frame::MappingKey(
                    IndexMap::with_capacity_and_hasher(4, FxBuildHasher),
                    anchor,
                ));
                Ok(())
            }
            Event::SequenceEnd { .. } | Event::MappingEnd { .. } => self.close_collection(),
            Event::Alias { anchor, .. } => self.on_alias(&anchor),
            Event::StreamStart | Event::StreamEnd | Event::DocumentStart => Ok(()),
        }
    }

    fn on_scalar(
        &mut self,
        value: Cow<'a, str>,
        style: ScalarStyle,
        anchor: Option<String>,
        tag: Option<&(String, String)>,
    ) -> Result<()> {
        if let Some(Frame::MappingKey(..)) = self.stack.last() {
            if style == ScalarStyle::Plain && value == MERGE_KEY {
                self.charge_merge_key()?;
            }
            self.set_key(value);
            return Ok(());
        }
        let resolved = self.resolve_scalar(value, style, tag);
        self.record_anchor(anchor, &resolved);
        self.push_value(resolved)
    }

    /// A plain `<<` key is a merge on the owned paths; the borrowed tree
    /// keeps it as an ordinary key, but it is charged and refused there
    /// exactly as the owned loaders do so the budget means one thing.
    fn charge_merge_key(&mut self) -> Result<()> {
        match self.config.merge_key_policy {
            InternalMergeKeyPolicy::AsOrdinary => Ok(()),
            InternalMergeKeyPolicy::Error => Err(Error::Custom(
                "merge key `<<` rejected by MergeKeyPolicy::Error".to_owned(),
            )),
            InternalMergeKeyPolicy::Auto => self.meter.charge_merge_key(self.config),
        }
    }

    fn open_collection(&mut self) -> Result<()> {
        self.depth += 1;
        if budget::depth_exceeded(self.depth, self.config.max_depth) {
            return Err(Error::RecursionLimitExceeded { depth: self.depth });
        }
        Ok(())
    }

    fn close_collection(&mut self) -> Result<()> {
        self.depth = self.depth.saturating_sub(1);
        let (value, anchor) = match self.stack.pop() {
            Some(Frame::Sequence(s, a)) => (BorrowedValue::Sequence(s), a),
            Some(Frame::MappingKey(m, a) | Frame::MappingValue(m, _, a)) => {
                (BorrowedValue::Mapping(m), a)
            }
            None => return Err(Error::Invalid("unexpected collection end".to_string())),
        };
        self.record_anchor(anchor, &value);
        self.push_value(value)
    }

    fn on_alias(&mut self, anchor: &str) -> Result<()> {
        self.meter.charge_alias(self.config)?;
        let (referent, cost) = self
            .anchors
            .get(anchor)
            .ok_or_else(|| Error::Parse(format!("unknown anchor: '{anchor}'")))?;
        self.meter.charge_expansion(cost, self.config)?;
        let referent = referent.clone();
        // Special-case: alias used as a mapping key. We need the
        // alias's resolved value to be a string for it to function as
        // one, mirroring how YAML 1.2 treats key aliases on the owned
        // path.
        if let Some(Frame::MappingKey(..)) = self.stack.last() {
            let key = alias_key(referent)?;
            self.set_key(key);
            return Ok(());
        }
        self.push_value(referent)
    }
}

/// The key an alias stands for when it is used as a mapping key. A
/// scalar renders as its text, matching the owned path's mapping-key
/// coercion; a collection cannot be a key here.
fn alias_key(referent: BorrowedValue<'_>) -> Result<Cow<'_, str>> {
    match referent {
        BorrowedValue::String(s) => Ok(s),
        BorrowedValue::Bool(b) => Ok(Cow::Owned(b.to_string())),
        BorrowedValue::Number(n) => Ok(Cow::Owned(n.to_string())),
        BorrowedValue::Null => Ok(Cow::Borrowed("null")),
        BorrowedValue::Sequence(_) | BorrowedValue::Mapping(_) => Err(Error::Invalid(
            "alias resolved to a non-scalar cannot be used as a mapping key".to_string(),
        )),
    }
}

/// The expansion cost of an anchored borrowed value, by the estimator
/// the owned loaders use. Walks without recursion.
fn borrowed_cost(root: &BorrowedValue<'_>) -> AliasCost {
    enum Step<'v, 'a> {
        Visit(&'v BorrowedValue<'a>),
        Close,
    }
    let mut tally = CostTally::default();
    let mut pending = vec![Step::Visit(root)];
    while let Some(step) = pending.pop() {
        let value = match step {
            Step::Visit(value) => value,
            Step::Close => {
                tally.close();
                continue;
            }
        };
        match value {
            BorrowedValue::String(s) => tally.scalar(s.len()),
            BorrowedValue::Sequence(items) => {
                tally.open();
                pending.push(Step::Close);
                pending.extend(items.iter().map(Step::Visit));
            }
            BorrowedValue::Mapping(map) => {
                tally.open();
                pending.push(Step::Close);
                for (key, item) in map {
                    tally.scalar(key.len());
                    pending.push(Step::Visit(item));
                }
            }
            BorrowedValue::Null | BorrowedValue::Bool(_) | BorrowedValue::Number(_) => {
                tally.scalar(0);
            }
        }
    }
    tally.finish()
}

/// Whether `tag` is YAML's core-schema string tag, in any of its
/// spellings: `!!str`, the verbatim `tag:yaml.org,2002:str`, or the
/// primary `!str`. Matches the owned loader's `resolve_tagged_scalar`.
fn is_core_string_tag(tag: Option<&(String, String)>) -> bool {
    tag.is_some_and(|(handle, suffix)| {
        suffix == "str" && (handle == "!!" || handle == "tag:yaml.org,2002:" || handle == "!")
    })
}
