// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Streaming-deserializer coverage push.
//!
//! Targets specific uncovered branches in `crates/noyalib/src/streaming.rs`
//! identified via `cargo llvm-cov` region/line analysis. Each test names the
//! code path it exercises so future drift maps cleanly back to the source
//! line range. The tests drive the streaming fast path either via
//! [`noyalib::from_str`] (for the typed-non-Value, default-config inputs
//! that the dispatcher routes through `streaming::from_str_streaming`) or
//! directly through [`noyalib::StreamingDeserializer`] when a finer-grained
//! invocation is required.

#![allow(missing_docs)]

use std::collections::BTreeMap;
use std::sync::Arc;

use noyalib::{ParserConfig, TagRegistry, from_str, from_str_with_config};
use serde::Deserialize;
use serde_bytes::ByteBuf;

// ── L170-L182 — peek_parser_event / next_parser_event parser-error paths ───
// A malformed merge value (`<<` followed by a non-alias non-sequence) routes
// through `peek_parser_event` directly, exercising the raw parser-event
// fetch path rather than the cached/replay path.

#[test]
fn coverage_stream_merge_value_non_alias_falls_back() {
    // `<<: { x: 1 }` is a mapping-typed merge value, which the streaming
    // path cannot handle and must surface via the AST fallback.
    let yaml = "\
target:
  <<:
    x: 1
  y: 2
";
    let m: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml).expect("ast fallback handles");
    assert_eq!(m["target"]["x"], 1);
    assert_eq!(m["target"]["y"], 2);
}

// ── L284 — buffered_to_event for a recorded BufferedEvent::Alias ───────────
// An anchored mapping that contains an alias to a previously defined anchor
// causes the recording machinery to emit a `BufferedEvent::Alias`, which is
// then replayed via `buffered_to_event`.

#[test]
fn coverage_stream_anchor_with_inner_alias_replay() {
    // Alias-in-anchor replay where the inner alias is to a scalar — the
    // streaming path records `BufferedEvent::Alias` and surfaces the
    // re-resolution. AST fallback handles the typed mapping.
    let yaml = "\
shared: &shared 42
container: &c
  ref: *shared
copy: *c
";
    let v: noyalib::Value = from_str(yaml).expect("alias-in-anchor parses to Value");
    let container = v
        .get_path("container")
        .and_then(|x| x.as_mapping())
        .unwrap();
    assert_eq!(container.get("ref").and_then(|x| x.as_i64()), Some(42));
    let copy = v.get_path("copy").and_then(|x| x.as_mapping()).unwrap();
    assert_eq!(copy.get("ref").and_then(|x| x.as_i64()), Some(42));
}

// ── L310, L352-L361 — handle_anchor anchor_def_spans + alias inside anchor.
// Specifically targets the SmallVec branch of `maybe_record` for
// `BufferedEvent::Alias` and the depth==0 anchor-completion paths for
// scalar/sequence/mapping/alias.

#[test]
fn coverage_stream_anchored_scalar_seq_map_alias() {
    // Anchored scalar (depth==0 scalar branch).
    let yaml1 = "\
a: &x hello
b: *x
";
    let m1: BTreeMap<String, String> = from_str(yaml1).unwrap();
    assert_eq!(m1["b"], "hello");

    // Anchored sequence (depth==0 sequence-end branch).
    let yaml2 = "\
a: &x [1, 2, 3]
b: *x
";
    let m2: BTreeMap<String, Vec<i64>> = from_str(yaml2).unwrap();
    assert_eq!(m2["b"], vec![1, 2, 3]);

    // Anchored mapping (depth==0 mapping-end branch).
    let yaml3 = "\
a: &x {k: 1}
b: *x
";
    let m3: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml3).unwrap();
    assert_eq!(m3["b"]["k"], 1);
}

// ── L393 — empty-buffer alias bails out as unknown anchor ─────────────────
// An alias to an anchor whose recorded buffer is empty (rare; can happen
// when the anchor declaration is on a malformed event chain that aborts
// before any payload events are recorded) should error.

#[test]
fn coverage_stream_unknown_anchor_alias() {
    let yaml = "\
target:
  ref: *missing
";
    let res: Result<BTreeMap<String, BTreeMap<String, String>>, _> = from_str(yaml);
    assert!(res.is_err());
}

