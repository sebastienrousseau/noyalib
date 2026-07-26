// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! YAML schema validation (core, JSON, failsafe).
//!
//! Run: `cargo run --example schema`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str, validate_yaml_core_schema, validate_yaml_json_schema};

fn main() {
    support::header("noyalib -- schema");

    let yaml = "name: noyalib\nversion: 1\nenabled: true\ntags:\n  - yaml\n  - serde\n";
    let value: Value = from_str(yaml).unwrap();

    support::task_with_output(
        "Validate against core schema",
        || match validate_yaml_core_schema(&value) {
            Ok(()) => vec!["Result: valid".to_string()],
            Err(e) => vec![format!("Result: rejected"), format!("Reason: {e}")],
        },
    );

    support::task_with_output(
        "Validate against JSON schema",
        || match validate_yaml_json_schema(&value) {
            Ok(()) => vec!["Result: valid".to_string()],
            Err(e) => vec![format!("Result: rejected"), format!("Reason: {e}")],
        },
    );

    support::task_with_output("NaN rejected by JSON schema", || {
        let nan: Value = from_str("value: .nan\n").unwrap();
        match validate_yaml_json_schema(&nan) {
            Ok(()) => vec!["Status: accepted (unexpected)".to_string()],
            Err(e) => vec!["Status: rejected".to_string(), format!("Reason: {e}")],
        }
    });

    support::summary(3);
}
