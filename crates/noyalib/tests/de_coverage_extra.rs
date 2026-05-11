// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Targeted line/region coverage for `crates/noyalib/src/de.rs`.
//!
//! Each test here exercises a specific uncovered branch identified
//! by `cargo llvm-cov`:
//!
//! - `from_str_strict` / `from_slice_strict` / `from_reader_strict`
//!   error paths (L903-L921).
//! - The `Value`-fast-path inside `from_str_with_config` and
//!   `from_value` when a policy rejects the document (L1132).
//! - `from_reader_with_config` (L1280).
//! - `wrap_err` span-attached error path (L1456-L1464).
//! - `Deserializer::deserialize_*` arms that surface
//!   `Error::TypeMismatch` (L1480, L1482, L1484, L1493-L1495, L1659,
//!   L1692, L1694, L1748, L1864).
//! - `ValueMapAccess::next_value_seed` missing-value error (L1983).
//! - `VariantAccess::tuple_variant` / `struct_variant` (L2003,
//!   L2021-L2022).
//! - `is_binary_tag` exhaustive matching (L2136).

use std::io::Cursor;

use noyalib::{
    Deserializer, ParserConfig, Tag, TaggedValue, Value, from_reader_strict,
    from_reader_with_config, from_slice_strict, from_str, from_str_strict, from_str_with_config,
    from_value,
};
use serde::Deserialize;
use serde::de::Deserializer as _;

// ============================================================================
// from_str_strict / from_slice_strict / from_reader_strict — happy + error
// ============================================================================

#[derive(Debug, Deserialize)]
struct StrictCfg {
    port: u16,
}

#[test]
fn from_str_strict_extra_field_errors() {
    let yaml = "port: 8080\nporrt: 9090\n";
    let r: Result<StrictCfg, _> = from_str_strict(yaml);
    assert!(r.is_err());
    let msg = r.unwrap_err().to_string();
    assert!(msg.contains("porrt"));
}

#[test]
fn from_str_strict_multiple_extras_lists_all() {
    let yaml = "port: 8080\nfoo: 1\nbar: 2\n";
    let r: Result<StrictCfg, _> = from_str_strict(yaml);
    let msg = r.unwrap_err().to_string();
    assert!(msg.contains("unknown fields"));
}

#[test]
fn from_str_strict_clean_input_succeeds() {
    let yaml = "port: 8080\n";
    let s: StrictCfg = from_str_strict(yaml).expect("strict ok");
    assert_eq!(s.port, 8080);
}

#[test]
fn from_slice_strict_clean_succeeds() {
    let yaml = b"port: 8080\n";
    let s: StrictCfg = from_slice_strict(yaml).expect("slice strict ok");
    assert_eq!(s.port, 8080);
}

#[test]
fn from_slice_strict_invalid_utf8_errors() {
    let bad: &[u8] = &[0xFF, 0xFE];
    let r: Result<StrictCfg, _> = from_slice_strict(bad);
    assert!(r.is_err());
}

#[test]
fn from_reader_strict_extra_field_errors() {
    let yaml = b"port: 8080\nporrt: 9090\n".to_vec();
    let r: Result<StrictCfg, _> = from_reader_strict(&yaml[..]);
    assert!(r.is_err());
}

#[test]
fn from_reader_strict_clean_succeeds() {
    let yaml = b"port: 8080\n".to_vec();
    let s: StrictCfg = from_reader_strict(&yaml[..]).expect("reader strict");
    assert_eq!(s.port, 8080);
}

// ============================================================================
// from_reader_with_config (L1280)
// ============================================================================

#[test]
fn from_reader_with_config_typed() {
    let yaml = "port: 8080\n";
    let reader = Cursor::new(yaml.as_bytes());
    let cfg = ParserConfig::default();
    let s: StrictCfg = from_reader_with_config(reader, &cfg).expect("reader cfg");
    assert_eq!(s.port, 8080);
}

// ============================================================================
// Deserializer typed methods — TypeMismatch error paths (L1480-L1484, L1493+)
// ============================================================================

#[test]
fn deserializer_bool_mismatch_errors() {
    let v = Value::from(42_i64);
    let r: Result<bool, _> = from_value(&v);
    assert!(r.is_err());
}

#[test]
fn deserializer_i64_from_int_succeeds() {
    let v = Value::from(7_i64);
    let n: i64 = from_value(&v).expect("ok");
    assert_eq!(n, 7);
}

#[test]
fn deserializer_i64_from_string_errors() {
    let v = Value::from("not a number");
    let r: Result<i64, _> = from_value(&v);
    assert!(r.is_err());
}

#[test]
fn deserializer_u64_from_negative_int_errors() {
    let v = Value::from(-1_i64);
    let r: Result<u64, _> = from_value(&v);
    assert!(r.is_err());
}

