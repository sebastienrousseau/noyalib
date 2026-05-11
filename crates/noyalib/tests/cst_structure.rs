// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Structural-shape assertions for the nested green tree.
//!
//! Round-trip is covered separately by `cst_round_trip.rs` — these
//! tests check that the new `BlockMapping` / `BlockSequence` /
//! `MappingEntry` / `SequenceItem` composites are produced where
//! expected, with the leaf order required for downstream
//! neighbour-aware tooling.

use noyalib::cst::{GreenChild, GreenNode, SyntaxKind, parse_document};

fn kinds(node: &GreenNode) -> Vec<SyntaxKind> {
    node.children()
        .map(|c| match c {
            GreenChild::Token { kind, .. } => *kind,
            GreenChild::Node(n) => n.kind(),
        })
        .collect()
}

fn first_node_of(node: &GreenNode, kind: SyntaxKind) -> Option<&GreenNode> {
    for c in node.children() {
        if let GreenChild::Node(n) = c {
            if n.kind() == kind {
                return Some(n);
            }
        }
    }
    None
}

fn entries_of(node: &GreenNode) -> Vec<&GreenNode> {
    node.children()
        .filter_map(|c| match c {
            GreenChild::Node(n)
                if matches!(
                    n.kind(),
                    SyntaxKind::MappingEntry | SyntaxKind::SequenceItem
                ) =>
            {
                Some(n)
            }
            _ => None,
        })
        .collect()
}

fn token_kinds_of(node: &GreenNode) -> Vec<SyntaxKind> {
    node.children()
        .filter_map(|c| match c {
            GreenChild::Token { kind, .. } => Some(*kind),
            GreenChild::Node(_) => None,
        })
        .collect()
}

#[test]
fn top_level_mapping_wraps_in_block_mapping() {
    let doc = parse_document("a: 1\nb: 2\n").unwrap();
    let bm = first_node_of(doc.syntax(), SyntaxKind::BlockMapping).expect("BlockMapping");
    let entries = entries_of(bm);
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|e| e.kind() == SyntaxKind::MappingEntry));
}

#[test]
fn nested_mapping_nests_under_parent_entry() {
    let doc = parse_document("outer:\n  inner: 1\n").unwrap();
    let outer_bm = first_node_of(doc.syntax(), SyntaxKind::BlockMapping).expect("outer mapping");
    let outer_entries = entries_of(outer_bm);
    assert_eq!(outer_entries.len(), 1);
    let outer_entry = outer_entries[0];
    // The outer entry's value should be a nested BlockMapping.
    let inner_bm =
        first_node_of(outer_entry, SyntaxKind::BlockMapping).expect("nested BlockMapping");
    let inner_entries = entries_of(inner_bm);
    assert_eq!(inner_entries.len(), 1);
}

#[test]
fn block_sequence_produces_sequence_items() {
    let doc = parse_document("- one\n- two\n- three\n").unwrap();
    let bs = first_node_of(doc.syntax(), SyntaxKind::BlockSequence).expect("BlockSequence");
    let items = entries_of(bs);
    assert_eq!(items.len(), 3);
    assert!(items.iter().all(|i| i.kind() == SyntaxKind::SequenceItem));
    // Each SequenceItem must lead with a DashIndicator.
    for item in items {
        let first = item.children().next().expect("at least one child");
        match first {
            GreenChild::Token { kind, .. } => {
                assert_eq!(*kind, SyntaxKind::DashIndicator)
            }
            GreenChild::Node(_) => panic!("first child of SequenceItem must be `-`"),
        }
    }
}

#[test]
fn nested_sequence_under_mapping_value() {
    let doc = parse_document("items:\n  - a\n  - b\n").unwrap();
    let bm = first_node_of(doc.syntax(), SyntaxKind::BlockMapping).expect("BlockMapping");
    let entry = entries_of(bm)[0];
    let inner = first_node_of(entry, SyntaxKind::BlockSequence).expect("nested BlockSequence");
    assert_eq!(entries_of(inner).len(), 2);
}

