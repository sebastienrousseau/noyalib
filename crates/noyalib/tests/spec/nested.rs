// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Nested/complex structures

use std::collections::HashMap;

use noyalib::{Value, from_str};
use serde::Deserialize;

#[test]
fn mapping_of_sequences() {
    let v: HashMap<String, Vec<String>> = from_str(
        "american:\n  - Boston Red Sox\n  - Detroit Tigers\nnational:\n  - New York Mets\n  - Chicago Cubs\n",
    )
    .unwrap();
    assert_eq!(v["american"].len(), 2);
    assert_eq!(v["national"].len(), 2);
    assert_eq!(v["american"][0], "Boston Red Sox");
}

#[test]
fn sequence_of_mappings_complex() {
    #[derive(Debug, Deserialize)]
    struct Item {
        item: String,
        quantity: i64,
    }

    let v: Vec<Item> =
        from_str("- item: widget\n  quantity: 10\n- item: gadget\n  quantity: 5\n").unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].item, "widget");
    assert_eq!(v[0].quantity, 10);
}

#[test]
fn three_level_nesting() {
    let v: Value = from_str("level1:\n  level2:\n    level3:\n      value: deep\n").unwrap();
    assert_eq!(
        v.get("level1")
            .unwrap()
            .get("level2")
            .unwrap()
            .get("level3")
            .unwrap()
            .get("value")
            .unwrap()
            .as_str(),
        Some("deep")
    );
}

#[test]
fn mixed_nested_collections() {
    let v: Value = from_str(
        "users:\n  - name: Alice\n    roles:\n      - admin\n      - user\n  - name: Bob\n    roles:\n      - user\n",
    )
    .unwrap();
    let users = v.get("users").unwrap().as_sequence().unwrap();
    assert_eq!(users.len(), 2);
    let alice_roles = users[0].get("roles").unwrap().as_sequence().unwrap();
    assert_eq!(alice_roles.len(), 2);
}

#[test]
fn sequence_of_sequences_of_mappings() {
    let v: Vec<Vec<HashMap<String, i64>>> = from_str("- - a: 1\n  - b: 2\n- - c: 3\n").unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].len(), 2);
    assert_eq!(v[0][0]["a"], 1);
    assert_eq!(v[1][0]["c"], 3);
}

#[test]
fn real_world_config() {
    #[derive(Debug, Deserialize)]
    struct Config {
        server: Server,
        database: Database,
    }
    #[derive(Debug, Deserialize)]
    struct Server {
        host: String,
        port: u16,
    }
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Database {
        url: String,
        pool_size: u32,
    }

    let c: Config = from_str(
        "server:\n  host: 0.0.0.0\n  port: 8080\ndatabase:\n  url: postgres://localhost/db\n  pool_size: 5\n",
    )
    .unwrap();
    assert_eq!(c.server.host, "0.0.0.0");
    assert_eq!(c.server.port, 8080);
    assert_eq!(c.database.pool_size, 5);
}

#[test]
fn optional_nested_fields() {
    #[derive(Debug, Deserialize)]
    struct Config {
        name: String,
        debug: Option<bool>,
        extra: Option<HashMap<String, String>>,
    }

    let c: Config = from_str("name: app\n").unwrap();
    assert_eq!(c.name, "app");
    assert!(c.debug.is_none());
    assert!(c.extra.is_none());
}
