// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Comprehensive integration tests for all 4 competitive features.
//!
//! Covers all permutations of anchor replay, AnchorRegistry,
//! lossless-float numeric types, and miette diagnostic bridge.

use std::collections::BTreeMap;

// ════════════════════════════════════════════════════════════════════════
// Feature 1: Anchor Event Replay in Streaming Deserializer (15+ tests)
// ════════════════════════════════════════════════════════════════════════

mod anchor_replay {
    use super::*;

    // ── Scalar anchors + aliases for each type ──────────────────────

    #[test]
    fn scalar_anchor_string() {
        let yaml = "a: &v hello\nb: *v\n";
        let map: BTreeMap<String, String> = noyalib::from_str(yaml).unwrap();
        assert_eq!(map["a"], "hello");
        assert_eq!(map["b"], "hello");
    }

    #[test]
    fn scalar_anchor_integer() {
        let yaml = "x: &n 42\ny: *n\n";
        let map: BTreeMap<String, i64> = noyalib::from_str(yaml).unwrap();
        assert_eq!(map["x"], 42);
        assert_eq!(map["y"], 42);
    }

    #[test]
    fn scalar_anchor_float() {
        let yaml = "x: &f 1.234\ny: *f\n";
        let map: BTreeMap<String, f64> = noyalib::from_str(yaml).unwrap();
        assert!((map["x"] - 1.234).abs() < 1e-10);
        assert!((map["y"] - 1.234).abs() < 1e-10);
    }

    #[test]
    fn scalar_anchor_bool() {
        let yaml = "a: &flag true\nb: *flag\n";
        let map: BTreeMap<String, bool> = noyalib::from_str(yaml).unwrap();
        assert!(map["a"]);
        assert!(map["b"]);
    }

    #[test]
    fn scalar_anchor_null() {
        let yaml = "a: &nil null\nb: *nil\n";
        let map: BTreeMap<String, Option<String>> = noyalib::from_str(yaml).unwrap();
        assert!(map["a"].is_none());
        assert!(map["b"].is_none());
    }

    // ── Mapping anchor + alias ──────────────────────────────────────

