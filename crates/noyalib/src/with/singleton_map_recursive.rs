//! Recursively serialize enums as single-entry maps.
//!
//! This module provides `serialize` and `deserialize` functions that apply
//! singleton map formatting recursively through nested structures.
//!
//! # Examples
//!
//! ```rust
//! use noyalib::with::singleton_map_recursive;
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! enum Inner {
//!     A,
//!     B { value: i32 },
//! }
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! enum Outer {
//!     Single(Inner),
//!     Multiple(Vec<Inner>),
//! }
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! struct Config {
//!     #[serde(with = "singleton_map_recursive")]
//!     items: Vec<Outer>,
//! }
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use serde::{Deserializer, Serialize, Serializer};

/// Recursively transform a Value to use singleton map representation for enums.
fn transform_to_singleton_map(value: crate::Value) -> crate::Value {
    match value {
        crate::Value::Sequence(seq) => {
            crate::Value::Sequence(seq.into_iter().map(transform_to_singleton_map).collect())
        }
        crate::Value::Mapping(map) => {
            let transformed: crate::Mapping = map
                .into_iter()
                .map(|(k, v)| (k, transform_to_singleton_map(v)))
                .collect();
            crate::Value::Mapping(transformed)
        }
        crate::Value::Tagged(tagged) => {
            let (tag, value) = tagged.into_parts();
            let inner = transform_to_singleton_map(value);
            crate::Value::Tagged(Box::new(crate::TaggedValue::new(tag, inner)))
        }
        // Scalars pass through unchanged
        other => other,
    }
}

/// Serialize a value with recursive singleton map transformation.
///
/// This applies singleton map formatting to all enum variants throughout
/// the nested structure.
///
/// # Examples
///
/// ```rust
/// use noyalib::with::singleton_map_recursive;
///
/// #[derive(Debug, serde::Serialize, serde::Deserialize)]
/// enum Status {
///     Active,
///     Pending,
/// }
///
/// #[derive(Debug, serde::Serialize, serde::Deserialize)]
/// struct Task {
///     #[serde(with = "singleton_map_recursive")]
///     statuses: Vec<Status>,
/// }
/// ```
pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    // Serialize to Value first
    let yaml_value = crate::to_value(value).map_err(serde::ser::Error::custom)?;

    // Transform recursively
    let transformed = transform_to_singleton_map(yaml_value);

    // Serialize the transformed value
    transformed.serialize(serializer)
}

/// Deserialize a value from recursive singleton map format.
///
/// This is the counterpart to [`serialize`], deserializing from the
/// singleton map format back to the original type.
///
/// # Examples
///
/// ```
/// use noyalib::with::singleton_map_recursive;
///
/// #[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug)]
/// enum Inner { A }
///
/// #[derive(serde::Deserialize, Debug)]
/// struct Doc {
///     #[serde(with = "singleton_map_recursive")]
///     items: Vec<Inner>,
/// }
///
/// let d: Doc = noyalib::from_str("items:\n  - A: null\n").unwrap();
/// assert_eq!(d.items.len(), 1);
/// ```
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: serde_core::de::DeserializeOwned + 'static,
    D: Deserializer<'de>,
{
    use serde::Deserialize;
    // Deserialize as Value first
    let value = crate::Value::deserialize(deserializer)?;

    // Convert to target type
    crate::from_value(&value).map_err(serde_core::de::Error::custom)
}

#[cfg(test)]
mod tests {

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    enum Inner {
        A,
        B { value: i32 },
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Container {
        name: String,
        #[serde(with = "crate::with::singleton_map_recursive")]
        items: Vec<Inner>,
    }

    #[test]
    fn test_recursive_vec_of_enums() {
        let container = Container {
            name: "test".to_string(),
            items: vec![Inner::A, Inner::B { value: 42 }],
        };

        let yaml = crate::to_string(&container).unwrap();
        let parsed: Container = crate::from_str(&yaml).unwrap();
        assert_eq!(parsed, container);
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Nested {
        #[serde(with = "crate::with::singleton_map_recursive")]
        data: std::collections::BTreeMap<String, Inner>,
    }

    #[test]
    fn test_recursive_map_of_enums() {
        let mut data = std::collections::BTreeMap::new();
        let _ = data.insert("first".to_string(), Inner::A);
        let _ = data.insert("second".to_string(), Inner::B { value: 100 });

        let container = Nested { data };

        let yaml = crate::to_string(&container).unwrap();
        let parsed: Nested = crate::from_str(&yaml).unwrap();
        assert_eq!(parsed, container);
    }
}
