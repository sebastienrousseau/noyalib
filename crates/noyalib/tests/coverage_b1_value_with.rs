// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Integration coverage for `Value`'s serde impl (`value/serde_impl.rs`),
//! the `Tag` / `TaggedValue` types (`value/tag.rs`) and the four
//! `with/singleton_map*` adapters.
//!
//! Every adapter is applied through its real, documented `#[serde(with =
//! "noyalib::with::…")]` path (mirrored from the module docs) and driven
//! serialize -> YAML -> deserialize, asserting round-trip equality. The
//! `Value` variants are exercised both through the public
//! `to_string`/`from_str` YAML round-trip and through the borrowed
//! `&Value` deserializer via `from_value`.

#![allow(
    missing_docs,
    dead_code,
    unused_results,
    unused_must_use,
    non_snake_case,
    clippy::all
)]
#![allow(clippy::unwrap_used)]

use noyalib::{
    Mapping, MaybeTag, Number, Tag, TaggedValue, Value, check_for_tag, from_str, from_value,
    nobang, to_string,
};

// ---------------------------------------------------------------------------
// Value: YAML round-trips for every variant
// ---------------------------------------------------------------------------

/// Round-trip a `Value` through YAML and assert structural equality.
fn yaml_round_trip(v: &Value) -> Value {
    let yaml = to_string(v).unwrap();
    from_str::<Value>(&yaml).unwrap()
}

#[test]
fn value_null_round_trip() {
    let v = Value::Null;
    assert_eq!(yaml_round_trip(&v), Value::Null);
}

#[test]
fn value_bool_round_trip() {
    for b in [true, false] {
        let v = Value::Bool(b);
        assert_eq!(yaml_round_trip(&v), v);
    }
}

#[test]
fn value_integer_round_trip() {
    for n in [0_i64, 42, -17, i64::MAX, i64::MIN] {
        let v = Value::Number(Number::Integer(n));
        assert_eq!(yaml_round_trip(&v), v);
    }
}

#[test]
fn value_float_round_trip() {
    let v = Value::Number(Number::Float(2.5));
    assert_eq!(yaml_round_trip(&v), v);
}

#[test]
fn value_string_round_trip() {
    let v = Value::String("hello world".to_owned());
    assert_eq!(yaml_round_trip(&v), v);
}

#[test]
fn value_sequence_round_trip() {
    let v = Value::Sequence(vec![
        Value::from(1_i64),
        Value::from("two"),
        Value::Bool(true),
        Value::Null,
    ]);
    assert_eq!(yaml_round_trip(&v), v);
}

#[test]
fn value_mapping_round_trip() {
    let mut m = Mapping::new();
    let _ = m.insert("name", Value::from("noya"));
    let _ = m.insert("count", Value::from(3_i64));
    let v = Value::Mapping(m);
    assert_eq!(yaml_round_trip(&v), v);
}

#[test]
fn value_nested_round_trip() {
    let mut inner = Mapping::new();
    let _ = inner.insert(
        "items",
        Value::Sequence(vec![Value::from(1_i64), Value::from(2_i64)]),
    );
    let _ = inner.insert("flag", Value::Bool(false));
    let mut outer = Mapping::new();
    let _ = outer.insert("nested", Value::Mapping(inner));
    let v = Value::Mapping(outer);
    assert_eq!(yaml_round_trip(&v), v);
}

// ---------------------------------------------------------------------------
// Value: borrowed `&Value` deserializer via `from_value`
// ---------------------------------------------------------------------------

#[test]
fn from_value_scalars() {
    let b: bool = from_value(&Value::Bool(true)).unwrap();
    assert!(b);
    let n: i64 = from_value(&Value::from(99_i64)).unwrap();
    assert_eq!(n, 99);
    let f: f64 = from_value(&Value::from(1.25_f64)).unwrap();
    assert!((f - 1.25).abs() < f64::EPSILON);
    let s: String = from_value(&Value::from("text")).unwrap();
    assert_eq!(s, "text");
    from_value::<()>(&Value::Null).unwrap();
}

#[test]
fn from_value_sequence_into_vec() {
    let v = Value::Sequence(vec![
        Value::from(1_i64),
        Value::from(2_i64),
        Value::from(3_i64),
    ]);
    let got: Vec<i64> = from_value(&v).unwrap();
    assert_eq!(got, vec![1, 2, 3]);
}

