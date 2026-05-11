// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `noyafmt` — format YAML files via the CST formatter.
//!
//! Mirrors the `rustfmt` / `prettier` ergonomics so it slots into
//! existing developer workflows: `--check` for CI gates, `--write`
//! for in-place rewrites, stdin/stdout for editor integration.
//!
//! The argv-parsing surface lives in [`noya_cli::NoyafmtCli`] so
//! the same Command tree feeds the binary, the build-time codegen,
//! and the `cargo xtask` runner.

use clap::Parser;
use noya_cli::NoyafmtCli;
use noyalib::cst::{FormatConfig, format_with_config};
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = NoyafmtCli::parse();

    if !args.stdin && args.files.is_empty() {
        eprintln!("error: no FILE arguments and --stdin not given (use --help)");
        return ExitCode::from(2);
    }

    let cfg = FormatConfig {
        indent_size: args.indent,
    };

    if args.stdin {
        return run_stdin(&cfg);
    }

    let mut any_changed = false;
    let mut had_error = false;
    for file in &args.files {
        match run_file(file, &cfg, args.check, args.write) {
            Ok(changed) => any_changed |= changed,
            Err(e) => {
                eprintln!("{}: {}", file.display(), e);
                had_error = true;
            }
        }
    }

    if had_error {
        return ExitCode::from(1);
    }
    if args.check && any_changed {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn run_stdin(cfg: &FormatConfig) -> ExitCode {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("error: reading stdin: {e}");
        return ExitCode::from(1);
    }
    match format_with_config(&input, cfg) {
        Ok(formatted) => {
            if let Err(e) = io::stdout().write_all(formatted.as_bytes()) {
                eprintln!("error: writing stdout: {e}");
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

/// Returns `Ok(true)` when the file would change under formatting,
/// `Ok(false)` when it is already canonical.
fn run_file(
    file: &std::path::Path,
    cfg: &FormatConfig,
    check: bool,
    write: bool,
) -> io::Result<bool> {
    let input = fs::read_to_string(file)?;
    let formatted = match format_with_config(&input, cfg) {
        Ok(s) => s,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("parse: {e}"),
            ));
        }
    };
    let changed = formatted != input;
    if check {
        if changed {
            // rustfmt convention: print the path of each unformatted file
            // to stdout so `xargs` / shell pipelines can act on them.
            println!("{}", file.display());
        }
        return Ok(changed);
    }
    if write {
        if changed {
            fs::write(file, formatted.as_bytes())?;
        }
        return Ok(changed);
    }
    // Default: print formatted source to stdout.
    io::stdout().write_all(formatted.as_bytes())?;
    Ok(changed)
}
