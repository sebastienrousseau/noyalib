// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `!!binary` round-trip tests against the `serde_bytes` ecosystem.
//!
//! The contract Phase 1.2 establishes:
//!
//! - Any byte target (`Vec<u8>`/`&[u8]` annotated with
//!   `#[serde(with = "serde_bytes")]`, plus `serde_bytes::ByteBuf`
//!   and `serde_bytes::Bytes` directly) round-trips through YAML
//!   as a `!!binary` tagged scalar carrying the RFC 4648 base64
//!   encoding.
//! - The reverse direction (`!!binary` in input YAML →
//!   `Vec<u8>`-shaped target) recognises the tag and base64-decodes
//!   on demand. Whitespace inside the scalar (typical for
//!   multi-line block-scalar `!!binary` content) is tolerated.

#![allow(missing_docs)]

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct Payload {
    name: String,
    #[serde(with = "serde_bytes")]
    body: Vec<u8>,
}

#[test]
fn vec_u8_with_serde_bytes_roundtrips_via_binary() {
    let original = Payload {
        name: "snapshot".into(),
        body: vec![0x00, 0x01, 0x02, 0xfe, 0xff, 0x80, 0x7f],
    };

    let yaml = noyalib::to_string(&original).unwrap();

    // Serialiser must emit a `!!binary` tagged scalar — not a UTF-8
    // string (which would fail on bytes outside the printable
    // range) and not a sequence of integers (which is what plain
    // `Vec<u8>` becomes without `serde_bytes`).
    assert!(
        yaml.contains("!!binary"),
        "expected !!binary tag in emitted YAML, got: {yaml}"
    );

    let round: Payload = noyalib::from_str(&yaml).unwrap();
    assert_eq!(round, original);
}

#[test]
fn byte_buf_roundtrips_via_binary() {
    use serde_bytes::ByteBuf;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Wrapper {
        data: ByteBuf,
    }

    let original = Wrapper {
        data: ByteBuf::from(b"Hello, World!".to_vec()),
    };

    let yaml = noyalib::to_string(&original).unwrap();
    assert!(yaml.contains("!!binary"));
    // `Hello, World!` in base64 is `SGVsbG8sIFdvcmxkIQ==`.
    assert!(
        yaml.contains("SGVsbG8sIFdvcmxkIQ=="),
        "expected base64 of \"Hello, World!\" in: {yaml}"
    );

    let round: Wrapper = noyalib::from_str(&yaml).unwrap();
    assert_eq!(round, original);
}

#[test]
fn empty_byte_buf_roundtrips() {
    use serde_bytes::ByteBuf;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Wrapper {
        data: ByteBuf,
    }

    let original = Wrapper {
        data: ByteBuf::new(),
    };
    let yaml = noyalib::to_string(&original).unwrap();
    let round: Wrapper = noyalib::from_str(&yaml).unwrap();
    assert_eq!(round, original);
}

#[test]
fn deserialize_handwritten_binary_scalar() {
    // The user authors `!!binary` directly. Any reasonable
    // formatting — leading whitespace, multi-line continuation —
    // must decode cleanly.
    let yaml = "\
name: snapshot
body: !!binary |
  SGVsbG8s
  IFdvcmxk
  IQ==
";
    let p: Payload = noyalib::from_str(yaml).unwrap();
    assert_eq!(p.name, "snapshot");
    assert_eq!(p.body, b"Hello, World!");
}

#[test]
fn deserialize_inline_binary_scalar() {
    // Same content, inline (no block-scalar continuation).
    let yaml = "\
name: snapshot
body: !!binary SGVsbG8sIFdvcmxkIQ==
";
    let p: Payload = noyalib::from_str(yaml).unwrap();
    assert_eq!(p.body, b"Hello, World!");
}

#[test]
fn deserialize_quoted_binary_scalar() {
    // Quoted form — rarer but valid.
    let yaml = "\
name: snapshot
body: !!binary \"SGVsbG8sIFdvcmxkIQ==\"
";
    let p: Payload = noyalib::from_str(yaml).unwrap();
    assert_eq!(p.body, b"Hello, World!");
}

#[test]
fn deserialize_rejects_invalid_base64() {
    let yaml = "\
name: snapshot
body: !!binary \"this is not base64\"
";
    let res: Result<Payload, _> = noyalib::from_str(yaml);
    assert!(
        res.is_err(),
        "non-base64 !!binary content must be a deserialize error"
    );
}

#[test]
fn full_byte_range_roundtrips() {
    use serde_bytes::ByteBuf;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Wrapper {
        data: ByteBuf,
    }

    // 0..=255 covers the full byte range, including all the bytes
    // that are unrepresentable as YAML scalars (NUL, the C0
    // controls, anything > 0x7F that does not form a valid UTF-8
    // sequence). The whole point of `!!binary` is making *these*
    // round-trip safely.
    let original = Wrapper {
        data: ByteBuf::from((0u8..=255).collect::<Vec<u8>>()),
    };
    let yaml = noyalib::to_string(&original).unwrap();
    let round: Wrapper = noyalib::from_str(&yaml).unwrap();
    assert_eq!(round, original);
}

#[test]
fn binary_under_strict_parser_config() {
    use noyalib::ParserConfig;

    let yaml = "\
secret: !!binary SGVsbG8sIFdvcmxkIQ==
";
    let cfg = ParserConfig::strict();

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct S {
        #[serde(with = "serde_bytes")]
        secret: Vec<u8>,
    }
    let v: S = noyalib::from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(v.secret, b"Hello, World!");
}