#[test]
fn from_value_mapping_into_struct() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Point {
        x: i64,
        y: i64,
    }
    let mut m = Mapping::new();
    let _ = m.insert("x", Value::from(4_i64));
    let _ = m.insert("y", Value::from(7_i64));
    let got: Point = from_value(&Value::Mapping(m)).unwrap();
    assert_eq!(got, Point { x: 4, y: 7 });
}

#[test]
fn from_value_type_mismatch_errors() {
    // A string cannot deserialize into an integer — drives the visitor
    // error path of the borrowed `&Value` deserializer.
    let r: Result<i64, _> = from_value(&Value::from("not a number"));
    assert!(r.is_err());
}

// A shared enum exercised through every deserialize entry point below.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
enum Shape {
    Circle,
    Named(String),
    Pair(i64, i64),
    Rect { w: i64, h: i64 },
}

#[test]
fn from_value_enum_from_bare_string() {
    // `Value::String` -> unit variant via the string-enum arm.
    let got: Shape = from_value(&Value::from("Circle")).unwrap();
    assert_eq!(got, Shape::Circle);
}

#[test]
fn from_value_enum_from_singleton_mapping() {
    // A single-entry `Value::Mapping` drives the `deserialize_enum`
    // fallback arm (`_ => deserialize_any` -> `visit_map`).
    let mut m = Mapping::new();
    let _ = m.insert("Named", Value::from("hi"));
    let got: Shape = from_value(&Value::Mapping(m)).unwrap();
    assert_eq!(got, Shape::Named("hi".to_owned()));
}

#[cfg(feature = "lossless-u64")]
#[test]
fn from_value_unsigned_visits_u64() {
    // The `lossless-u64` visitor arm of the borrowed deserializer.
    let v = noyalib::to_value(&u64::MAX).unwrap();
    let n: u64 = from_value(&v).unwrap();
    assert_eq!(n, u64::MAX);
}

// ---------------------------------------------------------------------------
// TaggedValue: serialize + deserialize
// ---------------------------------------------------------------------------

#[test]
fn tagged_value_serializes_as_single_entry_map() {
    let tv = TaggedValue::new(Tag::new("!custom"), Value::from(7_i64));
    let yaml = to_string(&tv).unwrap();
    assert!(yaml.contains("custom"), "{yaml}");
    assert!(yaml.contains('7'), "{yaml}");
}

#[test]
fn tagged_value_from_value_single_entry_map() {
    // A single-entry mapping deserializes into a `TaggedValue`, driving
    // the `ValueMapAccess` key/value seeds of the borrowed deserializer.
    let mut m = Mapping::new();
    let _ = m.insert("!label", Value::from("payload"));
    let tv: TaggedValue = from_value(&Value::Mapping(m)).unwrap();
    assert_eq!(tv.tag().as_str(), "!label");
    assert_eq!(tv.value().as_str(), Some("payload"));
}

#[test]
fn tagged_value_from_str_single_entry_map() {
    let tv: TaggedValue = from_str("'!ts': '2024-01-01'\n").unwrap();
    assert_eq!(tv.tag(), &Tag::new("!ts"));
    assert_eq!(tv.value().as_str(), Some("2024-01-01"));
}

#[test]
fn tagged_value_into_parts_and_value_mut() {
    let mut tv = TaggedValue::new(Tag::new("!Color"), Value::from("#000"));
    *tv.value_mut() = Value::from("#ff8800");
    let (tag, value) = tv.into_parts();
    assert_eq!(tag.as_str(), "!Color");
    assert_eq!(value.as_str(), Some("#ff8800"));
}

// ---------------------------------------------------------------------------
// Tag: edge cases
// ---------------------------------------------------------------------------

#[test]
fn nobang_strips_single_leading_bang() {
    assert_eq!(nobang("!foo"), "foo");
    assert_eq!(nobang("foo"), "foo");
    assert_eq!(nobang("!!int"), "!int");
}

#[test]
fn tag_equality_ignores_bang_prefix() {
    assert_eq!(Tag::new("!foo"), Tag::new("foo"));
    assert_ne!(Tag::new("!foo"), Tag::new("bar"));
    // `!!str` keeps one `!` after `nobang`, so it differs from `!str`.
    assert_ne!(Tag::new("!!str"), Tag::new("!str"));
}

