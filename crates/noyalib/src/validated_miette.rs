// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! [`Spanned<T>`] + `garde` / `validator` → [`miette::Report`]
//! bridge.
//!
//! Walks a validation error tree produced by [`garde::Report`] or
//! [`validator::ValidationErrors`] and emits a single
//! [`miette::Report`] whose source label points at the
//! corresponding [`Spanned<T>`] range — enabling beautiful,
//! line-and-column-precise output for declarative validation
//! failures.
//!
//! Behind feature combinations:
//! * [`garde_errors_to_miette`] requires `miette` + `garde`
//! * [`validator_errors_to_miette`] requires `miette` + `validator`
//!
//! These functions are deliberately free-standing rather than
//! impls on [`crate::Spanned`] to keep the dependency surface
//! visible at the call site and avoid leaking miette into every
//! `Spanned<T>` user.

#[allow(unused_imports)]
use crate::prelude::*;
#[allow(unused_imports)]
use crate::spanned::Spanned;

/// Range covering the entire `Spanned<T>` payload, clamped to the
/// supplied `source` length so a stale snippet never panics.
///
/// Only defined when `garde` or `validator` is on — those are the
/// features that bring callers (`garde_errors_to_miette` /
/// `validator_errors_to_miette`). Otherwise the body would be
/// dead code on a `miette`-only build (e.g. enabled transitively
/// by `noyalib-lsp`'s `validate-schema`).
#[cfg(any(feature = "garde", feature = "validator"))]
fn spanned_byte_range<T>(spanned: &Spanned<T>, source: &str) -> core::ops::Range<usize> {
    let len = source.len();
    let start = spanned.start.index().min(len);
    let end = spanned.end.index().max(start).min(len);
    // Ariadne / miette both expect a non-empty range to draw a label.
    if end == start {
        return start..(start + 1).min(len.max(start + 1));
    }
    start..end
}

/// What the bridge produces — boxed as `miette::Report` so callers
/// can chain it through `Result<_, miette::Report>` workflows.
///
/// Implements [`miette::Diagnostic`] so it renders with full
/// source-context, labelled span, and helpful one-line summary
/// when fed through `miette::ReportHandler`. Display + Error
/// impls are hand-rolled to keep `thiserror` out of the
/// dependency closure (matches the policy in `error.rs`). Only
/// compiled when at least one validator framework is enabled —
/// otherwise the type would be dead code on a `miette`-only
/// build.
#[cfg(any(feature = "garde", feature = "validator"))]
#[derive(Debug)]
struct ValidationDiagnostic {
    summary: String,
    src: miette::NamedSource<String>,
    span: miette::SourceSpan,
}

#[cfg(any(feature = "garde", feature = "validator"))]
impl ValidationDiagnostic {
    fn new<T>(spanned: &Spanned<T>, source: String, summary: String, name: String) -> Self {
        let range = spanned_byte_range(spanned, &source);
        let span: miette::SourceSpan = (range.start, range.end - range.start).into();
        Self {
            summary,
            src: miette::NamedSource::new(name, source),
            span,
        }
    }
}

#[cfg(any(feature = "garde", feature = "validator"))]
impl core::fmt::Display for ValidationDiagnostic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.summary)
    }
}

#[cfg(any(feature = "garde", feature = "validator"))]
impl std::error::Error for ValidationDiagnostic {}

#[cfg(any(feature = "garde", feature = "validator"))]
impl miette::Diagnostic for ValidationDiagnostic {
    fn code<'a>(&'a self) -> Option<Box<dyn core::fmt::Display + 'a>> {
        Some(Box::new("noyalib::validated"))
    }
    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.src)
    }
    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        Some(Box::new(core::iter::once(
            miette::LabeledSpan::new_with_span(
                Some("validation failed here".to_string()),
                self.span,
            ),
        )))
    }
}

