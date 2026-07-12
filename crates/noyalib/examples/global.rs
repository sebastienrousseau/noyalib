//! Serialization configuration example for noyalib.
//!
//! Demonstrates using SerializerConfig to customize YAML output.
//!
//! Run: `cargo run --example global`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use noyalib::{SerializerConfig, Value, to_string, to_string_with_config};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ServerConfig {
    host: String,
    port: u16,
    description: String,
    features: Vec<String>,
}

fn main() {
    support::header("noyalib -- global");

    let config = ServerConfig {
        host: "localhost".to_string(),
        port: 8080,
        description:
            "This is a long description\nthat spans multiple lines\nfor demonstration purposes."
                .to_string(),
        features: vec![
            "authentication".to_string(),
            "rate-limiting".to_string(),
            "caching".to_string(),
        ],
    };

    support::task_with_output("Default serialization", || {
        let yaml = to_string(&config).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("With document start/end markers", || {
        let config_with_markers = SerializerConfig::default()
            .document_start(true)
            .document_end(true);
        let yaml = to_string_with_config(&config, &config_with_markers).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("With 4-space indent", || {
        let config_4_indent = SerializerConfig::default().indent(4);
        let yaml = to_string_with_config(&config, &config_4_indent).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("With block scalars enabled", || {
        let config_block = SerializerConfig::default()
            .block_scalars(true)
            .block_scalar_threshold(1);
        let yaml = to_string_with_config(&config, &config_block).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("Combined configuration", || {
        let full_config = SerializerConfig::new()
            .indent(2)
            .document_start(true)
            .document_end(true)
            .block_scalars(true)
            .block_scalar_threshold(2);
        let yaml = to_string_with_config(&config, &full_config).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("Using builder pattern", || {
        let builder_config = SerializerConfig::default()
            .indent(2)
            .document_start(true)
            .block_scalars(true);
        let yaml = to_string_with_config(&config, &builder_config).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("Value with custom config", || {
        let value = Value::from("A string with special chars: *anchor, &alias, #comment");
        let yaml = to_string_with_config(&value, &SerializerConfig::default()).unwrap();
        vec![format!("Value: {}", yaml.trim_end())]
    });

    support::summary(7);
}
