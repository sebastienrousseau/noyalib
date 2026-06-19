//! YAML tag types (`Tag`, `TaggedValue`) and tag utilities.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use super::Value;
use crate::prelude::*;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};

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
    /// The value. `pub(crate)` so the parent module's `Value` impls can
    /// move the inner value out (e.g. `Value::untag`).
    pub(crate) value: Box<Value>,
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
