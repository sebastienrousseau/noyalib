// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! JSON Schema 2020-12 validation against a parsed [`crate::Value`].
//!
//! The schema document may itself be expressed as YAML (since JSON
//! is a subset of YAML 1.2). Pairs naturally with the codegen
//! surface: derive [`crate::JsonSchema`] on a Rust type, emit the
//! schema with [`crate::schema_for_yaml`], then enforce it on
//! inputs that arrive as YAML at runtime.
//!
//! Gated behind the `validate-schema` Cargo feature (which implies
//! `schema`).
//!
//! # Hardening guarantees
//!
//! Two properties hold and are pinned by `tests/schema_hardening.rs`:
//!
//! - **External `$ref` is never dereferenced.** A schema referencing a
//!   remote URI is refused, not fetched. `jsonschema` is declared
//!   `default-features = false`, so its `resolve-http` support is
//!   absent; the tests assert both the refusal and that it is fast,
//!   since a network attempt would be slow.
//! - **Schema recursion is bounded.** Deeply nested schemas are refused
//!   with `recursion depth limit exceeded` rather than exhausting the
//!   stack — a stack overflow aborts the process instead of returning an
//!   error a caller can handle. The bound belongs to `jsonschema`.
//!
//! Local `$ref` and `$defs` are unaffected; they are the composition
//! mechanism JSON Schema 2020-12 tool definitions rely on.
//!
//! Both properties are inherited from how the dependency is configured
//! rather than implemented here, which is exactly why they are tested:
//! a feature flag or a dependency bump could remove either silently.
//!
//! # Examples
//!
//! ```
//! use noyalib::{from_str, validate_against_schema, Value};
//!
//! let schema_yaml = "\
//! type: object
//! properties:
//!   port:
//!     type: integer
//!     minimum: 0
//!     maximum: 65535
//! required:
//!   - port
//! ";
//! let schema: Value = from_str(schema_yaml).unwrap();
//!
//! let good: Value = from_str("port: 8080\n").unwrap();
//! assert!(validate_against_schema(&good, &schema).is_ok());
//!
//! let bad: Value = from_str("port: not-a-number\n").unwrap();
//! assert!(validate_against_schema(&bad, &schema).is_err());
//! ```

use crate::error::{Error, Result};
// Via the prelude, not `std`: `validate-schema` without `std` is a
// valid combination (`jsonschema` is carried with
// `default-features = false`), checked by the weekly feature-powerset
// sweep.
use crate::prelude::*;
use crate::value::{Number, Value};

/// Validate `value` against the JSON Schema 2020-12 document
/// `schema`. Both inputs are [`Value`] trees — the schema is
/// usually loaded from a YAML / JSON file via [`crate::from_str`],
/// or built programmatically.
///
/// Multiple violations are aggregated into a single error message,
/// each line carrying the JSON-pointer path of the offending
/// instance. The path syntax follows RFC 6901: `/` for the root,
/// `/port` for a top-level field, `/items/0/name` for nested
/// data. A schema-side build failure (malformed schema document)
/// is reported separately so callers can distinguish "your schema
/// is broken" from "your data is broken".
///
/// # Errors
///
/// - The schema cannot be compiled (invalid JSON Schema shape).
/// - The instance violates one or more constraints declared in
///   the schema.
/// - Internal JSON serialization fails for either input
///   (vanishingly unlikely — would indicate a noyalib serializer
///   bug).
///
/// # Examples
///
/// ```
/// use noyalib::{from_str, validate_against_schema, Value};
///
/// let schema: Value = from_str(
///     "type: object\nrequired: [port]\nproperties:\n  port:\n    type: integer\n",
/// ).unwrap();
/// let v: Value = from_str("port: 8080\n").unwrap();
/// validate_against_schema(&v, &schema).unwrap();
/// ```
pub fn validate_against_schema(value: &Value, schema: &Value) -> Result<()> {
    CompiledSchema::compile(schema)?.validate(value)
}

