// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Zero-copy patterns: minimize allocations during deserialization.
//!
//! For true zero-copy `&'de str` borrowing straight from the input, use
//! `from_str_borrowing` (typed) or `from_str_borrowed` → `BorrowedValue`
//! (untyped) — demonstrated in `zero_copy_borrow.rs`.
//!
//! This example demonstrates the complementary allocation-reduction
//! patterns: Cow<str>, from_value with references, and Value-as-view.
//!
//! Run: `cargo run --example borrow`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str};
use std::borrow::Cow;

fn main() {
    support::header("noyalib -- borrow");

    // ── Value as zero-copy view ──────────────────────────────────────
    support::task_with_output("Value::as_str() borrows from the Value", || {
        let yaml = "name: noyalib\nhost: localhost\npath: /api/v1\n";
        let v: Value = from_str(yaml).unwrap();

        // as_str() returns &str — no allocation, borrows from Value
        let name: &str = v.get("name").and_then(|v| v.as_str()).unwrap();
        let host: &str = v.get("host").and_then(|v| v.as_str()).unwrap();
        let path: &str = v.get("path").and_then(|v| v.as_str()).unwrap();

        vec![
            format!("name = {name} (borrowed &str)"),
            format!("host = {host} (borrowed &str)"),
            format!("path = {path} (borrowed &str)"),
            "0 allocations for field access".to_string(),
        ]
    });

    // ── Cow<str> for conditional ownership ───────────────────────────
    support::task_with_output("Cow<str>: own only when needed", || {
        let yaml = "greeting: hello\n";
        let v: Value = from_str(yaml).unwrap();

        let greeting: Cow<str> = match v.get("greeting").and_then(|v| v.as_str()) {
            Some(s) => Cow::Borrowed(s),
            None => Cow::Owned("default".to_string()),
        };

        let missing: Cow<str> = match v.get("missing").and_then(|v| v.as_str()) {
            Some(s) => Cow::Borrowed(s),
            None => Cow::Owned("fallback".to_string()),
        };

        vec![
            format!(
                "greeting = {greeting} ({})",
                if matches!(greeting, Cow::Borrowed(_)) {
                    "borrowed"
                } else {
                    "owned"
                }
            ),
            format!(
                "missing  = {missing} ({})",
                if matches!(missing, Cow::Borrowed(_)) {
                    "borrowed"
                } else {
                    "owned"
                }
            ),
        ]
    });

    // ── get_path avoids intermediate copies ──────────────────────────
    support::task_with_output("get_path: traverse without copying", || {
        let yaml =
            "server:\n  database:\n    host: db.internal\n    pool:\n      min: 5\n      max: 20\n";
        let v: Value = from_str(yaml).unwrap();

        // Each get_path call returns a reference — no cloning
        let host: &str = v
            .get_path("server.database.host")
            .and_then(|v| v.as_str())
            .unwrap();
        let max: i64 = v
            .get_path("server.database.pool.max")
            .and_then(|v| v.as_i64())
            .unwrap();

        vec![
            format!("host = {host} (4 levels deep, 0 copies)"),
            format!("max  = {max} (5 levels deep, 0 copies)"),
        ]
    });

    // ── Batch processing without per-item allocation ─────────────────
    support::task_with_output("Batch: process 1000 items, borrow all", || {
        let yaml: String = (0..1000)
            .map(|i| format!("- name: item_{i}\n  value: {i}\n"))
            .collect();

        let v: Value = from_str(&yaml).unwrap();
        let seq = v.as_sequence().unwrap();

        // Count items by borrowing — zero per-item allocation
        let total: i64 = seq
            .iter()
            .filter_map(|item| item.get("value").and_then(|v| v.as_i64()))
            .sum();

        vec![
            format!("items = {}", seq.len()),
            format!("sum   = {total}"),
            "0 String allocations during iteration".to_string(),
        ]
    });

    // ── Status: zero-copy options ────────────────────────────────────
    support::task_with_output("Zero-copy status", || {
        vec![
            "Today:   Value::as_str() borrows from parsed AST".to_string(),
            "Today:   get_path() traverses without cloning".to_string(),
            "Today:   Cow<str> for conditional ownership".to_string(),
            "Today:   from_str_borrowing / BorrowedValue borrow &'de str \
                      from input (see zero_copy_borrow.rs)"
                .to_string(),
        ]
    });

    support::summary(5);
}
