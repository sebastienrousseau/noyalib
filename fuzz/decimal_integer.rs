// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact comparison for the value oracle's leading-zero resolver difference.

pub(crate) fn equivalent(source: &str, number: &serde_json::Number) -> bool {
    source
        .parse::<i64>()
        .map(serde_json::Number::from)
        .or_else(|_| source.parse::<u64>().map(serde_json::Number::from))
        .ok()
        .or_else(|| {
            source
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
        })
        .is_some_and(|parsed| super::json_numbers::equal(&parsed, number))
}

/// ADR-0004 keeps a decimal integer outside both 64-bit domains as its
/// digits; the archive rounds it to the nearest finite float.
pub(crate) fn beyond_64_bits(source: &str, number: &serde_json::Number) -> bool {
    let digits = source.strip_prefix(['+', '-']).unwrap_or(source);
    !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && source.parse::<i64>().is_err()
        && digits.parse::<u64>().is_err()
        && number.is_f64()
        && source.parse::<f64>().ok() == number.as_f64()
}

#[cfg(test)]
mod tests {
    use serde_json::Number;

    #[test]
    fn integers_beyond_64_bits_match_only_their_own_float() {
        let digits = "2".repeat(54);
        let float = |s: &str| Number::from_f64(s.parse().unwrap()).unwrap();
        assert!(super::beyond_64_bits(&digits, &float(&digits)));
        let negative = format!("-{digits}");
        assert!(super::beyond_64_bits(&negative, &float(&negative)));
        assert!(super::beyond_64_bits(
            "+18446744073709551616",
            &float("18446744073709551616")
        ));
        assert!(!super::beyond_64_bits(&digits, &float("2.3e53")));
        assert!(!super::beyond_64_bits(
            "18446744073709551615",
            &Number::from(u64::MAX)
        ));
        assert!(!super::beyond_64_bits(
            "18446744073709551615",
            &float("18446744073709551615")
        ));
        assert!(!super::beyond_64_bits("1e60", &float("1e60")));
        assert!(!super::beyond_64_bits("0x1F", &Number::from(31)));
    }

    #[test]
    fn leading_zero_integer_spelling_keeps_exact_digits() {
        assert!(super::equivalent(
            "09007199254740993",
            &Number::from(9_007_199_254_740_993_u64)
        ));
        assert!(!super::equivalent(
            "09007199254740993",
            &Number::from(9_007_199_254_740_992_u64)
        ));
        assert!(!super::equivalent(
            "-09007199254740993",
            &Number::from(-9_007_199_254_740_992_i64)
        ));
        assert!(super::equivalent(
            "018446744073709551615",
            &Number::from(u64::MAX)
        ));
        assert!(!super::equivalent(
            "018446744073709551615",
            &Number::from(u64::MAX - 1)
        ));
    }
}
