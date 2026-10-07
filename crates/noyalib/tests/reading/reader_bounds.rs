// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The `io::Read` entry points stop reading one byte past
//! `max_document_length`. Before this, `from_reader` buffered the whole
//! source with `read_to_string` and only then checked the budget, so an
//! unbounded reader could exhaust memory before any limit applied.

use std::io::{self, Read};

use noyalib::{ParserConfig, Value, from_reader, from_reader_with_config};

/// A reader far larger than any budget under test that counts how much
/// was pulled from it. Finite, so an unbounded implementation fails the
/// assertion instead of hanging the suite.
struct Big {
    remaining: usize,
    served: usize,
}

impl Big {
    fn over(budget: usize) -> Self {
        Self {
            remaining: budget + (4 << 20),
            served: 0,
        }
    }
}

impl Read for Big {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.remaining);
        buf[..n].fill(b'a');
        self.remaining -= n;
        self.served += n;
        Ok(n)
    }
}

/// `read_to_end` may probe past the `take` limit by one buffer; allow it.
const SLACK: usize = 1 + 8192;

#[test]
fn endless_reader_fails_the_length_budget_without_buffering_it_all() {
    let cfg = ParserConfig::default().max_document_length(64);
    let mut src = Big::over(64);
    let err = from_reader_with_config::<_, Value>(&mut src, &cfg).unwrap_err();
    assert!(
        err.to_string().contains("maximum length of 64 bytes"),
        "unexpected error: {err}"
    );
    // One byte past the budget is enough to know; the read must stop
    // there, not run until the allocator gives up.
    assert!(src.served <= 64 + SLACK, "read {} bytes", src.served);
}

#[test]
fn reader_exactly_at_the_budget_parses() {
    let yaml = "key: value\n";
    let cfg = ParserConfig::default().max_document_length(yaml.len());
    let v: Value = from_reader_with_config(yaml.as_bytes(), &cfg).unwrap();
    assert_eq!(v["key"], Value::from("value"));
}

#[test]
fn reader_one_byte_over_the_budget_is_rejected() {
    let yaml = "key: value\n";
    let cfg = ParserConfig::default().max_document_length(yaml.len() - 1);
    let err = from_reader_with_config::<_, Value>(yaml.as_bytes(), &cfg).unwrap_err();
    assert!(err.to_string().contains("exceeds maximum length"), "{err}");
}

#[test]
fn invalid_utf8_still_reports_an_io_error() {
    let bytes: &[u8] = b"key: \xff\xfe\n";
    let err = from_reader::<_, Value>(bytes).unwrap_err();
    assert!(
        matches!(err, noyalib::Error::Io(ref e) if e.kind() == io::ErrorKind::InvalidData),
        "{err:?}"
    );
}

#[test]
fn default_budget_bounds_the_plain_entry_point_too() {
    let cap = ParserConfig::default().max_document_length;
    let mut src = Big::over(cap);
    let err = from_reader::<_, Value>(&mut src).unwrap_err();
    assert!(err.to_string().contains("exceeds maximum length"), "{err}");
    assert!(
        src.served <= cap + SLACK,
        "read {} bytes for a cap of {cap}",
        src.served
    );
}

#[cfg(feature = "strict-deserialise")]
#[test]
fn strict_reader_is_bounded() {
    let cap = ParserConfig::default().max_document_length;
    let mut src = Big::over(cap);
    let err = noyalib::from_reader_strict::<_, Value>(&mut src).unwrap_err();
    assert!(err.to_string().contains("exceeds maximum length"), "{err}");
    assert!(src.served <= cap + SLACK, "read {} bytes", src.served);
}

#[cfg(feature = "compat-serde-yaml")]
#[test]
fn compat_reader_is_bounded() {
    let cap = ParserConfig::serde_yaml_compat().max_document_length;
    let mut src = Big::over(cap);
    let err = noyalib::compat::serde_yaml::from_reader::<_, Value>(&mut src).unwrap_err();
    assert!(err.to_string().contains("exceeds maximum length"), "{err}");
    assert!(src.served <= cap + SLACK, "read {} bytes", src.served);
}
