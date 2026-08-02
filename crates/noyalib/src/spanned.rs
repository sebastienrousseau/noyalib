//! Source location tracking for deserialized values.
//!
//! `Spanned<T>` wraps a deserialized value and records the start/end
//! [`Location`](crate::Location) in the original YAML source.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::error::Location;

/// Sentinel struct name used by the deserializer to detect `Spanned<T>`.
pub(crate) const SPANNED_TYPE_NAME: &str = "$__noyalib_private_Spanned";

/// Field names for the virtual Spanned struct.
pub(crate) const SPANNED_FIELD_START_LINE: &str = "$__noyalib_start_line";
pub(crate) const SPANNED_FIELD_START_COLUMN: &str = "$__noyalib_start_column";
pub(crate) const SPANNED_FIELD_START_INDEX: &str = "$__noyalib_start_index";
pub(crate) const SPANNED_FIELD_END_LINE: &str = "$__noyalib_end_line";
pub(crate) const SPANNED_FIELD_END_COLUMN: &str = "$__noyalib_end_column";
pub(crate) const SPANNED_FIELD_END_INDEX: &str = "$__noyalib_end_index";
pub(crate) const SPANNED_FIELD_VALUE: &str = "$__noyalib_value";

pub(crate) const SPANNED_FIELDS: &[&str] = &[
    SPANNED_FIELD_START_LINE,
    SPANNED_FIELD_START_COLUMN,
    SPANNED_FIELD_START_INDEX,
    SPANNED_FIELD_END_LINE,
    SPANNED_FIELD_END_COLUMN,
    SPANNED_FIELD_END_INDEX,
    SPANNED_FIELD_VALUE,
];

/// A value annotated with source span information.
///
/// During serialization, `Spanned<T>` serializes transparently as `T`.
/// During deserialization from a YAML string via [`from_str`](crate::from_str)
/// or [`from_str_with_config`](crate::from_str_with_config), real source
/// locations are populated. When deserializing via
/// [`from_value`](crate::from_value), locations default to zero.
///
/// # Examples
///
/// ```rust
/// use noyalib::Spanned;
///
/// #[derive(serde::Serialize, serde::Deserialize, Debug)]
/// struct Config {
///     port: Spanned<u16>,
/// }
/// ```
///
/// # Limitation: `#[serde(flatten)]`
///
/// `Spanned<T>` cannot be the target of a `#[serde(flatten)]` field. The
/// limitation is in serde itself: `FlatMapDeserializer::deserialize_struct`
/// uses `FlatStructAccess`, which filters the residue by the FIELDS list
/// passed to `deserialize_struct`. Because `Spanned` advertises internal
/// magic field names (`$__noyalib_value` and friends) that never appear in
/// real input, every residue entry is filtered out before our visitor
/// runs.
///
/// The supported pattern is to flatten a bare [`crate::Value`] (which works
/// today) and look up source spans through the [`crate::cst::Document`]
/// API separately:
///
/// ```rust,ignore
/// // Works.
/// #[derive(serde::Deserialize)]
/// struct Config {
///     name: String,
///     #[serde(flatten)]
///     extra: noyalib::Value,
/// }
///
/// // Errors with a clear message at deserialize time.
/// #[derive(serde::Deserialize)]
/// struct AlsoConfig {
///     name: String,
///     #[serde(flatten)]
///     extra: noyalib::Spanned<noyalib::Value>,  // not supported
/// }
/// ```
///
/// If the use case is "agent edits YAML, also wants source position of
/// each unflattened field", parse the source via
/// [`crate::cst::parse_document`] and use
/// [`crate::cst::Document::span_at`] / [`crate::cst::Document::comments_at`]
/// — those resolve byte ranges by path, work post-edit, and are not
/// constrained by serde's flatten mechanics.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    /// The deserialized value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Spanned;
    /// let s = Spanned::new("x".to_string());
    /// assert_eq!(s.value, "x");
    /// ```
    pub value: T,
    /// Start location in the source.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Spanned;
    /// let s: Spanned<i32> = Spanned::new(1);
    /// assert_eq!(s.start.line(), 0);
    /// ```
    pub start: Location,
    /// End location in the source.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Spanned;
    /// let s: Spanned<i32> = Spanned::new(1);
    /// assert_eq!(s.end.line(), 0);
    /// ```
    pub end: Location,
}

