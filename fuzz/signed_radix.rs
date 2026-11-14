// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact comparison for the archive's legacy binary integer reading.

/// The profile's YAML 1.1 binary reading admits `_` separators; the
/// archive keeps those spellings as strings.
pub(crate) fn legacy_binary(source: &str, number: &serde_json::Number) -> bool {
    let unsigned = source.strip_prefix(['+', '-']).unwrap_or(source);
    let Some(digits) = unsigned.strip_prefix("0b") else {
        return false;
    };
    if !digits.contains('_') || !digits.bytes().all(|b| matches!(b, b'0' | b'1' | b'_')) {
        return false;
    }
    let Ok(magnitude) = u64::from_str_radix(&digits.replace('_', ""), 2) else {
        return false;
    };
    let value = if source.starts_with('-') {
        0_i64
            .checked_sub_unsigned(magnitude)
            .map(serde_json::Number::from)
    } else {
        Some(serde_json::Number::from(magnitude))
    };
    value.is_some_and(|value| super::json_numbers::equal(&value, number))
}

#[cfg(test)]
mod tests {
    use serde_json::Number;

    #[test]
    fn legacy_binary_spellings_match_only_their_exact_value() {
        assert!(super::legacy_binary("0b0_", &Number::from(0)));
        assert!(super::legacy_binary("-0b1_0", &Number::from(-2)));
        assert!(!super::legacy_binary("0b10", &Number::from(2)));
        assert!(!super::legacy_binary("0b1_0", &Number::from(3)));
        assert!(!super::legacy_binary("0B0", &Number::from(0)));
        assert!(!super::legacy_binary("-0B1_1", &Number::from(-3)));
        let wide = format!("0b{}_1", "1".repeat(63));
        assert!(super::legacy_binary(&wide, &Number::from(u64::MAX)));
    }
}
