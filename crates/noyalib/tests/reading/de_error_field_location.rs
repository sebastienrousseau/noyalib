// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! An unknown or missing field reported by `from_str` carries a location
//! and a field path, like a type mismatch or a visitor's own error.
//!
//! `deny_unknown_fields` and a hand-written `visit_map` raise serde's
//! `unknown_field` / `missing_field`, which reach `wrap_err` as
//! `Error::UnknownField` / `Error::MissingField`; without this they were
//! the one typed rejection `from_str` returned with no position. A derive
//! rejects the unknown field while reading its key, so that error is
//! located at the entry and pathed to the field itself; a visitor's own
//! `unknown_field`, and every `missing_field`, is raised against the
//! mapping and located and pathed there. The message keeps serde's
//! wording. `from_value` has no source to locate against and keeps
//! returning the bare variants.

#![cfg(feature = "std")]

use noyalib::{Error, Value, from_str, from_value};
use serde::de::{self, Deserializer, MapAccess, Visitor};
use std::fmt;

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Server {
    port: u16,
    host: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Cfg {
    server: Server,
}

#[test]
fn unknown_field_in_a_nested_mapping_is_located_and_pathed() {
    let err = from_str::<Cfg>("server:\n  port: 1\n  porrt: 2\n").unwrap_err();
    assert_eq!(
        err.to_string(),
        "deserialization error at line 3, column 10: server.porrt: unknown field: porrt"
    );
    let loc = err.location().expect("located");
    assert_eq!((loc.line(), loc.column()), (3, 10));
}

#[test]
fn missing_field_in_a_nested_mapping_is_located_and_pathed() {
    let err = from_str::<Cfg>("server:\n  host: h\n").unwrap_err();
    assert_eq!(
        err.to_string(),
        "deserialization error at line 2, column 3: server: missing field: port"
    );
    let loc = err.location().expect("located");
    assert_eq!((loc.line(), loc.column()), (2, 3));
}

#[test]
fn unknown_field_at_the_root_is_located_and_pathed_to_the_field() {
    let err = from_str::<Server>("port: 1\nporrt: 2\n").unwrap_err();
    assert_eq!(
        err.to_string(),
        "deserialization error at line 2, column 8: porrt: unknown field: porrt"
    );
    assert!(err.location().is_some());
}

#[test]
fn missing_field_at_the_root_is_located_without_a_path() {
    let err = from_str::<Server>("host: h\n").unwrap_err();
    assert_eq!(
        err.to_string(),
        "deserialization error at line 1, column 1: missing field: port"
    );
    assert!(err.location().is_some());
}

/// A hand-written `visit_map` raising `unknown_field` / `missing_field`
/// itself, reached through `deserialize_any`, is located the same way.
#[derive(Debug)]
#[allow(dead_code)]
struct Mapping {
    local: u16,
    container: u16,
}

impl<'de> serde::Deserialize<'de> for Mapping {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Mapping;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a port mapping")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Mapping, A::Error> {
                let (mut local, mut container) = (None, None);
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "local" => local = Some(map.next_value()?),
                        "container" => container = Some(map.next_value()?),
                        other => {
                            return Err(de::Error::unknown_field(other, &["local", "container"]));
                        }
                    }
                }
                Ok(Mapping {
                    local: local.ok_or_else(|| de::Error::missing_field("local"))?,
                    container: container.ok_or_else(|| de::Error::missing_field("container"))?,
                })
            }
        }
        deserializer.deserialize_any(V)
    }
}

#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct Ports {
    ports: Vec<Mapping>,
}

#[test]
fn unknown_field_from_a_hand_written_visitor_is_located_and_pathed() {
    let yaml = "ports:\n  - local: 1\n    container: 2\n  - local: 3\n    other: 4\n";
    let err = from_str::<Ports>(yaml).unwrap_err();
    assert_eq!(
        err.to_string(),
        "deserialization error at line 4, column 5: ports[1]: unknown field: other"
    );
}

#[test]
fn missing_field_from_a_hand_written_visitor_is_located_and_pathed() {
    let yaml = "ports:\n  - local: 1\n    container: 2\n  - local: 3\n";
    let err = from_str::<Ports>(yaml).unwrap_err();
    assert_eq!(
        err.to_string(),
        "deserialization error at line 4, column 5: ports[1]: missing field: container"
    );
}

#[test]
fn from_value_keeps_the_bare_variants() {
    let value: Value = from_str("server:\n  port: 1\n  porrt: 2\n").unwrap();
    let err = from_value::<Cfg>(&value).unwrap_err();
    assert!(
        matches!(err, Error::UnknownField(ref f) if f == "porrt"),
        "{err:?}"
    );
    assert!(err.location().is_none());

    let value: Value = from_str("server:\n  host: h\n").unwrap();
    let err = from_value::<Cfg>(&value).unwrap_err();
    assert!(
        matches!(err, Error::MissingField(ref f) if f == "port"),
        "{err:?}"
    );
    assert!(err.location().is_none());
}
