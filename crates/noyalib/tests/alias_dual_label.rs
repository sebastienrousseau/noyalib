// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 1a — dual-label miette diagnostics for alias errors.
//!
//! Covers the new `Error::UnknownAnchorAt` variant along both the loader
//! (AST) and streaming paths, including "did you mean …?" typo suggestions
//! derived from the closest known anchor name.

use noyalib::{Error, Value, from_str};
use serde::Deserialize;

// ── Loader path (AST deserialisation to `Value`) ─────────────────────────

#[test]
fn loader_unknown_anchor_with_similar_name_carries_suggestion() {
    let yaml = "db: &logger prod\nserver: *logg\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    match err {
        Error::UnknownAnchorAt {
            name,
            location,
            suggestion,
        } => {
            assert_eq!(name, "logg");
            assert!(location.index() > 0);
            let (sugg, def_loc) = suggestion.expect("typo should suggest '&logger'");
            assert_eq!(sugg, "logger");
            assert!(
                def_loc.index() < location.index(),
                "anchor def precedes alias"
            );
        }
        other => panic!("expected UnknownAnchorAt, got {other:?}"),
    }
}

#[test]
fn loader_unknown_anchor_without_similar_name_has_no_suggestion() {
    // Anchor name "xyz" is distance 3 from "abc" → beyond threshold.
    let yaml = "a: &abc value\nb: *xyz\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    match err {
        Error::UnknownAnchorAt {
            name, suggestion, ..
        } => {
            assert_eq!(name, "xyz");
            assert!(
                suggestion.is_none(),
                "no close match expected, got {suggestion:?}"
            );
        }
        other => panic!("expected UnknownAnchorAt, got {other:?}"),
    }
}

#[test]
fn loader_unknown_anchor_empty_registry_no_suggestion() {
    // No anchors defined at all.
    let yaml = "foo: *bar\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    match err {
        Error::UnknownAnchorAt {
            name, suggestion, ..
        } => {
            assert_eq!(name, "bar");
            assert!(suggestion.is_none());
        }
        other => panic!("expected UnknownAnchorAt, got {other:?}"),
    }
}

#[test]
fn loader_unknown_anchor_picks_closest_among_many() {
    // Anchors: logger (dist 2), debug (dist 5), info (dist 4) vs "logg" → logger wins.
    let yaml = "a: &debug ok\nb: &info ok\nc: &logger ok\nd: &trace ok\nresult: *logg\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    match err {
        Error::UnknownAnchorAt { suggestion, .. } => {
            let (sugg, _) = suggestion.expect("a close match exists");
            assert_eq!(sugg, "logger");
        }
        other => panic!("expected UnknownAnchorAt, got {other:?}"),
    }
}

// ── Streaming path (typed deserialisation skipping the AST) ──────────────

#[test]
fn streaming_unknown_anchor_with_similar_name_carries_suggestion() {
    #[derive(Debug, Deserialize)]
    struct Doc {
        #[allow(dead_code)]
        db: String,
        #[allow(dead_code)]
        server: String,
    }
    let yaml = "db: &logger prod\nserver: *logg\n";
    let err = from_str::<Doc>(yaml).unwrap_err();
    match err {
        Error::UnknownAnchorAt {
            name,
            location,
            suggestion,
        } => {
            assert_eq!(name, "logg");
            assert!(location.index() > 0, "alias location must be populated");
            let (sugg, def_loc) = suggestion.expect("typo should suggest '&logger'");
            assert_eq!(sugg, "logger");
            assert!(def_loc.index() < location.index());
        }
        other => panic!("expected UnknownAnchorAt, got {other:?}"),
    }
}

#[test]
fn streaming_unknown_anchor_without_similar_name() {
    #[derive(Debug, Deserialize)]
    struct Doc {
        #[allow(dead_code)]
        a: String,
        #[allow(dead_code)]
        b: String,
    }
    let yaml = "a: &abc value\nb: *xyz\n";
    let err = from_str::<Doc>(yaml).unwrap_err();
    match err {
        Error::UnknownAnchorAt { suggestion, .. } => {
            assert!(suggestion.is_none());
        }
        other => panic!("expected UnknownAnchorAt, got {other:?}"),
    }
}

// ── Legacy UnknownAnchor variant still exists (backward compatibility) ──

#[test]
fn legacy_unknown_anchor_variant_still_constructible() {
    // Tests that downstream code matching on the old variant still compiles.
    let _ = Error::UnknownAnchor("foo".to_string());
}

// ── Miette rendering (feature-gated) ─────────────────────────────────────

#[cfg(feature = "miette")]
mod miette_render {
    use super::*;
    use miette::{Diagnostic, NamedSource, Report};

    #[test]
    fn dual_label_diagnostic_has_two_labels_when_suggestion_present() {
        let yaml = "db: &logger prod\nserver: *logg\n";
        let err = from_str::<Value>(yaml).unwrap_err();
        let labels: Vec<_> = err.labels().unwrap().collect();
        assert_eq!(labels.len(), 2, "dual-label path: {labels:?}");
        // Labels are ordered: alias site first, anchor-definition site second.
        assert!(labels[0].label().unwrap().contains("unknown anchor 'logg'"));
        assert!(labels[1].label().unwrap().contains("did you mean"));
    }

    #[test]
    fn single_label_diagnostic_when_no_suggestion() {
        let yaml = "foo: *bar\n";
        let err = from_str::<Value>(yaml).unwrap_err();
        let labels: Vec<_> = err.labels().unwrap().collect();
        assert_eq!(labels.len(), 1);
        assert!(labels[0].label().unwrap().contains("unknown anchor 'bar'"));
    }

    #[test]
    fn diagnostic_code_is_unknown_anchor() {
        let yaml = "foo: *bar\n";
        let err = from_str::<Value>(yaml).unwrap_err();
        assert_eq!(err.code().unwrap().to_string(), "noyalib::unknown_anchor");
    }

    #[test]
    fn diagnostic_help_surfaces_suggestion_name() {
        let yaml = "db: &logger prod\nserver: *logg\n";
        let err = from_str::<Value>(yaml).unwrap_err();
        let help = err.help().unwrap().to_string();
        assert!(help.contains("&logger"), "help was: {help}");
    }

    #[test]
    fn diagnostic_help_falls_back_when_no_suggestion() {
        let yaml = "foo: *bar\n";
        let err = from_str::<Value>(yaml).unwrap_err();
        assert!(err.help().is_none());
    }

    #[test]
    fn rendered_report_includes_both_label_texts() {
        let yaml = "db: &logger prod\nserver: *logg\n";
        let err = from_str::<Value>(yaml).unwrap_err();
        let report =
            Report::new(err).with_source_code(NamedSource::new("doc.yaml", yaml.to_string()));
        let rendered = format!("{report:?}");
        assert!(
            rendered.contains("unknown anchor 'logg'"),
            "missing alias label: {rendered}"
        );
        assert!(
            rendered.contains("did you mean '&logger'?"),
            "missing suggestion label: {rendered}"
        );
    }
}
