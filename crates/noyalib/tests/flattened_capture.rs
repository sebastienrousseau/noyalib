// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Flattened<T>` — capture the raw `Value` tree alongside the
//! typed deserialization for `#[serde(flatten)]` targets.

#![allow(missing_docs)]

use noyalib::{Flattened, Value, from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct Inner {
    port: u16,
    host: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct Config {
    name: String,
    inner: Flattened<Inner>,
}

const YAML: &str = "\
name: noyalib
inner:
  port: 8080
  host: localhost
  extra: not-in-Inner
";

#[test]
fn typed_view_extracts_declared_fields() {
    let cfg: Config = from_str(YAML).unwrap();
    assert_eq!(cfg.inner.value.port, 8080);
    assert_eq!(cfg.inner.value.host, "localhost");
}

#[test]
fn raw_view_preserves_undeclared_keys() {
    let cfg: Config = from_str(YAML).unwrap();
    let raw = match &cfg.inner.raw {
        Value::Mapping(m) => m,
        other => panic!("expected mapping, got {other:?}"),
    };
    // Both declared and undeclared keys survive in `raw`.
    assert!(raw.contains_key("port"));
    assert!(raw.contains_key("host"));
    assert!(raw.contains_key("extra"));
    assert_eq!(raw["extra"].as_str(), Some("not-in-Inner"));
}

#[test]
fn deref_provides_typed_view() {
    let cfg: Config = from_str(YAML).unwrap();
    // `Deref` lets callers reach inner fields without `.value.`.
    assert_eq!(cfg.inner.port, 8080);
    assert_eq!(cfg.inner.host, "localhost");
}

#[test]
fn round_trip_through_serialize_then_deserialize() {
    let cfg: Config = from_str(YAML).unwrap();
    // Serialize the wrapper transparently — only the typed view
    // is emitted; the captured raw side-channel is by design not
    // part of the on-the-wire format.
    let yaml = to_string(&cfg).unwrap();
    assert!(yaml.contains("name: noyalib"));
    assert!(yaml.contains("port: 8080"));
    assert!(yaml.contains("host: localhost"));
    // The undeclared `extra` key is NOT round-tripped — that's
    // the documented contract: serialization mirrors the typed
    // view, not the captured raw.
    assert!(!yaml.contains("extra"));
}

#[test]
fn into_value_consumes_to_inner_t() {
    let cfg: Config = from_str(YAML).unwrap();
    let inner: Inner = cfg.inner.into_value();
    assert_eq!(inner.port, 8080);
}

#[test]
fn flatten_attribute_works_with_wrapper() {
    // The "flagship" use case — a struct with `#[serde(flatten)]`
    // pointing at a `Flattened<T>` field. The wrapper captures
    // every residue key the source supplied, so callers see the
    // typed view AND the raw extras.
    #[derive(Debug, Deserialize)]
    struct Outer {
        version: u8,
        #[serde(flatten)]
        body: Flattened<Inner>,
    }

    let yaml = "\
version: 1
port: 8080
host: localhost
custom_field: extra
debug: true
";
    let outer: Outer = from_str(yaml).unwrap();
    // Top-level explicit field.
    assert_eq!(outer.version, 1);
    // Typed view of the flattened residue.
    assert_eq!(outer.body.port, 8080);
    assert_eq!(outer.body.host, "localhost");
    // Raw view captures the residue keys that aren't in `Inner`.
    let raw = match &outer.body.raw {
        Value::Mapping(m) => m,
        other => panic!("expected mapping, got {other:?}"),
    };
    // `version` is NOT in the residue (it's claimed by Outer).
    assert!(!raw.contains_key("version"));
    // The Inner fields ARE in the residue.
    assert!(raw.contains_key("port"));
    assert!(raw.contains_key("host"));
    // Plus the extras.
    assert!(raw.contains_key("custom_field"));
    assert!(raw.contains_key("debug"));
}

#[test]
fn flattened_inside_sequence() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Item {
        id: u32,
        details: Flattened<Inner>,
    }

    let yaml = "\
- id: 1
  details:
    port: 1
    host: a
    note: first
- id: 2
  details:
    port: 2
    host: b
    note: second
";
    let items: Vec<Item> = from_str(yaml).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].details.port, 1);
    assert_eq!(items[1].details.host, "b");
    // Each captures its own raw residue.
    let r0 = match &items[0].details.raw {
        Value::Mapping(m) => m,
        _ => panic!(),
    };
    assert_eq!(r0["note"].as_str(), Some("first"));
}

#[test]
fn flattened_with_failing_typed_projection_errors() {
    // If the inner type cannot be deserialized from the captured
    // value, the error must surface — not silently produce a
    // default-constructed `T`.
    let yaml = "\
name: app
inner:
  port: not-a-number
  host: localhost
";
    let res: Result<Config, _> = from_str(yaml);
    assert!(res.is_err());
}

#[test]
fn flattened_with_simple_scalar_target() {
    // Smaller use case — capture even a bare scalar.
    let v: Flattened<u16> = from_str("8080").unwrap();
    assert_eq!(*v, 8080);
    assert_eq!(v.raw.as_i64(), Some(8080));
}

#[test]
fn flattened_constructor_helper() {
    // Direct construction for callers that already have both the
    // typed and raw views.
    let v = Flattened::new(42_u16, Value::from(42_i64));
    assert_eq!(v.value, 42);
    assert_eq!(v.raw.as_i64(), Some(42));
}

#[test]
fn flattened_clone_eq() {
    let cfg: Config = from_str(YAML).unwrap();
    let clone = cfg.inner.clone();
    assert_eq!(clone, cfg.inner);
}

#[test]
fn flattened_as_value_and_as_raw_borrows() {
    let f = Flattened::new(42_u16, Value::from(42_i64));
    // The borrow accessors round-trip the typed and raw views
    // without consuming the wrapper.
    assert_eq!(f.as_value(), &42);
    assert_eq!(f.as_raw().as_i64(), Some(42));
    // Wrapper still usable after the borrows.
    assert_eq!(f.value, 42);
}

#[test]
fn flattened_into_value_consumes_to_typed() {
    let f = Flattened::new("hello".to_string(), Value::from("hello"));
    let typed: String = f.into_value();
    assert_eq!(typed, "hello");
}

#[test]
fn flattened_serializes_as_typed_view_only() {
    // Round-trip transparency contract: a `Flattened<T>` serializes
    // equivalently to its inner `T` — the raw view is an
    // ingest-side capture, not a serializable side-channel.
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    struct Inner {
        port: u16,
    }
    let raw: Value = from_str("port: 8080\n").unwrap();
    let f = Flattened::new(Inner { port: 8080 }, raw);
    let yaml = to_string(&f).unwrap();
    assert_eq!(yaml.trim(), "port: 8080");
}

#[test]
fn flattened_deserialize_error_propagates_from_inner_type() {
    // The two-phase deserialize first captures the Value tree,
    // then re-runs T::deserialize. When T can't accept the raw
    // shape (e.g. negative number into u16) the second phase
    // surfaces the error through `serde::de::Error::custom`.
    let res: Result<Flattened<u16>, _> = from_str("-1");
    assert!(
        res.is_err(),
        "negative integer must not coerce into u16 via Flattened",
    );
}

#[test]
fn flattened_deref_resolves_through_to_inner() {
    // Methods on T are reachable through the Deref impl.
    let raw: Value = from_str("[1, 2, 3]").unwrap();
    let f = Flattened::new(vec![1_i32, 2, 3], raw);
    assert_eq!(f.len(), 3);
    assert_eq!(f.first(), Some(&1));
}
