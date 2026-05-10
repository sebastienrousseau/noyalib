// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `${KEY}` / `${KEY:-default}` substitution during parse via
//! `ParserConfig::properties`.
//!
//! Walks every YAML scalar after parse and replaces placeholders
//! against a [`HashMap`]. Pair with `strict_properties(true)` to
//! abort on an unknown placeholder, leave it `false` (the default)
//! to substitute the empty string. Inline `${KEY:-fallback}`
//! syntax always wins over both modes.
//!
//! Run: `cargo run --example properties_interpolation`

#[path = "support.rs"]
mod support;

use noyalib::{from_str_with_config, ParserConfig, Value};
use std::collections::HashMap;
use std::sync::Arc;

fn main() {
    support::header("noyalib -- properties_interpolation");

    support::task_with_output("Substitute ${KEY} from a property map", || {
        let mut props = HashMap::new();
        let _ = props.insert("HOST".to_string(), "localhost".to_string());
        let _ = props.insert("PORT".to_string(), "8080".to_string());
        let cfg = ParserConfig::new().properties(Arc::new(props));
        let v: Value = from_str_with_config("api: http://${HOST}:${PORT}/v1\n", &cfg).unwrap();
        vec![format!("api = {}", v["api"].as_str().unwrap())]
    });

    support::task_with_output("${KEY:-default} fallback for missing keys", || {
        let cfg = ParserConfig::new().properties(Arc::new(HashMap::new()));
        let v: Value =
            from_str_with_config("log: ${LEVEL:-info}\nport: ${PORT:-3000}\n", &cfg).unwrap();
        vec![
            format!("log  = {}", v["log"].as_str().unwrap()),
            format!("port = {}", v["port"].as_str().unwrap()),
        ]
    });

    support::task_with_output("strict_properties(true) errors on unknown key", || {
        let cfg = ParserConfig::new()
            .properties(Arc::new(HashMap::new()))
            .strict_properties(true);
        let res: Result<Value, _> = from_str_with_config("token: ${SECRET}\n", &cfg);
        vec![format!(
            "res = {}",
            match &res {
                Ok(_) => "Ok (unexpected)".into(),
                Err(e) => format!("Err: {e}"),
            }
        )]
    });

    support::task_with_output("Lossy mode: missing keys → empty string", || {
        let cfg = ParserConfig::new().properties(Arc::new(HashMap::new()));
        let v: Value = from_str_with_config("x: ${UNKNOWN}\n", &cfg).unwrap();
        vec![format!("x = {:?}", v["x"].as_str().unwrap())]
    });

    support::task_with_output("Escapes: $$ → $, ${{ → ${ ", || {
        let cfg = ParserConfig::new().properties(Arc::new(HashMap::new()));
        let v: Value =
            from_str_with_config("price: $$5.00\ntemplate: \"${{name}\"\n", &cfg).unwrap();
        vec![
            format!("price    = {}", v["price"].as_str().unwrap()),
            format!("template = {}", v["template"].as_str().unwrap()),
        ]
    });
}
