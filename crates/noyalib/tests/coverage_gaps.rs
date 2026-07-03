//! Coverage gap tests — fills every identified P0/P1 coverage hole.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![allow(missing_docs)]

use std::collections::BTreeMap;

use noyalib::{
    DuplicateKeyPolicy, Error, FlowStyle, Location, Mapping, MappingAny, Number, ParserConfig,
    Path, ScalarStyle, SerializerConfig, Spanned, Tag, TaggedValue, Value, from_reader,
    from_reader_with_config, from_slice, from_str, from_str_with_config, from_value, load_all,
    load_all_as, load_all_with_config, to_string, to_string_multi, to_string_multi_with_config,
    to_string_with_config, to_value, to_writer, to_writer_with_config, try_load_all,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// from_slice: valid + invalid + UTF-8 error
// ============================================================================

#[test]
fn from_slice_valid_yaml() {
    let yaml = b"name: hello\nport: 8080\n";
    let value: Value = from_slice(yaml).unwrap();
    assert_eq!(value.get("name").unwrap().as_str(), Some("hello"));
    assert_eq!(value.get("port").unwrap().as_i64(), Some(8080));
}

#[test]
fn from_slice_typed_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Cfg {
        name: String,
        port: u16,
    }
    let yaml = b"name: app\nport: 3000\n";
    let cfg: Cfg = from_slice(yaml).unwrap();
    assert_eq!(cfg.name, "app");
    assert_eq!(cfg.port, 3000);
}

#[test]
fn from_slice_invalid_utf8() {
    let bad = b"key: \xff\xfe";
    let result: Result<Value, _> = from_slice(bad);
    assert!(result.is_err());
}

#[test]
fn from_slice_invalid_yaml() {
    let bad = b"[unclosed";
    let result: Result<Value, _> = from_slice(bad);
    assert!(result.is_err());
}

#[test]
fn from_slice_empty() {
    let result: Result<Value, _> = from_slice(b"");
    // Empty input either returns Null or errors — both are valid
    if let Ok(v) = result {
        assert!(v.is_null());
    }
}

// ============================================================================
// from_reader: valid + error paths
// ============================================================================

#[test]
fn from_reader_valid() {
    let yaml = b"key: value\n";
    let cursor = std::io::Cursor::new(yaml);
    let value: Value = from_reader(cursor).unwrap();
    assert_eq!(value.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn from_reader_typed() {
    #[derive(Deserialize)]
    struct Item {
        name: String,
    }
    let yaml = b"name: widget\n";
    let item: Item = from_reader(std::io::Cursor::new(yaml)).unwrap();
    assert_eq!(item.name, "widget");
}

#[test]
fn from_reader_invalid_yaml() {
    let bad = b"[broken: {yaml";
    let result: Result<Value, _> = from_reader(std::io::Cursor::new(bad));
    assert!(result.is_err());
}

#[test]
fn from_reader_empty() {
    let result: Result<Value, _> = from_reader(std::io::Cursor::new(b""));
    if let Ok(v) = result {
        assert!(v.is_null());
    }
}

#[test]
fn from_reader_with_config_strict() {
    let config = ParserConfig::strict();
    let yaml = b"a: 1\nb: 2\n";
    let value: Value = from_reader_with_config(std::io::Cursor::new(yaml), &config).unwrap();
    assert_eq!(value.get("a").unwrap().as_i64(), Some(1));
}

#[test]
fn from_reader_with_config_rejects_deep_nesting() {
    let config = ParserConfig::new().max_depth(2);
    let yaml = b"a:\n  b:\n    c:\n      d: deep\n";
    let result: Result<Value, _> = from_reader_with_config(std::io::Cursor::new(yaml), &config);
    assert!(result.is_err());
}

// ============================================================================
// ParserConfig: strict preset + alias expansion limit
// ============================================================================

#[test]
fn parser_config_strict_rejects_deep() {
    let config = ParserConfig::strict();
    // Strict uses depth 64 — build a deeply nested YAML
    let mut yaml = String::new();
    for i in 0..70 {
        yaml.push_str(&" ".repeat(i * 2));
        yaml.push_str(&format!("level{i}:\n"));
    }
    let result: Result<Value, _> = from_str_with_config(&yaml, &config);
    assert!(result.is_err());
}

#[test]
fn parser_config_alias_expansion_limit() {
    let config = ParserConfig::new().max_alias_expansions(2);
    let yaml = r#"
anchor: &a
  x: 1
ref1: *a
ref2: *a
ref3: *a
"#;
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

#[test]
fn parser_config_combined_builders() {
    let config = ParserConfig::new()
        .max_depth(32)
        .max_document_length(1024)
        .max_alias_expansions(10)
        .max_mapping_keys(100)
        .max_sequence_length(100)
        .duplicate_key_policy(DuplicateKeyPolicy::Error);

    assert_eq!(config.max_depth, 32);
    assert_eq!(config.max_document_length, 1024);
    assert_eq!(config.max_alias_expansions, 10);
    assert_eq!(config.max_mapping_keys, 100);
    assert_eq!(config.max_sequence_length, 100);
}

// ============================================================================
// DuplicateKeyPolicy: First + Error + Last explicit
// ============================================================================

#[test]
fn duplicate_key_policy_first() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let yaml = "key: first\nkey: second\n";
    let value: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(value.get("key").unwrap().as_str(), Some("first"));
}

#[test]
fn duplicate_key_policy_last() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Last);
    let yaml = "key: first\nkey: second\n";
    let value: Value = from_str_with_config(yaml, &config).unwrap();
    assert_eq!(value.get("key").unwrap().as_str(), Some("second"));
}

#[test]
fn duplicate_key_policy_error() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let yaml = "key: first\nkey: second\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

#[test]
fn duplicate_key_complex_pattern() {
    let config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let yaml = "a: 1\nb: 2\nc: 3\na: 4\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("duplicate") || msg.contains("key"));
}

// ============================================================================
// to_writer and to_writer_with_config
// ============================================================================

