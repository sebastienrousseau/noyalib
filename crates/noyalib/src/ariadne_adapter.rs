// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! [`ariadne`] adapter for [`crate::Error`].
//!
//! Renders a noyalib error as an [`ariadne::Report`] with the
//! offending byte range labelled, the line/column annotated, and
//! the surrounding source available for terminal-friendly display.
//! Pairs with the existing [`miette::Diagnostic`] impl for users
//! who prefer ariadne's rendering.
//!
//! # Examples
//!
//! ```
//! use noyalib::{from_str, Value};
//! use noyalib::ariadne_adapter::error_to_ariadne_report;
//! use ariadne::Source;
//!
//! let source = "a: [unclosed";
//! let err = from_str::<Value>(source).unwrap_err();
//! let report = error_to_ariadne_report(&err, "input.yaml", source);
//! let mut buf: Vec<u8> = Vec::new();
//! report.write(("input.yaml", Source::from(source)), &mut buf).unwrap();
//! assert!(!buf.is_empty());
//! ```

use crate::diagnostic::{DiagnosticSeverity, SourceSpan};
use crate::error::Error;
use ariadne::{Color, Label, Report, ReportKind};

/// Convert a noyalib [`Error`] into an [`ariadne::Report`].
///
/// `filename` is the source identifier ariadne prints in the
/// report header. `source` is the YAML input — its byte length is
/// used to clamp the highlighted byte range so a stale or trimmed
/// `source` never panics inside ariadne.
///
/// When the structured diagnostic has no source labels, the resulting report
/// is a header-only message without a source label, matching the existing
/// [`Error::format_with_source`] fallback.
///
/// # Examples
///
/// ```
/// use noyalib::{from_str, Value};
/// use noyalib::ariadne_adapter::error_to_ariadne_report;
/// use ariadne::Source;
///
/// let source = "a: [unclosed";
/// let err = from_str::<Value>(source).unwrap_err();
/// let report = error_to_ariadne_report(&err, "input.yaml", source);
/// let mut out: Vec<u8> = Vec::new();
/// report.write(("input.yaml", Source::from(source)), &mut out).unwrap();
/// assert!(!out.is_empty());
/// ```
#[must_use]
pub fn error_to_ariadne_report<'a>(
    err: &Error,
    filename: &'a str,
    source: &str,
) -> Report<'a, (&'a str, core::ops::Range<usize>)> {
    let diagnostic = err.diagnostic();
    let kind = match diagnostic.severity() {
        DiagnosticSeverity::Error => ReportKind::Error,
        DiagnosticSeverity::Warning => ReportKind::Warning,
        DiagnosticSeverity::Information | DiagnosticSeverity::Hint => ReportKind::Advice,
    };
    let mut builder = Report::build(kind, (filename, 0..source.len()))
        .with_code(diagnostic.code())
        .with_message(diagnostic.message());

    for diagnostic_label in diagnostic.labels() {
        let span = label_span(diagnostic_label.span(), source);
        let color = if diagnostic_label.is_primary() {
            Color::Red
        } else {
            Color::Yellow
        };
        builder = builder.with_label(
            Label::new((filename, span))
                .with_message(diagnostic_label.message())
                .with_color(color),
        );
    }

    builder.finish()
}

/// Compute a sensible byte range for the error's primary label.
///
/// [`Location`] carries a single byte index; ariadne wants a
/// `Range<usize>`. We expand the index to cover the next character
/// (so the caret doesn't render as a zero-width range) and clamp
/// to the source bounds — never panics on a trimmed `source`.
fn label_span(span: SourceSpan, source: &str) -> core::ops::Range<usize> {
    let mut start = span.offset().min(source.len());
    while start > 0 && !source.is_char_boundary(start) {
        start -= 1;
    }
    if start >= source.len() {
        return start..start;
    }

    let mut end = span.end().min(source.len());
    if end <= start {
        end = source[start..]
            .chars()
            .next()
            .map_or(start + 1, |character| start + character.len_utf8());
    }
    while end < source.len() && !source.is_char_boundary(end) {
        end += 1;
    }
    start..end
}
