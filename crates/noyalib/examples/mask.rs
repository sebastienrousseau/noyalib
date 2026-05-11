// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Secret masking: serialize safely, redact in logs.
//!
//! Demonstrates a `Secret<T>` pattern that serializes the real value to YAML
//! but displays `***` in Debug/Display — preventing accidental key leakage.
//!
//! Run: `cargo run --example mask`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str, to_string};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A value that serializes normally but redacts in Display/Debug.
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
struct Secret<T>(T);

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
    username: String,
    password: Secret<String>,
    api_key: Secret<String>,
}

/// Redact sensitive fields in a Value tree by key name.
fn redact(value: &mut Value, sensitive_keys: &[&str]) {
    if let Some(map) = value.as_mapping_mut() {
        for (key, val) in map.iter_mut() {
            if sensitive_keys.iter().any(|&s| key.contains(s)) {
                *val = Value::String("***REDACTED***".to_string());
            } else {
                redact(val, sensitive_keys);
            }
        }
    }
    if let Some(seq) = value.as_sequence_mut() {
        for item in seq.iter_mut() {
            redact(item, sensitive_keys);
        }
    }
}

fn main() {
    support::header("noyalib -- mask");

    let config = DatabaseConfig {
        host: "db.production.internal".to_string(),
        port: 5432,
        username: "admin".to_string(),
        password: Secret("super-secret-password-123".to_string()),
        api_key: Secret("sk_live_abc123def456".to_string()),
    };

    // ── Secret<T> in Debug output ────────────────────────────────────
    support::task_with_output("Secret<T> redacts in Debug/Display", || {
        vec![
            format!("Debug:   {:?}", config.password),
            format!("Display: {}", config.api_key),
            "Secrets never appear in logs or error messages".to_string(),
        ]
    });

    // ── Secret<T> serializes real value ───────────────────────────────
    support::task_with_output("Secret<T> serializes real value to YAML", || {
        let yaml = to_string(&config).unwrap();
        let has_password = yaml.contains("super-secret");
        let has_api_key = yaml.contains("sk_live");
        vec![
            format!("password in YAML = {} (real value preserved)", has_password),
            format!("api_key in YAML  = {} (real value preserved)", has_api_key),
        ]
    });

    // ── Roundtrip with Secret<T> ─────────────────────────────────────
    support::task_with_output("Roundtrip: YAML -> struct -> safe log", || {
        let yaml = to_string(&config).unwrap();
        let parsed: DatabaseConfig = from_str(&yaml).unwrap();
        vec![
            format!("host     = {}", parsed.host),
            format!("username = {}", parsed.username),
            format!("password = {} (redacted in display)", parsed.password),
            format!("api_key  = {} (redacted in display)", parsed.api_key),
        ]
    });

    // ── Value-level redaction ────────────────────────────────────────
    support::task_with_output("Value-level redaction by key pattern", || {
        let yaml = "database:\n  host: db.local\n  password: secret123\n  connection_string: postgres://user:pass@host\napi:\n  key: sk_live_xyz\n  secret_token: tok_abc\n";
        let mut v: Value = from_str(yaml).unwrap();
        redact(&mut v, &["password", "secret", "key"]);
        let output = to_string(&v).unwrap();
        output.lines().map(|l| l.to_string()).collect()
    });

    support::summary(4);
}
