//! Phase 5 feature tests: Value merge utilities and error context.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Error, Location, Value, from_str};

// ============================================================================
// Value Merge Tests
// ============================================================================

#[test]
fn test_merge_simple_mappings() {
    let mut base: Value = from_str("a: 1\nb: 2\n").unwrap();
    let other: Value = from_str("b: 3\nc: 4\n").unwrap();

    base.merge(other);

    assert_eq!(base.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(base.get("b").unwrap().as_i64(), Some(3)); // overwritten
    assert_eq!(base.get("c").unwrap().as_i64(), Some(4)); // added
}

#[test]
fn test_merge_nested_mappings() {
    let mut base: Value = from_str(
        r#"
server:
  host: localhost
  port: 8080
"#,
    )
    .unwrap();

    let other: Value = from_str(
        r#"
server:
  port: 9090
  ssl: true
"#,
    )
    .unwrap();

    base.merge(other);

    assert_eq!(
        base.get_path("server.host").unwrap().as_str(),
        Some("localhost")
    );
    assert_eq!(base.get_path("server.port").unwrap().as_i64(), Some(9090));
    assert_eq!(base.get_path("server.ssl").unwrap().as_bool(), Some(true));
}

#[test]
fn test_merge_sequence_replace() {
    let mut base: Value = from_str("items:\n  - a\n  - b\n").unwrap();
    let other: Value = from_str("items:\n  - x\n  - y\n  - z\n").unwrap();

    base.merge(other);

    let items = base.get("items").unwrap().as_sequence().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].as_str(), Some("x"));
    assert_eq!(items[1].as_str(), Some("y"));
    assert_eq!(items[2].as_str(), Some("z"));
}

#[test]
fn test_merge_concat_sequences() {
    let mut base: Value = from_str("items:\n  - a\n  - b\n").unwrap();
    let other: Value = from_str("items:\n  - c\n  - d\n").unwrap();

    base.merge_concat(other);

    let items = base.get("items").unwrap().as_sequence().unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0].as_str(), Some("a"));
    assert_eq!(items[1].as_str(), Some("b"));
    assert_eq!(items[2].as_str(), Some("c"));
    assert_eq!(items[3].as_str(), Some("d"));
}

#[test]
fn test_merge_scalar_replace() {
    let mut base: Value = Value::from(42);
    let other: Value = Value::from(100);

    base.merge(other);

    assert_eq!(base.as_i64(), Some(100));
}

#[test]
fn test_merge_null_replaces() {
    let mut base: Value = from_str("value: 42\n").unwrap();
    let other: Value = from_str("value: null\n").unwrap();

    base.merge(other);

    assert!(base.get("value").unwrap().is_null());
}

#[test]
fn test_merge_deep_nesting() {
    let mut base: Value = from_str(
        r#"
level1:
  level2:
    level3:
      a: 1
      b: 2
"#,
    )
    .unwrap();

    let other: Value = from_str(
        r#"
level1:
  level2:
    level3:
      b: 3
      c: 4
"#,
    )
    .unwrap();

    base.merge(other);

    assert_eq!(
        base.get_path("level1.level2.level3.a").unwrap().as_i64(),
        Some(1)
    );
    assert_eq!(
        base.get_path("level1.level2.level3.b").unwrap().as_i64(),
        Some(3)
    );
    assert_eq!(
        base.get_path("level1.level2.level3.c").unwrap().as_i64(),
        Some(4)
    );
}

// ============================================================================
// Value Insert/Remove Tests
// ============================================================================

#[test]
fn test_value_insert() {
    let mut value: Value = from_str("a: 1\n").unwrap();

    let _ = value.insert("b", Value::from(2));
    let _ = value.insert("c", Value::from("hello"));

    assert_eq!(value.get("b").unwrap().as_i64(), Some(2));
    assert_eq!(value.get("c").unwrap().as_str(), Some("hello"));
}

