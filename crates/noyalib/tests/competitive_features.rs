// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Integration tests for the 4 competitive features.

// ── Feature 1: Anchor Event Replay in Streaming Deserializer ────────────

/// Tests that anchors and aliases work through the streaming path
/// (which used to fall back to the Value AST path).
mod anchor_replay {

    #[test]
    fn scalar_anchor_alias() {
        let yaml = "a: &val hello\nb: *val\n";
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Doc {
            a: String,
            b: String,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.a, "hello");
        assert_eq!(doc.b, "hello");
    }

    #[test]
    fn sequence_anchor_alias() {
        let yaml = "original: &items\n  - 1\n  - 2\ncopy: *items\n";
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Doc {
            original: Vec<i32>,
            copy: Vec<i32>,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.original, vec![1, 2]);
        assert_eq!(doc.copy, vec![1, 2]);
    }

    #[test]
    fn mapping_anchor_alias() {
        let yaml = r"
base: &base
  host: localhost
  port: 8080
dev: *base
";
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Server {
            host: String,
            port: u16,
        }
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Doc {
            base: Server,
            dev: Server,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.base.host, "localhost");
        assert_eq!(doc.dev.port, 8080);
    }

    #[test]
    fn multiple_aliases_same_anchor() {
        let yaml = "x: &v 42\ny: *v\nz: *v\n";
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            x: i32,
            y: i32,
            z: i32,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.x, 42);
        assert_eq!(doc.y, 42);
        assert_eq!(doc.z, 42);
    }

    #[test]
    fn nested_sequence_alias() {
        let yaml = "items: &list\n  - a\n  - b\nother: *list\n";
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            items: Vec<String>,
            other: Vec<String>,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.items, vec!["a", "b"]);
        assert_eq!(doc.other, vec!["a", "b"]);
    }

    #[test]
    fn bool_anchor_alias() {
        let yaml = "a: &flag true\nb: *flag\n";
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            a: bool,
            b: bool,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert!(doc.a);
        assert!(doc.b);
    }

    #[test]
    fn float_anchor_alias() {
        let yaml = "a: &pi 3.125\nb: *pi\n";
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            a: f64,
            b: f64,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert!((doc.a - 3.125_f64).abs() < 1e-10);
        assert!((doc.b - 3.125_f64).abs() < 1e-10);
    }

    #[test]
    fn optional_with_anchor() {
        let yaml = "a: &v hello\nb: *v\nc: null\n";
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            a: Option<String>,
            b: Option<String>,
            c: Option<String>,
        }
        let doc: Doc = noyalib::from_str(yaml).unwrap();
        assert_eq!(doc.a.as_deref(), Some("hello"));
        assert_eq!(doc.b.as_deref(), Some("hello"));
        assert!(doc.c.is_none());
    }
}

// ── Feature 2: miette diagnostic bridge ─────────────────────────────────

#[cfg(feature = "miette")]
mod diagnostic_bridge {

    use noyalib::Spanned;

    #[test]
    fn spanned_error_report() {
        let yaml = "port: 80\n";
        #[derive(serde::Deserialize)]
        struct Cfg {
            port: Spanned<u16>,
        }
        let cfg: Cfg = noyalib::from_str(yaml).unwrap();
        assert_eq!(cfg.port.value, 80);

        let report = noyalib::diagnostic::spanned_error(yaml, &cfg.port, "port < 1024");
        let msg = format!("{report}");
        assert!(msg.contains("port < 1024"));
    }

    #[test]
    fn spanned_error_has_span_info() {
        use miette::Diagnostic;

        let yaml = "name: test\nvalue: 42\n";
        #[derive(serde::Deserialize)]
        struct Cfg {
            #[allow(dead_code)]
            name: String,
            value: Spanned<i32>,
        }
        let cfg: Cfg = noyalib::from_str(yaml).unwrap();
        let report = noyalib::diagnostic::spanned_error(yaml, &cfg.value, "bad value");
        let diag: &dyn Diagnostic = report.as_ref();
        assert!(diag.labels().is_some());
        assert!(diag.source_code().is_some());
    }
}

// ── Feature 3: Shared-Memory DAGs (Rc/Arc Registry) ────────────────────

mod anchor_registry {
    use std::rc::Rc;
    use std::sync::Arc;

    use noyalib::{AnchorRegistry, ArcAnchorRegistry};

    #[test]
    fn rc_register_and_resolve() {
        let mut reg = AnchorRegistry::<String>::new();
        let rc = reg.register("anchor1".into(), "hello".into());
        let resolved = reg.resolve("anchor1").unwrap();
        assert!(Rc::ptr_eq(&rc, &resolved));
        assert_eq!(*resolved, "hello");
    }

    #[test]
    fn rc_resolve_unknown_returns_none() {
        let reg = AnchorRegistry::<String>::new();
        assert!(reg.resolve("missing").is_none());
    }

    #[test]
    fn rc_len_and_clear() {
        let mut reg = AnchorRegistry::<i32>::new();
        assert!(reg.is_empty());
        let _ = reg.register("a".into(), 1);
        let _ = reg.register("b".into(), 2);
        assert_eq!(reg.len(), 2);
        reg.clear();
        assert!(reg.is_empty());
    }

