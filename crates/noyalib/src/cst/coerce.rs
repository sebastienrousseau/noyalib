// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Lossless schema-driven type coercion on the CST path.
//!
//! [`coerce_to_schema`] is the comment-preserving counterpart to
//! [`crate::coerce_to_schema`]. Given a [`Document`] and a JSON
//! Schema, it walks every type-mismatch the schema reports and
//! rewrites the offending scalar via the same surgical
//! [`Document::set`] machinery that powers
//! [`crate::cst::format`] — only the touched scalar bytes change;
//! comments, anchors, indentation, and sibling entries survive
//! byte-faithfully.
//!
//! The library-level [`crate::coerce_to_schema`] keeps the simpler
//! `&mut Value` shape; CLI tooling that needs to round-trip user
//! files (`noyavalidate --schema --fix`) uses this lossless path
//! instead.
//!
//! # Trade-offs
//!
//! - Only string → integer / number / boolean coercions are
//!   supported (the safe directions, identical to
//!   [`crate::coerce_to_schema`]).
//! - The schema must be a JSON Schema 2020-12 document. Same
//!   contract as [`crate::validate_against_schema`].
//! - Fixed-point iteration: the function loops until no further
//!   coercion applies (capped at 1024 iterations to bound work
//!   on adversarial schemas).
//!
//! # Examples
//!
//! ```
//! use noyalib::{from_str, Value};
//! use noyalib::cst::{coerce_to_schema, parse_document};
//!
//! let schema: Value = from_str(
//!     "type: object\nproperties:\n  port: { type: integer }\n",
//! ).unwrap();
//! let mut doc = parse_document("# inline comment\nport: \"8080\"\n").unwrap();
//! let n = coerce_to_schema(&mut doc, &schema).unwrap();
//! assert_eq!(n, 1);
//! let after = doc.to_string();
//! assert!(after.contains("# inline comment"), "comment must survive");
//! assert!(after.contains("port: 8080"), "scalar must be coerced");
//! assert!(!after.contains("\"8080\""), "quotes must be gone");
//! ```

use crate::cst::document::Document;
use crate::error::{Error, Result};
use crate::value::{Number, Value};

/// Apply schema-driven type coercions to `doc` **lossless**ly:
/// only the bytes of the coerced scalars are rewritten; comments,
/// indentation, and sibling entries survive byte-faithfully.
///
/// Returns the number of coercions applied. `0` means the input
/// already matches the schema (or no coercion is possible — e.g.
/// `port: "abc"` against `type: integer`).
///
/// # Errors
///
/// - Schema does not parse as a valid JSON Schema 2020-12 document.
/// - The CST `Document::set` operation rejects the spliced
///   fragment (would only happen on adversarial schemas that
///   request a coercion to a non-scalar type — guarded against
///   here, but the error is surfaced if it slips through).
pub fn coerce_to_schema(doc: &mut Document, schema: &Value) -> Result<usize> {
    use jsonschema::JsonType;
    use jsonschema::error::{TypeKind, ValidationErrorKind};

    // Compile the schema once. Re-uses the same JSON-bridge helper
    // path as `crate::coerce_to_schema` so the two functions share
    // their schema-acceptance contract.
    let schema_json = crate::schema_validate::value_to_json(schema)
        .map_err(|e| Error::Custom(format!("cst::coerce_to_schema: schema -> JSON: {e}")))?;
    let validator = jsonschema::validator_for(&schema_json).map_err(|e| {
        Error::Custom(format!(
            "cst::coerce_to_schema: schema is not a valid JSON Schema: {e}"
        ))
    })?;

    let mut applied: usize = 0;
    let max_iterations = 1024;

    for _ in 0..max_iterations {
        // Snapshot the document as a Value so we can ask the
        // validator which paths fail. Cloning the Value is `O(n)`
        // but the loop terminates the moment no coercion applies,
        // so the typical cost is one or two passes.
        let value: Value = crate::from_str(&doc.to_string())?;
        let instance_json = crate::schema_validate::value_to_json(&value)
            .map_err(|e| Error::Parse(format!("cst::coerce_to_schema: value -> JSON: {e}")))?;

        let mut applied_this_pass = false;
        let mut targets: Vec<(String, JsonType)> = Vec::new();
        for err in validator.iter_errors(&instance_json) {
            if let ValidationErrorKind::Type {
                kind: TypeKind::Single(target),
            } = err.kind()
            {
                targets.push((err.instance_path().to_string(), *target));
            }
        }

        for (json_pointer, target) in targets {
            // Walk the value tree along the JSON pointer to
            // discover whether each segment addresses a mapping
            // key or a sequence index. The CST's `Document::entry`
            // path syntax distinguishes them: keys join with `.`,
            // indices appear as `[N]`.
            let cst_path = match value_path_from_pointer(&value, &json_pointer) {
                Some(p) => p,
                None => continue, // path not reachable — skip
            };

            // Look up the current scalar; if it's not a string we
            // cannot coerce (only safe direction is string → typed).
            let current = match doc.entry(&cst_path).get() {
                Some(s) => s.to_owned(),
                None => continue,
            };

            // Strip surrounding quotes if present so we operate on
            // the logical scalar value, not the source-form
            // representation.
            let logical = strip_quotes(&current);
            let coerced = match coerce_logical(&logical, target) {
                Some(v) => v,
                None => continue,
            };

            doc.entry(&cst_path).set(&coerced)?;
            applied += 1;
            applied_this_pass = true;
        }

        if !applied_this_pass {
            break;
        }
    }

    Ok(applied)
}

