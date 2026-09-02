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

// ── Corpus integrity ───────────────────────────────────────────────
// The corpus is the immutable half of the contract: zfb's evaluation
// (their #2851, verdict MIGRATE) reproduces these cases against
// their own baseline, so a silent edit here would fork the contract
// without either side noticing. Changing the corpus is a deliberate
// act: update the hash in the same commit and say why.

#[test]
fn corpus_is_byte_identical_to_the_evaluated_contract() {
    let bytes = include_bytes!("fixtures/serde_yaml_contract/corpus.json");
    let mut hasher = Sha256Lite::new();
    hasher.update(bytes);
    assert_eq!(
        hasher.hex(),
        "8316d88c943b68adb662e146981db419272d76e0713e58a7a803d07e617e703b",
        "corpus.json changed; if intentional, update this pin in the \
         same commit and record why in the CHANGELOG"
    );
}

#[test]
fn corpus_covers_every_compatibility_category() {
    // Mirrors zfb's corpus_covers_every_named_compatibility_category:
    // the 18 cases must keep spanning the full category set.
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/serde_yaml_contract/corpus.json")).unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 18, "the contract is exactly 18 cases");
    let categories: std::collections::BTreeSet<&str> = cases
        .iter()
        .filter_map(|c| c["category"].as_str())
        .collect();
    for want in [
        "alias-anchor-resource-limits",
        "anchors-aliases",
        "duplicate-keys",
        "explicit-tags",
        "malformed-input",
        "merge-keys",
        "non-finite-overflowing-numbers",
        "non-string-keys",
        "scalar-edge-cases",
        "unicode-bom-crlf-emoji",
    ] {
        assert!(categories.contains(want), "category {want} lost coverage");
    }
}

/// Minimal SHA-256 so the integrity pin needs no new dependency.
struct Sha256Lite {
    state: [u32; 8],
    buf: Vec<u8>,
    len: u64,
}

impl Sha256Lite {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: Vec::new(),
            len: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.len += data.len() as u64;
        self.buf.extend_from_slice(data);
        while self.buf.len() >= 64 {
            let block: [u8; 64] = self.buf[..64].try_into().unwrap();
            self.compress(&block);
            let _ = self.buf.drain(..64);
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (slot, c) in w.iter_mut().zip(block.chunks_exact(4)) {
            *slot = u32::from_be_bytes(c.try_into().unwrap());
        }
        // Index-based on purpose: the SHA-256 message schedule reads
        // relative offsets (i-2, i-7, i-15, i-16), which iterator
        // adapters only obscure.
        #[allow(clippy::needless_range_loop)]
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for (wi, ki) in w.iter().zip(Self::K.iter()) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(*ki)
                .wrapping_add(*wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }

    fn hex(mut self) -> String {
        let bit_len = self.len * 8;
        self.buf.push(0x80);
        while self.buf.len() % 64 != 56 {
            self.buf.push(0);
        }
        self.buf.extend_from_slice(&bit_len.to_be_bytes());
        let blocks: Vec<[u8; 64]> = self
            .buf
            .chunks_exact(64)
            .map(|c| c.try_into().unwrap())
            .collect();
        for b in &blocks {
            self.compress(b);
        }
        self.state.iter().map(|w| format!("{w:08x}")).collect()
    }
}
