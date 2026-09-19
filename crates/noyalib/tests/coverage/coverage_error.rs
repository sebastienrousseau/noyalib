//! Error module coverage tests — all Error variants, Location,
//! format_with_source edge cases.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::sync::Arc;

use noyalib::{Error, Location};

// ============================================================================
// Location
// ============================================================================

#[test]
fn location_new_and_accessors() {
    let loc = Location::new(10, 5, 100);
    assert_eq!(loc.line(), 10);
    assert_eq!(loc.column(), 5);
    assert_eq!(loc.index(), 100);
}

#[test]
fn location_default() {
    let loc = Location::default();
    assert_eq!(loc.line(), 0);
    assert_eq!(loc.column(), 0);
    assert_eq!(loc.index(), 0);
}

#[test]
fn location_display() {
    let loc = Location::new(3, 7, 42);
    let s = format!("{loc}");
    assert_eq!(s, "line 3, column 7");
}

#[test]
fn location_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let _ = set.insert(Location::new(1, 1, 0));
    let _ = set.insert(Location::new(1, 1, 0)); // duplicate
    let _ = set.insert(Location::new(2, 1, 5));
    assert_eq!(set.len(), 2);
}

#[test]
fn location_clone_and_eq() {
    let loc = Location::new(1, 2, 3);
    let loc2 = loc;
    assert_eq!(loc, loc2);
}

#[test]
fn location_from_index_start() {
    let source = "hello\nworld\n";
    let loc = Location::from_index(source, 0);
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.column(), 1);
}

#[test]
fn location_from_index_second_line() {
    let source = "hello\nworld\n";
    let loc = Location::from_index(source, 6);
    assert_eq!(loc.line(), 2);
    assert_eq!(loc.column(), 1);
}

#[test]
fn location_from_index_middle_of_line() {
    let source = "hello\nworld\n";
    let loc = Location::from_index(source, 8);
    assert_eq!(loc.line(), 2);
    assert_eq!(loc.column(), 3);
}

#[test]
fn location_from_index_past_end() {
    let source = "ab";
    let loc = Location::from_index(source, 100);
    // Should iterate through all chars and stop
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.index(), 100);
}

#[test]
fn location_from_index_multibyte() {
    let source = "héllo\nworld";
    let loc = Location::from_index(source, 7); // past 'é' (2 bytes)
    assert_eq!(loc.line(), 2);
    assert_eq!(loc.column(), 1);
}

// ============================================================================
// Error variants display
// ============================================================================

#[test]
fn error_parse_display() {
    let e = Error::Parse("bad yaml".into());
    assert_eq!(e.to_string(), "YAML parse error: bad yaml");
}

#[test]
fn error_parse_with_location_display() {
    let e = Error::ParseWithLocation {
        message: "unexpected token".into(),
        location: Location::new(3, 5, 20),
    };
    let s = e.to_string();
    assert!(s.contains("line 3, column 5"));
    assert!(s.contains("unexpected token"));
}

#[test]
fn error_serialize_display() {
    let e = Error::Serialize("cannot serialize".into());
    assert!(e.to_string().contains("cannot serialize"));
}

#[test]
fn error_deserialize_display() {
    let e = Error::Deserialize("cannot deserialize".into());
    assert!(e.to_string().contains("cannot deserialize"));
}

#[test]
fn error_deserialize_with_location_display() {
    let e = Error::DeserializeWithLocation {
        message: "wrong type".into(),
        location: Location::new(1, 1, 0),
    };
    let s = e.to_string();
    assert!(s.contains("wrong type"));
    assert!(s.contains("line 1, column 1"));
}

#[test]
fn error_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let e: Error = io_err.into();
    assert!(e.to_string().contains("file not found"));
}

#[test]
fn error_invalid() {
    let e = Error::Invalid("not valid yaml structure".into());
    assert!(e.to_string().contains("not valid yaml structure"));
}

#[test]
fn error_type_mismatch() {
    let e = Error::TypeMismatch {
        expected: "string",
        found: "integer".into(),
    };
    let s = e.to_string();
    assert!(s.contains("string"));
    assert!(s.contains("integer"));
}

#[test]
fn error_missing_field() {
    let e = Error::MissingField("name".into());
    assert!(e.to_string().contains("name"));
}

#[test]
fn error_unknown_field() {
    let e = Error::UnknownField("extra".into());
    assert!(e.to_string().contains("extra"));
}

#[test]
fn error_recursion_limit() {
    let e = Error::RecursionLimitExceeded { depth: 42 };
    assert!(e.to_string().contains("42"));
}

#[test]
fn error_repetition_limit() {
    let e = Error::RepetitionLimitExceeded;
    assert!(!e.to_string().is_empty());
}

#[test]
fn error_unknown_anchor() {
    let e = Error::UnknownAnchor("myanchor".into());
    assert!(e.to_string().contains("myanchor"));
}

#[test]
fn error_scalar_in_merge() {
    let e = Error::ScalarInMerge;
    assert!(e.to_string().contains("scalar"));
}

#[test]
fn error_tagged_in_merge() {
    let e = Error::TaggedInMerge;
    assert!(e.to_string().contains("tagged"));
}

#[test]
fn error_scalar_in_merge_element() {
    let e = Error::ScalarInMergeElement;
    assert!(e.to_string().contains("scalar"));
}

#[test]
fn error_sequence_in_merge_element() {
    let e = Error::SequenceInMergeElement;
    assert!(e.to_string().contains("sequence"));
}

