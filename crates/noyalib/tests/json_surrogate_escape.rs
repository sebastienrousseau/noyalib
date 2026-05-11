// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! JSON-style UTF-16 surrogate-pair escape pairing in double-quoted
//! YAML scalars.
//!
//! YAML 1.2's escape rules for double-quoted scalars inherit JSON's
//! treatment of `\uXXXX` escapes — including the surrogate-pair
//! encoding of supplementary-plane code points. A high surrogate
//! (`\uD800`–`\uDBFF`) immediately followed by a low surrogate
//! (`\uDC00`–`\uDFFF`) combines into a single character via the
//! UTF-16 algorithm:
//!
//! ```text
//! cp = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
//! ```
//!
//! Lone or reversed surrogates remain rejected — they are invalid
//! Unicode and must not silently become replacement characters.

#![allow(missing_docs)]

use noyalib::{Value, from_str};

// ── Paired surrogates produce supplementary-plane characters ────────

#[test]
fn surrogate_pair_encodes_musical_g_clef() {
    // U+1D11E (𝄞 musical G clef) = high D834 + low DD1E.
    let yaml = "music: \"\\uD834\\uDD1E\"";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["music"].as_str(), Some("𝄞"));
    // UTF-8 encoding of U+1D11E is F0 9D 84 9E.
    assert_eq!(
        v["music"].as_str().unwrap().as_bytes(),
        &[0xF0, 0x9D, 0x84, 0x9E]
    );
}

#[test]
fn surrogate_pair_encodes_emoji_face() {
    // U+1F600 (😀 grinning face) = high D83D + low DE00.
    let yaml = "emoji: \"\\uD83D\\uDE00\"";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["emoji"].as_str(), Some("😀"));
}

#[test]
fn multiple_surrogate_pairs_in_one_string() {
    // Three supplementary-plane characters in sequence.
    let yaml = "music: \"\\uD834\\uDD1E\\uD83C\\uDFB5\\uD83C\\uDFB6\"";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["music"].as_str(), Some("𝄞🎵🎶"));
}

#[test]
fn surrogate_pair_mixed_with_bmp_escapes() {
    // BMP escape, then a surrogate pair, then more BMP.
    let yaml = "s: \"\\u00E9\\uD834\\uDD1E\\u00E9\"";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v["s"].as_str(), Some("é𝄞é"));
}

// ── Lone / reversed surrogates remain rejected ──────────────────────

#[test]
fn lone_high_surrogate_errors() {
    let yaml = "bad: \"\\uD834\"";
    let err = from_str::<Value>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("D834") || msg.contains("surrogate"),
        "expected surrogate-rejection error, got: {msg}"
    );
}

#[test]
fn lone_high_surrogate_followed_by_non_escape_errors() {
    let yaml = "bad: \"\\uD834z\"";
    let err = from_str::<Value>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("D834") || msg.contains("surrogate"),
        "expected surrogate-rejection error, got: {msg}"
    );
}

#[test]
fn high_followed_by_non_surrogate_escape_errors() {
    // `\uD834A` — high surrogate then a regular BMP escape.
    let yaml = "bad: \"\\uD834\\u0041\"";
    let err = from_str::<Value>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("low surrogate") || msg.contains("surrogate"),
        "expected low-surrogate-required error, got: {msg}"
    );
}

#[test]
fn reversed_surrogate_pair_errors() {
    // Low surrogate appearing first — invalid.
    let yaml = "bad: \"\\uDC00\\uD800\"";
    let err = from_str::<Value>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("DC00") || msg.contains("surrogate"),
        "expected reversed-pair rejection, got: {msg}"
    );
}

#[test]
fn lone_low_surrogate_errors() {
    let yaml = "bad: \"\\uDC00\"";
    let err = from_str::<Value>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("DC00") || msg.contains("surrogate"),
        "expected lone-low-surrogate rejection, got: {msg}"
    );
}

#[test]
fn high_surrogate_with_truncated_low_surrogate_errors() {
    // High surrogate, then `\u` started but only 3 hex digits provided
    // before the closing quote — the pair-validation loop must
    // surface the error path.
    let yaml = "bad: \"\\uD834\\uDD1\"";
    let err = from_str::<Value>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("low surrogate") || msg.contains("hex"),
        "expected low-surrogate truncation error, got: {msg}"
    );
}

#[test]
fn high_surrogate_with_non_hex_after_backslash_u_errors() {
    // `\uD834\uDDXY` — the second `\u` parses but its third hex digit
    // is non-hex.
    let yaml = "bad: \"\\uD834\\uDDXY\"";
    let err = from_str::<Value>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("hex") || msg.contains("low surrogate"),
        "expected hex-digit / low-surrogate error, got: {msg}"
    );
}

#[test]
fn surrogate_high_at_eof_errors() {
    // High surrogate is the last thing in the document — no chance
    // for a low surrogate. Hits the lone-surrogate fall-through
    // branch (peek != `\\`).
    let yaml = "x: \"\\uD834";
    let err = from_str::<Value>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("D834") || msg.contains("surrogate") || msg.contains("quote"),
        "expected surrogate-or-string error, got: {msg}"
    );
}

// ── Round-trip preserves supplementary-plane characters ─────────────

#[test]
fn surrogate_pair_round_trips() {
    let yaml = "music: \"\\uD834\\uDD1E\"";
    let v: Value = from_str(yaml).unwrap();
    let serialized = noyalib::to_string(&v).unwrap();
    let v2: Value = from_str(&serialized).unwrap();
    assert_eq!(v, v2);
    assert_eq!(v2["music"].as_str(), Some("𝄞"));
}
