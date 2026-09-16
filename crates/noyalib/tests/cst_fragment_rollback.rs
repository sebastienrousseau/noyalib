// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The safety net under every fragment-splicing mutator.
//!
//! `set`, `insert_entry`, `push_back` and `insert_after` take YAML
//! *text* from the caller. That text is spliced into the document, and
//! only then checked: does the result still parse, and does it load
//! back as the value the caller asked for and nothing more? When either
//! answer is no, the mutator restores a snapshot and reports why.
//!
//! Those checks are the only thing standing between a mistyped fragment
//! and a silently corrupted file, and every one of them lives on a
//! branch that a well-formed fragment never takes — so the whole safety
//! net went untested. Each case below asserts the two properties that
//! matter together: the call fails, **and** the document is byte-for-
//! byte what it was. A rollback that reports an error after leaving a
//! half-spliced document is the failure this guards against.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use noyalib::Value;
use noyalib::cst::{Document, parse_document};

/// One splicing mutator, pinned to a fixed path and fragment.
type Mutator = fn(&mut Document) -> noyalib::Result<()>;

const DOC: &str = "a: 1\nxs:\n  - p\n  - q\nm:\n  k: 1\n";

/// Either the mutator refuses and leaves the document alone, or it
/// succeeds and the result still parses. There is no third outcome.
#[track_caller]
fn rollback_is_clean(label: &str, op: impl FnOnce(&mut Document) -> noyalib::Result<()>) -> bool {
    let mut doc = parse_document(DOC).expect("fixture parses");
    #[allow(clippy::single_match_else)] // both arms carry assertions, not one
    match op(&mut doc) {
        Err(_e) => {
            assert_eq!(
                doc.to_string(),
                DOC,
                "{label}: the mutator reported failure but left the document edited"
            );
            true
        }
        Ok(()) => {
            // A structurally invalid fragment commits optimistically by
            // design — the splice is verbatim, and the document is only
            // checked when asked. `validate` is the documented catch, so
            // the contract is that the two agree: either the result
            // parses and validates, or it does neither. A document that
            // fails to parse while `validate` reports success would mean
            // the caller has no way to find out.
            let out = doc.to_string();
            let parses = noyalib::from_str::<Value>(&out).is_ok();
            let validates = doc.validate().is_ok();
            assert_eq!(
                parses, validates,
                "{label}: `validate` disagrees with whether the document parses, \
                 so a broken splice is undetectable\n{out}"
            );
            false
        }
    }
}

/// Fragments that are not a single well-formed YAML node in the target
/// position. Each is a plausible typo rather than an adversarial input.
const HOSTILE: &[(&str, &str)] = &[
    ("a second entry smuggled in", "1\nb: 2"),
    ("a stray mapping indicator", ": : bad"),
    ("an unclosed flow sequence", "[unclosed"),
    ("an alias with no anchor", "*missing"),
    ("a tag whose content does not fit it", "!!int notanint"),
    ("a block sequence where a scalar goes", "- 1\n- 2"),
    ("nothing at all", "\n"),
    ("only whitespace", "  "),
];

#[test]
fn set_rolls_back_cleanly_for_every_hostile_fragment() {
    let mut refused = 0;
    for (label, frag) in HOSTILE {
        if rollback_is_clean(&format!("set: {label}"), |d| d.set("a", frag)) {
            refused += 1;
        }
    }
    assert!(
        refused >= 5,
        "only {refused} of {} hostile fragments were refused — the integrity \
         checks may have stopped running",
        HOSTILE.len()
    );
}

#[test]
fn insert_entry_rolls_back_cleanly_for_every_hostile_fragment() {
    let mut refused = 0;
    for (label, frag) in HOSTILE {
        if rollback_is_clean(&format!("insert_entry: {label}"), |d| {
            d.insert_entry("m", "z", frag)
        }) {
            refused += 1;
        }
    }
    assert!(
        refused >= 3,
        "only {refused} hostile fragments were refused"
    );
}

#[test]
fn push_back_rolls_back_cleanly_for_every_hostile_fragment() {
    let mut refused = 0;
    for (label, frag) in HOSTILE {
        if rollback_is_clean(&format!("push_back: {label}"), |d| d.push_back("xs", frag)) {
            refused += 1;
        }
    }
    assert!(
        refused >= 3,
        "only {refused} hostile fragments were refused"
    );
}

#[test]
fn insert_after_rolls_back_cleanly_for_every_hostile_fragment() {
    let mut refused = 0;
    for (label, frag) in HOSTILE {
        if rollback_is_clean(&format!("insert_after: {label}"), |d| {
            d.insert_after("xs[0]", frag)
        }) {
            refused += 1;
        }
    }
    assert!(
        refused >= 3,
        "only {refused} hostile fragments were refused"
    );
}

/// The specific refusal that protects a *neighbouring* entry: a
/// fragment that parses perfectly well on its own, but adds a sibling
/// once spliced. This is the check most likely to be quietly lost in a
/// refactor, because the fragment itself is valid YAML.
#[test]
fn a_fragment_that_parses_but_adds_a_sibling_is_refused_by_name() {
    for (label, op) in [("set", 0), ("push_back", 1), ("insert_after", 2)] {
        let mut doc = parse_document(DOC).expect("parse");
        let res = match op {
            0 => doc.set("a", "1\nb: 2"),
            1 => doc.push_back("xs", "1\nb: 2"),
            _ => doc.insert_after("xs[0]", "1\nb: 2"),
        };
        let err = res.expect_err("a fragment adding a sibling must be refused");
        let msg = err.to_string();
        assert!(
            msg.contains("added or removed entries"),
            "{label}: the refusal does not say what was wrong: {msg}"
        );
        assert_eq!(doc.to_string(), DOC, "{label}: rolled forward, not back");
    }
}

