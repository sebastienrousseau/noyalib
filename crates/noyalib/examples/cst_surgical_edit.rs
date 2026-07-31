// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The full CST surgical-edit API on one document.
//!
//! `noyalib::cst::Document` edits YAML *in place*: every mutator
//! rewrites only the bytes it must and leaves comments, indentation,
//! blank lines and sibling entries byte-for-byte intact. The
//! re-parse-guarded mutators (`rename_key`, `swap_items`, `move_item`,
//! `rename_anchor`, the comment setters, and multi-line `remove`)
//! additionally verify the edit against a typed oracle and roll back on
//! any mismatch, so a bad edit can never corrupt the document.
//!
//! This example applies every mutator to a single realistic config and
//! asserts the byte-exact result at each step, so it doubles as living
//! documentation of the edit surface.
//!
//! Run with: `cargo run --example cst_surgical_edit`

use noyalib::cst::parse_document;

fn main() {
    // ── set / set_value: rewrite a scalar, comments preserved ────────
    let mut doc = parse_document("service: api\nport: 8080  # bumped for the load test\n").unwrap();
    doc.set("port", "9090").unwrap();
    assert_eq!(
        doc.source(),
        "service: api\nport: 9090  # bumped for the load test\n"
    );
    println!("set            → {:?}", doc.source());

    // ── read accessors: span_at (value) and key_span (key token) ─────
    let (vs, ve) = doc.span_at("port").unwrap();
    assert_eq!(&doc.source()[vs..ve], "9090");
    let (ks, ke) = doc.key_span("port").unwrap();
    assert_eq!(&doc.source()[ks..ke], "port");
    println!("span_at/key_span → value {:?}, key {:?}", "9090", "port");

    // ── rename_key: key spelling matched, value + comment untouched ──
    doc.rename_key("port", "listen_port").unwrap();
    assert_eq!(
        doc.source(),
        "service: api\nlisten_port: 9090  # bumped for the load test\n"
    );
    println!("rename_key     → {:?}", doc.source());

    // ── insert_entry / push_back / insert_after: grow collections ────
    let mut doc = parse_document("features:\n  - auth\n  - api\n").unwrap();
    doc.push_back("features", "metrics").unwrap();
    assert_eq!(doc.source(), "features:\n  - auth\n  - api\n  - metrics\n");
    doc.insert_after("features[0]", "tracing").unwrap();
    assert_eq!(
        doc.source(),
        "features:\n  - auth\n  - tracing\n  - api\n  - metrics\n"
    );
    doc.insert_entry("", "enabled", "true").unwrap();
    assert!(doc.source().contains("enabled: true"));
    println!("push/insert    → {:?}", doc.source());

    // ── the *_value tier: same growth, but the data stays data ───────
    //
    // The three mutators above splice their fragment verbatim, so a
    // fragment that looks like YAML syntax *becomes* syntax. Their
    // `_value` counterparts emit the spelling that re-parses to the
    // value given — quoting when the plain form would not — and hold
    // the splice to a typed-value check afterwards.
    let mut doc = parse_document("features:\n  - auth\n").unwrap();
    doc.push_back_value("features", "- not an item").unwrap();
    assert_eq!(
        doc.source(),
        "features:\n  - auth\n  - \"- not an item\"\n",
        "the dash is data, not a nested sequence",
    );
    doc.insert_entry_value("", "version", "8080").unwrap();
    assert!(
        doc.source().contains("version: \"8080\""),
        "quoted, or it would load as the number 8080",
    );
    assert_eq!(doc.as_value()["version"], noyalib::Value::from("8080"));
    println!("*_value        → {:?}", doc.source());

    // ── swap_items / move_item: reorder a block sequence ─────────────
    let mut doc = parse_document("order:\n  - first\n  - second\n  - third\n").unwrap();
    doc.swap_items("order", 0, 2).unwrap();
    assert_eq!(doc.source(), "order:\n  - third\n  - second\n  - first\n");
    doc.move_item("order", 2, 0).unwrap();
    assert_eq!(doc.source(), "order:\n  - first\n  - third\n  - second\n");
    println!("swap/move      → {:?}", doc.source());

    // ── remove: multi-line / nested block value, whole entry gone ────
    let mut doc =
        parse_document("keep: 1\nserver:\n  host: 0.0.0.0\n  port: 8080\ntrailer: 2\n").unwrap();
    doc.remove("server").unwrap();
    assert_eq!(doc.source(), "keep: 1\ntrailer: 2\n");
    println!("remove(nested) → {:?}", doc.source());

    // ── comment mutation: inline and leading blocks ─────────────────
    let mut doc = parse_document("port: 8080\n").unwrap();
    doc.set_inline_comment("port", "the listen port").unwrap();
    assert_eq!(doc.source(), "port: 8080  # the listen port\n");
    doc.set_leading_comment("port", "network\nconfiguration")
        .unwrap();
    assert_eq!(
        doc.source(),
        "# network\n# configuration\nport: 8080  # the listen port\n"
    );
    // Read them back through the typed comment view, then clear them.
    let bundle = doc.comments_at("port");
    assert_eq!(bundle.before.len(), 2);
    assert_eq!(bundle.inline.as_ref().unwrap().text, " the listen port");
    doc.remove_inline_comment("port").unwrap();
    doc.remove_leading_comment("port").unwrap();
    assert_eq!(doc.source(), "port: 8080\n");
    println!("comments       → set, read, removed cleanly");

    // ── rename_anchor: rename a `&declaration` and every `*alias` ────
    // (including one inside a `<<` merge) in one atomic edit ──────────
    let mut doc =
        parse_document("defaults: &cfg\n  port: 8080\nservice:\n  <<: *cfg\nbackup: *cfg\n")
            .unwrap();
    let renamed = doc.rename_anchor("cfg", "shared").unwrap();
    assert_eq!(renamed, 3); // 1 anchor + 2 aliases (one via `<<`)
    assert_eq!(
        doc.source(),
        "defaults: &shared\n  port: 8080\nservice:\n  <<: *shared\nbackup: *shared\n"
    );
    // Renaming onto a name another anchor already uses is refused — it
    // would change which value the aliases resolve to.
    let mut clash = parse_document("x: &a 1\ny: &b 2\nz: *a\n").unwrap();
    assert!(clash.rename_anchor("a", "b").is_err());
    println!("rename_anchor  → {:?}", doc.source());

    // ── the guard refuses a data-changing edit and rolls back ────────
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    // Renaming `a` to an existing sibling `b` would duplicate a key.
    assert!(doc.rename_key("a", "b").is_err());
    assert_eq!(doc.source(), "a: 1\nb: 2\n"); // untouched
    println!("guard          → refused a duplicate-key rename, doc intact");

    println!("\nAll surgical edits preserved every untouched byte.");
}
