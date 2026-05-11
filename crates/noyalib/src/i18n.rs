// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Pluggable error-message formatters for user-facing rendering.
//!
//! [`crate::Error`]'s `Display` impl is developer-facing — it
//! preserves the noyalib-internal vocabulary (`!!binary`, "merge
//! key", "recursion depth limit") that's useful for debugging but
//! noisy for end-users of a config-loading binary. The
//! [`MessageFormatter`] trait lets callers plug in their own
//! formatting strategy: localisation tables, simplification,
//! richer formatting, or anything else.
//!
//! Two implementations ship in-tree:
//!
//! * [`DefaultFormatter`] — preserves the standard developer-facing
//!   message verbatim. Equivalent to `format!("{err}")`.
//! * [`UserFormatter`] — collapses noyalib's diagnostic vocabulary
//!   into short user-friendly sentences ("The configuration file
//!   has a syntax error on line 5.") suitable for surfacing in
//!   GUIs and `--help`-style command output.
//!
//! Use [`crate::Error::render_with_formatter`] to render an error
//! through a chosen formatter.
//!
//! # Examples
//!
//! ```
//! use noyalib::i18n::{DefaultFormatter, UserFormatter};
//! use noyalib::{from_str, Value};
//!
//! let err = from_str::<Value>("a: [unclosed").unwrap_err();
//! let dev = err.render_with_formatter(&DefaultFormatter);
//! let user = err.render_with_formatter(&UserFormatter);
//! assert!(!dev.is_empty());
//! assert!(!user.is_empty());
//! ```

use crate::error::Error;
use crate::prelude::*;

/// Pluggable formatter for converting an [`Error`] into a
/// user-visible message.
///
/// Implement this trait to plug in localisation, simplification,
/// or rich formatting strategies. The trait is `Send + Sync` so a
/// single formatter instance can be shared across threads.
pub trait MessageFormatter: Send + Sync {
    /// Render the supplied error as a single-string message.
    fn format(&self, error: &Error) -> String;
}

/// Default formatter — preserves the standard developer-facing
/// message verbatim.
///
/// Equivalent to `format!("{err}")`. The reference implementation
/// `MessageFormatter` consumers should compare against.
///
/// # Examples
///
/// ```
/// use noyalib::i18n::{DefaultFormatter, MessageFormatter};
/// use noyalib::{from_str, Value};
///
/// let err = from_str::<Value>("a: [unclosed").unwrap_err();
/// let s = DefaultFormatter.format(&err);
/// assert_eq!(s, err.to_string());
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultFormatter;

impl MessageFormatter for DefaultFormatter {
    fn format(&self, error: &Error) -> String {
        error.to_string()
    }
}

/// User-facing formatter — collapses noyalib's diagnostic
/// vocabulary into short, plain-language sentences appropriate
/// for non-developer audiences (CLI `--help` text, GUI alert
/// dialogs).
///
/// Maps the major [`Error`] variants onto user-readable
/// templates:
///
/// | Variant family | User message |
/// | :--- | :--- |
/// | `Parse`, `ParseWithLocation` | `"The configuration file has a syntax error at line N."` |
/// | `Deserialize`, `DeserializeWithLocation` | `"The configuration file does not match the expected shape."` |
/// | `Io` | `"Could not read the configuration file."` |
/// | `RecursionLimitExceeded`, `Budget`, `RepetitionLimitExceeded` | `"The configuration file is too large or deeply nested."` |
/// | `DuplicateKey` | `"A configuration key appears twice."` |
/// | `UnknownAnchor`, `UnknownAnchorAt` | `"A configuration reference points at something that does not exist."` |
/// | `MissingField` | `"A required configuration field is missing."` |
/// | `TypeMismatch` | `"A configuration value has the wrong type."` |
/// | other | `"The configuration file is invalid."` |
///
/// Line numbers are included when the source location is
/// available; sensitive field names and noyalib internal terms
/// (`!!binary`, "merge key") are stripped.
///
/// # Examples
///
/// ```
/// use noyalib::i18n::{MessageFormatter, UserFormatter};
/// use noyalib::{from_str, Value};
///
/// let err = from_str::<Value>("a: [unclosed").unwrap_err();
/// let msg = UserFormatter.format(&err);
/// assert!(msg.contains("syntax error"));
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct UserFormatter;

impl MessageFormatter for UserFormatter {
    fn format(&self, error: &Error) -> String {
        match error {
            Error::Parse(_) => "The configuration file has a syntax error.".to_string(),
            Error::ParseWithLocation { location, .. } => format!(
                "The configuration file has a syntax error at line {}.",
                location.line()
            ),
            Error::Deserialize(_) => {
                "The configuration file does not match the expected shape.".to_string()
            }
            Error::DeserializeWithLocation { location, .. } => format!(
                "The configuration file does not match the expected shape at line {}.",
                location.line()
            ),
            #[cfg(feature = "std")]
            Error::Io(_) => "Could not read the configuration file.".to_string(),
            Error::RecursionLimitExceeded { .. }
            | Error::Budget(_)
            | Error::RepetitionLimitExceeded => {
                "The configuration file is too large or deeply nested.".to_string()
            }
            Error::DuplicateKey(_) => "A configuration key appears twice.".to_string(),
            Error::UnknownAnchor(_) | Error::UnknownAnchorAt { .. } => {
                "A configuration reference points at something that does not exist.".to_string()
            }
            Error::MissingField(_) => "A required configuration field is missing.".to_string(),
            Error::TypeMismatch { .. } => "A configuration value has the wrong type.".to_string(),
            _ => "The configuration file is invalid.".to_string(),
        }
    }
}

impl Error {
    /// Render this error via a custom [`MessageFormatter`].
    ///
    /// Pairs with [`DefaultFormatter`] (developer-facing,
    /// verbatim) and [`UserFormatter`] (user-facing, simplified).
    /// Callers needing localisation or rich formatting plug in
    /// their own `MessageFormatter` impl.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::i18n::UserFormatter;
    /// use noyalib::{from_str, Value};
    ///
    /// let err = from_str::<Value>("a: [unclosed").unwrap_err();
    /// let msg = err.render_with_formatter(&UserFormatter);
    /// assert!(msg.contains("syntax error"));
    /// ```
    #[must_use]
    pub fn render_with_formatter(&self, formatter: &dyn MessageFormatter) -> String {
        formatter.format(self)
    }
}
