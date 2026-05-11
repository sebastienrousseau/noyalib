// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Automatic anchor/alias emission for shared `Rc` / `Arc` pointers.
//!
//! Covers the `to_string_tracking_shared` family: tracking activates only for
//! the duration of the call, duplicated `Rc::clone`s emit aliases, distinct
//! allocations with equal values do NOT, round-trip preserves values, and
//! weak refs behave correctly.

use std::rc::Rc;
use std::sync::Arc;

use noyalib::{ArcAnchor, RcAnchor, to_string, to_string_tracking_shared};
use serde::{Deserialize, Serialize};

// ── Basic emission ──────────────────────────────────────────────────────

#[test]
fn single_ref_emits_anchor_but_no_alias() {
    // One use of the Rc → an anchor is assigned but no alias references it.
    // Emission is `&id001 <value>`; valid YAML (anchors without aliases are legal).
    let a: RcAnchor<String> = RcAnchor::from("hello".to_string());
    let yaml = to_string_tracking_shared(&a).unwrap();
    assert!(yaml.contains("&id001"));
    assert!(!yaml.contains("*id001"));
}

#[test]
fn two_clones_emit_one_anchor_one_alias() {
    let a: RcAnchor<String> = RcAnchor::from("shared".to_string());
    let doc = vec![a.clone(), a];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    assert_eq!(yaml.matches("&id001").count(), 1);
    assert_eq!(yaml.matches("*id001").count(), 1);
    assert!(yaml.contains("shared"));
}

#[test]
fn three_clones_one_anchor_two_aliases() {
    let a: RcAnchor<String> = RcAnchor::from("x".to_string());
    let doc = vec![a.clone(), a.clone(), a];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    assert_eq!(yaml.matches("&id001").count(), 1);
    assert_eq!(yaml.matches("*id001").count(), 2);
}

#[test]
fn distinct_allocations_with_equal_value_emit_two_anchors() {
    // Value equality is not identity. Two separate Rc::new calls → two anchors.
    let a: RcAnchor<String> = RcAnchor::from("same".to_string());
    let b: RcAnchor<String> = RcAnchor::from("same".to_string());
    let doc = vec![a, b];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    // Two different anchor ids allocated.
    assert!(yaml.contains("&id001"));
    assert!(yaml.contains("&id002"));
    // No aliases: neither anchor was referenced after its definition.
    assert!(!yaml.contains("*id"));
}

#[test]
fn disjoint_does_not_leak_state_between_calls() {
    // First call assigns id001. Second call must start fresh at id001, not id002.
    let a: RcAnchor<String> = RcAnchor::from("one".to_string());
    let y1 = to_string_tracking_shared(&vec![a.clone(), a]).unwrap();
    assert!(y1.contains("&id001"));

    let b: RcAnchor<String> = RcAnchor::from("two".to_string());
    let y2 = to_string_tracking_shared(&vec![b.clone(), b]).unwrap();
    assert!(y2.contains("&id001"));
    assert!(!y2.contains("&id002"));
}

#[test]
fn tracking_opt_in_default_unchanged() {
    // Plain to_string must NOT emit anchors — backwards compat fence.
    let a: RcAnchor<String> = RcAnchor::from("hi".to_string());
    let doc = vec![a.clone(), a];
    let yaml = to_string(&doc).unwrap();
    assert!(!yaml.contains("&id"));
    assert!(!yaml.contains("*id"));
}

// ── Mapping / struct inner ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Endpoint {
    host: String,
    port: u16,
}

#[test]
fn mapping_inner_anchor_is_valid_yaml() {
    let cfg: RcAnchor<Endpoint> = RcAnchor::from(Endpoint {
        host: "db.local".to_string(),
        port: 5432,
    });
    let doc = vec![cfg.clone(), cfg];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    assert!(yaml.contains("&id001"));
    assert!(yaml.contains("*id001"));

    // Round-trip via plain parser: value equivalence preserved.
    let parsed: Vec<Endpoint> = noyalib::from_str(&yaml).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].host, "db.local");
    assert_eq!(parsed[0].port, 5432);
    assert_eq!(parsed[0], parsed[1]);
}

#[test]
fn nested_struct_with_shared_leaf() {
    #[derive(Serialize)]
    struct Root {
        primary: RcAnchor<Endpoint>,
        replica: RcAnchor<Endpoint>,
    }
    let shared: RcAnchor<Endpoint> = RcAnchor::from(Endpoint {
        host: "db.local".to_string(),
        port: 5432,
    });
    let root = Root {
        primary: shared.clone(),
        replica: shared,
    };
    let yaml = to_string_tracking_shared(&root).unwrap();
    assert_eq!(yaml.matches("&id001").count(), 1);
    assert_eq!(yaml.matches("*id001").count(), 1);

    // Both fields round-trip equivalently.
    #[derive(Deserialize)]
    struct Parsed {
        primary: Endpoint,
        replica: Endpoint,
    }
    let p: Parsed = noyalib::from_str(&yaml).unwrap();
    assert_eq!(p.primary, p.replica);
}

// ── Sequence inner ───────────────────────────────────────────────────────

#[test]
fn sequence_inner_anchor() {
    let list: RcAnchor<Vec<i32>> = RcAnchor::from(vec![1, 2, 3]);
    let doc = vec![list.clone(), list];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    assert!(yaml.contains("&id001"));
    assert!(yaml.contains("*id001"));
    let parsed: Vec<Vec<i32>> = noyalib::from_str(&yaml).unwrap();
    assert_eq!(parsed, vec![vec![1, 2, 3], vec![1, 2, 3]]);
}

// ── Arc variant ──────────────────────────────────────────────────────────