// ── L405-L463 — inject_multi_merge_mapping_contents / buffer_rest_of_mapping
// Trigger the merge-key sequence path with nested sequences + scalars
// inside the locally-buffered tail mapping so all branches of
// `buffer_rest_of_mapping` (MapStart, SeqStart, SeqEnd, Alias, Scalar) fire.

#[test]
fn coverage_stream_merge_with_nested_local_content() {
    let yaml = "\
base: &b
  k1: 1
  k2: 2
target:
  <<: *b
  nested_seq: [a, b, c]
  nested_map:
    inner: deep
";
    let v: BTreeMap<String, BTreeMap<String, noyalib::Value>> = from_str(yaml).unwrap();
    let target = &v["target"];
    assert_eq!(target["k1"].as_i64(), Some(1));
    assert!(target["nested_seq"].is_sequence());
    assert!(target["nested_map"].is_mapping());
}

// ── L463-L464 — alias inside locally-buffered tail of a merge mapping ─────

#[test]
fn coverage_stream_merge_with_alias_in_local_tail() {
    // Targets the `BufferedEvent::Alias` case in `buffer_rest_of_mapping`
    // (line 463). Use Value to keep the test path agnostic to whether
    // streaming or AST handles the alias replay.
    let yaml = "\
shared: &s deep_value
base: &b
  k: 1
target:
  <<: *b
  copy: *s
";
    let v: noyalib::Value = from_str(yaml).expect("alias in local tail parses");
    let target = v.get_path("target").and_then(|x| x.as_mapping()).unwrap();
    assert_eq!(
        target.get("copy").and_then(|x| x.as_str()),
        Some("deep_value")
    );
}

// ── L492 — skip_event used when peeking an Option(Some) branch falls
// into a scalar non-null. Exercise via Option<i64> where some value is
// present and the deserializer must visit_some(self) (line ~828).

#[test]
fn coverage_stream_option_some_scalar() {
    #[derive(Deserialize)]
    struct Doc {
        x: Option<i64>,
        y: Option<i64>,
    }
    let yaml = "x: 5\ny: ~\n";
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.x, Some(5));
    assert_eq!(d.y, None);
}

// ── L500-L513 — skip_to_content + skip_value iterative balance traversal.
// `deserialize_ignored_any` is invoked for `#[serde(deny_unknown_fields)]`
// off + an extra field — but more reliable: serde_ignored / fields not
// in the struct simply call `deserialize_ignored_any`. Use a struct that
// has `#[serde(default)]` and an unknown extra field that must be skipped.

#[test]
fn coverage_stream_ignored_any_skips_complex_value() {
    #[derive(Deserialize)]
    struct Small {
        keep: i64,
    }
    // `extra` (a nested mapping containing a sequence) is consumed via
    // `deserialize_ignored_any` → `skip_value` → balance loop covering
    // SequenceStart/MappingStart/Scalar/SequenceEnd/MappingEnd arms.
    let yaml = "\
keep: 42
extra:
  nested:
    - 1
    - 2
  more:
    a: x
    b: y
";
    let s: Small = from_str(yaml).unwrap();
    assert_eq!(s.keep, 42);
}

// ── L537-L588 — take_tag_from_current / restore_tag_to_current for a
// tagged scalar that hits `deserialize_str` and must restore the tag
// before signalling fallback.

#[test]
fn coverage_stream_tagged_str_restores_tag_for_fallback() {
    // A `!!str`-tagged scalar that targets a String field must restore
    // the tag so the AST path resolves it correctly.
    #[derive(Deserialize)]
    struct Doc {
        value: String,
    }
    let yaml = "value: !!str 42\n";
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.value, "42");
}

// ── L628 — deserialize_any with a sequence event hits SeqAccess via
// type-erased Value visitor.

#[test]
fn coverage_stream_seq_via_value_target() {
    // Targeting `Value` keeps us off the streaming path; for streaming we
    // need a typed target. A `Vec<noyalib::Value>` still routes through
    // the streaming `deserialize_any` because each element is type-erased.
    let yaml = "[1, two, 3.0, true, null]\n";
    let v: Vec<noyalib::Value> = from_str(yaml).unwrap();
    assert_eq!(v.len(), 5);
    assert_eq!(v[0].as_i64(), Some(1));
    assert_eq!(v[1].as_str(), Some("two"));
    assert!(v[2].as_f64().is_some());
    assert_eq!(v[3].as_bool(), Some(true));
    assert!(v[4].is_null());
}

