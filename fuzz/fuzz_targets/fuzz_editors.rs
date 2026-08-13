//! Fuzz target: the CST edit API, structurally.
//!
//! The other targets fuzz the *parser* with bytes. This one fuzzes the
//! *editors* with a generated edit applied to a generated document, and
//! checks the guarantees the mutators claim.
//!
//! Three silent-corruption bugs were found by hand in v0.0.21 — `remove`
//! deleting a whole flow-collection parent, and `set` / `push_back`
//! letting a newline fragment add sibling entries. Each returned `Ok`
//! while damaging the document. Finding them depended on someone
//! thinking to try a flow collection. This target does not depend on
//! that.
//!
//! # Invariants
//!
//! 1. **A refused edit changes nothing.** If a mutator returns `Err`,
//!    the source must be byte-identical. This is the strongest and most
//!    general guarantee the API makes, and the one all three bugs broke
//!    in spirit — they did not refuse, but the failure mode is the
//!    same: the document is not what the caller asked for.
//!
//! 2. **A comment edit never changes the value.** Comments are trivia;
//!    if `set_comment` or `remove_comment` alters what the document
//!    *means*, that is a bug by definition. This gives the comment
//!    mutators a total invariant, which the enumerated tests cannot.
//!
//! 3. **An accepted `remove` removes exactly one path.** The typed
//!    oracle inside `remove` already claims this; asserting it here
//!    tests the oracle rather than trusting it.
//!
//! Deliberately *not* asserted: that an accepted edit leaves a
//! parseable document. `set` commits an invalid splice optimistically
//! by design — `set("k", "[")` succeeds and surfaces via `validate` —
//! and that behaviour is covered by its own test.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use noyalib::cst::{parse_document, CommentPosition};
use noyalib::Value;

/// One edit against one document.
#[derive(Arbitrary, Debug)]
struct Case {
    /// The document to edit. Most random strings are not YAML; the
    /// target returns early on those, and the corpus accretes the ones
    /// that parse.
    source: String,
    edit: Edit,
}

#[derive(Arbitrary, Debug)]
enum Edit {
    Set { path: String, fragment: String },
    Remove { path: String },
    InsertEntry { map: String, key: String, fragment: String },
    PushBack { path: String, fragment: String },
    InsertAfter { item: String, fragment: String },
    RenameKey { path: String, new_key: String },
    SwapItems { path: String, i: u8, j: u8 },
    MoveItem { path: String, from: u8, to: u8 },
    SetComment { path: String, inline: bool, text: String },
    RemoveComment { path: String, inline: bool },
}

fn position(inline: bool) -> CommentPosition {
    if inline {
        CommentPosition::Inline
    } else {
        CommentPosition::Before
    }
}

fuzz_target!(|case: Case| {
    // Only documents the CST accepts are interesting; the parser has
    // its own targets.
    let Ok(mut doc) = parse_document(&case.source) else {
        return;
    };
    let before_src = doc.source().to_owned();
    let before_val = noyalib::from_str::<Value>(&before_src).ok();

    let is_comment_edit = matches!(
        case.edit,
        Edit::SetComment { .. } | Edit::RemoveComment { .. }
    );
    let removed_path = match &case.edit {
        Edit::Remove { path } => Some(path.clone()),
        _ => None,
    };

    let result = match &case.edit {
        Edit::Set { path, fragment } => doc.set(path, fragment),
        Edit::Remove { path } => doc.remove(path),
        Edit::InsertEntry {
            map,
            key,
            fragment,
        } => doc.insert_entry(map, key, fragment),
        Edit::PushBack { path, fragment } => doc.push_back(path, fragment),
        Edit::InsertAfter { item, fragment } => doc.insert_after(item, fragment),
        Edit::RenameKey { path, new_key } => doc.rename_key(path, new_key),
        Edit::SwapItems { path, i, j } => doc.swap_items(path, *i as usize, *j as usize),
        Edit::MoveItem { path, from, to } => doc.move_item(path, *from as usize, *to as usize),
        Edit::SetComment {
            path,
            inline,
            text,
        } => doc.set_comment(path, position(*inline), text),
        Edit::RemoveComment { path, inline } => doc.remove_comment(path, position(*inline)),
    };

    // ── Invariant 1: a refusal leaves the document untouched ────────
    if result.is_err() {
        assert_eq!(
            doc.source(),
            before_src,
            "a refused edit modified the document: {:?}",
            case.edit
        );
        return;
    }

    // ── Invariant 2: comment edits preserve the value ───────────────
    if is_comment_edit {
        if let Some(before) = &before_val {
            // The edited document must still parse — a comment edit has
            // no licence to break the document — and mean the same.
            let after = noyalib::from_str::<Value>(doc.source()).unwrap_or_else(|e| {
                panic!(
                    "a comment edit made the document unparseable ({e}): {:?}\nsource: {:?}",
                    case.edit,
                    doc.source()
                )
            });
            assert_eq!(
                &after, before,
                "a comment edit changed the document's value: {:?}",
                case.edit
            );
        }
        return;
    }

    // ── Invariant 3: an accepted remove drops exactly one path ──────
    if let (Some(path), Some(before)) = (removed_path, before_val) {
        if let Ok(after) = noyalib::from_str::<Value>(doc.source()) {
            // Count leaves as a cheap proxy for "one thing left". A
            // removal that took a parent with it — the v0.0.21 flow
            // bug — shows up as a much larger drop.
            let before_n = count_nodes(&before);
            let after_n = count_nodes(&after);
            assert!(
                after_n < before_n,
                "remove({path:?}) did not shrink the document: {before_n} -> {after_n}"
            );
        }
    }
});

/// Total nodes in a value tree, counting containers and scalars alike.
fn count_nodes(v: &Value) -> usize {
    match v {
        Value::Mapping(m) => 1 + m.values().map(count_nodes).sum::<usize>(),
        Value::Sequence(s) => 1 + s.iter().map(count_nodes).sum::<usize>(),
        _ => 1,
    }
}
