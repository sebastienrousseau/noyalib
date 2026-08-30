// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::set_value` with a value equal to the one already loaded
//! must leave the source byte-identical (issue #337).
//!
//! Re-rendering an equal value through the formatter rewrote spellings
//! the author chose -- `1.10` became `1.1`, `0x1F` became `31`, an
//! implicit null grew an explicit `null`, a `>-` folded scalar was
//! joined onto one line -- although the loaded document was unchanged.

#![allow(missing_docs)]

use noyalib::Value;
use noyalib::cst::parse_document;

const SRC: &str = concat!(
    "ver: 1.10\n",
    "hex: 0x1F\n",
    "oct: 0o17\n",
    "plus: +1\n",
    "exp: 1e3\n",
    "lz: 01\n",
    "empty:\n",
    "tilde: ~\n",
    "caps: Null\n",
    "country: \"NO\"\n",
    "sq: 'quoted'\n",
    "folded: >-\n  folded\n  text\n",
    "kept: >\n  folded\n\n  para\n",
    "lit: |\n  line\n",
    "nested:\n  deep: 2\n",
    "tags: [a, b]\n",
);

fn doc() -> noyalib::cst::Document {
    parse_document(SRC).unwrap()
}

#[test]
fn setting_every_scalar_to_its_loaded_value_is_byte_identical() {
    let d0 = doc();
    let root = d0.as_value().clone();
    let mut d = doc();
    for path in [
        "ver",
        "hex",
        "oct",
        "plus",
        "exp",
        "lz",
        "empty",
        "tilde",
        "caps",
        "country",
        "sq",
        "folded",
        "kept",
        "lit",
        "nested.deep",
        "tags[0]",
        "tags[1]",
    ] {
        let v = match path {
            "nested.deep" => root["nested"]["deep"].clone(),
            "tags[0]" => root["tags"][0].clone(),
            "tags[1]" => root["tags"][1].clone(),
            _ => root[path].clone(),
        };
        d.set_value(path, &v)
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(d.to_string(), SRC, "{path}: a no-op must not change bytes");
    }
}

#[test]
fn an_equal_value_built_by_the_caller_matches_too() {
    // The caller's `Value` need not come from this document -- equality
    // is on the loaded value, so `1.1_f64` matches the text `1.10`.
    let mut d = doc();
    d.set_value("ver", &Value::from(1.1_f64)).unwrap();
    d.set_value("hex", &Value::from(31_i64)).unwrap();
    d.set_value("exp", &Value::from(1000.0_f64)).unwrap();
    d.set_value("empty", &Value::Null).unwrap();
    d.set_value("country", &Value::String("NO".into())).unwrap();
    d.set_value("folded", &Value::String("folded text".into()))
        .unwrap();
    assert_eq!(d.to_string(), SRC);
}

#[test]
fn a_changed_value_still_writes() {
    let mut d = doc();
    d.set_value("ver", &Value::from(2.5_f64)).unwrap();
    assert!(d.to_string().contains("ver: 2.5\n"));
    d.set_value("empty", &Value::from(7_i64)).unwrap();
    assert!(d.to_string().contains("empty: 7\n"));
    assert_eq!(d.as_value()["ver"].as_f64(), Some(2.5));
}

#[test]
fn a_different_number_writes_and_the_equal_second_set_is_a_no_op() {
    // 1.5 is unequal to the loaded 1 under any feature set, so it
    // writes; the second set of the same value is then a no-op.
    // (1.0 over 1 is deliberately not pinned here: whether they are
    // equal follows `Number`'s `PartialEq` for the active features.)
    let mut d = parse_document("a: 1\n").unwrap();
    d.set_value("a", &Value::from(1.5_f64)).unwrap();
    assert_eq!(d.to_string(), "a: 1.5\n");
    d.set_value("a", &Value::from(1.5_f64)).unwrap();
    assert_eq!(d.to_string(), "a: 1.5\n");
}

#[test]
fn alias_refusals_and_path_errors_are_unchanged() {
    let mut d = parse_document("x: &a 1\ny: *a\n").unwrap();
    assert!(d.set_value("y", &Value::from(1_i64)).is_err());
    let mut d = doc();
    assert!(d.set_value("missing", &Value::Null).is_err());
}
