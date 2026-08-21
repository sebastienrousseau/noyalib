//! Alias resolution across the streaming deserializer's lookahead slot.
//!
//! The slot (`StreamingDeserializer::current`) holds one event of
//! lookahead, and it is written by three paths with two different
//! invariants — a *raw* parser event, whose `Event::Alias` the merge-key
//! handling needs to see unresolved, and a *processed* one that has been
//! alias-resolved, anchored and recorded.
//!
//! Two defects lived in that ambiguity:
//!
//! 1. `peek_event` and `next_event` each read from either the replay stack
//!    (injected merge content) or the parser, and resolved aliases on the
//!    **parser branch only** — then labelled both results processed. An
//!    alias arriving via replay was therefore stored as "processed" while
//!    still being an unresolved `Event::Alias`, and deserialised as the
//!    literal anchor name. That is #301, found by @mathstuf: `y: *other`
//!    sitting beside `<<: *b`.
//! 2. `next_parser_event` unconditionally re-ran `handle_anchor` and
//!    `maybe_record` on whatever it took from the slot. `maybe_record` is
//!    not idempotent — a second call pushes the event into an in-flight
//!    anchor buffer twice, so every alias to that anchor replays a
//!    corrupted stream.
//!
//! The replay stack only exists once a merge has injected something, which
//! is why every failing shape here has an alias *after* a `<<:` key and
//! every passing one does not. Most of this file is therefore ordering:
//! the same document with the alias before and after the merge must agree.
//!
//! The strongest check is [`agrees_with_ast`]: the streaming path and the
//! AST loader must produce the same `Value` for the same input. They share
//! no lookahead code, so a regression in one is visible as disagreement.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Value, from_str, load_all};
use std::collections::BTreeMap;

/// The streaming deserializer and the AST loader must agree.
///
/// `from_str::<Value>` drives the streaming path; `load_all` is the
/// separate document loader and shares none of the lookahead code. A
/// regression in either therefore shows up as disagreement, without the
/// test having to spell out the expected shape by hand.
#[track_caller]
fn agrees_with_ast(yaml: &str) -> Value {
    let streamed: Value = from_str(yaml).unwrap_or_else(|e| panic!("streaming: {e}\n{yaml}"));
    let ast = load_all(yaml)
        .unwrap_or_else(|e| panic!("ast: {e}\n{yaml}"))
        .next()
        .unwrap_or_else(|| panic!("ast produced no document for:\n{yaml}"))
        .unwrap_or_else(|e| panic!("ast: {e}\n{yaml}"));
    assert_eq!(streamed, ast, "streaming and AST disagree on:\n{yaml}");
    streamed
}

/// Deserialise `overridden` as an integer map and assert its contents.
#[track_caller]
fn overridden_ints(yaml: &str, want: &[(&str, i64)]) {
    #[derive(serde::Deserialize)]
    struct Doc {
        overridden: BTreeMap<String, i64>,
    }
    let d: Doc = from_str(yaml).unwrap_or_else(|e| panic!("{e}\n{yaml}"));
    let expect: BTreeMap<String, i64> = want.iter().map(|(k, v)| ((*k).to_string(), *v)).collect();
    assert_eq!(d.overridden, expect, "for:\n{yaml}");
}

// ── #301: an alias as a value, after a merge key ─────────────────────
//
// Everything in this block goes through the replay stack, because the
// merge injects content before the alias is read.

#[test]
fn alias_to_scalar_after_merge_key() {
    overridden_ints(
        "base: &b\n  x: 1\n  y: 1\nother: &other\n  2\noverridden:\n  <<: *b\n  y: *other\n",
        &[("x", 1), ("y", 2)],
    );
}

#[test]
fn alias_to_inline_scalar_after_merge_key() {
    overridden_ints(
        "base: &b\n  x: 1\n  y: 1\nother: &other 2\noverridden:\n  <<: *b\n  y: *other\n",
        &[("x", 1), ("y", 2)],
    );
}

#[test]
fn alias_introducing_a_new_key_after_merge() {
    overridden_ints(
        "base: &b\n  x: 1\nn: &n 9\noverridden:\n  <<: *b\n  z: *n\n",
        &[("x", 1), ("z", 9)],
    );
}

#[test]
fn several_alias_values_after_one_merge() {
    overridden_ints(
        "base: &b\n  x: 1\n  y: 1\n  z: 1\np: &p 7\nq: &q 8\noverridden:\n  <<: *b\n  y: *p\n  z: *q\n",
        &[("x", 1), ("y", 7), ("z", 8)],
    );
}

