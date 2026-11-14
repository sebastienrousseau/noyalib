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

#[test]
fn beyond_128_bit_decimal_spellings_use_float_fallback_on_all_paths() {
    let mut config = ParserConfig::serde_yaml_compat();
    for number in [
        "340282366920938463463374607431768211456",
        "+340282366920938463463374607431768211456",
        "-170141183460469231731687303715884105729",
        "-92233720368547784436854778446744073750808",
    ] {
        let expected = serde_json::json!({"a": number.parse::<f64>().unwrap()});
        parity(&format!("a: {number}\n"), &config, expected);
    }
    for number in [
        "340282366920938463463374607431768211455",
        "-170141183460469231731687303715884105728",
    ] {
        let source = format!("a: {number}\n");
        assert_eq!(
            from_str_with_config::<Value>(&source, &config)
                .unwrap_err()
                .kind(),
            ErrorKind::IntegerOverflow
        );
        assert_eq!(
            from_str_with_config::<serde_json::Value>(&source, &config)
                .unwrap_err()
                .kind(),
            ErrorKind::IntegerOverflow
        );
    }
    config.no_schema = true;
    parity(
        "a: -170141183460469231731687303715884105729\n",
        &config,
        serde_json::json!({"a": "-170141183460469231731687303715884105729"}),
    );
}

#[test]
fn overflowing_decimal_keys_stay_as_written() {
    // Found by fuzz_serde_yaml_compat on `88888888888888888888:`. The
    // archive reads a JSON map key as a string, so only values refuse.
    let config = ParserConfig::serde_yaml_compat();
    let mut fallback = config.clone();
    fallback.merge_key_policy = MergeKeyPolicy::AsOrdinary;
    for number in [
        "88888888888888888888",
        "-9223372036854775809",
        "340282366920938463463374607431768211455",
        "340282366920938463463374607431768211456",
    ] {
        for source in [format!("{number}: 1\n"), format!("{{{number}}}")] {
            let expected = serde_json::json!({ number: if source.starts_with('{') {
                serde_json::Value::Null
            } else {
                serde_json::json!(1)
            }});
            for cfg in [&config, &fallback] {
                let typed: serde_json::Value = from_str_with_config(&source, cfg)
                    .unwrap_or_else(|e| panic!("{source:?}: {e}"));
                assert_eq!(typed, expected, "{source:?}");
            }
            #[cfg(feature = "compat-serde-yaml")]
            {
                let shim: serde_json::Value = noyalib::compat::serde_yaml::from_str(&source)
                    .unwrap_or_else(|e| panic!("shim {source:?}: {e}"));
                assert_eq!(shim, expected, "shim {source:?}");
            }
            let value = from_str_with_config::<Value>(&source, &config);
            assert!(value.is_ok(), "Value {source:?}: {value:?}");
        }
    }
    // Distinct spellings stay distinct keys, as JSON keys are strings:
    // a float key would merge integers that round alike.
    for (source, keys) in [
        (
            "9990000003181795956718829566:\n9990000003181795456718829566: 8\n",
            [
                "9990000003181795456718829566",
                "9990000003181795956718829566",
            ],
        ),
        (
            "+9990000003181795956718829566:\n9990000003181795956718829566:\n",
            [
                "+9990000003181795956718829566",
                "9990000003181795956718829566",
            ],
        ),
    ] {
        for cfg in [&config, &fallback] {
            let typed: serde_json::Value = from_str_with_config(source, cfg).unwrap();
            let mut found: Vec<_> = typed.as_object().unwrap().keys().cloned().collect();
            found.sort();
            assert_eq!(found, keys, "{source:?}");
        }
    }
    let error = from_str_with_config::<serde_json::Value>("k: 88888888888888888888\n", &config)
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::IntegerOverflow);
}

#[test]
fn radix_overflow_refusal_matches_the_archive() {
    // Found by fuzz_serde_yaml_compat on `{m: 0xFF992547400000000}`. The
    // archive refuses a radix literal inside 128 bits but outside 64 and
    // keeps a larger one as a string; keys stay strings throughout.
    let config = ParserConfig::serde_yaml_compat();
    let mut fallback = config.clone();
    fallback.merge_key_policy = MergeKeyPolicy::AsOrdinary;
    let max128 = format!("0x{}", "F".repeat(32));
    let bin65 = format!("0b1{}", "0".repeat(64));
    for literal in [
        "0x10000000000000000",
        max128.as_str(),
        "0o2000000000000000000000",
        "-0x8000000000000001",
        "+0x10000000000000000",
        "-0o1000000000000000000001",
        bin65.as_str(),
    ] {
        let source = format!("k: {literal}\n");
        for cfg in [&config, &fallback] {
            let error = from_str_with_config::<serde_json::Value>(&source, cfg).expect_err(&source);
            assert_eq!(error.kind(), ErrorKind::IntegerOverflow, "{source:?}");
            assert_eq!(error.location().unwrap().index(), 3, "{source:?}");
            let error = from_str_with_config::<Value>(&source, cfg).expect_err(&source);
            assert_eq!(error.kind(), ErrorKind::IntegerOverflow, "{source:?}");
        }
        let keyed: serde_json::Value =
            from_str_with_config(&format!("{literal}: 1\n"), &config).unwrap();
        assert_eq!(keyed, serde_json::json!({ literal: 1 }), "{literal}");
    }
    let over128 = format!("0x1{}", "0".repeat(32));
    #[cfg(feature = "lossless-u64")]
    parity(
        "k: 0xFFFFFFFFFFFFFFFF\n",
        &config,
        serde_json::json!({ "k": u64::MAX }),
    );
    for (literal, expected) in [
        ("0o777", serde_json::json!(511)),
        (
            "-0x8000000000000000",
            serde_json::json!("-0x8000000000000000"),
        ),
        ("-0xF", serde_json::json!("-0xF")),
        (over128.as_str(), serde_json::json!(over128)),
    ] {
        parity(
            &format!("k: {literal}\n"),
            &config,
            serde_json::json!({ "k": expected }),
        );
    }
    #[cfg(feature = "lossless-u64")]
    {
        let mut default = ParserConfig::default();
        default.lossless_u64_integers = true;
        let value: serde_json::Value =
            from_str_with_config("k: 0x10000000000000000\n", &default).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"k": "0x10000000000000000"}),
            "ADR-0004 default"
        );
    }
}

#[cfg(feature = "lossless-u64")]
#[test]
fn legacy_binary_reaches_the_unsigned_range() {
    // Found by fuzz_serde_yaml_compat: a 64-digit binary literal above
    // i64::MAX stayed a string where the archive reads the u64.
    let config = ParserConfig::serde_yaml_compat();
    let literal = format!("0b{}", "1".repeat(64));
    parity(
        &format!("m: {literal}\n"),
        &config,
        serde_json::json!({ "m": u64::MAX }),
    );
    parity(
        "m: 0b1111011111111111111111111111111111111111111111111111111111111111\n",
        &config,
        serde_json::json!({ "m": 17_870_283_321_406_128_127_u64 }),
    );
    let text = noyalib::to_string(&literal).unwrap();
    let back: String = from_str_with_config(&text, &config).unwrap();
    assert_eq!(
        back, literal,
        "a binary-shaped string stays quoted: {text:?}"
    );
}
