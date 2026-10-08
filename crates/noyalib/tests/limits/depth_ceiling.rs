// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! No path builds a `Value` deeper than 256 levels, whatever
//! `max_depth` says.
//!
//! Dropping, cloning or printing a `Value` recurses once per level, so a
//! caller that raised `max_depth` far enough, or a `Value` read from
//! another serde format, could build one that aborted the process on a
//! small stack when it was dropped. Each case below runs on a 1 MiB
//! thread, which a deeper value would overflow.

use noyalib::{ParserConfig, SerializerConfig, Value};

const CEILING: usize = 256;

fn nested(depth: usize) -> String {
    format!("{}{}", "[".repeat(depth), "]".repeat(depth))
}

fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("the 1 MiB thread must not overflow")
}

fn raised() -> ParserConfig {
    ParserConfig::new().max_depth(1_000_000)
}

#[test]
fn a_raised_max_depth_stops_at_the_ceiling_on_every_loader() {
    on_small_stack(|| {
        let deep = nested(CEILING + 1);
        let ok = nested(CEILING);
        let cfg = raised();
        let value: Result<Value, _> = noyalib::from_str_with_config(&deep, &cfg);
        assert!(
            value.is_err(),
            "from_str_with_config accepted {} levels",
            CEILING + 1
        );
        let ok_value: Value = noyalib::from_str_with_config(&ok, &cfg).expect("256 levels parse");
        drop(ok_value);
        let docs: Result<Vec<_>, _> =
            noyalib::load_all_with_config(&deep, &cfg).map(|d| d.collect());
        assert!(
            docs.is_err(),
            "load_all_with_config accepted {} levels",
            CEILING + 1
        );
        let cst = noyalib::cst::parse_document_with_config(&deep, &cfg);
        assert!(cst.is_err(), "the CST accepted {} levels", CEILING + 1);
    });
}

#[test]
fn a_value_from_another_format_stops_at_the_ceiling() {
    on_small_stack(|| {
        let mut json = serde_json::Value::Null;
        for _ in 0..=CEILING {
            json = serde_json::Value::Array(vec![json]);
        }
        let err = serde_json::from_value::<Value>(json).expect_err("too deep");
        assert!(err.to_string().contains("recursion depth limit"), "{err}");
    });
}

#[test]
fn the_serializer_stops_at_the_ceiling() {
    on_small_stack(|| {
        let mut value = Value::Null;
        for _ in 0..=CEILING {
            value = Value::Sequence(vec![value]);
        }
        let cfg = SerializerConfig::default().max_depth(1_000_000);
        assert!(noyalib::to_string_with_config(&value, &cfg).is_err());
    });
}
