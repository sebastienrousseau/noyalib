// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 2.3 — anchor / alias management ("Smart Aliases").
//!
//! Covers the full discovery surface (`anchors`, `aliases`,
//! `aliases_of`), the propagation contract (edits to anchored
//! values are visible at every alias site after re-load), and the
//! materialise primitives that "break" an alias by inlining the
//! anchored value's source text into the alias position.
//!
//! Block-valued anchors are explicitly out-of-scope for v0.0.1
//! materialisation — those return an actionable error and the
//! source is left untouched. The expectation is that callers fall
//! back to `Document::anchors()` + `Document::replace_span()` for
//! manual block splicing.

#![allow(missing_docs)]

use noyalib::cst::parse_document;

// ── Discovery ───────────────────────────────────────────────────────

#[test]
fn anchors_and_aliases_in_one_doc() {
    let src = "\
defaults: &cfg
  port: 8080
service:
  <<: *cfg
  host: localhost
backup: *cfg
";
    let doc = parse_document(src).unwrap();
    let anchors = doc.anchors();
    let aliases = doc.aliases();
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].name, "cfg");
    assert_eq!(aliases.len(), 2);
    assert!(aliases.iter().all(|a| a.name == "cfg"));
}

#[test]
fn anchor_mark_span_is_only_the_lexeme() {
    let src = "x: &name 1\n";
    let doc = parse_document(src).unwrap();
    let a = &doc.anchors()[0];
    assert_eq!(&src[a.mark_span.0..a.mark_span.1], "&name");
}

#[test]
fn alias_mark_span_is_only_the_lexeme() {
    let src = "x: &name 1\ny: *name\n";
    let doc = parse_document(src).unwrap();
    let a = &doc.aliases()[0];
    assert_eq!(&src[a.mark_span.0..a.mark_span.1], "*name");
}

#[test]
fn aliases_of_unknown_anchor_is_empty_not_error() {
    // Discovery is read-only — looking up a non-existent anchor
    // returns an empty list, not an error. That matches the typical
    // "is this anchor referenced anywhere?" check pattern.
    let src = "x: 1\ny: 2\n";
    let doc = parse_document(src).unwrap();
    assert!(doc.aliases_of("ghost").is_empty());
}

#[test]
fn many_anchors_preserve_source_order() {
    let src = "a: &one 1\nb: &two 2\nc: &three 3\n";
    let doc = parse_document(src).unwrap();
    let anchors = doc.anchors();
    let names: Vec<&str> = anchors.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, vec!["one", "two", "three"]);
    let positions: Vec<usize> = anchors.iter().map(|a| a.mark_span.0).collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]));
}

// ── Propagation contract — edits propagate via re-load ──────────────

#[test]
fn editing_anchored_value_propagates_to_alias_sites() {
    // The contract documented in the module rustdoc: when the user
    // mutates the anchored value via `set`, every alias site sees
    // the new value when the typed Value tree is read.
    let src = "\
defaults: &cfg
  port: 8080
  host: localhost
primary:
  <<: *cfg
secondary:
  <<: *cfg
";
    let mut doc = parse_document(src).unwrap();
    doc.set("defaults.port", "9090").unwrap();

    let v = doc.as_value();
    assert_eq!(v["defaults"]["port"].as_i64(), Some(9090));
    assert_eq!(v["primary"]["port"].as_i64(), Some(9090));
    assert_eq!(v["secondary"]["port"].as_i64(), Some(9090));

    // The source still shows one anchor and two aliases — only the
    // anchored value's bytes changed, not the wiring.
    assert_eq!(doc.anchors().len(), 1);
    assert_eq!(doc.aliases().len(), 2);
}

#[test]
fn editing_anchored_value_preserves_byte_faithful_outside_target() {
    let src = "\
# project defaults
defaults: &cfg
  port: 8080  # bound port
service:
  <<: *cfg
  host: localhost
";
    let mut doc = parse_document(src).unwrap();
    doc.set("defaults.port", "9090").unwrap();

    let out = doc.to_string();
    assert!(out.contains("# project defaults"));
    assert!(out.contains("# bound port"));
    assert!(out.contains("&cfg"));
    assert!(out.contains("*cfg"));
    assert!(out.contains("port: 9090"));
}

// ── Materialise — scalar paths ──────────────────────────────────────