#[test]
fn tag_ordering_and_sorting() {
    // Drives `Ord::cmp` (compares the un-banged forms).
    let mut tags = vec![Tag::new("!charlie"), Tag::new("alpha"), Tag::new("!bravo")];
    tags.sort();
    let ordered: Vec<&str> = tags.iter().map(Tag::nobang).collect();
    assert_eq!(ordered, vec!["alpha", "bravo", "charlie"]);
    assert!(Tag::new("!a") < Tag::new("b"));
}

#[test]
fn tag_nobang_method_and_accessors() {
    assert_eq!(Tag::new("!Custom").nobang(), "Custom");
    assert_eq!(Tag::new("!!str").nobang(), "!str");
    assert_eq!(Tag::new("plain").nobang(), "plain");
    assert_eq!(Tag::new("!Custom").as_str(), "!Custom");
    assert_eq!(Tag::new("!X").into_string(), "!X");
    let t = Tag::new("!Y");
    let r: &str = t.as_ref();
    assert_eq!(r, "!Y");
}

#[test]
fn tag_from_bytes_try_from() {
    let t = Tag::try_from(&b"!bytes"[..]).unwrap();
    assert_eq!(t.as_str(), "!bytes");
    // Invalid UTF-8 is an error.
    assert!(Tag::try_from(&[0xff, 0xfe][..]).is_err());
}

#[test]
fn check_for_tag_classifies_values() {
    match check_for_tag(&"!mytag") {
        MaybeTag::Tag(s) => assert_eq!(s, "!mytag"),
        MaybeTag::NotTag(_) => panic!("expected a tag"),
    }
    match check_for_tag(&"plain") {
        MaybeTag::NotTag(s) => assert_eq!(s, "plain"),
        MaybeTag::Tag(_) => panic!("expected not-a-tag"),
    }
}

// ---------------------------------------------------------------------------
// with::singleton_map
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
enum Status {
    Active,
    Named(String),
    Coords(i64, i64),
    Error { code: i32, message: String },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct SingletonHolder {
    name: String,
    #[serde(with = "noyalib::with::singleton_map")]
    status: Status,
}

fn singleton_round_trip(status: Status) {
    let holder = SingletonHolder {
        name: "task".to_owned(),
        status,
    };
    let yaml = to_string(&holder).unwrap();
    let parsed: SingletonHolder = from_str(&yaml).unwrap();
    assert_eq!(parsed, holder);
}

#[test]
fn singleton_map_all_variant_kinds_round_trip() {
    // Unit, newtype, tuple and struct externally-tagged variants each
    // round-trip through the singleton-map adapter.
    singleton_round_trip(Status::Active);
    singleton_round_trip(Status::Named("hello".to_owned()));
    singleton_round_trip(Status::Coords(10, 20));
    singleton_round_trip(Status::Error {
        code: 500,
        message: "boom".to_owned(),
    });
}

#[test]
fn singleton_map_struct_variant_yaml_shape() {
    let holder = SingletonHolder {
        name: "task".to_owned(),
        status: Status::Error {
            code: 42,
            message: "oops".to_owned(),
        },
    };
    let yaml = to_string(&holder).unwrap();
    assert!(yaml.contains("Error"), "{yaml}");
    assert!(yaml.contains("code: 42"), "{yaml}");
}

#[test]
fn singleton_map_deserialize_unknown_variant_errors() {
    let yaml = "name: task\nstatus:\n  Nonexistent: null\n";
    let r: Result<SingletonHolder, _> = from_str(yaml);
    assert!(r.is_err());
}

// ---------------------------------------------------------------------------
// with::singleton_map_optional
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct OptionalHolder {
    name: String,
    #[serde(
        with = "noyalib::with::singleton_map_optional",
        skip_serializing_if = "Option::is_none",
        default
    )]
    status: Option<Status>,
}

#[test]
fn singleton_map_optional_some_variants() {
    for status in [
        Status::Active,
        Status::Error {
            code: 1,
            message: "e".to_owned(),
        },
    ] {
        let holder = OptionalHolder {
            name: "x".to_owned(),
            status: Some(status),
        };
        let yaml = to_string(&holder).unwrap();
        let parsed: OptionalHolder = from_str(&yaml).unwrap();
        assert_eq!(parsed, holder);
    }
}