    #[test]
    fn rc_overwrite_anchor() {
        let mut reg = AnchorRegistry::<String>::new();
        let _ = reg.register("key".into(), "old".into());
        let new_rc = reg.register("key".into(), "new".into());
        let resolved = reg.resolve("key").unwrap();
        assert!(Rc::ptr_eq(&new_rc, &resolved));
        assert_eq!(*resolved, "new");
    }

    #[test]
    fn arc_register_and_resolve() {
        let mut reg = ArcAnchorRegistry::<String>::new();
        let arc = reg.register("anchor1".into(), "hello".into());
        let resolved = reg.resolve("anchor1").unwrap();
        assert!(Arc::ptr_eq(&arc, &resolved));
        assert_eq!(*resolved, "hello");
    }

    #[test]
    fn arc_resolve_unknown_returns_none() {
        let reg = ArcAnchorRegistry::<String>::new();
        assert!(reg.resolve("missing").is_none());
    }

    #[test]
    fn arc_len_and_clear() {
        let mut reg = ArcAnchorRegistry::<i32>::new();
        assert!(reg.is_empty());
        let _ = reg.register("a".into(), 1);
        let _ = reg.register("b".into(), 2);
        assert_eq!(reg.len(), 2);
        reg.clear();
        assert!(reg.is_empty());
    }

    #[test]
    fn arc_thread_safety() {
        let mut reg = ArcAnchorRegistry::<String>::new();
        let arc = reg.register("shared".into(), "data".into());
        let arc2 = Arc::clone(&arc);
        let handle = std::thread::spawn(move || {
            assert_eq!(*arc2, "data");
        });
        handle.join().unwrap();
        assert_eq!(*arc, "data");
    }

    #[test]
    fn rc_default() {
        let reg = AnchorRegistry::<String>::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn arc_default() {
        let reg = ArcAnchorRegistry::<String>::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn rc_debug() {
        let reg = AnchorRegistry::<String>::new();
        let debug = format!("{reg:?}");
        assert!(debug.contains("AnchorRegistry"));
    }

    #[test]
    fn arc_debug() {
        let reg = ArcAnchorRegistry::<String>::new();
        let debug = format!("{reg:?}");
        assert!(debug.contains("ArcAnchorRegistry"));
    }
}

// ── Feature 4: Robotics/Scientific Numeric Profile ──────────────────────

#[cfg(feature = "robotics")]
// Pins the deprecated `robotics` compat surface for its final release
// (aliases into `lossless_float`, leaving `Degrees`/`Radians`).
#[allow(deprecated)]
mod robotics_types {
    use noyalib::robotics::{Degrees, Radians, StrictFloat};

    #[test]
    fn strict_float_roundtrip() {
        let sf: StrictFloat = noyalib::from_str("1.5").unwrap();
        assert!((sf.get() - 1.5).abs() < 1e-15);
        let yaml = noyalib::to_string(&sf).unwrap();
        let rt: StrictFloat = noyalib::from_str(&yaml).unwrap();
        assert!((rt.get() - sf.get()).abs() < 1e-15);
    }

    #[test]
    fn strict_float_rejects_inf() {
        let result: Result<StrictFloat, _> = noyalib::from_str(".inf");
        assert!(result.is_err());
    }

    #[test]
    fn strict_float_rejects_nan() {
        let result: Result<StrictFloat, _> = noyalib::from_str(".nan");
        assert!(result.is_err());
    }

    #[test]
    fn radians_from_180_degrees() {
        let r: Radians = noyalib::from_str("180.0").unwrap();
        assert!((r.0 - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn radians_from_90_degrees() {
        let r: Radians = noyalib::from_str("90.0").unwrap();
        assert!((r.0 - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
    }

    #[test]
    fn radians_to_degrees() {
        let r = Radians(std::f64::consts::PI);
        let d = r.to_degrees();
        assert!((d.0 - 180.0).abs() < 1e-10);
    }

    #[test]
    fn degrees_to_radians() {
        let d: Degrees = noyalib::from_str("360.0").unwrap();
        let r = d.to_radians();
        assert!((r.0 - std::f64::consts::TAU).abs() < 1e-10);
    }

    #[test]
    fn degrees_roundtrip() {
        let d: Degrees = noyalib::from_str("45.0").unwrap();
        let yaml = noyalib::to_string(&d).unwrap();
        let rt: Degrees = noyalib::from_str(&yaml).unwrap();
        assert!((rt.0 - 45.0).abs() < 1e-10);
    }

    #[test]
    fn strict_float_in_struct() {
        #[derive(Debug, serde::Deserialize)]
        struct Measurement {
            distance: StrictFloat,
            angle: Radians,
        }
        let yaml = "distance: 1.5\nangle: 90.0\n";
        let m: Measurement = noyalib::from_str(yaml).unwrap();
        assert!((m.distance.get() - 1.5).abs() < 1e-15);
        assert!((m.angle.0 - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
    }

    #[test]
    fn strict_float_negative_inf() {
        let result: Result<StrictFloat, _> = noyalib::from_str("-.inf");
        assert!(result.is_err());
    }

    #[test]
    fn strict_float_zero() {
        let sf: StrictFloat = noyalib::from_str("0.0").unwrap();
        assert!(sf.get().abs() < 1e-15);
    }

    #[test]
    fn strict_float_large_but_precise() {
        let sf: StrictFloat = noyalib::from_str("1e10").unwrap();
        assert!((sf.get() - 1e10).abs() < 1.0);
    }
}
