// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! I/O format examples: from_slice, from_reader, to_writer, to_fmt_writer.
//!
//! Run: `cargo run --example pipes`

#[path = "support.rs"]
mod support;

use std::io::Cursor;

fn main() {
    support::header("noyalib -- pipes");

    let yaml_bytes = b"name: noyalib\nversion: 1\n";

    support::task("from_slice", || {
        let _: noyalib::Value = noyalib::from_slice(yaml_bytes).unwrap();
    });

    let config = noyalib::ParserConfig::strict();

    support::task("from_slice_with_config (strict)", || {
        let _: noyalib::Value = noyalib::from_slice_with_config(yaml_bytes, &config).unwrap();
    });

    support::task("from_reader", || {
        let reader = Cursor::new("host: localhost\nport: 8080\n");
        let _: noyalib::Value = noyalib::from_reader(reader).unwrap();
    });

    support::task("from_reader_with_config", || {
        let reader = Cursor::new("key: value\n");
        let _: noyalib::Value = noyalib::from_reader_with_config(reader, &config).unwrap();
    });

    support::task("to_writer", || {
        let data = noyalib::Value::String("hello".to_string());
        let mut buf = Vec::new();
        noyalib::to_writer(&mut buf, &data).unwrap();
    });

    support::task("to_writer_with_config (indent=4)", || {
        let data = noyalib::Value::String("hello".to_string());
        let ser_config = noyalib::SerializerConfig::new().indent(4);
        let mut buf = Vec::new();
        noyalib::to_writer_with_config(&mut buf, &data, &ser_config).unwrap();
    });

    support::task("to_fmt_writer", || {
        let data = noyalib::Value::String("hello".to_string());
        let mut output = String::new();
        noyalib::to_fmt_writer(&mut output, &data).unwrap();
    });

    support::task("to_fmt_writer_with_config (document_start)", || {
        let data = noyalib::Value::String("hello".to_string());
        let mut output = String::new();
        let ser_config = noyalib::SerializerConfig::new().document_start(true);
        noyalib::to_fmt_writer_with_config(&mut output, &data, &ser_config).unwrap();
    });

    support::summary(8);
}