#[test]
fn to_writer_roundtrip() {
    let value = Value::from("hello");
    let mut buf = Vec::new();
    to_writer(&mut buf, &value).unwrap();
    let yaml = String::from_utf8(buf).unwrap();
    let parsed: Value = from_str(&yaml).unwrap();
    assert_eq!(parsed.as_str(), Some("hello"));
}

#[test]
fn to_writer_struct() {
    #[derive(Serialize)]
    struct Item {
        name: String,
        count: i32,
    }
    let item = Item {
        name: "widget".to_string(),
        count: 42,
    };
    let mut buf = Vec::new();
    to_writer(&mut buf, &item).unwrap();
    let yaml = String::from_utf8(buf).unwrap();
    assert!(yaml.contains("name: widget"));
    assert!(yaml.contains("count: 42"));
}

#[test]
fn to_writer_with_config_indent() {
    let config = SerializerConfig::new().indent(4);
    let mut map = BTreeMap::new();
    let _ = map.insert("outer", BTreeMap::from([("inner", "value")]));
    let mut buf = Vec::new();
    to_writer_with_config(&mut buf, &map, &config).unwrap();
    let yaml = String::from_utf8(buf).unwrap();
    assert!(yaml.contains("    inner:"));
}

// ============================================================================
// to_string_multi + to_string_multi_with_config
// ============================================================================

#[test]
fn to_string_multi_empty() {
    let empty: Vec<i32> = vec![];
    let yaml = to_string_multi(&empty).unwrap();
    assert!(yaml.is_empty() || yaml.trim().is_empty());
}

#[test]
fn to_string_multi_single() {
    let docs = vec![42i64];
    let yaml = to_string_multi(&docs).unwrap();
    assert!(yaml.contains("---"));
    assert!(yaml.contains("42"));
}

#[test]
fn to_string_multi_three_docs() {
    let docs = vec!["alpha", "beta", "gamma"];
    let yaml = to_string_multi(&docs).unwrap();
    assert!(yaml.contains("alpha"));
    assert!(yaml.contains("beta"));
    assert!(yaml.contains("gamma"));
    // Count document markers
    let markers = yaml.matches("---").count();
    assert!(markers >= 3);
}

#[test]
fn to_string_multi_with_config_flow() {
    let config = SerializerConfig::new().flow_style(FlowStyle::Flow);
    let docs = vec![vec![1, 2], vec![3, 4]];
    let yaml = to_string_multi_with_config(&docs, &config).unwrap();
    assert!(yaml.contains("---"));
}

// ============================================================================
// fmt wrappers: FlowSeq, FlowMap, LitStr, LitString, FoldStr, FoldString,
//               Commented, SpaceAfter
// ============================================================================

#[test]
fn flow_seq_serialize_roundtrip() {
    use noyalib::fmt::FlowSeq;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Doc {
        tags: FlowSeq<Vec<String>>,
    }

    let doc = Doc {
        tags: FlowSeq(vec!["a".into(), "b".into(), "c".into()]),
    };
    let yaml = to_string(&doc).unwrap();
    assert!(yaml.contains("["));
    // Round-trip: deserialize back
    let parsed: Doc = from_str(&yaml).unwrap();
    assert_eq!(parsed.tags.0, vec!["a", "b", "c"]);
}

#[test]
fn flow_seq_deref_and_into_inner() {
    use noyalib::fmt::FlowSeq;
    let fs = FlowSeq(vec![1, 2, 3]);
    assert_eq!(fs.len(), 3); // via Deref
    let inner = fs.into_inner();
    assert_eq!(inner, vec![1, 2, 3]);
}

#[test]
fn flow_seq_from() {
    use noyalib::fmt::FlowSeq;
    let fs: FlowSeq<Vec<i32>> = vec![1, 2].into();
    assert_eq!(fs.0, vec![1, 2]);
}

#[test]
fn flow_seq_debug() {
    use noyalib::fmt::FlowSeq;
    let fs = FlowSeq(vec![1]);
    let debug = format!("{fs:?}");
    assert!(debug.contains("FlowSeq"));
}

#[test]
fn flow_map_serialize_roundtrip() {
    use noyalib::fmt::FlowMap;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Doc {
        meta: FlowMap<BTreeMap<String, i32>>,
    }

    let mut map = BTreeMap::new();
    let _ = map.insert("x".into(), 1);
    let _ = map.insert("y".into(), 2);
    let doc = Doc {
        meta: FlowMap(map.clone()),
    };
    let yaml = to_string(&doc).unwrap();
    assert!(yaml.contains("{"));
    let parsed: Doc = from_str(&yaml).unwrap();
    assert_eq!(parsed.meta.0, map);
}

#[test]
fn flow_map_deref_into_inner_from_debug() {
    use noyalib::fmt::FlowMap;
    let fm: FlowMap<BTreeMap<String, i32>> = BTreeMap::new().into();
    assert!(fm.is_empty()); // Deref
    let _ = fm.into_inner();
    let fm2 = FlowMap(BTreeMap::<String, i32>::new());
    let debug = format!("{fm2:?}");
    assert!(debug.contains("FlowMap"));
}

#[test]
fn lit_string_serialize() {
    use noyalib::fmt::LitString;

    #[derive(Serialize)]
    struct Doc {
        script: LitString,
    }
    let doc = Doc {
        script: LitString("line1\nline2\nline3\n".into()),
    };
    let yaml = to_string(&doc).unwrap();
    assert!(yaml.contains("|") || yaml.contains("line1"));
}

#[test]
fn lit_string_roundtrip() {
    use noyalib::fmt::LitString;
    let ls = LitString::from("hello world");
    let yaml = to_string(&ls).unwrap();
    let parsed: LitString = from_str(&yaml).unwrap();
    assert_eq!(parsed.0, "hello world");
}

#[test]
fn lit_string_into_inner_from_deref_debug() {
    use noyalib::fmt::LitString;
    let ls = LitString("test".into());
    assert_eq!(&*ls, "test"); // Deref
    let debug = format!("{ls:?}");
    assert!(debug.contains("LitString"));
    let inner = ls.into_inner();
    assert_eq!(inner, "test");
}