#[test]
fn materialise_inlines_scalar_anchor_text() {
    let src = "a: &n 7\nb: *n\n";
    let mut doc = parse_document(src).unwrap();
    let alias_pos = doc.aliases()[0].mark_span.0;
    doc.materialise_alias_at(alias_pos).unwrap();

    let out = doc.to_string();
    assert_eq!(out, "a: &n 7\nb: 7\n");
    assert!(doc.aliases().is_empty());
    // The anchor itself is untouched — it still labels its value.
    assert_eq!(doc.anchors().len(), 1);
}

#[test]
fn materialise_with_double_quoted_scalar_keeps_quotes() {
    // Style preservation: the materialised text is the *source
    // bytes* of the anchored value, including quoting.
    let src = "a: &greet \"hello world\"\nb: *greet\n";
    let mut doc = parse_document(src).unwrap();
    let pos = doc.aliases()[0].mark_span.0;
    doc.materialise_alias_at(pos).unwrap();
    assert_eq!(
        doc.to_string(),
        "a: &greet \"hello world\"\nb: \"hello world\"\n"
    );
}

#[test]
fn materialise_with_single_quoted_scalar_keeps_quotes() {
    let src = "a: &g 'O''Brien'\nb: *g\n";
    let mut doc = parse_document(src).unwrap();
    let pos = doc.aliases()[0].mark_span.0;
    doc.materialise_alias_at(pos).unwrap();
    assert_eq!(doc.to_string(), "a: &g 'O''Brien'\nb: 'O''Brien'\n");
}

#[test]
fn materialise_aliases_of_handles_three_in_one_call() {
    let src = "a: &x 7\nb: *x\nc: *x\nd: *x\n";
    let mut doc = parse_document(src).unwrap();
    let n = doc.materialise_aliases_of("x").unwrap();
    assert_eq!(n, 3);
    let out = doc.to_string();
    assert!(!out.contains('*'));
    assert_eq!(out, "a: &x 7\nb: 7\nc: 7\nd: 7\n");
}

#[test]
fn materialise_one_does_not_break_others() {
    let src = "a: &x 1\nb: *x\nc: *x\n";
    let mut doc = parse_document(src).unwrap();
    let first_alias = doc.aliases()[0].mark_span.0;
    doc.materialise_alias_at(first_alias).unwrap();

    let out = doc.to_string();
    // First alias inlined; second alias still pointing at &x.
    assert_eq!(out, "a: &x 1\nb: 1\nc: *x\n");
    assert_eq!(doc.aliases_of("x").len(), 1);
}

// ── Materialise — error paths ───────────────────────────────────────

#[test]
fn materialise_block_anchor_errors_and_leaves_source_unchanged() {
    let src = "\
defaults: &cfg
  port: 8080
  host: localhost
service: *cfg
";
    let mut doc = parse_document(src).unwrap();
    let pos = doc.aliases()[0].mark_span.0;
    let err = doc.materialise_alias_at(pos).unwrap_err();
    let msg = err.to_string();
    // Error must point at the limitation and the workaround.
    assert!(msg.contains("multi-line"));
    assert!(msg.contains("scalar-valued"));
    assert!(msg.contains("replace_span"));
    // Source must be untouched on error.
    assert_eq!(doc.to_string(), src);
    assert_eq!(doc.anchors().len(), 1);
    assert_eq!(doc.aliases().len(), 1);
}

#[test]
fn materialise_unknown_position_errors_clearly() {
    let mut doc = parse_document("a: &x 1\nb: *x\n").unwrap();
    let err = doc.materialise_alias_at(0).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no alias mark begins at byte 0"));
}

// ── rename_anchor — atomic anchor + alias refactor ──────────────────

#[test]
fn rename_anchor_updates_declaration_and_all_aliases() {
    let src = "\
defaults: &cfg
  port: 8080
service:
  <<: *cfg
backup: *cfg
";
    let mut doc = parse_document(src).unwrap();
    let n = doc.rename_anchor("cfg", "defaults").unwrap();
    assert_eq!(n, 3, "1 anchor + 2 aliases");
    let out = doc.to_string();
    assert!(!out.contains("&cfg"));
    assert!(!out.contains("*cfg"));
    assert!(out.contains("&defaults"));
    assert!(out.contains("*defaults"));
}

#[test]
fn rename_anchor_with_only_declaration_no_aliases() {
    let src = "x: &solo 1\ny: 2\n";
    let mut doc = parse_document(src).unwrap();
    let n = doc.rename_anchor("solo", "renamed").unwrap();
    assert_eq!(n, 1);
    assert!(doc.to_string().contains("&renamed"));
}

