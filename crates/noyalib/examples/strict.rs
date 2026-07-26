// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Strict parsing: strict_booleans, duplicate key policies, security limits.
//!
//! Run: `cargo run --example strict`

#[path = "support.rs"]
mod support;

use noyalib::{DuplicateKeyPolicy, ParserConfig, Value, from_str_with_config};

fn main() {
    support::header("noyalib -- strict");

    let yaml = "a: True\nb: FALSE\nc: true\nd: false\n";

    support::task_with_output("strict_booleans=false (lenient)", || {
        let config = ParserConfig::new().strict_booleans(false);
        let v: Value = from_str_with_config(yaml, &config).unwrap();
        vec![
            format!("\"True\"  -> {}", type_of(&v["a"])),
            format!("\"FALSE\" -> {}", type_of(&v["b"])),
            format!("\"true\"  -> {}", type_of(&v["c"])),
        ]
    });

    support::task_with_output("strict_booleans=true (YAML 1.2 JSON Schema)", || {
        let config = ParserConfig::new().strict_booleans(true);
        let v: Value = from_str_with_config(yaml, &config).unwrap();
        vec![
            format!("\"True\"  -> {}", type_of(&v["a"])),
            format!("\"FALSE\" -> {}", type_of(&v["b"])),
            format!("\"true\"  -> {}", type_of(&v["c"])),
        ]
    });

    let dup_yaml = "key: first\nkey: second\n";

    support::task_with_output("DuplicateKeyPolicy::Last", || {
        let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Last);
        let v: Value = from_str_with_config(dup_yaml, &config).unwrap();
        vec![format!(
            "result: key = {:?}",
            v["key"].as_str().unwrap_or("?")
        )]
    });

    support::task_with_output("DuplicateKeyPolicy::First", || {
        let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
        let v: Value = from_str_with_config(dup_yaml, &config).unwrap();
        vec![format!(
            "result: key = {:?}",
            v["key"].as_str().unwrap_or("?")
        )]
    });

    let _ = support::task_result("DuplicateKeyPolicy::Error (expected failure)", || {
        let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
        from_str_with_config::<Value>(dup_yaml, &config)
    });

    let _ = support::task_result("Depth limit (40 levels, max_depth=32)", || {
        let strict = ParserConfig::strict();
        let mut deep = String::new();
        for i in 0..40 {
            for _ in 0..i {
                deep.push_str("  ");
            }
            deep.push_str("a:\n");
        }
        from_str_with_config::<Value>(&deep, &strict)
    });

    support::summary(6);
}

fn type_of(v: &Value) -> &'static str {
    match v {
        Value::Bool(_) => "Bool",
        Value::String(_) => "String",
        Value::Number(_) => "Number",
        Value::Null => "Null",
        _ => "Other",
    }
}
