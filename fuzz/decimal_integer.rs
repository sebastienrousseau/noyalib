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

#[cfg(test)]
mod tests {
    use serde_json::Number;

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