#[test]
fn error_empty_tag() {
    let e = Error::EmptyTag;
    assert!(e.to_string().contains("tag"));
}

#[test]
fn error_failed_to_parse_number() {
    let e = Error::FailedToParseNumber("abc".into());
    assert!(e.to_string().contains("abc"));
}

#[test]
fn error_end_of_stream() {
    let e = Error::EndOfStream;
    assert!(!e.to_string().is_empty());
}

#[test]
fn error_more_than_one_document() {
    // The message wording was changed (#351) to match serde_yaml's
    // exact text, so downstream error messages line up; it no longer
    // contains "multiple".
    let e = Error::MoreThanOneDocument;
    assert!(e.to_string().contains("more than one document"));
}

#[test]
fn error_duplicate_key() {
    let e = Error::DuplicateKey("name".into());
    assert!(e.to_string().contains("name"));
}

#[test]
fn error_custom() {
    let e = Error::Custom("custom error".into());
    assert_eq!(e.to_string(), "custom error");
}

// ============================================================================
// Error::location()
// ============================================================================

#[test]
fn error_location_parse_with_location() {
    let e = Error::ParseWithLocation {
        message: "test".into(),
        location: Location::new(5, 3, 20),
    };
    let loc = e.location().unwrap();
    assert_eq!(loc.line(), 5);
    assert_eq!(loc.column(), 3);
}

#[test]
fn error_location_deserialize_with_location() {
    let e = Error::DeserializeWithLocation {
        message: "test".into(),
        location: Location::new(2, 1, 10),
    };
    assert!(e.location().is_some());
}

#[test]
fn error_location_none_for_plain_errors() {
    assert!(Error::Parse("test".into()).location().is_none());
    assert!(Error::Serialize("test".into()).location().is_none());
    assert!(Error::Deserialize("test".into()).location().is_none());
    assert!(Error::Custom("test".into()).location().is_none());
}

// ============================================================================
// Error::parse_at / deserialize_at
// ============================================================================

#[test]
fn error_parse_at() {
    let source = "hello\nworld\n";
    let e = Error::parse_at("bad token", source, 6);
    let loc = e.location().unwrap();
    assert_eq!(loc.line(), 2);
    assert_eq!(loc.column(), 1);
}

#[test]
fn error_deserialize_at() {
    let source = "hello\nworld\n";
    let e = Error::deserialize_at("wrong type", source, 8);
    let loc = e.location().unwrap();
    assert_eq!(loc.line(), 2);
    assert_eq!(loc.column(), 3);
}

// ============================================================================
// format_with_source
// ============================================================================

#[test]
fn format_with_source_no_location() {
    let e = Error::Parse("no loc".into());
    let formatted = e.format_with_source("some source");
    assert_eq!(formatted, e.to_string());
}

#[test]
fn format_with_source_with_location() {
    let e = Error::ParseWithLocation {
        message: "unexpected".into(),
        location: Location::new(1, 3, 2),
    };
    let source = "hello world";
    let formatted = e.format_with_source(source);
    assert!(formatted.contains("error:"));
    assert!(formatted.contains("-->"));
    assert!(formatted.contains("hello world"));
    assert!(formatted.contains('^'));
}

#[test]
fn format_with_source_line_out_of_range() {
    let e = Error::ParseWithLocation {
        message: "test".into(),
        location: Location::new(100, 1, 0),
    };
    let source = "one line";
    let formatted = e.format_with_source(source);
    // Should fallback to plain error string
    assert_eq!(formatted, e.to_string());
}

#[test]
fn format_with_source_column_zero() {
    let e = Error::ParseWithLocation {
        message: "test".into(),
        location: Location::new(1, 0, 0),
    };
    let source = "hello";
    let formatted = e.format_with_source(source);
    // Should handle column 0 via saturating_sub(1) = 0
    assert!(formatted.contains('^'));
}

// ============================================================================
// Shared errors
// ============================================================================

#[test]
fn shared_error_location_delegates() {
    let e = Error::ParseWithLocation {
        message: "inner".into(),
        location: Location::new(5, 3, 20),
    };
    let shared = Error::from_shared(e.into_shared());
    let loc = shared.location().unwrap();
    assert_eq!(loc.line(), 5);
}

#[test]
fn shared_error_display() {
    let e = Error::Parse("test".into());
    let shared = Error::Shared(Arc::new(e));
    assert_eq!(shared.to_string(), "YAML parse error: test");
}

#[test]
fn into_shared_already_shared_no_double_wrap() {
    let e = Error::Parse("test".into());
    let arc1 = e.into_shared();
    let shared = Error::Shared(Arc::clone(&arc1));
    let arc2 = shared.into_shared();
    assert!(Arc::ptr_eq(&arc1, &arc2));
}

#[test]
fn is_shared_and_as_inner() {
    let e = Error::Parse("test".into());
    assert!(!e.is_shared());
    assert!(e.as_inner().is_none());

    let shared = Error::from_shared(e.into_shared());
    assert!(shared.is_shared());
    let inner = shared.as_inner().unwrap();
    assert!(inner.to_string().contains("test"));
}

// ============================================================================
// serde Error impls
// ============================================================================

#[test]
fn serde_de_error_custom() {
    use serde_core::de::Error as _;
    let e = Error::custom("custom de error");
    assert_eq!(e.to_string(), "custom de error");
}

#[test]
fn serde_ser_error_custom() {
    use serde_core::ser::Error as _;
    let e = Error::custom("custom ser error");
    assert_eq!(e.to_string(), "custom ser error");
}
