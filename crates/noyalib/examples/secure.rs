// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! ParserConfig: security limits for untrusted input.
//!
//! Run: `cargo run --example secure`

#[path = "support.rs"]
mod support;

use noyalib::{DuplicateKeyPolicy, ParserConfig, Value, from_str_with_config};

fn main() {
    support::header("noyalib -- secure");

    support::task_with_output("Strict preset for untrusted input", || {
        let strict = ParserConfig::strict();
        let yaml = "key: value\n";
        match from_str_with_config::<Value>(yaml, &strict) {
            Ok(v) => vec![
                "Status: accepted".to_string(),
                format!(
                    "Value:  {}",
                    v.get("key").and_then(|v| v.as_str()).unwrap_or("?")
                ),
            ],
            Err(e) => vec![format!("Caught: {e}")],
        }
    });

    support::task_with_output("Custom limits (max_depth = 4)", || {
        let config = ParserConfig::new()
            .max_depth(4)
            .max_document_length(1024)
            .max_alias_expansions(10);

        let deep = "a:\n  b:\n    c:\n      d:\n        e: too deep\n";
        match from_str_with_config::<Value>(deep, &config) {
            Ok(_) => vec!["Status: accepted (should have failed)".to_string()],
            Err(e) => vec![format!("Caught: {e}")],
        }
    });

    support::task_with_output("Reject duplicate keys", || {
        let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
        let yaml = "name: first\nname: second\n";
        match from_str_with_config::<Value>(yaml, &config) {
            Ok(_) => vec!["Status: accepted (should have failed)".to_string()],
            Err(e) => vec![format!("Caught: {e}")],
        }
    });

    support::summary(3);
}
