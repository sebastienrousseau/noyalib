// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Document::rename_key` — first-class mapping-key rename.
//!
//! Each test parses a YAML document, renames one mapping key via
//! [`noyalib::cst::Document::rename_key`], and checks that the
//! result is byte-identical to the expected output — the `:`, the
//! value, indentation, comments, and sibling entries are preserved
//! verbatim. Refusal tests additionally assert the document is left
//! untouched.

#![allow(missing_docs)]

use noyalib::cst::{parse_document, parse_stream};

// ── Happy paths ─────────────────────────────────────────────────────

#[test]
fn rename_key_in_simple_block_mapping() {
    let mut doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
    doc.rename_key("name", "title").unwrap();
    assert_eq!(doc.to_string(), "title: foo\nversion: 0.0.1\n");
    let v = doc.as_value();
    assert_eq!(v["title"].as_str(), Some("foo"));
    assert!(v.as_mapping().unwrap().get("name").is_none());
}

#[test]
fn rename_key_preserves_comments_and_blank_lines() {
    let src = "# heading\nname: foo  # inline\n\nversion: 0.0.1\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("name", "title").unwrap();
    assert_eq!(
        doc.to_string(),
        "# heading\ntitle: foo  # inline\n\nversion: 0.0.1\n"
    );
}

#[test]
fn rename_key_in_nested_mapping() {
    let src = "package:\n  name: foo\n  version: 0.0.1\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("package.version", "ver").unwrap();
    assert_eq!(doc.to_string(), "package:\n  name: foo\n  ver: 0.0.1\n");
    assert_eq!(doc.as_value()["package"]["ver"].as_str(), Some("0.0.1"));
}

#[test]
fn rename_key_whose_value_is_a_nested_block_mapping() {
    let src = "server:\n  host: localhost\n  port: 8080\nother: 1\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("server", "backend").unwrap();
    assert_eq!(
        doc.to_string(),
        "backend:\n  host: localhost\n  port: 8080\nother: 1\n"
    );
    assert_eq!(doc.as_value()["backend"]["port"].as_i64(), Some(8080));
}

#[test]
fn rename_key_preserves_everything_outside_the_key_token() {
    // Byte-identity check: splice out the key token from the input
    // and the output — the remainders must be identical.
    let src = "a: 1\nname: foo   # spaced comment\n\n\nz:   9\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("name", "n").unwrap();
    let out = doc.to_string();
    assert_eq!(out, "a: 1\nn: foo   # spaced comment\n\n\nz:   9\n");
    let (before_in, after_in) = src.split_once("name").unwrap();
    let (before_out, after_out) = out.split_once('n').unwrap();
    assert_eq!(before_in, before_out);
    assert_eq!(after_in, after_out);
}

#[test]
fn rename_single_quoted_key_stays_single_quoted() {
    let mut doc = parse_document("'name': foo\nversion: 1\n").unwrap();
    doc.rename_key("name", "title").unwrap();
    assert_eq!(doc.to_string(), "'title': foo\nversion: 1\n");
    assert_eq!(doc.as_value()["title"].as_str(), Some("foo"));
}

#[test]
fn rename_double_quoted_key_stays_double_quoted() {
    let mut doc = parse_document("\"name\": foo\nversion: 1\n").unwrap();
    doc.rename_key("name", "title").unwrap();
    assert_eq!(doc.to_string(), "\"title\": foo\nversion: 1\n");
    assert_eq!(doc.as_value()["title"].as_str(), Some("foo"));
}

#[test]
fn rename_quoted_key_containing_spaces() {
    let mut doc = parse_document("'app name': foo\n").unwrap();
    doc.rename_key("app name", "app title").unwrap();
    assert_eq!(doc.to_string(), "'app title': foo\n");
    assert_eq!(doc.as_value()["app title"].as_str(), Some("foo"));
}

#[test]
fn rename_to_key_requiring_quotes_colon_space() {
    // `a: b` cannot be spelled plain — the `: ` would terminate the
    // key — so the new key is double-quoted.
    let mut doc = parse_document("name: foo\n").unwrap();
    doc.rename_key("name", "a: b").unwrap();
    assert_eq!(doc.to_string(), "\"a: b\": foo\n");
    assert_eq!(doc.as_value()["a: b"].as_str(), Some("foo"));
}