#[test]
fn rename_anchor_unknown_name_errors() {
    let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
    let err = doc.rename_anchor("ghost", "newname").unwrap_err();
    assert!(err.to_string().contains("no `&ghost`"));
}

#[test]
fn rename_anchor_invalid_target_name_errors() {
    let mut doc = parse_document("x: &cfg 1\ny: *cfg\n").unwrap();
    // Whitespace and flow indicators are forbidden in YAML anchor
    // names per §6.9.2 — the rename must reject them upfront.
    let err = doc.rename_anchor("cfg", "has space").unwrap_err();
    assert!(err.to_string().contains("not a valid YAML anchor name"));
    let err2 = doc.rename_anchor("cfg", "").unwrap_err();
    assert!(err2.to_string().contains("not a valid YAML anchor name"));
    let err3 = doc.rename_anchor("cfg", "with[brackets").unwrap_err();
    assert!(err3.to_string().contains("not a valid YAML anchor name"));
}

#[test]
fn rename_anchor_preserves_byte_faithful_around_marks() {
    let src = "\
# top-level comment
defaults: &cfg
  port: 8080  # bound port
service:
  <<: *cfg
  host: api.example.com

backup: *cfg
";
    let mut doc = parse_document(src).unwrap();
    let _ = doc.rename_anchor("cfg", "shared").unwrap();
    let out = doc.to_string();
    assert!(out.contains("# top-level comment"));
    assert!(out.contains("# bound port"));
    assert!(out.contains("host: api.example.com"));
    assert!(out.contains("&shared"));
    assert!(out.contains("*shared"));
}

#[test]
fn rename_anchor_does_not_touch_unrelated_anchors() {
    let src = "\
a: &one 1
b: &two 2
c: *one
d: *two
";
    let mut doc = parse_document(src).unwrap();
    let n = doc.rename_anchor("one", "first").unwrap();
    assert_eq!(n, 2);
    let out = doc.to_string();
    // `one` and its aliases are renamed.
    assert!(!out.contains("&one"));
    assert!(!out.contains("*one"));
    assert!(out.contains("&first"));
    assert!(out.contains("*first"));
    // `two` is untouched.
    assert!(out.contains("&two"));
    assert!(out.contains("*two"));
}

#[test]
fn rename_anchor_resolves_to_same_value_after() {
    let src = "\
defaults: &cfg
  port: 8080
service:
  <<: *cfg
";
    let mut doc = parse_document(src).unwrap();
    let _ = doc.rename_anchor("cfg", "shared").unwrap();
    // The merged value at service.port still resolves correctly.
    let v = doc.as_value();
    assert_eq!(v["service"]["port"].as_i64(), Some(8080));
}

// ── Edits over anchor-decorated regions stay byte-faithful ──────────

#[test]
fn anchor_marks_unchanged_after_unrelated_edit() {
    let src = "\
flags: &flags
  debug: false
other: 1
";
    let mut doc = parse_document(src).unwrap();
    let before = doc.anchors();
    doc.set("other", "2").unwrap();
    let after = doc.anchors();
    assert_eq!(before, after);
    assert!(doc.to_string().contains("&flags"));
    assert!(doc.to_string().contains("other: 2"));
}

// ── Regression: set/set_value must not write through an alias ────────

#[test]
fn set_through_alias_is_refused_not_written_to_the_anchor() {
    // v0.0.14 review regression. `span_at` resolves an alias reference
    // *through* to its anchor's value span (issue #149) — the correct
    // target for a read. `set` / `set_value` reused that span for writes,
    // so `set("ref", …)` spliced over the ANCHOR's value (`foo` on the
    // `base:` line), silently corrupting a *different* key and leaving the
    // alias untouched. They must refuse the edit instead.
    let mut doc = parse_document("base: &a foo\nref: *a\n").unwrap();
    let before = doc.to_string();

    let err = doc.set("ref", "bar");
    assert!(
        err.is_err(),
        "set through an alias must be refused, not silently mis-applied to the anchor"
    );
    assert_eq!(
        doc.to_string(),
        before,
        "a refused set must leave the document byte-for-byte unchanged (no anchor corruption)"
    );

    // set_value shares the hazard and the guard.
    let err = doc.set_value("ref", &noyalib::Value::String("bar".into()));
    assert!(err.is_err());
    assert_eq!(doc.to_string(), before);

    // A plain (non-aliased) sibling value is still writable — the guard is
    // scoped to alias targets, it does not block ordinary edits.
    let mut doc2 = parse_document("base: &a foo\nplain: keep\n").unwrap();
    doc2.set("plain", "changed").unwrap();
    assert!(doc2.to_string().contains("plain: changed"));
}