/// Render a [`garde::Report`] tree against a [`Spanned<T>`] payload
/// as a [`miette::Report`].
///
/// The error message is the compact single-line summary garde's
/// own walker produces (path → message pairs joined with `;`); the
/// source label points at the supplied `Spanned<T>`'s byte range.
/// `name` is the source identifier shown in the report header
/// (typically a filename).
///
/// # Examples
///
/// ```ignore
/// use noyalib::Spanned;
/// use noyalib::validated_miette::garde_errors_to_miette;
/// use garde::Validate;
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, Validate)]
/// struct Cfg {
///     #[garde(range(min = 1024))]
///     port: u16,
/// }
///
/// let yaml = "port: 22\n";
/// let wrapped: Spanned<Cfg> = noyalib::from_str(yaml).unwrap();
/// if let Err(errs) = wrapped.value.validate() {
///     let report = garde_errors_to_miette(&wrapped, &errs, yaml, "config.yaml");
///     // `report.to_string()` carries the rendered diagnostic.
///     drop(report);
/// }
/// ```
#[cfg(all(feature = "miette", feature = "garde"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "miette", feature = "garde"))))]
#[must_use]
pub fn garde_errors_to_miette<T>(
    spanned: &Spanned<T>,
    errors: &garde::Report,
    source: &str,
    name: impl Into<String>,
) -> miette::Report {
    let summary = format_garde(errors);
    let diag = ValidationDiagnostic::new(spanned, source.to_string(), summary, name.into());
    miette::Report::new(diag)
}

/// Render a [`validator::ValidationErrors`] against a
/// [`Spanned<T>`] payload as a [`miette::Report`].
///
/// Mirrors [`garde_errors_to_miette`] for the `validator` crate.
///
/// # Examples
///
/// ```ignore
/// use noyalib::Spanned;
/// use noyalib::validated_miette::validator_errors_to_miette;
/// use serde::Deserialize;
/// use validator::Validate;
///
/// #[derive(Debug, Deserialize, Validate)]
/// struct Cfg {
///     #[validate(range(min = 1024))]
///     port: u16,
/// }
///
/// let yaml = "port: 22\n";
/// let wrapped: Spanned<Cfg> = noyalib::from_str(yaml).unwrap();
/// if let Err(errs) = wrapped.value.validate() {
///     let report = validator_errors_to_miette(&wrapped, &errs, yaml, "config.yaml");
///     drop(report);
/// }
/// ```
#[cfg(all(feature = "miette", feature = "validator"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "miette", feature = "validator"))))]
#[must_use]
pub fn validator_errors_to_miette<T>(
    spanned: &Spanned<T>,
    errors: &validator::ValidationErrors,
    source: &str,
    name: impl Into<String>,
) -> miette::Report {
    let summary = format_validator(errors);
    let diag = ValidationDiagnostic::new(spanned, source.to_string(), summary, name.into());
    miette::Report::new(diag)
}

#[cfg(feature = "garde")]
fn format_garde(errors: &garde::Report) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(64);
    out.push_str("validation failed: ");
    let mut first = true;
    for (path, error) in errors.iter() {
        if !first {
            out.push_str("; ");
        }
        first = false;
        let _ = write!(out, "{path}: {error}");
    }
    if first {
        out.push_str("<no details>");
    }
    out
}

#[cfg(feature = "validator")]
fn format_validator(errors: &validator::ValidationErrors) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(64);
    out.push_str("validation failed: ");
    let mut first = true;
    for (field, errs) in errors.field_errors() {
        if !first {
            out.push_str("; ");
        }
        first = false;
        let _ = write!(out, "{field}: ");
        let mut f_first = true;
        for e in errs {
            if !f_first {
                out.push_str(", ");
            }
            f_first = false;
            let _ = write!(out, "{e}");
        }
    }
    if first {
        out.push_str("<no details>");
    }
    out
}

/// Helper: clamp a span end so it never lands before its start.
/// Exposed for [`crate::Spanned`] integrations that compute their
/// own byte range and want to plug straight into a miette source
/// span without re-implementing the clamp.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "miette")]
/// # {
/// use noyalib::validated_miette::clamped_span;
/// let span: miette::SourceSpan = clamped_span(10, 20, 100);
/// assert_eq!(span.offset(), 10);
/// assert_eq!(span.len(), 10);
/// # }
/// ```
#[cfg(feature = "miette")]
#[cfg_attr(docsrs, doc(cfg(feature = "miette")))]
#[must_use]
pub fn clamped_span(start: usize, end: usize, source_len: usize) -> miette::SourceSpan {
    let s = start.min(source_len);
    let e = end.max(s).min(source_len);
    let e = if e == s {
        (s + 1).min(source_len.max(s + 1))
    } else {
        e
    };
    (s, e - s).into()
}
