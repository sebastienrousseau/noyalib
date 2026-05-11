// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Integration tests for comment capture via `load_comments`.

#![allow(clippy::unwrap_used)]

use noyalib::{CommentKind, load_comments};

#[test]
fn captures_full_line_comments() {
    let yaml = "# license header\n# author: sebs\nname: noyalib\n";
    let cs = load_comments(yaml).unwrap();
    assert_eq!(cs.len(), 2);
    assert!(cs[0].text.contains("license header"));
    assert!(cs[1].text.contains("author"));
    assert_eq!(cs[0].kind, CommentKind::Line);
    assert_eq!(cs[1].kind, CommentKind::Line);
}

#[test]
fn captures_inline_trailing() {
    let yaml = "port: 8080  # default HTTP\nhost: local  # bind\n";
    let cs = load_comments(yaml).unwrap();
    assert_eq!(cs.len(), 2);
    assert_eq!(cs[0].kind, CommentKind::Inline);
    assert_eq!(cs[1].kind, CommentKind::Inline);
    assert!(cs[0].text.contains("default"));
    assert!(cs[1].text.contains("bind"));
}

#[test]
fn spans_point_at_hash() {
    let yaml = "k: 1  # trail\n";
    let cs = load_comments(yaml).unwrap();
    assert_eq!(cs.len(), 1);
    // `#` is at byte 6.
    assert_eq!(&yaml[cs[0].start..cs[0].start + 1], "#");
    assert_eq!(cs[0].start, 6);
}

#[test]
fn end_stops_at_line_break() {
    let yaml = "# comment\nk: 1\n";
    let cs = load_comments(yaml).unwrap();
    assert_eq!(cs.len(), 1);
    // `# comment` is 9 bytes, no trailing newline in the range.
    assert_eq!(cs[0].end, 9);
    assert_eq!(&yaml[cs[0].start..cs[0].end], "# comment");
}

#[test]
fn empty_input_returns_no_comments() {
    let cs = load_comments("").unwrap();
    assert!(cs.is_empty());
}

#[test]
fn document_without_comments_returns_empty() {
    let cs = load_comments("a: 1\nb: 2\n").unwrap();
    assert!(cs.is_empty());
}

#[test]
fn hash_inside_quoted_string_is_not_a_comment() {
    let yaml = r#"k: "not a # comment"
"#;
    let cs = load_comments(yaml).unwrap();
    assert!(cs.is_empty());
}

#[test]
fn comments_inside_nested_mapping() {
    let yaml = "\
outer:
  # nested comment
  inner: value  # inline
";
    let cs = load_comments(yaml).unwrap();
    assert_eq!(cs.len(), 2);
    assert_eq!(cs[0].kind, CommentKind::Line);
    assert!(cs[0].text.contains("nested"));
    assert_eq!(cs[1].kind, CommentKind::Inline);
    assert!(cs[1].text.contains("inline"));
}

#[test]
fn comments_in_sequence() {
    let yaml = "\
- one    # first
- two    # second
- three  # third
";
    let cs = load_comments(yaml).unwrap();
    assert_eq!(cs.len(), 3);
    for c in &cs {
        assert_eq!(c.kind, CommentKind::Inline);
    }
}

#[test]
fn comment_crossref_with_spanned_field() {
    // The primary use case: find the comment attached to a specific
    // Spanned<T> field by byte-position proximity.
    use noyalib::Spanned;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Cfg {
        port: Spanned<u16>,
    }

    let yaml = "port: 8080  # default HTTP\n";
    let cfg: Cfg = noyalib::from_str(yaml).unwrap();
    let cs = load_comments(yaml).unwrap();

    // Find the first comment on the same line as `port`.
    let port_line = cfg.port.start.line();
    let on_same_line = cs.iter().find(|c| {
        let line = yaml[..c.start].matches('\n').count() + 1;
        line == port_line
    });
    assert!(on_same_line.is_some());
    assert!(on_same_line.unwrap().text.contains("default"));
}

#[test]
fn parse_error_propagates() {
    // Malformed YAML (unclosed flow seq) returns an error.
    let err = load_comments("k: [unclosed").unwrap_err();
    let _ = err.to_string();
}
