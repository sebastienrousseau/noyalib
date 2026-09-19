// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Streaming `!!binary` support.
//!
//! Verifies that [`StreamingDeserializer`] honours `!!binary` tags
//! directly — decoding RFC 4648 base64 on demand without falling
//! back to the AST path. Drives the deserializer with the public
//! constructors so the tests exercise the same code that downstream
//! users hit.

#![allow(missing_docs)]

use noyalib::StreamingDeserializer;
use serde_bytes::ByteBuf;
use serde_core::Deserialize as _;

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Payload {
    name: String,
    #[serde(with = "serde_bytes")]
    body: Vec<u8>,
}

#[test]
fn streaming_decodes_inline_binary() {
    let yaml = "name: snapshot\nbody: !!binary SGVsbG8sIFdvcmxkIQ==\n";
    let mut de = StreamingDeserializer::new(yaml);
    let p = Payload::deserialize(&mut de).unwrap();
    assert_eq!(p.name, "snapshot");
    assert_eq!(p.body, b"Hello, World!");
}

#[test]
fn streaming_decodes_block_scalar_binary() {
    // Multi-line block-scalar — the typical layout for real-world
    // `!!binary` payloads. Newlines/indents inside the scalar must
    // be stripped before base64-decoding.
    let yaml = "\
name: snapshot
body: !!binary |
  SGVsbG8s
  IFdvcmxk
  IQ==
";
    let mut de = StreamingDeserializer::new(yaml);
    let p = Payload::deserialize(&mut de).unwrap();
    assert_eq!(p.body, b"Hello, World!");
}

#[test]
fn streaming_byte_buf_roundtrip() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Wrapper {
        data: ByteBuf,
    }
    let yaml = "data: !!binary SGVsbG8sIFdvcmxkIQ==\n";
    let mut de = StreamingDeserializer::new(yaml);
    let w = Wrapper::deserialize(&mut de).unwrap();
    assert_eq!(w.data.as_ref(), b"Hello, World!");
}

#[test]
fn streaming_full_byte_range() {
    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Wrapper {
        data: ByteBuf,
    }
    let original: Vec<u8> = (0u8..=255).collect();
    // Round-trip via the canonical encoder, then drive deserialise
    // via the streaming path.
    let yaml = noyalib::to_string(&Wrapper {
        data: ByteBuf::from(original.clone()),
    })
    .unwrap();
    let mut de = StreamingDeserializer::new(&yaml);
    let round = Wrapper::deserialize(&mut de).unwrap();
    assert_eq!(round.data.as_ref(), original.as_slice());
}

#[test]
fn streaming_rejects_invalid_base64() {
    let yaml = "name: x\nbody: !!binary \"not valid base64!\"\n";
    let mut de = StreamingDeserializer::new(yaml);
    let res: Result<Payload, _> = Payload::deserialize(&mut de);
    assert!(res.is_err(), "non-base64 !!binary content must error");
}

#[test]
fn streaming_untagged_string_visits_raw_bytes() {
    // Without a `!!binary` tag, deserialising into a byte target
    // surfaces the underlying scalar bytes verbatim — base64-decoding
    // is opt-in via the tag.
    #[derive(Debug, serde::Deserialize)]
    struct Wrapper {
        data: ByteBuf,
    }
    let yaml = "data: hello\n";
    let mut de = StreamingDeserializer::new(yaml);
    let w = Wrapper::deserialize(&mut de).unwrap();
    assert_eq!(w.data.as_ref(), b"hello");
}