#[test]
fn test_value_insert_overwrite() {
    let mut value: Value = from_str("a: 1\n").unwrap();

    let old = value.insert("a", Value::from(2));

    assert_eq!(old.unwrap().as_i64(), Some(1));
    assert_eq!(value.get("a").unwrap().as_i64(), Some(2));
}

#[test]
fn test_value_remove() {
    let mut value: Value = from_str("a: 1\nb: 2\nc: 3\n").unwrap();

    let removed = value.remove("b");

    assert_eq!(removed.unwrap().as_i64(), Some(2));
    assert!(value.get("b").is_none());
    assert!(value.get("a").is_some());
    assert!(value.get("c").is_some());
}

#[test]
fn test_value_remove_nonexistent() {
    let mut value: Value = from_str("a: 1\n").unwrap();

    let removed = value.remove("nonexistent");

    assert!(removed.is_none());
}

#[test]
fn test_value_insert_on_non_mapping() {
    let mut value = Value::from(42);

    let result = value.insert("key", Value::from(1));

    assert!(result.is_none());
    assert_eq!(value.as_i64(), Some(42)); // unchanged
}

// ============================================================================
// Error Context Tests
// ============================================================================

#[test]
fn test_location_from_index() {
    let source = "line1\nline2\nline3\n";

    // Start of line 1
    let loc = Location::from_index(source, 0);
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.column(), 1);

    // Middle of line 1
    let loc = Location::from_index(source, 3);
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.column(), 4);

    // Start of line 2
    let loc = Location::from_index(source, 6);
    assert_eq!(loc.line(), 2);
    assert_eq!(loc.column(), 1);

    // Middle of line 3
    let loc = Location::from_index(source, 14);
    assert_eq!(loc.line(), 3);
    assert_eq!(loc.column(), 3);
}

#[test]
fn test_error_parse_at() {
    let source = "name: test\nport: invalid\n";
    let error = Error::parse_at("expected integer", source, 17);

    assert!(error.location().is_some());
    let loc = error.location().unwrap();
    assert_eq!(loc.line(), 2);
    assert_eq!(loc.column(), 7);
}

#[test]
fn test_error_format_with_source() {
    let source = "name: test\nport: invalid\n";
    let error = Error::parse_at("expected integer", source, 17);

    let formatted = error.format_with_source(source);

    assert!(formatted.contains("error:"));
    assert!(formatted.contains("line 2"));
    assert!(formatted.contains("port: invalid"));
    assert!(formatted.contains("^"));
}

#[test]
fn test_error_format_without_location() {
    let error = Error::Parse("generic error".to_string());

    let formatted = error.format_with_source("some source");

    // Should just return the error message without source context
    assert!(formatted.contains("generic error"));
    assert!(!formatted.contains("^"));
}

#[test]
fn test_error_format_with_invalid_line_number() {
    // Create an error with a location that has line number beyond source length
    // Using location line 100 on a source with only 1 line
    let loc = Location::new(100, 1, 0); // line 100 is way beyond "short"
    let error = Error::ParseWithLocation {
        message: "error at invalid line".to_string(),
        location: loc,
    };

    let formatted = error.format_with_source("short");

    // Should fall back to just the error message when line is out of range
    assert!(formatted.contains("error at invalid line"));
}

