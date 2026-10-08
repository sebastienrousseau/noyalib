// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A deeply nested edit fragment must be refused with an error, not
//! overflow the stack, on every edit entry point.

use noyalib::cst::parse_document;

const DEPTH: usize = 100_000;

fn deep_flow(depth: usize) -> String {
    let mut s = String::with_capacity(depth * 2);
    s.push_str(&"[".repeat(depth));
    s.push_str(&"]".repeat(depth));
    s
}

/// Run `f` on a thread with a 1 MiB stack, the size a small worker
/// thread gets, so unbounded recursion aborts the process.
fn on_small_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("join");
}

#[test]
fn set_with_deep_fragment_is_refused() {
    on_small_stack(|| {
        let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
        let before = doc.source().to_owned();
        assert!(doc.set("a", &deep_flow(DEPTH)).is_err());
        assert_eq!(doc.source(), before);
    });
}

#[test]
fn replace_span_with_deep_fragment_is_refused() {
    on_small_stack(|| {
        let mut doc = parse_document("a: 1\nb: 2\n").unwrap();
        let before = doc.source().to_owned();
        let (s, e) = doc.span_at("a").unwrap();
        assert!(doc.replace_span(s, e, &deep_flow(DEPTH)).is_err());
        assert_eq!(doc.source(), before);
    });
}

#[test]
fn replace_span_with_deep_block_entry_is_refused() {
    on_small_stack(|| {
        let mut doc = parse_document("a:\n  x: 1\nb: 2\n").unwrap();
        let before = doc.source().to_owned();
        let (s, e) = doc.span_at("a.x").unwrap();
        assert!(doc.replace_span(s, e, &deep_flow(DEPTH)).is_err());
        assert_eq!(doc.source(), before);
    });
}

#[test]
fn parse_document_with_deep_text_errors() {
    on_small_stack(|| {
        let src = format!("a: {}\n", deep_flow(DEPTH));
        assert!(parse_document(&src).is_err());
    });
}

#[test]
fn dropping_a_deep_green_tree_does_not_recurse() {
    use noyalib::cst::{GreenChild, GreenNode, SyntaxKind};
    on_small_stack(|| {
        let mut node = GreenNode::new(
            SyntaxKind::FlowSequence,
            vec![GreenChild::Token {
                kind: SyntaxKind::PlainScalar,
                len: 1,
            }],
        );
        for _ in 0..DEPTH {
            node = GreenNode::new(SyntaxKind::FlowSequence, vec![GreenChild::Node(node)]);
        }
        assert_eq!(node.text_len(), 1);
        drop(node);
    });
}

#[test]
fn every_fragment_editor_refuses_a_deep_fragment() {
    on_small_stack(|| {
        let src = "m:\n  a: 1\ns:\n  - 1\n";
        let deep = deep_flow(DEPTH);
        type Edit = fn(&mut noyalib::cst::Document, &str) -> noyalib::Result<()>;
        let edits: [(&str, Edit); 4] = [
            ("set", |d, f| d.set("m.a", f)),
            ("insert_entry", |d, f| d.insert_entry("m", "b", f)),
            ("push_back", |d, f| d.push_back("s", f)),
            ("insert_after", |d, f| d.insert_after("s[0]", f)),
        ];
        for (name, edit) in edits {
            let mut doc = parse_document(src).unwrap();
            assert!(edit(&mut doc, &deep).is_err(), "{name} accepted");
            assert_eq!(doc.source(), src, "{name} changed the document");
        }
    });
}
