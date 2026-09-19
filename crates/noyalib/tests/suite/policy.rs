// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Pluggable parser policies — `ParserConfig::with_policy`.
//!
//! Verifies the trait-based "Safe YAML" enforcement that ships
//! with v0.0.1. Each test pins the public API contract: the policy
//! fires during parsing, the streaming fast-path is bypassed when a
//! policy is present, and custom policies compose with the
//! built-ins.

#![allow(missing_docs)]

use noyalib::policy::{
    DenyAnchors, DenyTags, MaxScalarLength, Policy, PolicyEvent, PolicyEventKind,
};
use noyalib::{ParserConfig, Value, from_str_with_config};

// ── DenyAnchors ─────────────────────────────────────────────────────

#[test]
fn deny_anchors_rejects_anchor_definition() {
    let cfg = ParserConfig::new().with_policy(DenyAnchors);
    let res: Result<Value, _> = from_str_with_config("a: &x 1\n", &cfg);
    assert!(res.is_err());
    let msg = format!("{}", res.err().unwrap());
    assert!(msg.contains("DenyAnchors"), "got: {msg}");
    assert!(msg.contains("&x"), "should name the anchor: {msg}");
}

#[test]
fn deny_anchors_rejects_alias_dereference() {
    let cfg = ParserConfig::new().with_policy(DenyAnchors);
    let res: Result<Value, _> = from_str_with_config("a: &x 1\nb: *x\n", &cfg);
    assert!(res.is_err());
}

#[test]
fn deny_anchors_passes_anchor_free_input() {
    let cfg = ParserConfig::new().with_policy(DenyAnchors);
    let v: Value = from_str_with_config("a: 1\nb: 2\n", &cfg).unwrap();
    assert!(matches!(v, Value::Mapping(_)));
}

// ── DenyTags ───────────────────────────────────────────────────────

#[test]
fn deny_tags_rejects_custom_tag() {
    let cfg = ParserConfig::new().with_policy(DenyTags);
    let res: Result<Value, _> = from_str_with_config("a: !Custom 1\n", &cfg);
    assert!(res.is_err());
    let msg = format!("{}", res.err().unwrap());
    assert!(msg.contains("DenyTags"), "got: {msg}");
}

#[test]
fn deny_tags_allows_core_tags() {
    // The YAML 1.2 core schema tags must survive — they're the
    // mechanism by which a user *forces* a type, and policy denial
    // would break too much idiomatic YAML.
    let cfg = ParserConfig::new().with_policy(DenyTags);
    let v: Value = from_str_with_config("a: !!str 42\n", &cfg).unwrap();
    assert!(matches!(v, Value::Mapping(_)));
}

// ── MaxScalarLength ────────────────────────────────────────────────

#[test]
fn max_scalar_length_rejects_oversize_scalars() {
    let cfg = ParserConfig::new().with_policy(MaxScalarLength(8));
    let res: Result<Value, _> = from_str_with_config("a: thisisreallylong\n", &cfg);
    assert!(res.is_err());
}

#[test]
fn max_scalar_length_accepts_short_scalars() {
    let cfg = ParserConfig::new().with_policy(MaxScalarLength(8));
    let v: Value = from_str_with_config("a: short\n", &cfg).unwrap();
    assert!(matches!(v, Value::Mapping(_)));
}

// ── Custom policy & composition ────────────────────────────────────

#[derive(Debug, Default)]
struct DenyKeyContains(&'static str);

impl Policy for DenyKeyContains {
    fn check_event(&self, event: PolicyEvent<'_>) -> noyalib::Result<()> {
        if event.kind == PolicyEventKind::Scalar {
            if let Some(s) = event.scalar {
                if s.contains(self.0) {
                    return Err(noyalib::Error::Deserialize(format!(
                        "policy DenyKeyContains: scalar contains forbidden substring `{}`",
                        self.0
                    )));
                }
            }
        }
        Ok(())
    }
}

#[test]
fn custom_policy_fires() {
    let cfg = ParserConfig::new().with_policy(DenyKeyContains("secret"));
    let res: Result<Value, _> = from_str_with_config("password: secret123\n", &cfg);
    assert!(res.is_err());
}

#[test]
fn multiple_policies_short_circuit_on_first_failure() {
    let cfg = ParserConfig::new()
        .with_policy(DenyAnchors)
        .with_policy(DenyTags);
    // Both policies reject — first one to fire wins.
    let res: Result<Value, _> = from_str_with_config("a: !Custom &x 1\n", &cfg);
    assert!(res.is_err());
}

#[test]
fn multiple_policies_compose_when_all_accept() {
    let cfg = ParserConfig::new()
        .with_policy(DenyAnchors)
        .with_policy(DenyTags)
        .with_policy(MaxScalarLength(64));
    let v: Value = from_str_with_config("a: ok\nb: 42\n", &cfg).unwrap();
    assert!(matches!(v, Value::Mapping(_)));
}

// ── Streaming bypass ───────────────────────────────────────────────

#[test]
fn policies_route_through_ast_path_to_enforce_contract() {
    // The streaming fast-path doesn't run policies, so the loader
    // auto-bypasses streaming when any policy is registered. The
    // visible contract: policy enforcement is uniform regardless of
    // whether the document would normally take the streaming path.
    #[derive(Debug, serde::Deserialize)]
    struct Doc {
        #[allow(dead_code)]
        a: u32,
    }
    let cfg = ParserConfig::new().with_policy(DenyAnchors);
    let res: Result<Doc, _> = from_str_with_config("a: &x 1\n", &cfg);
    assert!(
        res.is_err(),
        "policy must fire even on inputs that would normally stream"
    );
}
