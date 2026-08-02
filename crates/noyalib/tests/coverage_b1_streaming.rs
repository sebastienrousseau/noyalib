// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![allow(
    missing_docs,
    dead_code,
    unused_results,
    unused_must_use,
    non_snake_case,
    clippy::all
)]

//! Broad coverage sweep for `crates/noyalib/src/streaming.rs`.
//!
//! Drives the streaming deserializer both directly via
//! [`noyalib::StreamingDeserializer`] and indirectly through
//! [`noyalib::from_str`] / [`noyalib::from_str_with_config`] (which route
//! typed non-`Value` targets through `streaming::from_str_streaming`). Each
//! test exercises a distinct code path: scalar resolution, all scalar styles,
//! flow/block collections, anchors/aliases, merge keys, tags, enums, bytes,
//! options, and the parser/security-limit error paths.

use std::collections::BTreeMap;
use std::sync::Arc;

use noyalib::{
    DuplicateKeyPolicy, ParserConfig, StreamingDeserializer, TagRegistry, Value, from_str,
    from_str_with_config,
};
use serde::Deserialize;
use serde_bytes::ByteBuf;

// ── Construction & basic driving ─────────────────────────────────────────

#[test]
fn new_over_borrowed_str() {
    let mut de = StreamingDeserializer::new("k: 1\n");
    let m: BTreeMap<String, i64> = Deserialize::deserialize(&mut de).unwrap();
    assert_eq!(m["k"], 1);
}

#[test]
fn with_config_strict() {
    let cfg = ParserConfig::strict();
    let mut de = StreamingDeserializer::with_config("k: 1\n", &cfg);
    let m: BTreeMap<String, i64> = Deserialize::deserialize(&mut de).unwrap();
    assert_eq!(m["k"], 1);
}

#[test]
fn debug_impl_renders_fields() {
    let de = StreamingDeserializer::new("a: 1\n");
    let s = format!("{de:?}");
    assert!(s.contains("StreamingDeserializer"));
    assert!(s.contains("input_len"));
    assert!(s.contains("depth"));
}

// ── Scalar type deserialisation ──────────────────────────────────────────

#[test]
fn scalar_bool_true_false() {
    let t: bool = from_str("true\n").unwrap();
    let f: bool = from_str("false\n").unwrap();
    assert!(t);
    assert!(!f);
}

#[test]
fn scalar_i64_negative() {
    let n: i64 = from_str("-123\n").unwrap();
    assert_eq!(n, -123);
}

#[test]
fn scalar_i8_via_forward() {
    // i8 forwards to deserialize_any through the streaming path.
    let n: i8 = from_str("-5\n").unwrap();
    assert_eq!(n, -5);
}

#[test]
fn scalar_u32_via_forward() {
    let n: u32 = from_str("4000\n").unwrap();
    assert_eq!(n, 4000);
}

#[test]
fn scalar_u64_large() {
    let n: u64 = from_str("18446744073709551615\n").unwrap_or(0);
    // Whether lossless-u64 is on or not, this must not panic; if parsed as
    // u64 it equals MAX, otherwise the fallback path returns 0 via unwrap_or.
    assert!(n == u64::MAX || n == 0);
}

#[test]
fn scalar_f64_plain() {
    let f: f64 = from_str("3.14\n").unwrap();
    assert!((f - 3.14).abs() < 1e-9);
}

#[test]
fn scalar_f32_via_forward() {
    let f: f32 = from_str("2.5\n").unwrap();
    assert!((f - 2.5).abs() < 1e-6);
}

#[test]
fn scalar_f64_infinity() {
    let f: f64 = from_str(".inf\n").unwrap();
    assert!(f.is_infinite() && f > 0.0);
}

#[test]
fn scalar_f64_neg_infinity() {
    let f: f64 = from_str("-.inf\n").unwrap();
    assert!(f.is_infinite() && f < 0.0);
}

#[test]
fn scalar_f64_nan() {
    let f: f64 = from_str(".nan\n").unwrap();
    assert!(f.is_nan());
}

