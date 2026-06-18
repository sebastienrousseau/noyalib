//! YAML number type (`Number`).

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::str::FromStr;

/// Represents a YAML number.
#[derive(Debug, Clone, Copy)]
pub enum Number {
    /// A signed integer.
    Integer(i64),
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
            Number::Integer(n) => Some(*n),
            Number::Float(_) => None,
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
            Number::Integer(n) if *n >= 0 => Some(*n as u64),
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
            Number::Integer(n) => *n as f64,
            Number::Float(n) => *n,
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
        matches!(self, Number::Integer(_))
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
        matches!(self, Number::Float(_))
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
        matches!(self, Number::Integer(_))
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
        matches!(self, Number::Integer(n) if *n >= 0)
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
            Number::Float(n) => n.is_nan(),
            Number::Integer(_) => false,
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
            Number::Float(n) => n.is_infinite(),
            Number::Integer(_) => false,
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
            Number::Float(n) => n.is_finite(),
            Number::Integer(_) => true,
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::Integer(n) => write!(f, "{n}"),
            Number::Float(n) => write!(f, "{n}"),
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

#[cfg(feature = "std")]
impl std::error::Error for ParseNumberError {}

impl FromStr for Number {
    type Err = ParseNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Handle special float values
        match s {
            ".nan" | ".NaN" | ".NAN" => return Ok(Number::Float(f64::NAN)),
            ".inf" | ".Inf" | ".INF" => return Ok(Number::Float(f64::INFINITY)),
            "+.inf" | "+.Inf" | "+.INF" => return Ok(Number::Float(f64::INFINITY)),
            "-.inf" | "-.Inf" | "-.INF" => return Ok(Number::Float(f64::NEG_INFINITY)),
            _ => {}
        }

        // Try parsing as integer first
        if let Ok(n) = s.parse::<i64>() {
            return Ok(Number::Integer(n));
        }

        // Handle hex (0x), octal (0o), and binary (0b) integers
        if s.len() > 2 {
            let (prefix, rest) = s.split_at(2);
            match prefix {
                "0x" | "0X" => {
                    if let Ok(n) = i64::from_str_radix(rest, 16) {
                        return Ok(Number::Integer(n));
                    }
                }
                "0o" | "0O" => {
                    if let Ok(n) = i64::from_str_radix(rest, 8) {
                        return Ok(Number::Integer(n));
                    }
                }
                "0b" | "0B" => {
                    if let Ok(n) = i64::from_str_radix(rest, 2) {
                        return Ok(Number::Integer(n));
                    }
                }
                _ => {}
            }
        }

        // Try parsing as float
        if let Ok(n) = s.parse::<f64>() {
            return Ok(Number::Float(n));
        }

        Err(ParseNumberError { _private: () })
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::Integer(a), Number::Integer(b)) => a == b,
            (Number::Float(a), Number::Float(b)) => {
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
            Number::Integer(n) => {
                0u8.hash(state);
                n.hash(state);
            }
            Number::Float(n) => {
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

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Number::Integer(a), Number::Integer(b)) => a.cmp(b),
            (Number::Float(a), Number::Float(b)) => {
                // Handle NaN: treat all NaN as equal and greater than any non-NaN
                match (a.is_nan(), b.is_nan()) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    (false, false) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                }
            }
            (Number::Integer(a), Number::Float(b)) => {
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
            (Number::Float(a), Number::Integer(b)) => {
                // Delegate to the Integer-Float case and invert.
                match Number::Integer(*b).cmp(&Number::Float(*a)) {
                    Ordering::Less => Ordering::Greater,
                    Ordering::Greater => Ordering::Less,
                    Ordering::Equal => Ordering::Equal,
                }
            }
        }
    }
}

// ============================================================================
// Number From impls
// ============================================================================

impl From<i8> for Number {
    fn from(v: i8) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<i16> for Number {
    fn from(v: i16) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<i32> for Number {
    fn from(v: i32) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<i64> for Number {
    fn from(v: i64) -> Self {
        Number::Integer(v)
    }
}

impl From<isize> for Number {
    fn from(v: isize) -> Self {
        Number::Integer(v as i64)
    }
}

impl From<u8> for Number {
    fn from(v: u8) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<u16> for Number {
    fn from(v: u16) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<u32> for Number {
    fn from(v: u32) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<u64> for Number {
    fn from(v: u64) -> Self {
        if v <= i64::MAX as u64 {
            Number::Integer(v as i64)
        } else {
            Number::Float(v as f64)
        }
    }
}

impl From<usize> for Number {
    fn from(v: usize) -> Self {
        Number::from(v as u64)
    }
}

impl From<f32> for Number {
    fn from(v: f32) -> Self {
        Number::Float(f64::from(v))
    }
}

impl From<f64> for Number {
    fn from(v: f64) -> Self {
        Number::Float(v)
    }
}
