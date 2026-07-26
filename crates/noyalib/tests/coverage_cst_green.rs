// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for `GreenChild::token_text` in `cst/green.rs`. The method
//! is only exercised by a doctest, which `cargo llvm-cov` does not
//! instrument by default — so its Token / Node arms show uncovered. A
//! plain integration walk over a parsed CST hits both.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::cst::{GreenChild, GreenNode, parse_document};

fn walk(node: &GreenNode, src: &str, mut offset: usize, saw_token: &mut bool, saw_node: &mut bool) {
    for c in node.children() {
        match c {
            GreenChild::Token { .. } => {
                // Token arm returns the source slice for the token span.
                let t = c.token_text(src, offset).expect("token carries text");
                assert!(!t.is_empty(), "token text should be non-empty");
                *saw_token = true;
            }
            GreenChild::Node(inner) => {
                // Node arm returns None (a node is not a single token).
                assert_eq!(c.token_text(src, offset), None);
                *saw_node = true;
                walk(inner, src, offset, saw_token, saw_node);
            }
        }
        offset += c.text_len();
    }
}

#[test]
fn green_child_token_text_covers_both_arms() {
    let doc = parse_document("a: 1\n").unwrap();
    let mut saw_token = false;
    let mut saw_node = false;
    walk(doc.syntax(), doc.source(), 0, &mut saw_token, &mut saw_node);
    assert!(saw_token, "expected at least one Token child");
    assert!(saw_node, "expected at least one Node child");
}
