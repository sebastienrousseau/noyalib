// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage-focused deserialization tests exercising the typed
//! `Deserializer` (`src/de/deserializer.rs`) and the public entry
//! points in `src/de.rs`. Concentrates on error / edge branches:
//! type mismatches, integer over/underflow, borrowed vs owned
//! deserialisation, `deserialize_any` scalar dispatch, `Spanned`
//! newtype handling, strict unknown-field detection, and
//! `from_value` / `from_slice` wrappers.

#![allow(
    missing_docs,
    dead_code,
    unused_results,
    unused_must_use,
    non_snake_case,
    clippy::all
)]
#![allow(clippy::unwrap_used)]

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use noyalib::{
    ParserConfig, Spanned, Value, from_slice, from_slice_with_config, from_str, from_str_borrowing,
    from_str_borrowing_with_config, from_str_strict, from_str_with_config, from_value,
};

// ============================================================================
// Concrete-type happy paths (typed Deserializer walk).
// ============================================================================

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Simple {
    name: String,
    port: u16,
    enabled: bool,
}

#[test]
fn struct_all_fields() {
    let yaml = "name: web\nport: 8080\nenabled: true\n";
    let got: Simple = from_str(yaml).unwrap();
    assert_eq!(
        got,
        Simple {
            name: "web".into(),
            port: 8080,
            enabled: true
        }
    );
}

#[test]
fn nested_struct_and_vec() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Outer {
        inner: Simple,
        tags: Vec<String>,
    }
    let yaml = "\
inner:
  name: web
  port: 80
  enabled: false
tags:
  - a
  - b
";
    let got: Outer = from_str(yaml).unwrap();
    assert_eq!(got.inner.port, 80);
    assert_eq!(got.tags, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn every_integer_width() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Ints {
        a: i8,
        b: i16,
        c: i32,
        d: i64,
        e: u8,
        f: u16,
        g: u32,
        h: u64,
    }
    let yaml = "a: -1\nb: -2\nc: -3\nd: -4\ne: 1\nf: 2\ng: 3\nh: 4\n";
    let got: Ints = from_str(yaml).unwrap();
    assert_eq!(got.a, -1);
    assert_eq!(got.h, 4);
}

#[test]
fn floats_from_int_and_float() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Floats {
        x: f32,
        y: f64,
    }
    // y is written as an integer scalar → deserialize_f64 int arm.
    let yaml = "x: 1.5\ny: 3\n";
    let got: Floats = from_str(yaml).unwrap();
    assert_eq!(got.x, 1.5);
    assert_eq!(got.y, 3.0);
}

#[test]
fn char_bool_unit_newtype() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct NewType(u32);

    let c: char = from_str("x\n").unwrap();
    assert_eq!(c, 'x');

    let b: bool = from_str("false\n").unwrap();
    assert!(!b);

    let nt: NewType = from_value(&Value::from(7_i64)).unwrap();
    assert_eq!(nt, NewType(7));

    // Unit type from null.
    let u: () = from_str("null\n").unwrap();
    assert_eq!(u, ());
}

#[test]
fn option_some_and_none() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Opt {
        present: Option<i32>,
        absent: Option<i32>,
    }
    let yaml = "present: 5\nabsent: null\n";
    let got: Opt = from_str(yaml).unwrap();
    assert_eq!(got.present, Some(5));
    assert_eq!(got.absent, None);
}

#[test]
fn tuple_and_tuple_struct() {
    let t: (i32, String, bool) = from_str("- 1\n- two\n- true\n").unwrap();
    assert_eq!(t, (1, "two".to_string(), true));

    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Pair(i32, i32);
    let p: Pair = from_str("- 3\n- 4\n").unwrap();
    assert_eq!(p, Pair(3, 4));
}

#[test]
fn hashmap_and_btreemap() {
    let hm: HashMap<String, i32> = from_str("a: 1\nb: 2\n").unwrap();
    assert_eq!(hm.get("a"), Some(&1));

    let bm: BTreeMap<String, i32> = from_str("x: 10\ny: 20\n").unwrap();
    assert_eq!(bm.get("y"), Some(&20));
}

// ============================================================================
// Enums (unit / newtype / tuple / struct variants).
// ============================================================================

#[derive(Debug, serde::Deserialize, PartialEq)]
enum Shape {
    Point,
    Circle(f64),
    Rect(f64, f64),
    Named { label: String },
}

#[test]
fn enum_unit_variant_from_string() {
    let s: Shape = from_str("Point\n").unwrap();
    assert_eq!(s, Shape::Point);
}