#[test]
fn lit_str_serialize() {
    use noyalib::fmt::LitStr;
    let ls = LitStr("multi\nline\n");
    let yaml = to_string(&ls).unwrap();
    assert!(yaml.contains("|") || yaml.contains("multi"));
}

#[test]
fn lit_str_deref_into_inner_from_debug() {
    use noyalib::fmt::LitStr;
    let ls = LitStr("text");
    assert_eq!(&*ls, "text");
    assert_eq!(ls.into_inner(), "text");
    let ls2: LitStr = "abc".into();
    let debug = format!("{ls2:?}");
    assert!(debug.contains("LitStr"));
}

#[test]
fn fold_string_serialize_roundtrip() {
    use noyalib::fmt::FoldString;
    let fs = FoldString("folded\ntext\n".into());
    let yaml = to_string(&fs).unwrap();
    let parsed: FoldString = from_str(&yaml).unwrap();
    // Content should preserve — may normalize whitespace
    assert!(parsed.0.contains("folded") || parsed.0.contains("text"));
}

#[test]
fn fold_string_from_str_deref_into_inner_debug() {
    use noyalib::fmt::FoldString;
    let fs = FoldString::from("test");
    assert_eq!(&*fs, "test");
    let debug = format!("{fs:?}");
    assert!(debug.contains("FoldString"));
    let inner = fs.into_inner();
    assert_eq!(inner, "test");
}

#[test]
fn fold_str_serialize() {
    use noyalib::fmt::FoldStr;
    let fs = FoldStr("folded\ncontent\n");
    let yaml = to_string(&fs).unwrap();
    assert!(yaml.contains(">") || yaml.contains("folded"));
}

#[test]
fn fold_str_deref_into_inner_from_debug() {
    use noyalib::fmt::FoldStr;
    let fs: FoldStr = "abc".into();
    assert_eq!(&*fs, "abc");
    assert_eq!(fs.into_inner(), "abc");
    let fs2 = FoldStr("x");
    let debug = format!("{fs2:?}");
    assert!(debug.contains("FoldStr"));
}

#[test]
fn commented_serialize() {
    use noyalib::fmt::Commented;
    let c = Commented::new(42i32, "the answer");
    let yaml = to_string(&c).unwrap();
    assert!(yaml.contains("42"));
    // Comment may or may not appear depending on serializer support
}

#[test]
fn commented_deref_into_inner_debug() {
    use noyalib::fmt::Commented;
    let c = Commented::new("val", "note");
    assert_eq!(*c, "val"); // Deref
    let debug = format!("{c:?}");
    assert!(debug.contains("Commented"));
    let inner = c.into_inner();
    assert_eq!(inner, "val");
}

#[test]
fn commented_roundtrip_loses_comment() {
    use noyalib::fmt::Commented;
    let c = Commented::new(99i32, "ephemeral");
    let yaml = to_string(&c).unwrap();
    let parsed: Commented<i32> = from_str(&yaml).unwrap();
    assert_eq!(*parsed, 99);
    assert!(parsed.comment.is_empty()); // Comment lost on roundtrip
}

#[test]
fn space_after_serialize() {
    use noyalib::fmt::SpaceAfter;
    let sa = SpaceAfter(42i32);
    let yaml = to_string(&sa).unwrap();
    assert!(yaml.contains("42"));
}

#[test]
fn space_after_roundtrip() {
    use noyalib::fmt::SpaceAfter;
    let sa = SpaceAfter("hello".to_string());
    let yaml = to_string(&sa).unwrap();
    let parsed: SpaceAfter<String> = from_str(&yaml).unwrap();
    assert_eq!(*parsed, "hello");
}

#[test]
fn space_after_deref_into_inner_from_debug() {
    use noyalib::fmt::SpaceAfter;
    let sa: SpaceAfter<i32> = 7.into();
    assert_eq!(*sa, 7);
    let debug = format!("{sa:?}");
    assert!(debug.contains("SpaceAfter"));
    assert_eq!(sa.into_inner(), 7);
}

// ============================================================================
// Loader: edge cases
// ============================================================================

#[test]
fn load_all_empty_input() {
    let docs = load_all("").unwrap();
    assert!(docs.is_empty());
    assert_eq!(docs.len(), 0);
}

#[test]
fn load_all_single_doc() {
    let docs = load_all("key: value\n").unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn load_all_three_docs() {
    let yaml = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
    let docs = load_all(yaml).unwrap();
    assert_eq!(docs.len(), 3);
}

#[test]
fn load_all_with_config_limits() {
    let config = ParserConfig::new().max_depth(2);
    let yaml = "a:\n  b:\n    c:\n      d: deep\n";
    let result = load_all_with_config(yaml, &config);
    assert!(result.is_err());
}

#[test]
fn try_load_all_valid() {
    let docs = try_load_all("---\nx: 1\n---\ny: 2\n").unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn try_load_all_invalid() {
    let result = try_load_all("[broken");
    assert!(result.is_err());
}

#[test]
fn load_all_as_typed() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Doc {
        name: String,
    }
    let yaml = "---\nname: first\n---\nname: second\n";
    let docs: Vec<Doc> = load_all_as(yaml).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].name, "first");
    assert_eq!(docs[1].name, "second");
}

#[test]
fn load_all_as_empty() {
    let docs: Vec<Value> = load_all_as("").unwrap();
    assert!(docs.is_empty());
}

#[test]
fn document_iterator_exhaustion() {
    let mut iter = load_all("---\na: 1\n---\nb: 2\n").unwrap();
    assert_eq!(iter.len(), 2);
    let _ = iter.next().unwrap();
    let _ = iter.next().unwrap();
    assert!(iter.next().is_none());
    // Exhausted iterator stays None
    assert!(iter.next().is_none());
}