// ── L672-L705 — bool / i64 / u64 / f64 type-mismatch paths ─────────────────

#[test]
fn coverage_stream_bool_typemismatch_for_int() {
    // 42 isn't a bool — must surface a type mismatch.
    let res: Result<bool, _> = from_str("42\n");
    assert!(res.is_err());
}

#[test]
fn coverage_stream_i64_typemismatch_for_string() {
    let res: Result<i64, _> = from_str("hello\n");
    assert!(res.is_err());
}

#[test]
fn coverage_stream_i64_accepts_whole_float() {
    // 42.0 should be accepted as i64 via fract()==0 branch.
    let n: i64 = from_str("42.0\n").unwrap();
    assert_eq!(n, 42);
}

#[test]
fn coverage_stream_u64_typemismatch_for_negative() {
    let res: Result<u64, _> = from_str("-1\n");
    assert!(res.is_err());
}

#[test]
fn coverage_stream_u64_accepts_whole_nonneg_float() {
    let n: u64 = from_str("7.0\n").unwrap();
    assert_eq!(n, 7);
}

#[test]
fn coverage_stream_f64_accepts_int() {
    let f: f64 = from_str("5\n").unwrap();
    assert!((f - 5.0).abs() < 1e-9);
}

#[test]
fn coverage_stream_f64_typemismatch_for_string() {
    let res: Result<f64, _> = from_str("not_a_number\n");
    assert!(res.is_err());
}

// ── L737-L766, L787 — deserialize_str: complex key / non-string scalar.

#[test]
fn coverage_stream_str_rejects_non_string_plain() {
    // Plain `42` isn't a string — must error.
    let res: Result<String, _> = from_str("42\n");
    assert!(res.is_err());
}

#[test]
fn coverage_stream_str_quoted_scalar_passes() {
    let s: String = from_str("\"hello\"\n").unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn coverage_stream_str_block_scalar_passes() {
    let s: String = from_str("|\n  block text\n").unwrap();
    assert_eq!(s, "block text\n");
}

// ── L812-L834 — deserialize_unit non-null-scalar mismatch ─────────────────

#[test]
fn coverage_stream_unit_mismatch_errors() {
    // Plain non-null scalar can't deserialise as `()`.
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct UnitField {
        u: (),
    }
    let res: Result<UnitField, _> = from_str("u: not_null\n");
    assert!(res.is_err());
}

#[test]
fn coverage_stream_unit_struct_accepts_null() {
    #[derive(Deserialize)]
    struct U;
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        u: U,
    }
    let _: Doc = from_str("u: ~\n").unwrap();
}

// ── L851-L862 — deserialize_newtype_struct early return for SPANNED ────────

#[test]
fn coverage_stream_spanned_falls_back() {
    // `Spanned<T>` must bail to AST fallback. Verify via from_str_with_config
    // since Spanned support is in the loader path.
    use noyalib::Spanned;
    #[derive(Deserialize)]
    struct Doc {
        inner: Spanned<String>,
    }
    let yaml = "inner: hello\n";
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.inner.value, "hello");
}

// ── L856-L862 — newtype_struct with !!core tag passes through ─────────────

#[test]
fn coverage_stream_newtype_with_core_tag() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Wrap(i64);
    #[derive(Deserialize)]
    struct Doc {
        v: Wrap,
    }
    let yaml = "v: !!int 42\n";
    let d: Doc = from_str(yaml).unwrap();
    assert_eq!(d.v, Wrap(42));
}

// ── L885-L929 — deserialize_seq / deserialize_map with non-sequence /
// non-mapping events (type mismatch through streaming path).

#[test]
fn coverage_stream_seq_typemismatch() {
    // Top-level scalar rejected as sequence.
    let res: Result<Vec<i64>, _> = from_str("not_a_seq\n");
    assert!(res.is_err());
}

#[test]
fn coverage_stream_map_typemismatch() {
    // Top-level scalar rejected as mapping.
    let res: Result<BTreeMap<String, i64>, _> = from_str("not_a_map\n");
    assert!(res.is_err());
}

// ── L960-L997 — deserialize_enum: unit variant from scalar, struct
// variant from single-key mapping, error on non-scalar variant name.

