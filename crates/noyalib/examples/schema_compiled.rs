// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Compile a JSON Schema once, validate many documents (#329).
//!
//! Demonstrates the compiled-validation surface:
//!
//! - [`CompiledSchema::compile`] — one schema compile serving any
//!   number of validations, where `validate_against_schema`
//!   recompiles per call.
//! - [`CompiledSchema::builder`] — opt in to `format` assertion
//!   (an annotation by default under Draft 2020-12) and register a
//!   custom format.
//! - [`CompiledSchema::iter_errors`] — structured violations with
//!   an instance path and the keyword that raised each.
//!
//! Run: `cargo run --example schema_compiled --features validate-schema`
//!
//! [`CompiledSchema::compile`]: noyalib::CompiledSchema::compile
//! [`CompiledSchema::builder`]: noyalib::CompiledSchema::builder
//! [`CompiledSchema::iter_errors`]: noyalib::CompiledSchema::iter_errors

#[path = "support.rs"]
mod support;

use noyalib::{CompiledSchema, Value, from_str};

fn main() -> noyalib::Result<()> {
    support::header("Compiled schema validation (compile once, validate many)");

    // A frontmatter-style schema, itself written in YAML.
    let schema: Value = from_str(
        "type: object
required: [title]
properties:
  title: {type: string}
  date: {type: string, format: date}
  slug: {type: string, format: kebab-slug}
  draft: {type: boolean}
",
    )?;

    // Compile once...
    let compiled = CompiledSchema::compile(&schema)?;

    // ...validate many. Each page costs a walk, not a schema compile.
    let pages = [
        "title: Home\ndate: 2026-08-30\nslug: home\n",
        "title: About\ndraft: true\n",
        "date: 2026-08-30\n", // missing required `title`
    ];
    println!("  Batch validation ({} pages, one compile):", pages.len());
    for (i, page) in pages.iter().enumerate() {
        let v: Value = from_str(page)?;
        match compiled.validate(&v) {
            Ok(()) => println!("    page {i}: OK"),
            Err(e) => println!(
                "    page {i}: {}",
                e.to_string().lines().next().unwrap_or("")
            ),
        }
    }

    // `format` asserts only when asked to; a custom format rides along.
    let strict = CompiledSchema::builder(&schema)
        .validate_formats(true)
        .with_format("kebab-slug", |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
        .build()?;

    let sloppy: Value = from_str("title: Post\ndate: 01/15/2024\nslug: Getting Started\n")?;
    println!();
    println!("  With validate_formats(true) + custom `kebab-slug`:");
    for violation in strict.iter_errors(&sloppy)? {
        println!(
            "    {} [{}]: {}",
            violation.instance_path,
            violation.keyword,
            violation.message.lines().next().unwrap_or("")
        );
    }

    support::footer();
    Ok(())
}
