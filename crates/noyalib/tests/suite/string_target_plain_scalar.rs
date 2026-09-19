// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Refs #344: `ParserConfig::plain_scalar_strings` opts a `String` (or
//! one-character `char`) target into accepting a plain non-string
//! scalar (a number, bool, null, or empty value) and receiving its
//! literal source text, matching `serde_yaml` 0.9.34's measured
//! behaviour, instead of refusing with a "non-string scalar" type
//! mismatch. Off by default — v0.0.28's contract is unchanged; every
//! other test file in this crate was restored to its original text
//! and passes unmodified.
//!
//! Covers both deserializers: the streaming path
//! (`from_str_with_config` + `plain_scalar_strings(true)`) and the
//! `Value`-AST path, additionally forced ineligible for the streaming
//! fast path via `ignore_binary_tag_for_string(true)` (see `de.rs`'s
//! `stream_eligible`, which `plain_scalar_strings` deliberately does
//! not affect).

#![allow(missing_docs)]

use noyalib::{ParserConfig, Value, from_str, from_str_with_config};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Holder {
    v: String,
}

#[derive(Debug, Deserialize)]
struct Smtp {
    password: String,
}

#[derive(Debug, Deserialize)]
struct Cfg {
    smtp: Smtp,
}

#[derive(Debug, Deserialize)]
struct OptHolder {
    v: Option<String>,
}

#[derive(Debug, Deserialize)]
struct U16Holder {
    v: u16,
}

#[derive(Debug, Deserialize)]
struct CharHolder {
    v: char,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum NumberOrString {
    Number(u64),
    Text(String),
}

#[derive(Debug, Deserialize)]
struct UntaggedHolder {
    v: NumberOrString,
}

fn on_config() -> ParserConfig {
    ParserConfig::new().plain_scalar_strings(true)
}

fn ast_on_config() -> ParserConfig {
    ParserConfig::new()
        .plain_scalar_strings(true)
        .ignore_binary_tag_for_string(true)
}

// ── Original bug report, option on ───────────────────────────────────

#[test]
fn smtp_password_accepts_plain_numeric_scalar() {
    let cfg: Cfg = from_str_with_config("smtp:\n  password: 123456\n", &on_config()).unwrap();
    assert_eq!(cfg.smtp.password, "123456");
}

// ── Mirror: option off is byte-for-byte the v0.0.28 refusal ─────────

#[test]
fn streaming_default_still_rejects_plain_numeric_scalar() {
    assert!(from_str::<Holder>("v: 123456\n").is_err());
}

#[test]
fn ast_default_still_rejects_plain_numeric_scalar() {
    let cfg = ParserConfig::new().ignore_binary_tag_for_string(true);
    let res: Result<Holder, _> = from_str_with_config("v: 123456\n", &cfg);
    assert!(res.is_err());
}

// ── Streaming path, option on: every row of the reference table ─────

#[test]
fn streaming_plain_integer_into_string() {
    let h: Holder = from_str_with_config("v: 123456\n", &on_config()).unwrap();
    assert_eq!(h.v, "123456");
}

#[test]
fn streaming_plain_float_into_string() {
    let h: Holder = from_str_with_config("v: 1.0\n", &on_config()).unwrap();
    assert_eq!(h.v, "1.0");
}

#[test]
fn streaming_plain_bool_into_string() {
    let h: Holder = from_str_with_config("v: true\n", &on_config()).unwrap();
    assert_eq!(h.v, "true");
}

#[test]
fn streaming_plain_tilde_into_string() {
    let h: Holder = from_str_with_config("v: ~\n", &on_config()).unwrap();
    assert_eq!(h.v, "~");
}

#[test]
fn streaming_plain_null_word_into_string() {
    let h: Holder = from_str_with_config("v: null\n", &on_config()).unwrap();
    assert_eq!(h.v, "null");
}

#[test]
fn streaming_plain_empty_into_string() {
    let h: Holder = from_str_with_config("v:\n", &on_config()).unwrap();
    assert_eq!(h.v, "");
}

#[test]
fn streaming_plain_hex_into_string() {
    let h: Holder = from_str_with_config("v: 0x1F\n", &on_config()).unwrap();
    assert_eq!(h.v, "0x1F");
}

// ── Streaming path, option on: `char` gets the same literal-text
//    treatment, constrained to exactly one character ────────────────

#[test]
fn streaming_single_digit_scalar_into_char() {
    let h: CharHolder = from_str_with_config("v: 5\n", &on_config()).unwrap();
    assert_eq!(h.v, '5');
}

#[test]
fn streaming_multi_digit_scalar_into_char_still_errors() {
    let res: Result<CharHolder, _> = from_str_with_config("v: 12\n", &on_config());
    assert!(res.is_err());
}

// ── AST path, option on (forced ineligible for the streaming
//    fast path) ──────────────────────────────────────────────────────

#[test]
fn ast_plain_integer_into_string() {
    let h: Holder = from_str_with_config("v: 123456\n", &ast_on_config()).unwrap();
    assert_eq!(h.v, "123456");
}

#[test]
fn ast_plain_float_into_string() {
    let h: Holder = from_str_with_config("v: 1.0\n", &ast_on_config()).unwrap();
    assert_eq!(h.v, "1.0");
}

#[test]
fn ast_plain_bool_into_string() {
    let h: Holder = from_str_with_config("v: true\n", &ast_on_config()).unwrap();
    assert_eq!(h.v, "true");
}

#[test]
fn ast_plain_empty_into_string() {
    let h: Holder = from_str_with_config("v:\n", &ast_on_config()).unwrap();
    assert_eq!(h.v, "");
}

#[test]
fn ast_single_digit_scalar_into_char() {
    let h: CharHolder = from_str_with_config("v: 5\n", &ast_on_config()).unwrap();
    assert_eq!(h.v, '5');
}

// ── Guards (default, option off): behaviour that must not change ────

#[test]
fn option_string_from_tilde_is_none() {
    let h: OptHolder = from_str("v: ~\n").unwrap();
    assert_eq!(h.v, None);
}

#[test]
fn option_string_from_empty_is_none() {
    let h: OptHolder = from_str("v:\n").unwrap();
    assert_eq!(h.v, None);
}

#[test]
fn quoted_numeric_string_into_u16_still_errors() {
    assert!(from_str::<U16Holder>("v: \"12\"\n").is_err());
    // A plain (unquoted) numeric scalar is unaffected by this fix and
    // still deserializes into `u16` normally.
    let h: U16Holder = from_str("v: 12\n").unwrap();
    assert_eq!(h.v, 12);
}

#[test]
fn value_target_still_resolves_plain_scalar_to_number() {
    let v: Value = from_str("password: 123456\n").unwrap();
    let password = v.as_mapping().unwrap().get("password").unwrap();
    assert!(matches!(password, Value::Number(_)));
}

#[test]
fn untagged_enum_still_prefers_number_arm_for_numeric_scalar() {
    let h: UntaggedHolder = from_str("v: 123456\n").unwrap();
    assert_eq!(h.v, NumberOrString::Number(123_456));
}

// ── Vec<String> element-wise threading, option on ────────────────────

#[test]
fn vec_string_from_integer_sequence_with_option_on() {
    let v: Vec<String> = from_str_with_config("[2024, 2025]", &on_config()).unwrap();
    assert_eq!(v, vec!["2024".to_string(), "2025".to_string()]);
}