#[test]
fn the_same_alias_used_twice_after_a_merge() {
    // The second use must replay the anchor again, not find it consumed.
    overridden_ints(
        "base: &b\n  x: 1\nn: &n 5\noverridden:\n  <<: *b\n  a: *n\n  b: *n\n",
        &[("x", 1), ("a", 5), ("b", 5)],
    );
}

// ── ordering: the same document, alias before vs after ────────────────

#[test]
fn alias_before_and_after_the_merge_agree() {
    let after = "base: &b\n  x: 1\n  y: 1\nn: &n 2\noverridden:\n  <<: *b\n  y: *n\n";
    let before = "base: &b\n  x: 1\n  y: 1\nn: &n 2\noverridden:\n  y: *n\n  <<: *b\n";
    overridden_ints(after, &[("x", 1), ("y", 2)]);
    overridden_ints(before, &[("x", 1), ("y", 2)]);
}

#[test]
fn alias_with_no_merge_key_at_all() {
    // The control: no merge means no replay stack, so this exercised the
    // parser branch and always worked.
    overridden_ints(
        "n: &n 2\noverridden:\n  x: 1\n  y: *n\n",
        &[("x", 1), ("y", 2)],
    );
}

// ── aliases to collections, not just scalars ─────────────────────────

#[test]
fn alias_to_a_mapping_after_merge() {
    let y = "base: &b\n  x: 1\nm: &m\n  inner: 2\nout:\n  <<: *b\n  nested: *m\n";
    let v = agrees_with_ast(y);
    assert_eq!(v["out"]["x"], Value::from(1));
    assert_eq!(v["out"]["nested"]["inner"], Value::from(2));
}

#[test]
fn alias_to_a_sequence_after_merge() {
    let y = "base: &b\n  x: 1\ns: &s\n  - 1\n  - 2\nout:\n  <<: *b\n  items: *s\n";
    let v = agrees_with_ast(y);
    assert_eq!(v["out"]["items"][0], Value::from(1));
    assert_eq!(v["out"]["items"][1], Value::from(2));
}

#[test]
fn alias_inside_a_sequence_value_after_merge() {
    let y = "base: &b\n  x: 1\nn: &n 3\nout:\n  <<: *b\n  items:\n    - *n\n    - 4\n";
    let v = agrees_with_ast(y);
    assert_eq!(v["out"]["items"][0], Value::from(3));
    assert_eq!(v["out"]["items"][1], Value::from(4));
}

#[test]
fn alias_inside_a_nested_mapping_after_merge() {
    let y = "base: &b\n  x: 1\nn: &n 3\nout:\n  <<: *b\n  deep:\n    k: *n\n";
    let v = agrees_with_ast(y);
    assert_eq!(v["out"]["deep"]["k"], Value::from(3));
}

// ── merge mechanics that must keep working ───────────────────────────

#[test]
fn merge_from_a_sequence_of_anchors_with_an_alias_override() {
    let y = "a: &a\n  x: 1\nb: &b\n  y: 2\nn: &n 9\nout:\n  <<: [*a, *b]\n  y: *n\n";
    let v = agrees_with_ast(y);
    assert_eq!(v["out"]["x"], Value::from(1));
    assert_eq!(
        v["out"]["y"],
        Value::from(9),
        "local override wins over merge"
    );
}

#[test]
fn a_local_alias_override_beats_the_merged_key() {
    overridden_ints(
        "base: &b\n  y: 1\nn: &n 42\noverridden:\n  <<: *b\n  y: *n\n",
        &[("y", 42)],
    );
}

#[test]
fn merging_a_mapping_that_itself_contains_an_alias() {
    // The anchor's own recorded events include an alias, so replaying it
    // has to resolve that too.
    let y = "n: &n 5\nbase: &b\n  x: *n\nout:\n  <<: *b\n  y: 1\n";
    let v = agrees_with_ast(y);
    assert_eq!(v["out"]["x"], Value::from(5));
    assert_eq!(v["out"]["y"], Value::from(1));
}

#[test]
fn two_independent_mappings_each_with_merge_and_alias() {
    // A second mapping must not inherit a slot left over from the first.
    let y = "b1: &b1\n  x: 1\nb2: &b2\n  x: 2\nn: &n 7\none:\n  <<: *b1\n  y: *n\ntwo:\n  <<: *b2\n  y: *n\n";
    let v = agrees_with_ast(y);
    assert_eq!(v["one"]["x"], Value::from(1));
    assert_eq!(v["one"]["y"], Value::from(7));
    assert_eq!(v["two"]["x"], Value::from(2));
    assert_eq!(v["two"]["y"], Value::from(7));
}

// ── anchor recording integrity (the not-idempotent half) ─────────────

