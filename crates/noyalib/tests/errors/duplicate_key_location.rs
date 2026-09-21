// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A duplicate mapping key or a distinct-typed key collision is
//! reported with the entry's dotted path and the key's position
//! (issue #378, ADR 0013), on every read path that knows them.

#![allow(missing_docs)]

use noyalib::cst::parse_document_with_config;
use noyalib::{
    DuplicateKeyPolicy, Error, ErrorKind, ParserConfig, Value, from_str, from_str_with_config,
};
use serde::Deserialize;

fn strict() -> ParserConfig {
    ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error)
}

const NESTED: &str = "site:\n  name: a\n  name: b\n";

fn assert_located_duplicate(err: &Error, key: &str, path: &str, line: usize, column: usize) {
    match err {
        Error::DuplicateKeyAt {
            key: k,
            path: p,
            location,
        } => {
            assert_eq!(k, key);
            assert_eq!(p, path);
            assert_eq!(
                (location.line(), location.column()),
                (line, column),
                "{err}"
            );
        }
        other => panic!("expected DuplicateKeyAt, got {other:?}"),
    }
    assert_eq!(err.kind(), ErrorKind::DuplicateKey);
    assert_eq!(
        err.location().map(|l| (l.line(), l.column())),
        Some((line, column))
    );
    assert_eq!(
        err.to_string(),
        format!("{path}: duplicate key \"{key}\" at line {line}, column {column}")
    );
}

#[test]
fn a_value_read_names_the_path_and_the_position() {
    let err = from_str_with_config::<Value>(NESTED, &strict()).unwrap_err();
    assert_located_duplicate(&err, "name", "site.name", 3, 3);
}

#[test]
fn a_typed_read_names_the_path_and_the_position() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Config {
        site: Site,
    }
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Site {
        name: String,
    }
    let err = from_str_with_config::<Config>(NESTED, &strict()).unwrap_err();
    assert_located_duplicate(&err, "name", "site.name", 3, 3);
}

#[test]
fn the_cst_parser_names_the_path_and_the_position() {
    let err = parse_document_with_config(NESTED, &strict()).unwrap_err();
    assert_located_duplicate(&err, "name", "site.name", 3, 3);
}

#[test]
fn a_root_key_and_a_key_under_a_sequence_item() {
    let err = from_str_with_config::<Value>("name: a\nname: b\n", &strict()).unwrap_err();
    assert_located_duplicate(&err, "name", "name", 2, 1);

    let err =
        from_str_with_config::<Value>("items:\n  - name: a\n    name: b\n", &strict()).unwrap_err();
    assert_located_duplicate(&err, "name", "items.0.name", 3, 5);
}

#[test]
fn a_key_collision_is_located_the_same_way() {
    let err = from_str::<Value>("m:\n  1: a\n  \"1\": b\n").unwrap_err();
    match &err {
        Error::KeyCollisionAt {
            key,
            path,
            location,
        } => {
            assert_eq!(key, "1");
            assert_eq!(path, "m.1");
            assert_eq!((location.line(), location.column()), (3, 3));
        }
        other => panic!("expected KeyCollisionAt, got {other:?}"),
    }
    assert_eq!(err.kind(), ErrorKind::KeyCollision);
    assert!(
        err.to_string()
            .starts_with("m.1: distinct mapping keys collide after string conversion: 1 "),
        "{err}"
    );
    assert!(err.to_string().ends_with(" at line 3, column 3"), "{err}");
}

#[test]
fn the_location_less_forms_are_unchanged() {
    assert_eq!(
        Error::DuplicateKey("name".into()).to_string(),
        "duplicate key: name"
    );
    assert_eq!(
        Error::DuplicateKey("name".into()).kind(),
        ErrorKind::DuplicateKey
    );
    assert!(Error::DuplicateKey("name".into()).location().is_none());
    assert_eq!(
        Error::KeyCollision("1".into()).kind(),
        ErrorKind::KeyCollision
    );
}

#[test]
fn the_default_policy_still_keeps_the_last_occurrence() {
    let value: Value = from_str(NESTED).unwrap();
    assert_eq!(value["site"]["name"].as_str(), Some("b"));
}