impl<T: fmt::Debug> fmt::Debug for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Spanned")
            .field("value", &self.value)
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> Spanned<T> {
    /// Create a new spanned value with sentinel zero locations
    /// (`line = 0`, `column = 0`, `index = 0`).
    ///
    /// The sentinel form exists so callers can synthesise a
    /// [`Spanned<T>`] without owning a parser context; it is
    /// distinguishable from a parsed location because parsed
    /// locations always have `line >= 1` and `column >= 1`. Use
    /// the deserialisation path (`#[derive(Deserialize)]` with a
    /// `Spanned<T>` field) when authentic source positions are
    /// required.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Spanned;
    /// let s = Spanned::new(42_u16);
    /// assert_eq!(*s, 42);
    /// // Sentinel — distinguishable from a parsed location.
    /// assert_eq!(s.start.line(), 0);
    /// ```
    pub fn new(value: T) -> Self {
        Self {
            value,
            start: Location::default(),
            end: Location::default(),
        }
    }

    /// Unwrap into the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Spanned;
    /// let s = Spanned::new("hello".to_string());
    /// assert_eq!(s.into_inner(), "hello");
    /// ```
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> From<T> for Spanned<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Serialize> Serialize for Spanned<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        // Transparent: just serialize the inner value
        self.value.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Spanned<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            SPANNED_TYPE_NAME,
            SPANNED_FIELDS,
            SpannedVisitor(core::marker::PhantomData),
        )
    }
}

struct SpannedVisitor<T>(core::marker::PhantomData<T>);

impl<'de, T: Deserialize<'de>> serde_core::de::Visitor<'de> for SpannedVisitor<T> {
    type Value = Spanned<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a spanned value")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde_core::de::MapAccess<'de>,
    {
        let mut start_line: Option<usize> = None;
        let mut start_column: Option<usize> = None;
        let mut start_index: Option<usize> = None;
        let mut end_line: Option<usize> = None;
        let mut end_column: Option<usize> = None;
        let mut end_index: Option<usize> = None;
        let mut value: Option<T> = None;
        // We track whether *any* keys were yielded so we can produce
        // a more helpful error in the `#[serde(flatten)]` corner —
        // serde's `FlatStructAccess` filters residue entries by the
        // FIELDS list passed to `deserialize_struct`, and our magic
        // SPANNED_FIELDS never match the real residue keys, so the
        // visitor sees zero entries. The bare `missing_field` error
        // is unhelpful in that case; we want to point the user at
        // the workaround.
        let mut saw_any_key = false;

        while let Some(key) = map.next_key::<&str>()? {
            saw_any_key = true;
            match key {
                SPANNED_FIELD_START_LINE => start_line = Some(map.next_value()?),
                SPANNED_FIELD_START_COLUMN => start_column = Some(map.next_value()?),
                SPANNED_FIELD_START_INDEX => start_index = Some(map.next_value()?),
                SPANNED_FIELD_END_LINE => end_line = Some(map.next_value()?),
                SPANNED_FIELD_END_COLUMN => end_column = Some(map.next_value()?),
                SPANNED_FIELD_END_INDEX => end_index = Some(map.next_value()?),
                SPANNED_FIELD_VALUE => value = Some(map.next_value()?),
                _ => {
                    // Unknown field — skip
                    let _ = map.next_value::<serde_core::de::IgnoredAny>()?;
                }
            }
        }

        let value = value.ok_or_else(|| {
            if saw_any_key {
                serde_core::de::Error::missing_field(SPANNED_FIELD_VALUE)
            } else {
                serde_core::de::Error::custom(
                    "Spanned<T> can not be deserialized via `#[serde(flatten)]` — \
                     serde's FlatStructAccess filters residue entries by the field \
                     name list, and Spanned uses internal magic field names that \
                     never match real residue keys. Use a bare `Value` for the \
                     flatten target and look the span up separately via \
                     `Document::span_at`/`comments_at`.",
                )
            }
        })?;

        let start = Location::new(
            start_line.unwrap_or(0),
            start_column.unwrap_or(0),
            start_index.unwrap_or(0),
        );
        let end = Location::new(
            end_line.unwrap_or(0),
            end_column.unwrap_or(0),
            end_index.unwrap_or(0),
        );

        Ok(Spanned { value, start, end })
    }
}
