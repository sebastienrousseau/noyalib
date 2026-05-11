// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `noyalib::figment::Yaml` provider integration tests.
//!
//! Asserts that noyalib drops cleanly into figment's layered
//! configuration chains — the same way `figment::providers::Toml`
//! and `figment::providers::Json` do.

#![cfg(feature = "figment")]
#![allow(missing_docs)]

use figment::Figment;
use figment::providers::Format;
use noyalib::figment::Yaml;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Cfg {
    name: String,
    port: u16,
}

#[test]
fn extract_from_string() {
    let yaml = "name: noyalib\nport: 8080\n";
    let cfg: Cfg = Figment::new().merge(Yaml::string(yaml)).extract().unwrap();
    assert_eq!(
        cfg,
        Cfg {
            name: "noyalib".into(),
            port: 8080,
        }
    );
}

#[test]
fn merge_two_yaml_layers_later_wins() {
    let base = "name: base\nport: 1\n";
    let overlay = "port: 9999\n";
    let cfg: Cfg = Figment::new()
        .merge(Yaml::string(base))
        .merge(Yaml::string(overlay))
        .extract()
        .unwrap();
    // The later `merge` wins for overlapping keys; `name` is
    // inherited from the base.
    assert_eq!(cfg.name, "base");
    assert_eq!(cfg.port, 9999);
}

#[test]
fn join_skips_overlap_first_wins() {
    // `join` is the opposite of `merge` — keys in the first
    // provider are kept; later providers only fill gaps.
    let base = "name: base\nport: 1\n";
    let overlay = "port: 9999\n";
    let cfg: Cfg = Figment::new()
        .merge(Yaml::string(base))
        .join(Yaml::string(overlay))
        .extract()
        .unwrap();
    // `name` from base (only source); `port` is `1` because
    // `join` doesn't overwrite.
    assert_eq!(cfg.name, "base");
    assert_eq!(cfg.port, 1);
}

#[test]
fn extract_from_file_via_figment_helper() {
    use std::io::Write as _;
    let dir = std::env::temp_dir().join("noyalib_figment_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.yaml");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"name: from-file\nport: 4242\n").unwrap();
    }
    let cfg: Cfg = Figment::new().merge(Yaml::file(&path)).extract().unwrap();
    assert_eq!(cfg.name, "from-file");
    assert_eq!(cfg.port, 4242);
}

#[test]
fn parse_error_propagates_as_figment_error() {
    // Invalid YAML — the figment chain must surface the parse
    // error at extract-time rather than panicking during merge.
    let bad = "port: { unclosed";
    let res = Figment::new().merge(Yaml::string(bad)).extract::<Cfg>();
    assert!(res.is_err(), "expected figment Error, got: {res:?}");
}

#[test]
fn missing_required_field_surfaces_clear_error() {
    let yaml = "name: only-name\n";
    let res: Result<Cfg, _> = Figment::new().merge(Yaml::string(yaml)).extract();
    let err = res.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("port"),
        "missing-field error must name the field, got: {msg}"
    );
}

#[test]
fn nested_struct_round_trip() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Db {
        url: String,
        pool_size: u16,
    }
    #[derive(Debug, Deserialize, PartialEq)]
    struct App {
        name: String,
        db: Db,
    }

    let yaml = "\
name: nested
db:
  url: postgres://localhost/x
  pool_size: 16
";
    let app: App = Figment::new().merge(Yaml::string(yaml)).extract().unwrap();
    assert_eq!(
        app,
        App {
            name: "nested".into(),
            db: Db {
                url: "postgres://localhost/x".into(),
                pool_size: 16,
            },
        }
    );
}

#[test]
fn anchor_and_alias_resolved_through_provider() {
    // The provider must honour YAML 1.2 anchor / alias semantics —
    // `<<: *anchor` should produce a merged mapping when extracted.
    #[derive(Debug, Deserialize, PartialEq)]
    struct Service {
        host: String,
        port: u16,
    }

    let yaml = "\
defaults: &cfg
  host: localhost
  port: 8080
service:
  <<: *cfg
  host: api.example.com
";
    let cfg: figment::value::Value = Figment::new().merge(Yaml::string(yaml)).extract().unwrap();
    // The figment value tree is a generic map; we re-extract
    // the `service` slice into a typed struct.
    let svc: Service = cfg
        .find("service")
        .expect("service key present")
        .deserialize()
        .unwrap();
    assert_eq!(svc.host, "api.example.com");
    assert_eq!(svc.port, 8080);
}