#[test]
fn an_anchor_defined_after_a_merge_replays_exactly_once() {
    // If `maybe_record` ran twice on any event of `&later`, this alias
    // replays a duplicated stream and the mapping gains phantom entries.
    let y = "base: &b\n  x: 1\nout:\n  <<: *b\n  later: &later\n    k: 1\ncopy: *later\n";
    let v = agrees_with_ast(y);
    assert_eq!(v["copy"]["k"], Value::from(1));
    assert_eq!(
        v["copy"].as_mapping().map(noyalib::Mapping::len),
        Some(1),
        "a double-recorded anchor shows up as extra entries"
    );
}

#[test]
fn an_anchored_sequence_after_a_merge_replays_exactly_once() {
    let y = "base: &b\n  x: 1\nout:\n  <<: *b\n  items: &s\n    - 1\n    - 2\ncopy: *s\n";
    let v = agrees_with_ast(y);
    assert_eq!(
        v["copy"].as_sequence().map(Vec::len),
        Some(2),
        "a double-recorded anchor doubles the sequence"
    );
}

#[test]
fn an_anchor_on_the_merged_mapping_itself_is_replayable() {
    let y = "base: &b\n  x: 1\nout: &o\n  <<: *b\n  y: 2\ncopy: *o\n";
    let v = agrees_with_ast(y);
    assert_eq!(v["copy"]["x"], Value::from(1));
    assert_eq!(v["copy"]["y"], Value::from(2));
}

// ── differential: streaming and AST must never disagree ──────────────

#[test]
fn streaming_and_ast_agree_across_every_shape() {
    for y in [
        "n: &n 1\na:\n  x: *n\n",
        "b: &b\n  x: 1\no:\n  <<: *b\n",
        "b: &b\n  x: 1\nn: &n 2\no:\n  <<: *b\n  y: *n\n",
        "b: &b\n  x: 1\nn: &n 2\no:\n  y: *n\n  <<: *b\n",
        "a: &a\n  x: 1\nb: &b\n  y: 2\no:\n  <<: [*a, *b]\n",
        "n: &n [1, 2]\no:\n  items: *n\n",
        "n: &n {k: 1}\no:\n  m: *n\n",
        "b: &b\n  x: 1\nn: &n 2\no:\n  <<: *b\n  deep:\n    k: *n\n",
        "b: &b\n  x: 1\no:\n  <<: *b\n  s:\n    - 1\n    - 2\n",
        "n: &n 1\nb: &b\n  x: *n\no:\n  <<: *b\n  y: *n\n",
    ] {
        let _ = agrees_with_ast(y);
    }
}

// ── things that must still be refused or left alone ──────────────────

#[test]
fn an_unknown_alias_after_a_merge_is_still_an_error() {
    let y = "b: &b\n  x: 1\no:\n  <<: *b\n  y: *nope\n";
    let r: Result<Value, _> = from_str(y);
    assert!(r.is_err(), "undefined alias must not resolve silently");
}

#[test]
fn an_unknown_merge_target_is_still_an_error() {
    let y = "o:\n  <<: *nope\n  y: 1\n";
    let r: Result<Value, _> = from_str(y);
    assert!(
        r.is_err(),
        "undefined merge source must not resolve silently"
    );
}

#[test]
fn a_plain_scalar_named_like_an_alias_is_not_one() {
    // `"*n"` quoted is a string, not an alias — the fix must not widen
    // resolution to anything that looks like one.
    let y = "b: &b\n  x: 1\no:\n  <<: *b\n  y: \"*n\"\n";
    let v = agrees_with_ast(y);
    assert_eq!(v["o"]["y"], Value::from("*n"));
}

// NOTE: a *quoted* `"<<"` key is currently treated as a merge key too, and
// both the streaming and AST paths reject it with `ScalarInMergeElement`.
// Per the YAML merge type only a **plain** `<<` carries
// `tag:yaml.org,2002:merge`; a quoted one resolves to `...:str` and should
// be an ordinary key. That deviation is consistent across both paths, is
// unrelated to the lookahead fix, and is left for a separate change rather
// than pinned here — asserting the current behaviour would bless it.

#[test]
fn a_plain_merge_key_with_a_scalar_target_is_refused_on_both_paths() {
    // The in-scope half: `<<` must take an alias or a sequence of them.
    let y = "o:\n  <<: 1\n  y: 2\n";
    assert!(
        from_str::<Value>(y).is_err(),
        "scalar merge target must be refused"
    );
    // Refused either by `load_all` itself or by the document it yields.
    let ast_err = load_all(y)
        .ok()
        .and_then(|mut i| i.next())
        .is_none_or(|r| r.is_err());
    assert!(ast_err, "the AST path must refuse it too");
}