    #[test]
    fn mapping_anchor_alias() {
        let yaml = "base: &cfg\n  host: localhost\n  port: 8080\ncopy: *cfg\n";
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Endpoint {
            host: String,
            port: u16,
        }
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            base: Endpoint,
            copy: Endpoint,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.base, doc.copy);
    }

    // ── Sequence anchor + alias ─────────────────────────────────────

    #[test]
    fn sequence_anchor_alias() {
        let yaml = "orig: &items\n  - a\n  - b\n  - c\ncopy: *items\n";
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Doc {
            orig: Vec<String>,
            copy: Vec<String>,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.orig, doc.copy);
        assert_eq!(doc.orig, vec!["a", "b", "c"]);
    }

    // ── Nested: anchor inside mapping value, alias at top level ─────

    #[test]
    fn nested_anchor_in_mapping_value() {
        let yaml = r"
config:
  db: &db_cfg
    host: db.local
    port: 5432
replica: *db_cfg
";
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct DbCfg {
            host: String,
            port: u16,
        }
        #[derive(Debug, serde::Deserialize)]
        struct Config {
            db: DbCfg,
        }
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            config: Config,
            replica: DbCfg,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.config.db, doc.replica);
    }

    // ── Multiple aliases to same anchor ─────────────────────────────

    #[test]
    fn multiple_aliases_same_anchor() {
        let yaml = "src: &v 99\na: *v\nb: *v\nc: *v\nd: *v\n";
        let map: BTreeMap<String, i64> = noyalib::from_str(yaml).unwrap();
        assert_eq!(map.len(), 5);
        for key in &["src", "a", "b", "c", "d"] {
            assert_eq!(map[*key], 99);
        }
    }

    // ── Anchor defined after alias reference (should error) ─────────

    #[test]
    fn alias_before_anchor_errors() {
        let yaml = "first: *later\nsecond: &later hello\n";
        let result: Result<BTreeMap<String, String>, _> = noyalib::from_str(yaml);
        assert!(result.is_err(), "alias before anchor should fail");
    }

    // ── Anchor with typed deserialization: struct ────────────────────

    #[test]
    fn anchor_typed_struct() {
        let yaml = r"
primary: &srv
  name: web-01
  cpu: 4
  memory: 16
failover: *srv
";
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Server {
            name: String,
            cpu: u32,
            memory: u32,
        }
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            primary: Server,
            failover: Server,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.primary, doc.failover);
    }

    // ── Anchor with typed deserialization: enum ──────────────────────

    #[test]
    fn anchor_typed_enum() {
        let yaml = "status: &s active\ncopy: *s\n";
        #[derive(Debug, serde::Deserialize, PartialEq)]
        #[serde(rename_all = "lowercase")]
        enum Status {
            Active,
            Inactive,
        }
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            status: Status,
            copy: Status,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.status, Status::Active);
        assert_eq!(doc.copy, Status::Active);
    }

    // ── Anchor with typed deserialization: tuple ─────────────────────

    #[test]
    fn anchor_typed_tuple() {
        let yaml = "pair: &p\n  - 10\n  - 20\ncopy: *p\n";
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Doc {
            pair: (i32, i32),
            copy: (i32, i32),
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.pair, (10, 20));
        assert_eq!(doc.copy, (10, 20));
    }

    // ── Anchor in sequence items ────────────────────────────────────

    #[test]
    fn anchor_in_sequence_items() {
        let yaml = "items:\n  - &x 10\n  - &y 20\n  - *x\n  - *y\n  - *x\n";
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            items: Vec<i32>,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.items, vec![10, 20, 10, 20, 10]);
    }

    // ── Anchor in mapping keys (handled via Value fallback) ─────────

    #[test]
    fn anchor_in_mapping_keys_via_value() {
        let yaml = "&key name: Alice\n";
        // Anchors on mapping keys may need fallback; verify it at least parses.
        let v: noyalib::Value = noyalib::from_str(yaml).unwrap();
        assert!(v.get("name").is_some());
    }

    // ── Deep nesting with anchors ───────────────────────────────────

    #[test]
    fn deep_nesting_with_anchors() {
        let yaml = r"
l1:
  l2:
    l3: &deep
      l4:
        value: found
shallow: *deep
";
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct L4 {
            value: String,
        }
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct L3 {
            l4: L4,
        }
        #[derive(Debug, serde::Deserialize)]
        struct L2 {
            l3: L3,
        }
        #[derive(Debug, serde::Deserialize)]
        struct L1 {
            l2: L2,
        }
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            l1: L1,
            shallow: L3,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.l1.l2.l3, doc.shallow);
    }

    // ── Roundtrip: serialize struct, verify output ───────────────────

    #[test]
    fn roundtrip_serialize_deserialize() {
        #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
        struct Data {
            name: String,
            count: u32,
        }

        let original = Data {
            name: "test".into(),
            count: 42,
        };
        let yaml = noyalib::to_string(&original).unwrap();
        let restored: Data = noyalib::from_str(&yaml).unwrap();
        assert_eq!(original, restored);
    }

    // ── Large document with many anchors ─────────────────────────────

    #[test]
    fn large_document_many_anchors() {
        let mut yaml = String::new();
        for i in 0..100 {
            yaml.push_str(&format!("key{i}: &anchor{i} value_{i}\n"));
        }
        for i in 0..100 {
            yaml.push_str(&format!("alias{i}: *anchor{i}\n"));
        }
        let map: BTreeMap<String, String> = noyalib::from_str(&yaml).unwrap();
        assert_eq!(map.len(), 200);
        for i in 0..100 {
            assert_eq!(
                map[&format!("key{i}")],
                map[&format!("alias{i}")],
                "anchor{i} mismatch"
            );
        }
    }

    // ── Anchor + alias + merge key (should work via fallback) ───────

    #[test]
    fn anchor_alias_merge_key_fallback() {
        let yaml = r"
base: &base
  timeout: 30
  retries: 3
server:
  <<: *base
  host: example.com
";
        let v: noyalib::Value = noyalib::from_str(yaml).unwrap();
        let server = v.get("server").unwrap();
        assert_eq!(server.get("timeout").unwrap(), &noyalib::Value::from(30));
        assert_eq!(server.get("retries").unwrap(), &noyalib::Value::from(3));
        assert_eq!(
            server.get("host").unwrap(),
            &noyalib::Value::from("example.com")
        );
    }
}