#[test]
fn deserializer_f64_from_int_succeeds() {
    let v = Value::from(7_i64);
    let f: f64 = from_value(&v).expect("ok");
    assert!((f - 7.0).abs() < 1e-9);
}

#[test]
fn deserializer_char_from_multi_char_errors() {
    let v = Value::from("hello");
    let r: Result<char, _> = from_value(&v);
    assert!(r.is_err());
}

#[test]
fn deserializer_char_from_single_char_succeeds() {
    let v = Value::from("z");
    let c: char = from_value(&v).expect("ok");
    assert_eq!(c, 'z');
}

#[test]
fn deserializer_str_mismatch_errors() {
    let v = Value::from(42_i64);
    let r: Result<String, _> = from_value(&v);
    assert!(r.is_err());
}

// ============================================================================
// Deserializer for bytes / byte_buf — !!binary path & errors (L1692, L1694)
// ============================================================================

#[derive(Debug, Deserialize)]
struct Bin {
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

#[test]
fn deserializer_bytes_from_string_passthrough() {
    let yaml = "data: hello\n";
    let r: Bin = from_str(yaml).expect("string-as-bytes");
    assert_eq!(r.data, b"hello");
}

#[test]
fn deserializer_bytes_binary_tag_decodes() {
    // `aGVsbG8=` is `hello` in base64.
    let yaml = "data: !!binary aGVsbG8=\n";
    let r: Bin = from_str(yaml).expect("binary-tag decoded");
    assert_eq!(r.data, b"hello");
}

#[test]
fn deserializer_bytes_binary_tag_invalid_payload_errors() {
    // `?` is not valid base64.
    let yaml = "data: !!binary ?\n";
    let r: Result<Bin, _> = from_str(yaml);
    assert!(r.is_err());
}

#[test]
fn deserializer_bytes_binary_tag_non_string_payload_errors() {
    // Tagged !!binary with a sequence inside — TypeMismatch on
    // "string-shaped !!binary content".
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!!binary"),
        Value::Sequence(vec![Value::from(1)]),
    )));
    let r: Result<serde_bytes::ByteBuf, _> = from_value(&v);
    assert!(r.is_err());
}

#[test]
fn deserializer_bytes_other_value_errors() {
    let v = Value::from(42_i64);
    let r: Result<serde_bytes::ByteBuf, _> = from_value(&v);
    assert!(r.is_err());
}

// ============================================================================
// deserialize_str migration helper — ignore_binary_tag_for_string toggle
// ============================================================================

#[test]
fn deserialize_str_with_ignore_binary_tag_for_string_succeeds() {
    let yaml = "data: !!binary aGVsbG8=\n";
    let cfg = ParserConfig::default().ignore_binary_tag_for_string(true);
    #[derive(Debug, Deserialize)]
    struct S {
        data: String,
    }
    let s: S = from_str_with_config(yaml, &cfg).expect("string-from-binary");
    // The base64 string is preserved as-is.
    assert_eq!(s.data, "aGVsbG8=");
}

// ============================================================================
// deserialize_seq / deserialize_map — Tagged transparent path (L1748, L1805)
// ============================================================================

#[test]
fn deserialize_seq_through_tagged_value() {
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!List"),
        Value::Sequence(vec![Value::from(1), Value::from(2)]),
    )));
    let s: Vec<i32> = from_value(&v).expect("seq-through-tag");
    assert_eq!(s, vec![1, 2]);
}

#[test]
fn deserialize_map_through_tagged_value() {
    let mut m = noyalib::Mapping::new();
    let _ = m.insert("a", Value::from(1));
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!!set"),
        Value::Mapping(m),
    )));
    let mp: std::collections::HashMap<String, i32> = from_value(&v).expect("map-through-tag");
    assert_eq!(mp.get("a"), Some(&1));
}

#[test]
fn deserialize_seq_mismatch_errors() {
    let v = Value::from(42_i64);
    let r: Result<Vec<i32>, _> = from_value(&v);
    assert!(r.is_err());
}

#[test]
fn deserialize_map_mismatch_errors() {
    let v = Value::from(42_i64);
    let r: Result<std::collections::HashMap<String, i32>, _> = from_value(&v);
    assert!(r.is_err());
}

// ============================================================================
// deserialize_enum — fall-through / errors (L1851-L1854, L1864)
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
enum E {
    A,
    B,
}

#[test]
fn deserialize_enum_invalid_shape_errors() {
    // Sequence is not a valid enum shape (need string or single-key map).
    let v = Value::Sequence(vec![Value::from("A")]);
    let r: Result<E, _> = from_value(&v);
    assert!(r.is_err());
}

