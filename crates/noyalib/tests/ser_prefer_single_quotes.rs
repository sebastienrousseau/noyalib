//! Coverage for `SerializerConfig::prefer_single_quotes` (Refs #352).
//!
//! When set and a string scalar needs quoting at all, it is written
//! single-quoted (with any embedded `'` doubled) instead of double-quoted
//! -- unless it contains a character only double-quoted style can carry
//! (a control character, a tab, or anything else needing an escape
//! sequence), in which case it still falls back to double quotes. Output
//! is unchanged when the option is left at its default (`false`).

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Mapping, SerializerConfig, Value, from_str, to_string_with_config};

fn prefer_single() -> SerializerConfig {
    SerializerConfig::new().prefer_single_quotes(true)
}

/// Serializes `s` as the value of key `k` and returns just the value's
/// rendered text (after `"k: "`), asserting it re-parses back to `s`.
fn quoted_value(s: &str, config: &SerializerConfig) -> String {
    let mut m = Mapping::new();
    let _ = m.insert("k", Value::String(s.to_owned()));
    let out = to_string_with_config(&Value::Mapping(m), config).unwrap();
    let value_text = out
        .strip_prefix("k: ")
        .unwrap_or_else(|| panic!("expected \"k: ...\", got {out:?}"))
        .to_owned();

    let back: Value = from_str(&out).unwrap();
    assert_eq!(
        back.get_path("k").and_then(Value::as_str),
        Some(s),
        "did not round-trip: {out:?}"
    );
    value_text
}

#[test]
fn colon_containing_string_prefers_single_quotes() {
    assert_eq!(
        quoted_value("Blog: a post", &prefer_single()),
        "'Blog: a post'"
    );
}

#[test]
fn digit_leading_string_prefers_single_quotes() {
    assert_eq!(quoted_value("01234", &prefer_single()), "'01234'");
}

#[test]
fn empty_string_prefers_single_quotes() {
    assert_eq!(quoted_value("", &prefer_single()), "''");
}

#[test]
fn reserved_word_prefers_single_quotes() {
    assert_eq!(quoted_value("true", &prefer_single()), "'true'");
}

#[test]
fn comment_marker_string_prefers_single_quotes() {
    assert_eq!(quoted_value("a #b", &prefer_single()), "'a #b'");
}

#[test]
fn dash_only_string_prefers_single_quotes() {
    assert_eq!(quoted_value("-", &prefer_single()), "'-'");
}

#[test]
fn plain_safe_string_needs_no_quoting_either_way() {
    // "it's" never needed quoting in the first place (see Refs #345's
    // `plain_scalars_stay_plain` coverage) -- the option only changes
    // *which* quote style is used when quoting is already required.
    assert_eq!(quoted_value("it's", &prefer_single()), "it's");
}

#[test]
fn embedded_single_quote_is_doubled() {
    assert_eq!(
        quoted_value("Blog: it's", &prefer_single()),
        "'Blog: it''s'"
    );
}

#[test]
fn tab_forces_double_quotes_even_when_preferring_single() {
    // A tab is a control character single-quoted style has no escape
    // for, so it still falls back to double quotes.
    assert_eq!(quoted_value("\tNote", &prefer_single()), "\"\\tNote\"");
}

#[test]
fn default_still_double_quotes() {
    assert_eq!(
        quoted_value("Blog: a post", &SerializerConfig::default()),
        "\"Blog: a post\""
    );
}
