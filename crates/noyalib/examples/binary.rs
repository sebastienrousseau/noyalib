// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Binary data handling: large integers, special float values, byte-like data.
//!
//! Demonstrates how noyalib handles edge cases at the boundary of YAML's
//! type system and Rust's numeric types.
//!
//! Run: `cargo run --example binary`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str, to_string};

fn main() {
    support::header("noyalib -- binary");

    // ── Large integers ───────────────────────────────────────────────
    support::task_with_output("Large integers (i64 boundary)", || {
        let yaml = format!(
            "max_i64: {}\nmin_i64: {}\noverflow: 99999999999999999999\n",
            i64::MAX,
            i64::MIN
        );
        let v: Value = from_str(&yaml).unwrap();
        vec![
            format!(
                "max_i64  = {} (i64)",
                v.get("max_i64").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
            format!(
                "min_i64  = {} (i64)",
                v.get("min_i64").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
            format!(
                "overflow = {} (stored as f64)",
                v.get("overflow").and_then(|v| v.as_f64()).unwrap_or(0.0)
            ),
        ]
    });

    // ── Special float values ─────────────────────────────────────────
    support::task_with_output("Special float values (.inf, .nan)", || {
        let yaml = "pos_inf: .inf\nneg_inf: -.inf\nnot_a_number: .nan\n";
        let v: Value = from_str(yaml).unwrap();

        let inf = v.get("pos_inf").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let neg = v.get("neg_inf").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let nan = v
            .get("not_a_number")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        vec![
            format!(".inf   = {} (is_infinite: {})", inf, inf.is_infinite()),
            format!("-.inf  = {} (is_infinite: {})", neg, neg.is_infinite()),
            format!(".nan   = {} (is_nan: {})", nan, nan.is_nan()),
        ]
    });

    // ── Hex and octal integers ───────────────────────────────────────
    support::task_with_output("Hex and octal integer parsing", || {
        let yaml = "hex: 0xFF\noctal: 0o77\nbinary_like: 0b1010\n";
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!(
                "0xFF  = {} (decimal)",
                v.get("hex").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
            format!(
                "0o77  = {} (decimal)",
                v.get("octal").and_then(|v| v.as_i64()).unwrap_or(0)
            ),
            format!(
                "0b1010 -> {}",
                match &v["binary_like"] {
                    Value::Number(n) => format!("{n} (parsed as number)"),
                    Value::String(s) => format!("\"{s}\" (kept as string)"),
                    other => format!("{other:?}"),
                }
            ),
        ]
    });

    // ── Roundtrip special values ─────────────────────────────────────
    support::task_with_output("Roundtrip special values", || {
        let yaml = "inf: .inf\nnan: .nan\nbig: 9223372036854775807\n";
        let v: Value = from_str(yaml).unwrap();
        let output = to_string(&v).unwrap();
        let rt: Value = from_str(&output).unwrap();

        vec![
            format!(
                "inf roundtrip = {}",
                rt.get("inf")
                    .and_then(|v| v.as_f64())
                    .map(|f| f.is_infinite())
                    .unwrap_or(false)
            ),
            format!(
                "nan roundtrip = {}",
                rt.get("nan")
                    .and_then(|v| v.as_f64())
                    .map(|f| f.is_nan())
                    .unwrap_or(false)
            ),
            format!(
                "big roundtrip = {}",
                rt.get("big").and_then(|v| v.as_i64()).unwrap_or(0) == i64::MAX
            ),
        ]
    });

    support::summary(4);
}