/// A JSON Schema 2020-12 document compiled once, for validating many
/// instances without re-compiling the schema on every call (#329).
///
/// [`validate_against_schema`] compiles its schema argument on every
/// invocation — the right trade-off for one-off checks, and a linear
/// waste for a config pipeline that validates thousands of documents
/// against a handful of schemas. `CompiledSchema` front-loads the
/// compile:
///
/// ```
/// use noyalib::{CompiledSchema, Value, from_str};
///
/// let schema: Value = from_str(
///     "type: object\nrequired: [port]\nproperties:\n  port:\n    type: integer\n",
/// ).unwrap();
/// let compiled = CompiledSchema::compile(&schema).unwrap();
///
/// for doc in ["port: 1\n", "port: 2\n", "port: 3\n"] {
///     let v: Value = from_str(doc).unwrap();
///     compiled.validate(&v).unwrap(); // no per-call schema compile
/// }
/// ```
///
/// The hardening guarantees pinned by `tests/schema_hardening.rs`
/// hold on this path too — [`validate_against_schema`] is a
/// compile-then-validate through this type, so the external-`$ref`
/// refusal and the bounded recursion are exercised by the same tests.
///
/// Format assertion (off by default under Draft 2020-12, where
/// `format` is an annotation) and custom formats are opt-in through
/// [`CompiledSchema::builder`].
pub struct CompiledSchema {
    validator: jsonschema::Validator,
}

impl fmt::Debug for CompiledSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledSchema").finish_non_exhaustive()
    }
}

impl CompiledSchema {
    /// Compile `schema` with the same configuration
    /// [`validate_against_schema`] uses: Draft 2020-12 semantics,
    /// `format` as an annotation, no external `$ref` resolution.
    ///
    /// # Errors
    ///
    /// - The schema cannot be converted to JSON (YAML-only constructs
    ///   like NaN keys).
    /// - The schema is not a valid JSON Schema.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{CompiledSchema, Value, from_str};
    ///
    /// let schema: Value = from_str("type: object\n").unwrap();
    /// let compiled = CompiledSchema::compile(&schema).unwrap();
    /// assert!(compiled.validate(&from_str("a: 1\n").unwrap()).is_ok());
    /// ```
    pub fn compile(schema: &Value) -> Result<Self> {
        Self::builder(schema).build()
    }

    /// Start a builder over `schema`, for turning on format assertion
    /// or registering custom formats before compiling.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{CompiledSchema, Value, from_str};
    ///
    /// let schema: Value = from_str(
    ///     "type: object\nproperties:\n  date: { type: string, format: date }\n",
    /// ).unwrap();
    /// let compiled = CompiledSchema::builder(&schema)
    ///     .validate_formats(true)
    ///     .build()
    ///     .unwrap();
    /// let bad: Value = from_str("date: 01/15/2024\n").unwrap();
    /// assert!(compiled.validate(&bad).is_err());
    /// ```
    #[must_use]
    pub fn builder(schema: &Value) -> CompiledSchemaBuilder {
        CompiledSchemaBuilder {
            schema_json: value_to_json(schema),
            validate_formats: None,
            formats: Vec::new(),
        }
    }

    /// Validate `value`, aggregating every violation into one error
    /// message with RFC 6901 instance paths — the same report
    /// [`validate_against_schema`] produces.
    ///
    /// # Errors
    ///
    /// - The instance violates one or more schema constraints.
    /// - The instance cannot be converted to JSON.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{CompiledSchema, Value, from_str};
    ///
    /// let schema: Value = from_str(
    ///     "type: object\nproperties:\n  port: { type: integer }\n",
    /// ).unwrap();
    /// let compiled = CompiledSchema::compile(&schema).unwrap();
    /// let err = compiled
    ///     .validate(&from_str("port: hello\n").unwrap())
    ///     .unwrap_err();
    /// assert!(err.to_string().contains("/port"));
    /// ```
    pub fn validate(&self, value: &Value) -> Result<()> {
        let instance_json = value_to_json(value)
            .map_err(|e| Error::Parse(format!("validate_against_schema: value -> JSON: {e}")))?;

        if self.validator.is_valid(&instance_json) {
            return Ok(());
        }

        let mut messages: Vec<String> = Vec::new();
        for err in self.validator.iter_errors(&instance_json) {
            messages.push(format!("{} (at `{}`)", err, err.instance_path()));
        }
        let summary = if messages.len() == 1 {
            format!("schema violation: {}", messages[0])
        } else {
            let joined = messages.join("\n  - ");
            format!(
                "schema violations ({} total):\n  - {}",
                messages.len(),
                joined
            )
        };
        Err(Error::Custom(summary))
    }

