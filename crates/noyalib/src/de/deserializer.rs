//! The serde `Deserializer` over a `&Value` and its access types.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::error::{Error, Result};
use crate::prelude::f64_fract;
use crate::prelude::*;
use crate::span_context;
use crate::value::{Number, Value};

/// A YAML deserializer.
///
/// # Examples
///
/// ```
/// use noyalib::{Deserializer, Value};
/// let v = Value::from(42_i64);
/// let de = Deserializer::new(&v);
/// let n: i32 = serde_core::Deserialize::deserialize(de).unwrap();
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
    /// Per-call flag mirroring
    /// [`ParserConfig::plain_scalar_strings`] (refs #344). When
    /// `true`, a `String`/`char` target accepts a non-string scalar
    /// (`Value::Bool`, `Value::Null`, `Value::Number`) and receives
    /// its formatted text. Default `false` preserves the historical
    /// refusal.
    pub(crate) plain_scalar_strings: bool,
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
            plain_scalar_strings: false,
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
            plain_scalar_strings: false,
        }
    }

    /// Pass-through constructor for the
    /// [`crate::ParserConfig::ignore_binary_tag_for_string`] and
    /// [`crate::ParserConfig::plain_scalar_strings`] flags. Used
    /// internally by [`from_str_with_config`] when the caller has
    /// opted in to either.
    pub(crate) fn with_options(
        value: &'de Value,
        span_ctx: Option<&'de span_context::SpanContext>,
        ignore_binary_tag_for_string: bool,
        plain_scalar_strings: bool,
    ) -> Self {
        Deserializer {
            value,
            span_ctx,
            ignore_binary_tag_for_string,
            plain_scalar_strings,
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
            plain_scalar_strings: self.plain_scalar_strings,
        }
    }

    /// Attach the source location of the value being deserialized to an
    /// error that does not carry one yet.
    ///
    /// Three error shapes reach this point without a location when a typed
    /// target rejects a value: [`Error::Deserialize`] (noyalib's own
    /// deserialization failures), [`Error::TypeMismatch`] (the catch-all
    /// arms of the typed `deserialize_*` methods), and [`Error::Custom`]
    /// (serde's `invalid_type` / `invalid_value` / `custom`, raised by the
    /// caller's visitor -- the path a `#[serde(flatten)]` field or any
    /// `deserialize_any` descent takes). All three are re-wrapped as
    /// [`Error::DeserializeWithLocation`] when the deserializer was built
    /// from text (`from_str`, which records a span per value); the message
    /// is the original error's text. Deserializers built without a span
    /// context (`from_value`, [`Deserializer::new`]) return the error as
    /// is, so `matches!(err, Error::TypeMismatch { .. })` keeps holding on
    /// that path.
    fn wrap_err<T>(&self, res: Result<T>) -> Result<T> {
        let err = match res {
            Ok(value) => return Ok(value),
            Err(err) => err,
        };
        let Some(ctx) = self.span_ctx else {
            return Err(err);
        };
        let ptr: *const Value = self.value;
        let addr = ptr as usize;
        let Some(span) = ctx.spans.get(&addr) else {
            return Err(err);
        };
        let message = match err {
            Error::Deserialize(msg) | Error::Custom(msg) => msg,
            mismatch @ Error::TypeMismatch { .. } => mismatch.to_string(),
            other => return Err(other),
        };
        // Only the innermost wrap reaches this line (an already-wrapped
        // error returns through the `other` arm above), so the recorded
        // node is the one the failure is about. `from_str` walks the
        // root value to turn it into a field path (#353).
        #[cfg(feature = "std")]
        span_context::record_error_node(addr);
        Err(Error::deserialize_at(message, &ctx.source, span.0))
    }
}

