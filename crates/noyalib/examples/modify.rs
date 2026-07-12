// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Value manipulation: to_value, from_value, MappingAny, get_path_mut, ValueIndex.
//!
//! Run: `cargo run --example modify`

#[path = "support.rs"]
mod support;

use noyalib::{Mapping, MappingAny, Value, from_str, from_value, to_value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

fn main() {
    support::header("noyalib -- modify");

    let server = Server {
        host: "localhost".to_string(),
        port: 8080,
    };

    support::task_with_output("to_value: struct -> Value", || {
        let value = to_value(&server).unwrap();
        vec![
            format!(
                "host = {}",
                value.get("host").and_then(|v| v.as_str()).unwrap_or("?")
            ),
            format!(
                "port = {}",
                value.get("port").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
        ]
    });

    support::task_with_output("from_value: Value -> struct", || {
        let value = to_value(&server).unwrap();
        let parsed: Server = from_value(&value).unwrap();
        assert_eq!(parsed, server);
        vec![format!("result = {:?}", parsed)]
    });

    let yaml = "servers:\n  - host: alpha\n    port: 80\n  - host: beta\n    port: 443\n";

    support::task_with_output("get_path: dot/bracket traversal", || {
        let doc: Value = from_str(yaml).unwrap();
        vec![
            format!(
                "servers[0].host = {}",
                doc.get_path("servers[0].host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
            format!(
                "servers[1].port = {}",
                doc.get_path("servers[1].port")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
            ),
        ]
    });

    support::task_with_output("get_path_mut: change port to 9090", || {
        let mut doc: Value = from_str(yaml).unwrap();
        let before = doc
            .get_path("servers[0].port")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if let Some(port) = doc.get_path_mut("servers[0].port") {
            *port = Value::from(9090);
        }
        let after = doc
            .get_path("servers[0].port")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        vec![format!("before = {before}"), format!("after  = {after}")]
    });

    support::task_with_output("ValueIndex: direct bracket access", || {
        let doc: Value = from_str(yaml).unwrap();
        vec![format!(
            "result = {}",
            doc["servers"][0]["host"].as_str().unwrap_or("?")
        )]
    });

    support::task_with_output("MappingAny: numeric keys", || {
        let yaml = "1: one\n2: two\n3: three\n";
        let map: MappingAny = from_str(yaml).unwrap();
        map.iter().map(|(k, v)| format!("{k} -> {v}")).collect()
    });

    support::task_with_output("Mapping.sort_keys", || {
        let mut m = Mapping::new();
        let _ = m.insert("zebra", Value::from(1));
        let _ = m.insert("alpha", Value::from(2));
        let _ = m.insert("middle", Value::from(3));
        let before: Vec<_> = m.keys().cloned().collect();
        m.sort_keys();
        let after: Vec<_> = m.keys().cloned().collect();
        vec![
            format!("before = [{}]", before.join(", ")),
            format!("after  = [{}]", after.join(", ")),
        ]
    });

    support::summary(7);
}
