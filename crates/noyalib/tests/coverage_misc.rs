// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Targeted coverage for smaller modules with lower line counts but
//! reachable public surface: `diagnostic`, `borrowed`, `validated` edge
//! cases, and miscellaneous error paths.

#![allow(unused_imports)]

use noyalib::{Error, Location, Value, from_str, to_string};
use serde::Deserialize;

// ── diagnostic.rs (miette bridge) ────────────────────────────────────────

#[cfg(feature = "miette")]
mod diagnostic {
    use super::*;
    use miette::Diagnostic;
    use noyalib::Spanned;
    use noyalib::diagnostic::{spanned_error, spanned_error_with_context};

    #[derive(Deserialize)]
    struct TwoFields {
        a: Spanned<String>,
        b: Spanned<i32>,
    }

    #[test]
    fn spanned_error_has_code_and_single_label() {
        let yaml = "a: hello\nb: 42\n";
        let cfg: TwoFields = from_str(yaml).unwrap();
        let report = spanned_error(yaml, &cfg.a, "bad a");
        let diag: &dyn Diagnostic = report.as_ref();
        assert!(diag.code().is_some());
        assert!(diag.source_code().is_some());
        let labels: Vec<_> = diag.labels().unwrap().collect();
        assert_eq!(labels.len(), 1);
        assert!(labels[0].label().unwrap().contains("bad a"));
    }

    #[test]
    fn spanned_error_with_context_has_two_labels() {
        let yaml = "a: hello\nb: 42\n";
        let cfg: TwoFields = from_str(yaml).unwrap();
        let report = spanned_error_with_context(yaml, &cfg.a, "primary", &cfg.b, "context");
        let diag: &dyn Diagnostic = report.as_ref();
        let labels: Vec<_> = diag.labels().unwrap().collect();
        assert_eq!(labels.len(), 2);
        assert!(labels[0].label().unwrap().contains("primary"));
        assert!(labels[1].label().unwrap().contains("context"));
    }

    #[test]
    fn spanned_error_format_contains_message() {
        let yaml = "a: hi\nb: 1\n";
        let cfg: TwoFields = from_str(yaml).unwrap();
        let report = spanned_error(yaml, &cfg.a, "my validation message");
        let rendered = format!("{report}");
        assert!(rendered.contains("my validation message"));
    }

    #[test]
    fn spanned_error_with_zero_length_span() {
        // Exercise the saturating-length path.
        let spanned: Spanned<i32> = Spanned::new(42);
        let report = spanned_error("source", &spanned, "zero-span");
        assert!(format!("{report}").contains("zero-span"));
    }
}

// ── span_context.rs (SpanTree construction / queries) ────────────────────

mod span_context {
    use super::*;
    use noyalib::Spanned;

    #[test]
    fn spanned_field_reports_location_of_value() {
        let yaml = "port: 8080\n";
        #[derive(Deserialize)]
        struct Cfg {
            port: Spanned<u16>,
        }
        let cfg: Cfg = from_str(yaml).unwrap();
        // Start/end indices should bracket the "8080" substring.
        let start = cfg.port.start.index();
        let end = cfg.port.end.index();
        assert!(start > 0);
        assert!(end > start);
        // End may include trailing whitespace — slice must START with "8080".
        assert!(yaml[start..end.min(yaml.len())].starts_with("8080"));
    }

    #[test]
    fn spanned_in_nested_mapping() {
        let yaml = "outer:\n  inner: hello\n";
        #[derive(Deserialize)]
        struct Inner {
            inner: Spanned<String>,
        }
        #[derive(Deserialize)]
        struct Outer {
            outer: Inner,
        }
        let d: Outer = from_str(yaml).unwrap();
        assert_eq!(d.outer.inner.value, "hello");
        let start = d.outer.inner.start.index();
        let end = d.outer.inner.end.index();
        assert!(yaml[start..end.min(yaml.len())].starts_with("hello"));
    }

    #[test]
    fn spanned_in_sequence() {
        let yaml = "items:\n  - first\n  - second\n";
        #[derive(Deserialize)]
        struct Doc {
            items: Vec<Spanned<String>>,
        }
        let d: Doc = from_str(yaml).unwrap();
        assert_eq!(d.items.len(), 2);
        assert_eq!(d.items[0].value, "first");
        assert_eq!(d.items[1].value, "second");
        assert!(d.items[0].start.index() < d.items[1].start.index());
    }

    #[test]
    fn spanned_location_zero_for_from_value() {
        // Going through from_value should give zero-location spans.
        let v = Value::from(42_i32);
        let s: Spanned<i32> = noyalib::from_value(&v).unwrap();
        assert_eq!(s.value, 42);
        // When constructed outside a source, indices default to 0.
        assert_eq!(s.start.index(), 0);
        assert_eq!(s.end.index(), 0);
    }
}

// ── Error utility methods ────────────────────────────────────────────────