impl<'de> serde_core::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        match self.value {
            Value::Null => self.wrap_err(visitor.visit_none()),
            Value::Bool(b) => self.wrap_err(visitor.visit_bool(*b)),
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_i64(*n)),
            #[cfg(feature = "lossless-u64")]
            Value::Number(Number::Unsigned(n)) => self.wrap_err(visitor.visit_u64(*n)),
            Value::Number(Number::Float(n)) => self.wrap_err(visitor.visit_f64(*n)),
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            Value::Sequence(_) => self.deserialize_seq(visitor),
            Value::Mapping(_) => self.deserialize_map(visitor),
            Value::Tagged(tagged) => {
                // `deserialize_any` is the "self-describing" entry point —
                // reached by `Value`'s own `Deserialize` impl (directly, or
                // nested inside `Mapping`, `Sequence`/`Vec<Value>`, or a
                // struct field of type `Value`) and by serde's untagged-enum
                // content buffering. Typed struct/map targets never reach
                // here for a tagged node — `deserialize_map`/
                // `deserialize_struct` have their own arm (below) that sees
                // through the tag transparently, e.g.
                // `#[derive(serde::Deserialize)] struct Foo { x: i32 }`
                // against `!Foo {x: 1}` yields `Foo { x: 1 }`.
                //
                // So a tagged node reaching *this* arm is a `Value` being
                // reconstructed, and it must keep its tag (see #350; the
                // top-level `Value` target already did, via the AST
                // loader's `parse_one_value` bypass in
                // `from_str_with_config` / `from_value`'s clone fast path —
                // this arm is what nested `Value`s go through instead).
                // Hand the tag/inner-value pair to the visitor as an enum
                // (variant name = tag, payload = the untagged value) —
                // `ValueVisitor::visit_enum` reassembles `Value::Tagged`.
                self.wrap_err(visitor.visit_enum(EnumAccess {
                    variant: tagged.tag().as_str(),
                    value: tagged.value(),
                    span_ctx: self.span_ctx,
                }))
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
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
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        match self.value {
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_i64(*n)),
            #[cfg(feature = "lossless-u64")]
            Value::Number(Number::Unsigned(n)) => match i64::try_from(*n) {
                Ok(n) => self.wrap_err(visitor.visit_i64(n)),
                // `type_name` reports both `Integer` and `Unsigned` as
                // "integer", so reusing it here would render the useless
                // "expected integer, found integer". Name the actual
                // problem instead: the value is an integer, it just does
                // not fit the signed target.
                Err(_) => self.wrap_err(Err(Error::TypeMismatch {
                    expected: "signed integer (i64)",
                    found: format!("unsigned integer {n}, above i64::MAX"),
                })),
            },
            Value::Number(Number::Float(n))
                if f64_fract(*n) == 0.0
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
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        match self.value {
            Value::Number(Number::Integer(n)) if *n >= 0 => {
                self.wrap_err(visitor.visit_u64(*n as u64))
            }
            // Mirror of the `Unsigned` -> i64 case in `deserialize_i64`:
            // falling through to the catch-all would report "expected
            // unsigned integer, found integer", which does not tell the
            // caller that the problem is the sign.
            Value::Number(Number::Integer(n)) => self.wrap_err(Err(Error::TypeMismatch {
                expected: "unsigned integer",
                found: format!("negative integer {n}"),
            })),
            #[cfg(feature = "lossless-u64")]
            Value::Number(Number::Unsigned(n)) => self.wrap_err(visitor.visit_u64(*n)),
            Value::Number(Number::Float(n))
                if f64_fract(*n) == 0.0 && *n >= 0.0 && *n <= u64::MAX as f64 && !n.is_nan() =>
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
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        match self.value {
            Value::Number(Number::Float(n)) => self.wrap_err(visitor.visit_f64(*n)),
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_f64(*n as f64)),
            #[cfg(feature = "lossless-u64")]
            Value::Number(Number::Unsigned(n)) => self.wrap_err(visitor.visit_f64(*n as f64)),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "float",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        match self.value {
            Value::String(s) if s.chars().count() == 1 => {
                self.wrap_err(visitor.visit_char(s.chars().next().unwrap()))
            }
            // Opt-in (refs #344, `plain_scalar_strings`): a `char`
            // field sees the same literal text a `String` field
            // would for a number, bool, or null — see
            // `scalar_as_text` — further constrained to exactly one
            // character, same as the arm above. Off by default.
            _ if self.plain_scalar_strings => match scalar_as_text(self.value) {
                Some(text) if text.chars().count() == 1 => {
                    self.wrap_err(visitor.visit_char(text.chars().next().unwrap()))
                }
                _ => self.wrap_err(Err(Error::TypeMismatch {
                    expected: "char",
                    found: type_name(self.value),
                })),
            },
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "char",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
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
            // Opt-in (refs #344, `plain_scalar_strings`): a `String`
            // target receives a number/bool/null scalar's text —
            // implicit typing is a fallback used by
            // `deserialize_any` for untyped targets, not a
            // constraint on an explicitly-typed `String` field. Off
            // by default — the catch-all below is the historical
            // refusal.
            _ if self.plain_scalar_strings => match scalar_as_text(self.value) {
                Some(text) => self.wrap_err(visitor.visit_str(&text)),
                None => self.wrap_err(Err(Error::TypeMismatch {
                    expected: "string",
                    found: type_name(self.value),
                })),
            },
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "string",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
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
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        match self.value {
            Value::Null => self.wrap_err(visitor.visit_none()),
            _ => self.wrap_err(visitor.visit_some(self)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
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
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return visitor.visit_map(SpannedMapAccess::new(self.value, self.span_ctx));
        }
        self.wrap_err(visitor.visit_newtype_struct(self))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
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
        V: serde_core::de::Visitor<'de>,
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
        V: serde_core::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        match self.value {
            Value::Mapping(map) => {
                self.wrap_err(visitor.visit_map(ValueMapAccess::from_de(&self, map)))
            }
            // The null document (empty / whitespace-only / comment-only
            // input, or a bare `---`) has "no entries" for a map or
            // `#[serde(default)]` struct target. See #349.
            Value::Null => self.wrap_err(visitor.visit_map(EmptyMapAccess)),
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
        V: serde_core::de::Visitor<'de>,
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
        V: serde_core::de::Visitor<'de>,
    {
        match self.value {
            Value::String(variant) => {
                let de =
                    serde_core::de::value::StrDeserializer::<'de, Error>::new(variant.as_str());
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
        V: serde_core::de::Visitor<'de>,
    {
        match self.value {
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        self.wrap_err(visitor.visit_unit())
    }
}

pub(crate) struct ValueSeqAccess<'de> {
    iter: core::slice::Iter<'de, Value>,
    span_ctx: Option<&'de span_context::SpanContext>,
    ignore_binary_tag_for_string: bool,
    plain_scalar_strings: bool,
}

impl<'de> ValueSeqAccess<'de> {
    pub(crate) fn from_de(de: &Deserializer<'de>, seq: &'de [Value]) -> Self {
        ValueSeqAccess {
            iter: seq.iter(),
            span_ctx: de.span_ctx,
            ignore_binary_tag_for_string: de.ignore_binary_tag_for_string,
            plain_scalar_strings: de.plain_scalar_strings,
        }
    }
}

impl<'de> serde_core::de::SeqAccess<'de> for ValueSeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde_core::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                let de = Deserializer {
                    value,
                    span_ctx: self.span_ctx,
                    ignore_binary_tag_for_string: self.ignore_binary_tag_for_string,
                    plain_scalar_strings: self.plain_scalar_strings,
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
    plain_scalar_strings: bool,
}

impl<'de> ValueMapAccess<'de> {
    pub(crate) fn from_de(de: &Deserializer<'de>, map: &'de crate::value::Mapping) -> Self {
        ValueMapAccess {
            iter: map.iter(),
            value: None,
            span_ctx: de.span_ctx,
            ignore_binary_tag_for_string: de.ignore_binary_tag_for_string,
            plain_scalar_strings: de.plain_scalar_strings,
        }
    }

    /// Build the child [`Deserializer`] used to read each map
    /// value — propagates every per-call toggle.
    fn child_de(&self, value: &'de Value) -> Deserializer<'de> {
        Deserializer {
            value,
            span_ctx: self.span_ctx,
            ignore_binary_tag_for_string: self.ignore_binary_tag_for_string,
            plain_scalar_strings: self.plain_scalar_strings,
        }
    }
}

impl<'de> serde_core::de::MapAccess<'de> for ValueMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde_core::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let de = self.child_de(value);
                let key_de =
                    serde_core::de::value::StrDeserializer::<'de, Error>::new(key.as_str());
                de.wrap_err(seed.deserialize(key_de).map(Some))
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde_core::de::DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => {
                let de = self.child_de(value);
                let res = seed.deserialize(de);
                de.wrap_err(res)
            }
            None => Err(serde_core::de::Error::custom("value is missing")),
        }
    }
}

/// `MapAccess` that immediately reports "no entries".
///
/// An empty, whitespace-only, or comment-only document (and a bare
/// `---` with no content) parses as the YAML null document
/// (`Value::Null`). `Mapping` and `#[serde(default)]` struct targets
/// treat that the same way `serde_yaml` does — as a map with no
/// entries — instead of a type-mismatch error. `deserialize_any`
/// still visits `Value::Null` as `None`; only the map/struct entry
/// points route through here. See #349.
pub(crate) struct EmptyMapAccess;

impl<'de> serde_core::de::MapAccess<'de> for EmptyMapAccess {
    type Error = Error;

    fn next_key_seed<K>(&mut self, _seed: K) -> Result<Option<K::Value>>
    where
        K: serde_core::de::DeserializeSeed<'de>,
    {
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, _seed: V) -> Result<V::Value>
    where
        V: serde_core::de::DeserializeSeed<'de>,
    {
        Err(serde_core::de::Error::custom("value is missing"))
    }
}

struct EnumAccess<'de> {
    variant: &'de str,
    value: &'de Value,
    span_ctx: Option<&'de span_context::SpanContext>,
}

impl<'de> serde_core::de::EnumAccess<'de> for EnumAccess<'de> {
    type Error = Error;
    type Variant = VariantAccess<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: serde_core::de::DeserializeSeed<'de>,
    {
        let de = serde_core::de::value::StrDeserializer::<'de, Error>::new(self.variant);
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

impl<'de> serde_core::de::VariantAccess<'de> for VariantAccess<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        serde_core::Deserialize::deserialize(de)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: serde_core::de::DeserializeSeed<'de>,
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
        V: serde_core::de::Visitor<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        serde_core::Deserializer::deserialize_seq(de, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: serde_core::de::Visitor<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        serde_core::Deserializer::deserialize_map(de, visitor)
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

impl<'de> serde_core::de::MapAccess<'de> for SpannedMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde_core::de::DeserializeSeed<'de>,
    {
        match self.fields.next() {
            Some(field) => {
                let de = serde_core::de::value::BorrowedStrDeserializer::<'_, Error>::new(field);
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde_core::de::DeserializeSeed<'de>,
    {
        use crate::spanned::{
            SPANNED_FIELD_END_COLUMN, SPANNED_FIELD_END_INDEX, SPANNED_FIELD_END_LINE,
            SPANNED_FIELD_START_COLUMN, SPANNED_FIELD_START_INDEX, SPANNED_FIELD_START_LINE,
            SPANNED_FIELD_VALUE, SPANNED_FIELDS,
        };
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

        seed.deserialize(serde_core::de::IntoDeserializer::into_deserializer(val))
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
        #[cfg(feature = "lossless-u64")]
        Value::Number(Number::Unsigned(_)) => "integer".to_owned(),
        Value::Number(Number::Float(_)) => "float".to_owned(),
        Value::String(_) => "string".to_owned(),
        Value::Sequence(_) => "sequence".to_owned(),
        Value::Mapping(_) => "mapping".to_owned(),
        Value::Tagged(tagged) => format!("tagged value (!{})", tagged.tag().as_str()),
    }
}

/// The literal text a scalar `Value` prints as, for the scalar kinds
/// that are not already a `Value::String` — `bool`, `null`, and the
/// `Number` variants. Returns `None` for `Value::String` (callers
/// already hold the text directly), `Value::Sequence`,
/// `Value::Mapping`, and `Value::Tagged` (handled at each call site).
///
/// Refs #344: a `String` (or one-character `char`) target receives a
/// scalar's literal text even when it would otherwise resolve as a
/// number, bool, or null for an untyped target — implicit typing is
/// a fallback `deserialize_any` uses, not a constraint on an
/// explicitly-typed field. Two caveats follow from `Value` no longer
/// holding the original source text:
///
/// - `Value::Null` carries no text of its own, so this yields `""`.
///   The *streaming* deserializer — which typed parses take by
///   default — instead keeps a written `~`, `null`, or empty scalar
///   verbatim; only a caller that forces the AST path (a non-default
///   `ParserConfig`, or the `Value` target itself) sees `""` here.
/// - An integer literal's original spelling is not preserved: `0x1F`
///   parses to `Number::Integer(31)`, which formats back as `"31"`,
///   not `"0x1F"`. The streaming path gives the byte-exact source
///   text instead.
fn scalar_as_text(value: &Value) -> Option<Cow<'_, str>> {
    match value {
        Value::Bool(b) => Some(Cow::Borrowed(if *b { "true" } else { "false" })),
        Value::Null => Some(Cow::Borrowed("")),
        Value::Number(Number::Integer(n)) => Some(Cow::Owned(n.to_string())),
        #[cfg(feature = "lossless-u64")]
        Value::Number(Number::Unsigned(n)) => Some(Cow::Owned(n.to_string())),
        Value::Number(Number::Float(n)) => Some(Cow::Owned(format_float_for_string(*n))),
        _ => None,
    }
}

/// Format a float exactly as the emitter (`ser.rs`'s `write_value`)
/// would print it as a YAML plain scalar — deliberately not
/// `Number`'s own `Display` impl, which prints `4.0` as `4` (Rust's
/// default float `Display` suppresses a redundant `.0`). Keeping the
/// two in step means a `String` field reading a `Value::Number(Float)`
/// back sees the same digits the value would serialize as.
fn format_float_for_string(n: f64) -> String {
    if n.is_nan() {
        return ".nan".to_owned();
    }
    if n.is_infinite() {
        return if n > 0.0 {
            ".inf".to_owned()
        } else {
            "-.inf".to_owned()
        };
    }
    #[cfg(feature = "fast-float")]
    {
        ryu::Buffer::new().format(n).to_owned()
    }
    #[cfg(not(feature = "fast-float"))]
    {
        format!("{n:?}")
    }
}
