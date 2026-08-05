//! Serialize enums as single-entry maps.
//!
//! This module provides `serialize` and `deserialize` functions for use with
//! serde's `#[serde(with = "...")]` attribute to serialize enum variants as
//! single-entry maps where the key is the variant name.
//!
//! # Examples
//!
//! ```rust
//! use noyalib::with::singleton_map;
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! enum Status {
//!     Active,
//!     Pending { reason: String },
//!     Error { code: i32, message: String },
//! }
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! struct Task {
//!     name: String,
//!     #[serde(with = "singleton_map")]
//!     status: Status,
//! }
//!
//! let task = Task {
//!     name: "example".to_string(),
//!     status: Status::Pending {
//!         reason: "waiting".to_string(),
//!     },
//! };
//!
//! let yaml = noyalib::to_string(&task).unwrap();
//! // Output:
//! // name: example
//! // status:
//! //   Pending:
//! //     reason: waiting
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

/// Serialize a value as a singleton map.
///
/// For enums, this serializes the variant as a map where the key is the
/// variant name and the value is the variant's data.
///
/// # Examples
///
/// ```rust
/// use noyalib::with::singleton_map;
///
/// #[derive(Debug, serde::Serialize, serde::Deserialize)]
/// enum Action {
///     Start,
///     Stop { graceful: bool },
/// }
///
/// #[derive(Debug, serde::Serialize, serde::Deserialize)]
/// struct Command {
///     #[serde(with = "singleton_map")]
///     action: Action,
/// }
/// ```
pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: serde_core::Serialize,
    S: serde_core::Serializer,
{
    use serde_core::ser::SerializeMap as _;

    // Serialize to Value to inspect structure
    let yaml_value = crate::to_value(value).map_err(serde_core::ser::Error::custom)?;

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
            serde_core::Serialize::serialize(&other, serializer)
        }
    }
}

/// Deserialize a value from a singleton map.
///
/// This is the counterpart to [`serialize`], deserializing from the
/// singleton map format back to the original type.
///
/// # Examples
///
/// ```
/// use noyalib::with::singleton_map;
///
/// #[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug)]
/// enum Status { Active }
///
/// #[derive(serde::Deserialize, Debug)]
/// struct Doc {
///     #[serde(with = "singleton_map")]
///     s: Status,
/// }
///
/// let d: Doc = noyalib::from_str("s:\n  Active: null\n").unwrap();
/// assert_eq!(d.s, Status::Active);
/// ```
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: serde_core::de::DeserializeOwned + 'static,
    D: serde_core::Deserializer<'de>,
{
    use serde_core::Deserialize as _;
    // Deserialize as Value first
    let value = crate::Value::deserialize(deserializer)?;
    crate::from_value(&value).map_err(serde_core::de::Error::custom)
}

#[cfg(test)]
mod tests {

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    enum TestEnum {
        Unit,
        Newtype(i32),
        Struct { value: String },
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Container {
        name: String,
        #[serde(with = "crate::with::singleton_map")]
        item: TestEnum,
    }

    #[test]
    fn test_singleton_map_unit_variant() {
        let container = Container {
            name: "test".to_string(),
            item: TestEnum::Unit,
        };

        let yaml = crate::to_string(&container).unwrap();
        assert!(yaml.contains("Unit"));

        let parsed: Container = crate::from_str(&yaml).unwrap();
        assert_eq!(parsed, container);
    }

    #[test]
    fn test_singleton_map_struct_variant() {
        let container = Container {
            name: "test".to_string(),
            item: TestEnum::Struct {
                value: "hello".to_string(),
            },
        };

        let yaml = crate::to_string(&container).unwrap();
        assert!(yaml.contains("Struct"));
        assert!(yaml.contains("value: hello"));

        let parsed: Container = crate::from_str(&yaml).unwrap();
        assert_eq!(parsed, container);
    }

    #[test]
    fn test_singleton_map_fallback_sequence() {
        // Test the fallback path when value is a Sequence
        #[derive(Debug, serde::Serialize)]
        struct SeqContainer {
            #[serde(with = "crate::with::singleton_map")]
            items: Vec<i32>,
        }

        let container = SeqContainer {
            items: vec![1, 2, 3],
        };

        let yaml = crate::to_string(&container).unwrap();
        assert!(yaml.contains("1"));
        assert!(yaml.contains("2"));
        assert!(yaml.contains("3"));
    }

    #[test]
    fn test_singleton_map_fallback_integer() {
        // Test the fallback path when value is a Number
        #[derive(Debug, serde::Serialize)]
        struct NumContainer {
            #[serde(with = "crate::with::singleton_map")]
            value: i32,
        }

        let container = NumContainer { value: 42 };

        let yaml = crate::to_string(&container).unwrap();
        assert!(yaml.contains("42"));
    }
}
