// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Targeted line/region coverage for `crates/noyalib/src/error.rs`.
//!
//! Each test exercises a specific uncovered branch:
//!
//! - `Error::source` arms for `Io` / `Shared` (L746-L747).
//! - `format_with_source` line-out-of-range fallback (L808).
//! - `format_with_source_radius` empty-source / out-of-range
//!   fallback paths (L864-L865).
//! - `format_with_source_truncated` &
//!   `format_with_source_radius_truncated` truncate paths
//!   (L939-L942, L958, L964-L966).
//! - `RenderOptions::new` / `crop_radius` / `color` builders
//!   (L1167-L1169, L1182-L1185, L1197-L1200).
//! - `CroppedRegion::extract` happy + empty + out-of-range paths
//!   (L1248-L1268).
//! - `colorize_render` arms for the `error:` / gutter / caret /
//!   plain branches (L1296-L1308).
//! - `serde::ser::Error` / `serde::de::Error` impls (L1305).
//! - `closest_name` + `edit_distance` low-level helpers (L1495+,
//!   L1508, L1511).
//! - miette integration when the `miette` feature is on
//!   (`noyalib::Diagnostic` impl — gated).

use std::sync::Arc;

use noyalib::{
    BudgetBreach, CroppedRegion, Error, ErrorKind, Location, RenderOptions, Result, Value, from_str,
};

/// One instance of every `Error` variant, used to drive the `kind()`
/// classifier and the `miette::Diagnostic` arms (`code` / `help` /
/// `labels`) across their full match. Kept in one place so the two
/// method-coverage tests below stay in lock-step with the enum.
fn variant_gallery() -> Vec<Error> {
    vec![
        Error::Parse("p".into()),
        Error::ParseWithLocation {
            message: "p".into(),
            location: Location::from_index("a: [", 3),
        },
        Error::Serialize("s".into()),
        Error::Deserialize("d".into()),
        Error::DeserializeWithLocation {
            message: "d".into(),
            location: Location::from_index("ab", 1),
        },
        Error::Custom("c".into()),
        Error::RecursionLimitExceeded { depth: 4 },
        Error::DuplicateKey("k".into()),
        Error::KeyCollision("k".into()),
        Error::RepetitionLimitExceeded,
        Error::Budget(BudgetBreach::MaxNodes {
            limit: 1,
            observed: 2,
        }),
        Error::UnknownAnchor("a".into()),
        Error::UnknownAnchorAt {
            name: "a".into(),
            location: Location::default(),
            suggestion: Some(("ab".into(), Location::default())),
        },
        Error::MissingField("f".into()),
        Error::UnknownField("f".into()),
        Error::ScalarInMergeElement,
        Error::SequenceInMergeElement,
        Error::TaggedInMerge,
        Error::ScalarInMerge,
        Error::Invalid("v".into()),
        Error::TypeMismatch {
            expected: "int",
            found: "str".into(),
        },
        Error::Shared(Arc::new(Error::EndOfStream)),
        Error::EndOfStream,
        Error::MoreThanOneDocument,
        Error::EmptyTag,
        Error::FailedToParseNumber("nan".into()),
        Error::Message("m".into(), Some(7)),
        Error::Message("m".into(), None),
        Error::Io(std::io::Error::other("i")),
    ]
}

#[test]
fn kind_classifier_covers_every_variant() {
    // Execute every arm of `Error::kind()`. Precise mappings are pinned
    // in error_kind.rs; here we drive the whole match and spot-check a
    // handful that the parse-driven suite can't reach.
    for e in &variant_gallery() {
        let _ = e.kind();
    }
    assert_eq!(Error::EndOfStream.kind(), ErrorKind::EndOfStream);
    assert_eq!(Error::EmptyTag.kind(), ErrorKind::Syntax);
    assert_eq!(Error::ScalarInMerge.kind(), ErrorKind::Policy);
    assert_eq!(Error::TaggedInMerge.kind(), ErrorKind::Policy);
    assert_eq!(Error::MoreThanOneDocument.kind(), ErrorKind::Data);
    assert_eq!(Error::RepetitionLimitExceeded.kind(), ErrorKind::Budget);
    assert_eq!(
        Error::TypeMismatch {
            expected: "int",
            found: "s".into()
        }
        .kind(),
        ErrorKind::Data
    );
    // `Shared` forwards to the inner error's kind.
    assert_eq!(
        Error::Shared(Arc::new(Error::EmptyTag)).kind(),
        ErrorKind::Syntax
    );
}

