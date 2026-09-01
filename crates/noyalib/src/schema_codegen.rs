// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! JSON Schema codegen for Rust types.
//!
//! Derive [`JsonSchema`] for a Rust type and emit a JSON Schema
//! 2020-12 document that describes its YAML shape. Useful for:
//!
//! - **Contract sharing.** Hand the generated schema to teams that
//!   consume your YAML configs (CI, CRD generators, IDE plugins,
//!   doc sites) without re-deriving the contract by hand.
//! - **Self-documenting configs.** `#[doc]` strings on a struct's
//!   fields propagate into the schema's `description` field, so the
//!   schema doubles as the manual.
//! - **Cross-language workflow.** A YAML schema artefact can be
//!   consumed by any JSON Schema implementation in any language.
//!
//! This module is gated behind the `schema` Cargo feature (off by
//! default). Schema *validation* of a YAML document against the
//! emitted schema lives in [`crate::validate_against_schema`] —
//! enable the `validate-schema` feature for that path.
//!
//! # Examples
//!
//! ```
//! use noyalib::{schema_for, schema_for_yaml, JsonSchema};
//!
//! #[derive(serde::Serialize, serde::Deserialize, JsonSchema)]
//! struct ServerConfig {
//!     /// Port the server binds on.
//!     port: u16,
//!     /// Hostname or IP.
//!     host: String,
//! }
//!
//! // As a `noyalib::Value` for further processing.
//! let v = schema_for::<ServerConfig>().unwrap();
//! assert_eq!(v["title"].as_str(), Some("ServerConfig"));
//!
//! // As YAML text for sharing.
//! let yaml = schema_for_yaml::<ServerConfig>().unwrap();
//! assert!(yaml.contains("title: ServerConfig"));
//! assert!(yaml.contains("port:"));
//! ```

use crate::error::{Error, Result};
// Via the prelude, not `std`: `schema` without `std` is a valid
// combination (schemars and serde_json both build on alloc), checked
// by the weekly feature-powerset sweep.
use crate::prelude::{Cow, String, format};
use crate::value::Value;

/// Re-export of the `schemars` derive macro and trait. Deriving
/// [`JsonSchema`] on your Rust type is what makes
/// [`schema_for`] / [`schema_for_yaml`] produce a schema for it.
///
/// # Downstream `Cargo.toml`
///
/// The proc-macro that powers `#[derive(JsonSchema)]` expands to
/// code that references `::schemars::...` paths in the *call
/// site's* dep graph. That means downstream crates that derive
/// [`JsonSchema`] must add `schemars` to their own
/// `Cargo.toml` alongside `noyalib`:
///
/// ```toml
/// [dependencies]
/// noyalib = { version = "0.0.7", features = ["schema"] }
/// schemars = "1.2"
/// ```
///
/// (Same precedent as `serde::Deserialize` — derive macros that
/// re-export through a wrapper crate cannot rewrite the
/// generated paths without an explicit
/// `#[schemars(crate = "...")]` attribute on every type.)
///
/// # Examples
///
/// ```
/// use noyalib::JsonSchema;
///
/// #[derive(JsonSchema)]
/// struct Cfg { port: u16 }
/// ```
pub use schemars::JsonSchema;

/// Generate the JSON Schema 2020-12 document for `T` and parse it
/// into a [`crate::Value`] tree, ready for indexing, transcoding,
/// or further programmatic walking.
///
/// The schema honours `#[doc]` comments on fields (they become
/// `description` properties), `#[serde(rename = "...")]` (renames
/// the schema property), `#[serde(default)]` (drops the property
/// from `required`), and the rest of the conventions documented
/// upstream in [`schemars`].
///
/// # Errors
///
/// - The schema serializer fails (vanishingly unlikely; schemas
///   never contain non-serializable shapes).
/// - The serialized JSON cannot be re-parsed as YAML (would
///   indicate a noyalib parser bug, since JSON ⊂ YAML 1.2).
///
/// # Examples
///
/// ```
/// use noyalib::{schema_for, JsonSchema};
///
/// #[derive(JsonSchema)]
/// struct Cfg { port: u16 }
///
/// let schema = schema_for::<Cfg>().unwrap();
/// assert_eq!(schema["type"].as_str(), Some("object"));
/// ```
pub fn schema_for<T: JsonSchema>() -> Result<Value> {
    let mut generator = schemars::SchemaGenerator::default();
    let schema = generator.root_schema_for::<T>();
    // JSON ⊂ YAML 1.2 — round-trip through JSON text is the most
    // direct path that doesn't require a `Schema` → `Value`
    // hand-mapping.
    let json = serde_json::to_string(&schema)
        .map_err(|e| Error::Parse(format!("schema_for: schema serialization failed: {e}")))?;
    crate::from_str::<Value>(&json)
}

/// Generate the JSON Schema 2020-12 document for `T` and emit it
/// as YAML text — ready to write to disk, share with downstream
/// consumers, or check into version control alongside the type
/// definition.
///
/// The output uses noyalib's standard serializer settings; round-
/// tripping the YAML back through [`crate::from_str`] yields an
/// equivalent [`Value`] tree.
///
/// # Errors
///
/// As [`schema_for`], plus YAML emission errors (likewise extremely
/// unlikely for schema-shaped data).
///
/// # Examples
///
/// ```
/// use noyalib::{schema_for_yaml, JsonSchema};
///
/// #[derive(JsonSchema)]
/// struct Cfg { port: u16 }
///
/// let yaml = schema_for_yaml::<Cfg>().unwrap();
/// assert!(yaml.contains("type: object"));
/// ```
pub fn schema_for_yaml<T: JsonSchema>() -> Result<String> {
    let mut generator = schemars::SchemaGenerator::default();
    let schema = generator.root_schema_for::<T>();
    crate::to_string(&schema)
}

