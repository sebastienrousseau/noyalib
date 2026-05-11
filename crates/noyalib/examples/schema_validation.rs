// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Library-level JSON Schema validation + auto-coercion.
//!
//! Demonstrates the three pieces of the schema-aware surface:
//!
//! - [`schema_for`] / [`schema_for_yaml`] — derive a JSON Schema
//!   2020-12 document from a Rust type.
//! - [`validate_against_schema`] — verify a parsed `Value` against
//!   the schema and surface every violation with a JSON-pointer
//!   path.
//! - [`coerce_to_schema`] — surgical fix-pass that rewrites
//!   string-shaped scalars into the schema's expected type when
//!   they parse cleanly. Mirrors what `noyavalidate --fix` does on
//!   the command line.
//!
//! Run: `cargo run --example schema_validation --features validate-schema`
//!
//! [`schema_for`]: noyalib::schema_for
//! [`schema_for_yaml`]: noyalib::schema_for_yaml
//! [`validate_against_schema`]: noyalib::validate_against_schema
//! [`coerce_to_schema`]: noyalib::coerce_to_schema

#[path = "support.rs"]
mod support;

use noyalib::{JsonSchema, Value, coerce_to_schema, from_str, schema_for, validate_against_schema};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ServerConfig {
    /// Port the server binds on.
    port: u16,
    /// Hostname or IP literal.
    host: String,
    /// Whether TLS is enabled.
    #[serde(default)]
    tls: bool,
}

fn main() -> noyalib::Result<()> {
    support::header("Schema validation + auto-coercion (library API)");

    // Derive the schema from the Rust type.
    let schema = schema_for::<ServerConfig>()?;
    println!("  Schema derived from Rust type:");
    println!("    title: {}", schema["title"].as_str().unwrap_or("?"));
    println!("    type:  {}", schema["type"].as_str().unwrap_or("?"));

    // Hand-written YAML where the user quoted the port number — a
    // common slip-up that strict validation flags but auto-coercion
    // can fix without an editor round-trip.
    let mut data: Value = from_str("port: \"8080\"\nhost: api.example.com\n")?;

    // Strict pass — should fail because `port` is a string.
    println!();
    println!("  Strict validation (port is a string):");
    match validate_against_schema(&data, &schema) {
        Ok(()) => println!("    OK (unexpected)"),
        Err(e) => println!("    {}", e.to_string().lines().next().unwrap_or("")),
    }

    // Apply schema-driven coercions.
    let fixes = coerce_to_schema(&mut data, &schema)?;
    println!();
    println!("  After coerce_to_schema: {fixes} fix(es) applied");

    // Re-validate — should pass now.
    match validate_against_schema(&data, &schema) {
        Ok(()) => println!("    Strict validation: OK"),
        Err(e) => println!("    Still invalid: {e}"),
    }

    support::footer();
    Ok(())
}
