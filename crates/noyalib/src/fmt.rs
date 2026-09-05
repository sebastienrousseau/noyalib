//! Formatting wrappers for fine-grained control over YAML output style.
//!
//! These wrappers let you control the YAML output style per-value rather than
//! globally via [`SerializerConfig`](crate::SerializerConfig). Each wrapper
//! serializes transparently during deserialization but emits style hints during
//! serialization.
//!
//! # Examples
//!
//! ```rust
//! use noyalib::fmt::{FlowSeq, LitString};
//!
//! #[derive(serde::Serialize, serde::Deserialize)]
//! struct Config {
//!     tags: FlowSeq<Vec<String>>,
//!     script: LitString,
//! }
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::ops::Deref;

// Magic names used as newtype struct sentinels.
// The serializer intercepts these to apply formatting hints.
pub(crate) const MAGIC_FLOW_SEQ: &str = "__noya_flow_seq";
pub(crate) const MAGIC_FLOW_MAP: &str = "__noya_flow_map";
pub(crate) const MAGIC_LIT_STR: &str = "__noya_lit_str";
pub(crate) const MAGIC_FOLD_STR: &str = "__noya_fold_str";
pub(crate) const MAGIC_COMMENTED: &str = "__noya_commented";
pub(crate) const MAGIC_SPACE_AFTER: &str = "__noya_space_after";
pub(crate) const MAGIC_ANCHOR_DEF: &str = "__noya_anchor_def";
pub(crate) const MAGIC_ANCHOR_REF: &str = "__noya_anchor_ref";
/// Newtype name `TaggedValue::serialize` wraps its single-entry-map wire
/// form in. A serializer with no tag concept (`serde_json`) passes a
/// newtype through and still sees `{"!tag": value}`; this crate's own
/// serializer recognises the name and rebuilds `Value::Tagged`, so a
/// genuine mapping whose only key starts with `!` is never mistaken for
/// a tag (#377).
pub(crate) const MAGIC_TAGGED: &str = "__noya_tagged";

/// Force flow style `[a, b, c]` for a sequence value.
///
/// # Examples
///
/// ```
/// use noyalib::{to_string, FlowSeq};
/// #[derive(serde::Serialize)]
/// struct Doc { v: FlowSeq<Vec<i32>> }
/// let yaml = to_string(&Doc { v: FlowSeq(vec![1, 2]) }).unwrap();
/// assert!(yaml.contains("["));
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FlowSeq<T>(pub T);

impl<T: fmt::Debug> fmt::Debug for FlowSeq<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FlowSeq").field(&self.0).finish()
    }
}

impl<T> Deref for FlowSeq<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> From<T> for FlowSeq<T> {
    fn from(v: T) -> Self {
        Self(v)
    }
}

impl<T> FlowSeq<T> {
    /// Unwrap into the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::FlowSeq;
    /// let f = FlowSeq(vec![1, 2, 3]);
    /// assert_eq!(f.into_inner(), vec![1, 2, 3]);
    /// ```
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: serde_core::Serialize> serde_core::Serialize for FlowSeq<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        serializer.serialize_newtype_struct(MAGIC_FLOW_SEQ, &self.0)
    }
}

impl<'de, T: serde_core::Deserialize<'de>> serde_core::Deserialize<'de> for FlowSeq<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(FlowSeq)
    }
}

/// Force flow style `{k: v, ...}` for a mapping value.
///
/// # Examples
///
/// ```
/// use noyalib::{to_string, FlowMap};
/// use std::collections::BTreeMap;
/// #[derive(serde::Serialize)]
/// struct Doc { m: FlowMap<BTreeMap<String, i32>> }
/// let mut m = BTreeMap::new();
/// let _ = m.insert("a".into(), 1);
/// let yaml = to_string(&Doc { m: FlowMap(m) }).unwrap();
/// assert!(yaml.contains("{"));
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FlowMap<T>(pub T);

impl<T: fmt::Debug> fmt::Debug for FlowMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FlowMap").field(&self.0).finish()
    }
}

impl<T> Deref for FlowMap<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> From<T> for FlowMap<T> {
    fn from(v: T) -> Self {
        Self(v)
    }
}

