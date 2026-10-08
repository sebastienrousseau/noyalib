// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! An anchor or tag on an empty sequence item does not swallow the
//! next item. A sequence nested under an item must be indented past
//! the item's `-`; only a mapping value may start its block sequence
//! at its key's column (YAML 1.2.2 §8.2.1, seq-spaces). Found by
//! fuzz_diff (input `-\n &z \n-`): noyalib read `[[null]]` where the
//! spec and libyaml read `[null, null]`.

use noyalib::{Value, from_str};

#[test]
fn properties_on_an_empty_item_leave_the_sibling_alone() {
    for doc in ["- &z\n-\n", "-\n &z \n-\n", "- &z\n- x\n"] {
        let v: Value = from_str(doc).unwrap_or_else(|e| panic!("{doc:?}: {e}"));
        let seq = v.as_sequence().unwrap_or_else(|| panic!("{doc:?}: {v:?}"));
        assert_eq!(seq.len(), 2, "{doc:?}: {v:?}");
        assert!(seq[0].is_null(), "{doc:?}: {v:?}");
        // The same reading on the typed, borrowed and CST paths.
        let typed: Vec<Option<String>> = from_str(doc).unwrap();
        assert_eq!(typed.len(), 2, "typed {doc:?}");
        let borrowed = noyalib::borrowed::from_str_borrowed(doc).unwrap();
        assert_eq!(
            borrowed.as_sequence().map(|s| s.len()),
            Some(2),
            "borrowed {doc:?}"
        );
        let cst = noyalib::cst::parse_document(doc).unwrap();
        assert_eq!(*cst.as_value(), v, "cst {doc:?}");
    }
}

#[test]
fn properties_still_reach_a_nested_or_value_sequence() {
    // Indented past the item: a nested sequence.
    let v: Value = from_str("- &z\n  - x\n").unwrap();
    assert_eq!(v[0][0].as_str(), Some("x"));
    // A mapping value may start its sequence at the key's column.
    let v: Value = from_str("k: &z\n- x\n").unwrap();
    assert_eq!(v["k"][0].as_str(), Some("x"));
    // A root node with properties, then the sequence.
    let v: Value = from_str("&z\n- x\n").unwrap();
    assert_eq!(v[0].as_str(), Some("x"));
}