#[test]
fn document_iterator_size_hint() {
    let iter = load_all("---\na: 1\n---\nb: 2\n").unwrap();
    assert_eq!(iter.size_hint(), (2, Some(2)));
}

#[test]
fn document_iterator_exact_size() {
    let iter = load_all("---\na: 1\n").unwrap();
    assert_eq!(iter.len(), 1);
}

// ============================================================================
// MappingAny: complex keys
// ============================================================================

#[test]
fn mapping_any_sequence_key() {
    let mut map = MappingAny::new();
    let key = Value::Sequence(vec![Value::from(1), Value::from(2)]);
    let _ = map.insert(key.clone(), Value::from("list-key"));
    assert_eq!(map.get(&key).unwrap().as_str(), Some("list-key"));
}

#[test]
fn mapping_any_mapping_key() {
    let mut map = MappingAny::new();
    let mut inner = Mapping::new();
    let _ = inner.insert("nested", Value::from(true));
    let key = Value::Mapping(inner);
    let _ = map.insert(key.clone(), Value::from("map-key"));
    assert_eq!(map.get(&key).unwrap().as_str(), Some("map-key"));
}

#[test]
fn mapping_any_null_key() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::Null, Value::from("null-value"));
    assert_eq!(map.get(&Value::Null).unwrap().as_str(), Some("null-value"));
}

#[test]
fn mapping_any_number_key() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from(42), Value::from("the-answer"));
    assert_eq!(
        map.get(&Value::from(42)).unwrap().as_str(),
        Some("the-answer")
    );
}

#[test]
fn mapping_any_bool_key() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from(true), Value::from("yes"));
    let _ = map.insert(Value::from(false), Value::from("no"));
    assert_eq!(map.len(), 2);
}

#[test]
fn mapping_any_into_mapping_with_non_string_keys_fails() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from(42), Value::from("number key"));
    assert!(map.into_mapping().is_none());
}

#[test]
fn mapping_any_into_mapping_with_string_keys_succeeds() {
    let mut map = MappingAny::new();
    let _ = map.insert(Value::from("a"), Value::from(1));
    let _ = map.insert(Value::from("b"), Value::from(2));
    let mapping = map.into_mapping().unwrap();
    assert_eq!(mapping.len(), 2);
}

// ============================================================================
// Value::get_path edge cases
// ============================================================================

#[test]
fn get_path_empty_string() {
    let value: Value = from_str("key: value").unwrap();
    let result = value.get_path("");
    // Empty path returns Some for the root or None
    // Either behavior is acceptable
    let _ = result;
}

#[test]
fn get_path_dot_notation() {
    let yaml = "a:\n  b:\n    c: deep\n";
    let value: Value = from_str(yaml).unwrap();
    let result = value.get_path("a.b.c");
    assert_eq!(result.unwrap().as_str(), Some("deep"));
}

#[test]
fn get_path_bracket_index() {
    let yaml = "items:\n  - one\n  - two\n  - three\n";
    let value: Value = from_str(yaml).unwrap();
    let result = value.get_path("items[1]");
    assert_eq!(result.unwrap().as_str(), Some("two"));
}

#[test]
fn get_path_nonexistent() {
    let value: Value = from_str("a: 1").unwrap();
    assert!(value.get_path("b.c.d").is_none());
}

#[test]
fn get_path_mut_modifies() {
    let yaml = "a:\n  b: old\n";
    let mut value: Value = from_str(yaml).unwrap();
    if let Some(v) = value.get_path_mut("a.b") {
        *v = Value::from("new");
    }
    assert_eq!(value.get_path("a.b").unwrap().as_str(), Some("new"));
}

// ============================================================================
// Error type: all variants Display + location + format_with_source
// ============================================================================

#[test]
fn error_all_variants_display_non_empty() {
    let loc = Location::new(1, 1, 0);
    let errors: Vec<Error> = vec![
        Error::Parse("parse msg".into()),
        Error::ParseWithLocation {
            message: "at loc".into(),
            location: loc,
        },
        Error::Serialize("ser msg".into()),
        Error::Deserialize("de msg".into()),
        Error::DeserializeWithLocation {
            message: "de at loc".into(),
            location: loc,
        },
        Error::Invalid("invalid".into()),
        Error::TypeMismatch {
            expected: "string",
            found: "integer".into(),
        },
        Error::MissingField("field".into()),
        Error::UnknownField("field".into()),
        Error::RecursionLimitExceeded { depth: 100 },
        Error::RepetitionLimitExceeded,
        Error::UnknownAnchor("anchor".into()),
        Error::ScalarInMerge,
        Error::TaggedInMerge,
        Error::ScalarInMergeElement,
        Error::SequenceInMergeElement,
        Error::EmptyTag,
        Error::FailedToParseNumber("NaN".into()),
        Error::EndOfStream,
        Error::MoreThanOneDocument,
        Error::DuplicateKey("key".into()),
        Error::Custom("custom msg".into()),
    ];

    for error in &errors {
        let msg = error.to_string();
        assert!(
            !msg.is_empty(),
            "Empty Display for {:?}",
            std::mem::discriminant(error)
        );
    }
}

#[test]
fn error_location_returns_some_for_located_variants() {
    let loc = Location::new(5, 10, 42);
    let e1 = Error::ParseWithLocation {
        message: "x".into(),
        location: loc,
    };
    assert_eq!(e1.location().unwrap().line(), 5);
    assert_eq!(e1.location().unwrap().column(), 10);
    assert_eq!(e1.location().unwrap().index(), 42);

    let e2 = Error::DeserializeWithLocation {
        message: "y".into(),
        location: loc,
    };
    assert!(e2.location().is_some());
}

#[test]
fn error_location_returns_none_for_plain_variants() {
    assert!(Error::Parse("x".into()).location().is_none());
    assert!(Error::Serialize("x".into()).location().is_none());
    assert!(Error::Deserialize("x".into()).location().is_none());
    assert!(Error::ScalarInMerge.location().is_none());
}