impl<T> FlowMap<T> {
    /// Unwrap into the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::FlowMap;
    /// use std::collections::BTreeMap;
    /// let mut m = BTreeMap::new();
    /// let _ = m.insert("k".to_string(), 1);
    /// let f = FlowMap(m);
    /// assert_eq!(f.into_inner().len(), 1);
    /// ```
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: serde_core::Serialize> serde_core::Serialize for FlowMap<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        serializer.serialize_newtype_struct(MAGIC_FLOW_MAP, &self.0)
    }
}

impl<'de, T: serde_core::Deserialize<'de>> serde_core::Deserialize<'de> for FlowMap<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(FlowMap)
    }
}

/// Force literal block scalar `|` style for a borrowed string.
///
/// # Examples
///
/// ```
/// use noyalib::fmt::LitStr;
/// let s = LitStr("line1\nline2\n");
/// assert_eq!(s.as_str(), "line1\nline2\n");
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LitStr<'a>(pub &'a str);

impl fmt::Debug for LitStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LitStr").field(&self.0).finish()
    }
}

impl Deref for LitStr<'_> {
    type Target = str;
    fn deref(&self) -> &str {
        self.0
    }
}

impl<'a> From<&'a str> for LitStr<'a> {
    fn from(v: &'a str) -> Self {
        Self(v)
    }
}

impl<'a> LitStr<'a> {
    /// Get the inner string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::fmt::LitStr;
    /// let s = LitStr("hello");
    /// assert_eq!(s.as_str(), "hello");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0
    }

    /// Unwrap into the inner string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::fmt::LitStr;
    /// let s = LitStr("hello");
    /// assert_eq!(s.into_inner(), "hello");
    /// ```
    #[must_use]
    pub fn into_inner(self) -> &'a str {
        self.0
    }
}

impl serde_core::Serialize for LitStr<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        serializer.serialize_newtype_struct(MAGIC_LIT_STR, self.0)
    }
}

/// Force literal block scalar `|` style for an owned string.
///
/// # Examples
///
/// ```
/// use noyalib::LitString;
/// let s = LitString("line1\nline2\n".to_string());
/// assert_eq!(&*s, "line1\nline2\n");
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LitString(pub String);

impl fmt::Debug for LitString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LitString").field(&self.0).finish()
    }
}

impl Deref for LitString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<String> for LitString {
    fn from(v: String) -> Self {
        Self(v)
    }
}

impl From<&str> for LitString {
    fn from(v: &str) -> Self {
        Self(v.to_owned())
    }
}

impl LitString {
    /// Unwrap into the inner `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::LitString;
    /// let s = LitString("hi".into());
    /// assert_eq!(s.into_inner(), "hi");
    /// ```
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl serde_core::Serialize for LitString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        serializer.serialize_newtype_struct(MAGIC_LIT_STR, &self.0)
    }
}

impl<'de> serde_core::Deserialize<'de> for LitString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(LitString)
    }
}

/// Force folded block scalar `>` style for a borrowed string.
///
/// # Examples
///
/// ```
/// use noyalib::fmt::FoldStr;
/// let s = FoldStr("line1\n\nline2");
/// assert_eq!(s.as_str(), "line1\n\nline2");
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FoldStr<'a>(pub &'a str);

impl fmt::Debug for FoldStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FoldStr").field(&self.0).finish()
    }
}

impl Deref for FoldStr<'_> {
    type Target = str;
    fn deref(&self) -> &str {
        self.0
    }
}

impl<'a> From<&'a str> for FoldStr<'a> {
    fn from(v: &'a str) -> Self {
        Self(v)
    }
}

impl<'a> FoldStr<'a> {
    /// Get the inner string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::fmt::FoldStr;
    /// assert_eq!(FoldStr("x").as_str(), "x");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0
    }

    /// Unwrap into the inner string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::fmt::FoldStr;
    /// assert_eq!(FoldStr("x").into_inner(), "x");
    /// ```
    #[must_use]
    pub fn into_inner(self) -> &'a str {
        self.0
    }
}

impl serde_core::Serialize for FoldStr<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        serializer.serialize_newtype_struct(MAGIC_FOLD_STR, self.0)
    }
}

/// Force folded block scalar `>` style for an owned string.
///
/// # Examples
///
/// ```
/// use noyalib::FoldString;
/// let s = FoldString("para one\n\npara two".to_string());
/// assert_eq!(&*s, "para one\n\npara two");
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FoldString(pub String);

