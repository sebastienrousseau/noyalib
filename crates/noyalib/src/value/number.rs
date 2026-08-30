//! YAML number type (`Number`).

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::str::FromStr;

/// Represents a YAML number.
///
/// Marked `#[non_exhaustive]` so future numeric variants can be
/// added without breaking downstream `match` arms. Callers must
/// include a `_ => { … }` wildcard when pattern-matching. Adding
/// `Unsigned(u64)` behind `feature = "lossless-u64"` is the first
/// use of this contract; see
/// [ADR-0004](https://github.com/sebastienrousseau/noyalib/blob/main/doc/adr/0004-lossless-u64-integers.md).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Number {
    /// A signed integer.
    Integer(i64),
    /// An unsigned integer that cannot be represented by `i64`.
    #[cfg(feature = "lossless-u64")]
    #[cfg_attr(docsrs, doc(cfg(feature = "lossless-u64")))]
    Unsigned(u64),
    /// A floating-point number.
    Float(f64),
}

impl Number {
    /// Returns the number as an `i64` if it is an integer.
    ///
    /// Floats return `None` even when their value happens to be a
    /// whole number; the type tag is part of the test.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert_eq!(Number::Integer(42).as_i64(), Some(42));
    /// assert_eq!(Number::Float(1.0).as_i64(), None);
    /// ```
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(n) => Some(*n),
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(n) => i64::try_from(*n).ok(),
            Self::Float(_) => None,
        }
    }

    /// Returns the number as a `u64` if it is a non-negative integer.
    ///
    /// Negative integers and floats return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert_eq!(Number::Integer(42).as_u64(), Some(42));
    /// assert_eq!(Number::Integer(-1).as_u64(), None);
    /// assert_eq!(Number::Float(1.0).as_u64(), None);
    /// ```
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Integer(n) if *n >= 0 => Some(*n as u64),
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the number as an `f64`.
    ///
    /// Always succeeds — integers are widened to `f64` (with the
    /// usual `i64 → f64` precision loss for magnitudes above
    /// 2^53), floats pass through unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert_eq!(Number::Integer(42).as_f64(), 42.0);
    /// assert_eq!(Number::Float(0.5).as_f64(), 0.5);
    /// ```
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Integer(n) => *n as f64,
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(n) => *n as f64,
            Self::Float(n) => *n,
        }
    }

    /// Returns `true` if the number is an integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Integer(42).is_integer());
    /// assert!(!Number::Float(1.0).is_integer());
    /// ```
    #[must_use]
    pub fn is_integer(&self) -> bool {
        match self {
            Self::Integer(_) => true,
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(_) => true,
            Self::Float(_) => false,
        }
    }

    /// Returns `true` if the number is a float.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Float(1.0).is_float());
    /// assert!(!Number::Integer(42).is_float());
    /// ```
    #[must_use]
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    /// Returns `true` if the number can be represented as an `i64`.
    ///
    /// True for all integer values, false for floats.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Integer(42).is_i64());
    /// assert!(!Number::Float(42.0).is_i64());
    /// ```
    #[must_use]
    pub fn is_i64(&self) -> bool {
        match self {
            Self::Integer(_) => true,
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(n) => i64::try_from(*n).is_ok(),
            Self::Float(_) => false,
        }
    }

    /// Returns `true` if the number can be represented as a `u64`.
    ///
    /// True for non-negative integers, false otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Integer(42).is_u64());
    /// assert!(!Number::Integer(-1).is_u64());
    /// assert!(!Number::Float(1.0).is_u64());
    /// ```
    #[must_use]
    pub fn is_u64(&self) -> bool {
        match self {
            Self::Integer(n) => *n >= 0,
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(_) => true,
            Self::Float(_) => false,
        }
    }

    /// Returns `true` if the number can be represented as an `f64`.
    ///
    /// Always true — both integers and floats convert to `f64`
    /// (with the usual precision caveats for very large
    /// integers).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Integer(42).is_f64());
    /// assert!(Number::Float(1.0).is_f64());
    /// ```
    #[must_use]
    pub fn is_f64(&self) -> bool {
        true
    }

    /// Returns `true` if the number is `NaN` (Not a Number).
    ///
    /// Integers are never `NaN` — only floats with the IEEE 754
    /// NaN bit pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Float(f64::NAN).is_nan());
    /// assert!(!Number::Float(0.0).is_nan());
    /// assert!(!Number::Integer(0).is_nan());
    /// ```
    #[must_use]
    pub fn is_nan(&self) -> bool {
        match self {
            Self::Float(n) => n.is_nan(),
            Self::Integer(_) => false,
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(_) => false,
        }
    }

    /// Returns `true` if the number is positive or negative infinity.
    ///
    /// Integers are always finite — only `Number::Float(±inf)`
    /// returns true.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Float(f64::INFINITY).is_infinite());
    /// assert!(Number::Float(f64::NEG_INFINITY).is_infinite());
    /// assert!(!Number::Integer(i64::MAX).is_infinite());
    /// ```
    #[must_use]
    pub fn is_infinite(&self) -> bool {
        match self {
            Self::Float(n) => n.is_infinite(),
            Self::Integer(_) => false,
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(_) => false,
        }
    }

    /// Returns `true` if the number is neither infinite nor `NaN`.
    ///
    /// Integers are always finite; floats are finite when neither
    /// `±∞` nor `NaN`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::Number;
    /// assert!(Number::Integer(0).is_finite());
    /// assert!(Number::Float(0.5).is_finite());
    /// assert!(!Number::Float(f64::NAN).is_finite());
    /// assert!(!Number::Float(f64::INFINITY).is_finite());
    /// ```
    #[must_use]
    pub fn is_finite(&self) -> bool {
        match self {
            Self::Float(n) => n.is_finite(),
            Self::Integer(_) => true,
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(_) => true,
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(n) => write!(f, "{n}"),
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(n) => write!(f, "{n}"),
            Self::Float(n) => write_float(f, *n),
        }
    }
}