#[test]
fn rename_to_key_requiring_quotes_leading_dash() {
    let mut doc = parse_document("name: foo\n").unwrap();
    doc.rename_key("name", "-flag").unwrap();
    assert_eq!(doc.to_string(), "\"-flag\": foo\n");
    assert_eq!(doc.as_value()["-flag"].as_str(), Some("foo"));
}

#[test]
fn rename_to_number_like_key_is_quoted_to_stay_a_string() {
    // A plain `8080` would re-parse as a number, not the string
    // `"8080"` — so the spelling must be quoted.
    let mut doc = parse_document("port: 1\n").unwrap();
    doc.rename_key("port", "8080").unwrap();
    assert_eq!(doc.to_string(), "\"8080\": 1\n");
    assert_eq!(doc.as_value()["8080"].as_i64(), Some(1));
}

#[test]
fn rename_key_with_anchored_value_preserves_the_anchor() {
    let src = "name: &a foo\nref: *a\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("name", "title").unwrap();
    assert_eq!(doc.to_string(), "title: &a foo\nref: *a\n");
    assert_eq!(doc.as_value()["ref"].as_str(), Some("foo"));
}

#[test]
fn rename_key_to_its_current_spelling_is_a_noop() {
    let src = "name: foo\nversion: 0.0.1\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("name", "name").unwrap();
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_not_plain_safe_key_to_itself_does_not_requote_it() {
    // `true` is not plain-safe as a *new* key (a plain `true` would
    // re-parse as a bool), but the key already reads `true:` in the
    // source. The no-op is decided on the decoded key, so the file
    // stays byte-identical instead of being rewritten to `"true":`.
    for src in ["true: 1\n", "on: 1\n", "8080: 1\n"] {
        let key = src.split(':').next().unwrap();
        let mut doc = parse_document(src).unwrap();
        doc.rename_key(key, key).unwrap();
        assert_eq!(doc.to_string(), src, "requoted {key}");
    }
}

#[test]
fn rename_to_current_name_in_a_flow_mapping_is_a_noop() {
    // Flow-mapping renames are refused, but a rename to the key's
    // own name changes nothing — the documented no-op holds for
    // every path that resolves to a mapping entry.
    let src = "m: {a: 1}\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("m.a", "a").unwrap();
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_sibling_of_a_complex_key_leaves_the_complex_entry_untouched() {
    // An explicit complex key (`? [a, b]`) stringifies to `[a, b]`,
    // which the path syntax reads as a bracket index — it cannot be
    // addressed, so it cannot be renamed. Its presence must not
    // disturb the renames of its ordinary siblings.
    let src = "? [a, b]\n: 1\nx: 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("x", "y").unwrap();
    let out = doc.to_string();
    assert_eq!(out, "? [a, b]\n: 1\ny: 2\n");
    // The complex-key entry's bytes are byte-identical.
    assert!(out.starts_with("? [a, b]\n: 1\n"), "got: {out}");
    let v = doc.as_value();
    assert_eq!(v["[a, b]"].as_i64(), Some(1));
    assert_eq!(v["y"].as_i64(), Some(2));
}

#[test]
fn rename_quoted_key_to_its_decoded_spelling_is_a_noop() {
    // The site is single-quoted, so the style-matched spelling of
    // `a b` is `'a b'` — identical to the current token, no edit.
    let src = "'a b': 1\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("a b", "a b").unwrap();
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_explicit_scalar_key() {
    // `? foo` explicit-key form with a simple scalar key — the key
    // token itself is a plain scalar, so the rename falls out of the
    // same splice.
    let src = "? foo\n: 1\nx: 2\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("foo", "bar").unwrap();
    assert_eq!(doc.to_string(), "? bar\n: 1\nx: 2\n");
    assert_eq!(doc.as_value()["bar"].as_i64(), Some(1));
}

#[test]
fn rename_key_below_a_sequence_index() {
    // The list-of-mappings shape every Kubernetes / CI manifest uses:
    // the path descends through a sequence index before naming the
    // entry, exercising the index recursion arm.
    let src = "items:\n  - name: a  # first\n    v: 1\n  - name: b\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("items[0].name", "id").unwrap();
    let out = doc.to_string();
    assert_eq!(out, "items:\n  - id: a  # first\n    v: 1\n  - name: b\n");
    // Byte fidelity: only the four bytes of the first `name` token
    // changed — everything around them is untouched.
    let (before_in, after_in) = src.split_once("name").unwrap();
    let (before_out, after_out) = out.split_once("id").unwrap();
    assert_eq!(before_in, before_out);
    assert_eq!(after_in, after_out);
    // The sibling item keeps its own key.
    let v = doc.as_value();
    assert_eq!(v["items"][0]["id"].as_str(), Some("a"));
    assert_eq!(v["items"][1]["name"].as_str(), Some("b"));
}

#[test]
fn rename_key_in_second_document_leaves_first_byte_identical() {
    let src = "---\na: 1\nb: 2\n---\na: 3\nc: 4\n";
    let docs = parse_stream(src).unwrap();
    let mut docs = docs;
    docs[1].rename_key("a", "z").unwrap();
    assert_eq!(docs[0].to_string(), "---\na: 1\nb: 2\n");
    assert_eq!(docs[1].to_string(), "---\nz: 3\nc: 4\n");
    assert_eq!(docs[1].as_value()["z"].as_i64(), Some(3));
}

// ── Refusals — document left untouched ──────────────────────────────

#[test]
fn rename_refuses_when_sibling_key_already_exists() {
    let src = "a: 1\nb: 2\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("a", "b").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("duplicate"), "got: {msg}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_reaches_flow_mapping_entries() {
    // A refusal until #338 (ADR-0011) brought flow mappings into the
    // rename surface; `cst_flow_inserts.rs` pins the details.
    let src = "m: {a: 1, b: 2}\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("m.a", "c").unwrap();
    assert_eq!(doc.to_string(), "m: {c: 1, b: 2}\n");
}

#[test]
fn rename_refuses_missing_path() {
    let src = "a: 1\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("missing", "b").unwrap_err();
    assert!(err.to_string().contains("path not found"));
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_refuses_sequence_item_path() {
    let src = "items:\n  - one\n  - two\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("items[0]", "b").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("mapping entry"), "got: {msg}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_refuses_alias_key() {
    // `*k :` — the entry's key is an alias reference, not a scalar
    // token; the loader records no key span for it.
    let src = "a: &k one\n*k : 2\n";
    let mut doc = parse_document(src).unwrap();
    // The typed view resolves the alias key to the anchored string.
    let err = doc.rename_key("one", "zzz").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("not a simple scalar token"), "got: {msg}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_refuses_bracket_segment_that_is_not_an_index() {
    // `servers[web]` is a typo for `servers.web`. The shared path
    // parser drops the unparseable bracket segment, which would
    // rename the *parent* key `servers` — a silent, destructive
    // edit. `rename_key` refuses the spelling instead.
    let src = "servers:\n  web: 1\n  db: 2\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("servers[web]", "hosts").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("[web]"), "got: {msg}");
    assert!(
        msg.contains("neither a sequence index nor a quoted key"),
        "got: {msg}"
    );
    assert!(
        msg.contains(r#"["web"]"#),
        "names the quoted spelling: {msg}"
    );
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_to_the_merge_key_is_refused() {
    // The loader matches `<<` on the decoded key, so no quote style
    // can demote it back to an ordinary key — the rename could never
    // round-trip.
    let src = "name: foo\nother: 1\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("name", "<<").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("merge directive"), "got: {msg}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_to_a_non_printable_key_is_refused() {
    // U+007F (DEL) is outside YAML's c-printable set and the
    // double-quoted formatter would splice it raw.
    let src = "name: foo\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("name", "a\u{7F}b").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("U+007F"), "got: {msg}");
    assert_eq!(doc.to_string(), src);

    // C1 controls (U+0080..=U+009F) are excluded too.
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("name", "a\u{85}b").unwrap_err();
    assert!(err.to_string().contains("U+0085"), "got: {err}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_refuses_a_path_reached_through_an_alias() {
    // `ref.inner` reads fine — the alias resolves to the anchor's
    // mapping — but the bytes at `ref` are just `*b`; the key lives
    // at the anchor's definition.
    let src = "base: &b\n  inner: 1\nref: *b\n";
    let mut doc = parse_document(src).unwrap();
    assert_eq!(doc.get("ref.inner"), Some("1"));
    let err = doc.rename_key("ref.inner", "renamed").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("alias-expanded content"), "got: {msg}");
    assert!(!msg.contains("path not found"), "got: {msg}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_inside_an_aliased_anchor_names_the_anchor() {
    // Renaming here would rename the key at every `*b` site too, so
    // the refusal points at the anchor rather than blaming a
    // duplicate key that does not exist.
    let src = "base: &b\n  inner: 1\nref: *b\nalso: *b\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("base.inner", "renamed").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("&b"), "got: {msg}");
    assert!(msg.contains('2'), "got: {msg}");
    assert!(msg.contains("materialise_aliases_of"), "got: {msg}");
    assert!(!msg.contains("duplicate"), "got: {msg}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_inside_an_anchor_without_aliases_is_allowed() {
    // No `*b` site exists, so the rename is local and safe — the
    // proactive guard must not over-refuse.
    let src = "base: &b\n  inner: 1\n";
    let mut doc = parse_document(src).unwrap();
    doc.rename_key("base.inner", "renamed").unwrap();
    assert_eq!(doc.to_string(), "base: &b\n  renamed: 1\n");
}

#[test]
fn rename_refuses_a_merge_produced_key_as_final_segment() {
    // `m.x` exists only because of `<<: *b`; there is no `x:` entry
    // in `m`'s source to rewrite.
    let src = "base: &b\n  x: 1\nm:\n  <<: *b\n  y: 2\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("m.x", "z").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("`<<` merge key"), "got: {msg}");
    assert!(msg.contains("no entry of its own"), "got: {msg}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_refuses_a_merge_produced_key_as_intermediate_segment() {
    // Same condition one segment earlier: the recursion arm must
    // spell it the same way rather than reporting "path not found".
    let src = "base: &b\n  sub:\n    q: 1\nm:\n  <<: *b\n  y: 2\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("m.sub.q", "z").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("`<<` merge key"), "got: {msg}");
    assert!(msg.contains("no entry of its own"), "got: {msg}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_onto_a_merge_provided_sibling_is_not_called_a_duplicate() {
    // `m` has no `x:` entry of its own — `x` arrives via `<<: *b`.
    // The rename is still refused, but as an override, not as a
    // duplicate key.
    let src = "base: &b\n  x: 1\nm:\n  <<: *b\n  y: 2\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("m.y", "x").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("provided by a `<<` merge key"), "got: {msg}");
    assert!(msg.contains("overrides the merged value"), "got: {msg}");
    assert!(!msg.contains("duplicate"), "got: {msg}");
    assert_eq!(doc.to_string(), src);
}

#[test]
fn rename_on_a_document_left_unparseable_errors_instead_of_panicking() {
    // `set` commits its local repair optimistically, so `[` leaves
    // the document structurally broken (see `Document::validate`).
    // `rename_key` must report that as an error — it returns
    // `Result` and documents no panics.
    let mut doc = parse_document("name: foo\n").unwrap();
    doc.set("name", "[").unwrap();
    let err = doc.rename_key("name", "x").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("does not parse"), "got: {msg}");
    assert!(msg.contains("left unchanged"), "got: {msg}");
}

#[test]
fn rename_refuses_duplicate_old_key_via_integrity_guard() {
    // Under the default DuplicateKeyPolicy::Last the typed view
    // keeps the *last* `k`; renaming that occurrence would
    // resurrect the first one as a distinct key — data change, so
    // the integrity guard must refuse and roll back.
    let src = "k: one\nk: two\n";
    let mut doc = parse_document(src).unwrap();
    let err = doc.rename_key("k", "j").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("integrity"), "got: {msg}");
    assert_eq!(doc.to_string(), src);
    assert_eq!(doc.as_value()["k"].as_str(), Some("two"));
}
