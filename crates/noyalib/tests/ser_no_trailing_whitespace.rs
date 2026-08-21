// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The serializer writes no trailing whitespace that is not string content —
//! issue #297.
//!
//! Two sources, both of them duplicated logic that had drifted from the rule
//! its sibling already applied:
//!
//! 1. **A dangling indicator.** `write_mapping` writes `key:` with no space
//!    when the value is a block collection, because the value begins on the
//!    next line. `write_sequence` carried its own copy of the key-writing and
//!    always wrote `key: `, so any pair inside a sequence item whose value was
//!    a mapping or a list ended its line with a space. The `-` indicator had
//!    the same problem for a nested sequence.
//! 2. **An empty block-scalar line.** The content loop wrote the block's
//!    indent before every line including empty ones, leaving the indent
//!    standing on a line that holds nothing. It existed in three identical
//!    copies (`|` auto, `|` explicit, `>`), so the rule was missing from all
//!    three.
//!
//! Nothing here is a correctness fix: every "before" output loads back to the
//! same value. What it is, is output a repo's own lint rejects —
//! `git diff --check`, `yamllint`'s `trailing-spaces`, `editorconfig-checker`.
//!
//! The distinction that governs every case below is **content versus
//! leftovers**. Trailing whitespace a block scalar's *string* owns must
//! survive untouched; the tests at the end pin that, because a blanket
//! line-strip would be the tempting fix and it silently changes values.

use noyalib::{Value, from_str, to_string_value};

/// Emit `src`, and assert the result carries no trailing whitespace outside
/// string content, still parses, and parses to the same value.
fn emits_clean(src: &str) -> String {
    let value: Value = from_str(src).expect("input parses");
    let out = to_string_value(&value).expect("emits");

    for (i, line) in out.split('\n').enumerate() {
        assert_eq!(
            line,
            line.trim_end(),
            "line {} of {out:?} carries trailing whitespace",
            i + 1
        );
    }
    let back: Value = from_str(&out).unwrap_or_else(|e| panic!("{out:?} does not re-parse: {e}"));
    assert_eq!(back, value, "emission changed the value: {out:?}");
    out
}

// ── A dangling `:` inside a sequence item ───────────────────────────────

#[test]
fn a_mapping_value_inside_a_sequence_item_does_not_dangle_a_space() {
    assert_eq!(
        emits_clean("x:\n  - key: a\n    value:\n      d: 1\n"),
        "x:\n  - key: a\n    value:\n      d: 1"
    );
}

#[test]
fn a_sequence_value_inside_a_sequence_item_does_not_dangle_a_space() {
    assert_eq!(
        emits_clean("x:\n  - key: a\n    value:\n      - 1\n"),
        "x:\n  - key: a\n    value:\n      - 1"
    );
}

#[test]
fn the_first_key_of_a_sequence_item_is_covered_too() {
    // The first key shares the dash's line and is written by a different
    // branch than the rest, which is how one of the two could have been
    // fixed while the other kept writing the space.
    assert_eq!(emits_clean("- a:\n    b: 1\n"), "- a:\n    b: 1");
}

#[test]
fn a_dash_introducing_a_block_collection_does_not_dangle_a_space() {
    assert_eq!(emits_clean("x:\n  - - 1\n"), "x:\n  -\n    - 1");
}

// ── Controls: the space is required and must stay ───────────────────────

#[test]
fn an_inline_scalar_keeps_its_space() {
    assert_eq!(
        emits_clean("x:\n  - key: a\n    value: 1\n"),
        "x:\n  - key: a\n    value: 1"
    );
    assert_eq!(emits_clean("x:\n  k: 1\n"), "x:\n  k: 1");
    assert_eq!(emits_clean("- 1\n"), "- 1");
}

#[test]
fn an_empty_collection_is_inline_so_it_keeps_its_space() {
    // `{}` and `[]` are written on the key's own line, so this is the inline
    // case however collection-shaped the value is — `needs_block_layout`
    // draws the line here and the indicator rule follows it.
    assert_eq!(emits_clean("x:\n  - k: {}\n"), "x:\n  - k: {}");
    assert_eq!(emits_clean("x:\n  - k: []\n"), "x:\n  - k: []");
    assert_eq!(emits_clean("x:\n  - {}\n"), "x:\n  - {}");
}

