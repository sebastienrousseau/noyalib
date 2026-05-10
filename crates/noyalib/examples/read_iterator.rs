// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Lazy multi-document iterator over `std::io::Read`.
//!
//! `noyalib::read` accepts any `R: io::Read` (file, network socket,
//! `Stdin`, byte slice via `Cursor`) and returns a
//! `DocumentReadIterator` that yields `Result<T>` per YAML document.
//! Per-document deserialisation errors surface as `Err` items so
//! callers can recover and continue; syntax errors are returned
//! synchronously from `read` before iteration starts.
//!
//! Run: `cargo run --example read_iterator`

#[path = "support.rs"]
mod support;

use noyalib::read;
use serde::Deserialize;
use std::io::Cursor;

#[derive(Debug, Deserialize)]
struct Doc {
    id: u32,
    name: String,
}

fn main() {
    support::header("noyalib -- read_iterator");

    support::task_with_output("Iterate three documents from a Cursor", || {
        let yaml = "id: 1\nname: alpha\n---\nid: 2\nname: beta\n---\nid: 3\nname: gamma\n";
        read::<_, Doc>(Cursor::new(yaml))
            .unwrap()
            .filter_map(Result::ok)
            .map(|d| format!("#{} {}", d.id, d.name))
            .collect()
    });

    support::task_with_output("Per-document errors do not halt iteration", || {
        let yaml = "id: 1\nname: a\n---\nid: 2\nbroken: nope\n---\nid: 3\nname: c\n";
        let mut ok = 0;
        let mut err = 0;
        for r in read::<_, Doc>(Cursor::new(yaml)).unwrap() {
            if r.is_ok() {
                ok += 1;
            } else {
                err += 1;
            }
        }
        vec![format!(
            "ok={ok} err={err}  (iteration continues across errors)"
        )]
    });

    support::task_with_output("Syntax errors are reported synchronously", || {
        use noyalib::{DocumentReadIterator, Value};
        let yaml = "key: [unclosed\n";
        let res: Result<DocumentReadIterator<Value>, _> = read(Cursor::new(yaml));
        vec![format!(
            "syntax-error eager surface: {}",
            if res.is_err() { "yes" } else { "no" }
        )]
    });
}
