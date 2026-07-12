//! Comprehensive benchmarks for noyalib.
//!
//! Covers the entire public API: deserialization, serialization, configuration,
//! multi-document loading, Value operations, Mapping, MappingAny, Number, Tag,
//! Path, schema validation, Spanned, fmt wrappers, anchors, singleton_map
//! helpers, Error, and Location.
//!
//! Run with: `cargo bench --bench benchmarks`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![allow(missing_docs, unused_results)]

use std::io::Cursor;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use noyalib::{
    // Anchors
    ArcAnchor,
    ArcWeakAnchor,
    // Fmt wrappers
    Commented,
    // Config
    DuplicateKeyPolicy,
    // Error
    Error,
    FlowMap,
    FlowSeq,
    FlowStyle,
    FoldStr,
    FoldString,
    LitStr,
    LitString,
    Location,
    Mapping,
    MappingAny,
    Number,
    ParserConfig,
    // Path & Spanned
    Path,
    RcAnchor,
    RcWeakAnchor,
    ScalarStyle,
    SerializerConfig,
    SpaceAfter,
    Spanned,
    Tag,
    TaggedValue,
    Value,
    // Value types
    check_for_tag,
    // Deserialization
    from_reader,
    from_reader_with_config,
    from_slice,
    from_str,
    from_str_with_config,
    from_value,
    // Schema
    is_yaml_failsafe_compatible,
    is_yaml_json_compatible,
    // Loader
    load_all,
    load_all_as,
    load_all_with_config,
    nobang,
    // Serialization
    to_string,
    to_string_multi,
    to_string_multi_with_config,
    to_string_with_config,
    to_value,
    to_writer,
    to_writer_multi,
    to_writer_multi_with_config,
    to_writer_with_config,
    try_load_all,
    validate_yaml_core_schema,
    validate_yaml_failsafe_schema,
    validate_yaml_json_schema,
};
use serde::{Deserialize, Serialize};
use std::hint::black_box;

// ============================================================================
// Test Data
// ============================================================================

const SIMPLE_YAML: &str = "\
name: test
version: 1
enabled: true
";

const NESTED_YAML: &str = "\
server:
  host: localhost
  port: 8080
  ssl:
    enabled: true
    cert: /path/to/cert
    key: /path/to/key
database:
  host: db.example.com
  port: 5432
  credentials:
    username: admin
    password: secret
";

const SEQUENCE_YAML: &str = "\
items:
  - name: item1
    value: 100
  - name: item2
    value: 200
  - name: item3
    value: 300
  - name: item4
    value: 400
  - name: item5
    value: 500
";

const LARGE_MAPPING_YAML: &str = "\
key1: value1
key2: value2
key3: value3
key4: value4
key5: value5
key6: value6
key7: value7
key8: value8
key9: value9
key10: value10
key11: value11
key12: value12
key13: value13
key14: value14
key15: value15
key16: value16
key17: value17
key18: value18
key19: value19
key20: value20
";

const MULTI_DOC_YAML: &str = "\
---
name: doc1
version: 1
enabled: true
---
name: doc2
version: 2
enabled: false
---
name: doc3
version: 3
enabled: true
";

const TAGGED_YAML: &str = "!custom tagged_value\n";

const JSON_COMPAT_YAML: &str = "\
name: test
count: 42
ratio: 1.5
enabled: true
items:
  - one
  - two
";

const FAILSAFE_YAML: &str = "\
name: test
count: '42'
enabled: 'true'
";

fn generate_deep_yaml(depth: usize) -> String {
    let mut yaml = String::new();
    for i in 0..depth {
        yaml.push_str(&"  ".repeat(i));
        yaml.push_str(&format!("level{i}:\n"));
    }
    yaml.push_str(&"  ".repeat(depth));
    yaml.push_str("value: deep\n");
    yaml
}

fn generate_wide_sequence(count: usize) -> String {
    let mut yaml = String::from("items:\n");
    for i in 0..count {
        yaml.push_str(&format!("  - item{i}\n"));
    }
    yaml
}

