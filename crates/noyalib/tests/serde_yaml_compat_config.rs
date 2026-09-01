//! Unit coverage for the serde_yaml-profile `ParserConfig` knobs and
//! the shim's error rendering — each flag exercised individually,
//! outside the bundled `serde_yaml_compat()` preset, on both loader
//! paths (the no-span fast path via a plain `Value` target, the
//! span-aware path via `Spanned`). The end-to-end 18-case contract
//! lives in `serde_yaml_contract.rs`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![cfg(feature = "compat-serde-yaml")]

use noyalib::{
    Error, ErrorKind, NonScalarKeyPolicy, ParserConfig, Spanned, Value, from_str_with_config,
};

fn v(yaml: &str, cfg: &ParserConfig) -> Value {
    from_str_with_config(yaml, cfg).unwrap()
}

// ── leading_zero_integer_strings ───────────────────────────────────

#[test]
fn leading_zero_integers_resolve_as_strings_when_enabled() {
    let mut cfg = ParserConfig::new();
    cfg.leading_zero_integer_strings = true;
    let doc = v("a: 0123\nb: -0123\nc: +07\n", &cfg);
    assert_eq!(doc["a"].as_str(), Some("0123"));
    assert_eq!(doc["b"].as_str(), Some("-0123"));
    assert_eq!(doc["c"].as_str(), Some("+07"));
}

#[test]
fn leading_zero_flag_leaves_other_numbers_alone() {
    let mut cfg = ParserConfig::new();
    cfg.leading_zero_integer_strings = true;
    let doc = v(
        "zero: 0\nplain: 123\noctal: 0o755\nhex: 0x1F\nfloat: 0.5\n",
        &cfg,
    );
    assert_eq!(doc["zero"].as_i64(), Some(0));
    assert_eq!(doc["plain"].as_i64(), Some(123));
    assert_eq!(doc["octal"].as_i64(), Some(0o755));
    assert_eq!(doc["hex"].as_i64(), Some(0x1F));
    assert_eq!(doc["float"].as_f64(), Some(0.5));
}

#[test]
fn leading_zero_integers_stay_numbers_by_default() {
    let doc = v("a: 0123\n", &ParserConfig::new());
    assert_eq!(doc["a"].as_i64(), Some(123));
}

// ── legacy_binary_numbers ──────────────────────────────────────────

#[test]
fn binary_literals_resolve_when_enabled() {
    let mut cfg = ParserConfig::new();
    cfg.legacy_binary_numbers = true;
    let doc = v("a: 0b11\nb: -0b101\nc: 0B1_0\n", &cfg);
    assert_eq!(doc["a"].as_i64(), Some(3));
    assert_eq!(doc["b"].as_i64(), Some(-5));
    assert_eq!(doc["c"].as_i64(), Some(2));
}

#[test]
fn malformed_binary_literals_stay_strings() {
    let mut cfg = ParserConfig::new();
    cfg.legacy_binary_numbers = true;
    let doc = v("empty: 0b\nbad: 0b2\nsep_only: 0b_\n", &cfg);
    assert_eq!(doc["empty"].as_str(), Some("0b"));
    assert_eq!(doc["bad"].as_str(), Some("0b2"));
    assert_eq!(doc["sep_only"].as_str(), Some("0b_"));
}

#[test]
fn binary_literals_are_strings_by_default() {
    let doc = v("a: 0b11\n", &ParserConfig::new());
    assert_eq!(doc["a"].as_str(), Some("0b11"));
}

// ── float_overflow_strings ─────────────────────────────────────────

#[test]
fn overflowing_float_literals_stay_strings_when_enabled() {
    let mut cfg = ParserConfig::new();
    cfg.float_overflow_strings = true;
    let doc = v("pos: 1e999\nneg: -1e999\n", &cfg);
    assert_eq!(doc["pos"].as_str(), Some("1e999"));
    assert_eq!(doc["neg"].as_str(), Some("-1e999"));
}

#[test]
fn explicit_infinity_spellings_keep_their_float_values() {
    let mut cfg = ParserConfig::new();
    cfg.float_overflow_strings = true;
    let doc = v("a: .inf\nb: -.Inf\nc: .nan\n", &cfg);
    assert_eq!(doc["a"].as_f64(), Some(f64::INFINITY));
    assert_eq!(doc["b"].as_f64(), Some(f64::NEG_INFINITY));
    assert!(doc["c"].as_f64().unwrap().is_nan());
}

// ── integer_overflow_errors ────────────────────────────────────────

