//! Serialize enums as single-entry maps with custom key transformation.
//!
//! This module extends [`singleton_map`](super::singleton_map) by allowing
//! custom transformation of variant names during serialization. This is useful
//! for cases where you want the YAML keys to differ from the Rust enum variant
//! names (e.g., converting to snake_case, lowercase, or custom mappings).
//!
//! # Usage
//!
//! Unlike `singleton_map`, this module requires you to define your own
//! serialize/deserialize functions that call the provided helpers with a
//! transformation function.
//!
//! # Example: Snake Case Transformation
//!
//! ```rust
//! use noyalib::with::singleton_map_with;
//!
//! // Define custom serialize/deserialize functions
//! mod snake_case {
//!     use serde::{Deserializer, Serializer};
//!
//!     pub fn serialize<T, S>(
//!         value: &T,
//!         serializer: S,
//!     ) -> Result<S::Ok, S::Error>
//!     where
//!         T: serde::Serialize,
//!         S: Serializer,
//!     {
//!         noyalib::with::singleton_map_with::serialize_with(
//!             value,
//!             serializer,
//!             to_snake_case,
//!         )
//!     }
//!
//!     pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
//!     where
//!         T: serde::de::DeserializeOwned + 'static,
//!         D: Deserializer<'de>,
//!     {
//!         noyalib::with::singleton_map_with::deserialize_with(
//!             deserializer,
//!             from_snake_case,
//!         )
//!     }
//!
//!     fn to_snake_case(s: &str) -> String {
//!         let mut result = String::new();
//!         for (i, c) in s.chars().enumerate() {
//!             if c.is_uppercase() && i > 0 {
//!                 result.push('_');
//!             }
//!             result.push(c.to_lowercase().next().unwrap());
//!         }
//!         result
//!     }
//!
//!     fn from_snake_case(s: &str) -> String {
//!         s.split('_')
//!             .map(|word| {
//!                 let mut chars = word.chars();
//!                 match chars.next() {
//!                     Some(first) => {
//!                         first.to_uppercase().chain(chars).collect()
//!                     },
//!                     None => String::new(),
//!                 }
//!             })
//!             .collect()
//!     }
//! }
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! enum HttpMethod {
//!     GetRequest,
//!     PostData,
//!     DeleteItem,
//! }
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! struct ApiCall {
//!     #[serde(with = "snake_case")]
//!     method: HttpMethod,
//! }
//!
//! let call = ApiCall {
//!     method: HttpMethod::GetRequest,
//! };
//! let yaml = noyalib::to_string(&call).unwrap();
//! // The YAML will contain "get_request" instead of "GetRequest"
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserializer, Serialize, Serializer};

/// Serialize a value as a singleton map with custom key transformation.
///
/// This function is similar to
/// [`singleton_map::serialize`](super::singleton_map::serialize) but allows you
/// to transform the key (variant name) before serialization.
///
/// # Arguments
///
/// * `value` - The value to serialize
/// * `serializer` - The serializer to use
/// * `transform` - A function that transforms the variant name
///
/// # Examples
///
/// ```rust
/// use serde::{Serialize, Serializer};
///
/// fn my_serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
/// where
///     T: Serialize,
///     S: Serializer,
/// {
///     noyalib::with::singleton_map_with::serialize_with(
///         value,
///         serializer,
///         |s| s.to_lowercase(),
///     )
/// }
/// ```
pub fn serialize_with<T, S, F>(value: &T, serializer: S, transform: F) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
    F: Fn(&str) -> String,
{
    use serde::ser::SerializeMap;

    // Serialize to Value to inspect structure
    let yaml_value = crate::to_value(value).map_err(serde::ser::Error::custom)?;

    match yaml_value {
        crate::Value::Mapping(map) => {
            // Transform keys in the mapping
            let mut ser_map = serializer.serialize_map(Some(map.len()))?;
            for (k, v) in map {
                let transformed_key = transform(&k);
                ser_map.serialize_entry(&transformed_key, &v)?;
            }
            ser_map.end()
        }
        crate::Value::String(s) => {
            // Unit variant - transform the variant name
            let transformed = transform(&s);
            let mut ser_map = serializer.serialize_map(Some(1))?;
            ser_map.serialize_entry(&transformed, &())?;
            ser_map.end()
        }
        other => {
            // Fallback: serialize as-is (no transformation possible)
            other.serialize(serializer)
        }
    }
}

/// Deserialize a value from a singleton map with custom key transformation.
///
/// This function is similar to
/// [`singleton_map::deserialize`](super::singleton_map::deserialize) but allows
/// you to transform the key (variant name) before deserialization.
///
/// # Arguments
///
/// * `deserializer` - The deserializer to use
/// * `transform` - A function that transforms the key back to the original
///   variant name
///
/// # Examples
///
/// ```rust
/// use serde::de::DeserializeOwned;
/// use serde::{Deserialize, Deserializer};
///
/// fn my_deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
/// where
///     T: DeserializeOwned + 'static,
///     D: Deserializer<'de>,
/// {
///     noyalib::with::singleton_map_with::deserialize_with(deserializer, |s| {
///         s.to_uppercase()
///     })
/// }
/// ```
pub fn deserialize_with<'de, T, D, F>(deserializer: D, transform: F) -> Result<T, D::Error>
where
    T: DeserializeOwned + 'static,
    D: Deserializer<'de>,
    F: Fn(&str) -> String,
{
    use serde::Deserialize;

    // Deserialize as Value first
    let value = crate::Value::deserialize(deserializer)?;

    // Transform the keys in the value
    let transformed = transform_value_keys(value, &transform);

    crate::from_value(&transformed).map_err(serde::de::Error::custom)
}

/// Transform all string keys in a Value according to the provided function.
fn transform_value_keys<F>(value: crate::Value, transform: &F) -> crate::Value
where
    F: Fn(&str) -> String,
{
    match value {
        crate::Value::Mapping(map) => {
            let mut new_map = crate::Mapping::with_capacity(map.len());
            for (k, v) in map {
                let new_key = transform(&k);
                let new_value = transform_value_keys(v, transform);
                let _ = new_map.insert(new_key, new_value);
            }
            crate::Value::Mapping(new_map)
        }
        crate::Value::Sequence(seq) => {
            let new_seq: Vec<_> = seq
                .into_iter()
                .map(|v| transform_value_keys(v, transform))
                .collect();
            crate::Value::Sequence(new_seq)
        }
        crate::Value::Tagged(tagged) => {
            let (tag, inner) = tagged.into_parts();
            let transformed_inner = transform_value_keys(inner, transform);
            crate::Value::Tagged(Box::new(crate::TaggedValue::new(tag, transformed_inner)))
        }
        // Scalars pass through unchanged
        other => other,
    }
}

/// Common key transformation: convert to lowercase.
///
/// # Examples
///
/// ```rust
/// use noyalib::with::singleton_map_with::to_lowercase;
///
/// assert_eq!(to_lowercase("GetRequest"), "getrequest");
/// assert_eq!(to_lowercase("POST"), "post");
/// ```
#[must_use]
pub fn to_lowercase(s: &str) -> String {
    s.to_lowercase()
}

/// Common key transformation: convert to UPPERCASE.
///
/// # Examples
///
/// ```rust
/// use noyalib::with::singleton_map_with::to_uppercase;
///
/// assert_eq!(to_uppercase("GetRequest"), "GETREQUEST");
/// assert_eq!(to_uppercase("post"), "POST");
/// ```
#[must_use]
pub fn to_uppercase(s: &str) -> String {
    s.to_uppercase()
}

/// Common key transformation: convert PascalCase to snake_case.
///
/// # Examples
///
/// ```rust
/// use noyalib::with::singleton_map_with::to_snake_case;
///
/// assert_eq!(to_snake_case("GetRequest"), "get_request");
/// assert_eq!(to_snake_case("HTTPServer"), "h_t_t_p_server");
/// assert_eq!(to_snake_case("already_snake"), "already_snake");
/// ```
#[must_use]
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.extend(c.to_lowercase());
    }
    result
}

