// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Private binary64 operations for alloc-only builds.

const SIGN: u64 = 1 << 63;
const FRACTION: u64 = (1 << 52) - 1;

pub(crate) fn fract(value: f64) -> f64 {
    let bits = value.to_bits();
    let exponent = (bits >> 52) & 0x7ff;
    let integer = if exponent < 1023 {
        f64::from_bits(bits & SIGN)
    } else if exponent < 1075 {
        let fractional_bits = (1_u64 << (1075 - exponent)) - 1;
        f64::from_bits(bits & !fractional_bits)
    } else {
        value
    };
    value - integer
}

pub(crate) fn mul_add(a: f64, b: f64, c: f64) -> f64 {
    if a.is_nan() || b.is_nan() || c.is_nan() || a.is_infinite() || b.is_infinite() {
        let product = a * b;
        return product + c;
    }
    if c.is_infinite() {
        return c;
    }
    if a == 0.0 || b == 0.0 {
        let product = a * b;
        return product + c;
    }

    let (am, ae) = decompose(a);
    let (bm, be) = decompose(b);
    let product = u128::from(am) * u128::from(bm);
    let product_exponent = ae + be;
    let negative = a.is_sign_negative() ^ b.is_sign_negative();
    if c == 0.0 {
        return round_binary64(product, product_exponent, negative);
    }

    let (cm, ce) = decompose(c);
    let product_top = product_exponent + 127 - product.leading_zeros() as i32;
    let addend_top = ce + 63 - cm.leading_zeros() as i32;
    // Both leading bits fit at position 126, leaving a carry bit. A product
    // has at most 106 significant bits, so close cancellation stays exact.
    let exponent = product_top.max(addend_top) - 126;
    let product = align(product, product_exponent - exponent);
    let addend = align(u128::from(cm), ce - exponent);
    if negative == c.is_sign_negative() {
        round_binary64(product + addend, exponent, negative)
    } else if product > addend {
        round_binary64(product - addend, exponent, negative)
    } else if product < addend {
        round_binary64(addend - product, exponent, !negative)
    } else {
        0.0
    }
}

// Finite nonzero values are exactly mantissa * 2^exponent, including
// subnormals, whose implicit leading bit is absent.
fn decompose(value: f64) -> (u64, i32) {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if exponent == 0 {
        (bits & FRACTION, -1074)
    } else {
        ((bits & FRACTION) | (1 << 52), exponent - 1075)
    }
}

fn align(value: u128, shift: i32) -> u128 {
    if shift >= 0 {
        value << shift as u32
    } else if shift <= -128 {
        u128::from(value != 0)
    } else {
        let distance = shift.unsigned_abs();
        // Preserve whether any discarded bit was nonzero. The larger term
        // has trailing zeros here, so addition/subtraction retains this sticky bit.
        let sticky = u128::from(value & ((1_u128 << distance) - 1) != 0);
        (value >> distance) | sticky
    }
}

fn round_binary64(significand: u128, exponent: i32, negative: bool) -> f64 {
    let sign = u64::from(negative) << 63;
    if significand == 0 {
        return f64::from_bits(sign);
    }
    let top = exponent + 127 - significand.leading_zeros() as i32;
    if top > 1023 {
        return f64::from_bits(sign | (0x7ff << 52));
    }
    // Normal values retain 53 bits; subnormals round on the fixed 2^-1074
    // grid. Quantizing once avoids double rounding at the normal boundary.
    let mut quantum = (top - 52).max(-1074);
    let mut mantissa = round_integer(significand, quantum - exponent);
    if mantissa == 1_u128 << 53 {
        mantissa >>= 1;
        quantum += 1;
    }
    let magnitude = if quantum > 971 {
        0x7ff << 52
    } else if mantissa < 1_u128 << 52 {
        mantissa as u64
    } else {
        ((quantum + 1075) as u64) << 52 | (mantissa as u64 & FRACTION)
    };
    f64::from_bits(sign | magnitude)
}

fn round_integer(value: u128, shift: i32) -> u128 {
    if shift <= 0 {
        return value << shift.unsigned_abs();
    }
    if shift > 128 {
        return 0;
    }
    if shift == 128 {
        return u128::from(value > 1_u128 << 127);
    }
    let shift = shift as u32;
    let integer = value >> shift;
    let remainder = value & ((1_u128 << shift) - 1);
    let halfway = 1_u128 << (shift - 1);
    integer + u128::from(remainder > halfway || (remainder == halfway && integer & 1 != 0))
}

