// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Lazy multi-document iterator over `std::io::Read`.
//!
//! Verifies [`noyalib::read`] / [`noyalib::read_with_config`]
//! against the issue-defined contract:
//!
//! * empty stream yields zero documents
//! * single document parses
//! * multi-document streams iterate doc-by-doc
//! * deserialisation errors on individual documents surface as
//!   `Err` items but do not terminate iteration
//! * syntax errors are returned synchronously from
//!   `read` / `read_with_config` before iteration starts
//! * `ParserConfig` security limits are honoured per-document

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::{read, read_with_config, DocumentReadIterator, ParserConfig, Value};
use serde::Deserialize;
use std::io::Cursor;

#[derive(Debug, Deserialize, PartialEq)]
struct Doc {
    id: u32,
    name: String,
}

#[test]
fn empty_stream_yields_zero_documents() {
    let iter: DocumentReadIterator<Value> = read(Cursor::new("")).unwrap();
    assert_eq!(iter.len(), 0);
    assert!(iter.is_empty());
    let collected: Vec<_> = iter.collect();
    assert!(collected.is_empty());
}

#[test]
fn single_document_parses() {
    let yaml = "id: 7\nname: alpha\n";
    let docs: Vec<Doc> = read::<_, Doc>(Cursor::new(yaml))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        docs,
        vec![Doc {
            id: 7,
            name: "alpha".into()
        }]
    );
}

#[test]
fn multi_document_iterates_lazily() {
    let yaml = "id: 1\nname: a\n---\nid: 2\nname: b\n---\nid: 3\nname: c\n";
    let iter = read::<_, Doc>(Cursor::new(yaml)).unwrap();
    assert_eq!(iter.len(), 3);
    let docs: Vec<Doc> = iter.map(Result::unwrap).collect();
    assert_eq!(
        docs,
        vec![
            Doc {
                id: 1,
                name: "a".into()
            },
            Doc {
                id: 2,
                name: "b".into()
            },
            Doc {
                id: 3,
                name: "c".into()
            },
        ]
    );
}

#[test]
fn deser_error_does_not_halt_iteration() {
    // Document 2 has the wrong shape — should yield Err but the
    // iterator continues so document 3 still parses.
    let yaml = "id: 1\nname: a\n---\nid: 2\nbroken: nope\n---\nid: 3\nname: c\n";
    let results: Vec<_> = read::<_, Doc>(Cursor::new(yaml)).unwrap().collect();
    assert_eq!(results.len(), 3);
    assert!(results[0].is_ok());
    assert!(results[1].is_err());
    assert!(results[2].is_ok());
    assert_eq!(results[2].as_ref().unwrap().id, 3);
}

#[test]
fn syntax_error_returned_eagerly() {
    let yaml = "key: [unclosed\n";
    let res: Result<DocumentReadIterator<Value>, _> = read(Cursor::new(yaml));
    assert!(res.is_err(), "syntax errors must be reported up-front");
}

#[test]
fn read_with_config_respects_max_document_length() {
    // Set max_document_length tiny so even a small input trips the
    // soft 64× aggregate cap.
    let cfg = ParserConfig::new().max_document_length(8);
    let yaml = "id: 1\nname: alpha\n".repeat(64); // > 8 * 64 bytes
    let res: Result<DocumentReadIterator<Value>, _> = read_with_config(Cursor::new(yaml), &cfg);
    assert!(
        res.is_err(),
        "aggregated buffer over 64× max_document_length must error"
    );
}

#[test]
fn read_with_config_strict_mode_round_trips() {
    let cfg = ParserConfig::strict();
    let yaml = "id: 9\nname: strict-mode\n";
    let docs: Vec<Doc> = read_with_config::<_, Doc>(Cursor::new(yaml), &cfg)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(docs[0].id, 9);
    assert_eq!(docs[0].name, "strict-mode");
}

#[test]
fn iterator_size_hint_and_exact_size() {
    let yaml = "a: 1\n---\nb: 2\n---\nc: 3\n";
    let iter: DocumentReadIterator<Value> = read(Cursor::new(yaml)).unwrap();
    assert_eq!(iter.size_hint(), (3, Some(3)));
    assert_eq!(iter.len(), 3);
}

#[test]
fn document_read_iterator_is_debug() {
    let iter: DocumentReadIterator<Value> = read(Cursor::new("a: 1\n")).unwrap();
    let dbg = format!("{iter:?}");
    assert!(dbg.contains("DocumentReadIterator"));
}

#[test]
fn read_propagates_io_error() {
    use std::io::{self, Read};
    struct FailReader;
    impl Read for FailReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("simulated I/O failure"))
        }
    }
    let res: Result<DocumentReadIterator<Value>, _> = read(FailReader);
    assert!(res.is_err());
    let msg = format!("{}", res.unwrap_err());
    assert!(msg.contains("reader I/O failed"));
}
