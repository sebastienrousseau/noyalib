//! The serde `Deserializer` over a `&Value` and its access types.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::error::{Error, Result};
use crate::prelude::*;
use crate::span_context;
use crate::value::{Number, Value};
use serde::Deserialize;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};

/// A YAML deserializer.
///
/// # Examples
///
/// ```
/// use noyalib::{Deserializer, Value};
/// use serde::Deserialize;
/// let v = Value::from(42_i64);
/// let de = Deserializer::new(&v);
/// let n: i32 = Deserialize::deserialize(de).unwrap();
/// assert_eq!(n, 42);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Deserializer<'de> {
    pub(crate) value: &'de Value,
    pub(crate) span_ctx: Option<&'de span_context::SpanContext>,
    /// Per-call flag mirroring
    /// [`ParserConfig::ignore_binary_tag_for_string`]. When `true`,
    /// `!!binary "ABCD"` deserializes into `String` as the literal
    /// `"ABCD"` (no base64 decode). Default `false` preserves YAML
    /// 1.2 semantics.
    pub(crate) ignore_binary_tag_for_string: bool,
    /// When `true`, [`Value::Tagged`] is surfaced through the
    /// magic-key [`crate::value::TagPreservingMapAccess`] so the
    /// outer [`Value::deserialize`] visitor can reconstruct
    /// `Value::Tagged(...)` losslessly. When `false` (default), a
    /// tagged scalar is unwrapped to its inner value — the
    /// transparent behaviour every typed `T::deserialize` expects.
    ///
    /// Set automatically by [`from_str_with_config`] /
    /// [`from_value`] when the caller's `T` is `Value` (detected
    /// via [`std::any::TypeId`]). Threaded through every
    /// `descend()` site so nested tagged values inside a
    /// `Mapping` / `Sequence` also survive.
    pub(crate) preserve_tags: bool,
}

impl<'de> Deserializer<'de> {
    /// Create a new deserializer from a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Deserializer, Value};
    /// let v = Value::from(1_i64);
    /// let _de = Deserializer::new(&v);
    /// ```
    #[must_use]
    pub fn new(value: &'de Value) -> Self {
        Deserializer {
            value,
            span_ctx: None,
            ignore_binary_tag_for_string: false,
            preserve_tags: false,
        }
    }

    /// Create a new deserializer from a value with an associated span context.
    ///
    /// The span context carries source-location information used to attach
    /// line/column details to errors and `Spanned<T>` fields. This
    /// constructor is primarily used internally by `from_str`; most callers
    /// should prefer [`Deserializer::new`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Constructed internally by from_str — external callers use Deserializer::new.
    /// use noyalib::Deserializer;
    /// # let value = unimplemented!();
    /// # let span_ctx = unimplemented!();
    /// let _de = Deserializer::with_span_context(value, span_ctx);
    /// ```
    #[must_use]
    pub fn with_span_context(value: &'de Value, span_ctx: &'de span_context::SpanContext) -> Self {
        Deserializer {
            value,
            span_ctx: Some(span_ctx),
            ignore_binary_tag_for_string: false,
            preserve_tags: false,
        }
    }

    /// Pass-through constructor for the
    /// [`crate::ParserConfig::ignore_binary_tag_for_string`] flag.
    /// Used internally by [`from_str_with_config`] when the caller
    /// has opted in to the migration helper.
    pub(crate) fn with_options(
        value: &'de Value,
        span_ctx: Option<&'de span_context::SpanContext>,
        ignore_binary_tag_for_string: bool,
    ) -> Self {
        Deserializer {
            value,
            span_ctx,
            ignore_binary_tag_for_string,
            preserve_tags: false,
        }
    }

    /// Internal constructor used by [`from_str_with_config`] /
    /// [`from_value`] when the caller's `T` is detected as
    /// [`Value`] via [`std::any::TypeId`]. Sets `preserve_tags`
    /// so [`Value::Tagged`] survives the data-binding return
    /// path. See `Deserializer::preserve_tags` for the contract.
    pub(crate) fn with_options_preserving_tags(
        value: &'de Value,
        span_ctx: Option<&'de span_context::SpanContext>,
        ignore_binary_tag_for_string: bool,
    ) -> Self {
        Deserializer {
            value,
            span_ctx,
            ignore_binary_tag_for_string,
            preserve_tags: true,
        }
    }

    /// Construct a child deserializer for `value`, propagating the
    /// span context and every per-call config toggle from `self`.
    /// Used by every descent site (struct field, sequence element,
    /// tagged inner value) so the toggles survive the walk.
    pub(crate) fn descend(&self, value: &'de Value) -> Self {
        Deserializer {
            value,
            span_ctx: self.span_ctx,
            ignore_binary_tag_for_string: self.ignore_binary_tag_for_string,
            preserve_tags: self.preserve_tags,
        }
    }

