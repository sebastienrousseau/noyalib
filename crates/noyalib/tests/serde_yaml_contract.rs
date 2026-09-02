//! The `serde_yaml` 0.9 behavioural contract — 18 cases, verbatim.
//!
//! The corpus (`tests/fixtures/serde_yaml_contract/corpus.json`) is
//! the evaluation harness another project built to decide whether
//! noyalib could replace `serde_yaml` 0.9 for them
//! (<https://github.com/Takazudo/zudo-front-builder/issues/2787> —
//! noyalib 0.0.28 diverged on 11 of the 18 and was rejected). The
//! expectations below are the *live* output of `serde_yaml
//! 0.9.34+deprecated` on every case — captured from the real crate,
//! not transcribed — covering the JSON value produced, the error
//! `Display` text, and the `location()` line/column/index pins.
//!
//! The shim path (`noyalib::compat::serde_yaml`) must reproduce all
//! of it: values, error locations, and error wording. This is what
//! "drop-in replacement" means once behaviour counts.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![cfg(all(feature = "compat-serde-yaml", feature = "lossless-u64"))]

use noyalib::compat::serde_yaml as syml;

/// The corpus case named `name`.
fn corpus_yaml(name: &str) -> String {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/serde_yaml_contract/corpus.json"))
            .expect("corpus fixture parses");
    corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("no corpus case named {name}"))["yaml"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Assert the shim produces exactly the JSON `serde_yaml` produced.
#[track_caller]
fn expect_ok(name: &str, baseline_json: &str) {
    let yaml = corpus_yaml(name);
    let got: serde_json::Value = syml::from_str(&yaml)
        .unwrap_or_else(|e| panic!("{name}: serde_yaml parsed this, the shim refused: {e}"));
    let want: serde_json::Value = serde_json::from_str(baseline_json).unwrap();
    assert_eq!(got, want, "{name}");
}

/// Assert the shim fails exactly as `serde_yaml` failed: same
/// `Display` text, same `location()` (or same absence of one).
#[track_caller]
fn expect_err(name: &str, baseline_display: &str, baseline_loc: Option<(usize, usize, usize)>) {
    let yaml = corpus_yaml(name);
    let err = syml::from_str::<serde_json::Value>(&yaml).expect_err(&format!(
        "{name}: serde_yaml refused this, the shim accepted it"
    ));
    assert_eq!(err.to_string(), baseline_display, "{name}: display");
    let loc = err.location().map(|l| (l.line(), l.column(), l.index()));
    assert_eq!(loc, baseline_loc, "{name}: location");
}

// ── The 11 cases noyalib 0.0.28 diverged on ────────────────────────

#[test]
fn merge_key_is_an_ordinary_json_key() {
    // serde_yaml never implemented the merge: `<<` stays a literal
    // entry whose alias value resolves.
    expect_ok(
        "merge-key-is-an-ordinary-json-key",
        r#"{"defaults":{"draft":false,"title":"Default"},"post":{"<<":{"draft":false,"title":"Default"},"title":"Override"}}"#,
    );
}

#[test]
fn non_string_composite_key_is_refused() {
    expect_err(
        "non-string-composite-key",
        "invalid type: sequence, expected a string key",
        Some((1, 1, 0)),
    );
}

#[test]
fn octals_sexagesimals_and_numbers() {
    // `0123` is a string (libyaml resolved neither the 1.1 octal nor
    // the 1.2 decimal reading); `0b11` is the 1.1 binary integer 3;
    // `0o123` and `0x10` resolve; `1:20` stays a string; `1e3` is a
    // float.
    expect_ok(
        "octals-sexagesimals-and-numbers",
        r#"{"binary":3,"exp":1000.0,"float":1.2,"hex":16,"octal_new":83,"octal_old":"0123","sexagesimal":"1:20"}"#,
    );
}

#[test]
fn non_finite_and_overflowing_numbers() {
    // Non-finite values normalise to JSON null; a literal float
    // overflow (`1e999`) stays the string it was written as.
    expect_ok(
        "non-finite-and-overflowing-numbers",
        r#"{"inf":null,"nan":null,"neg_inf":null,"overflow":"1e999"}"#,
    );
}

#[test]
fn integer_boundaries_keep_precision() {
    expect_ok(
        "integer-boundaries",
        r#"{"i64_min":-9223372036854775808,"u64_max":18446744073709551615}"#,
    );
}

#[test]
fn integer_overflow_is_refused() {
    expect_err(
        "integer-overflow",
        "u64_over: JSON number out of range at line 1 column 11",
        Some((1, 11, 10)),
    );
}

#[test]
fn alias_anchor_repetition_limit() {
    expect_err(
        "alias-anchor-repetition-limit",
        "repetition limit exceeded",
        None,
    );
}

#[test]
fn malformed_unicode_location() {
    expect_err(
        "malformed-unicode-location",
        "did not find expected node content at line 1 column 8, while parsing a flow node",
        Some((1, 8, 16)),
    );
}

#[test]
fn malformed_flow_sequence_at_eof() {
    // libyaml reports end-of-input as the line after the last one,
    // column 1, and names the opening bracket in the trailer.
    expect_err(
        "malformed-flow-sequence-at-eof",
        "did not find expected ',' or ']' at line 2 column 1, while parsing a flow sequence at line 1 column 8",
        Some((2, 1, 12)),
    );
}

#[test]
fn malformed_indentation() {
    expect_err(
        "malformed-indentation",
        "mapping values are not allowed in this context at line 2 column 9",
        Some((2, 9, 18)),
    );
}

#[test]
fn custom_explicit_tag_is_refused() {
    // Exact upstream parity since v0.0.30: the location anchors at
    // the tag (`1:8:7`), because a node's span includes its
    // properties. This was the final partial in the 18-case
    // contract — Takazudo/zudo-front-builder#2755 names this exact
    // pin as its re-evaluation trigger.
    expect_err(
        "custom-explicit-tag",
        "thing: invalid type: enum, expected any valid JSON value at line 1 column 8",
        Some((1, 8, 7)),
    );
}

// ── The 7 cases that already matched at 0.0.28 ─────────────────────

#[test]
fn anchors_and_aliases() {
    expect_ok(
        "anchors-and-aliases",
        r#"{"base":{"labels":["yaml","serde"],"name":"zfb"},"copy":{"labels":["yaml","serde"],"name":"zfb"}}"#,
    );
}

#[test]
fn non_string_scalar_keys() {
    expect_ok(
        "non-string-scalar-keys",
        r#"{"1":"one","null":"nil","true":"yes"}"#,
    );
}

#[test]
fn yaml_11_boolean_spellings() {
    // The famous middle ground: `y`/`yes`/`n`/`no`/`on`/`off` stay
    // strings, `true`/`FALSE` resolve.
    expect_ok(
        "yaml-11-boolean-spellings",
        r#"{"false_value":false,"n_value":"n","no_value":"no","off_value":"off","on_value":"on","true_value":true,"y_value":"y","yes_value":"yes"}"#,
    );
}

#[test]
fn null_and_date_scalars() {
    expect_ok(
        "null-and-date-scalars",
        r#"{"date":"2024-01-02","datetime":"2024-01-02T03:04:05Z","empty":null,"null_lower":null,"null_upper":null,"tilde":null}"#,
    );
}

#[test]
fn unicode_crlf_and_emoji() {
    expect_ok(
        "unicode-crlf-and-emoji",
        r#"{"items":["🍣"],"title":"日本語 😀"}"#,
    );
}

#[test]
fn built_in_explicit_tags() {
    expect_ok(
        "built-in-explicit-tags",
        r#"{"as_int":123,"as_string":"123"}"#,
    );
}

#[test]
fn duplicate_map_keys_last_wins() {
    expect_ok("duplicate-map-keys-last-wins", r#"{"a":2}"#);
}

// ── Ports of zfb's protected assertions ────────────────────────────
// Takazudo/zudo-front-builder's evaluation harness protects a set of
// location-convention pins beyond the 18-case corpus (their
// crates/zfb-md-wasm/tests/api.rs and error_messages.rs). The
// primitives those assertions reduce to are pinned here so a noyalib
// change can never silently break their arithmetic.

#[test]
fn eof_reports_one_line_past_the_flow_sequence() {
    // zfb: "serde_yaml reports the interruption one line past the
    // flow sequence" — their frontmatter layer then adds +1 for the
    // opening `---`. The primitive: EOF inside a flow sequence lands
    // at (last line + 1, column 1).
    let err = syml::from_str::<syml::Value>("title: [oops\n").expect_err("unclosed");
    assert_eq!(
        err.to_string(),
        "did not find expected ',' or ']' at line 2 column 1, while parsing a flow sequence at line 1 column 8"
    );
    let loc = err.location().map(|l| (l.line(), l.column(), l.index()));
    assert_eq!(loc, Some((2, 1, 13)));

    // Multi-line body: still one past the last line, column 1.
    let err = syml::from_str::<syml::Value>("title: [unclosed, broken\nother: ok\n")
        .expect_err("unclosed");
    let loc = err.location().map(|l| (l.line(), l.column(), l.index()));
    assert_eq!(loc, Some((3, 1, 35)));
}

#[test]
fn columns_count_characters_and_index_counts_bytes() {
    // zfb's md-wasm layer converts noyalib columns to UTF-16 columns
    // against the original source; that conversion is only correct
    // while columns count characters and `index()` counts bytes.
    // `é` is one character and two bytes: same column, shifted index.
    let multibyte = syml::from_str::<syml::Value>("t: \"é\" x\n").expect_err("trailing");
    let ascii = syml::from_str::<syml::Value>("t: \"e\" x\n").expect_err("trailing");
    let m = multibyte
        .location()
        .map(|l| (l.line(), l.column(), l.index()));
    let a = ascii.location().map(|l| (l.line(), l.column(), l.index()));
    assert_eq!(m, Some((1, 8, 8)), "column is character-based");
    assert_eq!(a, Some((1, 8, 7)));
}