#[cfg(feature = "miette")]
#[test]
fn miette_code_help_labels_cover_every_variant() {
    use miette::Diagnostic;
    for e in variant_gallery() {
        let _ = e.code().map(|c| c.to_string());
        let _ = e.help().map(|h| h.to_string());
        let _ = e.labels().map(|it| it.count());
    }
    // Spot-check representative code() / help() / labels() outputs so
    // this is not assertion-free.
    let rec = Error::RecursionLimitExceeded { depth: 3 };
    assert_eq!(rec.code().unwrap().to_string(), "noyalib::recursion_limit");
    let budget = Error::Budget(BudgetBreach::MaxNodes {
        limit: 1,
        observed: 2,
    });
    assert!(budget.help().is_some(), "budget errors carry a hint");
    // A located deserialise error emits exactly one miette label.
    let located = Error::DeserializeWithLocation {
        message: "expected int".into(),
        location: Location::from_index("ab", 1),
    };
    assert_eq!(located.labels().map(|it| it.count()), Some(1));
}

// ============================================================================
// Error::source — every arm
// ============================================================================

#[test]
fn error_source_io_some() {
    use std::error::Error as StdError;
    let ioe = std::io::Error::other("nope");
    let e = Error::Io(ioe);
    assert!(e.source().is_some());
}

#[test]
fn error_source_shared_some() {
    use std::error::Error as StdError;
    let inner = Error::EndOfStream;
    let e = Error::Shared(Arc::new(inner));
    assert!(e.source().is_some());
}

#[test]
fn error_source_other_none() {
    use std::error::Error as StdError;
    let e = Error::EndOfStream;
    assert!(e.source().is_none());
}

// ============================================================================
// From<std::io::Error> for Error
// ============================================================================

#[test]
fn error_from_io_error() {
    let ioe = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "x");
    let e: Error = ioe.into();
    assert!(matches!(e, Error::Io(_)));
}

// ============================================================================
// format_with_source — line out of range fallback (L808)
// ============================================================================

#[test]
fn format_with_source_line_out_of_range_falls_back() {
    let e = Error::ParseWithLocation {
        message: "boom".into(),
        location: Location::new(999, 1, 0),
    };
    let s = e.format_with_source("only one line");
    // The fallback is plain Display.
    assert!(s.contains("boom"));
}

#[test]
fn format_with_source_line_zero_uses_first_line() {
    // line == 0 → uses lines().next() (L808 branch).
    let e = Error::ParseWithLocation {
        message: "early".into(),
        location: Location::default(), // line = 0
    };
    let s = e.format_with_source("first line\nsecond line");
    assert!(s.contains("early"));
}

#[test]
fn format_with_source_with_valid_location() {
    let source = "line 1\nline 2 — bad\nline 3";
    let e = Error::ParseWithLocation {
        message: "bad token".into(),
        location: Location::new(2, 5, 12),
    };
    let s = e.format_with_source(source);
    assert!(s.contains("error"));
    assert!(s.contains("line 2"));
    assert!(s.contains("^"));
}

#[test]
fn format_with_source_no_location_falls_back_to_display() {
    let e = Error::EndOfStream;
    let s = e.format_with_source("anything");
    assert_eq!(s, format!("{e}"));
}

// ============================================================================
// format_with_source_radius — out-of-range, empty source, line zero
// ============================================================================

#[test]
fn format_with_source_radius_empty_source() {
    let e = Error::ParseWithLocation {
        message: "x".into(),
        location: Location::new(1, 1, 0),
    };
    let s = e.format_with_source_radius("", 2);
    // Empty source → plain Display fallback.
    assert!(s.contains("x"));
}