    /// Every violation for `value`, structured: the RFC 6901 path of
    /// the offending instance node, the schema keyword that raised it,
    /// and the human-readable message. Empty when `value` conforms.
    ///
    /// # Errors
    ///
    /// The instance cannot be converted to JSON (vanishingly
    /// unlikely — would indicate a noyalib serializer bug).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{CompiledSchema, Value, from_str};
    ///
    /// let schema: Value = from_str(
    ///     "type: object\nrequired: [host]\nproperties:\n  port: { type: integer }\n",
    /// ).unwrap();
    /// let compiled = CompiledSchema::compile(&schema).unwrap();
    /// let violations = compiled
    ///     .iter_errors(&from_str("port: hello\n").unwrap())
    ///     .unwrap();
    /// assert_eq!(violations.len(), 2);
    /// assert!(violations.iter().any(|v| v.instance_path == "/port"));
    /// assert!(violations.iter().any(|v| v.keyword == "required"));
    /// ```
    pub fn iter_errors(&self, value: &Value) -> Result<Vec<SchemaViolation>> {
        let instance_json = value_to_json(value)
            .map_err(|e| Error::Parse(format!("iter_errors: value -> JSON: {e}")))?;
        Ok(self
            .validator
            .iter_errors(&instance_json)
            .map(|err| SchemaViolation {
                instance_path: err.instance_path().to_string(),
                keyword: err
                    .schema_path()
                    .to_string()
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                message: err.to_string(),
            })
            .collect())
    }
}

/// Configures a [`CompiledSchema`] before compilation. Created by
/// [`CompiledSchema::builder`].
pub struct CompiledSchemaBuilder {
    schema_json: core::result::Result<serde_json::Value, String>,
    validate_formats: Option<bool>,
    #[allow(clippy::type_complexity)]
    formats: Vec<(String, Arc<dyn Fn(&str) -> bool + Send + Sync>)>,
}

impl fmt::Debug for CompiledSchemaBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledSchemaBuilder")
            .field("validate_formats", &self.validate_formats)
            .field(
                "formats",
                &self.formats.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

impl CompiledSchemaBuilder {
    /// Assert `format` keywords instead of treating them as
    /// annotations. Under Draft 2020-12 the default is annotation-only,
    /// so `format: date` accepts `01/15/2024` unless this is on.
    #[must_use]
    pub fn validate_formats(mut self, yes: bool) -> Self {
        self.validate_formats = Some(yes);
        self
    }

    /// Register a custom format checker for `format: <name>`. Implies
    /// nothing about assertion — combine with
    /// [`Self::validate_formats`] to have the checker enforced.
    #[must_use]
    pub fn with_format<N, F>(mut self, name: N, check: F) -> Self
    where
        N: Into<String>,
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.formats.push((name.into(), Arc::new(check)));
        self
    }

    /// Compile the schema with the accumulated configuration.
    ///
    /// # Errors
    ///
    /// As [`CompiledSchema::compile`].
    pub fn build(self) -> Result<CompiledSchema> {
        let schema_json = self
            .schema_json
            .map_err(|e| Error::Custom(format!("validate_against_schema: schema -> JSON: {e}")))?;
        let mut options = jsonschema::options();
        if let Some(yes) = self.validate_formats {
            options = options.should_validate_formats(yes);
        }
        for (name, check) in self.formats {
            options = options.with_format(name, move |s: &str| check(s));
        }
        let validator = options.build(&schema_json).map_err(|e| {
            Error::Custom(format!(
                "validate_against_schema: schema is not a valid JSON Schema: {e}"
            ))
        })?;
        Ok(CompiledSchema { validator })
    }
}

/// One schema violation from [`CompiledSchema::iter_errors`]:
/// where in the instance, which keyword, and the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaViolation {
    /// RFC 6901 JSON pointer to the offending instance node
    /// (`""` for the root, `/port`, `/items/0/name`).
    pub instance_path: String,
    /// The schema keyword that raised the violation (`type`,
    /// `required`, `format`, …) — the last segment of the schema path.
    pub keyword: String,
    /// Human-readable description of the violation.
    pub message: String,
}

/// Validate the YAML text in `yaml` against the JSON Schema
/// document in `schema_yaml`. Convenience wrapper around
/// [`validate_against_schema`] — parses both inputs and forwards.
///
/// # Errors
///
/// As [`validate_against_schema`], plus YAML parse errors for
/// either input.
///
/// # Examples
///
/// ```
/// use noyalib::validate_against_schema_str;
///
/// let schema = "type: object\nrequired: [port]\n";
/// let yaml = "port: 8080\n";
/// validate_against_schema_str(yaml, schema).unwrap();
/// ```
pub fn validate_against_schema_str(yaml: &str, schema_yaml: &str) -> Result<()> {
    let value: Value = crate::from_str(yaml)?;
    let schema: Value = crate::from_str(schema_yaml)?;
    validate_against_schema(&value, &schema)
}

/// Convert a [`Value`] tree to a [`serde_json::Value`] via the
/// existing `Serialize` impl on `Value`. Lossless for every
/// JSON-expressible shape; YAML-only constructs (NaN, Infinity,
/// non-string keys) become JSON-incompatible at this boundary —
/// JSON Schema does not have semantics for them either, so the
/// validator would reject them downstream regardless.
pub(crate) fn value_to_json(v: &Value) -> core::result::Result<serde_json::Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

