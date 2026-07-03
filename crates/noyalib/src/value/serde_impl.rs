//! serde `Serialize`/`Deserialize` for `Value`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use super::{
    Mapping, Number, TAGGED_VALUE_FIELD_TAG, TAGGED_VALUE_FIELD_VALUE, Tag, TaggedValue, Value,
};
use crate::prelude::*;
use indexmap::map::Iter;
use serde::{Deserialize, Serialize};

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, SeqAccess, Visitor};

        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("any valid YAML value")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
                Ok(Value::Number(Number::Integer(v)))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
                Ok(Value::Number(Number::from(v)))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
                Ok(Value::Number(Number::Float(v)))
            }

            fn visit_str<E>(self, v: &str) -> Result<Value, E> {
                Ok(Value::String(v.to_owned()))
            }

            fn visit_string<E>(self, v: String) -> Result<Value, E> {
                Ok(Value::String(v))
            }

            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                // Pre-size from the SeqAccess size_hint when
                // available — saves up to ~11 reallocations on a
                // 2 000-element sequence (Vec doubles on each
                // grow). Falls back to the default growth strategy
                // when the hint isn't reliable.
                let mut vec = match seq.size_hint() {
                    Some(n) if n > 0 && n < 1 << 20 => Vec::with_capacity(n),
                    _ => Vec::new(),
                };
                while let Some(elem) = seq.next_element()? {
                    vec.push(elem);
                }
                Ok(Value::Sequence(vec))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // Tag-preserving fast path: noyalib's own
                // [`crate::de::Deserializer::deserialize_any`]
                // routes `Value::Tagged(...)` through a
                // [`TagPreservingMapAccess`] when its
                // `preserve_tags` flag is on (set automatically
                // for `from_str::<Value>` / `from_value::<Value>`
                // — see [`crate::de::is_value_target`]). The first
                // key of that map is the [`TAGGED_VALUE_FIELD_TAG`]
                // sentinel; detect it and reconstruct
                // `Value::Tagged` so the tag survives the
                // data-binding return path.
                //
                // Other Deserializers (serde_json, FlatMap, …)
                // never see this magic shape, so this branch is
                // strictly additive.
                let first_key: Option<String> = map.next_key()?;
                if let Some(k) = first_key.as_deref() {
                    if k == TAGGED_VALUE_FIELD_TAG {
                        let tag_str: String = map.next_value()?;
                        let second_key: String = map.next_key()?.ok_or_else(|| {
                            <A::Error as serde::de::Error>::custom(
                                "tag-preserving map missing $__noyalib_value entry",
                            )
                        })?;
                        if second_key != TAGGED_VALUE_FIELD_VALUE {
                            return Err(<A::Error as serde::de::Error>::custom(format!(
                                "tag-preserving map: expected `{}`, got `{}`",
                                TAGGED_VALUE_FIELD_VALUE, second_key
                            )));
                        }
                        let inner: Value = map.next_value()?;
                        return Ok(Value::Tagged(Box::new(TaggedValue::new(
                            Tag::new(tag_str),
                            inner,
                        ))));
                    }
                }
                // Regular mapping path — collect every (k, v) pair
                // including the (k, v) we already consumed.
                // Pre-size when the MapAccess provides a usable
                // hint; saves ~10 IndexMap rehashes on large
                // mappings (capacity grows by ~doubling).
                let mut mapping = match map.size_hint() {
                    Some(n) if n > 0 && n < 1 << 20 => Mapping::with_capacity(n),
                    _ => Mapping::new(),
                };
                if let Some(k) = first_key {
                    let v: Value = map.next_value()?;
                    let _ = mapping.insert(k, v);
                }
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    let _ = mapping.insert(key, value);
                }
                Ok(Value::Mapping(mapping))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_none(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Number(Number::Integer(n)) => serializer.serialize_i64(*n),
            #[cfg(feature = "lossless-u64")]
            Value::Number(Number::Unsigned(n)) => serializer.serialize_u64(*n),
            Value::Number(Number::Float(n)) => serializer.serialize_f64(*n),
            Value::String(s) => serializer.serialize_str(s),
            Value::Sequence(s) => s.serialize(serializer),
            Value::Mapping(m) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in m {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            Value::Tagged(tagged) => {
                // Serialize as a single-entry map with tag as key
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(tagged.tag().as_str(), tagged.value())?;
                map.end()
            }
        }
    }
}

// ============================================================================
// Deserializer implementation for &Value
// ============================================================================

impl<'de> serde::de::IntoDeserializer<'de, crate::Error> for &'de Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

struct ValueSeqAccess<'de> {
    iter: core::slice::Iter<'de, Value>,
}

impl<'de> serde::de::SeqAccess<'de> for ValueSeqAccess<'de> {
    type Error = crate::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> crate::Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }
}

struct ValueMapAccess<'de> {
    iter: Iter<'de, String, Value>,
    value: Option<&'de Value>,
}

impl<'de> serde::de::MapAccess<'de> for ValueMapAccess<'de> {
    type Error = crate::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> crate::Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(serde::de::value::BorrowedStrDeserializer::new(key))
                    .map(Some)
            }
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

impl<'de> serde::Deserializer<'de> for &'de Value {
    type Error = crate::Error;

    fn deserialize_any<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Number(Number::Integer(n)) => visitor.visit_i64(*n),
            #[cfg(feature = "lossless-u64")]
            Value::Number(Number::Unsigned(n)) => visitor.visit_u64(*n),
            Value::Number(Number::Float(n)) => visitor.visit_f64(*n),
            Value::String(s) => visitor.visit_borrowed_str(s),
            Value::Sequence(seq) => visitor.visit_seq(ValueSeqAccess { iter: seq.iter() }),
            Value::Mapping(map) => visitor.visit_map(ValueMapAccess {
                iter: map.iter(),
                value: None,
            }),
            Value::Tagged(tagged) => {
                let tagged_ref: &'de TaggedValue = tagged;
                serde::Deserializer::deserialize_any(tagged_ref, visitor)
            }
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Tagged(tagged) => {
                let tagged_ref: &'de TaggedValue = tagged;
                serde::Deserializer::deserialize_enum(tagged_ref, name, variants, visitor)
            }
            Value::String(s) => visitor
                .visit_enum(serde::de::value::BorrowedStrDeserializer::<crate::Error>::new(s)),
            _ => serde::Deserializer::deserialize_any(self, visitor),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Sequence(seq) => visitor.visit_seq(ValueSeqAccess { iter: seq.iter() }),
            _ => serde::Deserializer::deserialize_any(self, visitor),
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Mapping(map) => visitor.visit_map(ValueMapAccess {
                iter: map.iter(),
                value: None,
            }),
            _ => serde::Deserializer::deserialize_any(self, visitor),
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return visitor.visit_map(crate::de::SpannedMapAccess::new(self, None));
        }
        serde::Deserializer::deserialize_map(self, visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct tuple
        tuple_struct identifier ignored_any
    }
}