#[test]
fn format_with_source_radius_out_of_range_line() {
    let e = Error::ParseWithLocation {
        message: "x".into(),
        location: Location::new(50, 1, 0),
    };
    let s = e.format_with_source_radius("only line", 2);
    // Out-of-range → falls back to plain Display.
    assert!(s.contains("x"));
}

#[test]
fn format_with_source_radius_line_zero_uses_first() {
    let e = Error::ParseWithLocation {
        message: "boot".into(),
        location: Location::default(),
    };
    let s = e.format_with_source_radius("a\nb\nc", 1);
    assert!(s.contains("boot"));
}

#[test]
fn format_with_source_radius_no_location_falls_back() {
    let e = Error::EndOfStream;
    let s = e.format_with_source_radius("anything", 2);
    assert_eq!(s, format!("{e}"));
}

#[test]
fn format_with_source_radius_with_valid_location_renders_window() {
    let source = "l1\nl2\nl3 BAD\nl4\nl5";
    let e = Error::ParseWithLocation {
        message: "bad".into(),
        location: Location::new(3, 4, 7),
    };
    let s = e.format_with_source_radius(source, 1);
    assert!(s.contains("l2"));
    assert!(s.contains("l3"));
    assert!(s.contains("l4"));
    // Caret line.
    assert!(s.contains("^"));
}

// ============================================================================
// format_with_source_truncated / format_with_source_radius_truncated
// ============================================================================

#[test]
fn format_with_source_truncated_short_input() {
    let e = Error::EndOfStream;
    let full = e.format_with_source("anything");
    let bounded = e.format_with_source_truncated("anything", full.len() + 100);
    assert_eq!(bounded, full);
}

#[test]
fn format_with_source_truncated_exceeds_budget() {
    let source = "abcdefghijklmnopqrstuvwxyz";
    let e = Error::ParseWithLocation {
        message: "this is a long message that should be cropped".into(),
        location: Location::new(1, 5, 4),
    };
    let s = e.format_with_source_truncated(source, 30);
    assert!(s.len() <= 30);
    assert!(s.ends_with("..."));
}

#[test]
fn format_with_source_radius_truncated_works() {
    let source = "l1\nl2\nl3\nl4\nl5";
    let e = Error::ParseWithLocation {
        message: "x".into(),
        location: Location::new(3, 1, 6),
    };
    let s = e.format_with_source_radius_truncated(source, 1, 50);
    assert!(s.len() <= 50);
}

// ============================================================================
// RenderOptions builders (L1167-L1169, L1182-L1185, L1197-L1200)
// ============================================================================

#[test]
fn render_options_new_equals_default() {
    let a = RenderOptions::new();
    let b = RenderOptions::default();
    assert_eq!(a, b);
}

#[test]
fn render_options_crop_radius_setter() {
    let opts = RenderOptions::new().crop_radius(7);
    assert_eq!(opts.crop_radius, 7);
    assert!(!opts.color);
}

#[test]
fn render_options_color_setter() {
    let opts = RenderOptions::new().color(true);
    assert!(opts.color);
    assert_eq!(opts.crop_radius, 2);
}

#[test]
fn render_options_chained_builder() {
    let opts = RenderOptions::new().crop_radius(0).color(true);
    assert_eq!(opts.crop_radius, 0);
    assert!(opts.color);
}

// ============================================================================
// Error::render / render_with_options — full coverage
// ============================================================================

#[test]
fn render_default_engages_radius() {
    let source = "a:\n  b: 1\n   c: 2\n";
    let r: Result<Value> = from_str(source);
    if let Err(e) = r {
        let s = e.render(source);
        assert!(s.contains("error"));
    }
}

#[test]
fn render_with_radius_zero_uses_single_line() {
    let source = "a: [unclosed";
    let r: Result<Value> = from_str(source);
    if let Err(e) = r {
        let opts = RenderOptions::new().crop_radius(0);
        let s = e.render_with_options(source, &opts);
        assert!(s.contains("error"));
    }
}

#[test]
fn render_with_color_engages_colorize_render() {
    let source = "a: [unclosed";
    let r: Result<Value> = from_str(source);
    if let Err(e) = r {
        let opts = RenderOptions::new().color(true).crop_radius(2);
        let s = e.render_with_options(source, &opts);
        // ANSI escape sequences present.
        assert!(s.contains("\x1b["));
    }
}