impl fmt::Debug for FoldString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FoldString").field(&self.0).finish()
    }
}

impl Deref for FoldString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<String> for FoldString {
    fn from(v: String) -> Self {
        Self(v)
    }
}

impl From<&str> for FoldString {
    fn from(v: &str) -> Self {
        Self(v.to_owned())
    }
}

impl FoldString {
    /// Unwrap into the inner `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::FoldString;
    /// assert_eq!(FoldString("x".into()).into_inner(), "x");
    /// ```
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl serde_core::Serialize for FoldString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        serializer.serialize_newtype_struct(MAGIC_FOLD_STR, &self.0)
    }
}

impl<'de> serde_core::Deserialize<'de> for FoldString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(FoldString)
    }
}

/// Attach an inline YAML comment `# ...` after a value.
///
/// The comment text should not include the `#` prefix.
///
/// **Note:** Comments are serialization-only metadata. When deserializing,
/// the `comment` field is always empty because YAML comments are not
/// part of the data model and cannot survive a roundtrip.
///
/// # Examples
///
/// ```
/// use noyalib::{to_string, Commented};
/// #[derive(serde::Serialize)]
/// struct Doc { v: Commented<i32> }
/// let yaml = to_string(&Doc { v: Commented::new(42, "meaning") }).unwrap();
/// assert!(yaml.contains("# meaning"));
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Commented<T> {
    /// The inner value.
    pub value: T,
    /// The comment text (without `#` prefix).
    pub comment: String,
}

impl<T: fmt::Debug> fmt::Debug for Commented<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Commented")
            .field("value", &self.value)
            .field("comment", &self.comment)
            .finish()
    }
}

impl<T> Commented<T> {
    /// Create a new commented value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Commented;
    /// let c = Commented::new(42, "the answer");
    /// assert_eq!(c.value, 42);
    /// assert_eq!(c.comment, "the answer");
    /// ```
    pub fn new(value: T, comment: impl Into<String>) -> Self {
        Self {
            value,
            comment: comment.into(),
        }
    }

    /// Unwrap into the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Commented;
    /// let c = Commented::new(7, "note");
    /// assert_eq!(c.into_inner(), 7);
    /// ```
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> Deref for Commented<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: serde_core::Serialize> serde_core::Serialize for Commented<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        use serde_core::ser::SerializeTuple as _;

        // Serialize as a tuple (value, comment) wrapped in the magic newtype
        struct Inner<'a, T>(&'a T, &'a str);

        impl<T: serde_core::Serialize> serde_core::Serialize for Inner<'_, T> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde_core::Serializer,
            {
                let mut tup = serializer.serialize_tuple(2)?;
                tup.serialize_element(self.0)?;
                tup.serialize_element(self.1)?;
                tup.end()
            }
        }

        serializer.serialize_newtype_struct(MAGIC_COMMENTED, &Inner(&self.value, &self.comment))
    }
}

impl<'de, T: serde_core::Deserialize<'de>> serde_core::Deserialize<'de> for Commented<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        // Note: comments are serialization-only metadata and cannot survive a
        // roundtrip through YAML. Deserializing always produces an empty comment.
        T::deserialize(deserializer).map(|v| Self {
            value: v,
            comment: String::new(),
        })
    }
}

/// Emit a blank line after the value.
///
/// # Examples
///
/// ```
/// use noyalib::SpaceAfter;
/// let s = SpaceAfter("section".to_string());
/// assert_eq!(s.0, "section");
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SpaceAfter<T>(pub T);

impl<T: fmt::Debug> fmt::Debug for SpaceAfter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SpaceAfter").field(&self.0).finish()
    }
}

impl<T> Deref for SpaceAfter<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> From<T> for SpaceAfter<T> {
    fn from(v: T) -> Self {
        Self(v)
    }
}

impl<T> SpaceAfter<T> {
    /// Unwrap into the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::SpaceAfter;
    /// assert_eq!(SpaceAfter("x".to_string()).into_inner(), "x");
    /// ```
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: serde_core::Serialize> serde_core::Serialize for SpaceAfter<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        serializer.serialize_newtype_struct(MAGIC_SPACE_AFTER, &self.0)
    }
}

impl<'de, T: serde_core::Deserialize<'de>> serde_core::Deserialize<'de> for SpaceAfter<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(SpaceAfter)
    }
}