#[test]
fn enum_newtype_variant() {
    let s: Shape = from_str("Circle: 2.0\n").unwrap();
    assert_eq!(s, Shape::Circle(2.0));
}

#[test]
fn enum_tuple_variant() {
    let s: Shape = from_str("Rect:\n  - 1.0\n  - 2.0\n").unwrap();
    assert_eq!(s, Shape::Rect(1.0, 2.0));
}

#[test]
fn enum_struct_variant() {
    let s: Shape = from_str("Named:\n  label: hi\n").unwrap();
    assert_eq!(s, Shape::Named { label: "hi".into() });
}

#[test]
fn enum_unit_variant_as_single_key_map() {
    // `{Point: null}` drives the Mapping arm of deserialize_enum and
    // VariantAccess::unit_variant.
    let s: Shape = from_str("Point: null\n").unwrap();
    assert_eq!(s, Shape::Point);
}

// ============================================================================
// deserialize_any scalar dispatch (Value elements inside a typed walk).
// ============================================================================

#[test]
fn value_sequence_exercises_deserialize_any() {
    // Vec<Value> is not the Value-target fast path, so the outer
    // Vec goes through the typed Deserializer and each element's
    // `Value::deserialize` calls `deserialize_any`, hitting the
    // null/bool/int/float/string/seq/map arms.
    let yaml = "\
- null
- true
- 7
- 1.5
- hello
- [1, 2]
- {k: v}
";
    let v: Vec<Value> = from_str(yaml).unwrap();
    assert_eq!(v.len(), 7);
    assert!(v[0].is_null());
    assert_eq!(v[1].as_bool(), Some(true));
    assert_eq!(v[2].as_i64(), Some(7));
    assert_eq!(v[4].as_str(), Some("hello"));
    assert!(v[5].as_sequence().is_some());
    assert!(v[6].as_mapping().is_some());
}

#[test]
fn from_value_into_typed_struct() {
    let map: Value = from_slice(b"name: z\nport: 1\nenabled: true\n").unwrap();
    let got: Simple = from_value(&map).unwrap();
    assert_eq!(got.name, "z");
}

// ============================================================================
// Borrowed deserialization.
// ============================================================================

#[test]
fn borrowed_str_field() {
    #[derive(Debug, serde::Deserialize)]
    struct Person<'a> {
        name: &'a str,
        role: &'a str,
    }
    let yaml = "name: noyalib\nrole: parser\n";
    let p: Person<'_> = from_str_borrowing(yaml).unwrap();
    assert_eq!(p.name, "noyalib");
    assert_eq!(p.role, "parser");
}

#[test]
fn borrowed_terminal_scalar() {
    let s: &str = from_str_borrowing("hello").unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn borrowed_with_config() {
    let cfg = ParserConfig::strict();
    let s: &str = from_str_borrowing_with_config("world", &cfg).unwrap();
    assert_eq!(s, "world");
}

#[test]
fn borrowed_cow_falls_back_to_owned_on_escape() {
    #[derive(Debug, serde::Deserialize)]
    struct Doc<'a> {
        #[serde(borrow)]
        plain: Cow<'a, str>,
        #[serde(borrow)]
        escaped: Cow<'a, str>,
    }
    let yaml = "plain: raw\nescaped: \"a\\tb\"\n";
    let d: Doc<'_> = from_str_borrowing(yaml).unwrap();
    assert_eq!(d.plain, "raw");
    assert_eq!(d.escaped, "a\tb");
}

#[test]
fn borrowed_with_config_document_too_long() {
    let cfg = ParserConfig::default().max_document_length(2);
    let res: Result<String, _> = from_str_borrowing_with_config("hello world", &cfg);
    assert!(res.is_err());
}

// ============================================================================
// Spanned<T> — drives deserialize_newtype_struct / deserialize_struct
// SPANNED_TYPE_NAME arm and SpannedMapAccess.
// ============================================================================

#[test]
fn spanned_scalar() {
    let val: Spanned<String> = from_str("hello\n").unwrap();
    assert_eq!(val.into_inner(), "hello");
}

#[test]
fn spanned_struct_field() {
    #[derive(Debug, serde::Deserialize)]
    struct Cfg {
        port: Spanned<u16>,
    }
    let got: Cfg = from_str("port: 8080\n").unwrap();
    assert_eq!(*got.port, 8080);
}

// ============================================================================
// from_value / from_slice / config wrappers.
// ============================================================================

#[test]
fn from_value_scalar() {
    let n: i32 = from_value(&Value::from(42_i64)).unwrap();
    assert_eq!(n, 42);
}

