// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Anchors, aliases, and merge keys.
//!
//! Run: `cargo run --example alias`

#[path = "support.rs"]
mod support;

use std::collections::BTreeMap;

use noyalib::{Value, from_str};
use serde::Deserialize;

fn main() {
    support::header("noyalib -- alias");

    // Simple anchor and alias
    support::task_with_output("Simple anchor and alias", || {
        let yaml = "first: &value 42\nsecond: *value\nthird: *value\n";
        let parsed: BTreeMap<String, i32> = from_str(yaml).unwrap();
        assert_eq!(parsed.get("first"), Some(&42));
        assert_eq!(parsed.get("second"), Some(&42));
        vec![
            format!("first = {}", parsed["first"]),
            format!("second = {} (alias)", parsed["second"]),
            format!("third = {} (alias)", parsed["third"]),
        ]
    });

    // Merge key with mapping
    support::task_with_output("Merge key with mapping", || {
        let yaml = r#"
defaults: &defaults
  timeout: 30
  retries: 3

server1:
  <<: *defaults
  host: s1.example.com

server2:
  <<: *defaults
  host: s2.example.com
  timeout: 60
"#;
        let v: Value = from_str(yaml).unwrap();
        let s1 = v.get("server1").unwrap();
        let s2 = v.get("server2").unwrap();
        vec![
            format!(
                "server1.timeout = {} (inherited)",
                s1.get("timeout").unwrap()
            ),
            format!(
                "server2.timeout = {} (overridden)",
                s2.get("timeout").unwrap()
            ),
        ]
    });

    // Typed deserialization
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct ServerConfig {
        host: String,
        timeout: u32,
        retries: u32,
    }

    support::task("Typed deserialization with merge keys", || {
        let yaml = "host: s1.example.com\ntimeout: 30\nretries: 3\n";
        let _: ServerConfig = from_str(yaml).unwrap();
    });

    // Anchor with sequence
    support::task("Anchor with sequence alias", || {
        let yaml = "base: &items\n  - a\n  - b\nlist1: *items\nlist2: *items\n";
        let v: Value = from_str(yaml).unwrap();
        assert_eq!(v.get("base"), v.get("list1"));
    });

    // Multiple merge sources
    support::task_with_output("Multiple merge sources", || {
        let yaml = "s1: &s1\n  a: 1\ns2: &s2\n  b: 2\ncombined:\n  <<: [*s1, *s2]\n  c: 3\n";
        let v: Value = from_str(yaml).unwrap();
        let c = v.get("combined").unwrap();
        vec![
            format!("a = {} (from s1)", c.get("a").unwrap()),
            format!("b = {} (from s2)", c.get("b").unwrap()),
            format!("c = {} (local)", c.get("c").unwrap()),
        ]
    });

    // Merge key precedence
    support::task_with_output("Merge key precedence (first source wins)", || {
        let yaml = "x: &x\n  key: from_x\ny: &y\n  key: from_y\nm:\n  <<: [*x, *y]\n";
        let v: Value = from_str(yaml).unwrap();
        vec![format!(
            "key = {} (first source wins)",
            v.get("m").unwrap().get("key").unwrap()
        )]
    });

    // Nested merge
    support::task("Nested merge (transitive)", || {
        let yaml = "b1: &b1\n  a: 1\nb2: &b2\n  <<: *b1\n  b: 2\nfinal:\n  <<: *b2\n  c: 3\n";
        let v: Value = from_str(yaml).unwrap();
        let f = v.get("final").unwrap();
        assert!(f.get("a").is_some());
        assert!(f.get("b").is_some());
        assert!(f.get("c").is_some());
    });

    support::summary(7);
}