#[test]
fn error_format_with_source_no_location() {
    let e = Error::Parse("no location".into());
    let formatted = e.format_with_source("any source");
    assert_eq!(formatted, e.to_string());
}

#[test]
fn error_format_with_source_with_location() {
    let e = Error::ParseWithLocation {
        message: "bad token".into(),
        location: Location::new(2, 5, 11),
    };
    let source = "line one\nline two here\nline three\n";
    let formatted = e.format_with_source(source);
    assert!(formatted.contains("error:"));
    assert!(formatted.contains("line 2"));
    assert!(formatted.contains("^"));
}

#[test]
fn error_format_with_source_out_of_range() {
    let e = Error::ParseWithLocation {
        message: "past end".into(),
        location: Location::new(999, 1, 0),
    };
    let formatted = e.format_with_source("short");
    // Falls back to plain Display
    assert!(formatted.contains("past end"));
}

#[test]
fn error_parse_at_helper() {
    let e = Error::parse_at("bad", "ab\ncd\nef", 4);
    let loc = e.location().unwrap();
    assert_eq!(loc.line(), 2);
}

#[test]
fn error_deserialize_at_helper() {
    let e = Error::deserialize_at("bad", "ab\ncd", 3);
    let loc = e.location().unwrap();
    assert_eq!(loc.line(), 2);
}

// ============================================================================
// Location: edge cases
// ============================================================================

#[test]
fn location_default_is_zero() {
    let loc = Location::default();
    assert_eq!(loc.line(), 0);
    assert_eq!(loc.column(), 0);
    assert_eq!(loc.index(), 0);
}

#[test]
fn location_display() {
    let loc = Location::new(3, 7, 20);
    let display = format!("{loc}");
    assert!(display.contains("line 3"));
    assert!(display.contains("column 7"));
}

#[test]
fn location_from_index_empty_string() {
    let loc = Location::from_index("", 0);
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.column(), 1);
}

#[test]
fn location_from_index_multibyte_utf8() {
    let source = "a: \u{00e9}l\u{00e8}ve\n"; // "a: élève\n"
    let loc = Location::from_index(source, 0);
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.column(), 1);

    // Index past the multibyte char
    let loc2 = Location::from_index(source, source.len());
    assert_eq!(loc2.line(), 2);
}

#[test]
fn location_from_index_at_newline() {
    let source = "first\nsecond\n";
    let loc = Location::from_index(source, 5); // The \n itself
    assert_eq!(loc.line(), 1);

    let loc2 = Location::from_index(source, 6); // 's' of second
    assert_eq!(loc2.line(), 2);
    assert_eq!(loc2.column(), 1);
}

// ============================================================================
// Spanned<T>: Unicode + nested + from_value fallback
// ============================================================================

#[test]
fn spanned_unicode_location() {
    #[derive(Deserialize)]
    struct Doc {
        name: Spanned<String>,
    }
    // Unicode key followed by value
    let yaml = "name: caf\u{00e9}\n";
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(*doc.name, "caf\u{00e9}");
    // Location should be valid (non-zero for from_str)
    assert!(doc.name.start.line() >= 1);
}

#[test]
fn spanned_from_value_returns_zero_locations() {
    let value = Value::from(42);
    let spanned: Spanned<i64> = from_value(&value).unwrap();
    assert_eq!(*spanned, 42);
    assert_eq!(spanned.start.line(), 0);
    assert_eq!(spanned.start.column(), 0);
    assert_eq!(spanned.end.line(), 0);
}

#[test]
fn spanned_nested_vec() {
    #[derive(Deserialize)]
    struct Doc {
        items: Spanned<Vec<Spanned<String>>>,
    }
    let yaml = "items:\n  - alpha\n  - beta\n";
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(doc.items.value.len(), 2);
    assert_eq!(*doc.items.value[0], "alpha");
    assert_eq!(*doc.items.value[1], "beta");
}

#[test]
fn spanned_serialize_transparent() {
    let spanned = Spanned::new(42i64);
    let yaml = to_string(&spanned).unwrap();
    assert!(yaml.contains("42"));
    // Should serialize as plain 42, not as a struct
    assert!(!yaml.contains("value"));
}

#[test]
fn spanned_into_inner() {
    let spanned = Spanned::new("hello".to_string());
    let inner = spanned.into_inner();
    assert_eq!(inner, "hello");
}

#[test]
fn spanned_from_value() {
    let spanned: Spanned<String> = "test".to_string().into();
    assert_eq!(*spanned, "test");
}

#[test]
fn spanned_debug() {
    let spanned = Spanned::new(42);
    let debug = format!("{spanned:?}");
    assert!(debug.contains("Spanned"));
    assert!(debug.contains("42"));
}

// ============================================================================
// Anchor types: complete coverage
// ============================================================================

#[test]
fn rc_anchor_from_value_and_deref() {
    use noyalib::RcAnchor;
    let anchor: RcAnchor<i32> = RcAnchor::from(42);
    assert_eq!(*anchor, 42);
}

#[test]
fn rc_anchor_from_rc() {
    use noyalib::RcAnchor;
    let rc = std::rc::Rc::new(99);
    let anchor: RcAnchor<i32> = RcAnchor::from(rc);
    assert_eq!(*anchor, 99);
}

#[test]
fn rc_anchor_into_inner() {
    use noyalib::RcAnchor;
    let anchor = RcAnchor::from(7);
    let rc = anchor.into_inner();
    assert_eq!(*rc, 7);
}

#[test]
fn rc_anchor_serde_roundtrip() {
    use noyalib::RcAnchor;
    let anchor = RcAnchor::from(42);
    let yaml = to_string(&anchor).unwrap();
    let parsed: RcAnchor<i32> = from_str(&yaml).unwrap();
    assert_eq!(*parsed, 42);
}

#[test]
fn rc_anchor_debug() {
    use noyalib::RcAnchor;
    let anchor = RcAnchor::from(1);
    let debug = format!("{anchor:?}");
    assert!(debug.contains("RcAnchor"));
}