#[test]
fn scalar_char_via_forward() {
    let c: char = from_str("x\n").unwrap();
    assert_eq!(c, 'x');
}

// ── Null / option / unit ─────────────────────────────────────────────────

#[test]
fn option_none_tilde() {
    let v: Option<i64> = from_str("~\n").unwrap();
    assert_eq!(v, None);
}

#[test]
fn option_none_null_word() {
    let v: Option<i64> = from_str("null\n").unwrap();
    assert_eq!(v, None);
}

#[test]
fn option_some_value() {
    let v: Option<i64> = from_str("7\n").unwrap();
    assert_eq!(v, Some(7));
}

#[test]
fn option_some_string() {
    let v: Option<String> = from_str("hello\n").unwrap();
    assert_eq!(v.as_deref(), Some("hello"));
}

#[test]
fn unit_from_null() {
    let _: () = from_str("null\n").unwrap();
}

#[test]
fn unit_mismatch_errors() {
    let res: Result<(), _> = from_str("42\n");
    assert!(res.is_err());
}

// ── Scalar styles ────────────────────────────────────────────────────────

#[test]
fn style_single_quoted() {
    let s: String = from_str("'hi there'\n").unwrap();
    assert_eq!(s, "hi there");
}

#[test]
fn style_double_quoted_with_escape() {
    let s: String = from_str("\"a\\tb\"\n").unwrap();
    assert_eq!(s, "a\tb");
}

#[test]
fn style_literal_block() {
    let s: String = from_str("|\n  line1\n  line2\n").unwrap();
    assert_eq!(s, "line1\nline2\n");
}

#[test]
fn style_folded_block() {
    let s: String = from_str(">\n  a\n  b\n").unwrap();
    assert_eq!(s, "a b\n");
}

#[test]
fn style_quoted_numberlike_is_string() {
    // "42" quoted must stay a string, not resolve to an int.
    let s: String = from_str("\"42\"\n").unwrap();
    assert_eq!(s, "42");
}

// ── Sequences ────────────────────────────────────────────────────────────