/// Apply schema-driven type coercions to `value` in place. Walks
/// the validator output for type-mismatch errors, and for each
/// case where the instance is a [`Value::String`] but the schema
/// requires an integer / number / boolean — and the string parses
/// cleanly into that target type — replaces the offending node
/// with the coerced value.
///
/// Returns the number of coercions applied. Coercions that don't
/// have a clean parse (e.g. `"abc"` against `type: integer`) are
/// left in place; the caller is expected to re-run
/// [`validate_against_schema`] afterwards and surface any
/// remaining violations.
///
/// This pairs naturally with hand-written YAML where every value
/// is a string in the typed sense (because the user did not quote
/// vs. unquote intentionally) but the schema knows the intended
/// types. It fits into a CI / formatter pipeline as a "fix pass"
/// before strict validation kicks in.
///
/// # Errors
///
/// As [`validate_against_schema`].
///
/// # Examples
///
/// ```
/// use noyalib::{coerce_to_schema, from_str, Value};
///
/// let schema: Value = from_str(
///     "type: object\nproperties:\n  port:\n    type: integer\n",
/// ).unwrap();
/// let mut data: Value = from_str("port: \"8080\"\n").unwrap();
/// let n = coerce_to_schema(&mut data, &schema).unwrap();
/// assert_eq!(n, 1, "one fix expected");
/// // The port is now an integer — re-validation succeeds.
/// noyalib::validate_against_schema(&data, &schema).unwrap();
/// ```
pub fn coerce_to_schema(value: &mut Value, schema: &Value) -> Result<usize> {
    use jsonschema::JsonType;
    use jsonschema::error::{TypeKind, ValidationErrorKind};

    let schema_json = value_to_json(schema)
        .map_err(|e| Error::Custom(format!("coerce_to_schema: schema -> JSON: {e}")))?;
    let validator = jsonschema::validator_for(&schema_json).map_err(|e| {
        Error::Custom(format!(
            "coerce_to_schema: schema is not a valid JSON Schema: {e}"
        ))
    })?;

    let mut applied: usize = 0;
    // Cap the fix-loop to bound total work even on adversarial
    // schemas where each coercion exposes a fresh error elsewhere.
    let max_iterations = 1024;

    for _ in 0..max_iterations {
        let instance_json = value_to_json(value)
            .map_err(|e| Error::Parse(format!("coerce_to_schema: value -> JSON: {e}")))?;
        let mut applied_this_pass = false;

        // Collect path + target type pairs first so the borrow on
        // `value` for the subsequent mutation doesn't overlap with
        // the iterator.
        let mut targets: Vec<(String, JsonType)> = Vec::new();
        for err in validator.iter_errors(&instance_json) {
            if let ValidationErrorKind::Type {
                kind: TypeKind::Single(target),
            } = err.kind()
            {
                targets.push((err.instance_path().to_string(), *target));
            }
        }

        for (path, target) in targets {
            let segments = parse_json_pointer(&path);
            if let Some(node) = navigate_mut(value, &segments) {
                if try_coerce(node, target) {
                    applied += 1;
                    applied_this_pass = true;
                }
            }
        }

        if !applied_this_pass {
            break;
        }
    }

    Ok(applied)
}

/// Parse an RFC 6901 JSON pointer into segment strings.
fn parse_json_pointer(s: &str) -> Vec<String> {
    if s.is_empty() || s == "/" {
        return Vec::new();
    }
    s.trim_start_matches('/')
        .split('/')
        .map(|seg| seg.replace("~1", "/").replace("~0", "~"))
        .collect()
}

/// Walk `value` along `path` and return a mutable reference to the
/// addressed node, or `None` if the path is unreachable.
fn navigate_mut<'a>(value: &'a mut Value, path: &[String]) -> Option<&'a mut Value> {
    let mut cursor = value;
    for seg in path {
        cursor = match cursor {
            Value::Mapping(m) => m.get_mut(seg.as_str())?,
            Value::Sequence(s) => {
                let idx: usize = seg.parse().ok()?;
                s.get_mut(idx)?
            }
            _ => return None,
        };
    }
    Some(cursor)
}

