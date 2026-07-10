//! Spanned value to `miette::Report` bridge.
//!
//! Converts validation errors on [`Spanned<T>`](crate::Spanned) values
//! into rich [`miette::Report`] diagnostics with source spans, so that
//! CLI tools get underlined error output for free.
//!
//! # Examples
//!
//! ```
//! use noyalib::{from_str, Spanned, diagnostic::spanned_error};
//!
//! #[derive(serde::Deserialize)]
//! struct Cfg { port: Spanned<u16> }
//! let yaml = "port: 80\n";
//! let cfg: Cfg = from_str(yaml).unwrap();
//! let report = spanned_error(yaml, &cfg.port, "port must be >= 1024");
//! assert!(format!("{report}").contains("port must be >= 1024"));
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::spanned::Spanned;
use core::fmt;

/// A diagnostic error tied to one or more source spans.
///
/// Implements [`miette::Diagnostic`] so it renders with highlighted
/// source regions in terminals that support it.
#[derive(Debug)]
struct SpannedDiagnostic {
    message: String,
    labels: Vec<miette::LabeledSpan>,
    source_code: String,
}

impl fmt::Display for SpannedDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SpannedDiagnostic {}

impl miette::Diagnostic for SpannedDiagnostic {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new("noyalib::validation"))
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.source_code)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        if self.labels.is_empty() {
            None
        } else {
            Some(Box::new(self.labels.iter().cloned()))
        }
    }
}

/// Create a [`miette::Report`] pointing at the source span of a
/// [`Spanned<T>`](crate::Spanned) value.
///
/// This bridges noyalib's source-location tracking with miette's rich
/// terminal diagnostic output, letting users validate deserialized
/// config and report errors that point back to the exact YAML source.
///
/// # Examples
///
/// ```
/// use noyalib::{from_str, Spanned, diagnostic::spanned_error};
///
/// #[derive(serde::Deserialize)]
/// struct Cfg { port: Spanned<u16> }
/// let yaml = "port: 80\n";
/// let cfg: Cfg = from_str(yaml).unwrap();
/// let report = spanned_error(yaml, &cfg.port, "too low");
/// assert!(format!("{report}").contains("too low"));
/// ```
pub fn spanned_error<T, M: fmt::Display>(
    source: &str,
    span: &Spanned<T>,
    message: M,
) -> miette::Report {
    let msg = message.to_string();
    let start = span.start.index();
    let end = span.end.index();
    let len = if end > start { end - start } else { 1 };

    miette::Report::new(SpannedDiagnostic {
        message: msg.clone(),
        labels: vec![miette::LabeledSpan::new(Some(msg), start, len)],
        source_code: source.to_owned(),
    })
}

/// Create a [`miette::Report`] pointing at a primary span with additional
/// context from a secondary span.
///
/// Useful for errors involving two locations, such as an alias error that
/// points to both the alias usage and the original anchor definition.
///
/// # Examples
///
/// ```rust,no_run
/// # use noyalib::{from_str, Spanned, diagnostic::spanned_error_with_context};
/// #[derive(serde::Deserialize)]
/// struct Cfg {
///     anchor: Spanned<String>,
///     alias: Spanned<String>,
/// }
/// let yaml = "anchor: &a 1\nalias: *a";
/// let cfg: Cfg = from_str(yaml).unwrap();
///
/// let report = spanned_error_with_context(
///     yaml,
///     &cfg.alias,
///     "circular reference",
///     &cfg.anchor,
///     "defined here",
/// );
/// ```
pub fn spanned_error_with_context<T, U, M: fmt::Display, C: fmt::Display>(
    source: &str,
    primary_span: &Spanned<T>,
    primary_message: M,
    context_span: &Spanned<U>,
    context_message: C,
) -> miette::Report {
    let p_msg = primary_message.to_string();
    let p_start = primary_span.start.index();
    let p_end = primary_span.end.index();
    let p_len = if p_end > p_start { p_end - p_start } else { 1 };

    let c_msg = context_message.to_string();
    let c_start = context_span.start.index();
    let c_end = context_span.end.index();
    let c_len = if c_end > c_start { c_end - c_start } else { 1 };

    miette::Report::new(SpannedDiagnostic {
        message: p_msg.clone(),
        labels: vec![
            miette::LabeledSpan::new(Some(p_msg), p_start, p_len),
            miette::LabeledSpan::new(Some(c_msg), c_start, c_len),
        ],
        source_code: source.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Spanned;

    #[test]
    fn spanned_error_creates_report() {
        let yaml = "port: 80\n";
        #[derive(serde::Deserialize)]
        struct Cfg {
            port: Spanned<u16>,
        }
        let cfg: Cfg = crate::from_str(yaml).unwrap();
        let report = spanned_error(yaml, &cfg.port, "port must be >= 1024");
        let msg = format!("{report}");
        assert!(msg.contains("port must be >= 1024"));
    }

    #[test]
    fn spanned_error_diagnostic_has_labels() {
        use miette::Diagnostic;

        let yaml = "value: 42\n";
        #[derive(serde::Deserialize)]
        struct Cfg {
            value: Spanned<i32>,
        }
        let cfg: Cfg = crate::from_str(yaml).unwrap();
        let report = spanned_error(yaml, &cfg.value, "too small");

        // The underlying diagnostic should have labels.
        let diag: &dyn Diagnostic = report.as_ref();
        assert!(diag.labels().is_some());
        assert!(diag.code().is_some());
        assert!(diag.source_code().is_some());
    }
}
