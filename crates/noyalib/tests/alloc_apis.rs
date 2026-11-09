// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Alloc-capable public APIs have the same behavior with and without std.

#![allow(missing_docs)]

use noyalib::{ParserConfig, RcRecursive, Value, from_str, from_str_with_config};
use std::sync::Arc;

#[cfg(feature = "std")]
type Properties = std::collections::HashMap<String, String>;
#[cfg(not(feature = "std"))]
type Properties = hashbrown::HashMap<String, String, rustc_hash::FxBuildHasher>;

#[test]
fn cst_growth_retains_comments_and_rejects_invalid_edits() {
    let mut doc = noyalib::cst::parse_document("items: x # keep\r\nnext: 1\r\n").unwrap();
    let items = from_str("[a, b]").unwrap();
    doc.grow_value("items", &items).unwrap();
    assert!(doc.source().contains("# keep"));
    assert!(doc.source().contains("\r\n"));
    assert_eq!(doc.as_value()["items"], items);
    let before = doc.source().to_owned();
    assert!(doc.set("next", "[unclosed").is_err());
    assert_eq!(doc.source(), before);
}

#[test]
fn properties_expand_on_typed_value_and_document_paths() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Config {
        name: String,
    }
    let mut props = Properties::default();
    let _ = props.insert("NAME".into(), "api".into());
    let cfg = ParserConfig::new()
        .properties(Arc::new(props))
        .strict_properties(true);
    let text = "name: ${NAME}\n";
    let typed: Config = from_str_with_config(text, &cfg).unwrap();
    assert_eq!(typed.name, "api");
    let value: Value = from_str_with_config(text, &cfg).unwrap();
    assert_eq!(value["name"].as_str(), Some("api"));
    let docs: Vec<Config> = noyalib::load_all_as_with_config(text, &cfg).unwrap();
    assert_eq!(docs, [typed]);
    assert!(from_str_with_config::<Value>("name: ${MISSING}", &cfg).is_err());
    let mut large = Properties::default();
    let _ = large.insert("NAME".into(), "x".repeat(100));
    let tiny = cfg.properties(Arc::new(large)).max_total_scalar_bytes(32);
    assert!(from_str_with_config::<Value>(text, &tiny).is_err());
}

#[test]
fn value_interpolation_supports_defaults_and_redaction() {
    let mut value: Value = from_str("name: ${NAME:-fallback}\n").unwrap();
    let props = Properties::default();
    value.interpolate_properties(&props).unwrap();
    assert_eq!(value["name"].as_str(), Some("fallback"));
    let mut missing: Value = from_str("name: ${PRIVATE_KEY}\n").unwrap();
    let error = missing.interpolate_properties_redacted(&props).unwrap_err();
    assert!(!error.to_string().contains("PRIVATE_KEY"));
}

#[test]
fn recursive_rc_anchors_share_values_and_release_weak_references() {
    let anchor = RcRecursive::empty();
    let weak = anchor.downgrade();
    let cloned = anchor.clone();
    assert!(anchor.set(42_u16).is_none());
    assert_eq!(*cloned.borrow(), Some(42));
    assert_eq!(noyalib::to_string(&anchor).unwrap(), "42");
    drop(anchor);
    assert!(weak.upgrade().is_some());
    drop(cloned);
    assert!(weak.upgrade().is_none());
}

#[cfg(feature = "strict-deserialise")]
#[test]
fn strict_helpers_report_unknown_paths_from_strings_and_slices() {
    #[derive(Debug, serde::Deserialize)]
    struct Config {
        port: u16,
    }
    let cfg: Config = noyalib::from_str_strict("port: 8080").unwrap();
    assert_eq!(cfg.port, 8080);
    for error in [
        noyalib::from_str_strict::<Config>("port: 8080\nporrt: 80").unwrap_err(),
        noyalib::from_slice_strict::<Config>(b"port: 8080\nporrt: 80").unwrap_err(),
    ] {
        assert!(matches!(error, noyalib::Error::UnknownField(_)));
        assert!(error.to_string().contains("porrt"));
    }
}

#[cfg(feature = "recovery")]
#[test]
fn recovery_retains_good_documents_and_enforces_stream_budgets() {
    let parsed = noyalib::recovery::parse_lenient("a: 1\n---\nb: [unclosed\n");
    assert!(!parsed.is_complete);
    assert!(!parsed.errors.is_empty());
    assert_eq!(parsed.value[0]["a"].as_i64(), Some(1));
    let cfg = ParserConfig::new().max_documents(1);
    let recovery_cfg = noyalib::recovery::LenientConfig {
        base_config: cfg,
        ..Default::default()
    };
    let limited = noyalib::recovery::parse_lenient_with("1\n---\n2\n", &recovery_cfg);
    assert!(!limited.is_complete);
    assert!(!limited.errors.is_empty());
}
