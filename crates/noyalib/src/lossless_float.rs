//! A float that refuses to silently lose information.
//!
//! [`LosslessFloat`](crate::lossless_float::LosslessFloat) is the floating-point sibling of the
//! `lossless-u64` feature's integer guarantee: an opt-in "refuse to
//! silently lose precision" number type. Deserialization rejects
//! infinities, NaN, and any value that does not survive a
//! format-and-reparse round trip, so a config field declared
//! `LosslessFloat` can never quietly hold a distorted number.
//!
//! Formerly `noyalib::robotics::StrictFloat` — the mechanism was
//! never robotics-specific, only the label; the `robotics` module
//! carried deprecated aliases for one release (v0.0.29) and was
//! removed in v0.0.30.
//!
//! # Examples
//!
//! ```
//! use noyalib::lossless_float::LosslessFloat;
//!
//! let f: LosslessFloat = noyalib::from_str("3.14159").unwrap();
//! assert!((f.get() - 3.14159).abs() < 1e-10);
//!
//! // Infinity is rejected.
//! let result: Result<LosslessFloat, _> = noyalib::from_str(".inf");
//! assert!(result.is_err());
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// The glob, not `std` or piecemeal names: `lossless-float` without
// `std` is a valid combination (checked by the weekly
// feature-powerset sweep), and which prelude names the round-trip
// check needs shifts with `fast-float` (`format!` without it,
// `ToOwned` with it) — a glob import stays correct and warning-free
// under every pairing.
use crate::prelude::*;

/// A float that rejects values outside f64's precise representation range.
///
/// The round-trip invariant is: if a value loses precision when converted
/// to `f64` and back, construction fails. This catches values like
/// `1e308 * 2` (infinity) or subnormals that cannot be faithfully
/// represented.
///
/// The check runs on the decoded `f64` value, after the parser's own
/// scalar resolution — it guards the *value*, not the source lexeme.
///
/// # Examples
///
/// ```rust
/// use noyalib::lossless_float::LosslessFloat;
///
/// let f: LosslessFloat = noyalib::from_str("3.14159").unwrap();
/// assert!((f.get() - 3.14159).abs() < 1e-10);
///
/// // Infinity is rejected.
/// let result: Result<LosslessFloat, _> = noyalib::from_str(".inf");
/// assert!(result.is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LosslessFloat(f64);

/// Error returned when a float value fails the precision check.
///
/// # Examples
///
/// ```
/// use noyalib::lossless_float::LosslessFloat;
/// let err = LosslessFloat::try_from(f64::INFINITY).unwrap_err();
/// assert!(err.to_string().contains("not precisely representable"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct LosslessFloatError(f64);

impl fmt::Display for LosslessFloatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "value {} is not precisely representable as f64", self.0)
    }
}

impl TryFrom<f64> for LosslessFloat {
    type Error = LosslessFloatError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_infinite() || value.is_nan() {
            return Err(LosslessFloatError(value));
        }
        // Check round-trip: format and re-parse to verify no precision loss.
        #[cfg(feature = "fast-float")]
        let repr = ryu::Buffer::new().format(value).to_owned();
        #[cfg(not(feature = "fast-float"))]
        let repr = format!("{value:?}");
        let roundtrip: f64 = repr.parse().unwrap_or(f64::NAN);
        if roundtrip != value {
            return Err(LosslessFloatError(value));
        }
        Ok(Self(value))
    }
}

// Manual impls against `serde_core` rather than derives: the old
// `robotics` home needed `dep:serde` only for the derive machinery —
// this module needs nothing beyond the mandatory `serde_core`.
impl serde_core::Serialize for LosslessFloat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_core::Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> serde_core::Deserialize<'de> for LosslessFloat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        let v = f64::deserialize(deserializer)?;
        Self::try_from(v).map_err(serde_core::de::Error::custom)
    }
}

impl LosslessFloat {
    /// Returns the inner `f64` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::lossless_float::LosslessFloat;
    /// let f = LosslessFloat::try_from(2.5).unwrap();
    /// assert_eq!(f.get(), 2.5);
    /// ```
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_precise() {
        let f: LosslessFloat = crate::from_str("1.23456789").unwrap();
        assert!((f.get() - 1.234_567_89).abs() < 1e-15);
    }

    #[test]
    fn rejects_infinity() {
        let result: Result<LosslessFloat, _> = crate::from_str(".inf");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_nan() {
        let result: Result<LosslessFloat, _> = crate::from_str(".nan");
        assert!(result.is_err());
    }

    #[test]
    fn zero() {
        let f: LosslessFloat = crate::from_str("0.0").unwrap();
        assert!((f.get()).abs() < 1e-15);
    }

    #[test]
    fn negative() {
        let f: LosslessFloat = crate::from_str("-1.5").unwrap();
        assert!((f.get() + 1.5).abs() < 1e-15);
    }

    #[test]
    fn serializes_transparently() {
        let f = LosslessFloat::try_from(2.5).unwrap();
        let yaml = crate::to_string(&f).unwrap();
        assert!(yaml.contains("2.5"));
    }
}
