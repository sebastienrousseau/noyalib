// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Scalar profile flags agree across typed and materialized loaders.

use noyalib::{ErrorKind, MergeKeyPolicy, ParserConfig, Value, from_str_with_config};

fn parity(source: &str, config: &ParserConfig, expected: serde_json::Value) {
    let value: Value = from_str_with_config(source, config).unwrap();
    assert_eq!(serde_json::to_value(value).unwrap(), expected);
    let typed: serde_json::Value = from_str_with_config(source, config).unwrap();
    assert_eq!(typed, expected);
    let mut fallback = config.clone();
    fallback.merge_key_policy = MergeKeyPolicy::AsOrdinary;
    let typed: serde_json::Value = from_str_with_config(source, &fallback).unwrap();
    assert_eq!(typed, expected);
}

#[test]
fn individual_scalar_profile_flags_apply_to_typed_targets() {
    let mut config = ParserConfig::default();
    config.leading_zero_integer_strings = true;
    parity("a: 0123\n", &config, serde_json::json!({"a": "0123"}));
    config = ParserConfig::default();
    config.legacy_binary_numbers = true;
    parity("a: 0b11\n", &config, serde_json::json!({"a": 3}));
    config = ParserConfig::default();
    config.float_overflow_strings = true;
    parity("a: 1e999\n", &config, serde_json::json!({"a": "1e999"}));
}

#[test]
fn decimal_overflow_refusal_agrees_for_both_signs() {
    let mut config = ParserConfig::default();
    config.integer_overflow_errors = true;
    for number in [
        "18446744073709551616",
        "+18446744073709551616",
        "-9223372036854775809",
        "-9223372036854778508",
    ] {
        let source = format!("a: {number}\n");
        let error = from_str_with_config::<Value>(&source, &config).expect_err("integer overflow");
        assert_eq!(error.kind(), ErrorKind::IntegerOverflow);
        assert_eq!(error.location().unwrap().index(), 3);
        let error = from_str_with_config::<serde_json::Value>(&source, &config)
            .expect_err("typed overflow");
        assert_eq!(error.kind(), ErrorKind::IntegerOverflow);
        assert_eq!(error.location().unwrap().index(), 3);
    }
}

#[test]
fn quoted_and_schema_disabled_integer_spellings_remain_strings() {
    let mut config = ParserConfig::default();
    config.integer_overflow_errors = true;
    parity(
        "a: '-9223372036854778508'\n",
        &config,
        serde_json::json!({"a": "-9223372036854778508"}),
    );
    config.no_schema = true;
    parity(
        "a: -9223372036854778508\n",
        &config,
        serde_json::json!({"a": "-9223372036854778508"}),
    );
    parity(
        "a: 18446744073709551616\n",
        &config,
        serde_json::json!({"a": "18446744073709551616"}),
    );
    config.legacy_binary_numbers = true;
    parity("a: 0b11\n", &config, serde_json::json!({"a": "0b11"}));
}

#[test]
fn negative_underflow_refusal_preserves_path_and_location() {
    let mut config = ParserConfig::default();
    config.integer_overflow_errors = true;
    let error = from_str_with_config::<Value>("outer:\n  a: -9223372036854778508\n", &config)
        .expect_err("signed overflow");
    assert_eq!(error.kind(), ErrorKind::IntegerOverflow);
    assert_eq!(error.location().unwrap().index(), 12);
    match error {
        noyalib::Error::IntegerOverflow { path, .. } => {
            assert_eq!(path.as_deref(), Some("outer.a"));
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn direct_streaming_refuses_materialized_scalar_profiles() {
    let mut config = ParserConfig::default();
    config.leading_zero_integer_strings = true;
    let mut stream = noyalib::StreamingDeserializer::with_config("a: 0123\n", &config);
    assert!(<serde_json::Value as serde::Deserialize>::deserialize(&mut stream).is_err());
}

#[test]
fn compatibility_leading_zero_overflow_stays_a_string() {
    let config = ParserConfig::serde_yaml_compat();
    for number in ["018446744073709551616", "-09223372036854775809"] {
        parity(
            &format!("a: {number}\n"),
            &config,
            serde_json::json!({"a": number}),
        );
    }
}
