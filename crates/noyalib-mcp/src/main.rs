// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `noyalib-mcp` — Model Context Protocol server exposing
//! noyalib's lossless YAML editing to AI agents.
//!
//! Communicates via newline-delimited JSON-RPC 2.0 over stdio per
//! the MCP 2025-06 spec. Two tools:
//!
//! - `noyalib_get`: read the value at a path in a YAML file.
//! - `noyalib_set`: set the value at a path, preserving every
//!   untouched byte (comments, indentation, sibling entries).
//!
//! # Why this exists
//!
//! AI agents that edit YAML configuration today regex-replace and
//! corrupt comments / formatting. noyalib's CST does the edits
//! losslessly; this server is the protocol shim that lets Claude,
//! Cursor, Zed, and any other MCP-aware client drive that engine
//! safely.
//!
//! # Install + connect
//!
//! ```text
//! cargo install noyalib-mcp
//! claude mcp add noyalib /usr/local/bin/noyalib-mcp
//! ```
//!
//! All dispatch logic lives in the `noyalib_mcp` library crate so
//! it can be exercised by `cargo test` directly. This binary is
//! the stdio transport shim — read a line, hand it to
//! [`noyalib_mcp::handle_message`], write the reply if any.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// Opt-in coverage exclusion: when `NOYALIB_COVERAGE=1`, the main
// stdio loop is excluded from instrumentation. The loop is the
// integration shim around `noyalib_mcp::handle_message`; its logic
// is covered end-to-end by `tests/protocol.rs` (subprocess-driven
// JSON-RPC) and the I/O failure paths on stdin/stdout require
// broken pipes that pure-Rust unit tests cannot reproduce.
#![cfg_attr(noyalib_coverage, allow(unstable_features))]
#![cfg_attr(noyalib_coverage, feature(coverage_attribute))]

use noyalib_mcp::{HandleOutcome, handle_message};
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

const HELP: &str = "\
noyalib-mcp — Model Context Protocol server for noyalib's lossless
              YAML editing.

USAGE:
  noyalib-mcp                   Start the JSON-RPC stdio loop (the
                                normal mode an MCP client invokes).
  noyalib-mcp --version | -V    Print version and exit.
  noyalib-mcp --help | -h       Print this help and exit.

NOTES:
  This binary speaks newline-delimited JSON-RPC 2.0 over stdio per
  the MCP 2025-06 spec. It is not designed for interactive use —
  configure your MCP-aware client (Claude, Cursor, Zed, …) to spawn
  it instead. Example for Claude:

    claude mcp add noyalib /usr/local/bin/noyalib-mcp

REPORTING BUGS:
  https://github.com/sebastienrousseau/noyalib/issues
";

#[cfg_attr(noyalib_coverage, coverage(off))]
fn main() -> ExitCode {
    // Honour the conventional `--version` / `--help` flags before
    // falling into the stdio JSON-RPC loop. Without these, a user
    // running `noyalib-mcp` to verify the install just sees a hung
    // process; printing version / help is the standard CLI hygiene.
    if let Some(arg) = std::env::args().nth(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("noyalib-mcp {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--help" | "-h" => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("noyalib-mcp: unknown argument `{other}`");
                eprintln!("Run `noyalib-mcp --help` for usage.");
                return ExitCode::from(2);
            }
        }
    }
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("noyalib-mcp: {e}");
            ExitCode::from(3)
        }
    }
}

fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut line = String::new();
    let mut handle = stdin.lock();

    loop {
        line.clear();
        let n = handle.read_line(&mut line)?;
        if n == 0 {
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match handle_message(trimmed) {
            HandleOutcome::Reply(payload) => {
                writeln!(stdout, "{payload}")?;
                stdout.flush()?;
            }
            HandleOutcome::Silent => {}
        }
    }
}