/// Try to coerce `node` into `target`. Returns `true` when the
/// coercion was applied. Only the safe directions are honoured:
/// string → integer / number / boolean when the parse succeeds.
fn try_coerce(node: &mut Value, target: jsonschema::JsonType) -> bool {
    use jsonschema::JsonType;
    let s = match node {
        Value::String(s) => s.clone(),
        _ => return false,
    };
    let coerced = match target {
        JsonType::Integer => s
            .parse::<i64>()
            .ok()
            .map(|n| Value::Number(Number::Integer(n))),
        JsonType::Number => s
            .parse::<f64>()
            .ok()
            .map(|f| Value::Number(Number::Float(f))),
        JsonType::Boolean => match s.as_str() {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            _ => None,
        },
        _ => None,
    };
    match coerced {
        Some(new_v) => {
            *node = new_v;
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Value {
        crate::from_str(s).unwrap()
    }

    #[test]
    fn valid_value_returns_ok() {
        let schema =
            parse("type: object\nrequired: [port]\nproperties:\n  port:\n    type: integer\n");
        let value = parse("port: 8080\n");
        assert!(validate_against_schema(&value, &schema).is_ok());
    }

    #[test]
    fn type_mismatch_returns_err() {
        let schema = parse("type: object\nproperties:\n  port:\n    type: integer\n");
        let value = parse("port: hello\n");
        let err = validate_against_schema(&value, &schema).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("schema violation"), "got: {msg}");
        assert!(msg.contains("/port"), "path missing: {msg}");
    }

    #[test]
    fn missing_required_field_returns_err() {
        let schema = parse("type: object\nrequired: [port]\n");
        let value = parse("host: localhost\n");
        let err = validate_against_schema(&value, &schema).unwrap_err();
        assert!(err.to_string().contains("port"));
    }

    #[test]
    fn multiple_violations_aggregated() {
        let schema = parse(
            "type: object
required: [port, host]
properties:
  port:
    type: integer
  host:
    type: string
",
        );
        let value = parse("port: not-int\n");
        let err = validate_against_schema(&value, &schema).unwrap_err();
        let msg = err.to_string();
        // Two distinct violations: type mismatch on port, missing host.
        assert!(msg.contains("schema violations"), "got: {msg}");
        assert!(msg.contains("port"));
        assert!(msg.contains("host"));
    }

    #[test]
    fn invalid_schema_distinguished_from_invalid_data() {
        // `type` cannot be a number per JSON Schema.
        let schema = parse("type: 42\n");
        let value = parse("anything: 1\n");
        let err = validate_against_schema(&value, &schema).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("not a valid JSON Schema"),
            "expected schema-side error, got: {msg}"
        );
    }

    #[test]
    fn enum_constraint_enforced() {
        let schema = parse(
            "type: object
properties:
  level:
    enum: [trace, debug, info, warn, error]
",
        );
        assert!(validate_against_schema(&parse("level: warn\n"), &schema).is_ok());
        assert!(validate_against_schema(&parse("level: ULTRA\n"), &schema).is_err());
    }

    #[test]
    fn integer_bounds_enforced() {
        let schema = parse(
            "type: object
properties:
  port:
    type: integer
    minimum: 0
    maximum: 65535
",
        );
        assert!(validate_against_schema(&parse("port: 8080\n"), &schema).is_ok());
        assert!(validate_against_schema(&parse("port: 70000\n"), &schema).is_err());
        assert!(validate_against_schema(&parse("port: -1\n"), &schema).is_err());
    }

    #[test]
    fn nested_object_validated() {
        let schema = parse(
            "type: object
properties:
  db:
    type: object
    required: [host]
    properties:
      host:
        type: string
",
        );
        let good = parse("db:\n  host: localhost\n");
        let bad = parse("db: {}\n");
        assert!(validate_against_schema(&good, &schema).is_ok());
        assert!(validate_against_schema(&bad, &schema).is_err());
    }

    #[test]
    fn validate_against_schema_str_parses_both_inputs() {
        let schema = "type: object\nrequired: [port]\n";
        let yaml = "port: 8080\n";
        assert!(validate_against_schema_str(yaml, schema).is_ok());
    }

    #[test]
    fn schema_for_codegen_round_trip_validates_self() {
        // Phase 3.1 + 3.2 together — derive JsonSchema, emit, then
        // validate sample data against the emitted schema.

        #[derive(serde::Serialize, serde::Deserialize, crate::JsonSchema)]
        #[allow(dead_code)]
        struct Cfg {
            port: u16,
            #[serde(default)]
            host: String,
        }

        let schema = crate::schema_for::<Cfg>().unwrap();
        let good = parse("port: 8080\nhost: localhost\n");
        assert!(validate_against_schema(&good, &schema).is_ok());

        let bad = parse("host: localhost\n"); // missing required `port`
        let err = validate_against_schema(&bad, &schema).unwrap_err();
        assert!(err.to_string().contains("port"));
    }
}