    fn wrap_err<T>(&self, res: Result<T>) -> Result<T> {
        match res {
            Err(Error::Deserialize(msg)) => {
                if let Some(ctx) = self.span_ctx {
                    let ptr: *const Value = self.value;
                    let addr = ptr as usize;
                    if let Some(span) = ctx.spans.get(&addr) {
                        return Err(Error::deserialize_at(msg, &ctx.source, span.0));
                    }
                }
                Err(Error::Deserialize(msg))
            }
            _ => res,
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => self.wrap_err(visitor.visit_none()),
            Value::Bool(b) => self.wrap_err(visitor.visit_bool(*b)),
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_i64(*n)),
            Value::Number(Number::Float(n)) => self.wrap_err(visitor.visit_f64(*n)),
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            Value::Sequence(_) => self.deserialize_seq(visitor),
            Value::Mapping(_) => self.deserialize_map(visitor),
            Value::Tagged(tagged) => {
                if self.preserve_tags {
                    // Tag-preserving path (`from_str::<Value>` /
                    // `from_value::<Value>`): surface the tag via
                    // a magic-key MapAccess so the outer
                    // `Value::deserialize` visitor reconstructs
                    // `Value::Tagged(...)` losslessly.
                    self.wrap_err(visitor.visit_map(crate::value::TagPreservingMapAccess::new(
                        tagged.tag().as_str(),
                        tagged.value(),
                    )))
                } else {
                    // Default path: typed targets see through the
                    // tag transparently — `#[derive(Deserialize)]
                    // struct Foo { x: i32 }` against `!Foo {x: 1}`
                    // yields `Foo { x: 1 }`.
                    let de = self.descend(tagged.value());
                    de.deserialize_any(visitor)
                }
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Bool(b) => self.wrap_err(visitor.visit_bool(*b)),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "bool",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_i64(*n)),
            Value::Number(Number::Float(n))
                if n.fract() == 0.0
                    && *n >= i64::MIN as f64
                    && *n <= i64::MAX as f64
                    && !n.is_nan() =>
            {
                self.wrap_err(visitor.visit_i64(*n as i64))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "integer",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(Number::Integer(n)) if *n >= 0 => {
                self.wrap_err(visitor.visit_u64(*n as u64))
            }
            Value::Number(Number::Float(n))
                if n.fract() == 0.0 && *n >= 0.0 && *n <= u64::MAX as f64 && !n.is_nan() =>
            {
                self.wrap_err(visitor.visit_u64(*n as u64))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "unsigned integer",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(Number::Float(n)) => self.wrap_err(visitor.visit_f64(*n)),
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_f64(*n as f64)),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "float",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) if s.chars().count() == 1 => {
                self.wrap_err(visitor.visit_char(s.chars().next().unwrap()))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "char",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            // Migration helper: when the source declared
            // `!!binary "ABCD"` and the caller opted in to
            // `ignore_binary_tag_for_string`, surface the literal
            // source string rather than rejecting on tag mismatch.
            // The base64 encoding stays as the user-facing value;
            // the application layer can decode (or not) as it
            // sees fit.
            Value::Tagged(boxed)
                if self.ignore_binary_tag_for_string && is_binary_tag(boxed.tag().as_str()) =>
            {
                match boxed.value() {
                    Value::String(s) => self.wrap_err(visitor.visit_str(s)),
                    other => self.wrap_err(Err(Error::TypeMismatch {
                        expected: "string-shaped !!binary content",
                        found: type_name(other),
                    })),
                }
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "string",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) => self.wrap_err(visitor.visit_bytes(s.as_bytes())),
            // YAML 1.2.2 §10.4: `!!binary` carries an RFC 4648
            // base64-encoded payload. Decode on demand when a serde
            // target asks for bytes / a byte buffer (Vec<u8>,
            // serde_bytes::ByteBuf, &[u8] via owned visit).
            Value::Tagged(boxed) if is_binary_tag(boxed.tag().as_str()) => match boxed.value() {
                Value::String(s) => match crate::base64::decode(s) {
                    Ok(bytes) => self.wrap_err(visitor.visit_byte_buf(bytes)),
                    Err(why) => self.wrap_err(Err(Error::Deserialize(format!("!!binary: {why}")))),
                },
                other => self.wrap_err(Err(Error::TypeMismatch {
                    expected: "string-shaped !!binary content",
                    found: type_name(other),
                })),
            },
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "bytes",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => self.wrap_err(visitor.visit_none()),
            _ => self.wrap_err(visitor.visit_some(self)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => self.wrap_err(visitor.visit_unit()),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "null",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return visitor.visit_map(SpannedMapAccess::new(self.value, self.span_ctx));
        }
        self.wrap_err(visitor.visit_newtype_struct(self))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Sequence(seq) => {
                self.wrap_err(visitor.visit_seq(ValueSeqAccess::from_de(&self, seq)))
            }
            // Tagged values are transparent for typed `deserialize_*`
            // calls — `Vec<T>::deserialize` against `!List [1, 2, 3]`
            // (which now surfaces as `Tagged(Sequence(...))` per the
            // tag-preserving loader) sees through the wrapper.
            Value::Tagged(tagged) => self.descend(tagged.value()).deserialize_seq(visitor),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "sequence",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Mapping(map) => {
                self.wrap_err(visitor.visit_map(ValueMapAccess::from_de(&self, map)))
            }
            // Tagged values are transparent for typed
            // `deserialize_*` calls — `HashMap::deserialize`
            // against `!!set { Mark, Sammy }` (which now surfaces
            // as `Tagged(Mapping(...))` per the tag-preserving
            // loader) sees through the wrapper.
            Value::Tagged(tagged) => self.descend(tagged.value()).deserialize_map(visitor),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "mapping",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return visitor.visit_map(SpannedMapAccess::new(self.value, self.span_ctx));
        }
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(variant) => {
                let de: de::value::StrDeserializer<'de, Error> =
                    variant.as_str().into_deserializer();
                self.wrap_err(visitor.visit_enum(de))
            }
            Value::Mapping(map) if map.len() == 1 => {
                let (variant, value) = map.iter().next().unwrap();
                self.wrap_err(visitor.visit_enum(EnumAccess {
                    variant,
                    value,
                    span_ctx: self.span_ctx,
                }))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "string or single-key mapping",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.wrap_err(visitor.visit_unit())
    }
}

pub(crate) struct ValueSeqAccess<'de> {
    iter: core::slice::Iter<'de, Value>,
    span_ctx: Option<&'de span_context::SpanContext>,
    ignore_binary_tag_for_string: bool,
    /// Mirror of the parent [`Deserializer::preserve_tags`] so
    /// nested `Value::Tagged(...)` nodes inside a sequence
    /// survive the data-binding return path.
    preserve_tags: bool,
}

