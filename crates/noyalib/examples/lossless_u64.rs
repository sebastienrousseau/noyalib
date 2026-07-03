// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Lossless unsigned integers above `i64::MAX`.
//!
//! Run: `cargo run --example lossless_u64 --features lossless-u64`

#[path = "support.rs"]
mod support;

use noyalib::{
    Number, ParserConfig, SerializerConfig, Value, from_str_with_config,
    to_string_value_with_config,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Record {
    id: u64,
}

fn main() {
    support::header("noyalib -- lossless u64");

    let parse_cfg = ParserConfig::new().lossless_u64_integers(true);
    // The serializer side has no runtime toggle: when the
    // `lossless-u64` feature is compiled in, `Number::Unsigned` is
    // the emitted variant for values above `i64::MAX`. When the
    // feature is off the variant does not exist. `SerializerConfig`
    // still governs indentation / flow style / etc.
    let ser_cfg = SerializerConfig::new();

    let yaml = format!("id: {}\n", u64::MAX);

    support::task_with_output("parse u64::MAX as Number::Unsigned", || {
        let v: Value = from_str_with_config(&yaml, &parse_cfg).unwrap();
        match &v["id"] {
            Value::Number(Number::Unsigned(n)) => vec![format!("id = Unsigned({n})")],
            other => vec![format!("unexpected: {other:?}")],
        }
    });

    support::task_with_output("round-trip Value through serializer config", || {
        let v: Value = from_str_with_config(&yaml, &parse_cfg).unwrap();
        let emitted = to_string_value_with_config(&v, &ser_cfg).unwrap();
        let reparsed: Value = from_str_with_config(&emitted, &parse_cfg).unwrap();
        vec![
            format!("emitted: {}", emitted.trim()),
            format!(
                "reparsed: {:?}",
                match &reparsed["id"] {
                    Value::Number(Number::Unsigned(n)) => format!("Unsigned({n})"),
                    other => format!("{other:?}"),
                }
            ),
        ]
    });

    support::task_with_output("typed struct field deserialises as u64", || {
        let record: Record = from_str_with_config(&yaml, &parse_cfg).unwrap();
        vec![format!("Record {{ id: {} }}", record.id)]
    });

    support::summary(3);
}
