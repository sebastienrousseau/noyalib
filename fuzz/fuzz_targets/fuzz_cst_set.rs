//! Fuzz target: `Document::set` and `Document::replace_span` with an
//! arbitrary path and fragment against an arbitrary document.
//!
//! The input is three or four NUL-separated fields:
//! `op \0 source \0 path \0 fragment`. `op` starting with `r` drives
//! `replace_span` over the span of `path`; anything else drives `set`.
//! Keeping the fields as plain text lets the seed and regression files
//! be read and written by hand.
//!
//! The edit runs on a thread with a 1 MiB stack, the size a small
//! worker thread gets, so a fragment that recurses per nesting level
//! aborts here at a depth a hand-written seed can reach instead of
//! needing the main thread's 8 MiB.
//!
//! # Invariants
//!
//! 1. No panic and no stack overflow, whatever the fragment.
//! 2. A refused edit leaves the source byte-identical.
//! 3. An accepted edit leaves a document that re-parses and passes
//!    `Document::validate`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::cst::parse_document;
use noyalib::Value;

fn check(op: &str, source: &str, path: &str, fragment: &str) {
    let Ok(mut doc) = parse_document(source) else {
        return;
    };
    let before = doc.source().to_owned();
    let result = if op.starts_with('r') {
        let Some((start, end)) = doc.span_at(path) else {
            return;
        };
        doc.replace_span(start, end, fragment)
    } else {
        doc.set(path, fragment)
    };
    if result.is_err() {
        assert_eq!(doc.source(), before, "a refused edit changed the document");
        return;
    }
    if let Err(e) = noyalib::from_str::<Value>(doc.source()) {
        panic!("an accepted edit does not re-parse ({e}): {:?}", doc.source());
    }
    if let Err(e) = doc.validate() {
        panic!("an accepted edit fails validate ({e}): {:?}", doc.source());
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut fields = text.splitn(4, '\0');
    let (Some(op), Some(source), Some(path)) = (fields.next(), fields.next(), fields.next()) else {
        return;
    };
    let fragment = fields.next().unwrap_or("");
    let (op, source, path, fragment) = (
        op.to_owned(),
        source.to_owned(),
        path.to_owned(),
        fragment.to_owned(),
    );
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(move || check(&op, &source, &path, &fragment))
        .expect("spawn")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
});
