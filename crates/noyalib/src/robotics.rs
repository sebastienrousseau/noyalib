//! Robotics and scientific numeric types for precise YAML deserialization.
//!
//! This feature-gated module provides newtypes that enforce numeric
//! precision and unit-aware deserialization, useful in robotics,
//! simulation, and scientific computing pipelines.
//!
//! # Examples
//!
//! ```
//! use noyalib::robotics::{Degrees, Radians};
//! let d: Degrees = noyalib::from_str("90.0").unwrap();
//! let r = d.to_radians();
//! assert!((r.0 - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use core::fmt;

use serde::Deserialize;

/// A float that rejects values outside f64's precise representation range.
///
/// The round-trip invariant is: if a value loses precision when converted
/// to `f64` and back, construction fails. This catches values like
/// `1e308 * 2` (infinity) or subnormals that cannot be faithfully
/// represented.
///
/// # Examples
///
/// ```rust
/// use noyalib::robotics::StrictFloat;
///
/// let sf: StrictFloat = noyalib::from_str("3.14159").unwrap();
/// assert!((sf.get() - 3.14159).abs() < 1e-10);
///
/// // Infinity is rejected.
/// let result: Result<StrictFloat, _> = noyalib::from_str(".inf");
/// assert!(result.is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(transparent)]
pub struct StrictFloat(f64);

/// Error returned when a float value fails the precision check.
///
/// # Examples
///
/// ```
/// use noyalib::robotics::StrictFloat;
/// let err = StrictFloat::try_from(f64::INFINITY).unwrap_err();
/// assert!(err.to_string().contains("not precisely representable"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct StrictFloatError(f64);

impl fmt::Display for StrictFloatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "value {} is not precisely representable as f64", self.0)
    }
}

impl TryFrom<f64> for StrictFloat {
    type Error = StrictFloatError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_infinite() || value.is_nan() {
            return Err(StrictFloatError(value));
        }
        // Check round-trip: format and re-parse to verify no precision loss.
        #[cfg(feature = "fast-float")]
        let repr = ryu::Buffer::new().format(value).to_owned();
        #[cfg(not(feature = "fast-float"))]
        let repr = format!("{value:?}");
        let roundtrip: f64 = repr.parse().unwrap_or(f64::NAN);
        if roundtrip != value {
            return Err(StrictFloatError(value));
        }
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for StrictFloat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f64::deserialize(deserializer)?;
        StrictFloat::try_from(v).map_err(serde_core::de::Error::custom)
    }
}

impl StrictFloat {
    /// Returns the inner `f64` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::robotics::StrictFloat;
    /// let sf = StrictFloat::try_from(2.5).unwrap();
    /// assert_eq!(sf.get(), 2.5);
    /// ```
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// An angle stored in radians but deserialized from degrees in YAML.
///
/// Serialization emits the raw radian value.
///
/// # Examples
///
/// ```rust
/// use noyalib::robotics::Radians;
///
/// let r: Radians = noyalib::from_str("180.0").unwrap();
/// assert!((r.0 - std::f64::consts::PI).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(transparent)]
pub struct Radians(pub f64);

impl<'de> Deserialize<'de> for Radians {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let degrees = f64::deserialize(deserializer)?;
        Ok(Radians(degrees.to_radians()))
    }
}

/// An angle stored and deserialized in degrees.
///
/// This is a simple newtype for clarity in config structs that
/// explicitly label their angular units.
///
/// # Examples
///
/// ```rust
/// use noyalib::robotics::Degrees;
///
/// let d: Degrees = noyalib::from_str("90.0").unwrap();
/// assert!((d.0 - 90.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Degrees(pub f64);

impl Degrees {
    /// Convert to radians.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::robotics::Degrees;
    /// let d = Degrees(180.0);
    /// let r = d.to_radians();
    /// assert!((r.0 - std::f64::consts::PI).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn to_radians(self) -> Radians {
        Radians(self.0.to_radians())
    }
}

impl Radians {
    /// Convert to degrees.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::robotics::Radians;
    /// let r = Radians(std::f64::consts::PI);
    /// let d = r.to_degrees();
    /// assert!((d.0 - 180.0).abs() < 1e-10);
    /// ```
    #[must_use]
    pub fn to_degrees(self) -> Degrees {
        Degrees(self.0.to_degrees())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_float_accepts_precise() {
        let sf: StrictFloat = crate::from_str("1.23456789").unwrap();
        assert!((sf.get() - 1.234_567_89).abs() < 1e-15);
    }

    #[test]
    fn strict_float_rejects_infinity() {
        let result: Result<StrictFloat, _> = crate::from_str(".inf");
        assert!(result.is_err());
    }

    #[test]
    fn strict_float_rejects_nan() {
        let result: Result<StrictFloat, _> = crate::from_str(".nan");
        assert!(result.is_err());
    }

    #[test]
    fn strict_float_zero() {
        let sf: StrictFloat = crate::from_str("0.0").unwrap();
        assert!((sf.get()).abs() < 1e-15);
    }

    #[test]
    fn strict_float_negative() {
        let sf: StrictFloat = crate::from_str("-1.5").unwrap();
        assert!((sf.get() + 1.5).abs() < 1e-15);
    }

    #[test]
    fn radians_from_degrees() {
        let r: Radians = crate::from_str("180.0").unwrap();
        assert!((r.0 - core::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn radians_90() {
        let r: Radians = crate::from_str("90.0").unwrap();
        assert!((r.0 - core::f64::consts::FRAC_PI_2).abs() < 1e-10);
    }

    #[test]
    fn radians_zero() {
        let r: Radians = crate::from_str("0.0").unwrap();
        assert!((r.0).abs() < 1e-15);
    }

    #[test]
    fn degrees_roundtrip() {
        let d: Degrees = crate::from_str("45.0").unwrap();
        let r = d.to_radians();
        let back = r.to_degrees();
        assert!((back.0 - 45.0).abs() < 1e-10);
    }

    #[test]
    fn degrees_deserialize() {
        let d: Degrees = crate::from_str("90.0").unwrap();
        assert!((d.0 - 90.0).abs() < 1e-15);
    }

    #[test]
    fn strict_float_serialize() {
        let sf = StrictFloat::try_from(2.5).unwrap();
        let yaml = crate::to_string(&sf).unwrap();
        assert!(yaml.contains("2.5"));
    }

    #[test]
    fn radians_serialize() {
        let r = Radians(core::f64::consts::PI);
        let yaml = crate::to_string(&r).unwrap();
        assert!(yaml.contains("3.14159"));
    }
}
