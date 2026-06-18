//! `From<T> for Value` conversions and `Index`/`IndexMut`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use super::{Mapping, Number, TaggedValue, Value};
use crate::prelude::*;
use core::ops::{Index, IndexMut};

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Number(Number::Integer(v))
    }
}

impl From<u8> for Value {
    fn from(v: u8) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<u16> for Value {
    fn from(v: u16) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Number(Number::Float(f64::from(v)))
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Number(Number::Float(v))
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_owned())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Sequence(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

impl From<Number> for Value {
    fn from(v: Number) -> Self {
        Value::Number(v)
    }
}

impl From<Mapping> for Value {
    fn from(v: Mapping) -> Self {
        Value::Mapping(v)
    }
}

// Note: From<Sequence> is covered by From<Vec<T>> since Sequence = Vec<Value>

impl From<TaggedValue> for Value {
    fn from(v: TaggedValue) -> Self {
        Value::Tagged(Box::new(v))
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::Number(Number::from(v))
    }
}

impl From<isize> for Value {
    fn from(v: isize) -> Self {
        Value::Number(Number::from(v))
    }
}

impl From<usize> for Value {
    fn from(v: usize) -> Self {
        Value::Number(Number::from(v))
    }
}

impl<'a> From<Cow<'a, str>> for Value {
    fn from(v: Cow<'a, str>) -> Self {
        Value::String(v.into_owned())
    }
}

impl<T: Clone + Into<Value>> From<&[T]> for Value {
    fn from(v: &[T]) -> Self {
        Value::Sequence(v.iter().cloned().map(Into::into).collect())
    }
}

impl<T: Into<Value>> FromIterator<T> for Value {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Value::Sequence(iter.into_iter().map(Into::into).collect())
    }
}

// ============================================================================
// Index trait implementations for Value
// ============================================================================

impl Index<usize> for Value {
    type Output = Value;

    /// Index into a YAML sequence.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a sequence or if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = "- a\n- b\n- c\n";
    /// let value: Value = from_str(yaml).unwrap();
    /// assert_eq!(value[0].as_str(), Some("a"));
    /// assert_eq!(value[1].as_str(), Some("b"));
    /// ```
    #[track_caller]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .expect("index out of bounds or not a sequence")
    }
}

impl IndexMut<usize> for Value {
    /// Mutably index into a YAML sequence.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a sequence or if the index is out of bounds.
    #[track_caller]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
            .expect("index out of bounds or not a sequence")
    }
}

impl Index<&str> for Value {
    type Output = Value;

    /// Index into a YAML mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a mapping or if the key is not found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = "name: test\nversion: 1\n";
    /// let value: Value = from_str(yaml).unwrap();
    /// assert_eq!(value["name"].as_str(), Some("test"));
    /// assert_eq!(value["version"].as_i64(), Some(1));
    /// ```
    #[track_caller]
    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).expect("key not found or not a mapping")
    }
}

impl IndexMut<&str> for Value {
    /// Mutably index into a YAML mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a mapping or if the key is not found.
    #[track_caller]
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.get_mut(key).expect("key not found or not a mapping")
    }
}