/// Formats a float the same way the YAML emitter does: `.nan`, `.inf`,
/// `-.inf` for the specials, and a float-preserving decimal form (`4.0`,
/// never `4`) otherwise.
///
/// This is the single source of truth for float formatting, shared by
/// [`Number`]'s `Display` impl and the serializer's value writer
/// (`crate::ser::write_value`) so the two never drift apart again — see
/// issue #348, where `Display` used `core::fmt`'s default `f64`
/// formatting (which drops the trailing `.0` from whole floats) while the
/// serializer already emitted `4.0`.
///
/// Generic over `W: fmt::Write` so it works both against a
/// `fmt::Formatter` (`Display`) and a `String` output buffer
/// (serializer).
pub(crate) fn write_float<W: fmt::Write>(output: &mut W, n: f64) -> fmt::Result {
    if n.is_nan() {
        output.write_str(".nan")
    } else if n.is_infinite() {
        if n > 0.0 {
            output.write_str(".inf")
        } else {
            output.write_str("-.inf")
        }
    } else {
        #[cfg(feature = "fast-float")]
        {
            let mut buf = ryu::Buffer::new();
            output.write_str(buf.format(n))
        }
        #[cfg(not(feature = "fast-float"))]
        {
            // `{:?}` preserves float-ness for whole numbers (`1.0` not
            // `1`) so the value round-trips back as `Number::Float`.
            write!(output, "{n:?}")
        }
    }
}

/// Error returned when parsing a number from a string fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseNumberError {
    _private: (),
}

impl fmt::Display for ParseNumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid number")
    }
}

impl core::error::Error for ParseNumberError {}

impl FromStr for Number {
    type Err = ParseNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Handle special float values
        match s {
            ".nan" | ".NaN" | ".NAN" => return Ok(Self::Float(f64::NAN)),
            ".inf" | ".Inf" | ".INF" => return Ok(Self::Float(f64::INFINITY)),
            "+.inf" | "+.Inf" | "+.INF" => return Ok(Self::Float(f64::INFINITY)),
            "-.inf" | "-.Inf" | "-.INF" => return Ok(Self::Float(f64::NEG_INFINITY)),
            _ => {}
        }