#[test]
fn flow_mapping_is_flat_in_phase_1() {
    let doc = parse_document("{a: 1, b: 2}\n").unwrap();
    let fm = first_node_of(doc.syntax(), SyntaxKind::FlowMapping).expect("FlowMapping");
    // Flat: no MappingEntry composites inside.
    assert!(entries_of(fm).is_empty());
    // But the brace tokens and scalars are direct leaves.
    let leaf_kinds = token_kinds_of(fm);
    assert!(leaf_kinds.contains(&SyntaxKind::OpenBrace));
    assert!(leaf_kinds.contains(&SyntaxKind::CloseBrace));
    assert!(leaf_kinds.contains(&SyntaxKind::PlainScalar));
}

#[test]
fn flow_sequence_is_flat_in_phase_1() {
    let doc = parse_document("[a, b, c]\n").unwrap();
    let fs = first_node_of(doc.syntax(), SyntaxKind::FlowSequence).expect("FlowSequence");
    assert!(entries_of(fs).is_empty());
    let leaf_kinds = token_kinds_of(fs);
    assert!(leaf_kinds.contains(&SyntaxKind::OpenBracket));
    assert!(leaf_kinds.contains(&SyntaxKind::CloseBracket));
    assert!(
        leaf_kinds
            .iter()
            .filter(|k| **k == SyntaxKind::Comma)
            .count()
            == 2
    );
}

#[test]
fn comment_between_entries_attaches_to_prior() {
    let doc = parse_document("a: 1\n# between\nb: 2\n").unwrap();
    let bm = first_node_of(doc.syntax(), SyntaxKind::BlockMapping).expect("BlockMapping");
    let entries = entries_of(bm);
    assert_eq!(entries.len(), 2);
    // Phase 1 attachment: the own-line comment lives at the tail of
    // the prior entry. Verify there is at least one Comment inside
    // entry[0] and none inside entry[1].
    let has_comment = |e: &GreenNode| {
        e.children().any(|c| {
            matches!(
                c,
                GreenChild::Token {
                    kind: SyntaxKind::Comment,
                    ..
                }
            )
        })
    };
    assert!(has_comment(entries[0]), "expected comment in first entry");
    assert!(!has_comment(entries[1]), "second entry should be clean");
}

#[test]
fn explicit_question_indicator_opens_mapping_entry() {
    let doc = parse_document("? key\n: value\n").unwrap();
    let bm = first_node_of(doc.syntax(), SyntaxKind::BlockMapping).expect("BlockMapping");
    let entries = entries_of(bm);
    assert_eq!(entries.len(), 1);
    let entry = entries[0];
    let first = entry.children().next().expect("at least one child");
    match first {
        GreenChild::Token { kind, .. } => assert_eq!(*kind, SyntaxKind::QuestionIndicator),
        GreenChild::Node(_) => panic!("first child should be the `?` token"),
    }
}

#[test]
fn round_trip_holds_for_nested_structure() {
    // Sanity: the structural rewrite must not break the byte-faithful
    // round-trip property. Round-trip is covered exhaustively by
    // tests/cst_round_trip.rs but it is worth a smoke test here.
    let cases = [
        "a: 1\n",
        "outer:\n  inner: 1\n",
        "items:\n  - a\n  - b\n",
        "{a: 1, b: 2}\n",
        "[a, b, c]\n",
        "a: 1  # inline\nb: 2\n",
        "a: 1\n# between\nb: 2\n",
        "? key\n: value\n",
    ];
    for src in cases {
        let doc = parse_document(src).expect("parses");
        assert_eq!(doc.to_string(), src, "round-trip failed for {src:?}");
    }
}

#[test]
fn document_root_kinds_top_level() {
    // Sanity: at the document root, what we see is composites
    // (collections) plus possibly leading trivia / directive lines.
    let doc = parse_document("a: 1\n").unwrap();
    let ks = kinds(doc.syntax());
    assert!(ks.contains(&SyntaxKind::BlockMapping), "kinds: {ks:?}");
}
