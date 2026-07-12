// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Serializer configuration: every output knob demonstrated.
//!
//! Run: `cargo run --example emit`

#[path = "support.rs"]
mod support;

use noyalib::{ScalarStyle, SerializerConfig, Value, from_str, to_string_with_config};

fn main() {
    support::header("noyalib -- emit");

    let yaml = "name: noyalib\ntags:\n  - yaml\n  - rust\n  - fast\n";
    let value: Value = from_str(yaml).unwrap();

    support::task_with_output("default (indent=2, block)", || {
        let out = to_string_with_config(&value, &SerializerConfig::new()).unwrap();
        out.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("indent=4", || {
        let out = to_string_with_config(&value, &SerializerConfig::new().indent(4)).unwrap();
        out.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("flow_style=Flow (JSON-like)", || {
        // Use FlowSeq/FlowMap wrappers for guaranteed inline output
        use noyalib::fmt::{FlowMap, FlowSeq};
        use std::collections::BTreeMap;

        let tags = FlowSeq(vec!["yaml", "rust", "fast"]);
        let tag_out = noyalib::to_string(&tags).unwrap();

        let mut map = BTreeMap::new();
        let _ = map.insert("name", "noyalib");
        let _ = map.insert("status", "stable");
        let flow = FlowMap(map);
        let map_out = noyalib::to_string(&flow).unwrap();

        vec![
            format!("FlowSeq: {}", tag_out.trim()),
            format!("FlowMap: {}", map_out.trim()),
        ]
    });

    support::task_with_output("scalar_style=DoubleQuoted", || {
        let out = to_string_with_config(
            &value,
            &SerializerConfig::new().scalar_style(ScalarStyle::DoubleQuoted),
        )
        .unwrap();
        out.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("quote_all=true", || {
        let out = to_string_with_config(&value, &SerializerConfig::new().quote_all(true)).unwrap();
        out.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("document_start + document_end markers", || {
        let out = to_string_with_config(
            &value,
            &SerializerConfig::new()
                .document_start(true)
                .document_end(true),
        )
        .unwrap();
        out.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("to_string_multi (3 documents)", || {
        let docs = vec![Value::from(1), Value::from(2), Value::from(3)];
        let out = noyalib::to_string_multi(&docs).unwrap();
        out.lines().map(|l| l.to_string()).collect()
    });

    support::summary(7);
}
