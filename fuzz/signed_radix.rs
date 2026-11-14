// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact comparison for the archive's signed radix integer reading.

/// serde_yaml's 1.1-flavoured resolver reads a signed radix integer
/// (`-0xF`, `+0o17`); YAML 1.2's core hex and octal forms carry no sign,
/// so the compatibility profile keeps the string. Match the exact value.
pub(crate) fn equal(source: &str, number: &serde_json::Number) -> bool {
    let (negative, unsigned) = match source.as_bytes().first() {
        Some(b'-') => (true, &source[1..]),
        Some(b'+') => (false, &source[1..]),
        _ => return false,
    };
    let (radix, digits) = match unsigned.get(..2) {
        Some("0x") => (16, &unsigned[2..]),
        Some("0o") => (8, &unsigned[2..]),
        _ => return false,
    };
    if digits.starts_with(['+', '-']) {
        return false;
    }
    let Ok(magnitude) = i128::from_str_radix(digits, radix) else {
        return false;
    };
    let value = if negative { -magnitude } else { magnitude };
    i64::try_from(value)
        .map(serde_json::Number::from)
        .or_else(|_| u64::try_from(value).map(serde_json::Number::from))
        .is_ok_and(|parsed| super::json_numbers::equal(&parsed, number))
}

/// The profile's YAML 1.1 binary reading admits `_` separators and an
/// upper-case `0B` prefix; the archive keeps those spellings as strings.
pub(crate) fn legacy_binary(source: &str, number: &serde_json::Number) -> bool {
    let unsigned = source.strip_prefix(['+', '-']).unwrap_or(source);
    let (upper, digits) = match unsigned.get(..2) {
        Some("0b") => (false, &unsigned[2..]),
        Some("0B") => (true, &unsigned[2..]),
        _ => return false,
    };
    if !(upper || digits.contains('_')) || !digits.bytes().all(|b| matches!(b, b'0' | b'1' | b'_'))
    {
        return false;
    }
    let Ok(magnitude) = i64::from_str_radix(&digits.replace('_', ""), 2) else {
        return false;
    };
    let value = if source.starts_with('-') {
        -magnitude
    } else {
        magnitude
    };
    super::json_numbers::equal(&serde_json::Number::from(value), number)
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
        assert!(super::legacy_binary("0B0", &Number::from(0)));
        assert!(super::legacy_binary("-0B1_1", &Number::from(-3)));
    }

    #[test]
    fn signed_radix_spellings_match_only_their_exact_value() {
        assert!(super::equal("-0xF", &Number::from(-15)));
        assert!(super::equal("+0o17", &Number::from(15)));
        assert!(super::equal("-0x8000000000000000", &Number::from(i64::MIN)));
        assert!(!super::equal("-0xF", &Number::from(15)));
        assert!(!super::equal("0xF", &Number::from(15)));
        assert!(!super::equal("-0b1", &Number::from(-1)));
        assert!(!super::equal("-0x", &Number::from(0)));
        assert!(!super::equal("-0x-5", &Number::from(5)));
    }
}