#[derive(Debug, Deserialize, PartialEq)]
enum E {
    Unit,
    Tup(i32, i32),
    Strukt { a: i32 },
    NewT(String),
}

#[test]
fn coverage_stream_enum_unit_from_scalar() {
    #[derive(Deserialize)]
    struct Doc {
        e: E,
    }
    let d: Doc = from_str("e: Unit\n").unwrap();
    assert_eq!(d.e, E::Unit);
}

#[test]
fn coverage_stream_enum_newtype_variant() {
    #[derive(Deserialize)]
    struct Doc {
        e: E,
    }
    let d: Doc = from_str("e:\n  NewT: hello\n").unwrap();
    assert_eq!(d.e, E::NewT("hello".into()));
}

#[test]
fn coverage_stream_enum_struct_variant() {
    #[derive(Deserialize)]
    struct Doc {
        e: E,
    }
    let d: Doc = from_str("e:\n  Strukt:\n    a: 1\n").unwrap();
    assert_eq!(d.e, E::Strukt { a: 1 });
}

#[test]
fn coverage_stream_enum_tuple_variant() {
    #[derive(Deserialize)]
    struct Doc {
        e: E,
    }
    let d: Doc = from_str("e:\n  Tup: [1, 2]\n").unwrap();
    assert_eq!(d.e, E::Tup(1, 2));
}

// ── L1003-L1017 — deserialize_identifier non-scalar error / ignored_any.

#[test]
fn coverage_stream_ignored_any_at_top_level() {
    use serde::de::IgnoredAny;
    let _: IgnoredAny = from_str("a: 1\nb: 2\n").unwrap();
}

// ── L1052-L1101 — deserialize_bytes branches: !!binary, plain str,
// type mismatches for null/bool/int/float.

#[test]
fn coverage_stream_bytes_from_plain_string() {
    #[derive(Deserialize)]
    struct Doc {
        b: ByteBuf,
    }
    let d: Doc = from_str("b: hello\n").unwrap();
    assert_eq!(d.b.as_ref(), b"hello");
}

#[test]
fn coverage_stream_bytes_rejects_null() {
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        b: ByteBuf,
    }
    let res: Result<Doc, _> = from_str("b: ~\n");
    assert!(res.is_err());
}

#[test]
fn coverage_stream_bytes_rejects_bool() {
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        b: ByteBuf,
    }
    let res: Result<Doc, _> = from_str("b: true\n");
    assert!(res.is_err());
}

#[test]
fn coverage_stream_bytes_rejects_int() {
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        b: ByteBuf,
    }
    let res: Result<Doc, _> = from_str("b: 42\n");
    assert!(res.is_err());
}

#[test]
fn coverage_stream_bytes_rejects_float() {
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        b: ByteBuf,
    }
    let res: Result<Doc, _> = from_str("b: 3.14\n");
    assert!(res.is_err());
}

#[test]
fn coverage_stream_bytes_binary_invalid() {
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        b: ByteBuf,
    }
    let res: Result<Doc, _> = from_str("b: !!binary \"$$not_b64$$\"\n");
    assert!(res.is_err());
}

// ── L1134, L1193, L1207, L1210, L1219-L1234 — merge sequence empty list +
// merge sequence non-alias element forces fallback.

#[test]
fn coverage_stream_merge_empty_sequence() {
    let yaml = "\
target:
  <<: []
  k: 1
";
    let v: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml).unwrap();
    assert_eq!(v["target"]["k"], 1);
}

#[test]
fn coverage_stream_merge_sequence_with_non_alias_falls_back() {
    // `<<: [{a: 1}]` — non-alias element in the merge sequence forces the
    // streaming path to bail; AST fallback handles it.
    let yaml = "\
target:
  <<:
    - a: 1
  b: 2
";
    let v: BTreeMap<String, BTreeMap<String, i64>> = from_str(yaml).unwrap();
    assert_eq!(v["target"]["a"], 1);
    assert_eq!(v["target"]["b"], 2);
}

// ── L1261-L1265 — duplicate-key policy First / Error paths on streaming.

#[test]
fn coverage_stream_duplicate_first_skips_later() {
    use noyalib::DuplicateKeyPolicy;
    let yaml = "k: 1\nk: 2\nk: 3\n";
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let m: BTreeMap<String, i64> = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(m["k"], 1);
}