#[test]
fn singleton_map_optional_none_skipped() {
    let holder = OptionalHolder {
        name: "x".to_owned(),
        status: None,
    };
    let yaml = to_string(&holder).unwrap();
    assert!(!yaml.contains("status"), "{yaml}");
    let parsed: OptionalHolder = from_str(&yaml).unwrap();
    assert_eq!(parsed, holder);
}

#[test]
fn singleton_map_optional_explicit_null_deserializes_none() {
    let parsed: OptionalHolder = from_str("name: x\nstatus: null\n").unwrap();
    assert_eq!(
        parsed,
        OptionalHolder {
            name: "x".to_owned(),
            status: None,
        }
    );
}

// ---------------------------------------------------------------------------
// with::singleton_map_recursive
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
enum Inner {
    A,
    B { value: i32 },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
enum Outer {
    Single(Inner),
    Multiple(Vec<Inner>),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct RecursiveHolder {
    #[serde(with = "noyalib::with::singleton_map_recursive")]
    items: Vec<Outer>,
}

#[test]
fn singleton_map_recursive_nested_enums() {
    let holder = RecursiveHolder {
        items: vec![
            Outer::Single(Inner::A),
            Outer::Multiple(vec![Inner::A, Inner::B { value: 7 }]),
        ],
    };
    let yaml = to_string(&holder).unwrap();
    let parsed: RecursiveHolder = from_str(&yaml).unwrap();
    assert_eq!(parsed, holder);
}

#[test]
fn singleton_map_recursive_map_of_enums() {
    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct MapHolder {
        #[serde(with = "noyalib::with::singleton_map_recursive")]
        data: std::collections::BTreeMap<String, Inner>,
    }
    let mut data = std::collections::BTreeMap::new();
    let _ = data.insert("one".to_owned(), Inner::A);
    let _ = data.insert("two".to_owned(), Inner::B { value: 3 });
    let holder = MapHolder { data };
    let yaml = to_string(&holder).unwrap();
    let parsed: MapHolder = from_str(&yaml).unwrap();
    assert_eq!(parsed, holder);
}

// ---------------------------------------------------------------------------
// with::singleton_map_with (custom key transform)
// ---------------------------------------------------------------------------

/// A `#[serde(with = "snake_case")]` module mirroring the module docs:
/// PascalCase variant names are emitted as snake_case and read back via
/// `to_pascal_case`.
mod snake_case {
    pub(crate) fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: serde::Serialize,
        S: serde_core::Serializer,
    {
        noyalib::with::singleton_map_with::serialize_with(
            value,
            serializer,
            noyalib::with::singleton_map_with::to_snake_case,
        )
    }

    pub(crate) fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: serde_core::de::DeserializeOwned + 'static,
        D: serde_core::Deserializer<'de>,
    {
        noyalib::with::singleton_map_with::deserialize_with(
            deserializer,
            noyalib::with::singleton_map_with::to_pascal_case,
        )
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
enum HttpMethod {
    GetRequest,
    PostData { body: String },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct ApiCall {
    #[serde(with = "snake_case")]
    method: HttpMethod,
}

#[test]
fn singleton_map_with_unit_variant_snake_case() {
    let call = ApiCall {
        method: HttpMethod::GetRequest,
    };
    let yaml = to_string(&call).unwrap();
    assert!(yaml.contains("get_request"), "{yaml}");
    let parsed: ApiCall = from_str(&yaml).unwrap();
    assert_eq!(parsed, call);
}

#[test]
fn singleton_map_with_case_helpers() {
    use noyalib::with::singleton_map_with::{
        from_kebab_case, to_kebab_case, to_lowercase, to_pascal_case, to_snake_case, to_uppercase,
    };
    assert_eq!(to_snake_case("GetRequest"), "get_request");
    assert_eq!(to_pascal_case("get_request"), "GetRequest");
    assert_eq!(to_kebab_case("GetRequest"), "get-request");
    assert_eq!(from_kebab_case("get-request"), "GetRequest");
    assert_eq!(to_lowercase("ABC"), "abc");
    assert_eq!(to_uppercase("abc"), "ABC");
}
