// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! AnchorRegistry and ArcAnchorRegistry for shared-memory DAG structures.
//!
//! Demonstrates programmatic anchor management using `Rc` and `Arc`
//! registries that let multiple aliases point to the same allocation.
//!
//! Run: `cargo run --example registry`

#[path = "support.rs"]
mod support;

use std::rc::Rc;
use std::sync::Arc;

use noyalib::{AnchorRegistry, ArcAnchorRegistry};

fn main() {
    support::header("noyalib -- registry (AnchorRegistry / ArcAnchorRegistry)");

    // ── Create registry, register values, resolve by name ────────────
    support::task_with_output("Register and resolve values", || {
        let mut reg = AnchorRegistry::<String>::new();
        let rc = reg.register("greeting".into(), "hello world".into());
        let resolved = reg.resolve("greeting").unwrap();
        assert_eq!(*resolved, "hello world");
        assert!(Rc::ptr_eq(&rc, &resolved));
        vec![
            format!("registered: greeting = {:?}", *rc),
            format!("resolved:   greeting = {:?} (same Rc)", *resolved),
        ]
    });

    // ── Multiple resolves return cloned Rc pointing to same alloc ────
    support::task_with_output("Multiple resolves share one allocation (Rc)", || {
        let mut reg = AnchorRegistry::<i64>::new();
        let original = reg.register("count".into(), 42);
        let alias1 = reg.resolve("count").unwrap();
        let alias2 = reg.resolve("count").unwrap();
        let alias3 = reg.resolve("count").unwrap();
        assert!(Rc::ptr_eq(&original, &alias1));
        assert!(Rc::ptr_eq(&alias1, &alias2));
        assert!(Rc::ptr_eq(&alias2, &alias3));
        vec![
            format!("Rc strong count = {}", Rc::strong_count(&original)),
            "All 4 references point to the same heap allocation".to_string(),
        ]
    });

    // ── ArcAnchorRegistry for thread-safe sharing ────────────────────
    support::task_with_output("ArcAnchorRegistry for thread-safe sharing", || {
        let mut reg = ArcAnchorRegistry::<String>::new();
        let arc = reg.register("shared".into(), "cross-thread data".into());
        let alias = reg.resolve("shared").unwrap();
        assert!(Arc::ptr_eq(&arc, &alias));

        // Prove thread-safety: send the Arc to another thread.
        let handle = std::thread::spawn(move || {
            assert_eq!(*alias, "cross-thread data");
            format!("Thread received: {:?}", *alias)
        });
        let thread_result = handle.join().unwrap();
        vec![
            format!("main thread: {:?}", *arc),
            thread_result,
            "Arc::ptr_eq confirmed across threads".to_string(),
        ]
    });

    // ── Building a DAG structure using the registry ──────────────────
    support::task_with_output("DAG structure with shared nodes", || {
        #[derive(Debug, Clone)]
        #[allow(dead_code)]
        struct Node {
            name: String,
            children: Vec<Rc<Node>>,
        }

        let mut reg = AnchorRegistry::<Node>::new();

        // Leaf nodes
        let leaf_a = reg.register(
            "leaf_a".into(),
            Node {
                name: "A".into(),
                children: vec![],
            },
        );
        let _leaf_b = reg.register(
            "leaf_b".into(),
            Node {
                name: "B".into(),
                children: vec![],
            },
        );

        // Internal node referencing shared leaves
        let shared_a = reg.resolve("leaf_a").unwrap();
        let shared_b = reg.resolve("leaf_b").unwrap();
        let _parent = reg.register(
            "parent".into(),
            Node {
                name: "Parent".into(),
                children: vec![shared_a, shared_b],
            },
        );

        // Another node also referencing leaf_a (true DAG)
        let shared_a2 = reg.resolve("leaf_a").unwrap();
        let _sibling = reg.register(
            "sibling".into(),
            Node {
                name: "Sibling".into(),
                children: vec![shared_a2],
            },
        );

        // leaf_a is shared: original + parent's ref + sibling's ref + registry
        assert!(Rc::strong_count(&leaf_a) >= 3);
        vec![
            format!("DAG nodes registered: {}", reg.len()),
            format!(
                "leaf_a strong_count = {} (shared by parent + sibling)",
                Rc::strong_count(&leaf_a)
            ),
            "True DAG: no duplication of leaf nodes".to_string(),
        ]
    });

    // ── Clearing and reusing the registry ────────────────────────────
    support::task_with_output("Clear and reuse registry", || {
        let mut reg = AnchorRegistry::<String>::new();
        let _ = reg.register("a".into(), "alpha".into());
        let _ = reg.register("b".into(), "beta".into());
        let _ = reg.register("c".into(), "gamma".into());
        assert_eq!(reg.len(), 3);
        assert!(!reg.is_empty());

        reg.clear();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.resolve("a").is_none());

        // Reuse after clear
        let _ = reg.register("x".into(), "new value".into());
        assert_eq!(reg.len(), 1);
        assert!(reg.resolve("x").is_some());

        vec![
            "Registered 3 entries, cleared, then added 1 new".to_string(),
            format!("Final len = {}", reg.len()),
        ]
    });

    // ── Default constructor and is_empty ─────────────────────────────
    support::task("Default constructor starts empty", || {
        let reg = AnchorRegistry::<()>::default();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    });

    // ── Resolve unknown returns None ─────────────────────────────────
    support::task("Resolve unknown name returns None", || {
        let reg = AnchorRegistry::<String>::new();
        assert!(reg.resolve("nonexistent").is_none());
    });

    // ── Overwrite existing anchor ────────────────────────────────────
    support::task_with_output("Overwrite replaces existing entry", || {
        let mut reg = AnchorRegistry::<String>::new();
        let first = reg.register("key".into(), "first".into());
        let second = reg.register("key".into(), "second".into());
        let resolved = reg.resolve("key").unwrap();
        assert!(!Rc::ptr_eq(&first, &second));
        assert!(Rc::ptr_eq(&second, &resolved));
        assert_eq!(*resolved, "second");
        vec![
            format!("first  = {:?} (orphaned)", *first),
            format!("second = {:?} (current)", *resolved),
        ]
    });

    support::summary(8);
}
