//! Collection serialization example for noyalib.
//!
//! Demonstrates serializing Vec, HashMap, and other collections.
//!
//! Run: `cargo run --example std`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use std::collections::HashMap;

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

fn main() {
    support::header("noyalib -- std");

    support::task_with_output("Serialize and roundtrip Vec", || {
        let numbers = vec![1, 2, 3, 4, 5];
        let yaml = to_string(&numbers).unwrap();

        let parsed: Vec<i32> = from_str(&yaml).unwrap();
        assert_eq!(numbers, parsed);

        let mut lines = vec!["Vec serialized:".to_string()];
        lines.extend(yaml.lines().map(|l| l.to_string()));
        lines
    });

    support::task_with_output("Serialize HashMap", || {
        let mut map = HashMap::new();
        let _ = map.insert("key1".to_string(), "value1".to_string());
        let _ = map.insert("key2".to_string(), "value2".to_string());
        let _ = map.insert("key3".to_string(), "value3".to_string());

        let yaml = to_string(&map).unwrap();

        let mut lines = vec!["HashMap serialized:".to_string()];
        lines.extend(yaml.lines().map(|l| l.to_string()));
        lines
    });

    support::task_with_output("Serialize and roundtrip nested collections", || {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Config {
            name: String,
            tags: Vec<String>,
            settings: HashMap<String, i32>,
        }

        let config = Config {
            name: "my-app".to_string(),
            tags: vec!["rust".to_string(), "yaml".to_string(), "safe".to_string()],
            settings: {
                let mut s = HashMap::new();
                let _ = s.insert("timeout".to_string(), 30);
                let _ = s.insert("retries".to_string(), 3);
                s
            },
        };

        let yaml = to_string(&config).unwrap();

        let parsed: Config = from_str(&yaml).unwrap();
        assert_eq!(config, parsed);

        let mut lines = vec!["Nested config serialized:".to_string()];
        lines.extend(yaml.lines().map(|l| l.to_string()));
        lines
    });

    support::summary(3);
}