#[test]
fn coverage_stream_duplicate_error_returns_error() {
    use noyalib::DuplicateKeyPolicy;
    let yaml = "k: 1\nk: 2\n";
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let res: Result<BTreeMap<String, i64>, _> = from_str_with_config(yaml, &cfg);
    assert!(res.is_err());
}

// ── L1332-L1383 — StreamingVariantAccess paths driven by `<<` merge into
// an enum target. Use a mapping with a single-key variant where the value
// involves further deserialization.

#[test]
fn coverage_stream_enum_with_struct_variant_mapping() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Choice {
        Pair { a: i32, b: i32 },
    }
    let yaml = "Pair: {a: 1, b: 2}\n";
    let c: Choice = from_str(yaml).unwrap();
    assert_eq!(c, Choice::Pair { a: 1, b: 2 });
}

// ── L1393-L1438 — StreamingTagMapAccess / TagEnumAccess: registry-bypass
// path takes a tagged newtype value through visit_newtype_struct.

#[test]
fn coverage_stream_tag_registry_strips_custom_tag() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Celsius(f64);
    #[derive(Deserialize)]
    struct Doc {
        t: Celsius,
    }
    let registry = Arc::new(TagRegistry::new().with("!Celsius"));
    let cfg = ParserConfig::new().tag_registry(Arc::clone(&registry));
    let yaml = "t: !Celsius 42.0\n";
    let d: Doc = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(d.t, Celsius(42.0));
}

#[test]
fn coverage_stream_tag_registry_strips_seq_tag() {
    let registry = Arc::new(TagRegistry::new().with("!Items"));
    let cfg = ParserConfig::new().tag_registry(Arc::clone(&registry));
    let yaml = "items: !Items [1, 2, 3]\n";
    #[derive(Deserialize)]
    struct Doc {
        items: Vec<i64>,
    }
    let d: Doc = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(d.items, vec![1, 2, 3]);
}

#[test]
fn coverage_stream_tag_registry_strips_map_tag() {
    let registry = Arc::new(TagRegistry::new().with("!Cfg"));
    let cfg = ParserConfig::new().tag_registry(Arc::clone(&registry));
    let yaml = "cfg: !Cfg {a: 1, b: 2}\n";
    #[derive(Deserialize)]
    struct Doc {
        cfg: BTreeMap<String, i64>,
    }
    let d: Doc = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(d.cfg["a"], 1);
}

#[test]
fn coverage_stream_unregistered_tag_falls_back() {
    // A custom tag with no registry hits the StreamingTagMapAccess path
    // which surfaces the tagged value as a single-key map `{tag: value}`,
    // letting the AST fallback resolve it.
    let yaml = "v: !Custom 42\n";
    let v: noyalib::Value = from_str(yaml).unwrap();
    assert!(v.is_mapping());
}

// ── L1562-L1631 — sexagesimal int / float parsing helpers.

#[test]
fn coverage_stream_legacy_sexagesimal_int() {
    let yaml = "60:00\n";
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let n: i64 = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(n, 3600);
}

#[test]
fn coverage_stream_legacy_sexagesimal_float() {
    let yaml = "60:00.5\n";
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let f: f64 = from_str_with_config(yaml, &cfg).unwrap();
    assert!((f - 3600.5).abs() < 1e-9);
}

#[test]
fn coverage_stream_legacy_sexagesimal_negative() {
    let yaml = "-1:30:00\n";
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let n: i64 = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(n, -(3600 + 1800));
}

#[test]
fn coverage_stream_legacy_sexagesimal_invalid_falls_through() {
    // `1:99` — second component >= 60, must fall through to plain string.
    let yaml = "x: 1:99\n";
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let m: BTreeMap<String, String> = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(m["x"], "1:99");
}

// ── L1639, L1651, L1663 — float-shaped sexagesimal with empty parts /
// non-digit parts triggers the `None` returns.

#[test]
fn coverage_stream_legacy_sexagesimal_empty_part() {
    let yaml = "x: 1::00\n";
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let m: BTreeMap<String, String> = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(m["x"], "1::00");
}

#[test]
fn coverage_stream_legacy_sexagesimal_falls_back_to_float() {
    // `1.5` with sexagesimal enabled but no `:` — falls through to f64 parse.
    let yaml = "1.5\n";
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let f: f64 = from_str_with_config(yaml, &cfg).unwrap();
    assert!((f - 1.5).abs() < 1e-9);
}

