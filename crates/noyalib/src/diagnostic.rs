//! Framework-neutral structured diagnostics and optional `miette` bridges.
//!
//! [`Diagnostic`] is the canonical machine-readable representation used by
//! terminal renderers, editor protocols, and service integrations. Optional
//! adapters translate that payload into ecosystem-specific formats instead of
//! maintaining separate error-code, help, and label tables.
//!
//! # Examples
//!
//! ```
//! use noyalib::{DiagnosticCode, Value, from_str};
//!
//! let err = from_str::<Value>("key: [unclosed").unwrap_err();
//! let diagnostic = err.diagnostic();
//! assert_eq!(diagnostic.code(), DiagnosticCode::Parse);
//! assert!(!diagnostic.labels().is_empty());
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::{String, Vec};
use core::fmt;

#[cfg(feature = "miette")]
use crate::spanned::Spanned;

/// Stable machine-readable code for a noyalib diagnostic.
///
/// Codes are independent of rendered prose, so an LSP client or service can
/// route diagnostics without parsing [`Diagnostic::message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// YAML syntax could not be parsed.
    Parse,
    /// A value could not be serialized.
    Serialize,
    /// Input could not be deserialized into the requested type.
    Deserialize,
    /// A value had a different type from the requested one.
    TypeMismatch,
    /// A required field was absent.
    MissingField,
    /// An undeclared field was present.
    UnknownField,
    /// The configured recursion limit was exceeded.
    RecursionLimit,
    /// The alias expansion limit was exceeded.
    RepetitionLimit,
    /// A configurable resource budget was exceeded.
    Budget,
    /// An alias referred to an unavailable anchor.
    UnknownAnchor,
    /// A mapping contained a duplicate key.
    DuplicateKey,
    /// Distinct YAML keys collapsed to one string key.
    KeyCollision,
    /// An integer exceeded the configured representation.
    IntegerOverflow,
    /// A mapping key violated the scalar-key policy.
    NonScalarKey,
    /// Input ended before the current construct completed.
    EndOfStream,
    /// A single-document API received more than one document.
    MoreThanOneDocument,
    /// An I/O operation failed.
    Io,
    /// The error has no more specific stable code.
    Other,
}

impl DiagnosticCode {
    /// Return the stable namespaced spelling used on external protocols.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "noyalib::parse",
            Self::Serialize => "noyalib::serialize",
            Self::Deserialize => "noyalib::deserialize",
            Self::TypeMismatch => "noyalib::type_mismatch",
            Self::MissingField => "noyalib::missing_field",
            Self::UnknownField => "noyalib::unknown_field",
            Self::RecursionLimit => "noyalib::recursion_limit",
            Self::RepetitionLimit => "noyalib::repetition_limit",
            Self::Budget => "noyalib::budget",
            Self::UnknownAnchor => "noyalib::unknown_anchor",
            Self::DuplicateKey => "noyalib::duplicate_key",
            Self::KeyCollision => "noyalib::key_collision",
            Self::IntegerOverflow => "noyalib::integer_overflow",
            Self::NonScalarKey => "noyalib::non_scalar_key",
            Self::EndOfStream => "noyalib::eof",
            Self::MoreThanOneDocument => "noyalib::multi_document",
            Self::Io => "noyalib::io",
            Self::Other => "noyalib::error",
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Protocol-neutral diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticSeverity {
    /// Processing cannot continue for the affected value.
    Error,
    /// Processing succeeded, but the input is risky or discouraged.
    Warning,
    /// Informational context with no required action.
    Information,
    /// A low-priority suggestion.
    Hint,
}

/// A half-open byte range in the original UTF-8 source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    offset: usize,
    length: usize,
}

impl SourceSpan {
    /// Create a byte span beginning at `offset` and covering `length` bytes.
    #[must_use]
    pub const fn new(offset: usize, length: usize) -> Self {
        Self { offset, length }
    }

    /// Return the zero-based byte offset.
    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset
    }

    /// Return the span length in bytes.
    #[must_use]
    pub const fn length(self) -> usize {
        self.length
    }

    /// Return the exclusive end offset, saturating on integer overflow.
    #[must_use]
    pub const fn end(self) -> usize {
        self.offset.saturating_add(self.length)
    }
}

/// A message attached to one source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    span: SourceSpan,
    message: String,
    primary: bool,
}

impl DiagnosticLabel {
    /// Create the primary label for a diagnostic.
    #[must_use]
    pub fn primary(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            primary: true,
        }
    }

    /// Create a secondary context label.
    #[must_use]
    pub fn secondary(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            primary: false,
        }
    }

    /// Return the labelled source span.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Return the label text.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return whether this is the primary label.
    #[must_use]
    pub const fn is_primary(&self) -> bool {
        self.primary
    }
}

/// Framework-neutral diagnostic payload derived from an error or validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    severity: DiagnosticSeverity,
    message: String,
    help: Option<String>,
    labels: Vec<DiagnosticLabel>,
}

impl Diagnostic {
    /// Create a diagnostic without help text or source labels.
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity,
            message: message.into(),
            help: None,
            labels: Vec::new(),
        }
    }

    /// Attach actionable help text.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Attach one source label.
    #[must_use]
    pub fn with_label(mut self, label: DiagnosticLabel) -> Self {
        self.labels.push(label);
        self
    }

    /// Return the stable machine-readable code.
    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// Return the protocol-neutral severity.
    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Return the rendered summary message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return optional actionable help text.
    #[must_use]
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    /// Return all labels in rendering order, primary first.
    #[must_use]
    pub fn labels(&self) -> &[DiagnosticLabel] {
        &self.labels
    }

    /// Return the primary label, when the error has a source location.
    #[must_use]
    pub fn primary_label(&self) -> Option<&DiagnosticLabel> {
        self.labels.iter().find(|label| label.is_primary())
    }
}

/// A diagnostic error tied to one or more source spans.
///
/// Implements [`miette::Diagnostic`] so it renders with highlighted
/// source regions in terminals that support it.
#[derive(Debug)]
#[cfg(feature = "miette")]
struct SpannedDiagnostic {
    message: String,
    labels: Vec<miette::LabeledSpan>,
    source_code: String,
}

#[cfg(feature = "miette")]
impl fmt::Display for SpannedDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[cfg(feature = "miette")]
impl std::error::Error for SpannedDiagnostic {}

#[cfg(feature = "miette")]
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
#[cfg(feature = "miette")]
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
#[cfg(feature = "miette")]
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

#[cfg(all(test, feature = "miette"))]
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