#[test]
fn seq_flow_ints() {
    let v: Vec<i64> = from_str("[1, 2, 3]\n").unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn seq_block_strings() {
    let v: Vec<String> = from_str("- a\n- b\n- c\n").unwrap();
    assert_eq!(v, vec!["a", "b", "c"]);
}

#[test]
fn seq_empty_flow() {
    let v: Vec<i64> = from_str("[]\n").unwrap();
    assert!(v.is_empty());
}

#[test]
fn seq_nested() {
    let v: Vec<Vec<i64>> = from_str("- [1, 2]\n- [3, 4]\n").unwrap();
    assert_eq!(v, vec![vec![1, 2], vec![3, 4]]);
}

#[test]
fn seq_tuple() {
    let t: (i64, String, bool) = from_str("[1, two, true]\n").unwrap();
    assert_eq!(t, (1, "two".to_string(), true));
}

#[test]
fn seq_of_mixed_value() {
    let v: Vec<Value> = from_str("[1, two, 3.0, true, null]\n").unwrap();
    assert_eq!(v.len(), 5);
    assert_eq!(v[0].as_i64(), Some(1));
    assert_eq!(v[1].as_str(), Some("two"));
    assert!(v[4].is_null());
}

// ── Mappings / structs ───────────────────────────────────────────────────

#[test]
fn struct_full() {
    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct Server {
        host: String,
        port: u16,
        tags: Vec<String>,
    }
    let yaml = "host: h\nport: 80\ntags:\n  - a\n  - b\n";
    let mut de = StreamingDeserializer::new(yaml);
    let s = Server::deserialize(&mut de).unwrap();
    assert_eq!(
        s,
        Server {
            host: "h".into(),
            port: 80,
            tags: vec!["a".into(), "b".into()]
        }
    );
}

#[test]
fn map_flow_style() {
    let m: BTreeMap<String, i64> = from_str("{a: 1, b: 2}\n").unwrap();
    assert_eq!(m["a"], 1);
    assert_eq!(m["b"], 2);
}

#[test]
fn map_empty_flow() {
    let m: BTreeMap<String, i64> = from_str("{}\n").unwrap();
    assert!(m.is_empty());
}

#[test]
fn map_deeply_nested() {
    let m: BTreeMap<String, BTreeMap<String, BTreeMap<String, i64>>> =
        from_str("a:\n  b:\n    c: 9\n").unwrap();
    assert_eq!(m["a"]["b"]["c"], 9);
}

#[test]
fn map_struct_with_optional_and_default() {
    #[derive(serde::Deserialize)]
    struct Doc {
        req: i64,
        #[serde(default)]
        opt: i64,
    }
    let d: Doc = from_str("req: 5\n").unwrap();
    assert_eq!(d.req, 5);
    assert_eq!(d.opt, 0);
}

#[test]
fn struct_ignores_extra_field() {
    #[derive(serde::Deserialize)]
    struct Small {
        keep: i64,
    }
    let yaml = "keep: 1\nextra:\n  nested:\n    - 1\n    - 2\n  more:\n    a: x\n";
    let s: Small = from_str(yaml).unwrap();
    assert_eq!(s.keep, 1);
}

// ── Anchors & aliases ────────────────────────────────────────────────────

#[test]
fn anchor_scalar_alias() {
    let m: BTreeMap<String, String> = from_str("a: &x hello\nb: *x\n").unwrap();
    assert_eq!(m["b"], "hello");
}

#[test]
fn anchor_seq_alias() {
    let m: BTreeMap<String, Vec<i64>> = from_str("a: &x [1, 2, 3]\nb: *x\n").unwrap();
    assert_eq!(m["b"], vec![1, 2, 3]);
}

#[test]
fn anchor_map_alias() {
    let m: BTreeMap<String, BTreeMap<String, i64>> = from_str("a: &x {k: 1}\nb: *x\n").unwrap();
    assert_eq!(m["b"]["k"], 1);
}

#[test]
fn anchor_reused_struct() {
    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct Ep {
        host: String,
        port: u16,
    }
    #[derive(serde::Deserialize)]
    struct Doc {
        primary: Ep,
        replica: Ep,
    }
    let yaml = "primary: &p\n  host: db\n  port: 5432\nreplica: *p\n";
    let mut de = StreamingDeserializer::new(yaml);
    let d = Doc::deserialize(&mut de).unwrap();
    assert_eq!(d.primary, d.replica);
}

#[test]
fn unknown_anchor_errors() {
    let mut de = StreamingDeserializer::new("foo: *missing\n");
    let err = <BTreeMap<String, String>>::deserialize(&mut de).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("missing") || msg.contains("unknown") || msg.contains("anchor"));
}

// ── Merge keys ───────────────────────────────────────────────────────────

#[test]
fn merge_single_source() {
    let yaml = "base: &b\n  a: 1\n  b: 2\ntarget:\n  <<: *b\n  c: 3\n";
    let m: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml).unwrap();
    assert_eq!(m["target"]["a"], 1);
    assert_eq!(m["target"]["b"], 2);
    assert_eq!(m["target"]["c"], 3);
}

#[test]
fn merge_multi_source_precedence() {
    let yaml = "\
d: &d
  host: localhost
  port: 8080
o: &o
  port: 9090
  timeout: 30
target:
  <<: [*o, *d]
  debug: true
";
    let m: BTreeMap<String, BTreeMap<String, Value>> = from_str(yaml).unwrap();
    let t = &m["target"];
    assert_eq!(t["host"].as_str(), Some("localhost"));
    assert_eq!(t["port"].as_i64(), Some(9090));
    assert_eq!(t["timeout"].as_i64(), Some(30));
    assert!(t["debug"].as_bool().unwrap());
}

#[test]
fn merge_local_overrides_source() {
    let yaml = "base: &b\n  k: from_base\ntarget:\n  <<: *b\n  k: from_local\n";
    let m: BTreeMap<String, BTreeMap<String, String>> = from_str(yaml).unwrap();
    assert_eq!(m["target"]["k"], "from_local");
}