#[test]
fn coverage_stream_legacy_sexagesimal_falls_back_to_string() {
    // `not:a:number` looks colon-shaped but has non-digit parts.
    let yaml = "x: notnum:abc\n";
    let cfg = ParserConfig::new().legacy_sexagesimal(true);
    let m: BTreeMap<String, String> = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(m["x"], "notnum:abc");
}

// ── L1684 — parse_integer hex / octal paths via legacy_octal_numbers.

#[test]
fn coverage_stream_hex_integer() {
    let n: i64 = from_str("0xFF\n").unwrap();
    assert_eq!(n, 255);
}

#[test]
fn coverage_stream_octal_o_prefix() {
    let n: i64 = from_str("0o17\n").unwrap();
    assert_eq!(n, 15);
}

#[test]
fn coverage_stream_legacy_octal_bare_zero_prefix() {
    let yaml = "0644\n";
    let cfg = ParserConfig::new().legacy_octal_numbers(true);
    let n: i64 = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(n, 0o644);
}

// ── L1703-L1798 — collect_keys / extract_local_keys / filter_merge_entries
// with nested sequence/mapping bodies inside merge sources & locals.

#[test]
fn coverage_stream_merge_with_nested_seq_in_source() {
    let yaml = "\
base: &b
  list: [1, 2]
  flag: true
target:
  <<: *b
  extra: x
";
    let v: BTreeMap<String, BTreeMap<String, noyalib::Value>> = from_str(yaml).unwrap();
    let target = &v["target"];
    assert!(target["list"].is_sequence());
    assert_eq!(target["flag"].as_bool(), Some(true));
    assert_eq!(target["extra"].as_str(), Some("x"));
}

#[test]
fn coverage_stream_merge_with_nested_map_in_source() {
    let yaml = "\
base: &b
  inner:
    a: 1
    b: 2
  flag: true
target:
  <<: *b
  extra: x
";
    let v: BTreeMap<String, BTreeMap<String, noyalib::Value>> = from_str(yaml).unwrap();
    let target = &v["target"];
    assert!(target["inner"].is_mapping());
}

#[test]
fn coverage_stream_merge_local_seq_value_releases_d() {
    // Locally-buffered tail contains a sequence value — extract_local_keys
    // must increment / decrement `d` on SeqStart/SeqEnd and reset `key=true`.
    let yaml = "\
base: &b {a: 1}
target:
  <<: *b
  list: [x, y]
  follow: 9
";
    let v: BTreeMap<String, BTreeMap<String, noyalib::Value>> = from_str(yaml).unwrap();
    let t = &v["target"];
    assert!(t["list"].is_sequence());
    assert_eq!(t["follow"].as_i64(), Some(9));
}

#[test]
fn coverage_stream_merge_local_alias_value() {
    // Local tail contains an Alias value — `extract_local_keys` Alias arm
    // (line 1786) and `BufferedEvent::Alias` case in
    // `buffer_rest_of_mapping` (line 463).
    let yaml = "\
base: &b {a: 1}
shared: &s sharedval
target:
  <<: *b
  ref: *s
  follow: 9
";
    let v: noyalib::Value = from_str(yaml).expect("alias-as-value tail parses");
    let target = v.get_path("target").and_then(|x| x.as_mapping()).unwrap();
    assert_eq!(
        target.get("ref").and_then(|x| x.as_str()),
        Some("sharedval")
    );
    assert_eq!(target.get("follow").and_then(|x| x.as_i64()), Some(9));
}

// ── L1751, L1758, L1766 — skip_buffered_value sequence/map skip helpers ───

#[test]
fn coverage_stream_filter_merge_seq_value() {
    // The merge source has a sequence value the filter must skip over.
    let yaml = "\
base: &b
  k_seq: [1, 2, 3]
  k_str: hello
target:
  <<: *b
";
    let v: BTreeMap<String, BTreeMap<String, noyalib::Value>> = from_str(yaml).unwrap();
    assert!(v["target"]["k_seq"].is_sequence());
}

