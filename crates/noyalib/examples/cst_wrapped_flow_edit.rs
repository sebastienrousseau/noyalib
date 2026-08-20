// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Editing a flow collection that is wrapped across several lines.
//!
//! Hand-written config wraps flow collections one member per line, which
//! reads well and diffs well:
//!
//! ```yaml
//! ports: [
//!   80,
//!   443,
//! ]
//! ```
//!
//! Removing a member has to take the member's *line* with it, not just
//! the member. Leaving the indentation behind produces a line holding
//! nothing but spaces — the value still round-trips, so nothing is
//! corrupt, but `git diff --check`, `yamllint`'s `trailing-spaces` and
//! `editorconfig-checker` all reject that patch. A library whose promise
//! is that an edit touches only what the path names should not hand its
//! caller a diff their own lint refuses. That was #294, reported by
//! @zoosky.
//!
//! The rule is **"the member is alone on its line"**, not "the collection
//! is wrapped". Anything else still holding the line keeps it standing,
//! and this example shows both sides of that line.
//!
//! Run with: `cargo run --example cst_wrapped_flow_edit`

use noyalib::cst::parse_document;

/// Assert an edit's exact bytes, and that it left no line holding only
/// whitespace — the invariant the issue was about.
fn edit(label: &str, src: &str, path: &str, want: &str) {
    let mut doc = parse_document(src).expect("parse");
    doc.remove(path).expect("remove");
    assert_eq!(doc.source(), want, "{label}");
    for (n, line) in doc.source().lines().enumerate() {
        assert!(
            line.is_empty() || !line.trim().is_empty(),
            "{label}: line {} is whitespace-only",
            n + 1
        );
    }
    println!("  {label}");
    for line in want.lines() {
        println!("      {}", line.replace(' ', "·"));
    }
}

fn main() {
    println!("The member owns its line — the line goes with it:\n");

    edit(
        "remove ports[0] from a wrapped sequence",
        "ports: [\n  80,\n  443,\n]\n",
        "ports[0]",
        "ports: [\n  443,\n]\n",
    );

    edit(
        "remove cfg.a from a wrapped mapping",
        "cfg: {\n  a: 1,\n  b: 2,\n}\n",
        "cfg.a",
        "cfg: {\n  b: 2,\n}\n",
    );

    println!("\nSomething else holds the line — it stays standing:\n");

    edit(
        "the opening indicator shares the line",
        "ports: [80,\n  443,\n]\n",
        "ports[0]",
        "ports: [\n  443,\n]\n",
    );

    edit(
        "a sibling member shares the line",
        "ports: [\n  80, 443,\n  8080,\n]\n",
        "ports[0]",
        "ports: [\n  443,\n  8080,\n]\n",
    );

    edit(
        "a trailing comment shares the line",
        "ports: [\n  80, # why this port\n  443,\n]\n",
        "ports[0]",
        "ports: [\n  # why this port\n  443,\n]\n",
    );

    edit(
        "single-line collections are untouched",
        "ports: [80, 443]\n",
        "ports[0]",
        "ports: [443]\n",
    );

    println!("\nThe block path has always answered this the same way:\n");

    edit(
        "a block sequence item takes its line too",
        "ports:\n  - 80\n  - 443\n",
        "ports[0]",
        "ports:\n  - 443\n",
    );

    println!("\n(· marks a space, so a stray indent would be visible above.)");
    println!("All edits byte-exact, no whitespace-only lines.");
}