// ════════════════════════════════════════════════════════════════════════
// Feature 2: AnchorRegistry / ArcAnchorRegistry (10+ tests)
// ════════════════════════════════════════════════════════════════════════

mod anchor_registry {
    use noyalib::{AnchorRegistry, ArcAnchorRegistry};
    use std::rc::Rc;
    use std::sync::Arc;

    #[test]
    fn register_and_resolve_single() {
        let mut reg = AnchorRegistry::<String>::new();
        let rc = reg.register("greeting".into(), "hello".into());
        let resolved = reg.resolve("greeting").unwrap();
        assert_eq!(*resolved, "hello");
        assert!(Rc::ptr_eq(&rc, &resolved));
    }

    #[test]
    fn register_multiple_resolve_each() {
        let mut reg = AnchorRegistry::<i32>::new();
        let _ = reg.register("a".into(), 1);
        let _ = reg.register("b".into(), 2);
        let _ = reg.register("c".into(), 3);
        assert_eq!(*reg.resolve("a").unwrap(), 1);
        assert_eq!(*reg.resolve("b").unwrap(), 2);
        assert_eq!(*reg.resolve("c").unwrap(), 3);
    }

    #[test]
    fn resolve_unknown_returns_none() {
        let reg = AnchorRegistry::<String>::new();
        assert!(reg.resolve("nonexistent").is_none());
    }

    #[test]
    fn multiple_resolves_same_rc_ptr() {
        let mut reg = AnchorRegistry::<String>::new();
        let original = reg.register("key".into(), "value".into());
        let r1 = reg.resolve("key").unwrap();
        let r2 = reg.resolve("key").unwrap();
        let r3 = reg.resolve("key").unwrap();
        assert!(Rc::ptr_eq(&original, &r1));
        assert!(Rc::ptr_eq(&r1, &r2));
        assert!(Rc::ptr_eq(&r2, &r3));
    }

    #[test]
    fn arc_register_and_resolve() {
        let mut reg = ArcAnchorRegistry::<String>::new();
        let arc = reg.register("key".into(), "value".into());
        let resolved = reg.resolve("key").unwrap();
        assert_eq!(*resolved, "value");
        assert!(Arc::ptr_eq(&arc, &resolved));
    }

    #[test]
    fn arc_multiple_resolves_same_ptr() {
        let mut reg = ArcAnchorRegistry::<i64>::new();
        let original = reg.register("n".into(), 99);
        let r1 = reg.resolve("n").unwrap();
        let r2 = reg.resolve("n").unwrap();
        assert!(Arc::ptr_eq(&original, &r1));
        assert!(Arc::ptr_eq(&r1, &r2));
    }

    #[test]
    fn arc_send_across_threads() {
        let mut reg = ArcAnchorRegistry::<String>::new();
        let arc = reg.register("data".into(), "cross-thread".into());
        let alias = reg.resolve("data").unwrap();

        let handle = std::thread::spawn(move || {
            assert_eq!(*alias, "cross-thread");
            Arc::strong_count(&alias)
        });
        let count = handle.join().unwrap();
        // After thread finishes and drops its alias, strong count reduces.
        assert!(count >= 2);
        assert_eq!(*arc, "cross-thread");
    }

    #[test]
    fn clear_and_reuse() {
        let mut reg = AnchorRegistry::<String>::new();
        let _ = reg.register("a".into(), "alpha".into());
        let _ = reg.register("b".into(), "beta".into());
        assert_eq!(reg.len(), 2);

        reg.clear();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.resolve("a").is_none());

        let _ = reg.register("c".into(), "gamma".into());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn len_and_is_empty() {
        let mut reg = AnchorRegistry::<u8>::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);

