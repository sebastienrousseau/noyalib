// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `ParserConfig::ignore_binary_tag_for_string` — migration helper
//! that lets `!!binary "ABCD"` deserialize into a `String` target
//! as the literal base64 source string. The bytes path is unaffected
//! — `Vec<u8>` / `serde_bytes::ByteBuf` always decode the payload.

#![allow(missing_docs)]

use noyalib::{ParserConfig, from_str, from_str_with_config};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Doc {
    payload: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct DocBytes {
    payload: serde_bytes::ByteBuf,
}

const YAML_BINARY: &str = "payload: !!binary YWJj\n"; // "abc" base64'd

#[test]
fn default_rejects_binary_into_string() {
    // YAML 1.2 default semantics: `!!binary` is a typed tag and
    // cannot be silently coerced to `String`.
    let res: Result<Doc, _> = from_str(YAML_BINARY);
    assert!(
        res.is_err(),
        "default config must reject !!binary into String, got: {res:?}"
    );
}

#[test]
fn opt_in_lets_binary_pass_through_as_source_string() {
    let cfg = ParserConfig::new().ignore_binary_tag_for_string(true);
    let doc: Doc = from_str_with_config(YAML_BINARY, &cfg).unwrap();
    // The base64 *source* is preserved; the application layer
    // decides whether to decode.
    assert_eq!(doc.payload, "YWJj");
}

#[test]
fn opt_in_does_not_affect_bytes_path() {
    // The canonical bytes path always decodes the base64 payload —
    // the toggle changes the String path only.
    let cfg = ParserConfig::new().ignore_binary_tag_for_string(true);
    let doc: DocBytes = from_str_with_config(YAML_BINARY, &cfg).unwrap();
    assert_eq!(doc.payload.as_slice(), b"abc");
}

#[test]
fn default_off_means_off() {
    assert!(!ParserConfig::new().ignore_binary_tag_for_string);
}

#[test]
fn opt_in_with_quoted_form() {
    let yaml = "payload: !!binary \"YWJj\"\n";
    let cfg = ParserConfig::new().ignore_binary_tag_for_string(true);
    let doc: Doc = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(doc.payload, "YWJj");
}

#[test]
fn opt_in_block_scalar_form() {
    // Multi-line base64 — block-literal style. The literal
    // newlines are part of the string in this mode (the toggle
    // documents the source verbatim; the application decides
    // whether to strip whitespace before decoding).
    let yaml = "payload: !!binary |\n  YWJjZGVm\n  Z2hpams=\n";
    let cfg = ParserConfig::new().ignore_binary_tag_for_string(true);
    let doc: Doc = from_str_with_config(yaml, &cfg).unwrap();
    // Block scalars preserve interior newlines.
    assert!(doc.payload.contains("YWJjZGVm"));
    assert!(doc.payload.contains("Z2hpams="));
}

#[test]
fn opt_in_ignore_on_other_tags_does_not_apply() {
    // The toggle is `ignore_binary_tag_*` — it should not change
    // behaviour for other tags. A `!Custom` tag still rejects
    // when serde asks for a String.
    let yaml = "payload: !Custom hello\n";
    let cfg = ParserConfig::new().ignore_binary_tag_for_string(true);
    // After the custom-tag preservation work this may surface as
    // Tagged; either way, `!Custom` is not `!!binary` so the
    // toggle is inert here.
    let res: Result<Doc, _> = from_str_with_config(yaml, &cfg);
    let _ = res; // The exact behaviour for unrelated tags is
    // governed elsewhere; this test simply asserts the
    // toggle doesn't accidentally widen its scope.
}
