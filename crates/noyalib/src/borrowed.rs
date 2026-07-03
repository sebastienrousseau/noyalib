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
use crate::parser::{Event, ParseConfig, Parser, ScalarStyle};
use crate::path::{QuerySegment, parse_query_path};
use crate::prelude::*;
use core::hash::{Hash, Hasher};
use indexmap::IndexMap;
use rustc_hash::{FxBuildHasher, FxHashMap};
use serde::Serialize;

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
    Sequence(Vec<BorrowedValue<'a>>),
    /// YAML mapping with borrowed keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::borrowed::{from_str_borrowed, BorrowedValue};
    /// let v = from_str_borrowed("k: 1\n").unwrap();
    /// assert!(matches!(v, BorrowedValue::Mapping(_)));
    /// ```
    Mapping(IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>),
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
    pub fn as_sequence(&self) -> Option<&[BorrowedValue<'a>]> {
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
    pub fn as_mapping(&self) -> Option<&IndexMap<Cow<'a, str>, BorrowedValue<'a>, FxBuildHasher>> {
        match self {
            Self::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Query nested values using an extended path expression.
    ///
    /// Returns all matching values. Supports dot notation, bracket indexing,
    /// wildcards (`*`), and recursive descent (`..`).
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
    pub fn query(&self, path: &str) -> Vec<&BorrowedValue<'a>> {
        let segments = parse_query_path(path);
        let mut results = Vec::new();
        borrowed_query_recursive(self, &segments, 0, &mut results);
        results
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
    pub fn get_path(&self, path: &str) -> Option<&BorrowedValue<'a>> {
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

impl Serialize for BorrowedValue<'_> {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
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
                use serde::ser::SerializeMap;
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
        match builder.process(event, input)? {
            BuilderState::Continue => {}
            BuilderState::Done => break,
        }
    }

    Ok(builder.into_value())
}

enum BuilderState {
    Continue,
    Done,
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

struct BorrowedBuilder<'a> {
    stack: Vec<Frame<'a>>,
    result: Option<BorrowedValue<'a>>,
    max_depth: usize,
    depth: usize,
    in_document: bool,
    /// Anchor → value table. Eager resolution: when an `Alias`
    /// event arrives we clone the anchored value into the tree.
    /// String fields are `Cow::Borrowed` so a clone is mostly
    /// cheap — only sequences and mappings duplicate, and that
    /// matches the owned-`Value` path's behaviour.
    anchors: FxHashMap<String, BorrowedValue<'a>>,
    /// Cumulative count of aliases expanded so far. Capped by
    /// `max_alias_expansions` to neutralise YAML bomb / billion
    /// laughs payloads on the borrowed path the same way the
    /// owned path does.
    alias_expansions: usize,
    max_alias_expansions: usize,
    strict_booleans: bool,
    legacy_booleans: bool,
    no_schema: bool,
    legacy_octal_numbers: bool,
    legacy_sexagesimal: bool,
    lossless_u64_integers: bool,
}

impl<'a> BorrowedBuilder<'a> {
    fn new(config: &ParseConfig) -> Self {
        Self {
            stack: Vec::new(),
            result: None,
            max_depth: config.max_depth,
            depth: 0,
            in_document: false,
            anchors: FxHashMap::default(),
            alias_expansions: 0,
            max_alias_expansions: config.max_alias_expansions,
            strict_booleans: config.strict_booleans,
            legacy_booleans: config.legacy_booleans,
            no_schema: config.no_schema,
            legacy_octal_numbers: config.legacy_octal_numbers,
            legacy_sexagesimal: config.legacy_sexagesimal,
            lossless_u64_integers: config.lossless_u64_integers(),
        }
    }

    fn into_value(self) -> BorrowedValue<'a> {
        self.result.unwrap_or(BorrowedValue::Null)
    }

    fn resolve_scalar(&self, value: Cow<'a, str>, style: ScalarStyle) -> BorrowedValue<'a> {
        if style != ScalarStyle::Plain {
            return BorrowedValue::String(value);
        }

        match crate::streaming::resolve_plain_ext(
            &value,
            self.strict_booleans,
            self.legacy_booleans,
            self.no_schema,
            self.legacy_octal_numbers,
            self.legacy_sexagesimal,
            self.lossless_u64_integers,
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

    fn push_value(&mut self, value: BorrowedValue<'a>) {
        match self.stack.last_mut() {
            Some(Frame::Sequence(seq, _)) => seq.push(value),
            Some(Frame::MappingValue(map, key, _)) => {
                let k = core::mem::replace(key, Cow::Borrowed(""));
                let _ = map.insert(k, value);
                // Transition back to key state
                let (map, anchor) = match self.stack.pop() {
                    Some(Frame::MappingValue(m, _, a)) => (m, a),
                    _ => crate::error::invariant_violated(
                        "stack frame must be MappingValue immediately after value emit",
                    ),
                };
                self.stack.push(Frame::MappingKey(map, anchor));
            }
            Some(Frame::MappingKey(_, _)) => {
                // This shouldn't happen — keys should transition to MappingValue
            }
            None => {
                self.result = Some(value);
            }
        }
    }

    /// Register `value` under `anchor` so a later `*anchor` event can
    /// resolve to a clone of it. No-op when `anchor` is `None`.
    fn record_anchor(&mut self, anchor: Option<String>, value: &BorrowedValue<'a>) {
        if let Some(name) = anchor {
            let _ = self.anchors.insert(name, value.clone());
        }
    }

    fn process(&mut self, event: Event<'a>, _input: &str) -> Result<BuilderState> {
        match event {
            Event::StreamStart => Ok(BuilderState::Continue),
            Event::StreamEnd => Ok(BuilderState::Done),
            Event::DocumentStart => {
                self.in_document = true;
                Ok(BuilderState::Continue)
            }
            Event::DocumentEnd => {
                self.in_document = false;
                // Per YAML spec each document has its own anchor
                // namespace. Reset between documents to match.
                self.anchors.clear();
                Ok(BuilderState::Continue)
            }
            Event::Scalar {
                value,
                style,
                anchor,
                ..
            } => {
                // Check if this is a mapping key
                if let Some(Frame::MappingKey(_, _)) = self.stack.last_mut() {
                    let key = value;
                    let (map, frame_anchor) = match self.stack.pop() {
                        Some(Frame::MappingKey(m, a)) => (m, a),
                        _ => crate::error::invariant_violated(
                            "stack frame must be MappingKey when consuming a mapping key",
                        ),
                    };
                    self.stack.push(Frame::MappingValue(map, key, frame_anchor));
                    return Ok(BuilderState::Continue);
                }

                let resolved = self.resolve_scalar(value, style);
                self.record_anchor(anchor, &resolved);
                self.push_value(resolved);
                Ok(BuilderState::Continue)
            }
            Event::SequenceStart { anchor, .. } => {
                self.depth += 1;
                if self.depth > self.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack
                    .push(Frame::Sequence(Vec::with_capacity(4), anchor));
                Ok(BuilderState::Continue)
            }
            Event::SequenceEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                let (seq, anchor) = match self.stack.pop() {
                    Some(Frame::Sequence(s, a)) => (s, a),
                    _ => return Err(Error::Invalid("unexpected sequence end".to_string())),
                };
                let value = BorrowedValue::Sequence(seq);
                self.record_anchor(anchor, &value);
                self.push_value(value);
                Ok(BuilderState::Continue)
            }
            Event::MappingStart { anchor, .. } => {
                self.depth += 1;
                if self.depth > self.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(Frame::MappingKey(
                    IndexMap::with_capacity_and_hasher(4, FxBuildHasher),
                    anchor,
                ));
                Ok(BuilderState::Continue)
            }
            Event::MappingEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                let (map, anchor) = match self.stack.pop() {
                    Some(Frame::MappingKey(m, a)) => (m, a),
                    Some(Frame::MappingValue(m, _, a)) => (m, a),
                    _ => return Err(Error::Invalid("unexpected mapping end".to_string())),
                };
                let value = BorrowedValue::Mapping(map);
                self.record_anchor(anchor, &value);
                self.push_value(value);
                Ok(BuilderState::Continue)
            }
            Event::Alias { anchor, .. } => {
                // Bound expansion to neutralise YAML bombs the same
                // way the owned path does.
                self.alias_expansions += 1;
                if self.alias_expansions > self.max_alias_expansions {
                    return Err(Error::Parse(format!(
                        "alias expansions exceeded limit of {}",
                        self.max_alias_expansions
                    )));
                }
                let referent = self
                    .anchors
                    .get(&anchor)
                    .cloned()
                    .ok_or_else(|| Error::Parse(format!("unknown anchor: '{anchor}'")))?;
                // Special-case: alias used as a mapping key. We need
                // the alias's resolved value to be a string for it to
                // function as one, mirroring how YAML 1.2 treats key
                // aliases on the owned path.
                if let Some(Frame::MappingKey(_, _)) = self.stack.last_mut() {
                    let key = match referent {
                        BorrowedValue::String(s) => s,
                        // For any other shape, fall back to a debug
                        // rendering — matches the owned path's
                        // mapping-key coercion behaviour.
                        BorrowedValue::Bool(b) => Cow::Owned(b.to_string()),
                        BorrowedValue::Number(n) => Cow::Owned(n.to_string()),
                        BorrowedValue::Null => Cow::Borrowed("null"),
                        BorrowedValue::Sequence(_) | BorrowedValue::Mapping(_) => {
                            return Err(Error::Invalid(
                                "alias resolved to a non-scalar cannot be used as a mapping key"
                                    .to_string(),
                            ));
                        }
                    };
                    let (map, frame_anchor) = match self.stack.pop() {
                        Some(Frame::MappingKey(m, a)) => (m, a),
                        _ => crate::error::invariant_violated(
                            "stack frame must be MappingKey when consuming an alias key",
                        ),
                    };
                    self.stack.push(Frame::MappingValue(map, key, frame_anchor));
                    return Ok(BuilderState::Continue);
                }
                self.push_value(referent);
                Ok(BuilderState::Continue)
            }
        }
    }
}
