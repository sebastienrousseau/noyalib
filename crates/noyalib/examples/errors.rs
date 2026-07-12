// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Error handling: error types, locations, formatted diagnostics.
//!
//! Run: `cargo run --example errors`

#[path = "support.rs"]
mod support;

use noyalib::{Error, Location, Value, from_str};
use serde::Deserialize;

fn main() {
    support::header("noyalib -- errors");

    // ── Syntax error (genuinely broken YAML) ─────────────────────────
    support::task_with_output("Catch syntax error (expected failure)", || {
        let broken = "key: [unclosed\n";
        match from_str::<Value>(broken) {
            Err(e) => {
                let mut lines = vec![format!("Caught: {e}")];
                if let Some(loc) = e.location() {
                    lines.push(format!("  at line {}, column {}", loc.line(), loc.column()));
                }
                lines
            }
            Ok(_) => vec!["BUG: should have failed".to_string()],
        }
    });

    // ── Type mismatch ────────────────────────────────────────────────
    support::task_with_output("Catch type mismatch (expected failure)", || {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Typed {
            name: i32,
        }
        match from_str::<Typed>("name: not_a_number\n") {
            Err(e) => vec![format!("Caught: {e}")],
            Ok(_) => vec!["BUG: should have failed".to_string()],
        }
    });

    // ── Missing field ────────────────────────────────────────────────
    support::task_with_output("Catch missing field (expected failure)", || {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Required {
            name: String,
            port: u16,
        }
        match from_str::<Required>("name: test\n") {
            Err(e) => vec![format!("Caught: {e}")],
            Ok(_) => vec!["BUG: should have failed".to_string()],
        }
    });

    // ── Formatted error with source context (rustc-style) ─────────────
    support::task_with_output("Formatted error with source pointer", || {
        // Use programmatic error creation to guarantee the pointer renders
        // at a visible line. This demonstrates the rustc-style diagnostic.
        let source = "host: localhost\nport: not_valid\ndb: postgres";
        let error = Error::parse_at("expected integer value", source, 16);
        let formatted = error.format_with_source(source);
        formatted.lines().map(|l| l.to_string()).collect()
    });

    // ── Location calculations ────────────────────────────────────────
    support::task_with_output("Location from byte index", || {
        let text = "first line\nsecond line\nthird line\n";
        vec![
            format!(
                "index  0 = line {}, col {}",
                Location::from_index(text, 0).line(),
                Location::from_index(text, 0).column()
            ),
            format!(
                "index 11 = line {}, col {}",
                Location::from_index(text, 11).line(),
                Location::from_index(text, 11).column()
            ),
            format!(
                "index 23 = line {}, col {}",
                Location::from_index(text, 23).line(),
                Location::from_index(text, 23).column()
            ),
        ]
    });

    // ── Error type matching ──────────────────────────────────────────
    support::task_with_output("Programmatic error type matching", || {
        let errors: Vec<Error> = vec![
            Error::Parse("bad syntax".to_string()),
            Error::TypeMismatch {
                expected: "string",
                found: "integer".to_string(),
            },
            Error::MissingField("port".to_string()),
            Error::Custom("application error".to_string()),
        ];
        errors
            .iter()
            .map(|e| {
                let kind = match e {
                    Error::Parse(_) => "Parse",
                    Error::TypeMismatch { .. } => "TypeMismatch",
                    Error::MissingField(_) => "MissingField",
                    Error::Custom(_) => "Custom",
                    _ => "Other",
                };
                format!("{kind:<13} -> {e}")
            })
            .collect()
    });

    // ── Graceful error recovery ──────────────────────────────────────
    support::task_with_output("Graceful error recovery pattern", || {
        let inputs = vec![
            ("valid", "name: app\nport: 8080\n"),
            ("broken", "key: {unclosed\n"),
            ("empty", ""),
        ];
        inputs
            .into_iter()
            .map(|(label, yaml)| match from_str::<Value>(yaml) {
                Ok(v) => format!("{label:<7} -> ok ({} nodes)", count_nodes(&v)),
                Err(e) => format!("{label:<7} -> err ({e})"),
            })
            .collect()
    });

    support::summary(7);
}

fn count_nodes(v: &Value) -> usize {
    match v {
        Value::Sequence(s) => 1 + s.iter().map(count_nodes).sum::<usize>(),
        Value::Mapping(m) => 1 + m.values().map(count_nodes).sum::<usize>(),
        Value::Tagged(t) => 1 + count_nodes(t.value()),
        _ => 1,
    }
}