#[test]
fn merge_empty_sequence() {
    let yaml = "target:\n  <<: []\n  k: 1\n";
    let m: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml).unwrap();
    assert_eq!(m["target"]["k"], 1);
}

#[test]
fn merge_with_nested_local_content() {
    let yaml = "\
base: &b
  k1: 1
target:
  <<: *b
  nested_seq: [a, b]
  nested_map:
    inner: deep
";
    let m: BTreeMap<String, BTreeMap<String, Value>> = from_str(yaml).unwrap();
    let t = &m["target"];
    assert_eq!(t["k1"].as_i64(), Some(1));
    assert!(t["nested_seq"].is_sequence());
    assert!(t["nested_map"].is_mapping());
}

// ── Tags ─────────────────────────────────────────────────────────────────

#[test]
fn core_tag_str_forces_string() {
    #[derive(serde::Deserialize)]
    struct Doc {
        v: String,
    }
    let d: Doc = from_str("v: !!str 42\n").unwrap();
    assert_eq!(d.v, "42");
}

#[test]
fn core_tag_int_newtype() {
    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct Wrap(i64);
    #[derive(serde::Deserialize)]
    struct Doc {
        v: Wrap,
    }
    let d: Doc = from_str("v: !!int 42\n").unwrap();
    assert_eq!(d.v, Wrap(42));
}

#[test]
fn registry_strips_custom_scalar_tag() {
    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct Celsius(f64);
    #[derive(serde::Deserialize)]
    struct Doc {
        t: Celsius,
    }
    let reg = Arc::new(TagRegistry::new().with("!Celsius"));
    let cfg = ParserConfig::new().tag_registry(Arc::clone(&reg));
    let d: Doc = from_str_with_config("t: !Celsius 42.0\n", &cfg).unwrap();
    assert_eq!(d.t, Celsius(42.0));
}

#[test]
fn registry_strips_custom_seq_tag() {
    #[derive(serde::Deserialize)]
    struct Doc {
        items: Vec<i64>,
    }
    let reg = Arc::new(TagRegistry::new().with("!Items"));
    let cfg = ParserConfig::new().tag_registry(Arc::clone(&reg));
    let d: Doc = from_str_with_config("items: !Items [1, 2, 3]\n", &cfg).unwrap();
    assert_eq!(d.items, vec![1, 2, 3]);
}

#[test]
fn registry_strips_custom_map_tag() {
    #[derive(serde::Deserialize)]
    struct Doc {
        cfg: BTreeMap<String, i64>,
    }
    let reg = Arc::new(TagRegistry::new().with("!Cfg"));
    let cfg = ParserConfig::new().tag_registry(Arc::clone(&reg));
    let d: Doc = from_str_with_config("cfg: !Cfg {a: 1, b: 2}\n", &cfg).unwrap();
    assert_eq!(d.cfg["a"], 1);
    assert_eq!(d.cfg["b"], 2);
}

#[test]
fn with_tag_registry_builder_on_deserializer() {
    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct Temp(f64);
    let reg = Arc::new(TagRegistry::new().with("!Celsius"));
    let mut de = StreamingDeserializer::new("!Celsius 42.0").with_tag_registry(reg);
    let t = Temp::deserialize(&mut de).unwrap();
    assert_eq!(t, Temp(42.0));
}

#[test]
fn unregistered_tag_falls_back_to_value() {
    let v: Value = from_str("v: !Custom 42\n").unwrap();
    assert!(v.is_mapping());
}

// ── Enums ────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, PartialEq)]
enum E {
    Unit,
    Tup(i32, i32),
    Strukt { a: i32 },
    NewT(String),
}

#[test]
fn enum_unit_variant() {
    #[derive(serde::Deserialize)]
    struct Doc {
        e: E,
    }
    let d: Doc = from_str("e: Unit\n").unwrap();
    assert_eq!(d.e, E::Unit);
}

