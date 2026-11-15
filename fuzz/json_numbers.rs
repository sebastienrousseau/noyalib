// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Compare JSON numbers without rounding distinct integers together.

pub(crate) fn equal(a: &serde_json::Number, b: &serde_json::Number) -> bool {
    let integer = |number: &serde_json::Number| {
        number
            .as_i64()
            .map(i128::from)
            .or_else(|| number.as_u64().map(i128::from))
    };
    let matches_integer = |number: &serde_json::Number, expected: i128| {
        number
            .as_f64()
            .is_some_and(|value| value.fract() == 0.0 && value as i128 == expected)
    };
    match (integer(a), integer(b)) {
        (Some(a), Some(b)) => a == b,
        (Some(a), None) => matches_integer(b, a),
        (None, Some(b)) => matches_integer(a, b),
        (None, None) => a.as_f64() == b.as_f64(),
    }
}

#[cfg(test)]
mod tests {
    use super::equal;
    use serde_json::Number;

    #[test]
    fn distinct_large_integers_remain_distinct() {
        for (a, b) in [
            (9_007_199_254_740_992_u64, 9_007_199_254_740_993_u64),
            (u64::MAX - 1, u64::MAX),
        ] {
            assert!(!equal(&Number::from(a), &Number::from(b)), "{a} != {b}");
        }
        assert!(!equal(&Number::from(i64::MIN), &Number::from(i64::MIN + 1)));
    }

    #[test]
    fn integer_float_comparison_requires_exact_value_agreement() {
        for (integer, float, expected) in [
            (Number::from(450), 450.0, true),
            (Number::from(0), -0.0, true),
            (Number::from(1), 1.5, false),
            (
                Number::from(9_007_199_254_740_992_u64),
                9_007_199_254_740_992.0,
                true,
            ),
            (
                Number::from(9_007_199_254_740_993_u64),
                9_007_199_254_740_992.0,
                false,
            ),
            (Number::from(i64::MIN), -9_223_372_036_854_775_808.0, true),
            (Number::from(i64::MAX), 9_223_372_036_854_775_808.0, false),
            (Number::from(u64::MAX), 18_446_744_073_709_551_616.0, false),
            (Number::from(1), 1e100, false),
        ] {
            let float = Number::from_f64(float).unwrap();
            assert_eq!(equal(&integer, &float), expected, "{integer} vs {float}");
            assert_eq!(equal(&float, &integer), expected, "{float} vs {integer}");
        }
        assert!(equal(&Number::from(1), &Number::from(1_u64)));
        assert!(equal(
            &Number::from_f64(1.5).unwrap(),
            &Number::from_f64(1.5).unwrap()
        ));
        assert!(!equal(
            &Number::from_f64(1.5).unwrap(),
            &Number::from_f64(1.6).unwrap()
        ));
    }
}