#[test]
fn one_past_u64_max_is_refused_with_location_and_path() {
    let mut cfg = ParserConfig::new();
    cfg.integer_overflow_errors = true;
    let err = from_str_with_config::<Value>("u64_over: 18446744073709551616\n", &cfg).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::IntegerOverflow);
    let loc = err.location().expect("carries the literal's location");
    assert_eq!((loc.line(), loc.column(), loc.index()), (1, 11, 10));
    match err {
        Error::IntegerOverflow { path, .. } => assert_eq!(path.as_deref(), Some("u64_over")),
        other => panic!("expected IntegerOverflow, got {other:?}"),
    }
}

#[test]
fn overflow_path_reflects_nesting_and_sequence_indices() {
    let mut cfg = ParserConfig::new();
    cfg.integer_overflow_errors = true;
    let err = from_str_with_config::<Value>(
        "outer:\n  items:\n    - 1\n    - 99999999999999999999\n",
        &cfg,
    )
    .unwrap_err();
    match err {
        Error::IntegerOverflow { path, .. } => {
            assert_eq!(path.as_deref(), Some("outer.items.1"));
        }
        other => panic!("expected IntegerOverflow, got {other:?}"),
    }
}

#[test]
fn plus_signed_overflow_is_refused_and_negative_overflow_is_not() {
    let mut cfg = ParserConfig::new();
    cfg.integer_overflow_errors = true;
    assert!(from_str_with_config::<Value>("a: +18446744073709551616\n", &cfg).is_err());
    // Negative giants keep the historical float fallback — upstream's
    // "JSON number out of range" was a u64 story.
    let doc = v("a: -99999999999999999999\n", &cfg);
    assert!(doc["a"].as_f64().is_some());
}

#[test]
fn u64_range_values_pass_untouched_by_the_overflow_flag() {
    let mut cfg = ParserConfig::new();
    cfg.integer_overflow_errors = true;
    #[cfg(feature = "lossless-u64")]
    {
        cfg.lossless_u64_integers = true;
        let doc = v("a: 18446744073709551615\n", &cfg);
        assert_eq!(doc["a"].as_u64(), Some(u64::MAX));
    }
    let doc = v("a: 42\n", &cfg);
    assert_eq!(doc["a"].as_i64(), Some(42));
}

// ── non_scalar_key_policy ──────────────────────────────────────────

#[test]
fn sequence_keys_are_refused_under_error_policy() {
    let mut cfg = ParserConfig::new();
    cfg.non_scalar_key_policy = NonScalarKeyPolicy::Error;
    let err = from_str_with_config::<Value>("[a, b]: v\n", &cfg).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NonScalarKey);
    assert_eq!(
        err.to_string(),
        "invalid type: sequence, expected a string key"
    );
    let loc = err.location().expect("carries the key's location");
    assert_eq!((loc.line(), loc.column(), loc.index()), (1, 1, 0));
}

#[test]
fn mapping_keys_are_refused_under_error_policy() {
    let mut cfg = ParserConfig::new();
    cfg.non_scalar_key_policy = NonScalarKeyPolicy::Error;
    let err = from_str_with_config::<Value>("{k: 1}: v\n", &cfg).unwrap_err();
    assert_eq!(
        err.to_string(),
        "invalid type: mapping, expected a string key"
    );
}

#[test]
fn non_scalar_keys_stringify_by_default() {
    let doc = v("[a, b]: v\n", &ParserConfig::new());
    assert_eq!(doc["[a, b]"].as_str(), Some("v"));
}

// ── alias_jump_event_factor ────────────────────────────────────────

/// Four levels of ten-fold aliasing — a few dozen source events that
/// expand to >10⁴ nodes.
const ALIAS_BOMB: &str = "leaf: &leaf lol\n\
    a: &a [*leaf, *leaf, *leaf, *leaf, *leaf, *leaf, *leaf, *leaf, *leaf, *leaf]\n\
    b: &b [*a, *a, *a, *a, *a, *a, *a, *a, *a, *a]\n\
    c: &c [*b, *b, *b, *b, *b, *b, *b, *b, *b, *b]\n\
    d: &d [*c, *c, *c, *c, *c, *c, *c, *c, *c, *c]\n\
    root: *d\n";

#[test]
fn alias_factor_trips_on_transitive_amplification() {
    let mut cfg = ParserConfig::new();
    cfg.alias_jump_event_factor = Some(100);
    let err = from_str_with_config::<Value>(ALIAS_BOMB, &cfg).unwrap_err();
    assert!(matches!(err, Error::RepetitionLimitExceeded), "{err:?}");
    assert_eq!(err.kind(), ErrorKind::Budget);
}

#[test]
fn alias_factor_leaves_ordinary_aliasing_alone() {
    let mut cfg = ParserConfig::new();
    cfg.alias_jump_event_factor = Some(100);
    let doc = v("base: &b {x: 1}\ncopy: *b\n", &cfg);
    assert_eq!(doc["copy"]["x"].as_i64(), Some(1));
}