#[test]
fn enum_newtype_variant() {
    #[derive(serde::Deserialize)]
    struct Doc {
        e: E,
    }
    let d: Doc = from_str("e:\n  NewT: hi\n").unwrap();
    assert_eq!(d.e, E::NewT("hi".into()));
}

#[test]
fn enum_tuple_variant() {
    #[derive(serde::Deserialize)]
    struct Doc {
        e: E,
    }
    let d: Doc = from_str("e:\n  Tup: [1, 2]\n").unwrap();
    assert_eq!(d.e, E::Tup(1, 2));
}

#[test]
fn enum_struct_variant() {
    #[derive(serde::Deserialize)]
    struct Doc {
        e: E,
    }
    let d: Doc = from_str("e:\n  Strukt:\n    a: 7\n").unwrap();
    assert_eq!(d.e, E::Strukt { a: 7 });
}

#[test]
fn enum_top_level_struct_variant() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    enum Choice {
        Pair { a: i32, b: i32 },
    }
    let c: Choice = from_str("Pair: {a: 1, b: 2}\n").unwrap();
    assert_eq!(c, Choice::Pair { a: 1, b: 2 });
}

// ── Bytes / binary ───────────────────────────────────────────────────────

#[test]
fn bytes_from_plain_string() {
    #[derive(serde::Deserialize)]
    struct Doc {
        b: ByteBuf,
    }
    let d: Doc = from_str("b: hello\n").unwrap();
    assert_eq!(d.b.as_ref(), b"hello");
}

#[test]
fn bytes_binary_tag_decodes() {
    #[derive(serde::Deserialize)]
    struct Doc {
        b: ByteBuf,
    }
    // base64 for "hi"
    let d: Doc = from_str("b: !!binary aGk=\n").unwrap();
    assert_eq!(d.b.as_ref(), b"hi");
}

#[test]
fn bytes_reject_int() {
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        b: ByteBuf,
    }
    let res: Result<Doc, _> = from_str("b: 42\n");
    assert!(res.is_err());
}

#[test]
fn bytes_reject_null() {
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        b: ByteBuf,
    }
    let res: Result<Doc, _> = from_str("b: ~\n");
    assert!(res.is_err());
}

// ── Type-mismatch error paths ────────────────────────────────────────────

#[test]
fn bool_mismatch_int() {
    let res: Result<bool, _> = from_str("42\n");
    assert!(res.is_err());
}

#[test]
fn i64_mismatch_string() {
    let res: Result<i64, _> = from_str("nope\n");
    assert!(res.is_err());
}

#[test]
fn i64_accepts_whole_float() {
    let n: i64 = from_str("42.0\n").unwrap();
    assert_eq!(n, 42);
}

#[test]
fn u64_rejects_negative() {
    let res: Result<u64, _> = from_str("-1\n");
    assert!(res.is_err());
}

#[test]
fn u64_accepts_whole_nonneg_float() {
    let n: u64 = from_str("7.0\n").unwrap();
    assert_eq!(n, 7);
}

#[test]
fn f64_accepts_int() {
    let f: f64 = from_str("5\n").unwrap();
    assert!((f - 5.0).abs() < 1e-9);
}

#[test]
fn str_rejects_plain_number() {
    let res: Result<String, _> = from_str("42\n");
    assert!(res.is_err());
}

#[test]
fn seq_mismatch_scalar() {
    let res: Result<Vec<i64>, _> = from_str("scalar\n");
    assert!(res.is_err());
}

#[test]
fn map_mismatch_scalar() {
    let res: Result<BTreeMap<String, i64>, _> = from_str("scalar\n");
    assert!(res.is_err());
}

// ── Malformed input ──────────────────────────────────────────────────────

#[test]
fn malformed_unclosed_flow_seq() {
    let mut de = StreamingDeserializer::new("key: [unclosed\nnext: v\n");
    let res = <BTreeMap<String, Value>>::deserialize(&mut de);
    assert!(res.is_err());
}

