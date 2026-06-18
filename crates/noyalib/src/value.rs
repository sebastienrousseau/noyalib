//! YAML value types.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

mod convert;
mod mapping;
mod number;
mod serde_impl;
pub use mapping::{Mapping, MappingAny};
pub use number::{Number, ParseNumberError};

/// A YAML sequence (array/list).
pub type Sequence = Vec<Value>;

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
use crate::path::{QuerySegment, parse_query_path};

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
/// Outcome of resolving a placeholder name.
///
/// Three-way return so [`expand_placeholders`] can distinguish a
/// genuinely missing key (which may then defer to a `:-default`
/// fallback) from a key whose own resolution errored.
#[cfg(feature = "std")]
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
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy)]
pub(crate) enum MissingAction {
    /// Substitute the empty string. Lossy/non-strict mode.
    Empty,
    /// Surface an error. Boolean is `true` to redact the
    /// placeholder name from the error message.
    Error(bool),
}

#[cfg(feature = "std")]
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
    if touched { Ok(Some(out)) } else { Ok(None) }
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
