// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Automatic YAML anchor/alias emission for shared `Rc` / `Arc` pointers.
//!
//! Demonstrates `to_string_tracking_shared`: identical `Rc::clone` /
//! `Arc::clone` siblings collapse into one anchor + N-1 aliases in the
//! emitted document, preserving DAG structure.
//!
//! Run: `cargo run --example anchor_shared`

#[path = "support.rs"]
mod support;

use noyalib::{ArcAnchor, RcAnchor, to_string, to_string_tracking_shared};
use serde::Serialize;

fn main() {
    support::header("noyalib -- anchor_shared (automatic Rc/Arc anchor emission)");

    // ── Two Rc clones collapse to one anchor + one alias ────────────
    support::task_with_output("Two Rc clones: &id001 + *id001", || {
        let a: RcAnchor<String> = RcAnchor::from("payload".to_string());
        let doc = vec![a.clone(), a];
        let yaml = to_string_tracking_shared(&doc).unwrap();
        assert_eq!(yaml.matches("&id001").count(), 1);
        assert_eq!(yaml.matches("*id001").count(), 1);
        yaml.lines().map(String::from).collect::<Vec<_>>()
    });

    // ── Three clones → 1 anchor + 2 aliases ─────────────────────────
    support::task_with_output("Three Rc clones: 1 anchor + 2 aliases", || {
        let a: RcAnchor<String> = RcAnchor::from("x".to_string());
        let doc = vec![a.clone(), a.clone(), a];
        let yaml = to_string_tracking_shared(&doc).unwrap();
        assert_eq!(yaml.matches("&id001").count(), 1);
        assert_eq!(yaml.matches("*id001").count(), 2);
        yaml.lines().map(String::from).collect::<Vec<_>>()
    });

    // ── Distinct allocations with equal value: two anchors, no alias ─
    support::task_with_output(
        "Identity, not value: distinct allocs = distinct anchors",
        || {
            let a: RcAnchor<String> = RcAnchor::from("same".to_string());
            let b: RcAnchor<String> = RcAnchor::from("same".to_string());
            let yaml = to_string_tracking_shared(&vec![a, b]).unwrap();
            assert!(yaml.contains("&id001"));
            assert!(yaml.contains("&id002"));
            assert!(!yaml.contains("*id"));
            yaml.lines().map(String::from).collect::<Vec<_>>()
        },
    );

    // ── Struct with shared mapping leaf ──────────────────────────────
    support::task_with_output("Struct field sharing a mapping inner", || {
        #[derive(Clone, Serialize)]
        struct Endpoint {
            host: String,
            port: u16,
        }
        #[derive(Serialize)]
        struct Topology {
            primary: RcAnchor<Endpoint>,
            replica: RcAnchor<Endpoint>,
        }
        let shared = RcAnchor::from(Endpoint {
            host: "db.local".to_string(),
            port: 5432,
        });
        let topo = Topology {
            primary: shared.clone(),
            replica: shared,
        };
        let yaml = to_string_tracking_shared(&topo).unwrap();
        assert_eq!(yaml.matches("&id001").count(), 1);
        assert_eq!(yaml.matches("*id001").count(), 1);
        yaml.lines().map(String::from).collect::<Vec<_>>()
    });

    // ── Arc across threads: worker serializes independently ──────────
    support::task_with_output("Arc: worker-thread tracking is independent", || {
        let a: ArcAnchor<String> = ArcAnchor::from("shared".to_string());
        let clone = a.clone();
        let y = std::thread::spawn(move || {
            let doc = vec![clone.clone(), clone];
            to_string_tracking_shared(&doc).unwrap()
        })
        .join()
        .unwrap();
        drop(a);
        assert!(y.contains("&id001"));
        assert!(y.contains("*id001"));
        y.lines().map(String::from).collect::<Vec<_>>()
    });

    // ── Opt-in: plain to_string is unchanged ─────────────────────────
    support::task_with_output("Opt-in: plain to_string emits NO anchors", || {
        let a: RcAnchor<String> = RcAnchor::from("no_anchors".to_string());
        let yaml = to_string(&vec![a.clone(), a]).unwrap();
        assert!(!yaml.contains("&id"));
        assert!(!yaml.contains("*id"));
        yaml.lines().map(String::from).collect::<Vec<_>>()
    });

    support::summary(6);
}
