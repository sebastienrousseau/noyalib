//! Coverage tests for Spanned<T> and anchor types (RcAnchor, ArcAnchor,
//! RcWeakAnchor, ArcWeakAnchor).

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::rc::Rc;
use std::sync::Arc;

use noyalib::{
    ArcAnchor, ArcWeakAnchor, Location, RcAnchor, RcWeakAnchor, Spanned, Value, from_str,
    from_value, to_string,
};

// ============================================================================
// Spanned<T> — construction
// ============================================================================

#[test]
fn spanned_new_default_locations() {
    let s = Spanned::new(42);
    assert_eq!(s.value, 42);
    assert_eq!(s.start, Location::default());
    assert_eq!(s.end, Location::default());
}

#[test]
fn spanned_from() {
    let s: Spanned<String> = Spanned::from("hello".to_string());
    assert_eq!(s.value, "hello");
}

#[test]
fn spanned_into_inner() {
    let s = Spanned::new(vec![1, 2, 3]);
    let inner = s.into_inner();
    assert_eq!(inner, vec![1, 2, 3]);
}

// ============================================================================
// Spanned<T> — traits
// ============================================================================

#[test]
fn spanned_deref() {
    let s = Spanned::new("hello".to_string());
    assert_eq!(s.len(), 5); // Deref to String
}

#[test]
fn spanned_clone() {
    let s = Spanned::new(42);
    let s2 = s.clone();
    assert_eq!(s, s2);
}

#[test]
fn spanned_eq() {
    let a = Spanned::new(42);
    let b = Spanned::new(42);
    assert_eq!(a, b);

    let c = Spanned::new(99);
    assert_ne!(a, c);
}

#[test]
fn spanned_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let _ = set.insert(Spanned::new(42));
    let _ = set.insert(Spanned::new(42));
    assert_eq!(set.len(), 1);
}

#[test]
fn spanned_debug() {
    let s = Spanned::new(42);
    let debug = format!("{s:?}");
    assert!(debug.contains("Spanned"));
    assert!(debug.contains("42"));
    assert!(debug.contains("start"));
    assert!(debug.contains("end"));
}

// ============================================================================
// Spanned<T> — serde
// ============================================================================

#[test]
fn spanned_serialize_transparent() {
    let s = Spanned::new(42i64);
    let yaml = to_string(&s).unwrap();
    assert!(yaml.contains("42"));
    // Should NOT contain any Spanned wrapper in output
    assert!(!yaml.contains("Spanned"));
}

#[test]
fn spanned_deserialize() {
    let s: Spanned<i64> = from_str("42").unwrap();
    assert_eq!(s.value, 42);
    // Real locations: "42" starts at byte 0, line 1, col 1
    assert_eq!(s.start.line(), 1);
    assert_eq!(s.start.column(), 1);
    assert_eq!(s.start.index(), 0);
    assert!(s.end.index() > 0);
}

#[test]
fn spanned_roundtrip_string() {
    let s = Spanned::new("hello".to_string());
    let yaml = to_string(&s).unwrap();
    let parsed: Spanned<String> = from_str(&yaml).unwrap();
    assert_eq!(parsed.value, "hello");
}

#[test]
fn spanned_in_struct() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        port: Spanned<u16>,
        name: Spanned<String>,
    }

    let yaml = "port: 8080\nname: test\n";
    let parsed: Config = from_str(yaml).unwrap();
    assert_eq!(parsed.port.value, 8080);
    assert_eq!(parsed.name.value, "test");
    // port value starts at "8080" (after "port: ")
    assert_eq!(parsed.port.start.line(), 1);
    assert!(parsed.port.start.index() > 0);
    // name value starts at "test" on line 2
    assert_eq!(parsed.name.start.line(), 2);
}

#[test]
fn spanned_with_complex_inner() {
    let v = Spanned::new(vec![1i64, 2, 3]);
    let yaml = to_string(&v).unwrap();
    let parsed: Spanned<Vec<i64>> = from_str(&yaml).unwrap();
    assert_eq!(parsed.value, vec![1, 2, 3]);
}

// ============================================================================
// Spanned<T> — real locations
// ============================================================================

#[test]
fn spanned_nested_vec() {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Doc {
        items: Spanned<Vec<Spanned<String>>>,
    }

    let yaml = "items:\n  - hello\n  - world\n";
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(doc.items.value.len(), 2);
    assert_eq!(doc.items.value[0].value, "hello");
    assert_eq!(doc.items.value[1].value, "world");
    // The sequence itself has a span
    assert!(doc.items.start.line() >= 1);
    // Each element has its own span
    assert!(doc.items.value[0].start.line() >= 2);
    assert!(doc.items.value[1].start.line() >= 3);
}

#[test]
fn spanned_from_value_fallback() {
    // from_value has no span context, so locations should be zero
    let value: Value = from_str("42").unwrap();
    let s: Spanned<i64> = from_value(&value).unwrap();
    assert_eq!(s.value, 42);
    assert_eq!(s.start, Location::default());
    assert_eq!(s.end, Location::default());
}

