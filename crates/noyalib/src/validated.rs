// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Declarative validation via [`garde`] or [`validator`].
//!
//! Provides [`Validated<T>`] (for `garde`) and [`ValidatedValidator<T>`]
//! (for `validator`), which are transparent wrappers that run validation
//! rules after deserialisation.
//!
//! # Example (garde)
//!
//! ```rust
//! use noyalib::Validated;
//! use garde::Validate;
//!
//! #[derive(serde::Deserialize, Validate)]
//! struct Server {
//!     #[garde(length(min = 1, max = 255))]
//!     host: String,
//!     #[garde(range(min = 1024, max = 65535))]
//!     port: u16,
//! }
//!
//! let yaml = "host: db.local\nport: 5432\n";
//! let wrapped: Validated<Server> = noyalib::from_str(yaml).unwrap();
//! assert_eq!(wrapped.0.host, "db.local");
//! ```

use crate::prelude::*;
use core::ops::{Deref, DerefMut};
use serde::{Deserialize, Deserializer, Serialize};

/// A newtype that validates its inner value using the [`garde`] crate.
///
/// Requires the `garde` feature.
///
/// # Examples
///
/// ```
/// use noyalib::Validated;
/// use garde::Validate;
///
/// #[derive(serde::Deserialize, Validate)]
/// struct Port {
///     #[garde(range(min = 1024))]
///     n: u16,
/// }
/// let v: Validated<Port> = noyalib::from_str("n: 8080\n").unwrap();
/// assert_eq!(v.0.n, 8080);
/// ```
#[cfg(feature = "garde")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Validated<T>(pub T);

/// A newtype that validates its inner value using the [`validator`] crate.
///
/// Requires the `validator` feature.
///
/// # Examples
///
/// ```
/// use noyalib::ValidatedValidator;
/// use validator::Validate;
///
/// #[derive(serde::Deserialize, Validate)]
/// struct Port {
///     #[validate(range(min = 1024))]
///     n: u16,
/// }
/// let v: ValidatedValidator<Port> = noyalib::from_str("n: 8080\n").unwrap();
/// assert_eq!(v.0.n, 8080);
/// ```
#[cfg(feature = "validator")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValidatedValidator<T>(pub T);

#[cfg(feature = "garde")]
impl<T> Validated<T> {
    /// Consume the wrapper and return the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Validated;
    /// let v: Validated<u32> = Validated(42);
    /// assert_eq!(v.into_inner(), 42);
    /// ```
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(feature = "validator")]
impl<T> ValidatedValidator<T> {
    /// Consume the wrapper and return the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ValidatedValidator;
    /// let v: ValidatedValidator<u32> = ValidatedValidator(42);
    /// assert_eq!(v.into_inner(), 42);
    /// ```
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(feature = "garde")]
impl<T> Deref for Validated<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

#[cfg(feature = "validator")]
impl<T> Deref for ValidatedValidator<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

#[cfg(feature = "garde")]
impl<T> DerefMut for Validated<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

#[cfg(feature = "validator")]
impl<T> DerefMut for ValidatedValidator<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

#[cfg(feature = "garde")]
impl<T> From<T> for Validated<T> {
    #[inline]
    fn from(v: T) -> Self {
        Validated(v)
    }
}

#[cfg(feature = "validator")]
impl<T> From<T> for ValidatedValidator<T> {
    #[inline]
    fn from(v: T) -> Self {
        ValidatedValidator(v)
    }
}

#[cfg(feature = "garde")]
impl<'de, T> Deserialize<'de> for Validated<T>
where
    T: Deserialize<'de> + garde::Validate<Context = ()>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde_core::de::Error as _;
        let inner = T::deserialize(deserializer)?;
        match inner.validate_with(&()) {
            Ok(()) => Ok(Validated(inner)),
            Err(report) => Err(D::Error::custom(format_report(&report))),
        }
    }
}

#[cfg(feature = "validator")]
impl<'de, T> Deserialize<'de> for ValidatedValidator<T>
where
    T: Deserialize<'de> + validator::Validate,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde_core::de::Error as _;
        let inner = T::deserialize(deserializer)?;
        match inner.validate() {
            Ok(()) => Ok(ValidatedValidator(inner)),
            Err(errors) => Err(D::Error::custom(format_validator_errors(&errors))),
        }
    }
}

#[cfg(feature = "garde")]
impl<T: Serialize> Serialize for Validated<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "validator")]
impl<T: Serialize> Serialize for ValidatedValidator<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        self.0.serialize(serializer)
    }
}

/// Format a garde report as a compact single-line message.
#[cfg(feature = "garde")]
fn format_report(report: &garde::Report) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(64);
    out.push_str("validation failed: ");
    let mut first = true;
    for (path, error) in report.iter() {
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

/// Format validator errors as a compact single-line message.
#[cfg(feature = "validator")]
fn format_validator_errors(errors: &validator::ValidationErrors) -> String {
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