        // Try parsing as integer first
        if let Ok(n) = s.parse::<i64>() {
            return Ok(Self::Integer(n));
        }
        #[cfg(feature = "lossless-u64")]
        if let Ok(n) = s.parse::<u64>() {
            return Ok(Self::Unsigned(n));
        }

        // Handle hex (0x), octal (0o), and binary (0b) integers
        if s.len() > 2 {
            let (prefix, rest) = s.split_at(2);
            match prefix {
                "0x" | "0X" => {
                    if let Ok(n) = i64::from_str_radix(rest, 16) {
                        return Ok(Self::Integer(n));
                    }
                    #[cfg(feature = "lossless-u64")]
                    if let Ok(n) = u64::from_str_radix(rest, 16) {
                        return Ok(Self::Unsigned(n));
                    }
                }
                "0o" | "0O" => {
                    if let Ok(n) = i64::from_str_radix(rest, 8) {
                        return Ok(Self::Integer(n));
                    }
                    #[cfg(feature = "lossless-u64")]
                    if let Ok(n) = u64::from_str_radix(rest, 8) {
                        return Ok(Self::Unsigned(n));
                    }
                }
                "0b" | "0B" => {
                    if let Ok(n) = i64::from_str_radix(rest, 2) {
                        return Ok(Self::Integer(n));
                    }
                    #[cfg(feature = "lossless-u64")]
                    if let Ok(n) = u64::from_str_radix(rest, 2) {
                        return Ok(Self::Unsigned(n));
                    }
                }
                _ => {}
            }
        }

        // Try parsing as float
        if let Ok(n) = s.parse::<f64>() {
            return Ok(Self::Float(n));
        }