#[test]
fn spanned_multiline_document() {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Doc {
        first: Spanned<String>,
        second: Spanned<i64>,
        third: Spanned<bool>,
    }

    let yaml = "first: hello\nsecond: 42\nthird: true\n";
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(doc.first.value, "hello");
    assert_eq!(doc.first.start.line(), 1);
    assert_eq!(doc.second.value, 42);
    assert_eq!(doc.second.start.line(), 2);
    assert!(doc.third.value);
    assert_eq!(doc.third.start.line(), 3);
}

#[test]
fn spanned_sequence_element_spans() {
    let yaml = "- 10\n- 20\n- 30\n";
    let items: Vec<Spanned<i64>> = from_str(yaml).unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].value, 10);
    assert_eq!(items[1].value, 20);
    assert_eq!(items[2].value, 30);
    // Each element should be on its own line
    assert_eq!(items[0].start.line(), 1);
    assert_eq!(items[1].start.line(), 2);
    assert_eq!(items[2].start.line(), 3);
}

// ============================================================================
// RcAnchor<T>
// ============================================================================

#[test]
fn rc_anchor_from_value() {
    let a = RcAnchor::from("hello".to_string());
    assert_eq!(&*a, "hello");
}

#[test]
fn rc_anchor_from_rc() {
    let rc = Rc::new(42i32);
    let a: RcAnchor<i32> = RcAnchor::from(rc);
    assert_eq!(*a, 42);
}

#[test]
fn rc_anchor_into_inner() {
    let a = RcAnchor::from(42);
    let rc: Rc<i32> = a.into_inner();
    assert_eq!(*rc, 42);
}

#[test]
fn rc_anchor_deref() {
    let a = RcAnchor::from("hello".to_string());
    assert_eq!(a.len(), 5); // Deref to String
}

#[test]
fn rc_anchor_clone() {
    let a = RcAnchor::from(42);
    let b = a.clone();
    assert_eq!(*a, *b);
    // Both point to the same Rc
    assert!(Rc::ptr_eq(&a.0, &b.0));
}

#[test]
fn rc_anchor_debug() {
    let a = RcAnchor::from(42);
    let debug = format!("{a:?}");
    assert!(debug.contains("RcAnchor"));
    assert!(debug.contains("42"));
}

#[test]
fn rc_anchor_serialize() {
    let a = RcAnchor::from("hello".to_string());
    let yaml = to_string(&a).unwrap();
    assert!(yaml.contains("hello"));
}

#[test]
fn rc_anchor_deserialize() {
    let a: RcAnchor<String> = from_str("hello").unwrap();
    assert_eq!(&*a, "hello");
}

#[test]
fn rc_anchor_roundtrip() {
    let a = RcAnchor::from(42i64);
    let yaml = to_string(&a).unwrap();
    let parsed: RcAnchor<i64> = from_str(&yaml).unwrap();
    assert_eq!(*parsed, 42);
}

// ============================================================================
// ArcAnchor<T>
// ============================================================================

#[test]
fn arc_anchor_from_value() {
    let a = ArcAnchor::from("hello".to_string());
    assert_eq!(&*a, "hello");
}

#[test]
fn arc_anchor_from_arc() {
    let arc = Arc::new(42i32);
    let a: ArcAnchor<i32> = ArcAnchor::from(arc);
    assert_eq!(*a, 42);
}

#[test]
fn arc_anchor_into_inner() {
    let a = ArcAnchor::from(42);
    let arc: Arc<i32> = a.into_inner();
    assert_eq!(*arc, 42);
}

#[test]
fn arc_anchor_deref() {
    let a = ArcAnchor::from("hello".to_string());
    assert_eq!(a.len(), 5);
}

#[test]
fn arc_anchor_clone() {
    let a = ArcAnchor::from(42);
    let b = a.clone();
    assert_eq!(*a, *b);
    assert!(Arc::ptr_eq(&a.0, &b.0));
}

#[test]
fn arc_anchor_debug() {
    let a = ArcAnchor::from(42);
    let debug = format!("{a:?}");
    assert!(debug.contains("ArcAnchor"));
    assert!(debug.contains("42"));
}

#[test]
fn arc_anchor_serialize() {
    let a = ArcAnchor::from(42i64);
    let yaml = to_string(&a).unwrap();
    assert!(yaml.contains("42"));
}

#[test]
fn arc_anchor_deserialize() {
    let a: ArcAnchor<i64> = from_str("42").unwrap();
    assert_eq!(*a, 42);
}

#[test]
fn arc_anchor_roundtrip() {
    let a = ArcAnchor::from(vec![1i64, 2, 3]);
    let yaml = to_string(&a).unwrap();
    let parsed: ArcAnchor<Vec<i64>> = from_str(&yaml).unwrap();
    assert_eq!(&*parsed, &vec![1, 2, 3]);
}

// ============================================================================
// RcWeakAnchor<T>
// ============================================================================