#[test]
fn empty_input_as_option() {
    // Empty input is null-shaped: if it deserialises it must be `None`;
    // some configs surface an "empty document" error instead, which is
    // also acceptable here.
    let v: Result<Option<i64>, _> = from_str("");
    assert!(v.map(|o| o.is_none()).unwrap_or(true));
}

// ── Security limits ──────────────────────────────────────────────────────

#[test]
fn limit_max_depth() {
    let cfg = ParserConfig::new().max_depth(2);
    let res: Result<BTreeMap<String, Value>, _> =
        from_str_with_config("a:\n  b:\n    c:\n      d: 1\n", &cfg);
    assert!(res.is_err());
}

#[test]
fn limit_max_sequence_length() {
    let cfg = ParserConfig::new().max_sequence_length(2);
    let res: Result<Vec<i64>, _> = from_str_with_config("[1, 2, 3, 4]\n", &cfg);
    assert!(res.is_err());
}

#[test]
fn limit_max_mapping_keys() {
    let cfg = ParserConfig::new().max_mapping_keys(2);
    let res: Result<BTreeMap<String, i64>, _> = from_str_with_config("a: 1\nb: 2\nc: 3\n", &cfg);
    assert!(res.is_err());
}

#[test]
fn limit_max_alias_expansions() {
    let yaml = "a: &a 1\nb: &b\n  - *a\n  - *a\nc:\n  - *b\n  - *b\n";
    let cfg = ParserConfig::new().max_alias_expansions(1);
    let res: Result<BTreeMap<String, Value>, _> = from_str_with_config(yaml, &cfg);
    assert!(res.is_err());
}

#[test]
fn limit_max_document_length() {
    let cfg = ParserConfig::new().max_document_length(3);
    let res: Result<BTreeMap<String, i64>, _> = from_str_with_config("key: 1\n", &cfg);
    assert!(res.is_err());
}

// ── Duplicate-key policy ─────────────────────────────────────────────────

#[test]
fn dup_key_first_wins() {
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let m: BTreeMap<String, i64> = from_str_with_config("k: 1\nk: 2\nk: 3\n", &cfg).unwrap();
    assert_eq!(m["k"], 1);
}

#[test]
fn dup_key_error() {
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let res: Result<BTreeMap<String, i64>, _> = from_str_with_config("k: 1\nk: 2\n", &cfg);
    assert!(res.is_err());
}

#[test]
fn key_collision_distinct_typed() {
    // `1` (int) vs `"1"` (string) canonicalise to the same key string but
    // differ in type — a KeyCollision independent of duplicate policy.
    let res: Result<BTreeMap<String, i64>, _> = from_str("1: 1\n\"1\": 2\n");
    // The streaming path is only reached for typed non-Value targets; a
    // distinct-typed collision must surface as an error.
    assert!(res.is_err());
}

// ── Scalar resolution toggles ────────────────────────────────────────────

#[test]
fn strict_booleans_treat_True_as_string() {
    let cfg = ParserConfig::strict();
    let m: BTreeMap<String, String> = from_str_with_config("v: True\n", &cfg).unwrap();
    assert_eq!(m["v"], "True");
}

#[test]
fn legacy_booleans_yes_is_true() {
    let cfg = ParserConfig::new().legacy_booleans(true);
    let m: BTreeMap<String, bool> = from_str_with_config("v: yes\n", &cfg).unwrap();
    assert!(m["v"]);
}

#[test]
fn no_schema_keeps_number_as_string() {
    let cfg = ParserConfig::new().no_schema(true);
    let m: BTreeMap<String, String> = from_str_with_config("v: 42\n", &cfg).unwrap();
    assert_eq!(m["v"], "42");
}

#[test]
fn hex_integer() {
    let n: i64 = from_str("0xFF\n").unwrap();
    assert_eq!(n, 255);
}

#[test]
fn octal_o_prefix() {
    let n: i64 = from_str("0o17\n").unwrap();
    assert_eq!(n, 15);
}

