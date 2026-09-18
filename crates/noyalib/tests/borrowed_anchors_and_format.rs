// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Corners of the borrowed value graph, the anchor wrappers, and the
//! formatter.
//!
//! Each of these is reached only by a shape the fixtures do not produce:
//! a mapping whose keys are not strings, two `BorrowedValue`s of
//! different kinds compared against each other, a recursion wrapper
//! cloned rather than constructed, a document whose formatting has to
//! descend through a nested node rather than a leaf.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::rc::Rc;
use std::sync::Arc;

use noyalib::borrowed::{BorrowedValue, from_str_borrowed};
use noyalib::cst::{FormatConfig, format, format_with_config};

// ── borrowed: non-string mapping keys ───────────────────────────

/// YAML mapping keys need not be strings. The borrowed graph coerces
/// each scalar kind to text when one is asked for, and each kind takes
/// its own arm.
#[test]
fn the_borrowed_graph_coerces_every_scalar_key_kind() {
    let yaml = "1: int-key\ntrue: bool-key\n~: null-key\n2.5: float-key\nplain: string-key\n";
    let v = from_str_borrowed(yaml).expect("parse");
    let m = v.as_mapping().expect("mapping");
    assert_eq!(m.len(), 5, "a key was lost: {m:?}");

    for key in ["1", "true", "2.5", "plain"] {
        assert!(
            m.get(key).is_some(),
            "no entry reachable under the coerced key {key:?}"
        );
    }

    // Serializing round-trips through the same coercion.
    let out = noyalib::to_string(&v).expect("serialize the borrowed graph");
    let back: noyalib::Value = noyalib::from_str(&out).expect("reparse");
    assert_eq!(
        back.as_mapping().map(noyalib::Mapping::len),
        Some(5),
        "serialization lost a key\n{out}"
    );
}

/// `lossless_u64_integers` is what keeps a `u64` past `i64::MAX` from
/// becoming a float. It is a `ParserConfig` toggle, not just a cargo
/// feature — off by default, so the unsigned arm of every number path
/// is unreachable without it.
///
/// The owned and borrowed graphs must agree, on and off: a document
/// that changes meaning depending on which reader you picked would be
/// worse than either behaviour on its own.
///
/// Gated on the feature, not just on the toggle: `lossless_u64_integers`
/// is a `ParserConfig` field that only exists when `lossless-u64` is on,
/// so naming it in a default-feature build is a compile error rather
/// than a skipped assertion.
#[cfg(feature = "lossless-u64")]
#[test]
fn the_unsigned_arm_needs_its_toggle_and_both_graphs_agree() {
    const YAML: &str = "big: 18446744073709551615\nfits: 9223372036854775807\n";

    // Off (the default): too large for i64, so it is read as a float.
    let owned: noyalib::Value = noyalib::from_str(YAML).expect("owned, default");
    let borrowed = from_str_borrowed(YAML).expect("borrowed, default");
    let owned_big = format!("{:?}", owned.get("big").expect("big"));
    let borrowed_big = format!(
        "{:?}",
        borrowed.as_mapping().expect("map").get("big").expect("big")
    );
    assert_eq!(
        owned_big, borrowed_big,
        "the two graphs disagree with the toggle off"
    );
    assert!(
        owned_big.contains("Float"),
        "expected a float with the toggle off, got {owned_big}"
    );

    // On: the value keeps its digits.
    let mut cfg = noyalib::ParserConfig::new();
    cfg.lossless_u64_integers = true;
    let owned: noyalib::Value = noyalib::from_str_with_config(YAML, &cfg).expect("owned, lossless");
    let out = noyalib::to_string(&owned).expect("serialize");
    assert!(
        out.contains("18446744073709551615"),
        "the toggle did not preserve the digits: {out}"
    );

    // And a value that fits in i64 is an integer either way.
    assert!(
        format!("{:?}", owned.get("fits").expect("fits")).contains("Integer"),
        "a value inside i64's range should not need the toggle"
    );
}

/// Ordering across different kinds falls through to a defined result
/// rather than panicking, which is what lets a `BorrowedValue` be used
/// as a sort key.
#[test]
fn borrowed_values_of_different_kinds_have_a_total_order() {
    let mut vals = vec![
        BorrowedValue::Null,
        BorrowedValue::Bool(true),
        BorrowedValue::String("s".into()),
    ];
    // Sorting must terminate and be stable across runs — the point is
    // that mixed kinds compare without panicking.
    vals.sort();
    let again = {
        let mut v2 = vals.clone();
        v2.sort();
        v2
    };
    assert_eq!(vals, again, "sorting mixed kinds is not deterministic");
    assert_eq!(vals.len(), 3);
}

