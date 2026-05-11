// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Standard serde-ecosystem wrapper interop.
//!
//! Two of the most-used serde companion crates are
//! `serde_path_to_error` (path-aware error reporting) and
//! `serde_ignored` (unknown-field detection). Both wrap any
//! `serde::Deserializer` impl and compose without crate-specific
//! support. These tests assert that `noyalib::Deserializer<'de>`
//! plugs in correctly, so users picking either tool from their
//! existing toolbox don't need a noyalib-specific integration.

#![allow(missing_docs)]

use noyalib::{Deserializer, Value, from_str};
use serde::Deserialize;

// ── serde_path_to_error ─────────────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq)]
struct App {
    name: String,
    server: Server,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
    database: Database,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Database {
    url: String,
    pool_size: u16,
}

fn parse_with_path_to_error(yaml: &str) -> Result<App, (String, String)> {
    let value: Value = from_str(yaml).map_err(|e| (String::new(), e.to_string()))?;
    let de = Deserializer::new(&value);
    serde_path_to_error::deserialize(de)
        .map_err(|err| (err.path().to_string(), err.inner().to_string()))
}

#[test]
fn path_to_error_succeeds_on_valid_input() {
    let yaml = "\
name: my-app
server:
  host: localhost
  port: 8080
  database:
    url: postgres://localhost/app
    pool_size: 16
";
    let app = parse_with_path_to_error(yaml).unwrap();
    assert_eq!(app.name, "my-app");
    assert_eq!(app.server.port, 8080);
}

#[test]
fn path_to_error_pinpoints_top_level_field() {
    let yaml = "\
name: 42
server:
  host: localhost
  port: 8080
  database:
    url: postgres://localhost/app
    pool_size: 16
";
    let (path, _msg) = parse_with_path_to_error(yaml).unwrap_err();
    assert_eq!(path, "name", "expected path 'name', got: {path:?}");
}

#[test]
fn path_to_error_pinpoints_nested_field() {
    let yaml = "\
name: my-app
server:
  host: localhost
  port: not-a-port
  database:
    url: postgres://localhost/app
    pool_size: 16
";
    let (path, _msg) = parse_with_path_to_error(yaml).unwrap_err();
    assert_eq!(
        path, "server.port",
        "expected path 'server.port', got: {path:?}"
    );
}

#[test]
fn path_to_error_pinpoints_doubly_nested_field() {
    let yaml = "\
name: my-app
server:
  host: localhost
  port: 8080
  database:
    url: postgres://localhost/app
    pool_size: huge
";
    let (path, _msg) = parse_with_path_to_error(yaml).unwrap_err();
    assert_eq!(
        path, "server.database.pool_size",
        "expected path 'server.database.pool_size', got: {path:?}"
    );
}

#[test]
fn path_to_error_inside_sequence() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Replicas {
        replicas: Vec<Server>,
    }

    let yaml = "\
replicas:
  - host: a
    port: 1
    database:
      url: x
      pool_size: 1
  - host: b
    port: bogus
    database:
      url: y
      pool_size: 2
";
    let value: Value = from_str(yaml).unwrap();
    let de = Deserializer::new(&value);
    let err = serde_path_to_error::deserialize::<_, Replicas>(de).unwrap_err();
    let path = err.path().to_string();
    // The exact format may vary across serde_path_to_error versions
    // but it must mention the index 1 and the field `port`.
    assert!(path.contains('1'), "path should reference index 1: {path}");
    assert!(
        path.contains("port"),
        "path should reference 'port': {path}"
    );
}

// ── serde_ignored ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq)]
struct Slim {
    name: String,
    port: u16,
}

#[test]
fn serde_ignored_collects_unknown_top_level_keys() {
    let yaml = "\
name: my-app
port: 8080
typo_extra: oops
another_unknown: 1
";
    let value: Value = from_str(yaml).unwrap();
    let de = Deserializer::new(&value);

    let mut unknown: Vec<String> = Vec::new();
    let parsed: Slim =
        serde_ignored::deserialize(de, |path| unknown.push(path.to_string())).unwrap();

    assert_eq!(parsed.name, "my-app");
    assert_eq!(parsed.port, 8080);
    unknown.sort();
    assert_eq!(unknown, vec!["another_unknown", "typo_extra"]);
}

#[test]
fn serde_ignored_collects_unknown_nested_keys() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Outer {
        inner: Inner,
    }
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Inner {
        a: u16,
    }

    let yaml = "\
inner:
  a: 1
  b: 2
  c: 3
";
    let value: Value = from_str(yaml).unwrap();
    let de = Deserializer::new(&value);

    let mut unknown: Vec<String> = Vec::new();
    let _: Outer = serde_ignored::deserialize(de, |path| unknown.push(path.to_string())).unwrap();

    unknown.sort();
    assert_eq!(unknown, vec!["inner.b", "inner.c"]);
}

#[test]
fn serde_ignored_no_callbacks_when_input_is_clean() {
    let yaml = "name: ok\nport: 1\n";
    let value: Value = from_str(yaml).unwrap();
    let de = Deserializer::new(&value);

    let mut unknown: Vec<String> = Vec::new();
    let parsed: Slim =
        serde_ignored::deserialize(de, |path| unknown.push(path.to_string())).unwrap();

    assert_eq!(parsed.name, "ok");
    assert!(unknown.is_empty(), "no extras expected, got: {unknown:?}");
}

// ── Combined: path_to_error wrapping serde_ignored ─────────────────

#[test]
fn path_to_error_composes_with_serde_ignored() {
    let yaml = "\
name: my-app
port: 8080
extra: unwanted
";
    let value: Value = from_str(yaml).unwrap();
    let de = Deserializer::new(&value);

    let mut unknown: Vec<String> = Vec::new();
    let mut cb = |path: serde_ignored::Path| unknown.push(path.to_string());
    let ignored_de = serde_ignored::Deserializer::new(de, &mut cb);

    let parsed: Slim = serde_path_to_error::deserialize(ignored_de).unwrap();
    assert_eq!(parsed.name, "my-app");
    assert_eq!(parsed.port, 8080);
    assert_eq!(unknown, vec!["extra"]);
}
