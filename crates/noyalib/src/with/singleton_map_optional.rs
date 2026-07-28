//! Serialize optional enums as single-entry maps.
//!
//! This module provides `serialize` and `deserialize` functions for use with
//! serde's `#[serde(with = "...")]` attribute to serialize `Option<T>` fields
//! where the inner type should use singleton map representation.
//!
//! # Examples
//!
//! ```rust
//! use noyalib::with::singleton_map_optional;
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! enum Status {
//!     Active,
//!     Pending { reason: String },
//! }
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! struct Task {
//!     name: String,
//!     #[serde(
//!         with = "singleton_map_optional",
//!         skip_serializing_if = "Option::is_none",
//!         default
//!     )]
//!     status: Option<Status>,
//! }
//!
//! let task = Task {
//!     name: "example".to_string(),
//!     status: Some(Status::Pending {
//!         reason: "waiting".to_string(),
//!     }),
//! };
//!
//! let yaml = noyalib::to_string(&task).unwrap();
//! let parsed: Task = noyalib::from_str(&yaml).unwrap();
//! assert_eq!(parsed, task);
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use serde::de::DeserializeOwned;
use serde::{Deserializer, Serialize, Serializer};

/// Serialize an optional value as a singleton map.
///
/// For `Some(value)`, this serializes the value using singleton map format.
/// For `None`, this serializes as null.
///
/// # Examples
///
/// ```rust
/// use noyalib::with::singleton_map_optional;
///
/// #[derive(Debug, serde::Serialize, serde::Deserialize)]
/// enum Action {
///     Start,
///     Stop,
/// }
///
/// #[derive(Debug, serde::Serialize, serde::Deserialize)]
/// struct Command {
///     #[serde(
///         with = "singleton_map_optional",
///         skip_serializing_if = "Option::is_none",
///         default
///     )]
///     action: Option<Action>,
/// }
/// ```
pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    match value {
        Some(inner) => {
            use serde::ser::SerializeMap;

            // Serialize to Value to inspect structure
            let yaml_value = crate::to_value(inner).map_err(serde::ser::Error::custom)?;

            match yaml_value {
                crate::Value::Mapping(map) => {
                    // Already a map (struct variant), serialize directly
                    let mut ser_map = serializer.serialize_map(Some(map.len()))?;
                    for (k, v) in map {
                        ser_map.serialize_entry(&k, &v)?;
                    }
                    ser_map.end()
                }
                crate::Value::String(s) => {
                    // Unit variant - serialize as { VariantName: null }
                    let mut ser_map = serializer.serialize_map(Some(1))?;
                    ser_map.serialize_entry(&s, &())?;
                    ser_map.end()
                }
                other => {
                    // Fallback: serialize as-is
                    other.serialize(serializer)
                }
            }
        }
        None => serializer.serialize_none(),
    }
}

/// Deserialize an optional value from a singleton map.
///
/// This is the counterpart to [`serialize`], deserializing from the
/// singleton map format back to `Option<T>`.
///
/// # Examples
///
/// ```
/// use noyalib::with::singleton_map_optional;
///
/// #[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug)]
/// enum Status { Active }
///
/// #[derive(serde::Deserialize, Debug)]
/// struct Doc {
///     #[serde(with = "singleton_map_optional", default)]
///     s: Option<Status>,
/// }
///
/// let d: Doc = noyalib::from_str("s: ~\n").unwrap();
/// assert!(d.s.is_none());
/// ```
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: DeserializeOwned + 'static,
    D: Deserializer<'de>,
{
    use serde::Deserialize;
    // Deserialize as Option<Value> first
    let opt_value: Option<crate::Value> = Option::deserialize(deserializer)?;

    match opt_value {
        Some(value) => {
            let inner: T = crate::from_value(&value).map_err(serde::de::Error::custom)?;
            Ok(Some(inner))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    enum TestEnum {
        Unit,
        Struct { value: String },
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Container {
        name: String,
        #[serde(
            with = "crate::with::singleton_map_optional",
            skip_serializing_if = "Option::is_none",
            default
        )]
        item: Option<TestEnum>,
    }

    #[test]
    fn test_singleton_map_optional_some() {
        let container = Container {
            name: "test".to_string(),
            item: Some(TestEnum::Struct {
                value: "hello".to_string(),
            }),
        };

        let yaml = crate::to_string(&container).unwrap();
        assert!(yaml.contains("Struct"));
        assert!(yaml.contains("value: hello"));

        let parsed: Container = crate::from_str(&yaml).unwrap();
        assert_eq!(parsed, container);
    }

    #[test]
    fn test_singleton_map_optional_none() {
        let container = Container {
            name: "test".to_string(),
            item: None,
        };

        let yaml = crate::to_string(&container).unwrap();
        // With skip_serializing_if, the field should not appear
        assert!(!yaml.contains("item"));

        let parsed: Container = crate::from_str(&yaml).unwrap();
        assert_eq!(parsed, container);
    }

    #[test]
    fn test_singleton_map_optional_unit_variant() {
        let container = Container {
            name: "test".to_string(),
            item: Some(TestEnum::Unit),
        };

        let yaml = crate::to_string(&container).unwrap();
        assert!(yaml.contains("Unit"));

        let parsed: Container = crate::from_str(&yaml).unwrap();
        assert_eq!(parsed, container);
    }

    #[test]
    fn test_singleton_map_optional_fallback() {
        // Test the fallback path when value is a Sequence
        #[derive(Debug, serde::Serialize)]
        struct SeqContainer {
            #[serde(with = "crate::with::singleton_map_optional")]
            items: Option<Vec<i32>>,
        }

        let container = SeqContainer {
            items: Some(vec![1, 2, 3]),
        };

        let yaml = crate::to_string(&container).unwrap();
        assert!(yaml.contains("1"));
    }

    #[test]
    fn test_singleton_map_optional_serialize_none() {
        // Test serializing None without skip_serializing_if
        #[derive(Debug, serde::Serialize)]
        struct NoneContainer {
            name: String,
            #[serde(with = "crate::with::singleton_map_optional")]
            item: Option<TestEnum>,
        }

        let container = NoneContainer {
            name: "test".to_string(),
            item: None,
        };

        let yaml = crate::to_string(&container).unwrap();
        // Should serialize None as null
        assert!(yaml.contains("name: test"));
    }

    #[test]
    fn test_singleton_map_optional_deserialize_none() {
        // Test deserializing null as None
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct NoneContainer {
            name: String,
            #[serde(with = "crate::with::singleton_map_optional", default)]
            item: Option<TestEnum>,
        }

        let yaml = "name: test\nitem: null\n";
        let parsed: NoneContainer = crate::from_str(yaml).unwrap();
        assert_eq!(parsed.name, "test");
        assert!(parsed.item.is_none());
    }
}