impl<'de> ValueSeqAccess<'de> {
    pub(crate) fn from_de(de: &Deserializer<'de>, seq: &'de [Value]) -> Self {
        ValueSeqAccess {
            iter: seq.iter(),
            span_ctx: de.span_ctx,
            ignore_binary_tag_for_string: de.ignore_binary_tag_for_string,
            preserve_tags: de.preserve_tags,
        }
    }
}

impl<'de> SeqAccess<'de> for ValueSeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                let de = Deserializer {
                    value,
                    span_ctx: self.span_ctx,
                    ignore_binary_tag_for_string: self.ignore_binary_tag_for_string,
                    preserve_tags: self.preserve_tags,
                };
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }
}

pub(crate) struct ValueMapAccess<'de> {
    iter: indexmap::map::Iter<'de, String, Value>,
    value: Option<&'de Value>,
    span_ctx: Option<&'de span_context::SpanContext>,
    ignore_binary_tag_for_string: bool,
    /// Mirror of the parent [`Deserializer::preserve_tags`] so
    /// nested `Value::Tagged(...)` nodes inside a mapping survive
    /// the data-binding return path. See `de::Deserializer` docs.
    preserve_tags: bool,
}

impl<'de> ValueMapAccess<'de> {
    pub(crate) fn from_de(de: &Deserializer<'de>, map: &'de crate::value::Mapping) -> Self {
        ValueMapAccess {
            iter: map.iter(),
            value: None,
            span_ctx: de.span_ctx,
            ignore_binary_tag_for_string: de.ignore_binary_tag_for_string,
            preserve_tags: de.preserve_tags,
        }
    }

    /// Build the child [`Deserializer`] used to read each map
    /// value — propagates every per-call toggle including
    /// `preserve_tags`.
    fn child_de(&self, value: &'de Value) -> Deserializer<'de> {
        Deserializer {
            value,
            span_ctx: self.span_ctx,
            ignore_binary_tag_for_string: self.ignore_binary_tag_for_string,
            preserve_tags: self.preserve_tags,
        }
    }
}

impl<'de> MapAccess<'de> for ValueMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let de = self.child_de(value);
                let key_de: de::value::StrDeserializer<'de, Error> =
                    key.as_str().into_deserializer();
                de.wrap_err(seed.deserialize(key_de).map(Some))
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => {
                let de = self.child_de(value);
                let res = seed.deserialize(de);
                de.wrap_err(res)
            }
            None => Err(de::Error::custom("value is missing")),
        }
    }
}