#[test]
fn render_with_color_radius_zero() {
    // Single-line + colour exercises a different colorize_render
    // line set (only error: + caret, no gutter).
    let source = "a: [unclosed";
    let r: Result<Value> = from_str(source);
    if let Err(e) = r {
        let opts = RenderOptions::new().color(true).crop_radius(0);
        let _s = e.render_with_options(source, &opts);
    }
}

// ============================================================================
// CroppedRegion::extract — happy, empty, out-of-range
// ============================================================================

#[test]
fn cropped_region_extract_happy_path() {
    let src = "a\nb\nc\nd\ne";
    let r = CroppedRegion::extract(src, 3, 1);
    assert_eq!(r.lines, vec!["b", "c", "d"]);
    assert_eq!(r.focus_index, 1);
    assert_eq!(r.focus_line, 3);
    assert_eq!(r.low_line, 2);
}

#[test]
fn cropped_region_extract_empty_source() {
    let r = CroppedRegion::extract("", 1, 1);
    assert!(r.lines.is_empty());
    assert_eq!(r.focus_line, 0);
}

#[test]
fn cropped_region_extract_target_out_of_range_clamps() {
    let src = "a\nb\nc";
    let r = CroppedRegion::extract(src, 99, 1);
    // Clamps to last line.
    assert_eq!(r.focus_line, 3);
}

#[test]
fn cropped_region_extract_zero_radius() {
    let src = "a\nb\nc\nd\ne";
    let r = CroppedRegion::extract(src, 3, 0);
    assert_eq!(r.lines, vec!["c"]);
    assert_eq!(r.focus_index, 0);
}

#[test]
fn cropped_region_extract_radius_overflows_top() {
    let src = "a\nb\nc";
    let r = CroppedRegion::extract(src, 1, 5);
    // saturating_sub clamps lo to 0.
    assert_eq!(r.low_line, 1);
}

// ============================================================================
// Error::into_shared / from_shared / is_shared / as_inner
// ============================================================================

#[test]
fn into_shared_already_shared_reuses_arc() {
    let inner = Arc::new(Error::EndOfStream);
    let e = Error::Shared(Arc::clone(&inner));
    let arc = e.into_shared();
    assert!(Arc::ptr_eq(&arc, &inner));
}

#[test]
fn into_shared_wraps_other_in_arc() {
    let arc = Error::EndOfStream.into_shared();
    assert!(matches!(&*arc, Error::EndOfStream));
}

#[test]
fn is_shared_and_as_inner() {
    let e = Error::Shared(Arc::new(Error::EndOfStream));
    assert!(e.is_shared());
    assert!(e.as_inner().is_some());
    let plain = Error::EndOfStream;
    assert!(!plain.is_shared());
    assert!(plain.as_inner().is_none());
}

// ============================================================================
// parse_at / deserialize_at / from_shared
// ============================================================================

#[test]
fn parse_at_carries_location() {
    let e = Error::parse_at("bad", "a: x", 3);
    assert!(matches!(e, Error::ParseWithLocation { .. }));
    assert_eq!(e.location().unwrap().index(), 3);
}

#[test]
fn deserialize_at_carries_location() {
    let e = Error::deserialize_at("bad", "a: x", 1);
    assert!(matches!(e, Error::DeserializeWithLocation { .. }));
}

#[test]
fn from_shared_constructor() {
    let e = Error::from_shared(Arc::new(Error::EndOfStream));
    assert!(e.is_shared());
}

// ============================================================================
// Display — every variant
// ============================================================================

