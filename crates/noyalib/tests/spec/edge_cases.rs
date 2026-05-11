// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Edge cases and unusual but valid YAML

use std::collections::HashMap;

use noyalib::{Value, from_str, to_string};
use serde::{Deserialize, Serialize};

#[test]
fn unicode_string() {
    let v: String = from_str("\"\\u00e9\"").unwrap();
    assert_eq!(v, "\u{00e9}");
}

#[test]
fn unicode_key() {
    let m: HashMap<String, String> = from_str("caf\u{00e9}: coffee\n").unwrap();
    assert_eq!(m["caf\u{00e9}"], "coffee");
}

#[test]
fn very_long_string() {
    let long = "a".repeat(10000);
    let yaml = format!("\"{}\"", long);
    let v: String = from_str(&yaml).unwrap();
    assert_eq!(v.len(), 10000);
}

#[test]
fn deeply_nested_10_levels() {
    let mut yaml = String::new();
    for i in 0..10 {
        yaml.push_str(&"  ".repeat(i));
        yaml.push_str(&format!("l{i}:\n"));
    }
    yaml.push_str(&"  ".repeat(10));
    yaml.push_str("val: 42\n");

    let v: Value = from_str(&yaml).unwrap();
    // Navigate down
    let mut current = &v;
    for i in 0..10 {
        current = current.get(format!("l{i}")).unwrap();
    }
    assert_eq!(current.get("val").unwrap().as_i64(), Some(42));
}

#[test]
fn empty_document() {
    let v: Option<i64> = from_str("---\n").unwrap();
    assert!(v.is_none());
}

#[test]
fn whitespace_only_values() {
    let m: HashMap<String, String> = from_str("key: \"  \"\n").unwrap();
    assert_eq!(m["key"], "  ");
}

#[test]
fn tab_in_string() {
    let v: String = from_str("\"hello\\tworld\"").unwrap();
    assert_eq!(v, "hello\tworld");
}

#[test]
fn carriage_return_in_string() {
    let v: String = from_str("\"hello\\rworld\"").unwrap();
    assert_eq!(v, "hello\rworld");
}

#[test]
fn roundtrip_complex_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        name: String,
        port: u16,
        debug: bool,
        tags: Vec<String>,
    }

    let original = Config {
        name: "app".into(),
        port: 8080,
        debug: true,
        tags: vec!["v1".into(), "stable".into()],
    };

    let yaml = to_string(&original).unwrap();
    let parsed: Config = from_str(&yaml).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn roundtrip_nested_option() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Outer {
        inner: Option<Inner>,
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Inner {
        value: i64,
    }

    let orig = Outer {
        inner: Some(Inner { value: 42 }),
    };
    let yaml = to_string(&orig).unwrap();
    let parsed: Outer = from_str(&yaml).unwrap();
    assert_eq!(orig, parsed);
}

#[test]
fn roundtrip_enum_variants() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Color {
        Red,
        Green,
        Blue,
    }

    for color in [Color::Red, Color::Green, Color::Blue] {
        let yaml = to_string(&color).unwrap();
        let parsed: Color = from_str(&yaml).unwrap();
        assert_eq!(color, parsed);
    }
}

#[test]
fn roundtrip_newtype_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Wrapper(i64);

    let orig = Wrapper(42);
    let yaml = to_string(&orig).unwrap();
    let parsed: Wrapper = from_str(&yaml).unwrap();
    assert_eq!(orig, parsed);
}

#[test]
fn roundtrip_tuple_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Point(f64, f64);

    let orig = Point(1.5, 2.5);
    let yaml = to_string(&orig).unwrap();
    let parsed: Point = from_str(&yaml).unwrap();
    assert_eq!(orig, parsed);
}

#[test]
fn roundtrip_unit_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Unit;

    let yaml = to_string(&Unit).unwrap();
    let parsed: Unit = from_str(&yaml).unwrap();
    assert_eq!(Unit, parsed);
}

#[test]
fn string_with_all_special_starters() {
    // All characters that require quoting when starting a string
    for ch in [
        '&', '*', '!', '|', '>', '%', '@', '`', '{', '}', '[', ']', ',', '?', '-',
    ] {
        let s = format!("{ch}test");
        let yaml = to_string(&s).unwrap();
        let parsed: String = from_str(&yaml).unwrap();
        assert_eq!(
            s, parsed,
            "roundtrip failed for string starting with '{ch}'"
        );
    }
}

#[test]
fn large_mapping_100_keys() {
    let mut yaml = String::new();
    for i in 0..100 {
        yaml.push_str(&format!("key{i}: {i}\n"));
    }
    let m: HashMap<String, i64> = from_str(&yaml).unwrap();
    assert_eq!(m.len(), 100);
    assert_eq!(m["key0"], 0);
    assert_eq!(m["key99"], 99);
}

#[test]
fn large_sequence_100_items() {
    let mut yaml = String::new();
    for i in 0..100 {
        yaml.push_str(&format!("- {i}\n"));
    }
    let v: Vec<i64> = from_str(&yaml).unwrap();
    assert_eq!(v.len(), 100);
    assert_eq!(v[0], 0);
    assert_eq!(v[99], 99);
}
