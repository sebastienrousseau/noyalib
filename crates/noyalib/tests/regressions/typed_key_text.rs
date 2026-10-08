// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A typed parse sees a mapping key as it was written, whichever load
//! path serves it. The streaming path always did; a tag anywhere in
//! the document moves a typed parse to the `Value` path, which spelled
//! a non-string key in canonical form (`0x1F` as "31", `~` as "null",
//! YAML 1.1 `on` as "true"). Found by fuzz_diff (input `?\n! ~""" [:`).
//! A `Value` keeps the canonical spelling.

use std::collections::BTreeMap;

use noyalib::{ParserConfig, Value, from_str, from_str_with_config};
use serde::Deserialize;

const KEYS: &[&str] = &[
    "0x1F", "True", "+1", "1e3", "~", "null", "Null", "0o7", "nan",
];

fn keys(doc: &str, cfg: &ParserConfig) -> Vec<String> {
    let m: BTreeMap<String, Value> = from_str_with_config(doc, cfg).unwrap();
    m.into_keys().filter(|k| k != "z").collect()
}

#[test]
fn a_tag_elsewhere_does_not_respell_typed_keys() {
    let cfg = ParserConfig::new();
    for key in KEYS {
        let plain = format!("{key}: 1\n");
        let tagged = format!("{key}: 1\nz: !t 2\n");
        assert_eq!(keys(&plain, &cfg), [*key], "streaming {plain:?}");
        assert_eq!(keys(&tagged, &cfg), [*key], "fallback {tagged:?}");
    }
}

#[test]
fn an_empty_key_stays_empty() {
    let cfg = ParserConfig::new();
    assert_eq!(keys("? \n: 1\nz: !t 2\n", &cfg), [""]);
    assert_eq!(keys(": 1\nz: !t 2\n", &cfg), [""]);
}

#[test]
fn yaml_1_1_on_key_reaches_a_struct_field() {
    #[derive(Deserialize)]
    struct Workflow {
        on: String,
        // Read, so a tagged `z` takes the document off the streaming path.
        #[serde(default)]
        #[allow(dead_code)]
        z: Option<Value>,
    }
    let cfg = ParserConfig::new().legacy_booleans(true);
    for doc in ["on: push\n", "on: push\nz: !t 1\n"] {
        let w: Workflow =
            from_str_with_config(doc, &cfg).unwrap_or_else(|e| panic!("{doc:?}: {e}"));
        assert_eq!(w.on, "push");
    }
}

#[test]
fn quoted_and_value_keys_are_unchanged() {
    let cfg = ParserConfig::new();
    assert_eq!(keys("\"0x1F\": 1\nz: !t 2\n", &cfg), ["0x1F"]);
    assert_eq!(keys("'~': 1\nz: !t 2\n", &cfg), ["~"]);
    // A Value keeps the canonical spelling.
    let v: Value = from_str("0x1F: 1\n~: 2\n").unwrap();
    let ks: Vec<_> = v.as_mapping().unwrap().keys().cloned().collect();
    assert_eq!(ks, ["31", "null"]);
}
