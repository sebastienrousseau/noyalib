// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Round-trip edge cases — the YAML "gotchas" production users
//! hit daily.
//!
//! Themed thematically rather than against any specific
//! competitor: each scenario maps to a real-world failure mode
//! that surfaces in CI logs of `serde_yaml`-based codebases.

#![allow(missing_docs)]

use noyalib::{Sequence, Value, from_str, to_string};

// ── Empty containers at every depth ────────────────────────────────

#[test]
fn empty_root_mapping_round_trips() {
    let v: Value = from_str("{}").unwrap();
    assert!(matches!(v, Value::Mapping(ref m) if m.is_empty()));
    let out = to_string(&v).unwrap();
    let v2: Value = from_str(&out).unwrap();
    assert_eq!(v, v2);
}

#[test]
fn empty_root_sequence_round_trips() {
    let v: Value = from_str("[]").unwrap();
    assert!(matches!(v, Value::Sequence(ref s) if s.is_empty()));
    let out = to_string(&v).unwrap();
    let v2: Value = from_str(&out).unwrap();
    assert_eq!(v, v2);
}

#[test]
fn nested_empty_sequence_in_mapping() {
    let yaml = "items: []\nname: noyalib\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["items"], Value::Sequence(Sequence::new()));
    let out = to_string(&v).unwrap();
    let v2: Value = from_str(&out).unwrap();
    assert_eq!(v, v2);
}

#[test]
fn nested_empty_mapping_in_sequence() {
    let yaml = "- {}\n- name: a\n- {}\n";
    let v: Value = from_str(yaml).unwrap();
    if let Value::Sequence(ref s) = v {
        assert_eq!(s.len(), 3);
        assert!(matches!(s[0], Value::Mapping(ref m) if m.is_empty()));
        assert!(matches!(s[2], Value::Mapping(ref m) if m.is_empty()));
    } else {
        panic!("expected sequence, got {v:?}");
    }
    let out = to_string(&v).unwrap();
    let v2: Value = from_str(&out).unwrap();
    assert_eq!(v, v2);
}

#[test]
fn mapping_with_three_levels_of_empty_nesting() {
    // outer.middle.inner = {} — exercises the empty-container
    // emit path at depth 3.
    let yaml = "outer:\n  middle:\n    inner: {}\n";
    let v: Value = from_str(yaml).unwrap();
    let out = to_string(&v).unwrap();
    let v2: Value = from_str(&out).unwrap();
    assert_eq!(v, v2);
}

#[test]
fn sequence_with_only_empty_mappings() {
    let yaml = "[{}, {}, {}]";
    let v: Value = from_str(yaml).unwrap();
    if let Value::Sequence(ref s) = v {
        assert_eq!(s.len(), 3);
        for item in s {
            assert!(matches!(item, Value::Mapping(m) if m.is_empty()));
        }
    } else {
        panic!();
    }
}

// ── Tagged scalars: type override survives the round-trip ──────────

#[test]
fn tagged_str_overrides_integer_resolution() {
    // Without the tag, "8080" resolves as an integer. The
    // `!!str` tag forces it back to a string.
    let yaml = "port: !!str 8080\n";
    let v: Value = from_str(yaml).unwrap();
    // The resolved value at `port` is the string "8080", not the
    // integer 8080.
    match &v["port"] {
        Value::String(s) => assert_eq!(s, "8080"),
        other => panic!("expected !!str 8080 to resolve as string, got {other:?}"),
    }
}

#[test]
fn tagged_int_overrides_quoted_string() {
    // "8080" quoted is normally a string. The `!!int` tag
    // forces it back to integer.
    let yaml = "port: !!int \"8080\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["port"].as_i64(), Some(8080));
}