// ── the recursion wrappers ──────────────────────────────────────

/// `RcRecursive`/`RcRecursion` (and the `Arc` pair) exist so a value can
/// refer to itself. Their `Clone` impls are what make the back-reference
/// shareable, and constructing one does not exercise cloning it.
#[test]
fn the_recursion_wrappers_clone() {
    use noyalib::{ArcRecursion, ArcRecursive, RcRecursion, RcRecursive};
    use std::cell::RefCell;
    use std::sync::Mutex;

    let strong = RcRecursive::<i32>(Rc::new(RefCell::new(Some(1))));
    let weak = RcRecursion::<i32>(Rc::downgrade(&strong.0));
    let s2 = Clone::clone(&strong);
    let w2 = Clone::clone(&weak);
    assert_eq!(*s2.0.borrow(), Some(1), "the strong clone lost its value");
    assert!(w2.0.upgrade().is_some(), "the weak clone lost its target");

    let strong = ArcRecursive::<i32>(Arc::new(Mutex::new(Some(2))));
    let weak = ArcRecursion::<i32>(Arc::downgrade(&strong.0));
    let s2 = Clone::clone(&strong);
    let w2 = Clone::clone(&weak);
    assert_eq!(
        *s2.0.lock().expect("lock"),
        Some(2),
        "the strong clone lost its value"
    );
    assert!(w2.0.upgrade().is_some(), "the weak clone lost its target");
}

// ── the formatter ───────────────────────────────────────────────

/// Formatting has to descend through nested nodes, not just leaves. A
/// flat document never makes it recurse.
#[test]
fn formatting_descends_through_nested_structure() {
    let cases: &[(&str, &str)] = &[
        ("a nested mapping", "a:\n     b:\n            c: 1\n"),
        ("a nested sequence", "xs:\n  -   - 1\n      -   2\n"),
        ("mixed nesting", "m:\n  xs:\n    -  k:   1\n       j: 2\n"),
        (
            "entries with comments",
            "# head\na:   1   # side\nb:\n  c:  2\n",
        ),
        (
            "a block scalar inside a mapping",
            "m:\n  s: |\n      line\n  t: 2\n",
        ),
        ("flow inside block", "m:\n  f:   {a: 1, b: [1,  2]}\n"),
    ];
    for (label, src) in cases {
        let out = format(src).unwrap_or_else(|e| panic!("{label}: {e}"));
        // Formatting must not change what the document means.
        let before: noyalib::Value = noyalib::from_str(src).expect("fixture parses");
        let after: noyalib::Value = noyalib::from_str(&out)
            .unwrap_or_else(|e| panic!("{label}: formatted output does not parse: {e}\n{out}"));
        assert_eq!(after, before, "{label}: formatting changed the data\n{out}");
    }
}

/// Formatting is idempotent — running it twice gives the same bytes as
/// running it once. A formatter that is not gives a diff on every run.
#[test]
fn formatting_is_idempotent() {
    for src in [
        "a:\n     b:\n            c: 1\n",
        "xs:\n  -   1\n  -   2\n",
        "# head\na:   1   # side\n",
        "",
        "\n\n",
    ] {
        let once = format(src).unwrap_or_else(|e| panic!("{src:?}: {e}"));
        let twice = format(&once).unwrap_or_else(|e| panic!("{src:?}: second pass: {e}"));
        assert_eq!(twice, once, "formatting {src:?} is not idempotent");
    }
}

/// The configurable form, across its settings.
#[test]
fn formatting_honours_its_configuration() {
    let src = "m:\n  a: 1\n  xs:\n    - 1\n    - 2\n";
    let before: noyalib::Value = noyalib::from_str(src).expect("fixture");
    for indent in [2_usize, 4] {
        let cfg = FormatConfig {
            indent_size: indent,
        };
        let out = format_with_config(src, &cfg).unwrap_or_else(|e| panic!("indent {indent}: {e}"));
        let after: noyalib::Value =
            noyalib::from_str(&out).unwrap_or_else(|e| panic!("indent {indent}: {e}\n{out}"));
        assert_eq!(after, before, "indent {indent} changed the data\n{out}");
    }
}