        let _ = reg.register("x".into(), 1);
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);

        let _ = reg.register("y".into(), 2);
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn default_constructor() {
        let reg = AnchorRegistry::<String>::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn arc_default_constructor() {
        let reg = ArcAnchorRegistry::<String>::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn overwrite_replaces_entry() {
        let mut reg = AnchorRegistry::<String>::new();
        let first = reg.register("key".into(), "first".into());
        let second = reg.register("key".into(), "second".into());
        let resolved = reg.resolve("key").unwrap();
        assert!(!Rc::ptr_eq(&first, &second));
        assert!(Rc::ptr_eq(&second, &resolved));
        assert_eq!(*resolved, "second");
    }

    #[test]
    fn arc_resolve_unknown_returns_none() {
        let reg = ArcAnchorRegistry::<u32>::new();
        assert!(reg.resolve("missing").is_none());
    }

    #[test]
    fn arc_clear_and_reuse() {
        let mut reg = ArcAnchorRegistry::<i32>::new();
        let _ = reg.register("a".into(), 1);
        let _ = reg.register("b".into(), 2);
        assert_eq!(reg.len(), 2);
        reg.clear();
        assert!(reg.is_empty());
        let _ = reg.register("c".into(), 3);
        assert_eq!(*reg.resolve("c").unwrap(), 3);
    }
}

// ════════════════════════════════════════════════════════════════════════
// Feature 3: Lossless Float (strict numeric profile)
// ════════════════════════════════════════════════════════════════════════

#[cfg(feature = "lossless-float")]
// Pins the `lossless_float` surface (formerly `robotics::StrictFloat`;
// the `robotics` module and its `Degrees`/`Radians` newtypes were
// removed in v0.0.30 per the one-release deprecation).
mod lossless_float_tests {
    use noyalib::lossless_float::LosslessFloat;

    #[test]
    fn lossless_float_valid_zero() {
        let sf: LosslessFloat = noyalib::from_str("0.0").unwrap();
        assert!((sf.get()).abs() < 1e-15);
    }

    #[test]
    fn lossless_float_valid_positive() {
        let sf: LosslessFloat = noyalib::from_str("1.0").unwrap();
        assert!((sf.get() - 1.0).abs() < 1e-15);
    }

    #[test]
    fn lossless_float_valid_negative() {
        let sf: LosslessFloat = noyalib::from_str("-1.0").unwrap();
        assert!((sf.get() + 1.0).abs() < 1e-15);
    }

    #[test]
    fn lossless_float_valid_large() {
        let sf: LosslessFloat = noyalib::from_str("1.0e10").unwrap();
        assert!((sf.get() - 1.0e10).abs() < 1.0);
    }

    #[test]
    fn lossless_float_reject_nan() {
        let result: Result<LosslessFloat, _> = noyalib::from_str(".nan");
        assert!(result.is_err());
    }

    #[test]
    fn lossless_float_reject_infinity() {
        let result: Result<LosslessFloat, _> = noyalib::from_str(".inf");
        assert!(result.is_err());
    }

    #[test]
    fn lossless_float_reject_neg_infinity() {
        let result: Result<LosslessFloat, _> = noyalib::from_str("-.inf");
        assert!(result.is_err());
    }

    #[test]
    fn lossless_float_precision_roundtrip() {
        let sf: LosslessFloat = noyalib::from_str("1.23456789012345").unwrap();
        let yaml = noyalib::to_string(&sf).unwrap();
        let restored: LosslessFloat = noyalib::from_str(yaml.trim()).unwrap();
        assert!((sf.get() - restored.get()).abs() < 1e-15);
    }

    #[test]
    fn lossless_float_try_from_valid() {
        let sf = LosslessFloat::try_from(2.5).unwrap();
        assert!((sf.get() - 2.5).abs() < 1e-15);
    }

    #[test]
    fn lossless_float_try_from_nan_fails() {
        let result = LosslessFloat::try_from(f64::NAN);
        assert!(result.is_err());
    }

    #[test]
    fn lossless_float_try_from_inf_fails() {
        let result = LosslessFloat::try_from(f64::INFINITY);
        assert!(result.is_err());
    }
}

// ════════════════════════════════════════════════════════════════════════
// Feature 4: Diagnostic Bridge (5+ tests)
// ════════════════════════════════════════════════════════════════════════

#[cfg(feature = "miette")]
mod diagnostic_tests {
    use miette::Diagnostic;
    use noyalib::Spanned;

    #[test]
    fn spanned_error_creates_report() {
        let yaml = "port: 80\n";
        #[derive(serde::Deserialize)]
        struct Cfg {
            port: Spanned<u16>,
        }
        let cfg: Cfg = noyalib::from_str(yaml).unwrap();
        let report = noyalib::diagnostic::spanned_error(yaml, &cfg.port, "port must be >= 1024");
        let msg = format!("{report}");
        assert!(msg.contains("port must be >= 1024"));
    }

    #[test]
    fn diagnostic_has_correct_source_code() {
        let yaml = "value: 42\n";
        #[derive(serde::Deserialize)]
        struct Doc {
            value: Spanned<i32>,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        let report = noyalib::diagnostic::spanned_error(yaml, &doc.value, "too small");
        let diag: &dyn Diagnostic = report.as_ref();
        assert!(diag.source_code().is_some());
    }

    #[test]
    fn diagnostic_has_correct_labels() {
        let yaml = "value: 42\n";
        #[derive(serde::Deserialize)]
        struct Doc {
            value: Spanned<i32>,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        let report = noyalib::diagnostic::spanned_error(yaml, &doc.value, "invalid");
        let diag: &dyn Diagnostic = report.as_ref();
        let labels: Vec<_> = diag.labels().unwrap().collect();
        assert!(!labels.is_empty());
        assert!(labels[0].label().unwrap().contains("invalid"));
    }

    #[test]
    fn diagnostic_has_code() {
        let yaml = "x: 1\n";
        #[derive(serde::Deserialize)]
        struct Doc {
            x: Spanned<i32>,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        let report = noyalib::diagnostic::spanned_error(yaml, &doc.x, "bad");
        let diag: &dyn Diagnostic = report.as_ref();
        let code = diag.code().unwrap().to_string();
        assert_eq!(code, "noyalib::validation");
    }

    #[test]
    fn multiple_validation_errors() {
        let yaml = "host: \"\"\nport: 80\n";
        #[derive(serde::Deserialize)]
        struct Cfg {
            host: Spanned<String>,
            port: Spanned<u16>,
        }
        let cfg: Cfg = noyalib::from_str(yaml).unwrap();
        let mut errors: Vec<miette::Report> = Vec::new();

        if cfg.host.value.is_empty() {
            errors.push(noyalib::diagnostic::spanned_error(
                yaml,
                &cfg.host,
                "host must not be empty",
            ));
        }
        if cfg.port.value < 1024 {
            errors.push(noyalib::diagnostic::spanned_error(
                yaml,
                &cfg.port,
                "port must be >= 1024",
            ));
        }

        assert_eq!(errors.len(), 2);
        assert!(errors[0].to_string().contains("host must not be empty"));
        assert!(errors[1].to_string().contains("port must be >= 1024"));
    }

    #[test]
    fn integration_parse_validate_diagnose() {
        let yaml = "database:\n  host: db.local\n  port: 80\n  name: prod\n";
        #[derive(serde::Deserialize)]
        struct DbCfg {
            #[allow(dead_code)]
            host: String,
            port: Spanned<u16>,
            #[allow(dead_code)]
            name: String,
        }
        #[derive(serde::Deserialize)]
        struct Root {
            database: DbCfg,
        }
        let root: Root = noyalib::from_str(yaml).unwrap();
        assert!(root.database.port.value < 1024);

        let report = noyalib::diagnostic::spanned_error(
            yaml,
            &root.database.port,
            "database port must be >= 1024",
        );

        // Verify the diagnostic renders without panicking.
        let rendered = format!("{report:?}");
        assert!(!rendered.is_empty());
        assert!(report.to_string().contains("database port must be >= 1024"));
    }

    #[test]
    fn spanned_error_with_zero_length_span() {
        // Edge case: Spanned with default (zero) locations.
        let spanned = Spanned::new(42_i32);
        let yaml = "42";
        let report = noyalib::diagnostic::spanned_error(yaml, &spanned, "bad value");
        // Should not panic, should produce a valid report.
        let msg = format!("{report}");
        assert!(msg.contains("bad value"));
    }
}