#[test]
fn unknown_tag_resolves_to_content_type_per_yaml_spec() {
    // YAML 1.2 §3.2.1.3: an unknown / user-defined tag with no
    // registered resolver falls through to the content type. So
    // `!Custom 42` parses as `String("42")` (the failsafe
    // schema's content type for an unrecognised scalar tag).
    //
    // Callers who need the tag preserved opt in via the
    // `TagRegistry` — register the tag and the parser returns
    // a `Value::Tagged(...)` with the original wrapper.
    let v: Value = from_str("!Custom 42").unwrap();
    match v {
        Value::String(s) => assert_eq!(s, "42"),
        Value::Tagged(boxed) => {
            // If a future build registers tags by default, this
            // path also documents the alternate shape.
            assert!(boxed.tag().as_str().contains("Custom"));
        }
        other => panic!("unexpected resolution: {other:?}"),
    }
}

#[test]
fn tagged_null_resolves_correctly() {
    // `!!null ~` and `!!null null` both resolve to Value::Null.
    let v: Value = from_str("a: !!null ~\nb: !!null null\nc: !!null \"\"\n").unwrap();
    assert_eq!(v["a"], Value::Null);
    assert_eq!(v["b"], Value::Null);
    assert_eq!(v["c"], Value::Null);
}

#[test]
fn tagged_bool_overrides_string_form() {
    let yaml = "active: !!bool \"true\"\nidle: !!bool \"false\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["active"].as_bool(), Some(true));
    assert_eq!(v["idle"].as_bool(), Some(false));
}

// ── Special strings that look like other types ────────────────────

#[test]
fn yaml_1_2_strict_keeps_norway_as_string() {
    // The "Norway problem" — YAML 1.1 resolved `NO` as `false`.
    // YAML 1.2 strict keeps it as a string.
    let v: Value = from_str("country: NO\nlocale: NL\n").unwrap();
    assert_eq!(v["country"].as_str(), Some("NO"));
    assert_eq!(v["locale"].as_str(), Some("NL"));
}

#[test]
fn quoted_numeric_string_does_not_become_integer() {
    let v: Value = from_str("zip: \"01234\"\n").unwrap();
    assert_eq!(v["zip"].as_str(), Some("01234"));
}

#[test]
fn explicit_null_forms_resolve_to_null() {
    let v: Value = from_str("a: ~\nb: null\nc: Null\nd: NULL\ne:\n").unwrap();
    for k in ['a', 'b', 'c', 'd', 'e'] {
        assert_eq!(v[k.to_string().as_str()], Value::Null, "key {k}");
    }
}

// ── Number edge cases that tend to break round-trip ───────────────

#[test]
fn negative_zero_round_trips_as_zero() {
    let v: Value = from_str("n: -0\n").unwrap();
    assert_eq!(v["n"].as_i64(), Some(0));
}

#[test]
fn scientific_notation_floats_round_trip() {
    let yaml = "small: 1.5e-10\nlarge: 6.022e23\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v["small"].as_f64().is_some());
    assert!(v["large"].as_f64().is_some());
    // Re-emit and re-parse — values must compare equal.
    let out = to_string(&v).unwrap();
    let v2: Value = from_str(&out).unwrap();
    assert_eq!(v, v2);
}

#[test]
fn hex_and_octal_integer_literals() {
    // YAML 1.2 `0x` and `0o` prefixes are recognised by noyalib.
    let v: Value = from_str("hex: 0xFF\noct: 0o17\n").unwrap();
    assert_eq!(v["hex"].as_i64(), Some(0xFF));
    assert_eq!(v["oct"].as_i64(), Some(0o17));
}

// ── Multi-line plain scalars ──────────────────────────────────────

#[test]
fn folded_block_scalar_preserves_paragraphs() {
    let yaml = "msg: >\n  line one\n  continues\n\n  line two\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["msg"].as_str().unwrap();
    assert!(s.contains("line one continues"));
    assert!(s.contains("line two"));
}

#[test]
fn literal_block_scalar_preserves_newlines() {
    let yaml = "code: |\n  fn main() {\n      println!();\n  }\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v["code"].as_str().unwrap();
    assert!(s.contains("fn main() {"));
    assert!(s.contains("println!();"));
    // Newlines preserved inside literal blocks.
    assert!(s.contains('\n'));
}