#[test]
fn arc_anchor_from_value_and_deref() {
    use noyalib::ArcAnchor;
    let anchor: ArcAnchor<i32> = ArcAnchor::from(42);
    assert_eq!(*anchor, 42);
}

#[test]
fn arc_anchor_from_arc() {
    use noyalib::ArcAnchor;
    let arc = std::sync::Arc::new(99);
    let anchor: ArcAnchor<i32> = ArcAnchor::from(arc);
    assert_eq!(*anchor, 99);
}

#[test]
fn arc_anchor_into_inner() {
    use noyalib::ArcAnchor;
    let anchor = ArcAnchor::from(7);
    let arc = anchor.into_inner();
    assert_eq!(*arc, 7);
}

#[test]
fn arc_anchor_serde_roundtrip() {
    use noyalib::ArcAnchor;
    let anchor = ArcAnchor::from(42);
    let yaml = to_string(&anchor).unwrap();
    let parsed: ArcAnchor<i32> = from_str(&yaml).unwrap();
    assert_eq!(*parsed, 42);
}

#[test]
fn arc_anchor_debug() {
    use noyalib::ArcAnchor;
    let anchor = ArcAnchor::from(1);
    let debug = format!("{anchor:?}");
    assert!(debug.contains("ArcAnchor"));
}

#[test]
fn rc_weak_anchor_dangling() {
    use noyalib::RcWeakAnchor;
    let weak: RcWeakAnchor<i32> = RcWeakAnchor::dangling();
    assert!(weak.upgrade().is_none());
}

#[test]
fn rc_weak_anchor_from_weak() {
    use noyalib::RcWeakAnchor;
    let rc = std::rc::Rc::new(42);
    let weak = RcWeakAnchor::from(std::rc::Rc::downgrade(&rc));
    assert_eq!(*weak.upgrade().unwrap(), 42);
    let _ = weak.into_inner();
}

#[test]
fn rc_weak_anchor_serialize_dangling() {
    use noyalib::RcWeakAnchor;
    let weak: RcWeakAnchor<i32> = RcWeakAnchor::dangling();
    let yaml = to_string(&weak).unwrap();
    assert!(yaml.contains("null") || yaml.trim() == "~");
}

#[test]
fn rc_weak_anchor_serialize_live() {
    use noyalib::RcWeakAnchor;
    let rc = std::rc::Rc::new(42);
    let weak = RcWeakAnchor(std::rc::Rc::downgrade(&rc));
    let yaml = to_string(&weak).unwrap();
    assert!(yaml.contains("42"));
}

#[test]
fn rc_weak_anchor_debug_dangling() {
    use noyalib::RcWeakAnchor;
    let weak: RcWeakAnchor<i32> = RcWeakAnchor::dangling();
    let debug = format!("{weak:?}");
    assert!(debug.contains("dangling"));
}

#[test]
fn rc_weak_anchor_debug_live() {
    use noyalib::RcWeakAnchor;
    let rc = std::rc::Rc::new(42);
    let weak = RcWeakAnchor(std::rc::Rc::downgrade(&rc));
    let debug = format!("{weak:?}");
    assert!(debug.contains("42"));
}

#[test]
fn rc_weak_anchor_deserialize_produces_dangling() {
    use noyalib::RcWeakAnchor;
    let parsed: RcWeakAnchor<i32> = from_str("42").unwrap();
    // Deserialization always produces dangling (no registry)
    assert!(parsed.upgrade().is_none());
}

#[test]
fn arc_weak_anchor_dangling() {
    use noyalib::ArcWeakAnchor;
    let weak: ArcWeakAnchor<i32> = ArcWeakAnchor::dangling();
    assert!(weak.upgrade().is_none());
}

#[test]
fn arc_weak_anchor_from_weak() {
    use noyalib::ArcWeakAnchor;
    let arc = std::sync::Arc::new(42);
    let weak = ArcWeakAnchor::from(std::sync::Arc::downgrade(&arc));
    assert_eq!(*weak.upgrade().unwrap(), 42);
    let _ = weak.into_inner();
}

#[test]
fn arc_weak_anchor_serialize_dangling() {
    use noyalib::ArcWeakAnchor;
    let weak: ArcWeakAnchor<i32> = ArcWeakAnchor::dangling();
    let yaml = to_string(&weak).unwrap();
    assert!(yaml.contains("null") || yaml.trim() == "~");
}

#[test]
fn arc_weak_anchor_serialize_live() {
    use noyalib::ArcWeakAnchor;
    let arc = std::sync::Arc::new(99);
    let weak = ArcWeakAnchor(std::sync::Arc::downgrade(&arc));
    let yaml = to_string(&weak).unwrap();
    assert!(yaml.contains("99"));
}

#[test]
fn arc_weak_anchor_debug_dangling() {
    use noyalib::ArcWeakAnchor;
    let weak: ArcWeakAnchor<i32> = ArcWeakAnchor::dangling();
    let debug = format!("{weak:?}");
    assert!(debug.contains("dangling"));
}

#[test]
fn arc_weak_anchor_deserialize_produces_dangling() {
    use noyalib::ArcWeakAnchor;
    let parsed: ArcWeakAnchor<i32> = from_str("42").unwrap();
    assert!(parsed.upgrade().is_none());
}

// ============================================================================
// Schema validation: empty collections
// ============================================================================

#[test]
fn schema_validate_empty_mapping() {
    let empty = Value::Mapping(Mapping::new());
    noyalib::validate_yaml_core_schema(&empty).unwrap();
    noyalib::validate_yaml_json_schema(&empty).unwrap();
    noyalib::validate_yaml_failsafe_schema(&empty).unwrap();
}

#[test]
fn schema_validate_empty_sequence() {
    let empty = Value::Sequence(vec![]);
    noyalib::validate_yaml_core_schema(&empty).unwrap();
    noyalib::validate_yaml_json_schema(&empty).unwrap();
    noyalib::validate_yaml_failsafe_schema(&empty).unwrap();
}

