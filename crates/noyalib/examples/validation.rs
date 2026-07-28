// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Spanned + miette diagnostic bridge for validation errors.
//!
//! Demonstrates how to parse YAML with `Spanned<T>` fields, validate
//! the values, and generate rich terminal diagnostics with source
//! span highlighting via miette.
//!
//! Run: `cargo run --example validation --features miette`

#[path = "support.rs"]
mod support;

fn main() {
    support::header("noyalib -- validation (Spanned + miette diagnostics)");

    #[cfg(not(feature = "miette"))]
    {
        println!("  This example requires the 'miette' feature.");
        println!("  Run: cargo run --example validation --features miette");
        println!();
    }

    #[cfg(feature = "miette")]
    run_miette_examples();
}

#[cfg(feature = "miette")]
fn run_miette_examples() {
    use noyalib::Spanned;

    // ── Parse YAML with Spanned<T> fields ────────────────────────────
    support::task_with_output("Parse YAML with Spanned<T> fields", || {
        let yaml = "name: myapp\nport: 8080\n";
        #[derive(Debug, serde::Deserialize)]
        struct Config {
            name: Spanned<String>,
            port: Spanned<u16>,
        }
        let cfg: Config = noyalib::from_str(yaml).unwrap();
        vec![
            format!(
                "name = {:?} (span {}..{})",
                cfg.name.value,
                cfg.name.start.index(),
                cfg.name.end.index()
            ),
            format!(
                "port = {} (span {}..{})",
                cfg.port.value,
                cfg.port.start.index(),
                cfg.port.end.index()
            ),
        ]
    });

    // ── Validate fields and generate diagnostics ─────────────────────
    support::task_with_output("Validate: port must be >= 1024", || {
        let yaml = "port: 80\n";
        #[derive(Debug, serde::Deserialize)]
        struct Cfg {
            port: Spanned<u16>,
        }
        let cfg: Cfg = noyalib::from_str(yaml).unwrap();
        let mut lines = Vec::new();
        if cfg.port.value < 1024 {
            let report = noyalib::diagnostic::spanned_error(
                yaml,
                &cfg.port,
                "port must be >= 1024 (privileged ports are not allowed)",
            );
            lines.push(format!("Validation failed: {report}"));
            // Show the rendered diagnostic
            let rendered = format!("{report:?}");
            for line in rendered.lines().take(8) {
                lines.push(format!("  {line}"));
            }
        }
        lines
    });

    // ── Validate: name must not be empty ─────────────────────────────
    support::task_with_output("Validate: name must not be empty", || {
        let yaml = "name: \"\"\nport: 8080\n";
        #[derive(Debug, serde::Deserialize)]
        struct Cfg {
            name: Spanned<String>,
            #[allow(dead_code)]
            port: Spanned<u16>,
        }
        let cfg: Cfg = noyalib::from_str(yaml).unwrap();
        let mut lines = Vec::new();
        if cfg.name.value.is_empty() {
            let report =
                noyalib::diagnostic::spanned_error(yaml, &cfg.name, "name must not be empty");
            lines.push(format!("Validation failed: {report}"));
        }
        lines
    });

    // ── Multiple validation errors collected ─────────────────────────
    support::task_with_output("Collect multiple validation errors", || {
        let yaml = "host: \"\"\nport: 80\nmax_connections: -5\n";
        #[derive(Debug, serde::Deserialize)]
        struct ServerCfg {
            host: Spanned<String>,
            port: Spanned<u16>,
            max_connections: Spanned<i32>,
        }
        let cfg: ServerCfg = noyalib::from_str(yaml).unwrap();
        let mut errors: Vec<miette::Report> = Vec::new();

        if cfg.host.value.is_empty() {
            errors.push(noyalib::diagnostic::spanned_error(
                yaml,
                &cfg.host,
                "host must not be empty",
            ));
        }
        if cfg.port.value < 1024 {
            errors.push(noyalib::diagnostic::spanned_error(
                yaml,
                &cfg.port,
                "port must be >= 1024",
            ));
        }
        if cfg.max_connections.value < 0 {
            errors.push(noyalib::diagnostic::spanned_error(
                yaml,
                &cfg.max_connections,
                "max_connections must be non-negative",
            ));
        }

        assert_eq!(errors.len(), 3);
        let mut lines = vec![format!("Found {} validation errors:", errors.len())];
        for (i, err) in errors.iter().enumerate() {
            lines.push(format!("  {}. {err}", i + 1));
        }
        lines
    });

    // ── Show rendered diagnostic output ──────────────────────────────
    support::task_with_output("Rendered diagnostic with source highlighting", || {
        let yaml = "database:\n  host: db.local\n  port: 80\n  name: prod\n";
        #[derive(Debug, serde::Deserialize)]
        struct DbCfg {
            #[allow(dead_code)]
            host: String,
            port: Spanned<u16>,
            #[allow(dead_code)]
            name: String,
        }
        #[derive(Debug, serde::Deserialize)]
        struct Root {
            database: DbCfg,
        }
        let root: Root = noyalib::from_str(yaml).unwrap();
        let mut lines = Vec::new();
        if root.database.port.value < 1024 {
            let report = noyalib::diagnostic::spanned_error(
                yaml,
                &root.database.port,
                "database port must be >= 1024",
            );
            // Render with miette debug format (includes source underlining)
            let rendered = format!("{report:?}");
            lines.push("Diagnostic output:".to_string());
            for line in rendered.lines() {
                lines.push(format!("  {line}"));
            }
        }
        lines
    });

    // ── Diagnostic has correct metadata ──────────────────────────────
    support::task_with_output("Diagnostic metadata (code, labels, source)", || {
        use miette::Diagnostic;

        let yaml = "value: 42\n";
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            value: Spanned<i32>,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        let report = noyalib::diagnostic::spanned_error(yaml, &doc.value, "too small");
        let diag: &dyn Diagnostic = report.as_ref();

        let has_code = diag.code().is_some();
        let has_labels = diag.labels().is_some();
        let has_source = diag.source_code().is_some();

        assert!(has_code);
        assert!(has_labels);
        assert!(has_source);

        vec![
            format!("code:        {}", diag.code().unwrap()),
            format!("has_labels:  {has_labels}"),
            format!("has_source:  {has_source}"),
            format!("message:     {report}"),
        ]
    });

    support::summary(6);
}