#[cfg(test)]
mod tests {
    use super::{fract, mul_add};

    fn same_bits(actual: f64, expected: f64) {
        if expected.is_nan() {
            assert!(actual.is_nan(), "actual={actual:?}");
        } else {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "{actual:?} != {expected:?}"
            );
        }
    }

    #[test]
    fn fractional_parts_match_the_existing_backends() {
        for value in [
            0.0,
            -0.0,
            f64::from_bits(1),
            -f64::from_bits(1),
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            0.5,
            -0.5,
            1.0,
            -1.0,
            1.25,
            -1.25,
            1.0 + f64::EPSILON,
            4_503_599_627_370_495.5,
            4_503_599_627_370_496.0,
            f64::MAX,
            f64::MIN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            same_bits(fract(value), value.fract());
            same_bits(fract(value), value - libm::trunc(value));
        }
    }

    #[test]
    fn fused_rounding_and_cancellation_match_the_existing_backends() {
        for (a, b, c) in [
            (1.0 + f64::EPSILON, 1.0 - f64::EPSILON, -1.0),
            (1_125_899_906_842_627.0, 60.0, 0.1),
            (f64::MAX, 2.0, -f64::MAX),
            (f64::from_bits(1), 0.5, f64::from_bits(1)),
            (-f64::from_bits(1), 0.5, -0.0),
            (f64::from_bits(16), f64::from_bits(16), -f64::MIN_POSITIVE),
            (-0.992, -0.992, -0.992),
        ] {
            same_bits(mul_add(a, b, c), a.mul_add(b, c));
            same_bits(mul_add(a, b, c), libm::fma(a, b, c));
        }
    }

    #[test]
    fn special_values_and_exponent_boundaries_match() {
        let values = [
            0.0,
            -0.0,
            f64::from_bits(1),
            -f64::from_bits(1),
            f64::from_bits(0x000f_ffff_ffff_ffff),
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            0.5,
            -1.0,
            f64::MAX,
            f64::MIN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        for a in values {
            for b in values {
                for c in values {
                    same_bits(mul_add(a, b, c), a.mul_add(b, c));
                    same_bits(mul_add(a, b, c), libm::fma(a, b, c));
                }
            }
        }
    }

    #[test]
    fn halfway_neighbors_round_to_even_across_exponents() {
        let power_of_two = |exponent: i32| {
            if exponent < -1022 {
                f64::from_bits(1 << (exponent + 1074))
            } else {
                f64::from_bits(((exponent + 1023) as u64) << 52)
            }
        };
        for exponent in [-1021, -1020, -1000, -100, -1, 0, 1, 52, 53, 100, 1000, 1023] {
            let a = power_of_two(exponent);
            let half = power_of_two(exponent - 53).to_bits();
            for bits in [a.to_bits(), a.to_bits() + 1] {
                for a in [f64::from_bits(bits), -f64::from_bits(bits)] {
                    for bits in [half - 1, half, half + 1] {
                        for c in [f64::from_bits(bits), -f64::from_bits(bits)] {
                            same_bits(mul_add(a, 1.0, c), a.mul_add(1.0, c));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn cancellation_residuals_match_the_existing_backends() {
        let mut state = 0xbb67_ae85_84ca_a73b_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            f64::from_bits(state)
        };
        let samples = if cfg!(miri) { 128 } else { 200_000 };
        for _ in 0..samples {
            let (a, b) = (next(), next());
            let rounded_product = a * b;
            for c in [
                -rounded_product,
                -f64::from_bits(rounded_product.to_bits().wrapping_add(1)),
            ] {
                same_bits(mul_add(a, b, c), a.mul_add(b, c));
                same_bits(mul_add(a, b, c), libm::fma(a, b, c));
            }
        }
    }

    #[test]
    fn deterministic_bit_patterns_match_the_existing_backends() {
        let mut state = 0x6a09_e667_f3bc_c909_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            f64::from_bits(state)
        };
        let samples = if cfg!(miri) { 128 } else { 1_000_000 };
        for _ in 0..samples {
            let (a, b, c) = (next(), next(), next());
            same_bits(fract(a), a.fract());
            same_bits(mul_add(a, b, c), a.mul_add(b, c));
            same_bits(mul_add(a, b, c), libm::fma(a, b, c));
        }
    }
}
