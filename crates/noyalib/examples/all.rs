// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Run all noyalib examples in sequence.
//!
//! Usage: `cargo run --example run_all`

use std::process::Command;
use std::time::Instant;

const EXAMPLES: &[&str] = &[
    // Core
    "hello",
    "std",
    "variants",
    "deep",
    "dynamic",
    "modify",
    "tags", // Spec
    "alias",
    "smart",
    "overlay",
    "inherit",
    "stream",
    "types",
    "binary",
    // Logic & Security
    "strict",
    "secure",
    "schema",
    "env", // DX
    "errors",
    "trace",
    "source",
    "style", // Advanced
    "emit",
    "rename",
    "flatten",
    "bridge",
    "pipes",
    "global",
    // Future-Proof
    "portable",
    "mask",
    "patch",
    "suggest",
    "schema_ext",
    // Deep Rust
    "untagged",
    "borrow",
    "transcode",
    "comments",
    // Platform
    "diagnostic",
    "nostd",
    "preserve",
    // Competitive Features
    "replay",
    "registry",
    "scientific",
    "validation",
    "anchor_shared",
    // Final
    "async_io",
    "recursive",
    // v0.0.5 — pluggable error formatting + declarative config macros
    "i18n_formatters",
    "config_macros",
    // Bench (last — longest)
    "bench",
];

// Feature-gated examples that aren't run by this umbrella because
// each one needs its own `--features X` flag. Listed here as a
// pointer for anyone reading this file:
//
//   cargo run --example schema_validation     --features validate-schema
//   cargo run --example figment               --features figment
//   cargo run --example validation_garde      --features garde
//   cargo run --example validation_validator  --features validator
//   cargo run --example robotics_polymorphism --features robotics
//   cargo run --example ariadne_diagnostic    --features ariadne
//   cargo run --example validated_miette      --features miette,garde
//   cargo run --example include_directive     --features include_fs
//   cargo run --example recovery_lenient      --features recovery
//   cargo run --example sval_streaming        --features sval
//   cargo run --example tokio_async_reader    --features tokio
//   cargo run --example lossless_u64          --features lossless-u64

fn main() {
    println!("\n  \x1b[1mnoyalib examples\x1b[0m\n");

    let start = Instant::now();
    let mut passed = 0;
    let mut failed = 0;

    for name in EXAMPLES {
        print!("  \x1b[90m{name:<28}\x1b[0m");

        let result = Command::new("cargo")
            .args(["run", "--example", name, "--quiet"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match result {
            Ok(status) if status.success() => {
                println!("\x1b[32mdone\x1b[0m");
                passed += 1;
            }
            _ => {
                println!("\x1b[31mfail\x1b[0m");
                failed += 1;
            }
        }
    }

    let elapsed = start.elapsed();
    println!();
    if failed == 0 {
        println!(
            "  \x1b[1;32m{passed} examples passed\x1b[0m \x1b[90m({:.1}s)\x1b[0m\n",
            elapsed.as_secs_f64()
        );
    } else {
        println!(
            "  \x1b[1;31m{failed} failed\x1b[0m, {passed} passed \x1b[90m({:.1}s)\x1b[0m\n",
            elapsed.as_secs_f64()
        );
        std::process::exit(1);
    }
}