#[test]
fn set_through_aliased_sequence_item_is_refused() {
    // The same hazard exists for an alias used as a sequence item.
    let mut doc = parse_document("anchor: &a foo\nlist:\n  - *a\n").unwrap();
    let before = doc.to_string();
    assert!(doc.set("list.0", "bar").is_err());
    assert_eq!(doc.to_string(), before);
}

// ── Regression: alias-write guard must cover undecodable (quoted) keys ──

#[test]
fn set_through_double_quoted_alias_key_is_refused() {
    // ultrareview BUG003. The alias-write guard matched keys via
    // `entry_key_text`, which can't decode double-quoted keys, so
    // `set("target_key", …)` on a double-quoted alias entry slipped past
    // the guard and spliced the ANCHOR's value — silent corruption of a
    // different key. Must be refused.
    let src = "anchor_key: &a foo\n\"target_key\": *a\n";
    let mut doc = parse_document(src).unwrap();
    let before = doc.to_string();
    assert!(
        doc.set("target_key", "bar").is_err(),
        "set through a double-quoted alias key must be refused"
    );
    assert_eq!(
        doc.to_string(),
        before,
        "document must be unchanged (no anchor corruption)"
    );
}

#[test]
fn set_double_quoted_key_with_plain_value_still_works() {
    // No regression: an ordinary double-quoted key whose value is NOT an
    // alias is still writable — the guard only fires on alias values.
    let mut doc = parse_document("\"my key\": old\nplain: keep\n").unwrap();
    doc.set("my key", "new").unwrap();
    assert!(
        doc.to_string().contains("\"my key\": new"),
        "got {:?}",
        doc.to_string()
    );
    assert!(doc.to_string().contains("plain: keep"));
}

// ── Alias-write guard: resolution-layer coverage (ultrareview #2) ──────
// The guard now keys off resolve_span's through-alias flag (the same
// resolution span_at uses), closing four holes the green-tree detector
// missed: flow collections, nested descent, mixed duplicate keys, and a
// false-positive refusal.

#[test]
fn set_through_alias_in_flow_sequence_is_refused() {
    // HOLE1: flow collections keep flat tokens (no SequenceItem nodes), so
    // the old green-walk missed them and set spliced the anchor.
    let mut doc = parse_document("a: &x foo\nb: [*x, 2]\n").unwrap();
    let before = doc.to_string();
    assert!(doc.set("b[0]", "bar").is_err());
    assert_eq!(doc.to_string(), before, "anchor must be untouched");
}

#[test]
fn set_nested_through_alias_is_refused() {
    // HOLE2: `ref.foo` descends through the alias into the anchor's mapping.
    let mut doc = parse_document("anchor: &a\n  foo: 1\n  bar: 2\nref: *a\n").unwrap();
    let before = doc.to_string();
    assert!(doc.set("ref.foo", "10").is_err());
    assert_eq!(
        doc.to_string(),
        before,
        "anchor's nested value must be untouched"
    );
}

#[test]
fn set_duplicate_key_last_is_alias_is_refused() {
    // HOLE3: `key: value1` then `"key": *a` — DuplicateKeyPolicy::Last makes
    // the resolved value the alias entry, even though a decodable plain entry
    // also matched the key.
    let mut doc = parse_document("anchor_key: &a foo\nkey: value1\n\"key\": *a\n").unwrap();
    let before = doc.to_string();
    assert!(doc.set("key", "bar").is_err());
    assert_eq!(doc.to_string(), before, "anchor must be untouched");
}

#[test]
fn set_quoted_key_with_plain_value_beside_alias_sibling_still_works() {
    // HOLE4: a plain-valued double-quoted key must remain writable even when
    // an unrelated sibling is an alias (the old boolean-flag guard wrongly
    // refused this).
    let mut doc =
        parse_document("anchor: &a foo\n\"my key\": old\n\"other quoted\": *a\n").unwrap();
    doc.set("my key", "new").unwrap();
    assert!(
        doc.to_string().contains("\"my key\": new"),
        "got {:?}",
        doc.to_string()
    );
    assert!(doc.to_string().contains("&a foo"), "anchor unchanged");
}