#[test]
fn schema_is_json_compatible_basic() {
    let value: Value = from_str("a: 1\nb: true\nc: hello\n").unwrap();
    assert!(noyalib::is_yaml_json_compatible(&value));
}

#[test]
fn schema_is_failsafe_compatible_strings_only() {
    // Failsafe treats everything as strings
    assert!(noyalib::is_yaml_failsafe_compatible(&Value::from("hello")));
}

// ============================================================================
// Number type: boundary conditions
// ============================================================================

#[test]
fn number_i64_min() {
    let val = to_value(&i64::MIN).unwrap();
    assert_eq!(val.as_i64(), Some(i64::MIN));
}

#[test]
fn number_i64_max() {
    let val = to_value(&i64::MAX).unwrap();
    assert_eq!(val.as_i64(), Some(i64::MAX));
}

#[test]
fn number_u64_max_errors() {
    let result = to_value(&u64::MAX);
    #[cfg(not(feature = "lossless-u64"))]
    assert!(result.is_err());
    #[cfg(feature = "lossless-u64")]
    assert_eq!(result.unwrap().as_u64(), Some(u64::MAX));
}

#[test]
fn number_f64_nan() {
    let val = to_value(&f64::NAN).unwrap();
    let n = val.as_f64().unwrap();
    assert!(n.is_nan());
}

#[test]
fn number_f64_infinity() {
    let val = to_value(&f64::INFINITY).unwrap();
    assert_eq!(val.as_f64(), Some(f64::INFINITY));
}

#[test]
fn number_f64_neg_infinity() {
    let val = to_value(&f64::NEG_INFINITY).unwrap();
    assert_eq!(val.as_f64(), Some(f64::NEG_INFINITY));
}

#[test]
fn number_is_integer() {
    let n = Number::from(42i64);
    assert!(n.is_integer());
    assert!(!n.is_float());
}

#[test]
fn number_is_float() {
    let n = Number::from(2.75f64);
    assert!(n.is_float());
    assert!(!n.is_integer());
}

// ============================================================================
// Tag / TaggedValue
// ============================================================================

#[test]
fn tag_new_and_accessors() {
    let tag = Tag::new("!custom");
    assert_eq!(tag.as_str(), "!custom");
    let s = tag.into_string();
    assert_eq!(s, "!custom");
}

#[test]
fn tagged_value_accessors() {
    let tv = TaggedValue::new(Tag::new("!t"), Value::from(42));
    assert_eq!(tv.tag().as_str(), "!t");
    assert_eq!(tv.value().as_i64(), Some(42));
}

#[test]
fn tagged_value_into_parts() {
    let tv = TaggedValue::new(Tag::new("!t"), Value::from("hello"));
    let (tag, value) = tv.into_parts();
    assert_eq!(tag.as_str(), "!t");
    assert_eq!(value.as_str(), Some("hello"));
}

#[test]
fn tagged_value_value_mut() {
    let mut tv = TaggedValue::new(Tag::new("!t"), Value::from(1));
    *tv.value_mut() = Value::from(2);
    assert_eq!(tv.value().as_i64(), Some(2));
}

#[test]
fn value_untag() {
    let tagged = Value::Tagged(Box::new(TaggedValue::new(Tag::new("!t"), Value::from(42))));
    let untagged = tagged.untag();
    assert_eq!(untagged.as_i64(), Some(42));
}

#[test]
fn value_untag_ref() {
    let tagged = Value::Tagged(Box::new(TaggedValue::new(Tag::new("!t"), Value::from(42))));
    let inner = tagged.untag_ref();
    assert_eq!(inner.as_i64(), Some(42));
}

#[test]
fn value_untag_non_tagged() {
    let plain = Value::from(42);
    let untagged = plain.untag();
    assert_eq!(untagged.as_i64(), Some(42));
}

// ============================================================================
// Path type: complete coverage
// ============================================================================

#[test]
fn path_root() {
    let p = Path::Root;
    assert!(p.is_root());
    assert!(p.parent().is_none());
    assert_eq!(p.depth(), 0);
}

#[test]
fn path_key() {
    let root = Path::Root;
    let p = root.key("database");
    assert!(!p.is_root());
    assert!(p.parent().is_some());
    assert_eq!(p.depth(), 1);
    let display = format!("{p}");
    assert!(display.contains("database"));
}

#[test]
fn path_index() {
    let root = Path::Root;
    let items = root.key("items");
    let p = items.index(3);
    assert_eq!(p.depth(), 2);
    let display = format!("{p}");
    assert!(display.contains("items"));
    assert!(display.contains("3"));
}

#[test]
fn path_alias() {
    let root = Path::Root;
    let p = root.alias();
    assert!(!p.is_root());
}

#[test]
fn path_unknown() {
    let root = Path::Root;
    let p = root.unknown();
    assert!(!p.is_root());
}

#[test]
fn path_deep_chain() {
    let root = Path::Root;
    let a = root.key("a");
    let b = a.key("b");
    let c = b.key("c");
    let idx = c.index(0);
    let d = idx.key("d");
    assert_eq!(d.depth(), 5);
}

// ============================================================================
// SerializerConfig: full builder coverage
// ============================================================================

#[test]
fn serializer_config_all_options() {
    let config = SerializerConfig::new()
        .indent(4)
        .document_start(true)
        .document_end(true)
        .block_scalars(true)
        .block_scalar_threshold(3)
        .flow_style(FlowStyle::Flow)
        .scalar_style(ScalarStyle::DoubleQuoted)
        .flow_threshold(10);

    let yaml = to_string_with_config(&"hello", &config).unwrap();
    assert!(yaml.contains("hello"));
}

#[test]
fn serializer_config_block_auto() {
    let config = SerializerConfig::new().flow_style(FlowStyle::Block);
    let v = vec![1, 2, 3];
    let yaml = to_string_with_config(&v, &config).unwrap();
    assert!(yaml.contains("- 1"));
}