#[test]
fn display_every_variant() {
    let cases: Vec<Error> = vec![
        Error::Parse("p".into()),
        Error::ParseWithLocation {
            message: "p".into(),
            location: Location::default(),
        },
        Error::Serialize("s".into()),
        Error::Deserialize("d".into()),
        Error::DeserializeWithLocation {
            message: "d".into(),
            location: Location::default(),
        },
        Error::Custom("c".into()),
        Error::RecursionLimitExceeded { depth: 4 },
        Error::DuplicateKey("k".into()),
        Error::RepetitionLimitExceeded,
        Error::Budget(BudgetBreach::MaxNodes {
            limit: 1,
            observed: 2,
        }),
        Error::UnknownAnchor("a".into()),
        Error::UnknownAnchorAt {
            name: "a".into(),
            location: Location::default(),
            suggestion: None,
        },
        Error::MissingField("f".into()),
        Error::UnknownField("f".into()),
        Error::ScalarInMergeElement,
        Error::SequenceInMergeElement,
        Error::TaggedInMerge,
        Error::Invalid("v".into()),
        Error::TypeMismatch {
            expected: "int",
            found: "str".into(),
        },
        Error::Shared(Arc::new(Error::EndOfStream)),
        Error::EndOfStream,
        Error::MoreThanOneDocument,
        Error::ScalarInMerge,
        Error::EmptyTag,
        Error::FailedToParseNumber("nan".into()),
        Error::Message("m".into(), Some(7)),
        Error::Message("m".into(), None),
        Error::Io(std::io::Error::other("i")),
    ];
    for e in cases {
        let s = format!("{e}");
        assert!(!s.is_empty());
    }
}

// ============================================================================
// BudgetBreach Display — every variant (L737, L746-747 etc.)
// ============================================================================

#[test]
fn budget_breach_display_all_variants() {
    let cases = vec![
        BudgetBreach::MaxEvents {
            limit: 1,
            observed: 2,
        },
        BudgetBreach::MaxNodes {
            limit: 1,
            observed: 2,
        },
        BudgetBreach::MaxTotalScalarBytes {
            limit: 1,
            observed: 2,
        },
        BudgetBreach::MaxDocuments {
            limit: 1,
            observed: 2,
        },
        BudgetBreach::MaxMergeKeys {
            limit: 1,
            observed: 2,
        },
        BudgetBreach::AliasAnchorRatio {
            ratio: 1.5,
            anchors: 2,
            aliases: 4,
        },
    ];
    for c in cases {
        let s = format!("{c}");
        assert!(!s.is_empty());
    }
}

// ============================================================================
// serde::ser::Error / serde::de::Error trait impls (L1305, L1322-L1326)
// ============================================================================

#[test]
fn serde_ser_error_custom() {
    let e: Error = <Error as serde::ser::Error>::custom("ser-msg");
    assert!(matches!(e, Error::Custom(_)));
}

#[test]
fn serde_de_error_custom() {
    let e: Error = <Error as serde::de::Error>::custom("de-msg");
    assert!(matches!(e, Error::Custom(_)));
}

#[test]
fn serde_de_error_missing_field() {
    let e: Error = <Error as serde::de::Error>::missing_field("foo");
    assert!(matches!(e, Error::MissingField(_)));
}

#[test]
fn serde_de_error_unknown_field() {
    let e: Error = <Error as serde::de::Error>::unknown_field("bad", &["good"]);
    assert!(matches!(e, Error::UnknownField(_)));
}

// ============================================================================
// Error::location() — every arm (L770-L777)
// ============================================================================

#[test]
fn location_for_parse_with_location() {
    let e = Error::ParseWithLocation {
        message: "x".into(),
        location: Location::new(2, 3, 5),
    };
    assert!(e.location().is_some());
}

#[test]
fn location_for_deserialize_with_location() {
    let e = Error::DeserializeWithLocation {
        message: "x".into(),
        location: Location::new(1, 1, 0),
    };
    assert!(e.location().is_some());
}

#[test]
fn location_for_unknown_anchor_at() {
    let e = Error::UnknownAnchorAt {
        name: "x".into(),
        location: Location::new(1, 1, 0),
        suggestion: None,
    };
    assert!(e.location().is_some());
}

#[test]
fn location_for_shared_delegates() {
    let inner = Error::ParseWithLocation {
        message: "x".into(),
        location: Location::new(1, 1, 0),
    };
    let e = Error::Shared(Arc::new(inner));
    assert!(e.location().is_some());
}

#[test]
fn location_for_other_returns_none() {
    let e = Error::EndOfStream;
    assert!(e.location().is_none());
}
