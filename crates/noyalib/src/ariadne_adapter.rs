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

use crate::error::{Error, Location};
use ariadne::{Color, Label, Report, ReportKind};

/// Convert a noyalib [`Error`] into an [`ariadne::Report`].
///
/// `filename` is the source identifier ariadne prints in the
/// report header. `source` is the YAML input — its byte length is
/// used to clamp the highlighted byte range so a stale or trimmed
/// `source` never panics inside ariadne.
///
/// When the error has no location attached (`Error::location()`
/// returns `None`), the resulting report is a header-only message
/// without a source label — matching the existing
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
    let title = format!("{err}");
    let mut builder =
        Report::build(ReportKind::Error, (filename, 0..source.len())).with_message(title.clone());

    if let Some(loc) = err.location() {
        let span = label_span(loc, source);
        builder = builder.with_label(
            Label::new((filename, span))
                .with_message(title)
                .with_color(Color::Red),
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
fn label_span(loc: Location, source: &str) -> core::ops::Range<usize> {
    let start = loc.index().min(source.len());
    if start >= source.len() {
        return start..start;
    }
    let end = source[start..]
        .chars()
        .next()
        .map_or(start + 1, |c| start + c.len_utf8())
        .min(source.len());
    start..end
}
