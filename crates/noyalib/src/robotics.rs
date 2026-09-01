//! Deprecated: renamed and dissolving — one release of aliases.
//!
//! `StrictFloat` was never robotics-specific; it is noyalib's
//! "refuse to silently lose precision" float, and it now lives at
//! [`crate::lossless_float::LosslessFloat`] behind the
//! `lossless-float` feature (the floating-point sibling of
//! `lossless-u64`). The aliases here, the [`Degrees`](crate::robotics::Degrees) /
//! [`Radians`](crate::robotics::Radians)
//! unit newtypes, and the `robotics` feature flag itself are
//! deprecated and will be removed in the next release, per the
//! crate's deprecation policy.
//!
//! Migration:
//!
//! - `robotics::StrictFloat` → `lossless_float::LosslessFloat`
//!   (feature `lossless-float`; no serde-derive dependency needed).
//! - `Degrees` / `Radians` are ~40 lines of domain newtype with no
//!   dependence on noyalib — copy them into your own code.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

/// Deprecated alias for [`crate::lossless_float::LosslessFloat`].
#[deprecated(
    since = "0.0.29",
    note = "renamed to `noyalib::lossless_float::LosslessFloat` (feature \
            `lossless-float`); the `robotics` module and feature are removed \
            in the next release"
)]
pub type StrictFloat = crate::lossless_float::LosslessFloat;

/// Deprecated alias for [`crate::lossless_float::LosslessFloatError`].
#[deprecated(
    since = "0.0.29",
    note = "renamed to `noyalib::lossless_float::LosslessFloatError` (feature \
            `lossless-float`); the `robotics` module and feature are removed \
            in the next release"
)]
pub type StrictFloatError = crate::lossless_float::LosslessFloatError;

/// An angle stored in radians but deserialized from degrees in YAML.
///
/// Serialization emits the raw radian value.
///
/// # Examples
///
/// ```rust
/// # #![allow(deprecated)]
/// use noyalib::robotics::Radians;
///
/// let r: Radians = noyalib::from_str("180.0").unwrap();
/// assert!((r.0 - std::f64::consts::PI).abs() < 1e-10);
/// ```
#[deprecated(
    since = "0.0.29",
    note = "domain unit newtypes are leaving noyalib with the `robotics` \
            module next release — copy the type into your own code (it has \
            no dependence on noyalib)"
)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(transparent)]
pub struct Radians(pub f64);

#[allow(deprecated)]
impl<'de> serde_core::Deserialize<'de> for Radians {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_core::Deserializer<'de>,
    {
        let degrees = f64::deserialize(deserializer)?;
        Ok(Self(degrees.to_radians()))
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
/// # #![allow(deprecated)]
/// use noyalib::robotics::Degrees;
///
/// let d: Degrees = noyalib::from_str("90.0").unwrap();
/// assert!((d.0 - 90.0).abs() < 1e-10);
/// ```
#[deprecated(
    since = "0.0.29",
    note = "domain unit newtypes are leaving noyalib with the `robotics` \
            module next release — copy the type into your own code (it has \
            no dependence on noyalib)"
)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Degrees(pub f64);

#[allow(deprecated)]
impl Degrees {
    /// Convert to radians.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![allow(deprecated)]
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

#[allow(deprecated)]
impl Radians {
    /// Convert to degrees.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![allow(deprecated)]
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
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn strict_float_alias_still_deserializes() {
        let sf: StrictFloat = crate::from_str("1.23456789").unwrap();
        assert!((sf.get() - 1.234_567_89).abs() < 1e-15);
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
    fn radians_serialize() {
        let r = Radians(core::f64::consts::PI);
        let yaml = crate::to_string(&r).unwrap();
        assert!(yaml.contains("3.14159"));
    }
}