        Err(ParseNumberError { _private: () })
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a == b,
            #[cfg(feature = "lossless-u64")]
            (Self::Unsigned(a), Self::Unsigned(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => {
                // Treat NaN == NaN to satisfy the Eq contract (reflexivity)
                (a.is_nan() && b.is_nan()) || a == b
            }
            _ => false,
        }
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Integer(n) => {
                0u8.hash(state);
                n.hash(state);
            }
            #[cfg(feature = "lossless-u64")]
            Self::Unsigned(n) => {
                2u8.hash(state);
                n.hash(state);
            }
            Self::Float(n) => {
                1u8.hash(state);
                // Eq/Hash contract: equal values must hash equal. Two
                // edge cases break naive `to_bits()` hashing:
                //   - `+0.0 == -0.0` is true under IEEE 754 (and our
                //     PartialEq), but `to_bits()` gives 0x0000… vs
                //     0x8000…. Normalise zeros to a single bit pattern.
                //   - PartialEq treats NaN == NaN as true (so `Eq` is
                //     reflexive), but distinct NaN payloads have
                //     distinct bits. Hash a fixed sentinel for NaN.
                let bits = if n.is_nan() {
                    0x7FF8_0000_0000_0001
                } else if *n == 0.0 {
                    0
                } else {
                    n.to_bits()
                };
                bits.hash(state);
            }
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Total ordering for [`Number`].
///
/// Same-kind comparisons (`Integer` vs `Integer`, `Unsigned` vs `Unsigned`,
/// `Float` vs `Float`) use exact arithmetic. Cross-kind comparisons between
/// integers and floats widen the integer side to [`f64`], matching the
/// existing `Integer` ↔ `Float` arms. Values with magnitude above 2^53 may
/// compare [`Ordering::Equal`] to nearby floats because of IEEE rounding; for
/// exact ordering, compare within the same kind.
impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a.cmp(b),
            #[cfg(feature = "lossless-u64")]
            (Self::Unsigned(a), Self::Unsigned(b)) => a.cmp(b),
            #[cfg(feature = "lossless-u64")]
            (Self::Integer(a), Self::Unsigned(b)) => {
                if *a < 0 {
                    Ordering::Less
                } else {
                    (*a as u64).cmp(b)
                }
            }
            #[cfg(feature = "lossless-u64")]
            (Self::Unsigned(a), Self::Integer(b)) => {
                if *b < 0 {
                    Ordering::Greater
                } else {
                    a.cmp(&(*b as u64))
                }
            }
            (Self::Float(a), Self::Float(b)) => {
                // Handle NaN: treat all NaN as equal and greater than any non-NaN
                match (a.is_nan(), b.is_nan()) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    (false, false) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                }
            }
            (Self::Integer(a), Self::Float(b)) => {
                if b.is_nan() {
                    Ordering::Less
                } else if *a > (1_i64 << 53) || *a < -(1_i64 << 53) {
                    // Large integer outside f64 safe range — compare via string
                    // to avoid precision loss from i64→f64 cast.
                    let a_f = *a as f64;
                    if (a_f as i64) == *a {
                        a_f.partial_cmp(b).unwrap_or(Ordering::Equal)
                    } else {
                        // Precision lost — compare integer magnitude vs float
                        if *a > 0 {
                            if *b < (1_i64 << 53) as f64 {
                                Ordering::Greater
                            } else {
                                (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal)
                            }
                        } else if *b > -(1_i64 << 53) as f64 {
                            Ordering::Less
                        } else {
                            (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal)
                        }
                    }
                } else {
                    (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal)
                }
            }
            (Self::Float(a), Self::Integer(b)) => {
                // Delegate to the Integer-Float case and invert.
                match Self::Integer(*b).cmp(&Self::Float(*a)) {
                    Ordering::Less => Ordering::Greater,
                    Ordering::Greater => Ordering::Less,
                    Ordering::Equal => Ordering::Equal,
                }
            }
            #[cfg(feature = "lossless-u64")]
            (Self::Unsigned(a), Self::Float(b)) => {
                if b.is_nan() {
                    Ordering::Less
                } else {
                    (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal)
                }
            }
            #[cfg(feature = "lossless-u64")]
            (Self::Float(a), Self::Unsigned(b)) => match Self::Unsigned(*b).cmp(&Self::Float(*a)) {
                Ordering::Less => Ordering::Greater,
                Ordering::Greater => Ordering::Less,
                Ordering::Equal => Ordering::Equal,
            },
        }
    }
}

// ============================================================================
// Number From impls
// ============================================================================

impl From<i8> for Number {
    fn from(v: i8) -> Self {
        Self::Integer(i64::from(v))
    }
}

impl From<i16> for Number {
    fn from(v: i16) -> Self {
        Self::Integer(i64::from(v))
    }
}

impl From<i32> for Number {
    fn from(v: i32) -> Self {
        Self::Integer(i64::from(v))
    }
}

impl From<i64> for Number {
    fn from(v: i64) -> Self {
        Self::Integer(v)
    }
}

impl From<isize> for Number {
    fn from(v: isize) -> Self {
        Self::Integer(v as i64)
    }
}

impl From<u8> for Number {
    fn from(v: u8) -> Self {
        Self::Integer(i64::from(v))
    }
}

impl From<u16> for Number {
    fn from(v: u16) -> Self {
        Self::Integer(i64::from(v))
    }
}

impl From<u32> for Number {
    fn from(v: u32) -> Self {
        Self::Integer(i64::from(v))
    }
}

impl From<u64> for Number {
    fn from(v: u64) -> Self {
        if let Ok(v) = i64::try_from(v) {
            Self::Integer(v)
        } else {
            #[cfg(feature = "lossless-u64")]
            {
                Self::Unsigned(v)
            }
            #[cfg(not(feature = "lossless-u64"))]
            {
                Self::Float(v as f64)
            }
        }
    }
}

impl From<usize> for Number {
    fn from(v: usize) -> Self {
        Self::from(v as u64)
    }
}

impl From<f32> for Number {
    fn from(v: f32) -> Self {
        Self::Float(f64::from(v))
    }
}

impl From<f64> for Number {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}
