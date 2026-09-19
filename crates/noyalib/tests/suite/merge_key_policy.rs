// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `ParserConfig::merge_key_policy` — control over how YAML's
//! merge key (`<<`) is handled at parse time.

#![allow(missing_docs)]

use noyalib::{MergeKeyPolicy, ParserConfig, Value, from_str_with_config};

const DOC_WITH_MERGE: &str = "\
defaults: &cfg
  port: 8080
  host: localhost
service:
  <<: *cfg
  host: api.example.com
";

#[test]
fn auto_policy_is_default() {
    assert_eq!(ParserConfig::new().merge_key_policy, MergeKeyPolicy::Auto);
}

#[test]
fn auto_policy_merges_into_enclosing_mapping() {
    let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::Auto);
    let v: Value = from_str_with_config(DOC_WITH_MERGE, &cfg).unwrap();
    // Both keys present in service, host overridden by the local entry.
    assert_eq!(v["service"]["port"].as_i64(), Some(8080));
    assert_eq!(v["service"]["host"].as_str(), Some("api.example.com"));
    // The literal `<<` key is gone — it has been spliced.
    let service = match &v["service"] {
        Value::Mapping(m) => m,
        _ => panic!("expected mapping"),
    };
    assert!(!service.contains_key("<<"));
}

#[test]
fn as_ordinary_policy_keeps_double_angle_as_literal_key() {
    let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::AsOrdinary);
    let v: Value = from_str_with_config(DOC_WITH_MERGE, &cfg).unwrap();
    let service = match &v["service"] {
        Value::Mapping(m) => m,
        _ => panic!("expected mapping"),
    };
    // `<<` survives as a literal key carrying the alias's resolved
    // value (a mapping).
    assert!(service.contains_key("<<"));
    let merge_value = &service["<<"];
    assert!(matches!(merge_value, Value::Mapping(_)));
    // The local `host` is still present; no merge occurred so
    // `port` is NOT lifted into the enclosing mapping.
    assert_eq!(
        service.get("host").and_then(|v| v.as_str()),
        Some("api.example.com")
    );
    assert!(!service.contains_key("port"));
}

#[test]
fn error_policy_rejects_documents_with_merge_key() {
    let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::Error);
    let err = from_str_with_config::<Value>(DOC_WITH_MERGE, &cfg).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("merge key") && msg.contains("MergeKeyPolicy::Error"),
        "expected actionable rejection message, got: {msg}"
    );
}

#[test]
fn error_policy_passes_documents_without_merge_key() {
    let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::Error);
    // No `<<` anywhere — the policy is a no-op.
    let yaml = "\
service:
  host: api.example.com
  port: 8080
";
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(v["service"]["port"].as_i64(), Some(8080));
}

#[test]
fn as_ordinary_round_trips_double_angle_in_emit() {
    // When a user has `<<` as a literal key (e.g. some tooling emits
    // it as a sentinel), AsOrdinary preserves it through the typed
    // value tree.
    let yaml = "weights: { <<: 1.5, normal: 1.0 }\n";
    let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::AsOrdinary);
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    let weights = match &v["weights"] {
        Value::Mapping(m) => m,
        _ => panic!("expected mapping"),
    };
    assert!(weights.contains_key("<<"));
    assert_eq!(weights["<<"].as_f64(), Some(1.5));
}

#[test]
fn auto_with_sequence_of_merge_targets() {
    // YAML 1.2 §10.2 lets `<<:` take a sequence of mappings.
    let yaml = "\
base_a: &a
  x: 1
base_b: &b
  y: 2
combined:
  <<: [*a, *b]
  z: 3
";
    let cfg = ParserConfig::new();
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(v["combined"]["x"].as_i64(), Some(1));
    assert_eq!(v["combined"]["y"].as_i64(), Some(2));
    assert_eq!(v["combined"]["z"].as_i64(), Some(3));
}

#[test]
fn as_ordinary_with_sequence_value_keeps_sequence() {
    let yaml = "\
base_a: &a
  x: 1
combined:
  <<: [*a]
  z: 3
";
    let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::AsOrdinary);
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    let combined = match &v["combined"] {
        Value::Mapping(m) => m,
        _ => panic!("expected mapping"),
    };
    // `<<` survives as a sequence; merge did NOT happen, so x is
    // not in the enclosing mapping.
    assert!(matches!(combined["<<"], Value::Sequence(_)));
    assert!(!combined.contains_key("x"));
    assert_eq!(combined["z"].as_i64(), Some(3));
}