#[test]
fn a_plain_nested_mapping_is_unchanged_by_all_of_this() {
    // `write_mapping` already got this right; the tests exist so a future
    // unification cannot regress the side that was correct.
    assert_eq!(emits_clean("x:\n  k:\n    d: 1\n"), "x:\n  k:\n    d: 1");
    assert_eq!(emits_clean("x:\n  k:\n    - 1\n"), "x:\n  k:\n    - 1");
}

#[test]
fn an_anchor_definition_keeps_the_space_before_its_ampersand() {
    // An anchor-wrapped block value renders as `&idNNN` followed by the block
    // on the next line, so the `&` is on *this* line and the space is real
    // separation rather than leftovers.
    let out = emits_clean("a: &anc\n  b: 1\nc: *anc\n");
    assert!(
        !out.contains(": \n") && !out.contains(":  "),
        "no dangling separator: {out:?}"
    );
}

// ── Empty lines inside a block scalar ───────────────────────────────────

#[test]
fn an_empty_block_scalar_line_carries_no_indent() {
    assert_eq!(emits_clean("k: |\n  a\n\n  b\n"), "k: |\n  a\n\n  b\n");
}

#[test]
fn every_block_scalar_writer_applies_the_rule() {
    // Three copies of the content loop existed — `|` auto, `|` explicit and
    // `>` — so fixing one would have left the others writing the indent.
    // Nested one level deeper than the top, where the indent is wide enough
    // that a missing rule is unmistakable.
    for src in [
        "outer:\n  inner: |\n    a\n\n    b\n",
        "outer:\n  - inner: |\n      a\n\n      b\n",
    ] {
        let out = emits_clean(src);
        assert!(
            out.contains("\n\n"),
            "the empty line must be truly empty: {out:?}"
        );
    }

    // The explicit `|` and `>` writers are reached through the formatting
    // hints rather than through a parse, since a `>` scalar folds its blank
    // line away at load and so has none left to emit — the value round-trips
    // as `|`. Driving them from the hint is the only way to exercise those
    // two functions at all.
    #[derive(serde::Serialize)]
    struct Doc {
        lit: noyalib::LitString,
        fold: noyalib::FoldString,
    }
    let out = noyalib::to_string(&Doc {
        lit: noyalib::LitString::from("a\n\nb\n".to_string()),
        fold: noyalib::FoldString::from("a\n\nb\n".to_string()),
    })
    .expect("emits");
    for (i, line) in out.split('\n').enumerate() {
        assert_eq!(
            line,
            line.trim_end(),
            "line {} of {out:?} carries trailing whitespace",
            i + 1
        );
    }
    assert!(out.contains('|'), "the literal writer ran: {out:?}");
    assert!(out.contains('>'), "the folded writer ran: {out:?}");
}

// ── Controls: whitespace the string owns is untouchable ─────────────────

#[test]
fn trailing_spaces_that_belong_to_the_string_survive() {
    // The reason a blanket line-strip is not the fix. This value's first line
    // genuinely ends in two spaces; stripping them changes the string, which
    // is a strictly worse defect than the cosmetic one being fixed.
    let value = Value::String("a  \nb\n".to_string());
    let out = to_string_value(&value).expect("emits");
    let back: Value = from_str(&out).expect("re-parses");
    assert_eq!(back, value, "the string's own spaces must survive: {out:?}");

    let stripped: String = out
        .split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    let after_strip: Value = from_str(&stripped).expect("re-parses");
    assert_ne!(
        after_strip, value,
        "if this ever passes, the blanket strip became safe and this test's premise is gone"
    );
}

#[test]
fn a_line_of_only_spaces_inside_a_string_is_content_not_indent() {
    // Distinguishable from the empty-line case only by what the *string*
    // holds, which is exactly why the rule keys on `line.is_empty()` rather
    // than on how the line looks once written.
    let value = Value::String("a\n   \nb\n".to_string());
    let out = to_string_value(&value).expect("emits");
    let back: Value = from_str(&out).expect("re-parses");
    assert_eq!(
        back, value,
        "an all-spaces content line must survive: {out:?}"
    );
}
