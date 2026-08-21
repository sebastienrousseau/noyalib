// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Merge keys (`<<:`) and aliases (`*name`) in the same mapping.
//!
//! A merge key pulls another mapping's entries in; an alias substitutes an
//! anchored node. Both are ordinary YAML, and they compose — a key written
//! after a `<<:` may take an alias as its value, and overrides the merged
//! entry of the same name:
//!
//! ```yaml
//! defaults: &defaults
//!   retries: 3
//!   timeout: 30
//!
//! long_timeout: &long 300
//!
//! job:
//!   <<: *defaults      # retries: 3, timeout: 30
//!   timeout: *long     # ...then timeout becomes 300
//! ```
//!
//! That last line used to deserialise as the string `"long"` rather than
//! `300`. The streaming deserializer resolved aliases only when reading
//! straight from the parser, and after a merge the remaining entries arrive
//! through a replay stack instead — so the alias came back unresolved
//! (#301, found by @mathstuf). The ordering was the whole tell: the same
//! document with `timeout: *long` written *before* the `<<:` line worked.
//!
//! Run with: `cargo run --example merge_keys_with_aliases`

use noyalib::{Value, from_str};
use std::collections::BTreeMap;

/// Deserialise just the `job` mapping and print it, so the resolved values
/// are visible rather than merely asserted. The sibling keys are anchors of
/// mixed shape (a mapping and a scalar), so only `job` is captured.
#[derive(serde::Deserialize)]
struct Doc {
    job: BTreeMap<String, i64>,
}

fn show(label: &str, yaml: &str) {
    let doc: Doc = from_str(yaml).unwrap_or_else(|e| panic!("{label}: {e}"));
    let rendered: Vec<String> = doc.job.iter().map(|(k, v)| format!("{k}={v}")).collect();
    println!("  {:<44} {}", label, rendered.join("  "));
}

fn main() {
    let defaults = "defaults: &defaults\n  retries: 3\n  timeout: 30\nlong: &long 300\n";

    println!("A merged mapping, and an alias overriding one of its keys:\n");

    show(
        "alias after the merge key",
        &format!("{defaults}job:\n  <<: *defaults\n  timeout: *long\n"),
    );
    show(
        "alias before the merge key",
        &format!("{defaults}job:\n  timeout: *long\n  <<: *defaults\n"),
    );
    show(
        "no merge key at all",
        &format!("{defaults}job:\n  retries: 3\n  timeout: *long\n"),
    );

    println!("\nAll three agree — order does not change the result.\n");

    println!("Aliases to collections work the same way:\n");

    let nested = concat!(
        "base: &base\n  name: build\n",
        "matrix: &matrix\n  - linux\n  - macos\n",
        "job:\n  <<: *base\n  targets: *matrix\n",
    );
    let v: Value = from_str(nested).expect("parse");
    println!("  job.name    = {:?}", v["job"]["name"]);
    println!("  job.targets = {:?}", v["job"]["targets"]);

    println!("\nAn anchor defined after a merge is still replayable:\n");

    let after = concat!(
        "base: &base\n  x: 1\n",
        "job:\n  <<: *base\n  spec: &spec\n    cpu: 2\n",
        "copy: *spec\n",
    );
    let v: Value = from_str(after).expect("parse");
    println!("  copy = {:?}", v["copy"]);
    assert_eq!(
        v["copy"].as_mapping().map(noyalib::Mapping::len),
        Some(1),
        "recording an event twice would show up here as extra entries"
    );

    println!("\nOnly a *plain* `<<` is a merge key:\n");

    // The YAML merge type gives `tag:yaml.org,2002:merge` to a plain `<<`.
    // A quoted one resolves to `...:str`, as does an alias that happens to
    // point at the string — both are ordinary keys whose value is whatever
    // they were given.
    for (label, doc) in [
        ("plain      <<: *base", "base: &base\n  x: 1\nout:\n  <<: *base\n"),
        ("quoted   \"<<\": 1", "out:\n  \"<<\": 1\n  x: 1\n"),
        ("alias to \"<<\"", "k: &k \"<<\"\nout:\n  *k : 1\n  x: 1\n"),
        ("alias to plain <<", "k: &k <<\nout:\n  *k : 1\n  x: 1\n"),
    ] {
        let v: Value = from_str(doc).expect("parse");
        let merged = v["out"].get("<<").is_none();
        println!(
            "  {:<22} -> {:<34} {}",
            label,
            format!("{:?}", v["out"]),
            if merged { "merged" } else { "ordinary key" }
        );
    }

    println!("\nA merge target must be an alias, or a sequence of them:\n");
    for bad in ["job:\n  <<: 1\n", "job:\n  <<: *missing\n"] {
        let err = from_str::<Value>(bad).unwrap_err();
        println!(
            "  {:<28} -> {}",
            bad.lines().nth(1).unwrap_or("").trim(),
            err
        );
    }
}