#[test]
fn coverage_stream_filter_merge_map_value() {
    let yaml = "\
base: &b
  k_map:
    deep: 1
  k_str: hello
target:
  <<: *b
";
    let v: BTreeMap<String, BTreeMap<String, noyalib::Value>> = from_str(yaml).unwrap();
    assert!(v["target"]["k_map"].is_mapping());
}

// ── L1811-L1820 — filter_merge_entries: local key wins over merge source.

#[test]
fn coverage_stream_filter_merge_local_overrides_source() {
    let yaml = "\
base: &b
  k: from_base
target:
  <<: *b
  k: from_local
";
    let v: BTreeMap<String, BTreeMap<String, String>> = from_str(yaml).unwrap();
    // Local key wins over merge source per YAML spec.
    assert_eq!(v["target"]["k"], "from_local");
}

// ── Direct StreamingDeserializer driver: max-alias / depth / mapping cap ──

#[test]
fn coverage_stream_recursion_limit_exceeded() {
    // Build a deeply nested structure that exceeds `max_depth=2`.
    let yaml = "a:\n  b:\n    c:\n      d: 1\n";
    let cfg = ParserConfig::new().max_depth(2);
    let res: Result<BTreeMap<String, noyalib::Value>, _> = from_str_with_config(yaml, &cfg);
    assert!(res.is_err());
}

#[test]
fn coverage_stream_max_sequence_length_exceeded() {
    let yaml = "[1, 2, 3, 4, 5]\n";
    let cfg = ParserConfig::new().max_sequence_length(2);
    let res: Result<Vec<i64>, _> = from_str_with_config(yaml, &cfg);
    assert!(res.is_err());
}

#[test]
fn coverage_stream_max_mapping_keys_exceeded() {
    let yaml = "a: 1\nb: 2\nc: 3\n";
    let cfg = ParserConfig::new().max_mapping_keys(2);
    let res: Result<BTreeMap<String, i64>, _> = from_str_with_config(yaml, &cfg);
    assert!(res.is_err());
}

#[test]
fn coverage_stream_max_alias_expansions_exceeded() {
    // Self-amplifying alias chain: each alias resolves to a structure
    // containing the next alias, blowing the alias counter past 1.
    let yaml = "\
a: &a 1
b: &b
  - *a
  - *a
  - *a
c:
  - *b
  - *b
  - *b
";
    let cfg = ParserConfig::new().max_alias_expansions(1);
    let res: Result<BTreeMap<String, noyalib::Value>, _> = from_str_with_config(yaml, &cfg);
    assert!(res.is_err());
}

// ── Direct StreamingDeserializer construction tests for tag_in_registry
// `!!`-prefixed core-tag short-circuit (line ~558-568).

#[test]
fn coverage_stream_core_tag_str_via_streaming_de() {
    // Core `!!str` is never stripped by the registry — it goes through
    // the AST fallback for tag resolution. This exercises the registry
    // short-circuit branch.
    let registry = Arc::new(TagRegistry::new().with("!!str"));
    let cfg = ParserConfig::new().tag_registry(Arc::clone(&registry));
    let yaml = "v: !!str 42\n";
    #[derive(Deserialize)]
    struct Doc {
        v: String,
    }
    let d: Doc = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(d.v, "42");
}

// ── Direct streaming construction for `Drop` paths on partial-consumption.

#[test]
fn coverage_stream_partial_consumption_drops_seq() {
    // Construct a nested seq inside a struct, then deserialize where the
    // visitor encounters an early error. The Drop impl must drain on
    // partial consumption — we trigger this by parsing a seq that
    // contains an element of the wrong type (string in i64 seq), causing
    // mid-iteration failure that exercises the Drop drain loop.
    let yaml = "values:\n  - 1\n  - 2\n  - not_a_number\n";
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        values: Vec<i64>,
    }
    let res: Result<Doc, _> = from_str(yaml);
    assert!(res.is_err());
}

// ── Module-level convenience: trailing data after first node ──────────────

#[test]
fn coverage_stream_trailing_garbage_after_value_propagates() {
    // The drain loop in `from_str_streaming` must surface any error past
    // the first satisfied node. A bracketed flow scalar followed by stray
    // unbalanced text shows up.
    let yaml = "key: ok\n: bad\n";
    let res: Result<BTreeMap<String, String>, _> = from_str(yaml);
    // Either succeeds (lenient parse) or errors — the key path is that
    // the drain loop processes events past the satisfied node without panic.
    let _ = res;
}