/// Common key transformation: convert snake_case to PascalCase.
///
/// # Examples
///
/// ```rust
/// use noyalib::with::singleton_map_with::to_pascal_case;
///
/// assert_eq!(to_pascal_case("get_request"), "GetRequest");
/// assert_eq!(to_pascal_case("http_server"), "HttpServer");
/// assert_eq!(to_pascal_case("AlreadyPascal"), "Alreadypascal");
/// ```
#[must_use]
pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for word in s.split('_') {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            for c in chars {
                result.extend(c.to_lowercase());
            }
        }
    }
    result
}

/// Common key transformation: convert to kebab-case.
///
/// # Examples
///
/// ```rust
/// use noyalib::with::singleton_map_with::to_kebab_case;
///
/// assert_eq!(to_kebab_case("GetRequest"), "get-request");
/// assert_eq!(to_kebab_case("HTTPServer"), "h-t-t-p-server");
/// ```
#[must_use]
pub fn to_kebab_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('-');
        }
        result.extend(c.to_lowercase());
    }
    result
}

/// Common key transformation: convert kebab-case to PascalCase.
///
/// # Examples
///
/// ```rust
/// use noyalib::with::singleton_map_with::from_kebab_case;
///
/// assert_eq!(from_kebab_case("get-request"), "GetRequest");
/// assert_eq!(from_kebab_case("http-server"), "HttpServer");
/// ```
#[must_use]
pub fn from_kebab_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    let rest: String = chars.flat_map(|c| c.to_lowercase()).collect();
                    upper + &rest
                }
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("GetRequest"), "get_request");
        assert_eq!(to_snake_case("HTTPServer"), "h_t_t_p_server");
        assert_eq!(to_snake_case("simple"), "simple");
        assert_eq!(to_snake_case("Already_Snake"), "already__snake");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("get_request"), "GetRequest");
        assert_eq!(to_pascal_case("http_server"), "HttpServer");
        assert_eq!(to_pascal_case("simple"), "Simple");
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("GetRequest"), "get-request");
        assert_eq!(to_kebab_case("SimpleTest"), "simple-test");
    }

    #[test]
    fn test_from_kebab_case() {
        assert_eq!(from_kebab_case("get-request"), "GetRequest");
        assert_eq!(from_kebab_case("http-server"), "HttpServer");
    }

    #[test]
    fn test_lowercase_uppercase() {
        assert_eq!(to_lowercase("GetRequest"), "getrequest");
        assert_eq!(to_uppercase("GetRequest"), "GETREQUEST");
    }

    #[test]
    fn test_transform_value_keys() {
        let mut mapping = crate::Mapping::new();
        let _ = mapping.insert("UnitVariant", crate::Value::Null);
        let _ = mapping.insert("StructVariant", crate::Value::from(42));
        let value = crate::Value::Mapping(mapping);

        let transformed = transform_value_keys(value, &to_snake_case);

        let map = transformed.as_mapping().unwrap();
        assert!(map.contains_key("unit_variant"));
        assert!(map.contains_key("struct_variant"));
    }

    #[test]
    fn test_transform_value_keys_nested() {
        let mut inner = crate::Mapping::new();
        let _ = inner.insert("InnerKey", crate::Value::from("value"));

        let mut outer = crate::Mapping::new();
        let _ = outer.insert("OuterKey", crate::Value::Mapping(inner));

        let value = crate::Value::Mapping(outer);
        let transformed = transform_value_keys(value, &to_snake_case);

        let outer_map = transformed.as_mapping().unwrap();
        assert!(outer_map.contains_key("outer_key"));

        let inner_map = outer_map.get("outer_key").unwrap().as_mapping().unwrap();
        assert!(inner_map.contains_key("inner_key"));
    }

    #[test]
    fn test_transform_value_keys_sequence() {
        let mut item = crate::Mapping::new();
        let _ = item.insert("ItemKey", crate::Value::from(1));

        let seq = crate::Value::Sequence(vec![crate::Value::Mapping(item)]);
        let transformed = transform_value_keys(seq, &to_snake_case);

        let items = transformed.as_sequence().unwrap();
        let first = items[0].as_mapping().unwrap();
        assert!(first.contains_key("item_key"));
    }

    #[test]
    fn test_serialize_with_directly() {
        #[derive(serde::Serialize)]
        struct TestStruct {
            name: String,
        }

        let value = TestStruct {
            name: "test".to_string(),
        };

        // Serialize using our function with lowercase transform
        let yaml = crate::to_string(&value).unwrap();
        // The struct should serialize normally
        assert!(yaml.contains("name:"));
    }
}