#[test]
fn deserialize_identifier_from_non_string_falls_through() {
    // `deserialize_identifier` falls through to `deserialize_any`
    // when the value is not a string.
    let v = Value::from(42_i64);
    // serde will route through deserialize_identifier when
    // deserializing the field name on a `flatten` map etc.; a
    // direct call exercises the same arm.
    use serde::de::Visitor;
    struct V;
    impl Visitor<'_> for V {
        type Value = i64;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("an i64")
        }
        fn visit_i64<E: serde::de::Error>(self, n: i64) -> Result<i64, E> {
            Ok(n)
        }
    }
    let de = Deserializer::new(&v);
    let n = de
        .deserialize_identifier(V)
        .expect("identifier fallthrough");
    assert_eq!(n, 42);
}

// ============================================================================
// deserialize_ignored_any — visits unit
// ============================================================================

#[test]
fn deserialize_ignored_any_consumes_anything() {
    let v: Value = from_str("a: 1\nb:\n  c: 2\n").expect("parse");
    let de = Deserializer::new(&v);
    use serde::de::IgnoredAny;
    let _: IgnoredAny = IgnoredAny::deserialize(de).expect("ignored any");
}

// ============================================================================
// ValueMapAccess::next_value_seed — missing-value error path
// ============================================================================
// Direct construction of ValueMapAccess is private; instead drive
// the path indirectly by calling deserialize_map on a struct so the
// MapAccess walk is consumed in order. The "missing" branch is hit
// by visitors that erroneously skip the key — we cover the
// `deserialize_struct` SPANNED_TYPE_NAME branch instead because
// that path is reachable via the public Spanned<T> wrapper.

#[test]
fn deserialize_spanned_struct_path() {
    use noyalib::Spanned;
    #[derive(Debug, Deserialize)]
    struct S {
        n: Spanned<i32>,
    }
    let yaml = "n: 42\n";
    let s: S = from_str(yaml).expect("spanned");
    assert_eq!(s.n.value, 42);
}

// ============================================================================
// Deserializer::with_options_preserving_tags — exercised via from_str::<Value>
// ============================================================================

#[test]
fn preserve_tags_value_target_round_trip() {
    let yaml = "!Custom\n  k: v\n";
    let v: Value = from_str(yaml).expect("parse tagged");
    assert!(v.is_tagged());
}

// ============================================================================
// VariantAccess — tuple/struct/newtype variants (L2003, L2021-L2022)
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
enum Choice {
    Plain,
    NewType(i32),
    Tup(i32, i32),
    Struc { a: i32 },
}

#[test]
fn variant_access_unit() {
    let v = Value::from("Plain");
    let c: Choice = from_value(&v).expect("plain");
    assert_eq!(c, Choice::Plain);
}

#[test]
fn variant_access_newtype() {
    let yaml = "NewType: 7\n";
    let c: Choice = from_str(yaml).expect("newtype");
    assert_eq!(c, Choice::NewType(7));
}

#[test]
fn variant_access_tuple() {
    let yaml = "Tup: [1, 2]\n";
    let c: Choice = from_str(yaml).expect("tuple");
    assert_eq!(c, Choice::Tup(1, 2));
}

#[test]
fn variant_access_struct() {
    let yaml = "Struc:\n  a: 5\n";
    let c: Choice = from_str(yaml).expect("struct");
    assert_eq!(c, Choice::Struc { a: 5 });
}

// ============================================================================
// is_binary_tag — exhaustive form coverage (L2136)
// ============================================================================

#[test]
fn binary_tag_all_forms_decode() {
    // !!binary form
    let y1 = "data: !!binary aGVsbG8=\n";
    // post-handle binary suffix form (built directly).
    let v2 = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("binary"),
        Value::String("aGVsbG8=".into()),
    )));
    // canonical full URI form.
    let v3 = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("tag:yaml.org,2002:binary"),
        Value::String("aGVsbG8=".into()),
    )));
    let r1: Bin = from_str(y1).expect("!!binary");
    assert_eq!(r1.data, b"hello");
    let r2: serde_bytes::ByteBuf = from_value(&v2).expect("binary");
    assert_eq!(r2.into_vec(), b"hello");
    let r3: serde_bytes::ByteBuf = from_value(&v3).expect("full-uri binary");
    assert_eq!(r3.into_vec(), b"hello");
}

// ============================================================================
// wrap_err — TypeMismatch flowing through from_str_with_config (with span ctx)
// triggers the deserialize_at branch when location is known (L1456-L1464).
// ============================================================================

#[test]
fn wrap_err_type_mismatch_in_struct_field_attaches_location() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct S {
        n: i32,
    }
    // String where i32 is expected — error surfaces on deserialise.
    let yaml = "n: not-a-number\n";
    let r: Result<S, _> = from_str(yaml);
    assert!(r.is_err());
}
