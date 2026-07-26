// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::comments_at(path)` — read the YAML comments that
//! decorate a node, classified into leading and inline.
//!
//! The motivating use case: an AI agent or linter needs to understand
//! what a config field *means*. The doc-comment authored next to the
//! value is the natural source. With this API, the agent reads the
//! value AND the human's annotation in a single call — and edits via
//! `Document::set` round-trip both untouched.
//!
//! Run: `cargo run --example comments_at`

use noyalib::cst::parse_document;

fn main() {
    let src = "\
# noyalib service config — Renovate bumps `version`,\n\
# everything else is owned by the platform team.\n\
\n\
name: noyalib  # the project\n\
version: 0.0.1  # do not edit by hand\n\
\n\
# server section is read by the bootstrapper\n\
server:\n  \
  host: localhost  # bind address\n  \
  port: 8080       # main HTTP port\n\
\n\
features:\n  \
  - auth   # OIDC + passkeys\n  \
  - api    # public REST surface\n\
";

    let doc = parse_document(src).expect("parse");

    println!("── reading comments ─────────────────────────────────────");
    for path in [
        "name",
        "version",
        "server",
        "server.host",
        "server.port",
        "features[0]",
        "features[1]",
    ] {
        let b = doc.comments_at(path);
        println!("{path}:");
        for c in &b.before {
            println!("  before: #{}", c.text);
        }
        if let Some(c) = &b.inline {
            println!("  inline: #{}", c.text);
        }
        if b.is_empty() {
            println!("  (no comments)");
        }
    }

    println!();
    println!("── lossless edit + comments survive ─────────────────────");
    let mut doc = doc;
    doc.set("version", "0.0.2").expect("edit");
    let b = doc.comments_at("version");
    println!(
        "after `set version=0.0.2`, version's inline comment is still: #{}",
        b.inline.as_ref().unwrap().text
    );
    println!();
    println!("── full document after edit ─────────────────────────────");
    println!("{}", doc);
}
