// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Spanned<T>: track exact source locations for deserialized fields.
//!
//! Run: `cargo run --example source`

#[path = "support.rs"]
mod support;

use noyalib::{Spanned, from_str};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    host: Spanned<String>,
    port: Spanned<u16>,
    debug: Spanned<bool>,
}

fn main() {
    support::header("noyalib -- source");

    support::task_with_output("Parse and inspect spanned values", || {
        let yaml = "host: localhost\nport: 8080\ndebug: true\n";
        let config: Config = from_str(yaml).unwrap();

        vec![
            format!("host  = \"{}\"", *config.host),
            format!(
                "  at: line {}, col {}  (byte {})",
                config.host.start.line(),
                config.host.start.column(),
                config.host.start.index()
            ),
            format!("port  = {}", *config.port),
            format!(
                "  at: line {}, col {}  (byte {})",
                config.port.start.line(),
                config.port.start.column(),
                config.port.start.index()
            ),
            format!("debug = {}", *config.debug),
            format!(
                "  at: line {}, col {}  (byte {})",
                config.debug.start.line(),
                config.debug.start.column(),
                config.debug.start.index()
            ),
        ]
    });

    support::task("Spanned<T> serializes transparently as T", || {
        let yaml = "host: localhost\nport: 8080\ndebug: true\n";
        let config: Config = from_str(yaml).unwrap();
        let output = noyalib::to_string(&config).unwrap();
        // The output should NOT contain any span metadata
        assert!(!output.contains("line"));
        assert!(!output.contains("column"));
        assert!(output.contains("host: localhost"));
    });

    support::summary(2);
}
