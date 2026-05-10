// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Render `garde` validation failures as `miette::Report` pinned
//! at the source location of the offending `Spanned<T>` payload.
//!
//! Run: `cargo run --example validated_miette --features miette,garde`

#[path = "support.rs"]
mod support;

use garde::Validate;
use noyalib::validated_miette::garde_errors_to_miette;
use noyalib::Spanned;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
struct ServerCfg {
    #[garde(length(min = 1, max = 255))]
    host: String,
    #[garde(range(min = 1024, max = 65535))]
    port: u16,
}

fn main() {
    support::header("noyalib -- validated_miette");

    support::task_with_output("Render garde violations as miette::Report", || {
        let yaml = "host: \"\"\nport: 22\n"; // both fields invalid
        let wrapped: Spanned<ServerCfg> = noyalib::from_str(yaml).unwrap();
        match wrapped.value.validate() {
            Ok(()) => vec!["unexpectedly valid".into()],
            Err(errs) => {
                let report = garde_errors_to_miette(&wrapped, &errs, yaml, "config.yaml");
                vec![format!("{report}")]
            }
        }
    });

    support::task_with_output("Happy path: validation succeeds, no diagnostic", || {
        let yaml = "host: db.local\nport: 5432\n";
        let wrapped: Spanned<ServerCfg> = noyalib::from_str(yaml).unwrap();
        match wrapped.value.validate() {
            Ok(()) => vec![format!(
                "valid: host={} port={}",
                wrapped.value.host, wrapped.value.port
            )],
            Err(_) => vec!["unexpectedly invalid".into()],
        }
    });
}
