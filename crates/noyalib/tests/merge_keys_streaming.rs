// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 5 — native merge-key expansion in the streaming deserializer.
//!
//! The streaming path now expands `<<: *anchor` inline for the common
//! "merge is the first key" pattern. More exotic layouts (sequence merge,
//! locals-before-merge, non-mapping merge target) still fall back to the
//! AST path — these tests verify correctness on both paths.

use noyalib::from_str;
use std::collections::BTreeMap;

// ── Native: single-anchor merge at start of mapping ──────────────────────

#[test]
fn native_merge_at_start_single_anchor() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Cfg {
        timeout: u32,
        retries: u32,
        host: String,
    }
    let yaml = r#"
defaults: &d
  timeout: 30
  retries: 3
server:
  <<: *d
  host: example.com
"#;
    #[derive(serde::Deserialize)]
    struct Doc {
        server: Cfg,
    }
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.server.timeout, 30);
    assert_eq!(d.server.retries, 3);
    assert_eq!(d.server.host, "example.com");
}

#[test]
fn native_merge_local_key_overrides_merged() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Cfg {
        host: String,
        port: u16,
    }
    let yaml = r#"
base: &b
  host: default.local
  port: 80
server:
  <<: *b
  host: override.local
"#;
    #[derive(serde::Deserialize)]
    struct Doc {
        server: Cfg,
    }
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.server.host, "override.local");
    assert_eq!(d.server.port, 80);
}

#[test]
fn native_merge_into_btreemap() {
    let yaml = r#"
base: &b
  a: 1
  b: 2
target:
  <<: *b
  c: 3
"#;
    let m: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml).unwrap();
    let target = &m["target"];
    assert_eq!(target["a"], 1);
    assert_eq!(target["b"], 2);
    assert_eq!(target["c"], 3);
}

#[test]
fn native_merge_empty_target_is_noop() {
    let yaml = r#"
empty: &e {}
target:
  <<: *e
  only: here
"#;
    #[derive(serde::Deserialize)]
    struct Doc {
        target: BTreeMap<String, String>,
    }
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.target.len(), 1);
    assert_eq!(d.target["only"], "here");
}

#[test]
fn native_merge_preserves_merged_nested_value() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Limits {
        max: u32,
        min: u32,
    }
    #[derive(Debug, serde::Deserialize)]
    struct Cfg {
        limits: Limits,
        name: String,
    }
    let yaml = r#"
base: &b
  limits:
    max: 100
    min: 1
target:
  <<: *b
  name: test
"#;
    #[derive(serde::Deserialize)]
    struct Doc {
        target: Cfg,
    }
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.target.limits.max, 100);
    assert_eq!(d.target.limits.min, 1);
    assert_eq!(d.target.name, "test");
}

// ── Fallback paths: correctness preserved via AST ────────────────────────

#[test]
fn fallback_locals_before_merge_still_correct() {
    // `host` appears BEFORE the merge, so native path would let the merged
    // `host` override the local (wrong). We fall back to the AST which
    // implements correct "local wins" semantics.
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Cfg {
        host: String,
        port: u16,
    }
    let yaml = r#"
base: &b
  host: merged.local
  port: 80
server:
  host: local.local
  <<: *b
"#;
    #[derive(serde::Deserialize)]
    struct Doc {
        server: Cfg,
    }
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.server.host, "local.local", "local-before-merge must win");
    assert_eq!(d.server.port, 80);
}

#[test]
fn fallback_sequence_merge_still_correct() {
    // `<<: [*a, *b]` is not handled by native path, falls back.
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Cfg {
        a: u32,
        b: u32,
        c: u32,
    }
    let yaml = r#"
src1: &s1
  a: 1
  b: 2
src2: &s2
  b: 20
  c: 30
target:
  <<: [*s1, *s2]
"#;
    #[derive(serde::Deserialize)]
    struct Doc {
        target: Cfg,
    }
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.target.a, 1);
    // s1 takes precedence over s2 per YAML spec for sequence-of-merges.
    assert_eq!(d.target.b, 2, "first source in merge sequence wins");
    assert_eq!(d.target.c, 30);
}

// ── Error propagation: unknown merge target ──────────────────────────────

#[test]
fn unknown_merge_target_yields_anchor_error() {
    let yaml = r#"
target:
  <<: *missing
  host: x
"#;
    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)]
    struct Cfg {
        host: String,
    }
    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        target: Cfg,
    }
    let err = from_str::<Doc>(yaml).unwrap_err();
    // The error surfaces via the fallback AST path as an unknown-anchor
    // failure; variant details (UnknownAnchor vs ParseWithLocation) depend
    // on which path produced it — we just assert that a recognisable error
    // text identifies the missing anchor.
    let msg = err.to_string();
    assert!(
        msg.contains("missing") || msg.contains("unknown"),
        "expected missing-anchor signal, got: {msg}"
    );
}

// ── Round-trip with to_string preserves merged shape ────────────────────

#[test]
fn roundtrip_after_native_merge() {
    use noyalib::to_string;
    #[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
    struct Cfg {
        host: String,
        port: u16,
    }
    let yaml = r#"
base: &b
  host: db.local
  port: 5432
target:
  <<: *b
"#;
    #[derive(serde::Deserialize, serde::Serialize)]
    struct Doc {
        target: Cfg,
    }
    let d: Doc = from_str(yaml).unwrap();
    let out = to_string(&d).unwrap();
    let d2: Doc = from_str(&out).unwrap();
    assert_eq!(d.target, d2.target);
}

// ── Multiple independent mappings with their own merge keys ──────────────

#[test]
fn multiple_mappings_with_independent_merges() {
    let yaml = r#"
base_a: &ba
  x: 1
base_b: &bb
  y: 2
one:
  <<: *ba
two:
  <<: *bb
"#;
    #[derive(serde::Deserialize)]
    struct Doc {
        one: BTreeMap<String, i64>,
        two: BTreeMap<String, i64>,
    }
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.one["x"], 1);
    assert_eq!(d.two["y"], 2);
}
