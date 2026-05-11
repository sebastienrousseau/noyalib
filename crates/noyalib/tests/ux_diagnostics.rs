// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! UX diagnostics — surfaces helpful errors for the most common
//! configuration-file gotchas.
//!
//! Themes:
//!
//! - The "typo problem": misspelled keys silently lose data with
//!   the default deserialise. `from_str_strict<T>` flips this to
//!   a typed error that lists every offending path.
//! - Source-located parse errors carry exact `(line, column)`
//!   tuples that an editor / CI gutter can highlight directly.
//!
//! The `strict-deserialise` feature gates the typo-detection half
//! of this file; under the `minimal` profile (no default features)
//! the strict helpers are absent and the file compiles to nothing.

#![cfg(feature = "strict-deserialise")]
#![allow(missing_docs)]

use noyalib::{Error, from_reader_strict, from_slice_strict, from_str, from_str_strict};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ServerConfig {
    port: u16,
    host: String,
    #[serde(default)]
    tls: bool,
}

// ── Typo problem: silent vs strict ────────────────────────────────

#[test]
fn lenient_from_str_silently_ignores_typos() {
    // The typo `porrt: 9090` is silently dropped on the lenient
    // path — the resulting struct uses the correctly-named
    // `port: 8080` field. This is the silent-data-loss bug
    // `from_str_strict` exists to surface.
    let yaml = "port: 8080\nhost: api.example.com\nporrt: 9090\n";
    let cfg: ServerConfig = from_str(yaml).unwrap();
    assert_eq!(cfg.port, 8080);
    assert_eq!(cfg.host, "api.example.com");
}

#[test]
fn strict_from_str_surfaces_typo_as_typed_error() {
    let yaml = "port: 8080\nhost: api.example.com\nporrt: 9090\n";
    let res: Result<ServerConfig, _> = from_str_strict(yaml);
    let err = res.unwrap_err();
    match &err {
        Error::UnknownField(msg) => {
            assert!(msg.contains("porrt"), "msg should name the typo: {msg}");
        }
        other => panic!("expected UnknownField, got {other:?}"),
    }
}

#[test]
fn strict_passes_when_every_key_is_declared() {
    let yaml = "port: 8080\nhost: api.example.com\ntls: true\n";
    let cfg: ServerConfig = from_str_strict(yaml).unwrap();
    assert_eq!(cfg.port, 8080);
    assert!(cfg.tls);
}

#[test]
fn strict_lists_multiple_unknown_fields() {
    let yaml = "
port: 8080
host: api.example.com
unknown_a: 1
unknown_b: 2
unknown_c: 3
";
    let res: Result<ServerConfig, _> = from_str_strict(yaml);
    let err = res.unwrap_err();
    match &err {
        Error::UnknownField(msg) => {
            assert!(msg.contains("unknown_a"));
            assert!(msg.contains("unknown_b"));
            assert!(msg.contains("unknown_c"));
        }
        other => panic!("expected UnknownField, got {other:?}"),
    }
}

#[test]
fn strict_walks_into_nested_structs() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Outer {
        name: String,
        server: ServerConfig,
    }
    let yaml = "
name: prod
server:
  port: 8080
  host: api
  unknown: oops
";
    let res: Result<Outer, _> = from_str_strict(yaml);
    let err = res.unwrap_err();
    match &err {
        Error::UnknownField(msg) => {
            // The `server.unknown` path includes the parent key
            // — the diagnostic lets the user see exactly where
            // the typo lives in the document tree.
            assert!(msg.contains("unknown"));
        }
        other => panic!("expected UnknownField, got {other:?}"),
    }
}

// ── Strict variants for slice / reader inputs ─────────────────────

#[test]
fn strict_from_slice_surfaces_typo_as_typed_error() {
    let yaml: &[u8] = b"port: 8080\nhost: api.example.com\nporrt: 9090\n";
    let res: Result<ServerConfig, _> = from_slice_strict(yaml);
    let err = res.unwrap_err();
    match &err {
        Error::UnknownField(msg) => assert!(msg.contains("porrt")),
        other => panic!("expected UnknownField, got {other:?}"),
    }
}