// ============================================================================
// Helper Types for Serde Benchmarks
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimpleConfig {
    name: String,
    version: u64,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct SpannedConfig {
    #[allow(dead_code)]
    port: Spanned<u16>,
    #[allow(dead_code)]
    name: Spanned<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum Action {
    Get(String),
    Set { key: String, value: String },
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SingletonAction {
    #[serde(with = "noyalib::with::singleton_map")]
    action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecursiveAction {
    #[serde(with = "noyalib::with::singleton_map_recursive")]
    action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OptionalAction {
    #[serde(with = "noyalib::with::singleton_map_optional")]
    action: Option<Action>,
}

// ============================================================================
// 1. Deserialization Benchmarks
// ============================================================================

fn bench_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize");

    // from_str -> Value
    let _ = group.bench_function("from_str_value", |b| {
        b.iter(|| {
            let _: Value = from_str(black_box(SIMPLE_YAML)).unwrap();
        });
    });

    // from_str -> typed
    let _ = group.bench_function("from_str_typed", |b| {
        b.iter(|| {
            let _: SimpleConfig = from_str(black_box(SIMPLE_YAML)).unwrap();
        });
    });

    // from_str_with_config (default)
    let config_default = ParserConfig::new();
    let _ = group.bench_function("from_str_with_config_default", |b| {
        b.iter(|| {
            let _: Value =
                from_str_with_config(black_box(SIMPLE_YAML), black_box(&config_default)).unwrap();
        });
    });

    // from_str_with_config (strict)
    let config_strict = ParserConfig::strict();
    let _ = group.bench_function("from_str_with_config_strict", |b| {
        b.iter(|| {
            let _: Value =
                from_str_with_config(black_box(SIMPLE_YAML), black_box(&config_strict)).unwrap();
        });
    });

    // from_slice
    let _ = group.bench_function("from_slice", |b| {
        let bytes = SIMPLE_YAML.as_bytes();
        b.iter(|| {
            let _: Value = from_slice(black_box(bytes)).unwrap();
        });
    });

    // from_reader
    let _ = group.bench_function("from_reader", |b| {
        let bytes = SIMPLE_YAML.as_bytes();
        b.iter(|| {
            let cursor = Cursor::new(black_box(bytes));
            let _: Value = from_reader(cursor).unwrap();
        });
    });

    // from_reader_with_config
    let config = ParserConfig::new();
    let _ = group.bench_function("from_reader_with_config", |b| {
        let bytes = SIMPLE_YAML.as_bytes();
        b.iter(|| {
            let cursor = Cursor::new(black_box(bytes));
            let _: Value = from_reader_with_config(cursor, &config).unwrap();
        });
    });

    // from_value
    let value: Value = from_str(SIMPLE_YAML).unwrap();
    let _ = group.bench_function("from_value", |b| {
        b.iter(|| {
            let _: SimpleConfig = from_value(black_box(&value)).unwrap();
        });
    });

    // Deserializer::new
    let _ = group.bench_function("deserializer_new", |b| {
        b.iter(|| {
            let _ = noyalib::Deserializer::new(black_box(&value));
        });
    });

    group.finish();
}

// ============================================================================
// 2. Serialization Benchmarks
// ============================================================================

fn bench_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize");

    let simple: Value = from_str(SIMPLE_YAML).unwrap();
    let nested: Value = from_str(NESTED_YAML).unwrap();
    let sequence: Value = from_str(SEQUENCE_YAML).unwrap();
    let large_mapping: Value = from_str(LARGE_MAPPING_YAML).unwrap();

    let _ = group.bench_function("simple", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&simple)).unwrap();
        });
    });

    let _ = group.bench_function("nested", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&nested)).unwrap();
        });
    });

    let _ = group.bench_function("sequence", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&sequence)).unwrap();
        });
    });

    let _ = group.bench_function("large_mapping", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&large_mapping)).unwrap();
        });
    });

    // to_string typed
    let config_val = SimpleConfig {
        name: "test".into(),
        version: 1,
        enabled: true,
    };
    let _ = group.bench_function("typed", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&config_val)).unwrap();
        });
    });

    // to_string_with_config
    let ser_config = SerializerConfig::new().indent(4).document_start(true);
    let _ = group.bench_function("with_config", |b| {
        b.iter(|| {
            let _ = to_string_with_config(black_box(&simple), &ser_config).unwrap();
        });
    });

    // to_writer
    let _ = group.bench_function("to_writer", |b| {
        b.iter(|| {
            let mut buf = Vec::with_capacity(256);
            to_writer(&mut buf, black_box(&simple)).unwrap();
            black_box(buf);
        });
    });

    // to_writer_with_config
    let _ = group.bench_function("to_writer_with_config", |b| {
        b.iter(|| {
            let mut buf = Vec::with_capacity(256);
            to_writer_with_config(&mut buf, black_box(&simple), &ser_config).unwrap();
            black_box(buf);
        });
    });

    // to_value
    let _ = group.bench_function("to_value", |b| {
        b.iter(|| {
            let _ = to_value(black_box(&config_val)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// 3. Multi-Document Serialization Benchmarks
// ============================================================================

fn bench_serialize_multi(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_multi");

    let docs: Vec<Value> = vec![
        from_str("name: doc1\n").unwrap(),
        from_str("name: doc2\n").unwrap(),
        from_str("name: doc3\n").unwrap(),
    ];

    let configs: Vec<SimpleConfig> = vec![
        SimpleConfig {
            name: "a".into(),
            version: 1,
            enabled: true,
        },
        SimpleConfig {
            name: "b".into(),
            version: 2,
            enabled: false,
        },
    ];

    // to_string_multi
    let _ = group.bench_function("to_string_multi", |b| {
        b.iter(|| {
            let _ = to_string_multi(black_box(&docs)).unwrap();
        });
    });

    // to_string_multi_with_config
    let ser_config = SerializerConfig::new()
        .document_start(true)
        .document_end(true);
    let _ = group.bench_function("to_string_multi_with_config", |b| {
        b.iter(|| {
            let _ = to_string_multi_with_config(black_box(&docs), &ser_config).unwrap();
        });
    });

    // to_string_multi typed
    let _ = group.bench_function("to_string_multi_typed", |b| {
        b.iter(|| {
            let _ = to_string_multi(black_box(&configs)).unwrap();
        });
    });

    // to_writer_multi
    let _ = group.bench_function("to_writer_multi", |b| {
        b.iter(|| {
            let mut buf = Vec::with_capacity(256);
            to_writer_multi(&mut buf, black_box(&docs)).unwrap();
            black_box(buf);
        });
    });

    // to_writer_multi_with_config
    let _ = group.bench_function("to_writer_multi_with_config", |b| {
        b.iter(|| {
            let mut buf = Vec::with_capacity(256);
            to_writer_multi_with_config(&mut buf, black_box(&docs), &ser_config).unwrap();
            black_box(buf);
        });
    });

    group.finish();
}

// ============================================================================
// 4. SerializerConfig Benchmarks
// ============================================================================

fn bench_serializer_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("serializer_config");

    let value: Value = from_str(NESTED_YAML).unwrap();

    // Config construction
    let _ = group.bench_function("new", |b| {
        b.iter(|| black_box(SerializerConfig::new()));
    });

    let _ = group.bench_function("builder_full", |b| {
        b.iter(|| {
            black_box(
                SerializerConfig::new()
                    .indent(4)
                    .document_start(true)
                    .document_end(true)
                    .flow_style(FlowStyle::Flow)
                    .scalar_style(ScalarStyle::DoubleQuoted)
                    .block_scalars(true)
                    .block_scalar_threshold(80)
                    .flow_threshold(60),
            )
        });
    });

    // Serialization with different styles
    let flow_config = SerializerConfig::new().flow_style(FlowStyle::Flow);
    let _ = group.bench_function("flow_style", |b| {
        b.iter(|| {
            let _ = to_string_with_config(black_box(&value), &flow_config).unwrap();
        });
    });

    let auto_config = SerializerConfig::new().flow_style(FlowStyle::Auto);
    let _ = group.bench_function("auto_flow_style", |b| {
        b.iter(|| {
            let _ = to_string_with_config(black_box(&value), &auto_config).unwrap();
        });
    });

    let double_quoted = SerializerConfig::new().scalar_style(ScalarStyle::DoubleQuoted);
    let _ = group.bench_function("double_quoted", |b| {
        b.iter(|| {
            let _ = to_string_with_config(black_box(&value), &double_quoted).unwrap();
        });
    });

    let single_quoted = SerializerConfig::new().scalar_style(ScalarStyle::SingleQuoted);
    let _ = group.bench_function("single_quoted", |b| {
        b.iter(|| {
            let _ = to_string_with_config(black_box(&value), &single_quoted).unwrap();
        });
    });

    let literal = SerializerConfig::new().scalar_style(ScalarStyle::Literal);
    let _ = group.bench_function("literal_scalar", |b| {
        b.iter(|| {
            let _ = to_string_with_config(black_box(&value), &literal).unwrap();
        });
    });

    let folded = SerializerConfig::new().scalar_style(ScalarStyle::Folded);
    let _ = group.bench_function("folded_scalar", |b| {
        b.iter(|| {
            let _ = to_string_with_config(black_box(&value), &folded).unwrap();
        });
    });

    let plain = SerializerConfig::new().scalar_style(ScalarStyle::Plain);
    let _ = group.bench_function("plain_scalar", |b| {
        b.iter(|| {
            let _ = to_string_with_config(black_box(&value), &plain).unwrap();
        });
    });

    let indent_4 = SerializerConfig::new().indent(4);
    let _ = group.bench_function("indent_4", |b| {
        b.iter(|| {
            let _ = to_string_with_config(black_box(&value), &indent_4).unwrap();
        });
    });

    let doc_markers = SerializerConfig::new()
        .document_start(true)
        .document_end(true);
    let _ = group.bench_function("document_markers", |b| {
        b.iter(|| {
            let _ = to_string_with_config(black_box(&value), &doc_markers).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// 5. ParserConfig Benchmarks
// ============================================================================

fn bench_parser_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_config");

    // Construction
    let _ = group.bench_function("new", |b| {
        b.iter(|| black_box(ParserConfig::new()));
    });

    let _ = group.bench_function("strict", |b| {
        b.iter(|| black_box(ParserConfig::strict()));
    });

    let _ = group.bench_function("builder_chain", |b| {
        b.iter(|| {
            black_box(
                ParserConfig::new()
                    .max_depth(64)
                    .max_document_length(1024 * 1024)
                    .max_alias_expansions(100)
                    .max_mapping_keys(1000)
                    .max_sequence_length(1000)
                    .duplicate_key_policy(DuplicateKeyPolicy::Error),
            )
        });
    });

    // DuplicateKeyPolicy variants
    let dup_yaml = "a: 1\nb: 2\na: 3\n";

    let last_config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Last);
    let _ = group.bench_function("dup_policy_last", |b| {
        b.iter(|| {
            let _: Value = from_str_with_config(black_box(dup_yaml), &last_config).unwrap();
        });
    });

    let first_config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let _ = group.bench_function("dup_policy_first", |b| {
        b.iter(|| {
            let _: Value = from_str_with_config(black_box(dup_yaml), &first_config).unwrap();
        });
    });

    let error_config = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let _ = group.bench_function("dup_policy_error", |b| {
        b.iter(|| {
            let result: Result<Value, _> = from_str_with_config(black_box(dup_yaml), &error_config);
            let _ = black_box(result);
        });
    });

    group.finish();
}

// ============================================================================
// 6. Parse Benchmarks
// ============================================================================

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    let _ = group.bench_function("simple", |b| {
        b.iter(|| {
            let _: Value = from_str(black_box(SIMPLE_YAML)).unwrap();
        });
    });

    let _ = group.bench_function("nested", |b| {
        b.iter(|| {
            let _: Value = from_str(black_box(NESTED_YAML)).unwrap();
        });
    });

    let _ = group.bench_function("sequence", |b| {
        b.iter(|| {
            let _: Value = from_str(black_box(SEQUENCE_YAML)).unwrap();
        });
    });

    let _ = group.bench_function("large_mapping", |b| {
        b.iter(|| {
            let _: Value = from_str(black_box(LARGE_MAPPING_YAML)).unwrap();
        });
    });

    group.finish();
}

fn bench_parse_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_depth");

    for depth in [5, 10, 20, 50] {
        let yaml = generate_deep_yaml(depth);
        let _ = group.throughput(Throughput::Bytes(yaml.len() as u64));
        let _ = group.bench_with_input(BenchmarkId::from_parameter(depth), &yaml, |b, yaml| {
            b.iter(|| {
                let _: Value = from_str(black_box(yaml)).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_parse_width(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_width");

    for count in [10, 50, 100, 500] {
        let yaml = generate_wide_sequence(count);
        let _ = group.throughput(Throughput::Bytes(yaml.len() as u64));
        let _ = group.bench_with_input(BenchmarkId::from_parameter(count), &yaml, |b, yaml| {
            b.iter(|| {
                let _: Value = from_str(black_box(yaml)).unwrap();
            });
        });
    }

    group.finish();
}

// ============================================================================
// 7. Roundtrip Benchmarks
// ============================================================================

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    let _ = group.bench_function("simple", |b| {
        b.iter(|| {
            let value: Value = from_str(black_box(SIMPLE_YAML)).unwrap();
            let _ = to_string(black_box(&value)).unwrap();
        });
    });

    let _ = group.bench_function("nested", |b| {
        b.iter(|| {
            let value: Value = from_str(black_box(NESTED_YAML)).unwrap();
            let _ = to_string(black_box(&value)).unwrap();
        });
    });

    let _ = group.bench_function("typed", |b| {
        b.iter(|| {
            let config: SimpleConfig = from_str(black_box(SIMPLE_YAML)).unwrap();
            let _ = to_string(black_box(&config)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// 8. Multi-Document Loader Benchmarks
// ============================================================================

fn bench_loader(c: &mut Criterion) {
    let mut group = c.benchmark_group("loader");

    // load_all single document
    let _ = group.bench_function("load_all_single", |b| {
        b.iter(|| {
            let iter = load_all(black_box(SIMPLE_YAML)).unwrap();
            for doc in iter {
                black_box(doc.unwrap());
            }
        });
    });

    // load_all multiple documents
    let _ = group.bench_function("load_all_multi", |b| {
        b.iter(|| {
            let iter = load_all(black_box(MULTI_DOC_YAML)).unwrap();
            for doc in iter {
                black_box(doc.unwrap());
            }
        });
    });

    // load_all_with_config
    let config = ParserConfig::new();
    let _ = group.bench_function("load_all_with_config", |b| {
        b.iter(|| {
            let iter = load_all_with_config(black_box(MULTI_DOC_YAML), &config).unwrap();
            for doc in iter {
                black_box(doc.unwrap());
            }
        });
    });

    // load_all_as typed
    let _ = group.bench_function("load_all_as_typed", |b| {
        b.iter(|| {
            let docs: Vec<SimpleConfig> = load_all_as(black_box(MULTI_DOC_YAML)).unwrap();
            black_box(docs);
        });
    });

    // try_load_all
    let _ = group.bench_function("try_load_all", |b| {
        b.iter(|| {
            let iter = try_load_all(black_box(MULTI_DOC_YAML)).unwrap();
            for doc in iter {
                black_box(doc.unwrap());
            }
        });
    });

    // DocumentIterator len/is_empty
    let _ = group.bench_function("doc_iterator_len", |b| {
        let iter = load_all(MULTI_DOC_YAML).unwrap();
        b.iter(|| {
            black_box(iter.len());
        });
    });

    group.finish();
}

// ============================================================================
// 9. Value Access Benchmarks
// ============================================================================

fn bench_value_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("value_access");

    let nested: Value = from_str(NESTED_YAML).unwrap();
    let sequence: Value = from_str(SEQUENCE_YAML).unwrap();

    // get by &str
    let _ = group.bench_function("get_str", |b| {
        b.iter(|| {
            let _ = black_box(&nested).get("server");
        });
    });

    // get by usize (sequence)
    let _ = group.bench_function("get_usize", |b| {
        let items = sequence.get("items").unwrap();
        b.iter(|| {
            let _ = black_box(items).get(2);
        });
    });

    // get_mut by &str
    let _ = group.bench_function("get_mut_str", |b| {
        b.iter_batched(
            || nested.clone(),
            |mut val| {
                let _ = val.get_mut("server");
                black_box(val);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // get_path shallow
    let _ = group.bench_function("get_path_shallow", |b| {
        b.iter(|| {
            let _ = black_box(&nested).get_path("server.host");
        });
    });

    // get_path deep
    let _ = group.bench_function("get_path_deep", |b| {
        b.iter(|| {
            let _ = black_box(&nested).get_path("server.ssl.enabled");
        });
    });

    // get_path_mut
    let _ = group.bench_function("get_path_mut", |b| {
        b.iter_batched(
            || nested.clone(),
            |mut val| {
                let _ = val.get_path_mut("server.ssl.enabled");
                black_box(val);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Type predicates
    let null_val = Value::Null;
    let bool_val = Value::from(true);
    let num_val = Value::from(42i64);
    let str_val = Value::from("hello");
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!custom"),
        Value::from("val"),
    )));

    let _ = group.bench_function("is_null", |b| {
        b.iter(|| black_box(&null_val).is_null());
    });
    let _ = group.bench_function("is_bool", |b| {
        b.iter(|| black_box(&bool_val).is_bool());
    });
    let _ = group.bench_function("is_number", |b| {
        b.iter(|| black_box(&num_val).is_number());
    });
    let _ = group.bench_function("is_string", |b| {
        b.iter(|| black_box(&str_val).is_string());
    });
    let _ = group.bench_function("is_sequence", |b| {
        b.iter(|| black_box(&sequence).is_sequence());
    });
    let _ = group.bench_function("is_mapping", |b| {
        b.iter(|| black_box(&nested).is_mapping());
    });
    let _ = group.bench_function("is_tagged", |b| {
        b.iter(|| black_box(&tagged).is_tagged());
    });
    let _ = group.bench_function("is_i64", |b| {
        b.iter(|| black_box(&num_val).is_i64());
    });
    let _ = group.bench_function("is_u64", |b| {
        b.iter(|| black_box(&num_val).is_u64());
    });
    let _ = group.bench_function("is_f64", |b| {
        b.iter(|| black_box(&Value::from(1.5f64)).is_f64());
    });

    // as_* accessors
    let _ = group.bench_function("as_null", |b| {
        b.iter(|| black_box(&null_val).as_null());
    });
    let _ = group.bench_function("as_bool", |b| {
        b.iter(|| black_box(&bool_val).as_bool());
    });
    let _ = group.bench_function("as_i64", |b| {
        b.iter(|| black_box(&num_val).as_i64());
    });
    let _ = group.bench_function("as_u64", |b| {
        b.iter(|| black_box(&num_val).as_u64());
    });
    let _ = group.bench_function("as_f64", |b| {
        b.iter(|| black_box(&Value::from(1.5f64)).as_f64());
    });
    let _ = group.bench_function("as_str", |b| {
        b.iter(|| black_box(&str_val).as_str());
    });
    let _ = group.bench_function("as_sequence", |b| {
        b.iter(|| black_box(&sequence).as_sequence());
    });
    let _ = group.bench_function("as_mapping", |b| {
        b.iter(|| black_box(&nested).as_mapping());
    });
    let _ = group.bench_function("as_tagged", |b| {
        b.iter(|| black_box(&tagged).as_tagged());
    });

    group.finish();
}

// ============================================================================
// 10. Value Construction Benchmarks
// ============================================================================

fn bench_value_construct(c: &mut Criterion) {
    let mut group = c.benchmark_group("value_construct");

    let _ = group.bench_function("default", |b| {
        b.iter(|| black_box(Value::default()));
    });

    let _ = group.bench_function("from_bool", |b| {
        b.iter(|| Value::from(black_box(true)));
    });

    let _ = group.bench_function("from_i64", |b| {
        b.iter(|| Value::from(black_box(42i64)));
    });

    let _ = group.bench_function("from_u64", |b| {
        b.iter(|| Value::from(black_box(42u64)));
    });

    let _ = group.bench_function("from_f64", |b| {
        b.iter(|| Value::from(black_box(2.75f64)));
    });

    let _ = group.bench_function("from_str", |b| {
        b.iter(|| Value::from(black_box("hello world")));
    });

    let _ = group.bench_function("from_string", |b| {
        b.iter(|| Value::from(black_box(String::from("hello world"))));
    });

    let _ = group.bench_function("from_number", |b| {
        b.iter(|| Value::from(black_box(Number::Integer(42))));
    });

    let _ = group.bench_function("from_mapping", |b| {
        b.iter(|| {
            let map = Mapping::new();
            Value::from(black_box(map))
        });
    });

    let _ = group.bench_function("from_tagged_value", |b| {
        b.iter(|| {
            let tv = TaggedValue::new(Tag::new("!tag"), Value::from("val"));
            Value::from(black_box(tv))
        });
    });

    let _ = group.bench_function("from_sequence", |b| {
        b.iter(|| {
            let seq: Vec<Value> = vec![Value::from(1), Value::from(2), Value::from(3)];
            Value::from(black_box(seq))
        });
    });

    // Clone
    let complex: Value = from_str(NESTED_YAML).unwrap();
    let _ = group.bench_function("clone_complex", |b| {
        b.iter(|| black_box(&complex).clone());
    });

    // Display
    let _ = group.bench_function("display_simple", |b| {
        let val = Value::from("hello");
        b.iter(|| format!("{}", black_box(&val)));
    });

    group.finish();
}

// ============================================================================
// 11. Value Mutation Benchmarks
// ============================================================================

fn bench_value_mutate(c: &mut Criterion) {
    let mut group = c.benchmark_group("value_mutate");

    let base: Value = from_str("a: 1\nb: 2\nc: 3\n").unwrap();
    let other: Value = from_str("b: 20\nd: 4\ne: 5\n").unwrap();

    // merge (small)
    let _ = group.bench_function("merge_small", |b| {
        b.iter(|| {
            let mut base_clone = base.clone();
            base_clone.merge(black_box(other.clone()));
            black_box(base_clone)
        });
    });

    // merge (nested)
    let nested_base: Value = from_str(NESTED_YAML).unwrap();
    let nested_other: Value =
        from_str("server:\n  port: 9090\n  timeout: 30\nlogging:\n  level: debug\n").unwrap();
    let _ = group.bench_function("merge_nested", |b| {
        b.iter(|| {
            let mut base_clone = nested_base.clone();
            base_clone.merge(black_box(nested_other.clone()));
            black_box(base_clone)
        });
    });

    // merge_concat
    let _ = group.bench_function("merge_concat", |b| {
        b.iter(|| {
            let mut base_clone = base.clone();
            base_clone.merge_concat(black_box(other.clone()));
            black_box(base_clone)
        });
    });

    // remove
    let _ = group.bench_function("remove", |b| {
        b.iter_batched(
            || base.clone(),
            |mut val| {
                let _ = val.remove("b");
                black_box(val);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // insert
    let _ = group.bench_function("insert", |b| {
        b.iter_batched(
            || base.clone(),
            |mut val| {
                let _ = val.insert("new_key", Value::from("new_value"));
                black_box(val);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // apply_merge
    let merge_yaml =
        "defaults: &defaults\n  timeout: 30\nserver:\n  <<: *defaults\n  host: localhost\n";
    let _ = group.bench_function("apply_merge", |b| {
        b.iter_batched(
            || -> Value { from_str(merge_yaml).unwrap() },
            |mut val| {
                let _ = val.apply_merge();
                black_box(val);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // untag
    let tagged = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!custom"),
        Value::from("val"),
    )));
    let _ = group.bench_function("untag", |b| {
        b.iter_batched(
            || tagged.clone(),
            |val| {
                black_box(val.untag());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // untag_ref
    let _ = group.bench_function("untag_ref", |b| {
        b.iter(|| {
            let _ = black_box(&tagged).untag_ref();
        });
    });

    group.finish();
}

// ============================================================================
// 12. Mapping Benchmarks
// ============================================================================

fn bench_mapping(c: &mut Criterion) {
    let mut group = c.benchmark_group("mapping");

    // Construction
    let _ = group.bench_function("new", |b| {
        b.iter(|| black_box(Mapping::new()));
    });

    let _ = group.bench_function("with_capacity", |b| {
        b.iter(|| black_box(Mapping::with_capacity(black_box(100))));
    });

    // Insert
    let _ = group.bench_function("insert_10", |b| {
        b.iter(|| {
            let mut map = Mapping::new();
            for i in 0..10 {
                let _ = map.insert(format!("key{i}"), Value::from(i));
            }
            black_box(map)
        });
    });

    let _ = group.bench_function("insert_100", |b| {
        b.iter(|| {
            let mut map = Mapping::with_capacity(100);
            for i in 0..100 {
                let _ = map.insert(format!("key{i}"), Value::from(i));
            }
            black_box(map)
        });
    });

    // Build a map for read benchmarks
    let mut map = Mapping::new();
    for i in 0..50 {
        let _ = map.insert(format!("key{i}"), Value::from(i));
    }

    // get
    let _ = group.bench_function("get_hit", |b| {
        b.iter(|| black_box(map.get(black_box("key25"))));
    });

    let _ = group.bench_function("get_miss", |b| {
        b.iter(|| black_box(map.get(black_box("nonexistent"))));
    });

    // get_mut
    let _ = group.bench_function("get_mut", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                let _ = m.get_mut("key25");
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // contains_key
    let _ = group.bench_function("contains_key_hit", |b| {
        b.iter(|| black_box(map.contains_key(black_box("key10"))));
    });

    let _ = group.bench_function("contains_key_miss", |b| {
        b.iter(|| black_box(map.contains_key(black_box("missing"))));
    });

    // get_index
    let _ = group.bench_function("get_index", |b| {
        b.iter(|| black_box(map.get_index(black_box(10))));
    });

    // remove
    let _ = group.bench_function("remove", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                let _ = m.remove("key25");
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // remove_entry
    let _ = group.bench_function("remove_entry", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                let _ = m.remove_entry("key25");
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // swap_remove
    let _ = group.bench_function("swap_remove", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                let _ = m.swap_remove("key25");
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // entry
    let _ = group.bench_function("entry_occupied", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                let _ = m.entry("key10");
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    let _ = group.bench_function("entry_vacant", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                let _ = m.entry("new_key");
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // retain
    let _ = group.bench_function("retain", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                m.retain(|k, _| k.ends_with('0'));
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // sort_keys
    let _ = group.bench_function("sort_keys", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                m.sort_keys();
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // reverse
    let _ = group.bench_function("reverse", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                m.reverse();
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // iter
    let _ = group.bench_function("iter", |b| {
        b.iter(|| {
            for entry in black_box(&map).iter() {
                black_box(entry);
            }
        });
    });

    // keys
    let _ = group.bench_function("keys", |b| {
        b.iter(|| {
            for key in black_box(&map).keys() {
                black_box(key);
            }
        });
    });

    // values
    let _ = group.bench_function("values", |b| {
        b.iter(|| {
            for val in black_box(&map).values() {
                black_box(val);
            }
        });
    });

    // len, is_empty, capacity
    let _ = group.bench_function("len", |b| {
        b.iter(|| black_box(map.len()));
    });

    let _ = group.bench_function("is_empty", |b| {
        b.iter(|| black_box(map.is_empty()));
    });

    let _ = group.bench_function("capacity", |b| {
        b.iter(|| black_box(map.capacity()));
    });

    // first / last
    let _ = group.bench_function("first", |b| {
        b.iter(|| black_box(map.first()));
    });

    let _ = group.bench_function("last", |b| {
        b.iter(|| black_box(map.last()));
    });

    // pop_first / pop_last
    let _ = group.bench_function("pop_first", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                black_box(m.pop_first());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    let _ = group.bench_function("pop_last", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                black_box(m.pop_last());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // clear
    let _ = group.bench_function("clear", |b| {
        b.iter_batched(
            || map.clone(),
            |mut m| {
                m.clear();
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // shrink_to_fit
    let _ = group.bench_function("shrink_to_fit", |b| {
        b.iter_batched(
            || {
                let mut m = Mapping::with_capacity(200);
                for i in 0..10 {
                    let _ = m.insert(format!("k{i}"), Value::from(i));
                }
                m
            },
            |mut m| {
                m.shrink_to_fit();
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // reserve
    let _ = group.bench_function("reserve", |b| {
        b.iter_batched(
            Mapping::new,
            |mut m| {
                m.reserve(100);
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // into_inner
    let _ = group.bench_function("into_inner", |b| {
        b.iter_batched(
            || map.clone(),
            |m| {
                black_box(m.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // from_inner
    let _ = group.bench_function("from_inner", |b| {
        let inner = map.clone().into_inner();
        b.iter(|| black_box(Mapping::from_inner(inner.clone())));
    });

    // extend
    let _ = group.bench_function("extend", |b| {
        let extra: Vec<(String, Value)> = (0..10)
            .map(|i| (format!("extra{i}"), Value::from(i)))
            .collect();
        b.iter_batched(
            || map.clone(),
            |mut m| {
                m.extend(extra.clone());
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Serialize/Deserialize
    let _ = group.bench_function("serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&map)).unwrap();
        });
    });

    let yaml_str = to_string(&map).unwrap();
    let _ = group.bench_function("deserialize", |b| {
        b.iter(|| {
            let _: Mapping = from_str(black_box(&yaml_str)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// 13. MappingAny Benchmarks
// ============================================================================

fn bench_mapping_any(c: &mut Criterion) {
    let mut group = c.benchmark_group("mapping_any");

    // Construction
    let _ = group.bench_function("new", |b| {
        b.iter(|| black_box(MappingAny::new()));
    });

    let _ = group.bench_function("with_capacity", |b| {
        b.iter(|| black_box(MappingAny::with_capacity(black_box(50))));
    });

    // Insert with various key types
    let _ = group.bench_function("insert_string_keys", |b| {
        b.iter(|| {
            let mut m = MappingAny::new();
            for i in 0..10 {
                let _ = m.insert(Value::from(format!("key{i}")), Value::from(i));
            }
            black_box(m)
        });
    });

    let _ = group.bench_function("insert_integer_keys", |b| {
        b.iter(|| {
            let mut m = MappingAny::new();
            for i in 0..10 {
                let _ = m.insert(Value::from(i as i64), Value::from(format!("val{i}")));
            }
            black_box(m)
        });
    });

    // Build a map for read benchmarks
    let mut any_map = MappingAny::new();
    for i in 0..50 {
        let _ = any_map.insert(Value::from(format!("key{i}")), Value::from(i));
    }

    // get
    let key = Value::from("key25");
    let _ = group.bench_function("get_hit", |b| {
        b.iter(|| black_box(any_map.get(black_box(&key))));
    });

    let missing = Value::from("missing");
    let _ = group.bench_function("get_miss", |b| {
        b.iter(|| black_box(any_map.get(black_box(&missing))));
    });

    // contains_key
    let _ = group.bench_function("contains_key", |b| {
        b.iter(|| black_box(any_map.contains_key(black_box(&key))));
    });

    // get_index
    let _ = group.bench_function("get_index", |b| {
        b.iter(|| black_box(any_map.get_index(black_box(10))));
    });

    // remove
    let _ = group.bench_function("remove", |b| {
        b.iter_batched(
            || any_map.clone(),
            |mut m| {
                let _ = m.remove(&Value::from("key25"));
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // entry
    let _ = group.bench_function("entry", |b| {
        b.iter_batched(
            || any_map.clone(),
            |mut m| {
                let _ = m.entry(Value::from("key10"));
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // sort_keys
    let _ = group.bench_function("sort_keys", |b| {
        b.iter_batched(
            || any_map.clone(),
            |mut m| {
                m.sort_keys();
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // iter
    let _ = group.bench_function("iter", |b| {
        b.iter(|| {
            for entry in black_box(&any_map).iter() {
                black_box(entry);
            }
        });
    });

    // into_mapping (all-string keys)
    let _ = group.bench_function("into_mapping", |b| {
        b.iter_batched(
            || any_map.clone(),
            |m| {
                black_box(m.into_mapping());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // into_mapping with mixed keys (returns None)
    let _ = group.bench_function("into_mapping_mixed", |b| {
        let mut mixed = MappingAny::new();
        let _ = mixed.insert(Value::from("str_key"), Value::from(1));
        let _ = mixed.insert(Value::from(42i64), Value::from(2));
        b.iter_batched(
            || mixed.clone(),
            |m| {
                black_box(m.into_mapping());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // len / is_empty
    let _ = group.bench_function("len", |b| {
        b.iter(|| black_box(any_map.len()));
    });

    let _ = group.bench_function("is_empty", |b| {
        b.iter(|| black_box(any_map.is_empty()));
    });

    // retain
    let _ = group.bench_function("retain", |b| {
        b.iter_batched(
            || any_map.clone(),
            |mut m| {
                m.retain(|k, _| k.is_string());
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // reverse
    let _ = group.bench_function("reverse", |b| {
        b.iter_batched(
            || any_map.clone(),
            |mut m| {
                m.reverse();
                black_box(m);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // first / last
    let _ = group.bench_function("first", |b| {
        b.iter(|| black_box(any_map.first()));
    });

    let _ = group.bench_function("last", |b| {
        b.iter(|| black_box(any_map.last()));
    });

    // keys / values
    let _ = group.bench_function("keys", |b| {
        b.iter(|| {
            for k in black_box(&any_map).keys() {
                black_box(k);
            }
        });
    });

    let _ = group.bench_function("values", |b| {
        b.iter(|| {
            for v in black_box(&any_map).values() {
                black_box(v);
            }
        });
    });

    // into_inner / from_inner
    let _ = group.bench_function("into_inner", |b| {
        b.iter_batched(
            || any_map.clone(),
            |m| {
                black_box(m.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// 14. Number Benchmarks
// ============================================================================

fn bench_number(c: &mut Criterion) {
    let mut group = c.benchmark_group("number");

    let int = Number::Integer(42);
    let float = Number::Float(3.125);

    // Accessors
    let _ = group.bench_function("as_i64_int", |b| {
        b.iter(|| black_box(&int).as_i64());
    });

    let _ = group.bench_function("as_i64_float", |b| {
        b.iter(|| black_box(&float).as_i64());
    });

    let _ = group.bench_function("as_u64", |b| {
        b.iter(|| black_box(&int).as_u64());
    });

    let _ = group.bench_function("as_f64_int", |b| {
        b.iter(|| black_box(&int).as_f64());
    });

    let _ = group.bench_function("as_f64_float", |b| {
        b.iter(|| black_box(&float).as_f64());
    });

    // Predicates
    let _ = group.bench_function("is_integer", |b| {
        b.iter(|| black_box(&int).is_integer());
    });

    let _ = group.bench_function("is_float", |b| {
        b.iter(|| black_box(&float).is_float());
    });

    let _ = group.bench_function("is_i64", |b| {
        b.iter(|| black_box(&int).is_i64());
    });

    let _ = group.bench_function("is_u64", |b| {
        b.iter(|| black_box(&int).is_u64());
    });

    let _ = group.bench_function("is_f64", |b| {
        b.iter(|| black_box(&float).is_f64());
    });

    let nan = Number::Float(f64::NAN);
    let inf = Number::Float(f64::INFINITY);

    let _ = group.bench_function("is_nan", |b| {
        b.iter(|| black_box(&nan).is_nan());
    });

    let _ = group.bench_function("is_infinite", |b| {
        b.iter(|| black_box(&inf).is_infinite());
    });

    let _ = group.bench_function("is_finite", |b| {
        b.iter(|| black_box(&float).is_finite());
    });

    // Display
    let _ = group.bench_function("display_integer", |b| {
        b.iter(|| format!("{}", black_box(&int)));
    });

    let _ = group.bench_function("display_float", |b| {
        b.iter(|| format!("{}", black_box(&float)));
    });

    // FromStr
    let _ = group.bench_function("from_str_integer", |b| {
        b.iter(|| black_box("42").parse::<Number>());
    });

    let _ = group.bench_function("from_str_float", |b| {
        b.iter(|| black_box("3.14159").parse::<Number>());
    });

    // Comparison
    let _ = group.bench_function("cmp_integers", |b| {
        let a = Number::Integer(42);
        let b_num = Number::Integer(100);
        b.iter(|| black_box(&a).cmp(black_box(&b_num)));
    });

    let _ = group.bench_function("eq_integers", |b| {
        let a = Number::Integer(42);
        let b_num = Number::Integer(42);
        b.iter(|| black_box(&a) == black_box(&b_num));
    });

    // From conversions
    let _ = group.bench_function("from_i32", |b| {
        b.iter(|| Number::from(black_box(42i32)));
    });

    let _ = group.bench_function("from_f32", |b| {
        b.iter(|| Number::from(black_box(2.75f32)));
    });

    let _ = group.bench_function("from_usize", |b| {
        b.iter(|| Number::from(black_box(42usize)));
    });

    // Hash
    let _ = group.bench_function("hash", |b| {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            black_box(&int).hash(&mut hasher);
            black_box(hasher.finish());
        });
    });

    group.finish();
}

// ============================================================================
// 15. Tag & TaggedValue Benchmarks
// ============================================================================

fn bench_tag(c: &mut Criterion) {
    let mut group = c.benchmark_group("tag");

    // Tag construction
    let _ = group.bench_function("tag_new_str", |b| {
        b.iter(|| Tag::new(black_box("!custom")));
    });

    let _ = group.bench_function("tag_new_string", |b| {
        b.iter(|| Tag::new(black_box(String::from("!custom"))));
    });

    // Tag accessors
    let tag = Tag::new("!custom");
    let _ = group.bench_function("tag_as_str", |b| {
        b.iter(|| black_box(&tag).as_str());
    });

    let _ = group.bench_function("tag_nobang", |b| {
        b.iter(|| black_box(&tag).nobang());
    });

    let _ = group.bench_function("tag_into_string", |b| {
        b.iter_batched(
            || tag.clone(),
            |t| {
                black_box(t.into_string());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Tag Display
    let _ = group.bench_function("tag_display", |b| {
        b.iter(|| format!("{}", black_box(&tag)));
    });

    // Tag equality (ignores leading !)
    let tag_a = Tag::new("!custom");
    let tag_b = Tag::new("custom");
    let _ = group.bench_function("tag_eq_nobang", |b| {
        b.iter(|| black_box(&tag_a) == black_box(&tag_b));
    });

    // TaggedValue construction
    let _ = group.bench_function("tagged_value_new", |b| {
        b.iter(|| {
            TaggedValue::new(
                Tag::new(black_box("!custom")),
                Value::from(black_box("value")),
            )
        });
    });

    // TaggedValue accessors
    let tv = TaggedValue::new(Tag::new("!custom"), Value::from("hello"));
    let _ = group.bench_function("tagged_value_tag", |b| {
        b.iter(|| black_box(&tv).tag());
    });

    let _ = group.bench_function("tagged_value_value", |b| {
        b.iter(|| black_box(&tv).value());
    });

    let _ = group.bench_function("tagged_value_value_mut", |b| {
        b.iter_batched(
            || tv.clone(),
            |mut t| {
                black_box(t.value_mut());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    let _ = group.bench_function("tagged_value_into_parts", |b| {
        b.iter_batched(
            || tv.clone(),
            |t| {
                black_box(t.into_parts());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Free functions
    let _ = group.bench_function("nobang_fn", |b| {
        b.iter(|| nobang(black_box("!tag")));
    });

    let _ = group.bench_function("nobang_fn_no_prefix", |b| {
        b.iter(|| nobang(black_box("tag")));
    });

    let _ = group.bench_function("check_for_tag_tagged", |b| {
        b.iter(|| check_for_tag(black_box(&"!custom value")));
    });

    let _ = group.bench_function("check_for_tag_untagged", |b| {
        b.iter(|| check_for_tag(black_box(&"plain value")));
    });

    // Tagged YAML parse
    let _ = group.bench_function("parse_tagged", |b| {
        b.iter(|| {
            let _: Value = from_str(black_box(TAGGED_YAML)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// 16. Path Benchmarks
// ============================================================================

fn bench_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("path");

    // Construction
    let _ = group.bench_function("root", |b| {
        b.iter(|| black_box(Path::Root));
    });

    let _ = group.bench_function("key", |b| {
        let root = Path::Root;
        b.iter(|| black_box(root.key(black_box("field"))));
    });

    let _ = group.bench_function("index", |b| {
        let root = Path::Root;
        b.iter(|| black_box(root.index(black_box(0))));
    });

    let _ = group.bench_function("alias", |b| {
        let root = Path::Root;
        b.iter(|| black_box(root.alias()));
    });

    let _ = group.bench_function("unknown", |b| {
        let root = Path::Root;
        b.iter(|| black_box(root.unknown()));
    });

    // Chained construction (measure depth to consume in-place)
    let _ = group.bench_function("chain_3_deep", |b| {
        b.iter(|| {
            let root = Path::Root;
            let p1 = root.key("server");
            let p2 = p1.key("ssl");
            let p3 = p2.key("enabled");
            black_box(p3.depth());
        });
    });

    let _ = group.bench_function("chain_mixed", |b| {
        b.iter(|| {
            let root = Path::Root;
            let p1 = root.key("items");
            let p2 = p1.index(0);
            let p3 = p2.key("name");
            black_box(p3.depth());
        });
    });

    // Display
    let root = Path::Root;
    let p1 = root.key("server");
    let p2 = p1.key("ssl");
    let deep = p2.key("enabled");

    let _ = group.bench_function("display_root", |b| {
        b.iter(|| format!("{}", black_box(Path::Root)));
    });

    let _ = group.bench_function("display_deep", |b| {
        b.iter(|| format!("{}", black_box(deep)));
    });

    // Properties
    let _ = group.bench_function("is_root_true", |b| {
        b.iter(|| black_box(Path::Root).is_root());
    });

    let _ = group.bench_function("is_root_false", |b| {
        b.iter(|| black_box(deep).is_root());
    });

    let _ = group.bench_function("depth_0", |b| {
        b.iter(|| black_box(Path::Root).depth());
    });

    let _ = group.bench_function("depth_3", |b| {
        b.iter(|| black_box(deep).depth());
    });

    let _ = group.bench_function("parent", |b| {
        b.iter(|| {
            let root = Path::Root;
            let p1 = root.key("server");
            let p2 = p1.key("ssl");
            let p3 = p2.key("enabled");
            black_box(p3.parent().is_some());
        });
    });

    group.finish();
}

// ============================================================================
// 17. Schema Validation Benchmarks
// ============================================================================

fn bench_schema(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema");

    let json_value: Value = from_str(JSON_COMPAT_YAML).unwrap();
    let failsafe_value: Value = from_str(FAILSAFE_YAML).unwrap();
    let nested: Value = from_str(NESTED_YAML).unwrap();

    // validate_yaml_core_schema
    let _ = group.bench_function("validate_core", |b| {
        b.iter(|| validate_yaml_core_schema(black_box(&nested)));
    });

    // validate_yaml_json_schema
    let _ = group.bench_function("validate_json", |b| {
        b.iter(|| validate_yaml_json_schema(black_box(&json_value)));
    });

    // validate_yaml_failsafe_schema
    let _ = group.bench_function("validate_failsafe", |b| {
        b.iter(|| validate_yaml_failsafe_schema(black_box(&failsafe_value)));
    });

    // is_yaml_json_compatible
    let _ = group.bench_function("is_json_compatible_true", |b| {
        b.iter(|| is_yaml_json_compatible(black_box(&json_value)));
    });

    let tagged: Value = from_str(TAGGED_YAML).unwrap();
    let _ = group.bench_function("is_json_compatible_false", |b| {
        b.iter(|| is_yaml_json_compatible(black_box(&tagged)));
    });

    // is_yaml_failsafe_compatible
    let _ = group.bench_function("is_failsafe_compatible_true", |b| {
        b.iter(|| is_yaml_failsafe_compatible(black_box(&failsafe_value)));
    });

    let _ = group.bench_function("is_failsafe_compatible_false", |b| {
        b.iter(|| is_yaml_failsafe_compatible(black_box(&json_value)));
    });

    group.finish();
}

// ============================================================================
// 18. Spanned Benchmarks
// ============================================================================

fn bench_spanned(c: &mut Criterion) {
    let mut group = c.benchmark_group("spanned");

    // Spanned::new
    let _ = group.bench_function("new", |b| {
        b.iter(|| Spanned::new(black_box(42u32)));
    });

    // Spanned::into_inner
    let _ = group.bench_function("into_inner", |b| {
        b.iter_batched(
            || Spanned::new(42u32),
            |s| {
                black_box(s.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Spanned deref
    let spanned = Spanned::new(42u32);
    let _ = group.bench_function("deref", |b| {
        b.iter(|| {
            let val: &u32 = black_box(&spanned);
            black_box(val);
        });
    });

    // Deserialize Spanned from YAML
    let _ = group.bench_function("deserialize_scalar", |b| {
        b.iter(|| {
            let _: Spanned<u32> = from_str(black_box("42")).unwrap();
        });
    });

    // Deserialize Spanned in struct
    let spanned_yaml = "port: 8080\nname: test\n";
    let _ = group.bench_function("deserialize_struct", |b| {
        b.iter(|| {
            let _: SpannedConfig = from_str(black_box(spanned_yaml)).unwrap();
        });
    });

    // Serialize Spanned
    let _ = group.bench_function("serialize", |b| {
        let s = Spanned::new(42u32);
        b.iter(|| {
            let _ = to_string(black_box(&s)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// 19. Fmt Wrapper Benchmarks
// ============================================================================

fn bench_fmt_wrappers(c: &mut Criterion) {
    let mut group = c.benchmark_group("fmt_wrappers");

    // FlowSeq
    let flow_seq = FlowSeq(vec![1i32, 2, 3, 4, 5]);
    let _ = group.bench_function("flow_seq_serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&flow_seq)).unwrap();
        });
    });

    let _ = group.bench_function("flow_seq_into_inner", |b| {
        b.iter_batched(
            || FlowSeq(vec![1i32, 2, 3]),
            |s| {
                black_box(s.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // FlowMap
    let mut fmap = std::collections::BTreeMap::new();
    fmap.insert("a", 1i32);
    fmap.insert("b", 2);
    let flow_map = FlowMap(fmap);
    let _ = group.bench_function("flow_map_serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&flow_map)).unwrap();
        });
    });

    let _ = group.bench_function("flow_map_into_inner", |b| {
        let mut m = std::collections::BTreeMap::new();
        m.insert("a", 1i32);
        b.iter_batched(
            || FlowMap(m.clone()),
            |fm| {
                black_box(fm.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // LitStr
    let text = "line one\nline two\nline three\n";
    let lit = LitStr(text);
    let _ = group.bench_function("lit_str_serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&lit)).unwrap();
        });
    });

    let _ = group.bench_function("lit_str_into_inner", |b| {
        b.iter(|| {
            let _ = black_box(LitStr(text).into_inner());
        });
    });

    // LitString
    let lit_string = LitString("line one\nline two\n".into());
    let _ = group.bench_function("lit_string_serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&lit_string)).unwrap();
        });
    });

    let _ = group.bench_function("lit_string_into_inner", |b| {
        b.iter_batched(
            || LitString("hello\nworld\n".into()),
            |ls| {
                black_box(ls.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // FoldStr
    let fold = FoldStr(text);
    let _ = group.bench_function("fold_str_serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&fold)).unwrap();
        });
    });

    let _ = group.bench_function("fold_str_into_inner", |b| {
        b.iter(|| {
            let _ = black_box(FoldStr(text).into_inner());
        });
    });

    // FoldString
    let fold_string = FoldString("line one\nline two\n".into());
    let _ = group.bench_function("fold_string_serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&fold_string)).unwrap();
        });
    });

    let _ = group.bench_function("fold_string_into_inner", |b| {
        b.iter_batched(
            || FoldString("hello\nworld\n".into()),
            |fs| {
                black_box(fs.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Commented
    let commented = Commented::new(42i32, "the answer");
    let _ = group.bench_function("commented_serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&commented)).unwrap();
        });
    });

    let _ = group.bench_function("commented_new", |b| {
        b.iter(|| Commented::new(black_box(42i32), black_box("a comment")));
    });

    let _ = group.bench_function("commented_into_inner", |b| {
        b.iter_batched(
            || Commented::new(42i32, "comment"),
            |c| {
                black_box(c.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // SpaceAfter
    let space = SpaceAfter(42i32);
    let _ = group.bench_function("space_after_serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&space)).unwrap();
        });
    });

    let _ = group.bench_function("space_after_into_inner", |b| {
        b.iter_batched(
            || SpaceAfter(42i32),
            |s| {
                black_box(s.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // From conversions
    let _ = group.bench_function("flow_seq_from", |b| {
        b.iter(|| FlowSeq::from(black_box(vec![1i32, 2, 3])));
    });

    let _ = group.bench_function("space_after_from", |b| {
        b.iter(|| SpaceAfter::from(black_box(42i32)));
    });

    let _ = group.bench_function("lit_string_from_str", |b| {
        b.iter(|| LitString::from(black_box("hello\nworld")));
    });

    let _ = group.bench_function("fold_string_from_str", |b| {
        b.iter(|| FoldString::from(black_box("hello\nworld")));
    });

    group.finish();
}

// ============================================================================
// 20. Anchor Benchmarks
// ============================================================================

fn bench_anchors(c: &mut Criterion) {
    let mut group = c.benchmark_group("anchors");

    // RcAnchor
    let _ = group.bench_function("rc_from_value", |b| {
        b.iter(|| RcAnchor::from(black_box(42i32)));
    });

    let _ = group.bench_function("rc_into_inner", |b| {
        b.iter_batched(
            || RcAnchor::from(42i32),
            |a| {
                black_box(a.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    let _ = group.bench_function("rc_deref", |b| {
        let anchor = RcAnchor::from(42i32);
        b.iter(|| {
            let val: &i32 = black_box(&anchor);
            black_box(val);
        });
    });

    let _ = group.bench_function("rc_clone", |b| {
        let anchor = RcAnchor::from(42i32);
        b.iter(|| black_box(anchor.clone()));
    });

    let _ = group.bench_function("rc_serialize", |b| {
        let anchor = RcAnchor::from(42i32);
        b.iter(|| {
            let _ = to_string(black_box(&anchor)).unwrap();
        });
    });

    let _ = group.bench_function("rc_deserialize", |b| {
        b.iter(|| {
            let _: RcAnchor<i32> = from_str(black_box("42")).unwrap();
        });
    });

    // ArcAnchor
    let _ = group.bench_function("arc_from_value", |b| {
        b.iter(|| ArcAnchor::from(black_box(42i32)));
    });

    let _ = group.bench_function("arc_into_inner", |b| {
        b.iter_batched(
            || ArcAnchor::from(42i32),
            |a| {
                black_box(a.into_inner());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    let _ = group.bench_function("arc_clone", |b| {
        let anchor = ArcAnchor::from(42i32);
        b.iter(|| black_box(anchor.clone()));
    });

    let _ = group.bench_function("arc_serialize", |b| {
        let anchor = ArcAnchor::from(42i32);
        b.iter(|| {
            let _ = to_string(black_box(&anchor)).unwrap();
        });
    });

    let _ = group.bench_function("arc_deserialize", |b| {
        b.iter(|| {
            let _: ArcAnchor<i32> = from_str(black_box("42")).unwrap();
        });
    });

    // RcWeakAnchor
    let _ = group.bench_function("rc_weak_dangling", |b| {
        b.iter(RcWeakAnchor::<i32>::dangling);
    });

    let _ = group.bench_function("rc_weak_upgrade_none", |b| {
        let weak = RcWeakAnchor::<i32>::dangling();
        b.iter(|| black_box(weak.upgrade()));
    });

    let _ = group.bench_function("rc_weak_serialize", |b| {
        let weak = RcWeakAnchor::<i32>::dangling();
        b.iter(|| {
            let _ = to_string(black_box(&weak)).unwrap();
        });
    });

    // ArcWeakAnchor
    let _ = group.bench_function("arc_weak_dangling", |b| {
        b.iter(ArcWeakAnchor::<i32>::dangling);
    });

    let _ = group.bench_function("arc_weak_upgrade_none", |b| {
        let weak = ArcWeakAnchor::<i32>::dangling();
        b.iter(|| black_box(weak.upgrade()));
    });

    let _ = group.bench_function("arc_weak_serialize", |b| {
        let weak = ArcWeakAnchor::<i32>::dangling();
        b.iter(|| {
            let _ = to_string(black_box(&weak)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// 21. Singleton Map Benchmarks
// ============================================================================

fn bench_singleton_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("singleton_map");

    // singleton_map serialize
    let action = SingletonAction {
        action: Action::Get("resource".into()),
    };
    let _ = group.bench_function("serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&action)).unwrap();
        });
    });

    // singleton_map serialize unit variant
    let unit_action = SingletonAction {
        action: Action::Delete,
    };
    let _ = group.bench_function("serialize_unit", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&unit_action)).unwrap();
        });
    });

    // singleton_map serialize struct variant
    let struct_action = SingletonAction {
        action: Action::Set {
            key: "k".into(),
            value: "v".into(),
        },
    };
    let _ = group.bench_function("serialize_struct_variant", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&struct_action)).unwrap();
        });
    });

    // singleton_map deserialize
    let yaml = "action:\n  Get: resource\n";
    let _ = group.bench_function("deserialize", |b| {
        b.iter(|| {
            let _: SingletonAction = from_str(black_box(yaml)).unwrap();
        });
    });

    let unit_yaml = "action:\n  Delete: null\n";
    let _ = group.bench_function("deserialize_unit", |b| {
        b.iter(|| {
            let _: SingletonAction = from_str(black_box(unit_yaml)).unwrap();
        });
    });

    // singleton_map_recursive serialize
    let recursive = RecursiveAction {
        action: Action::Get("resource".into()),
    };
    let _ = group.bench_function("recursive_serialize", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&recursive)).unwrap();
        });
    });

    // singleton_map_recursive deserialize
    let _ = group.bench_function("recursive_deserialize", |b| {
        b.iter(|| {
            let _: RecursiveAction = from_str(black_box(yaml)).unwrap();
        });
    });

    // singleton_map_optional serialize (Some)
    let opt_some = OptionalAction {
        action: Some(Action::Delete),
    };
    let _ = group.bench_function("optional_serialize_some", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&opt_some)).unwrap();
        });
    });

    // singleton_map_optional serialize (None)
    let opt_none = OptionalAction { action: None };
    let _ = group.bench_function("optional_serialize_none", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&opt_none)).unwrap();
        });
    });

    // singleton_map_optional deserialize
    let opt_yaml = "action:\n  Delete: null\n";
    let _ = group.bench_function("optional_deserialize", |b| {
        b.iter(|| {
            let _: OptionalAction = from_str(black_box(opt_yaml)).unwrap();
        });
    });

    // Transform helpers
    let _ = group.bench_function("to_snake_case", |b| {
        b.iter(|| noyalib::with::singleton_map_with::to_snake_case(black_box("GetRequest")));
    });

    let _ = group.bench_function("to_pascal_case", |b| {
        b.iter(|| noyalib::with::singleton_map_with::to_pascal_case(black_box("get_request")));
    });

    let _ = group.bench_function("to_kebab_case", |b| {
        b.iter(|| noyalib::with::singleton_map_with::to_kebab_case(black_box("GetRequest")));
    });

    let _ = group.bench_function("from_kebab_case", |b| {
        b.iter(|| noyalib::with::singleton_map_with::from_kebab_case(black_box("get-request")));
    });

    let _ = group.bench_function("to_lowercase", |b| {
        b.iter(|| noyalib::with::singleton_map_with::to_lowercase(black_box("GetRequest")));
    });

    let _ = group.bench_function("to_uppercase", |b| {
        b.iter(|| noyalib::with::singleton_map_with::to_uppercase(black_box("GetRequest")));
    });

    group.finish();
}

// ============================================================================
// 22. Error & Location Benchmarks
// ============================================================================

fn bench_error_location(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_location");

    // Location construction
    let _ = group.bench_function("location_new", |b| {
        b.iter(|| Location::new(black_box(10), black_box(5), black_box(100)));
    });

    // Location::from_index
    let source = "line one\nline two\nline three\nline four\n";
    let _ = group.bench_function("location_from_index_start", |b| {
        b.iter(|| Location::from_index(black_box(source), black_box(0)));
    });

    let _ = group.bench_function("location_from_index_mid", |b| {
        b.iter(|| Location::from_index(black_box(source), black_box(20)));
    });

    let _ = group.bench_function("location_from_index_end", |b| {
        b.iter(|| Location::from_index(black_box(source), black_box(38)));
    });

    // Location accessors
    let loc = Location::new(10, 5, 100);
    let _ = group.bench_function("location_line", |b| {
        b.iter(|| black_box(&loc).line());
    });

    let _ = group.bench_function("location_column", |b| {
        b.iter(|| black_box(&loc).column());
    });

    let _ = group.bench_function("location_index", |b| {
        b.iter(|| black_box(&loc).index());
    });

    // Location Display
    let _ = group.bench_function("location_display", |b| {
        b.iter(|| format!("{}", black_box(&loc)));
    });

    // Error construction
    let _ = group.bench_function("error_parse_at", |b| {
        b.iter(|| {
            Error::parse_at(
                black_box("unexpected token"),
                black_box(source),
                black_box(10),
            )
        });
    });

    let _ = group.bench_function("error_deserialize_at", |b| {
        b.iter(|| {
            Error::deserialize_at(black_box("type mismatch"), black_box(source), black_box(10))
        });
    });

    // Error::location()
    let err = Error::parse_at("unexpected token", source, 10);
    let _ = group.bench_function("error_location", |b| {
        b.iter(|| black_box(&err).location());
    });

    // Error::format_with_source
    let _ = group.bench_function("error_format_with_source", |b| {
        b.iter(|| err.format_with_source(black_box(source)));
    });

    // Error Display
    let _ = group.bench_function("error_display", |b| {
        b.iter(|| format!("{}", black_box(&err)));
    });

    // Error into_shared / from_shared
    let _ = group.bench_function("error_into_shared", |b| {
        b.iter_batched(
            || Error::parse_at("test error", source, 5),
            |e| {
                black_box(e.into_shared());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    let shared: Arc<Error> = Error::parse_at("shared error", source, 5).into_shared();
    let _ = group.bench_function("error_from_shared", |b| {
        b.iter(|| black_box(Error::from_shared(shared.clone())));
    });

    let shared_err = Error::Shared(shared.clone());
    let _ = group.bench_function("error_is_shared", |b| {
        b.iter(|| black_box(&shared_err).is_shared());
    });

    let _ = group.bench_function("error_as_inner", |b| {
        b.iter(|| black_box(&shared_err).as_inner());
    });

    group.finish();
}

// ============================================================================
// 23. Value Serde & Display Benchmarks
// ============================================================================

fn bench_value_serde(c: &mut Criterion) {
    let mut group = c.benchmark_group("value_serde");

    // Value::serialize (various types)
    let null = Value::Null;
    let _ = group.bench_function("serialize_null", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&null)).unwrap();
        });
    });

    let bool_val = Value::from(true);
    let _ = group.bench_function("serialize_bool", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&bool_val)).unwrap();
        });
    });

    let int_val = Value::from(42i64);
    let _ = group.bench_function("serialize_integer", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&int_val)).unwrap();
        });
    });

    let float_val = Value::from(2.75f64);
    let _ = group.bench_function("serialize_float", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&float_val)).unwrap();
        });
    });

    let str_val = Value::from("hello world");
    let _ = group.bench_function("serialize_string", |b| {
        b.iter(|| {
            let _ = to_string(black_box(&str_val)).unwrap();
        });
    });

    // Value Display
    let _ = group.bench_function("display_null", |b| {
        b.iter(|| format!("{}", black_box(&null)));
    });

    let _ = group.bench_function("display_mapping", |b| {
        let v: Value = from_str(SIMPLE_YAML).unwrap();
        b.iter(|| format!("{}", black_box(&v)));
    });

    let _ = group.bench_function("display_sequence", |b| {
        let v: Value = from_str(SEQUENCE_YAML).unwrap();
        b.iter(|| format!("{}", black_box(&v)));
    });

    // Value PartialEq
    let a: Value = from_str(SIMPLE_YAML).unwrap();
    let b_val: Value = from_str(SIMPLE_YAML).unwrap();
    let _ = group.bench_function("eq_same", |b| {
        b.iter(|| black_box(&a) == black_box(&b_val));
    });

    let c_val: Value = from_str(NESTED_YAML).unwrap();
    let _ = group.bench_function("eq_different", |b| {
        b.iter(|| black_box(&a) == black_box(&c_val));
    });

    // Value Ord
    let _ = group.bench_function("cmp", |b| {
        b.iter(|| black_box(&a).cmp(black_box(&c_val)));
    });

    // Value Hash
    let _ = group.bench_function("hash", |b| {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            black_box(&a).hash(&mut hasher);
            black_box(hasher.finish());
        });
    });

    // Value Index operators
    let nested: Value = from_str(NESTED_YAML).unwrap();
    let _ = group.bench_function("index_str", |b| {
        b.iter(|| {
            let _ = &black_box(&nested)["server"];
        });
    });

    let seq: Value = from_str("- a\n- b\n- c\n").unwrap();
    let _ = group.bench_function("index_usize", |b| {
        b.iter(|| {
            let _ = &black_box(&seq)[1];
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches_serde,
    bench_deserialize,
    bench_serialize,
    bench_serialize_multi,
    bench_serializer_config,
    bench_parser_config,
);

criterion_group!(
    benches_parse,
    bench_parse,
    bench_parse_depth,
    bench_parse_width,
    bench_roundtrip,
    bench_loader,
);

criterion_group!(
    benches_value,
    bench_value_access,
    bench_value_construct,
    bench_value_mutate,
    bench_mapping,
    bench_mapping_any,
);

criterion_group!(
    benches_types,
    bench_number,
    bench_tag,
    bench_path,
    bench_schema,
    bench_spanned,
);

criterion_group!(
    benches_extras,
    bench_fmt_wrappers,
    bench_anchors,
    bench_singleton_map,
    bench_error_location,
    bench_value_serde,
);

criterion_main!(
    benches_serde,
    benches_parse,
    benches_value,
    benches_types,
    benches_extras,
);