#[test]
fn error_location_returns_some_for_parsewithlocation() {
    let e = Error::ParseWithLocation {
        message: "test".into(),
        location: Location::new(2, 5, 10),
    };
    let loc = e.location().unwrap();
    assert_eq!(loc.line(), 2);
    assert_eq!(loc.column(), 5);
    assert_eq!(loc.index(), 10);
}

#[test]
fn error_location_returns_none_for_plain_parse() {
    let e = Error::Parse("no location".into());
    assert!(e.location().is_none());
}

#[test]
fn error_format_with_source_without_location_falls_back_to_display() {
    let e = Error::Parse("no location".into());
    let formatted = e.format_with_source("source");
    assert_eq!(formatted, format!("{e}"));
}

#[test]
fn error_into_shared_then_back_is_idempotent() {
    let e = Error::Parse("x".into());
    let arc1 = e.into_shared();
    let shared = Error::from_shared(arc1.clone());
    let arc2 = shared.into_shared();
    assert!(std::sync::Arc::ptr_eq(&arc1, &arc2));
}

#[test]
fn error_is_shared_distinguishes_wrapped_from_plain() {
    let e = Error::Parse("x".into());
    assert!(!e.is_shared());
    let shared = Error::from_shared(e.into_shared());
    assert!(shared.is_shared());
}

#[test]
fn error_as_inner_returns_inner_for_shared() {
    let e = Error::Parse("inner message".into());
    let shared = Error::from_shared(e.into_shared());
    let inner = shared.as_inner().unwrap();
    assert!(inner.to_string().contains("inner message"));
}

#[test]
fn error_as_inner_returns_none_for_non_shared() {
    let e = Error::Parse("x".into());
    assert!(e.as_inner().is_none());
}

#[test]
fn location_display_includes_line_column() {
    let loc = Location::new(7, 3, 20);
    let s = format!("{loc}");
    assert!(s.contains("7"));
    assert!(s.contains("3"));
}

#[test]
fn location_from_index_tracks_newlines() {
    let input = "line1\nline2\nline3";
    let loc = Location::from_index(input, 8);
    assert_eq!(loc.line(), 2);
    // byte 8 = 'i' of "line2"; column relative to line start.
    assert!(loc.column() >= 2);
}

// ── More borrowed.rs coverage paths ──────────────────────────────────────

mod borrowed_extra {
    use super::*;
    use noyalib::borrowed::{BorrowedValue, from_str_borrowed};

    #[test]
    fn float_borrowed_to_str_is_none() {
        let v: BorrowedValue<'_> = from_str_borrowed("1.5").unwrap();
        assert_eq!(v.as_str(), None);
    }

    #[test]
    fn boolean_to_i64_is_none() {
        let v: BorrowedValue<'_> = from_str_borrowed("true").unwrap();
        assert_eq!(v.as_i64(), None);
    }

    #[test]
    fn integer_to_bool_is_none() {
        let v: BorrowedValue<'_> = from_str_borrowed("42").unwrap();
        assert_eq!(v.as_bool(), None);
    }

    #[test]
    fn get_path_empty_string_returns_root() {
        let v: BorrowedValue<'_> = from_str_borrowed("a: 1\n").unwrap();
        // Empty path returns the root value.
        let r = v.get_path("");
        assert!(r.is_some());
    }

    #[test]
    fn query_empty_string_returns_root() {
        let v: BorrowedValue<'_> = from_str_borrowed("a: 1\n").unwrap();
        let results = v.query("");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn query_no_match_returns_empty() {
        let v: BorrowedValue<'_> = from_str_borrowed("a: 1\n").unwrap();
        let results = v.query("nonexistent.path");
        assert!(results.is_empty());
    }

    #[test]
    fn into_owned_mapping() {
        let v: BorrowedValue<'_> = from_str_borrowed("a: 1\nb: 2\n").unwrap();
        let owned = v.into_owned();
        assert!(owned.as_mapping().is_some());
        assert_eq!(owned.get("a").and_then(|v| v.as_i64()), Some(1));
    }

    #[test]
    fn into_owned_sequence() {
        let v: BorrowedValue<'_> = from_str_borrowed("- 1\n- 2\n- 3\n").unwrap();
        let owned = v.into_owned();
        assert!(owned.as_sequence().is_some());
        assert_eq!(owned.as_sequence().unwrap().len(), 3);
    }
}

// ── validated.rs construction paths not hit by garde/validator tests ────

#[cfg(feature = "garde")]
mod validated_extra {
    use super::*;
    use garde::Validate;
    use noyalib::Validated;

    #[derive(Debug, Deserialize, Validate)]
    struct Plain {
        #[garde(skip)]
        x: i32,
    }

    #[test]
    fn deref_mut_allows_mutation() {
        let mut v: Validated<Plain> = from_str("x: 1\n").unwrap();
        v.x = 99;
        assert_eq!(v.x, 99);
    }

    #[test]
    fn debug_format_works() {
        let v: Validated<Plain> = from_str("x: 5\n").unwrap();
        let s = format!("{:?}", v);
        assert!(s.contains("Validated"));
    }
}