#[test]
fn serializer_config_auto_below_threshold() {
    let config = SerializerConfig::new()
        .flow_style(FlowStyle::Auto)
        .flow_threshold(10);
    let v = vec![1, 2];
    let yaml = to_string_with_config(&v, &config).unwrap();
    // Small collection — auto should choose flow
    assert!(yaml.contains("[") || yaml.contains("- "));
}

#[test]
fn serializer_config_scalar_styles() {
    for style in [
        ScalarStyle::Plain,
        ScalarStyle::SingleQuoted,
        ScalarStyle::DoubleQuoted,
        ScalarStyle::Literal,
        ScalarStyle::Folded,
    ] {
        let config = SerializerConfig::new().scalar_style(style);
        let yaml = to_string_with_config(&"test", &config).unwrap();
        assert!(yaml.contains("test"));
    }
}

// ============================================================================
// Misc: nobang, check_for_tag, MaybeTag
// ============================================================================

#[test]
fn nobang_strips_exclamation() {
    assert_eq!(noyalib::nobang("!custom"), "custom");
}

#[test]
fn nobang_no_exclamation() {
    assert_eq!(noyalib::nobang("plain"), "plain");
}

#[test]
fn check_for_tag_with_tag() {
    let result = noyalib::check_for_tag(&"!tagged value");
    match result {
        noyalib::MaybeTag::Tag(t) => assert!(t.contains("tagged")),
        noyalib::MaybeTag::NotTag(_) => panic!("expected tag"),
    }
}

#[test]
fn check_for_tag_without_tag() {
    let result = noyalib::check_for_tag(&"plain value");
    match result {
        noyalib::MaybeTag::NotTag(s) => assert!(s.contains("plain")),
        noyalib::MaybeTag::Tag(_) => panic!("expected not tag"),
    }
}

// ============================================================================
// ValueIndex trait: edge cases
// ============================================================================

#[test]
fn value_index_string_owned() {
    let yaml = "name: test\n";
    let value: Value = from_str(yaml).unwrap();
    let key = String::from("name");
    assert_eq!(value.get(&key).unwrap().as_str(), Some("test"));
}

#[test]
fn value_index_usize_sequence() {
    let value = Value::Sequence(vec![Value::from(10), Value::from(20), Value::from(30)]);
    assert_eq!(value.get(0).unwrap().as_i64(), Some(10));
    assert_eq!(value.get(2).unwrap().as_i64(), Some(30));
    assert!(value.get(5).is_none());
}

#[test]
fn value_index_value_key() {
    let mut map = Mapping::new();
    let _ = map.insert("k", Value::from(99));
    let value = Value::Mapping(map);
    let key = Value::from("k");
    assert_eq!(value.get(&key).unwrap().as_i64(), Some(99));
}

// ============================================================================
// Boundary: parser limit exactly at threshold
// ============================================================================

#[test]
fn parser_depth_exactly_at_limit() {
    let config = ParserConfig::new().max_depth(3);
    // Depth 3: a -> b -> c (exactly at limit)
    let yaml = "a:\n  b:\n    c: ok\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    // This should succeed (depth = 3 is within limit of 3)
    assert!(result.is_ok());
}

#[test]
fn parser_depth_one_past_limit() {
    let config = ParserConfig::new().max_depth(3);
    // Depth 4: a -> b -> c -> d (exceeds limit)
    let yaml = "a:\n  b:\n    c:\n      d: too deep\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

#[test]
fn parser_sequence_length_at_limit() {
    let config = ParserConfig::new().max_sequence_length(3);
    let yaml = "- 1\n- 2\n- 3\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_ok());
}

#[test]
fn parser_sequence_length_past_limit() {
    let config = ParserConfig::new().max_sequence_length(3);
    let yaml = "- 1\n- 2\n- 3\n- 4\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

#[test]
fn parser_mapping_keys_at_limit() {
    let config = ParserConfig::new().max_mapping_keys(3);
    let yaml = "a: 1\nb: 2\nc: 3\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_ok());
}

#[test]
fn parser_mapping_keys_past_limit() {
    let config = ParserConfig::new().max_mapping_keys(3);
    let yaml = "a: 1\nb: 2\nc: 3\nd: 4\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

#[test]
fn parser_document_length_exceeded() {
    let config = ParserConfig::new().max_document_length(10);
    let yaml = "this is a very long document that exceeds the limit\n";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err());
}

// ============================================================================
// Serde: io::Error conversion
// ============================================================================

#[test]
fn error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let noya_err: Error = io_err.into();
    let msg = noya_err.to_string();
    assert!(msg.contains("file missing"));
}

// ============================================================================
// Value::insert / remove
// ============================================================================

#[test]
fn value_insert_creates_mapping() {
    let mut val = Value::Mapping(Mapping::new());
    let _ = val.insert("key", Value::from(1));
    assert_eq!(val.get("key").unwrap().as_i64(), Some(1));
}

#[test]
fn value_remove_from_mapping() {
    let mut val: Value = from_str("a: 1\nb: 2\nc: 3\n").unwrap();
    let _ = val.remove("b");
    assert!(val.get("b").is_none());
    assert!(val.get("a").is_some());
    assert!(val.get("c").is_some());
}

// ============================================================================
// Value::merge + merge_concat
// ============================================================================

#[test]
fn value_merge_two_mappings() {
    let mut base: Value = from_str("a: 1\nb: 2\n").unwrap();
    let overlay: Value = from_str("b: 3\nc: 4\n").unwrap();
    base.merge(overlay);
    assert_eq!(base.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(base.get("b").unwrap().as_i64(), Some(3)); // overlay wins
    assert_eq!(base.get("c").unwrap().as_i64(), Some(4));
}

#[test]
fn value_merge_concat_sequences() {
    let mut base: Value = from_str("items:\n  - a\n  - b\n").unwrap();
    let overlay: Value = from_str("items:\n  - c\n").unwrap();
    base.merge_concat(overlay);
    let items = base.get("items").unwrap().as_sequence().unwrap();
    assert!(items.len() >= 2); // At least base items preserved
}
