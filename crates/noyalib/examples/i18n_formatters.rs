// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `MessageFormatter` — render `Error` through pluggable
//! formatters.
//!
//! Two bundled impls: `DefaultFormatter` (verbatim
//! developer-facing message) and `UserFormatter` (short plain-
//! language sentences appropriate for non-developer audiences).
//! Custom localisation / rich-rendering plug in via the trait.
//!
//! Run: `cargo run --example i18n_formatters`

#[path = "support/mod.rs"]
mod support;

use noyalib::i18n::{DefaultFormatter, MessageFormatter, UserFormatter};
use noyalib::{Error, Value, from_str};

fn main() {
    support::header("noyalib -- i18n_formatters");

    support::task_with_output("DefaultFormatter — verbatim Display output", || {
        let err = from_str::<Value>("a: [unclosed").unwrap_err();
        vec![DefaultFormatter.format(&err)]
    });

    support::task_with_output("UserFormatter — plain-language sentence", || {
        let err = from_str::<Value>("a: [unclosed").unwrap_err();
        vec![UserFormatter.format(&err)]
    });

    support::task_with_output("UserFormatter handles each Error category", || {
        let errors: Vec<Error> = vec![
            Error::DuplicateKey("api_key".into()),
            Error::MissingField("password".into()),
            Error::TypeMismatch {
                expected: "integer",
                found: "string".into(),
            },
            Error::RecursionLimitExceeded { depth: 256 },
            Error::UnknownAnchor("backend-config".into()),
        ];
        errors
            .iter()
            .map(|e| format!("{:32}{}", format!("{e:.32}"), UserFormatter.format(e)))
            .collect()
    });

    support::task_with_output("Custom formatter — implement MessageFormatter", || {
        // A trivial example: uppercase all letters in the
        // developer message. Real-world: localisation tables,
        // GUI-friendly templates, audit-log structured output.
        struct ShoutFormatter;
        impl MessageFormatter for ShoutFormatter {
            fn format(&self, error: &Error) -> String {
                error.to_string().to_uppercase()
            }
        }
        let err = from_str::<Value>("a: [unclosed").unwrap_err();
        vec![err.render_with_formatter(&ShoutFormatter)]
    });

    support::task_with_output(
        "Error::render_with_formatter — dispatch entry point",
        || {
            let err = from_str::<Value>("a: [unclosed").unwrap_err();
            vec![
                format!("dev:  {}", err.render_with_formatter(&DefaultFormatter)),
                format!("user: {}", err.render_with_formatter(&UserFormatter)),
            ]
        },
    );
}
