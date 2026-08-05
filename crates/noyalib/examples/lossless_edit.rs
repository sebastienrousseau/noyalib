// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Renovate-style version bump that preserves comments, indentation,
//! and ordering byte-for-byte.
//!
//! When you read a YAML file with `from_str` and write it back with
//! `to_string`, comments and trivia are gone — the round-trip
//! travels through `Value`, which only carries data. For tooling
//! that *edits* YAML in place (formatters, linters, dependency
//! bumpers, k8s manifest patchers) you need lossless round-trip.
//!
//! `noyalib::cst::Document` solves that. It carries three
//! coordinated views of the same input — a green tree that
//! reproduces the source verbatim, a typed `Value` for data access,
//! and a span tree that maps any path back to a byte range. Edits
//! flow through `Document::set` (or `replace_span`) and untouched
//! bytes — indentation, comments, blank lines, sibling entries —
//! are preserved.
//!
//! Run with: `cargo run --example lossless_edit`

use noyalib::cst::parse_document;

fn main() {
    let input = "\
# Project manifest — keep these comments aligned with releases.
name: noyalib                 # crate name (also used as the binary)
version: 0.0.1                # bump per release
authors:
  - Sebastien Rousseau         # primary maintainer
dependencies:
  serde: 1.0.228               # serde-native deserialise
  indexmap: 2.12.0             # ordered mappings
";

    let mut doc = parse_document(input).expect("input is valid YAML");

    // Bump the version in place. Every other byte — comments, the
    // exact whitespace, even the trailing comment on the same line —
    // is left alone.
    doc.set("version", "0.0.2")
        .expect("`version` exists at the document root");

    println!("=== before ===\n{input}");
    println!("=== after ===\n{doc}");

    // Round-trip property: the only diff is the bumped value.
    let after = doc.to_string();
    assert!(after.contains("version: 0.0.2                # bump per release"));
    assert!(after.contains("# Project manifest — keep these comments aligned with releases."));
    assert!(after.contains("authors:\n  - Sebastien Rousseau         # primary maintainer"));

    // Path-targeted reads still work — useful for diff'ing or for
    // gating an edit on the current value.
    let dep_serde = doc.get("dependencies.serde").expect("path resolves");
    println!("dependencies.serde currently at: {dep_serde}");

    // Bumping a nested key with the typed setter chooses an
    // appropriate scalar style at the target site automatically.
    doc.set_value(
        "dependencies.indexmap",
        &noyalib::Value::String("2.13.0".into()),
    )
    .expect("`dependencies.indexmap` is a scalar leaf");

    let final_yaml = doc.to_string();
    println!("=== final ===\n{final_yaml}");
    assert!(final_yaml.contains("2.13.0"));
    assert!(final_yaml.contains("# ordered mappings"));
}