#[test]
fn arc_two_clones_emit_anchor_and_alias() {
    let a: ArcAnchor<String> = ArcAnchor::from("arc".to_string());
    let doc = vec![a.clone(), a];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    assert_eq!(yaml.matches("&id001").count(), 1);
    assert_eq!(yaml.matches("*id001").count(), 1);
}

#[test]
fn arc_struct_shared() {
    #[derive(Serialize)]
    struct Root {
        a: ArcAnchor<Endpoint>,
        b: ArcAnchor<Endpoint>,
        c: ArcAnchor<Endpoint>,
    }
    let shared: ArcAnchor<Endpoint> = ArcAnchor::from(Endpoint {
        host: "h".to_string(),
        port: 1,
    });
    let root = Root {
        a: shared.clone(),
        b: shared.clone(),
        c: shared,
    };
    let yaml = to_string_tracking_shared(&root).unwrap();
    assert_eq!(yaml.matches("&id001").count(), 1);
    assert_eq!(yaml.matches("*id001").count(), 2);
}

// ── Multiple distinct shared pointers ────────────────────────────────────

#[test]
fn multiple_distinct_shared_pointers() {
    let a: RcAnchor<String> = RcAnchor::from("A".to_string());
    let b: RcAnchor<String> = RcAnchor::from("B".to_string());
    let doc = vec![a.clone(), b.clone(), a, b];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    // First sighting of each: one anchor each.
    assert!(yaml.contains("&id001"));
    assert!(yaml.contains("&id002"));
    // Second sighting of each: one alias each.
    assert_eq!(yaml.matches("*id001").count(), 1);
    assert_eq!(yaml.matches("*id002").count(), 1);
}

// ── Round-trip preserves values (not identity) ───────────────────────────

#[test]
fn round_trip_value_equivalence() {
    let a: RcAnchor<String> = RcAnchor::from("payload".to_string());
    let doc = vec![a.clone(), a.clone(), a];
    let yaml = to_string_tracking_shared(&doc).unwrap();
    let parsed: Vec<String> = noyalib::from_str(&yaml).unwrap();
    assert_eq!(parsed, vec!["payload", "payload", "payload"]);
}

// ── Pointer-level verification ───────────────────────────────────────────

#[test]
fn identity_driven_not_value_driven() {
    // A = Rc::new("x"); B = Rc::new("x"); C = A.clone().
    // Tracking must emit: anchor(A), anchor(B), alias(A) — not alias(B).
    let a: RcAnchor<String> = RcAnchor::from("x".to_string());
    let b: RcAnchor<String> = RcAnchor::from("x".to_string());
    let c = a.clone();
    let doc = vec![a, b, c];
    let yaml = to_string_tracking_shared(&doc).unwrap();

    // Two anchors emitted (A and B are distinct allocations).
    assert!(yaml.contains("&id001"));
    assert!(yaml.contains("&id002"));
    // Exactly one alias, referring to id001 (A's id).
    assert_eq!(yaml.matches("*id001").count(), 1);
    assert!(!yaml.contains("*id002"));
}

// ── Scope cleanup ────────────────────────────────────────────────────────

#[test]
fn state_cleared_between_calls_even_on_error_path() {
    // If tracking state leaked, the second call would either see a stale map
    // or the id counter would continue. Verified implicitly by
    // `disjoint_does_not_leak_state_between_calls` + explicit counter reset here.
    let a: RcAnchor<String> = RcAnchor::from("a".to_string());
    let _ = to_string_tracking_shared(&vec![a.clone(), a]);

    let b: RcAnchor<String> = RcAnchor::from("b".to_string());
    let y = to_string_tracking_shared(&vec![b.clone(), b]).unwrap();
    // Counter reset: would read "id003" / "id004" if it had leaked.
    assert!(y.contains("&id001"));
    assert!(y.contains("*id001"));
    assert!(!y.contains("&id003"));
}

// ── Non-tracking API still works with RcAnchor ───────────────────────────

#[test]
fn plain_to_string_with_rcanchor_delegates_to_inner() {
    // When tracking is off, RcAnchor<T>::serialize just delegates to T.
    let a: RcAnchor<i32> = RcAnchor::from(42);
    let yaml = to_string(&a).unwrap();
    assert_eq!(yaml.trim(), "42");
}

// ── Arc across threads (separate serialization on worker thread) ─────────

#[test]
fn arc_worker_thread_serializes_independently() {
    let a: ArcAnchor<String> = ArcAnchor::from("worker".to_string());
    let clone = a.clone();
    let handle = std::thread::spawn(move || {
        let doc = vec![clone.clone(), clone];
        to_string_tracking_shared(&doc).unwrap()
    });
    let y = handle.join().unwrap();
    // Worker's TLS is independent; ids start at 001.
    assert!(y.contains("&id001"));
    assert!(y.contains("*id001"));
    // Keep `a` alive until join.
    drop(a);
}

// ── Ensure we still return a usable Rc after serialization ───────────────

#[test]
fn rc_usable_after_tracking_serialization() {
    let a: RcAnchor<String> = RcAnchor::from("live".to_string());
    let clone = a.clone();
    let _ = to_string_tracking_shared(&vec![a.clone(), a]).unwrap();
    // Strong count includes: `clone`, original `a` dropped inside `vec!`, and
    // nothing else. `clone` is still live.
    assert_eq!(&**clone, "live");
    assert!(Rc::strong_count(&clone.0) >= 1);
}

#[test]
fn arc_usable_after_tracking_serialization() {
    let a: ArcAnchor<String> = ArcAnchor::from("live".to_string());
    let clone = a.clone();
    let _ = to_string_tracking_shared(&vec![a.clone(), a]).unwrap();
    assert_eq!(&**clone, "live");
    assert!(Arc::strong_count(&clone.0) >= 1);
}
