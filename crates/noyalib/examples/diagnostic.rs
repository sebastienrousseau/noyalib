// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! miette::Diagnostic integration: rich terminal error diagnostics.
//!
//! Demonstrates how noyalib errors integrate with the miette ecosystem
//! for CLI tools that need beautiful, actionable error output.
//!
//! Run: `cargo run --example diagnostic --features miette`
//!
//! Without the miette feature, this example still demonstrates the
//! Error API (code, help, labels are just not rendered via miette).

#[path = "support.rs"]
mod support;

use noyalib::{DuplicateKeyPolicy, ParserConfig, Value, from_str, from_str_with_config};

fn main() {
    support::header("noyalib -- diagnostic");

    // ── Parse error with location ────────────────────────────────────
    support::task_with_output("Parse error with source location", || {
        let yaml = "host: localhost\nport: [broken\n";
        let err = from_str::<Value>(yaml).unwrap_err();
        let mut lines = vec![
            format!("error:    {err}"),
            format!("location: {:?}", err.location()),
        ];

        #[cfg(feature = "miette")]
        {
            use miette::Diagnostic;
            lines.push(format!("code:     {:?}", err.code().map(|c| c.to_string())));
            lines.push(format!("help:     {:?}", err.help().map(|h| h.to_string())));
            lines.push(format!(
                "labels:   {:?}",
                err.labels()
                    .map(|l| l.map(|s| format!("{:?}", s)).collect::<Vec<_>>())
            ));
        }

        #[cfg(not(feature = "miette"))]
        {
            lines.push("(enable --features miette for rich diagnostics)".to_string());
        }

        lines
    });

    // ── Duplicate key with help text ─────────────────────────────────
    support::task_with_output("Duplicate key error (with help)", || {
        let yaml = "name: first\nname: second\n";
        let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
        let err = from_str_with_config::<Value>(yaml, &config).unwrap_err();

        let mut lines = vec![format!("error: {err}")];

        #[cfg(feature = "miette")]
        {
            use miette::Diagnostic;
            lines.push(format!("code:  {}", err.code().unwrap()));
            lines.push(format!("help:  {}", err.help().unwrap()));
        }

        #[cfg(not(feature = "miette"))]
        {
            lines.push("(enable --features miette for code/help)".to_string());
        }

        lines
    });

    // ── Recursion limit with actionable help ─────────────────────────
    support::task_with_output("Recursion limit (actionable help)", || {
        let config = ParserConfig::new().max_depth(3);
        let yaml = "a:\n  b:\n    c:\n      d: too deep\n";
        let err = from_str_with_config::<Value>(yaml, &config).unwrap_err();

        let mut lines = vec![format!("error: {err}")];

        #[cfg(feature = "miette")]
        {
            use miette::Diagnostic;
            if let Some(code) = err.code() {
                lines.push(format!("code:  {code}"));
            }
            if let Some(help) = err.help() {
                lines.push(format!("help:  {help}"));
            }
        }

        #[cfg(not(feature = "miette"))]
        {
            lines.push("(enable --features miette for code/help)".to_string());
        }

        lines
    });

    // ── format_with_source (always available) ────────────────────────
    support::task_with_output("format_with_source (built-in, no feature needed)", || {
        let yaml = "host: localhost\nport: [broken\ndb: postgres\n";
        let err = noyalib::Error::parse_at("expected ',' or ']'", yaml, 22);
        let formatted = err.format_with_source(yaml);
        formatted.lines().map(|l| l.to_string()).collect()
    });

    // ── miette::Report usage pattern ─────────────────────────────────
    support::task_with_output("Usage pattern for CLI tools", || {
        vec![
            "// In your CLI tool's main():".to_string(),
            "// fn main() -> miette::Result<()> {".to_string(),
            "//     let config: Config = noyalib::from_str(yaml)".to_string(),
            "//         .map_err(|e| miette::Report::new(e)".to_string(),
            "//             .with_source_code(yaml.to_owned()))?;".to_string(),
            "//     Ok(())".to_string(),
            "// }".to_string(),
            String::new(),
            "// This gives you:".to_string(),
            "//   x noyalib::parse".to_string(),
            "//   |-> expected ',' or ']'".to_string(),
            "//    --> input.yaml:2:7".to_string(),
            "//   1 | host: localhost".to_string(),
            "//   2 | port: [broken".to_string(),
            "//     |        ^^^^^^ here".to_string(),
        ]
    });

    support::summary(5);
}
