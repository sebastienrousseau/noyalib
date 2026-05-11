//! Smart pointer anchor type tests.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::sync::Arc;

use noyalib::{ArcAnchor, ArcWeakAnchor, RcAnchor, RcWeakAnchor, from_str, to_string};

#[test]
fn test_rc_anchor_serialize() {
    let anchor = RcAnchor::from("hello".to_string());
    let yaml = to_string(&anchor).unwrap();
    assert_eq!(yaml.trim(), "hello");
}

#[test]
fn test_arc_anchor_serialize() {
    let anchor = ArcAnchor::from(42i64);
    let yaml = to_string(&anchor).unwrap();
    assert_eq!(yaml.trim(), "42");
}

#[test]
fn test_weak_anchor_dangling() {
    let weak: RcWeakAnchor<String> = RcWeakAnchor::dangling();
    let yaml = to_string(&weak).unwrap();
    assert_eq!(yaml.trim(), "null");
}

#[test]
fn test_arc_weak_anchor_dangling() {
    let weak: ArcWeakAnchor<String> = ArcWeakAnchor::dangling();
    let yaml = to_string(&weak).unwrap();
    assert_eq!(yaml.trim(), "null");
}

#[test]
fn test_weak_anchor_with_strong() {
    let strong = std::rc::Rc::new("alive".to_string());
    let weak = RcWeakAnchor(std::rc::Rc::downgrade(&strong));
    let yaml = to_string(&weak).unwrap();
    assert_eq!(yaml.trim(), "alive");
    drop(strong);
    // After dropping strong, weak is dangling
    let yaml2 = to_string(&weak).unwrap();
    assert_eq!(yaml2.trim(), "null");
}

#[test]
fn test_arc_weak_anchor_with_strong() {
    let strong = Arc::new(100i64);
    let weak = ArcWeakAnchor(Arc::downgrade(&strong));
    let yaml = to_string(&weak).unwrap();
    assert_eq!(yaml.trim(), "100");
    drop(strong);
    let yaml2 = to_string(&weak).unwrap();
    assert_eq!(yaml2.trim(), "null");
}

#[test]
fn test_anchor_deserialize() {
    let rc: RcAnchor<String> = from_str("hello").unwrap();
    assert_eq!(*rc.0, "hello");

    let arc: ArcAnchor<i64> = from_str("42").unwrap();
    assert_eq!(*arc.0, 42);
}

#[test]
fn test_rc_anchor_deref() {
    let anchor = RcAnchor::from("test".to_string());
    assert_eq!(anchor.len(), 4); // Deref to String, then str
}

#[test]
fn test_arc_anchor_deref() {
    let anchor = ArcAnchor::from(vec![1, 2, 3]);
    assert_eq!(anchor.len(), 3); // Deref to Vec
}