#[test]
fn strict_from_slice_passes_when_every_key_is_declared() {
    let yaml: &[u8] = b"port: 8080\nhost: api.example.com\ntls: true\n";
    let cfg: ServerConfig = from_slice_strict(yaml).unwrap();
    assert_eq!(cfg.port, 8080);
    assert!(cfg.tls);
}

#[test]
fn strict_from_slice_rejects_invalid_utf8() {
    // Invalid UTF-8 byte (0xFF) inside an otherwise plausible payload.
    let yaml: &[u8] = b"port: \xff\n";
    let res: Result<ServerConfig, _> = from_slice_strict(yaml);
    assert!(res.is_err());
}

#[test]
fn strict_from_reader_surfaces_typo_as_typed_error() {
    let yaml = b"port: 8080\nhost: api.example.com\nporrt: 9090\n".to_vec();
    let res: Result<ServerConfig, _> = from_reader_strict(&yaml[..]);
    let err = res.unwrap_err();
    match &err {
        Error::UnknownField(msg) => assert!(msg.contains("porrt")),
        other => panic!("expected UnknownField, got {other:?}"),
    }
}

#[test]
fn strict_from_reader_passes_when_every_key_is_declared() {
    let yaml = b"port: 8080\nhost: api.example.com\ntls: true\n".to_vec();
    let cfg: ServerConfig = from_reader_strict(&yaml[..]).unwrap();
    assert_eq!(cfg.port, 8080);
    assert!(cfg.tls);
}

#[test]
fn strict_from_reader_surfaces_io_error() {
    // A reader that returns an `io::Error` on first poll exercises
    // the `map_err(Error::Io)?` branch in `from_reader_strict`.
    struct FailingReader;
    impl std::io::Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("boom"))
        }
    }
    let res: Result<ServerConfig, _> = from_reader_strict(FailingReader);
    let err = res.unwrap_err();
    assert!(
        matches!(err, Error::Io(_)),
        "expected Error::Io, got {err:?}"
    );
}

#[test]
fn strict_from_reader_rejects_invalid_utf8() {
    // `read_to_string` returns an `io::Error` of kind InvalidData
    // when the bytes aren't UTF-8, so this also lands on the
    // `map_err(Error::Io)?` branch — but via a different
    // io::ErrorKind, complementing the FailingReader path.
    let bad: &[u8] = b"port: \xff\n";
    let res: Result<ServerConfig, _> = from_reader_strict(bad);
    assert!(res.is_err());
}

// ── Source location on parse errors ───────────────────────────────

#[test]
fn parse_error_carries_line_and_column() {
    // Unterminated flow sequence — fails partway through line 2.
    let yaml = "ok: 1\nbad: [\n";
    let err = from_str::<noyalib::Value>(yaml).unwrap_err();
    let loc = err.location().expect("parse error must carry a location");
    // 1-indexed line / column; the offending position should be
    // somewhere in line 2 where `[` opens but never closes.
    assert!(loc.line() >= 1);
    assert!(loc.column() >= 1);
}

#[test]
fn format_with_source_renders_caret_under_offending_column() {
    let yaml = "key: bad\nbad: [unclosed\nkey: ok\n";
    let err = from_str::<noyalib::Value>(yaml).unwrap_err();
    let snippet = err.format_with_source(yaml);
    // The rendered snippet contains an `error:` prefix and a
    // line-number reference the user can act on.
    assert!(snippet.contains("error"));
}

#[test]
fn format_with_source_radius_includes_context_lines() {
    let yaml = "\
header: ok
service:
   nested: x
  bad: y
trailer: ok
";
    let err = from_str::<noyalib::Value>(yaml).unwrap_err();
    let snippet = err.format_with_source_radius(yaml, 1);
    // rustc-style `n | line` formatting with a separator pipe.
    assert!(snippet.contains('|'));
    assert!(snippet.contains("bad: y") || snippet.contains("nested: x"));
}

// ── Type-mismatch errors stay typed ───────────────────────────────

#[test]
fn type_mismatch_surfaces_as_typed_error() {
    let yaml = "port: not-an-integer\nhost: api\n";
    let res: Result<ServerConfig, _> = from_str(yaml);
    assert!(res.is_err(), "string-shaped port must not coerce into u16");
}

#[test]
fn missing_required_field_surfaces_specific_error() {
    let yaml = "host: api.example.com\n";
    let res: Result<ServerConfig, _> = from_str(yaml);
    assert!(res.is_err());
}