#[test]
fn from_value_identity_value_target() {
    let src = Value::from(9_i64);
    let out: Value = from_value(&src).unwrap();
    assert_eq!(out.as_i64(), Some(9));
}

#[test]
fn from_slice_and_slice_config() {
    let n: i32 = from_slice(b"5").unwrap();
    assert_eq!(n, 5);
    let cfg = ParserConfig::new();
    let m: i32 = from_slice_with_config(b"6", &cfg).unwrap();
    assert_eq!(m, 6);
}

#[test]
fn from_str_with_config_value_target() {
    // Value target routes through the AST loader fast path in
    // from_str_with_config.
    let cfg = ParserConfig::default();
    let v: Value = from_str_with_config("a: 1\n", &cfg).unwrap();
    assert!(v.as_mapping().is_some());
}

// ============================================================================
// Strict deserialisation (unknown fields).
// ============================================================================

#[test]
fn strict_accepts_declared_fields() {
    #[derive(Debug, serde::Deserialize)]
    struct C {
        port: u16,
    }
    let got: C = from_str_strict("port: 8080\n").unwrap();
    assert_eq!(got.port, 8080);
}

#[test]
fn strict_rejects_unknown_field() {
    #[derive(Debug, serde::Deserialize)]
    struct C {
        port: u16,
    }
    let res: Result<C, _> = from_str_strict("port: 8080\nporrt: 9090\n");
    assert!(res.is_err());
    // The lenient path silently ignores it.
    assert!(from_str::<C>("port: 8080\nporrt: 9090\n").is_ok());
}

// ============================================================================
// Deliberate failure cases — assert is_err() only.
// ============================================================================

#[test]
fn err_wrong_type_bool() {
    let res: Result<bool, _> = from_str("not_a_bool\n");
    assert!(res.is_err());
}

#[test]
fn err_string_into_integer() {
    let res: Result<i32, _> = from_str("hello\n");
    assert!(res.is_err());
}

#[test]
fn err_u8_overflow() {
    // Value fits i64 but overflows u8 → serde custom error →
    // Error::Deserialize, routed through wrap_err with span context.
    #[derive(Debug, serde::Deserialize)]
    struct S {
        x: u8,
    }
    let res: Result<S, _> = from_str("x: 99999\n");
    assert!(res.is_err());
}

#[test]
fn err_i8_overflow() {
    let res: Result<i8, _> = from_str("1000\n");
    assert!(res.is_err());
}

#[test]
fn err_negative_into_unsigned() {
    let res: Result<u32, _> = from_str("-5\n");
    assert!(res.is_err());
}

#[test]
fn err_missing_required_field() {
    let res: Result<Simple, _> = from_str("name: web\n");
    assert!(res.is_err());
}

#[test]
fn err_seq_into_map() {
    let res: Result<Simple, _> = from_str("- 1\n- 2\n");
    assert!(res.is_err());
}

#[test]
fn err_map_into_seq() {
    let res: Result<Vec<i32>, _> = from_str("a: 1\n");
    assert!(res.is_err());
}

#[test]
fn err_float_into_bool() {
    let res: Result<bool, _> = from_str("1.5\n");
    assert!(res.is_err());
}

#[test]
fn err_multichar_into_char() {
    let res: Result<char, _> = from_str("abc\n");
    assert!(res.is_err());
}

#[test]
fn err_unknown_enum_variant() {
    let res: Result<Shape, _> = from_str("Triangle\n");
    assert!(res.is_err());
}

#[test]
fn err_malformed_yaml_unclosed_flow() {
    let res: Result<Value, _> = from_str("[1, 2, 3\n");
    assert!(res.is_err());
}

#[test]
fn err_empty_input_into_struct() {
    let res: Result<Simple, _> = from_str("");
    assert!(res.is_err());
}

#[test]
fn err_from_slice_invalid_utf8() {
    let bad = [0xff, 0xfe, 0xfd];
    let res: Result<Value, _> = from_slice(&bad);
    assert!(res.is_err());
}

#[test]
fn err_document_too_long() {
    let cfg = ParserConfig::default().max_document_length(3);
    let res: Result<Value, _> = from_str_with_config("name: web\n", &cfg);
    assert!(res.is_err());
}

#[test]
fn err_float_into_int_with_fraction() {
    // A fractional float cannot become an i32.
    let res: Result<i32, _> = from_str("1.5\n");
    assert!(res.is_err());
}

#[test]
fn unit_type_from_non_null_errs() {
    let res: Result<(), _> = from_str("1\n");
    assert!(res.is_err());
}
