// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `strict-deserialise` — reject YAML keys the target type does not
//! declare.
//!
//! By default serde *ignores* unknown fields. That is the right
//! behaviour for forward-compatible wire formats, but the wrong one for
//! configuration files: a typo like `porrt: 9090` is silently dropped,
//! the struct keeps its default, and the service quietly listens on the
//! wrong port. No error, no warning, nothing to grep for.
//!
//! The `strict-deserialise` feature adds `from_str_strict`,
//! `from_slice_strict` and `from_reader_strict`, which surface any
//! undeclared key as a typed [`noyalib::Error::UnknownField`]. The
//! non-strict entry points are unchanged, so strictness is opt-in per
//! call site — be strict about config you own, lenient about a third
//! party's payload, in the same program.
//!
//! Run: `cargo run --example strict_deserialise --features strict-deserialise`

#[path = "support.rs"]
mod support;

use noyalib::{from_slice_strict, from_str, from_str_strict};

#[derive(Debug, serde::Deserialize, PartialEq)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct AppConfig {
    name: String,
    server: ServerConfig,
}

fn main() {
    support::header("noyalib — strict-deserialise (unknown-field rejection)");

    support::task_with_output("A conforming document deserialises normally", || {
        let yaml = "host: 0.0.0.0\nport: 8080\n";
        let cfg: ServerConfig = from_str_strict(yaml).unwrap();
        assert_eq!(cfg.port, 8080);
        vec![format!("{cfg:?}")]
    });

    support::task_with_output("The silent-typo problem, with plain `from_str`", || {
        // `porrt` is not a field of ServerConfig. serde ignores it, so
        // the document parses cleanly and the typo leaves no trace.
        let yaml = "host: 0.0.0.0\nport: 8080\nporrt: 9090\n";
        let cfg: ServerConfig = from_str(yaml).unwrap();
        assert_eq!(cfg.port, 8080, "the typo never reached the struct");
        vec![
            format!("parsed OK: {cfg:?}"),
            "`porrt: 9090` was dropped silently — no error, no warning".to_string(),
        ]
    });

    support::task_with_output("The same document under `from_str_strict`", || {
        let yaml = "host: 0.0.0.0\nport: 8080\nporrt: 9090\n";
        let err = from_str_strict::<ServerConfig>(yaml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("porrt"),
            "the error must name the offending key so it is greppable"
        );
        vec![msg]
    });

    support::task_with_output("Unknown keys are caught at any nesting depth", || {
        // `tls` is undeclared on the *nested* ServerConfig.
        let yaml = "name: api\nserver:\n  host: 0.0.0.0\n  port: 8080\n  tls: true\n";
        let err = from_str_strict::<AppConfig>(yaml).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("tls"));
        vec![msg]
    });

    support::task_with_output("`from_slice_strict` for bytes off the wire", || {
        let bytes = b"host: 127.0.0.1\nport: 3000\nextra: nope\n";
        let err = from_slice_strict::<ServerConfig>(bytes).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("extra"));
        vec![msg]
    });

    support::task_with_output("Strictness is per call site, not global", || {
        let yaml = "host: h\nport: 1\nvendor_ext: {}\n";
        assert!(from_str::<ServerConfig>(yaml).is_ok());
        assert!(from_str_strict::<ServerConfig>(yaml).is_err());
        vec!["lenient: Ok    strict: Err — same bytes, same process".to_string()]
    });

    support::summary(6);
    support::footer();
}