struct EnumAccess<'de> {
    variant: &'de str,
    value: &'de Value,
    span_ctx: Option<&'de span_context::SpanContext>,
}

impl<'de> de::EnumAccess<'de> for EnumAccess<'de> {
    type Error = Error;
    type Variant = VariantAccess<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let de: de::value::StrDeserializer<'de, Error> = self.variant.into_deserializer();
        let variant = seed.deserialize(de)?;
        let visitor = VariantAccess {
            value: self.value,
            span_ctx: self.span_ctx,
        };
        Ok((variant, visitor))
    }
}

struct VariantAccess<'de> {
    value: &'de Value,
    span_ctx: Option<&'de span_context::SpanContext>,
}

impl<'de> de::VariantAccess<'de> for VariantAccess<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        Deserialize::deserialize(de)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        seed.deserialize(de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        de::Deserializer::deserialize_seq(de, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        de::Deserializer::deserialize_map(de, visitor)
    }
}

pub(crate) struct SpannedMapAccess<'de> {
    value: &'de Value,
    span_ctx: Option<&'de span_context::SpanContext>,
    fields: core::slice::Iter<'static, &'static str>,
}

impl<'de> SpannedMapAccess<'de> {
    pub(crate) fn new(value: &'de Value, span_ctx: Option<&'de span_context::SpanContext>) -> Self {
        SpannedMapAccess {
            value,
            span_ctx,
            fields: crate::spanned::SPANNED_FIELDS.iter(),
        }
    }
}

impl<'de> MapAccess<'de> for SpannedMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.fields.next() {
            Some(field) => {
                use serde::de::value::BorrowedStrDeserializer;
                let de: BorrowedStrDeserializer<'_, Error> = BorrowedStrDeserializer::new(field);
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        use crate::spanned::*;
        let last_field = SPANNED_FIELDS[SPANNED_FIELDS.len() - 1 - (self.fields.len())];

        if last_field == SPANNED_FIELD_VALUE {
            let de = if let Some(ctx) = self.span_ctx {
                Deserializer::with_span_context(self.value, ctx)
            } else {
                Deserializer::new(self.value)
            };
            return de.wrap_err(seed.deserialize(de));
        }

        let ptr: *const Value = self.value;
        let addr = ptr as usize;
        let span = self.span_ctx.and_then(|ctx| ctx.spans.get(&addr));
        let loc = if let Some(s) = span {
            crate::error::Location::from_index(&self.span_ctx.unwrap().source, s.0)
        } else {
            crate::error::Location::default()
        };
        let end_loc = if let Some(s) = span {
            crate::error::Location::from_index(&self.span_ctx.unwrap().source, s.1)
        } else {
            crate::error::Location::default()
        };

        let val = match last_field {
            SPANNED_FIELD_START_LINE => loc.line(),
            SPANNED_FIELD_START_COLUMN => loc.column(),
            SPANNED_FIELD_START_INDEX => loc.index(),
            SPANNED_FIELD_END_LINE => end_loc.line(),
            SPANNED_FIELD_END_COLUMN => end_loc.column(),
            SPANNED_FIELD_END_INDEX => end_loc.index(),
            _ => crate::error::invariant_violated(
                "spanned-field index outside the SPANNED_FIELDS array",
            ),
        };

        seed.deserialize(val.into_deserializer())
    }
}

/// True if `tag` names the YAML 1.2 binary tag, in any of the forms
/// the scanner / loader may produce: shorthand `!!binary`, suffix
/// `binary` (post-handle-stripping), or the canonical full URI
/// `tag:yaml.org,2002:binary`. Stripping the leading `!` on the
/// shorthand keeps `Tag::new("!!binary") == Tag::new("binary")` —
/// which noyalib's `Tag` already considers equal — both matching.
pub(crate) fn is_binary_tag(tag: &str) -> bool {
    matches!(
        tag,
        "!!binary" | "binary" | "tag:yaml.org,2002:binary" | "!<tag:yaml.org,2002:binary>"
    )
}

fn type_name(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(_) => "bool".to_owned(),
        Value::Number(Number::Integer(_)) => "integer".to_owned(),
        Value::Number(Number::Float(_)) => "float".to_owned(),
        Value::String(_) => "string".to_owned(),
        Value::Sequence(_) => "sequence".to_owned(),
        Value::Mapping(_) => "mapping".to_owned(),
        Value::Tagged(tagged) => format!("tagged value (!{})", tagged.tag().as_str()),
    }
}
