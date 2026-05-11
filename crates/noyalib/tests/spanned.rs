//! Spanned<T> tests.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Spanned, from_str, to_string};
use serde::{Deserialize, Serialize};

#[test]
fn test_spanned_basic() {
    #[derive(Debug, Deserialize)]
    struct Config {
        port: Spanned<u16>,
    }

    let yaml = "port: 8080";
    let config: Config = from_str(yaml).unwrap();
    assert_eq!(*config.port, 8080);
    assert_eq!(config.port.value, 8080);
}

#[test]
fn test_spanned_serialize_transparent() {
    let val = Spanned::new(42i64);
    let yaml = to_string(&val).unwrap();
    assert_eq!(yaml.trim(), "42");
}

#[test]
fn test_spanned_locations_real() {
    let val: Spanned<String> = from_str("hello").unwrap();
    assert_eq!(val.value, "hello");
    // Real locations from parser
    assert_eq!(val.start.line(), 1);
    assert_eq!(val.start.column(), 1);
    assert_eq!(val.start.index(), 0);
    assert!(val.end.index() > 0);
}

#[test]
fn test_spanned_in_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        name: Spanned<String>,
        port: Spanned<u16>,
    }

    let config = Config {
        name: Spanned::new("myapp".to_string()),
        port: Spanned::new(8080),
    };

    let yaml = to_string(&config).unwrap();
    assert!(yaml.contains("name: myapp"));
    assert!(yaml.contains("port: 8080"));

    let parsed: Config = from_str(&yaml).unwrap();
    assert_eq!(parsed.name.value, "myapp");
    assert_eq!(parsed.port.value, 8080);
}