#[test]
fn rc_weak_anchor_dangling() {
    let w = RcWeakAnchor::<i32>::dangling();
    assert!(w.upgrade().is_none());
}

#[test]
fn rc_weak_anchor_from_weak() {
    let rc = Rc::new(42);
    let weak = Rc::downgrade(&rc);
    let w = RcWeakAnchor::from(weak);
    assert_eq!(*w.upgrade().unwrap(), 42);
}

#[test]
fn rc_weak_anchor_into_inner() {
    let rc = Rc::new(42);
    let w = RcWeakAnchor::from(Rc::downgrade(&rc));
    let weak = w.into_inner();
    assert_eq!(*weak.upgrade().unwrap(), 42);
}

#[test]
fn rc_weak_anchor_clone() {
    let rc = Rc::new(42);
    let w = RcWeakAnchor::from(Rc::downgrade(&rc));
    let w2 = w.clone();
    assert_eq!(*w.upgrade().unwrap(), *w2.upgrade().unwrap());
}

#[test]
fn rc_weak_anchor_debug_live() {
    let rc = Rc::new(42);
    let w = RcWeakAnchor::from(Rc::downgrade(&rc));
    let debug = format!("{w:?}");
    assert!(debug.contains("RcWeakAnchor"));
    assert!(debug.contains("42"));
}

#[test]
fn rc_weak_anchor_debug_dangling() {
    let w = RcWeakAnchor::<i32>::dangling();
    let debug = format!("{w:?}");
    assert!(debug.contains("dangling"));
}

#[test]
fn rc_weak_anchor_serialize_dangling() {
    let w = RcWeakAnchor::<i32>::dangling();
    let yaml = to_string(&w).unwrap();
    assert!(yaml.contains("null") || yaml.contains("~"));
}

#[test]
fn rc_weak_anchor_serialize_live() {
    let rc = Rc::new(42);
    let w = RcWeakAnchor::from(Rc::downgrade(&rc));
    let yaml = to_string(&w).unwrap();
    assert!(yaml.contains("42"));
}

#[test]
fn rc_weak_anchor_deserialize_always_dangling() {
    let w: RcWeakAnchor<i32> = from_str("42").unwrap();
    assert!(w.upgrade().is_none()); // Always dangling after deser
}

#[test]
fn rc_weak_anchor_deserialize_null() {
    let w: RcWeakAnchor<i32> = from_str("~").unwrap();
    assert!(w.upgrade().is_none());
}

// ============================================================================
// ArcWeakAnchor<T>
// ============================================================================

#[test]
fn arc_weak_anchor_dangling() {
    let w = ArcWeakAnchor::<i32>::dangling();
    assert!(w.upgrade().is_none());
}

#[test]
fn arc_weak_anchor_from_weak() {
    let arc = Arc::new(42);
    let weak = Arc::downgrade(&arc);
    let w = ArcWeakAnchor::from(weak);
    assert_eq!(*w.upgrade().unwrap(), 42);
}

#[test]
fn arc_weak_anchor_into_inner() {
    let arc = Arc::new(42);
    let w = ArcWeakAnchor::from(Arc::downgrade(&arc));
    let weak = w.into_inner();
    assert_eq!(*weak.upgrade().unwrap(), 42);
}

#[test]
fn arc_weak_anchor_clone() {
    let arc = Arc::new(42);
    let w = ArcWeakAnchor::from(Arc::downgrade(&arc));
    let w2 = w.clone();
    assert_eq!(*w.upgrade().unwrap(), *w2.upgrade().unwrap());
}

#[test]
fn arc_weak_anchor_debug_live() {
    let arc = Arc::new(42);
    let w = ArcWeakAnchor::from(Arc::downgrade(&arc));
    let debug = format!("{w:?}");
    assert!(debug.contains("ArcWeakAnchor"));
    assert!(debug.contains("42"));
}

#[test]
fn arc_weak_anchor_debug_dangling() {
    let w = ArcWeakAnchor::<i32>::dangling();
    let debug = format!("{w:?}");
    assert!(debug.contains("dangling"));
}

#[test]
fn arc_weak_anchor_serialize_dangling() {
    let w = ArcWeakAnchor::<i32>::dangling();
    let yaml = to_string(&w).unwrap();
    assert!(yaml.contains("null") || yaml.contains("~"));
}

#[test]
fn arc_weak_anchor_serialize_live() {
    let arc = Arc::new("hello".to_string());
    let w = ArcWeakAnchor::from(Arc::downgrade(&arc));
    let yaml = to_string(&w).unwrap();
    assert!(yaml.contains("hello"));
}

#[test]
fn arc_weak_anchor_deserialize_always_dangling() {
    let w: ArcWeakAnchor<String> = from_str("hello").unwrap();
    assert!(w.upgrade().is_none());
}

#[test]
fn arc_weak_anchor_deserialize_null() {
    let w: ArcWeakAnchor<String> = from_str("~").unwrap();
    assert!(w.upgrade().is_none());
}