/// `JsonSchema` for [`crate::Value`] — the dynamic value tree
/// admits any JSON-expressible shape, so the schema we emit is
/// the JSON Schema 2020-12 idiom for "any value": `{"oneOf":
/// [...]}` enumerating null, boolean, number, string, array,
/// and object. Object values use a recursive `additionalProperties`
/// reference so deeply-nested `Value` payloads typecheck without
/// a depth bound.
///
/// This impl lets users derive [`JsonSchema`] on a struct that
/// has a `noyalib::Value` field — e.g. when the field's type is
/// "any user-supplied YAML scalar / mapping / sequence" and the
/// caller still wants a published schema for the surrounding
/// shape.
///
/// # Examples
///
/// ```
/// use noyalib::{schema_for, JsonSchema, Value};
///
/// #[derive(serde::Serialize, serde::Deserialize, JsonSchema)]
/// struct Envelope {
///     id: String,
///     payload: Value,
/// }
///
/// let schema = schema_for::<Envelope>().unwrap();
/// // The `payload` field is described by the `Value` schema —
/// // a oneOf union covering every JSON-expressible shape.
/// assert_eq!(schema["type"].as_str(), Some("object"));
/// assert!(matches!(schema["properties"]["payload"], Value::Mapping(_)));
/// ```
impl JsonSchema for Value {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("YamlValue")
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed("noyalib::Value")
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        // `serde_json::json!` is the path-of-least-resistance to
        // a `Schema` value. The shape mirrors what
        // `schemars::Schema::default()` plus a `oneOf` would
        // produce by hand, but the macro keeps the noise down.
        schemars::Schema::try_from(serde_json::json!({
            "oneOf": [
                { "type": "null" },
                { "type": "boolean" },
                { "type": "number" },
                { "type": "string" },
                {
                    "type": "array",
                    "items": { "$ref": "#/$defs/YamlValue" }
                },
                {
                    "type": "object",
                    "additionalProperties": { "$ref": "#/$defs/YamlValue" }
                }
            ],
            "title": "YamlValue",
            "description": "Any YAML 1.2 scalar, sequence, or mapping value. \
                            Recursive — sequence items and mapping values are \
                            themselves `YamlValue`s."
        }))
        .expect("YamlValue schema must be a valid JSON Schema document")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct Cfg {
        port: u16,
        name: String,
    }

    #[test]
    fn schema_for_returns_object_schema() {
        let v = schema_for::<Cfg>().unwrap();
        assert_eq!(v["type"].as_str(), Some("object"));
        assert_eq!(v["title"].as_str(), Some("Cfg"));
        let required = match &v["required"] {
            Value::Sequence(s) => s.clone(),
            other => panic!("expected required to be a sequence, got {other:?}"),
        };
        let names: Vec<&str> = required.iter().filter_map(Value::as_str).collect();
        assert!(names.contains(&"port"));
        assert!(names.contains(&"name"));
    }

    #[test]
    fn schema_for_yaml_round_trips_to_value() {
        let yaml = schema_for_yaml::<Cfg>().unwrap();
        let parsed: Value = crate::from_str(&yaml).unwrap();
        let direct = schema_for::<Cfg>().unwrap();
        assert_eq!(parsed, direct);
    }

    #[test]
    fn schema_records_field_constraints() {
        let v = schema_for::<Cfg>().unwrap();
        let port = &v["properties"]["port"];
        assert_eq!(port["type"].as_str(), Some("integer"));
        // schemars 1.x emits min/max for fixed-width integers.
        assert_eq!(port["minimum"].as_i64(), Some(0));
        assert_eq!(port["maximum"].as_i64(), Some(65_535));
    }

    #[derive(serde::Serialize, serde::Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct WithDoc {
        /// Bound TCP port.
        port: u16,
    }

    #[test]
    fn doc_comments_become_descriptions() {
        let v = schema_for::<WithDoc>().unwrap();
        let desc = v["properties"]["port"]["description"].as_str();
        assert_eq!(desc, Some("Bound TCP port."));
    }

    #[derive(serde::Serialize, serde::Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct WithDefault {
        port: u16,
        #[serde(default)]
        host: String,
    }

    #[test]
    fn serde_default_drops_field_from_required() {
        let v = schema_for::<WithDefault>().unwrap();
        let required = match &v["required"] {
            Value::Sequence(s) => s.clone(),
            other => panic!("expected required, got {other:?}"),
        };
        let names: Vec<&str> = required.iter().filter_map(Value::as_str).collect();
        assert!(names.contains(&"port"));
        assert!(
            !names.contains(&"host"),
            "default-bearing field should not be required"
        );
    }

    #[derive(serde::Serialize, serde::Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct Renamed {
        #[serde(rename = "bind_port")]
        port: u16,
    }

    #[test]
    fn serde_rename_renames_schema_property() {
        let v = schema_for::<Renamed>().unwrap();
        let props = match &v["properties"] {
            Value::Mapping(m) => m.clone(),
            other => panic!("expected properties to be a Mapping, got {other:?}"),
        };
        assert!(props.contains_key("bind_port"));
        assert!(!props.contains_key("port"));
    }
}