/// Resolve an RFC 6901 JSON pointer (e.g. `"/server/port"`) into
/// the dot/bracket path syntax `Document::entry` expects (e.g.
/// `"server.port"`). Returns `None` if any segment is unreachable
/// or if the addressed location is not a scalar.
fn value_path_from_pointer(root: &Value, pointer: &str) -> Option<String> {
    let segments = parse_json_pointer(pointer);
    if segments.is_empty() {
        return None;
    }
    let mut cursor = root;
    let mut out = String::new();
    for (i, seg) in segments.iter().enumerate() {
        match cursor {
            Value::Mapping(m) => {
                if i > 0 {
                    out.push('.');
                }
                out.push_str(seg);
                cursor = m.get(seg.as_str())?;
            }
            Value::Sequence(s) => {
                let idx: usize = seg.parse().ok()?;
                use core::fmt::Write;
                let _ = write!(out, "[{idx}]");
                cursor = s.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(out)
}

fn parse_json_pointer(s: &str) -> Vec<String> {
    if s.is_empty() || s == "/" {
        return Vec::new();
    }
    s.trim_start_matches('/')
        .split('/')
        .map(|seg| seg.replace("~1", "/").replace("~0", "~"))
        .collect()
}

/// Strip a single layer of `'…'` or `"…"` quotes so the logical
/// scalar value is exposed for parsing. YAML core-schema strings
/// without quotes pass through unchanged.
fn strip_quotes(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return s[1..s.len() - 1].to_owned();
        }
    }
    s.to_owned()
}

/// Try to coerce a logical scalar string into the schema's
/// expected type. Returns the YAML source-form of the coerced
/// value if the parse succeeds; `None` otherwise (caller leaves
/// the original source alone so a follow-up validate can surface
/// the residue).
fn coerce_logical(logical: &str, target: jsonschema::JsonType) -> Option<String> {
    use jsonschema::JsonType;
    let _ = Number::Integer(0); // ensure Number is in scope for future extension
    match target {
        JsonType::Integer => {
            let n: i64 = logical.parse().ok()?;
            Some(n.to_string())
        }
        JsonType::Number => {
            let f: f64 = logical.parse().ok()?;
            // Use the standard noyalib serializer for floats so the
            // emitted form matches the rest of the document
            // (`fast-float` / ryu when on, `core::fmt` otherwise).
            crate::ser::to_string(&Value::Number(Number::Float(f)))
                .ok()
                .map(|s| s.trim().to_owned())
        }
        JsonType::Boolean => match logical {
            "true" => Some("true".to_owned()),
            "false" => Some("false".to_owned()),
            _ => None,
        },
        _ => None,
    }
}