#[test]
fn test_error_deserialize_at() {
    let source = "items:\n  - bad\n";
    let error = Error::deserialize_at("invalid item", source, 10);

    assert!(error.location().is_some());
    let loc = error.location().unwrap();
    assert_eq!(loc.line(), 2);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_merge_empty_mapping() {
    let mut base: Value = from_str("a: 1\n").unwrap();
    let other: Value = from_str("{}").unwrap();

    base.merge(other);

    // Base should be unchanged
    assert_eq!(base.get("a").unwrap().as_i64(), Some(1));
}

#[test]
fn test_merge_into_empty_mapping() {
    let mut base: Value = from_str("{}").unwrap();
    let other: Value = from_str("a: 1\nb: 2\n").unwrap();

    base.merge(other);

    assert_eq!(base.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(base.get("b").unwrap().as_i64(), Some(2));
}

#[test]
fn test_merge_concat_empty_sequences() {
    let mut base: Value = from_str("items: []\n").unwrap();
    let other: Value = from_str("items:\n  - a\n").unwrap();

    base.merge_concat(other);

    let items = base.get("items").unwrap().as_sequence().unwrap();
    assert_eq!(items.len(), 1);
}

// ============================================================================
// Additional Coverage Tests
// ============================================================================

#[test]
fn test_location_new() {
    let loc = Location::new(5, 10, 100);
    assert_eq!(loc.line(), 5);
    assert_eq!(loc.column(), 10);
    assert_eq!(loc.index(), 100);
}

#[test]
fn test_location_index() {
    let loc = Location::from_index("hello\nworld", 6);
    assert_eq!(loc.line(), 2);
    assert_eq!(loc.column(), 1);
}

#[test]
fn test_location_from_index_middle_of_line() {
    // Test index in the middle of a line to trigger the break condition
    let source = "abc";
    let loc = Location::from_index(source, 1);
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.column(), 2);
}

#[test]
fn test_location_from_index_at_newline() {
    // Test index exactly at a newline
    let source = "ab\ncd";
    let loc = Location::from_index(source, 2);
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.column(), 3);
}

#[test]
fn test_error_display() {
    let err = Error::Parse("test error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("test error"));

    let err = Error::Serialize("serialize error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("serialize error"));

    let err = Error::Deserialize("deserialize error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("deserialize error"));

    let err = Error::Invalid("invalid error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("invalid error"));

    let err = Error::TypeMismatch {
        expected: "string",
        found: "integer".to_string(),
    };
    let display = format!("{}", err);
    assert!(display.contains("string"));
    assert!(display.contains("integer"));

    let err = Error::MissingField("name".to_string());
    let display = format!("{}", err);
    assert!(display.contains("name"));

    let err = Error::UnknownField("extra".to_string());
    let display = format!("{}", err);
    assert!(display.contains("extra"));

    let err = Error::RecursionLimitExceeded { depth: 100 };
    let display = format!("{}", err);
    assert!(display.contains("100"));

    let err = Error::Custom("custom error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("custom error"));
}

#[test]
fn test_error_parse_at_multiline() {
    let err = Error::parse_at("error msg", "hello\nworld\nthird", 12);
    let display = format!("{}", err);
    assert!(display.contains("error msg"));
    assert!(display.contains("line 3"));
}

#[test]
fn test_error_deserialize_at_with_column() {
    let err = Error::deserialize_at("deser error", "hello world", 6);
    let display = format!("{}", err);
    assert!(display.contains("deser error"));
    assert!(display.contains("column 7"));
}

#[test]
fn test_error_format_with_source_multiline() {
    let source = "line1\nline2\nline3\nline4";
    let err = Error::parse_at("test error", source, 12);
    let formatted = err.format_with_source(source);
    assert!(formatted.contains("test error"));
    assert!(formatted.contains("^"));
}

#[test]
fn test_value_get_path_deep() {
    let yaml = r#"
level1:
  level2:
    level3:
      level4:
        value: deep
"#;
    let value: Value = from_str(yaml).unwrap();
    assert_eq!(
        value
            .get_path("level1.level2.level3.level4.value")
            .unwrap()
            .as_str(),
        Some("deep")
    );
}

#[test]
fn test_value_get_path_mut_creates_value() {
    let yaml = "a:\n  b: 1\n";
    let mut value: Value = from_str(yaml).unwrap();

    if let Some(b) = value.get_path_mut("a.b") {
        *b = Value::from(999);
    }

    assert_eq!(value.get_path("a.b").unwrap().as_i64(), Some(999));
}

#[test]
fn test_value_get_path_nonexistent() {
    let yaml = "a: 1\n";
    let value: Value = from_str(yaml).unwrap();

    assert!(value.get_path("b").is_none());
    assert!(value.get_path("a.b.c").is_none());
    assert!(value.get_path("items[0]").is_none());
}