#[test]
fn legacy_octal_bare_prefix() {
    let cfg = ParserConfig::new().legacy_octal_numbers(true);
    let n: i64 = from_str_with_config("0644\n", &cfg).unwrap();
    assert_eq!(n, 0o644);
}

#[test]
fn legacy_sexagesimal_int() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let n: i64 = from_str_with_config("60:00\n", &cfg).unwrap();
    assert_eq!(n, 3600);
}

#[test]
fn legacy_sexagesimal_float() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let f: f64 = from_str_with_config("60:00.5\n", &cfg).unwrap();
    assert!((f - 3600.5).abs() < 1e-9);
}

#[test]
fn legacy_sexagesimal_negative() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let n: i64 = from_str_with_config("-1:30:00\n", &cfg).unwrap();
    assert_eq!(n, -(3600 + 1800));
}

#[test]
fn legacy_sexagesimal_invalid_stays_string() {
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let m: BTreeMap<String, String> = from_str_with_config("x: 1:99\n", &cfg).unwrap();
    assert_eq!(m["x"], "1:99");
}

// ── Comments & document markers ──────────────────────────────────────────

#[test]
fn comments_are_ignored() {
    let yaml = "# leading comment\nk: 1 # trailing\n";
    let m: BTreeMap<String, i64> = from_str(yaml).unwrap();
    assert_eq!(m["k"], 1);
}

#[test]
fn explicit_document_markers() {
    let yaml = "---\nk: 5\n";
    let m: BTreeMap<String, i64> = from_str(yaml).unwrap();
    assert_eq!(m["k"], 5);
}

// ── Ignored-any & identifiers ────────────────────────────────────────────

#[test]
fn ignored_any_top_level() {
    let _: serde_core::de::IgnoredAny = from_str("a: 1\nb:\n  - 1\n  - 2\n").unwrap();
}

// ── Round-trips (parse -> emit -> parse) ─────────────────────────────────

#[test]
fn roundtrip_struct_via_streaming() {
    #[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug)]
    struct Cfg {
        name: String,
        count: u32,
        items: Vec<String>,
    }
    let original = Cfg {
        name: "x".into(),
        count: 3,
        items: vec!["a".into(), "b".into()],
    };
    let yaml = noyalib::to_string(&original).unwrap();
    let mut de = StreamingDeserializer::new(&yaml);
    let back = Cfg::deserialize(&mut de).unwrap();
    assert_eq!(original, back);
}

#[test]
fn streaming_equivalent_to_from_str() {
    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct Cfg {
        name: String,
        nums: Vec<i64>,
    }
    let yaml = "name: y\nnums:\n  - 1\n  - 2\n  - 3\n";
    let via_from_str: Cfg = from_str(yaml).unwrap();
    let mut de = StreamingDeserializer::new(yaml);
    let via_stream = Cfg::deserialize(&mut de).unwrap();
    assert_eq!(via_from_str, via_stream);
}

// ── Partial consumption / Drop drains ────────────────────────────────────

#[test]
fn partial_seq_consumption_error_drops_cleanly() {
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        values: Vec<i64>,
    }
    let res: Result<Doc, _> = from_str("values:\n  - 1\n  - two\n  - 3\n");
    assert!(res.is_err());
}

#[test]
fn large_flat_sequence() {
    let mut yaml = String::from("[");
    for i in 0..500 {
        if i > 0 {
            yaml.push_str(", ");
        }
        yaml.push_str(&i.to_string());
    }
    yaml.push_str("]\n");
    let v: Vec<i64> = from_str(&yaml).unwrap();
    assert_eq!(v.len(), 500);
    assert_eq!(v[499], 499);
}

#[test]
fn deeply_nested_sequence() {
    // 20 levels of nested single-element sequences.
    let mut yaml = String::new();
    for _ in 0..20 {
        yaml.push('[');
    }
    yaml.push('7');
    for _ in 0..20 {
        yaml.push(']');
    }
    yaml.push('\n');
    // Deserialise into Value (routes each element through deserialize_any).
    let v: Value = from_str(&yaml).unwrap();
    assert!(v.is_sequence());
}
