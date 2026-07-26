// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Render a noyalib parse error through `ariadne`.
//!
//! Pairs with the existing `miette` adapter (`cargo run --example
//! diagnostic`); same source position information rendered with
//! whichever diagnostic crate the caller's pipeline already uses.
//!
//! Run: `cargo run --example ariadne_diagnostic --features ariadne`

#[path = "support.rs"]
mod support;

use ariadne::Source;
use noyalib::ariadne_adapter::error_to_ariadne_report;
use noyalib::{Value, from_str};

fn main() {
    support::header("noyalib -- ariadne_diagnostic");

    support::task_with_output("Render an unclosed-flow error via ariadne", || {
        let source = "service:\n  port: 8080\n  hosts: [primary, secondary\n";
        let err = from_str::<Value>(source).unwrap_err();
        let report = error_to_ariadne_report(&err, "config.yaml", source);
        let mut out: Vec<u8> = Vec::new();
        report
            .write(("config.yaml", Source::from(source)), &mut out)
            .unwrap();
        String::from_utf8_lossy(&out)
            .lines()
            .map(|l| l.to_string())
            .collect()
    });

    support::task_with_output("Header-only report when error carries no location", || {
        use noyalib::Error;
        let err = Error::Custom("synthetic test diagnostic".into());
        let report = error_to_ariadne_report(&err, "ad-hoc", "");
        let mut out: Vec<u8> = Vec::new();
        report
            .write(("ad-hoc", Source::from("")), &mut out)
            .unwrap();
        String::from_utf8_lossy(&out)
            .lines()
            .map(|l| l.to_string())
            .collect()
    });
}
