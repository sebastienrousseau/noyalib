// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Flattened<T>` — capture the underlying [`Value`] alongside the
//! typed deserialization for `#[serde(flatten)]` targets.
//!
//! `#[serde(flatten)]` is the idiomatic way to collect "extra" fields
//! into a residue type, but the built-in residue types (`HashMap<String,
//! Value>`, `serde_json::Value`, our `Value`) erase the typed view: you
//! get the residue keys *or* the typed struct, never both.
//!
//! `Flattened<T>` solves that by capturing the [`Value`] tree first,
//! then re-running deserialization into `T` from the captured tree.
//! Both views are exposed:
//!
//! - `flattened.value: T` — the typed struct view.
//! - `flattened.raw: Value` — the dynamic view that lets callers walk
//!   the original data, look up unknown keys, run schema validation,
//!   or attach span info via [`crate::cst::Document::span_at`].

use crate::value::Value;

/// Wrapper that pairs a typed deserialization of `T` with the
/// underlying [`Value`] tree captured from the source.
///
/// `Flattened<T>` is the answer to "I want `#[serde(flatten)]` plus
/// the dynamic view for span lookup / unknown-field detection /
/// schema validation". It deserializes by:
///
/// 1. First capturing the input as a [`Value`] (preserving every
///    key the source supplied, including ones the typed `T` may
///    not declare).
/// 2. Then re-running `T::deserialize` against the captured
///    [`Value`] via [`crate::from_value`].
///
/// The cost is one extra [`Value`] tree allocation per
/// `Flattened<T>` field and one extra deserialize pass — small
/// compared to the typical config-loading hot path.
///
/// # Examples
///
/// ```
/// use noyalib::{from_str, Flattened, Value};
///
/// #[derive(Debug, serde::Deserialize)]
/// struct Inner {
///     port: u16,
/// }
///
/// #[derive(Debug, serde::Deserialize)]
/// struct Config {
///     name: String,
///     // Capture both the typed view and the raw mapping so the
///     // application layer can still inspect unknown keys.
///     inner: Flattened<Inner>,
/// }
///
/// let cfg: Config = from_str(
///     "name: noyalib\ninner:\n  port: 8080\n  extra: not-in-Inner\n",
/// ).unwrap();
///
/// // Typed view — the fields Inner declares.
/// assert_eq!(cfg.inner.value.port, 8080);
///
/// // Raw view — the full mapping including the `extra` key.
/// match &cfg.inner.raw {
///     Value::Mapping(m) => {
///         assert!(m.contains_key("port"));
///         assert!(m.contains_key("extra"));
///     }
///     other => panic!("expected mapping, got {other:?}"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flattened<T> {
    /// The typed deserialization of the underlying value.
    pub value: T,
    /// The raw [`Value`] tree captured from the source, before
    /// typed projection. Useful for span lookup, unknown-key
    /// inspection, schema validation, and round-trip serialization.
    pub raw: Value,
}

impl<T> Flattened<T> {
    /// Construct a [`Flattened`] from an explicit (raw, typed) pair.
    ///
    /// This is mostly useful for tests and bespoke deserialization
    /// flows. Most callers obtain `Flattened<T>` via `serde::Deserialize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Flattened, Value};
    /// let f = Flattened::new(42_u16, Value::from(42_i64));
    /// assert_eq!(f.value, 42);
    /// assert_eq!(f.raw.as_i64(), Some(42));
    /// ```
    #[must_use]
    pub fn new(value: T, raw: Value) -> Self {
        Self { value, raw }
    }

    /// Consume the wrapper and return only the typed view.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Flattened, Value};
    /// let f = Flattened::new("hello".to_string(), Value::from("hello"));
    /// assert_eq!(f.into_value(), "hello");
    /// ```
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }

    /// Borrow the typed view.
    pub fn as_value(&self) -> &T {
        &self.value
    }

    /// Borrow the raw [`Value`] view.
    pub fn as_raw(&self) -> &Value {
        &self.raw
    }
}

impl<T> core::ops::Deref for Flattened<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<'de, T> serde_core::Deserialize<'de> for Flattened<T>
where
    T: for<'a> serde_core::Deserialize<'a> + 'static,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        // Step 1: capture the source as a typed `Value` tree —
        // every key the source supplied is preserved.
        let raw = Value::deserialize(deserializer)?;
        // Step 2: re-run `T::deserialize` against the captured
        // value via `from_value`. The HRTB on `T` lets the
        // returned `T` outlive the temporary `&raw` borrow.
        let value = crate::from_value::<T>(&raw).map_err(serde_core::de::Error::custom)?;
        Ok(Self { value, raw })
    }
}

impl<T: serde_core::Serialize> serde_core::Serialize for Flattened<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        // Round-trip transparency: serializing a `Flattened<T>`
        // is equivalent to serializing the typed view alone. This
        // matches the expectation that `Flattened<T>` is a
        // *capture* wrapper, not a separate schema element.
        self.value.serialize(serializer)
    }
}