#[test]
fn alias_factor_disabled_by_default() {
    // The absolute budgets still bound the bomb's materialised size;
    // the point here is that None never trips the factor rule.
    assert!(ParserConfig::new().alias_jump_event_factor.is_none());
    let doc = v("base: &b {x: 1}\ncopy: *b\n", &ParserConfig::new());
    assert_eq!(doc["copy"]["x"].as_i64(), Some(1));
}

// ── the span-aware loader implements the same knobs ────────────────

#[test]
fn span_loader_refuses_non_scalar_keys_too() {
    let mut cfg = ParserConfig::new();
    cfg.non_scalar_key_policy = NonScalarKeyPolicy::Error;
    let err = from_str_with_config::<Spanned<Value>>("[a, b]: v\n", &cfg).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NonScalarKey);
}

#[test]
fn span_loader_refuses_integer_overflow_too() {
    let mut cfg = ParserConfig::new();
    cfg.integer_overflow_errors = true;
    let err = from_str_with_config::<Spanned<Value>>("u64_over: 18446744073709551616\n", &cfg)
        .unwrap_err();
    match err {
        Error::IntegerOverflow { path, .. } => assert_eq!(path.as_deref(), Some("u64_over")),
        other => panic!("expected IntegerOverflow, got {other:?}"),
    }
}

#[test]
fn span_loader_applies_the_scalar_profile_too() {
    let mut cfg = ParserConfig::new();
    cfg.leading_zero_integer_strings = true;
    cfg.legacy_binary_numbers = true;
    cfg.float_overflow_strings = true;
    let doc: Spanned<Value> =
        from_str_with_config("lz: 0123\nbin: 0b11\nof: 1e999\n", &cfg).unwrap();
    let doc = doc.into_inner();
    assert_eq!(doc["lz"].as_str(), Some("0123"));
    assert_eq!(doc["bin"].as_i64(), Some(3));
    assert_eq!(doc["of"].as_str(), Some("1e999"));
}

#[test]
fn span_loader_applies_the_alias_factor_too() {
    let mut cfg = ParserConfig::new();
    cfg.alias_jump_event_factor = Some(100);
    let err = from_str_with_config::<Spanned<Value>>(ALIAS_BOMB, &cfg).unwrap_err();
    assert!(matches!(err, Error::RepetitionLimitExceeded), "{err:?}");
}

// ── shim error rendering edges ─────────────────────────────────────

mod shim_errors {
    use noyalib::compat::serde_yaml as syml;

    #[test]
    fn eof_remap_applies_only_at_end_of_input() {
        // Mid-document parse errors keep their position untouched.
        let err = syml::from_str::<serde_json::Value>("a: [1,\nb: 2\n").unwrap_err();
        let loc = err.location().unwrap();
        assert!(loc.index() < "a: [1,\nb: 2\n".len());
    }

    #[test]
    fn flow_sequence_trailer_names_the_innermost_open_bracket() {
        // Two opens; the inner one (line 1 column 9) is the context.
        let err = syml::from_str::<serde_json::Value>("a: [1, [2").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.ends_with("while parsing a flow sequence at line 1 column 8"),
            "{msg}"
        );
    }

    #[test]
    fn bracket_scan_ignores_brackets_inside_quotes() {
        let err = syml::from_str::<serde_json::Value>("a: ['[', [2").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.ends_with("while parsing a flow sequence at line 1 column 10"),
            "{msg}"
        );
    }

    #[test]
    fn recursion_limit_reads_like_upstream() {
        let mut deep = String::new();
        for _ in 0..200 {
            deep.push('[');
        }
        let err = syml::from_str::<serde_json::Value>(&deep).unwrap_err();
        assert_eq!(err.to_string(), "recursion limit exceeded");
        assert!(err.location().is_none());
    }

    #[test]
    fn from_conversion_without_input_still_renders() {
        // The `From` path has no input, so no EOF remap and no
        // bracket trailer — but the class wordings still apply.
        let e = syml::Error::from(noyalib::Error::RepetitionLimitExceeded);
        assert_eq!(e.to_string(), "repetition limit exceeded");
        assert!(e.location().is_none());
    }

    #[test]
    fn invalid_utf8_from_slice_is_an_error_not_a_panic() {
        let err = syml::from_slice::<serde_json::Value>(&[0xFF, 0xFE]).unwrap_err();
        assert!(err.to_string().contains("invalid UTF-8"), "{err}");
    }

    #[test]
    fn multi_document_helper_wraps_errors_with_input_context() {
        let err = syml::from_str_multi::<serde_json::Value>("a: [1\n---\nb: 2\n").unwrap_err();
        assert!(err.location().is_some());
    }
}