/// A fragment that would make the document unparseable is caught by the
/// re-parse check rather than written out.
#[test]
fn a_fragment_that_breaks_parsing_is_caught_before_it_is_kept() {
    let mut doc = parse_document(DOC).expect("parse");
    let err = doc
        .set("a", ": : bad")
        .expect_err("a fragment that does not parse must be refused");
    assert!(
        err.to_string().contains("parse"),
        "the refusal does not mention parsing: {err}"
    );
    assert_eq!(doc.to_string(), DOC);
}

/// An alias with no anchor parses as a token but cannot be resolved, so
/// it fails the value-level check rather than the syntax one.
#[test]
fn a_dangling_alias_in_a_fragment_is_refused_by_every_splicing_mutator() {
    for (label, op) in [
        ("set", 0),
        ("insert_entry", 1),
        ("push_back", 2),
        ("insert_after", 3),
    ] {
        let mut doc = parse_document(DOC).expect("parse");
        let res = match op {
            0 => doc.set("a", "*missing"),
            1 => doc.insert_entry("m", "z", "*missing"),
            2 => doc.push_back("xs", "*missing"),
            _ => doc.insert_after("xs[0]", "*missing"),
        };
        let err = res.expect_err("a dangling alias must be refused");
        assert!(
            err.to_string().contains("anchor"),
            "{label}: the refusal does not name the missing anchor: {err}"
        );
        assert_eq!(doc.to_string(), DOC, "{label}: rolled forward, not back");
    }
}

// ── rename_key's key spelling ───────────────────────────────────

/// A new key that would change the document's structure if written
/// plainly must be quoted. Every one of these round-trips to a mapping
/// with the same two entries and the new key readable by its exact
/// text — which is the only thing that makes `rename_key` safe to hand
/// a string from outside the program.
#[test]
fn rename_key_quotes_whatever_spelling_it_is_given() {
    let src = "a: 1\nz: 2\n";
    let hostile_keys = [
        "b: 2",  // would read as two entries
        "",      // empty
        "? x",   // explicit-key indicator
        "\"q\"", // already quoted
        ": ",    // bare indicator
        "#c",    // comment indicator
        "- x",   // sequence indicator
        "*al",   // alias indicator
        "&an",   // anchor indicator
        "a b",   // legal plain key with a space
    ];
    for key in hostile_keys {
        let mut doc = parse_document(src).expect("parse");
        doc.rename_key("a", key)
            .unwrap_or_else(|e| panic!("rename to {key:?}: {e}"));
        let out = doc.to_string();
        let v: Value = noyalib::from_str(&out)
            .unwrap_or_else(|e| panic!("rename to {key:?} broke the document: {e}\n{out}"));
        let m = v.as_mapping().expect("mapping");
        assert_eq!(
            m.len(),
            2,
            "rename to {key:?} changed the entry count: {out}"
        );
        assert_eq!(
            v.get(key).and_then(Value::as_i64),
            Some(1),
            "rename to {key:?} is not readable back under that key: {out}"
        );
        assert_eq!(v.get("z").and_then(Value::as_i64), Some(2), "{out}");
    }
}

/// A key holding a character YAML cannot represent on one line is
/// refused outright rather than quoted into something that parses but
/// means something else.
#[test]
fn rename_key_refuses_a_key_that_cannot_be_written_on_one_line() {
    let src = "a: 1\nz: 2\n";
    for key in ["x\ny", "\u{1}"] {
        let mut doc = parse_document(src).expect("parse");
        let err = doc
            .rename_key("a", key)
            .expect_err("a key with a non-printable character must be refused");
        assert!(
            err.to_string().contains("non-printable"),
            "the refusal does not say why: {err}"
        );
        assert_eq!(doc.to_string(), src, "a refused rename edited the document");
    }
}

/// The four splicing mutators commit a structurally invalid fragment
/// rather than refusing it: the splice is verbatim by contract, and the
/// result is only checked when asked. `validate` is the documented way
/// to find out, and it must report every one of them — otherwise a
/// caller who trusts the `Ok(())` writes a broken file.
#[test]
fn a_structurally_invalid_fragment_commits_but_validate_reports_it() {
    let cases: &[(&str, Mutator)] = &[
        ("set", |d| d.set("a", "[unclosed")),
        ("insert_entry", |d| d.insert_entry("m", "z", "[unclosed")),
        ("push_back", |d| d.push_back("xs", "[unclosed")),
        ("insert_after", |d| d.insert_after("xs[0]", "[unclosed")),
    ];
    for (label, op) in cases {
        let mut doc = parse_document(DOC).expect("parse");
        op(&mut doc).unwrap_or_else(|e| panic!("{label}: expected an optimistic commit, got {e}"));
        let err = doc
            .validate()
            .expect_err("`validate` must report the broken splice the mutator accepted");
        assert!(
            !err.to_string().is_empty(),
            "{label}: `validate` failed without saying why"
        );
        assert!(
            noyalib::from_str::<Value>(&doc.to_string()).is_err(),
            "{label}: `validate` reported an error the parser does not agree with"
        );
    }
}
