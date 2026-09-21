// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Comment capture on the parse path.
//!
//! YAML comments are outside the data model — the standard parser
//! drops them silently. This module exposes a *capture* API so tools
//! that need to preserve human-authored context (migration scripts,
//! config linters, documentation generators) can read comments out
//! of a source document alongside the parsed `Value`.
//!
//! # Scope
//!
//! This is the MVP: capture-only. The functions here return a
//! read-only `Vec<Comment>` with source spans and a crude classifier
//! ([`CommentKind::Line`] vs [`CommentKind::Inline`]) you can cross-
//! reference against [`Spanned<T>`](crate::Spanned) to associate
//! comments with specific fields.
//!
//! # Not (yet) in scope
//!
//! Emit-time round-tripping — i.e. parsing a YAML document with
//! comments into `Value`, then re-serialising it with the comments
//! back in the right places — is a separate, larger undertaking and
//! is tracked as a follow-up. The building blocks are here: the
//! scanner now preserves comment spans, so a future commit can layer
//! an AST side-table on top without re-plumbing the parser.
//!
//! # Examples
//!
//! ```
//! use noyalib::{load_comments, CommentKind};
//!
//! let yaml = "# top-of-file header\nname: noyalib  # inline\nversion: 0.0.1\n";
//! let comments = load_comments(yaml).unwrap();
//!
//! assert_eq!(comments.len(), 2);
//! assert_eq!(comments[0].kind, CommentKind::Line);
//! assert!(comments[0].text.contains("top-of-file"));
//! assert_eq!(comments[1].kind, CommentKind::Inline);
//! assert!(comments[1].text.contains("inline"));
//! ```

use crate::error::{Error, Result};
use crate::parser::Parser;
use crate::prelude::*;

/// Classification of a scanned comment.
///
/// # Examples
///
/// ```
/// use noyalib::CommentKind;
/// let k = CommentKind::Inline;
/// assert_eq!(k, CommentKind::Inline);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommentKind {
    /// A standalone comment occupying its own line (possibly indented).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::CommentKind;
    /// let _ = CommentKind::Line;
    /// ```
    Line,

    /// A comment trailing content on the same line, e.g.
    /// `key: value  # note`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::CommentKind;
    /// let _ = CommentKind::Inline;
    /// ```
    Inline,
}

/// A scanned YAML comment with its source span and classification.
///
/// `start` and `end` are byte offsets into the original input. The
/// range points at the `#` through the character before the line
/// break (or end of input).
///
/// # Examples
///
/// ```
/// use noyalib::load_comments;
/// let comments = load_comments("# hi\n").unwrap();
/// assert_eq!(comments[0].text, " hi");
/// assert_eq!(comments[0].start, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// Text of the comment excluding the leading `#`.
    ///
    /// Leading whitespace between the `#` and the comment body is
    /// preserved so callers can reconstruct exact formatting.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::load_comments;
    /// let c = &load_comments("# hello\n").unwrap()[0];
    /// assert_eq!(c.text, " hello");
    /// ```
    pub text: String,

    /// Byte offset of the leading `#` in the source.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::load_comments;
    /// let c = &load_comments("k: 1  # x\n").unwrap()[0];
    /// assert_eq!(c.start, 6);
    /// ```
    pub start: usize,

    /// Byte offset one past the last byte of the comment text.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::load_comments;
    /// let c = &load_comments("# x\n").unwrap()[0];
    /// assert_eq!(c.end, 3);
    /// ```
    pub end: usize,

    /// Classification: line-level or inline-trailing.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{load_comments, CommentKind};
    /// let cs = load_comments("# hi\nk: 1  # inl\n").unwrap();
    /// assert_eq!(cs[0].kind, CommentKind::Line);
    /// assert_eq!(cs[1].kind, CommentKind::Inline);
    /// ```
    pub kind: CommentKind,
}

/// Scan a YAML document and return the list of comments it contains.
///
/// The source is driven through the event parser so the returned
/// positions stay consistent with the spans on [`Spanned<T>`](crate::Spanned).
/// Comments are returned in source order.
///
/// # Errors
///
/// Returns an error if the input is not a lexically valid YAML
/// document — e.g. an unclosed flow collection. Comments inside an
/// otherwise-well-formed document are captured even if deeper parse
/// stages (e.g. merge resolution) would later fail.
///
/// # Examples
///
/// ```
/// use noyalib::{load_comments, CommentKind};
/// let yaml = "name: noyalib  # the library\n# trailing\n";
/// let comments = load_comments(yaml).unwrap();
/// assert_eq!(comments.len(), 2);
/// assert_eq!(comments[0].kind, CommentKind::Inline);
/// assert_eq!(comments[1].kind, CommentKind::Line);
/// ```
pub fn load_comments(input: &str) -> Result<Vec<Comment>> {
    let mut parser = Parser::new(input);
    parser.enable_comment_capture();
    // Drain events — we don't care about the tree, just need the
    // scanner to walk the whole document so every comment gets
    // captured.
    loop {
        match parser.next_event() {
            Ok(ev) => {
                if matches!(ev, crate::parser::Event::StreamEnd) {
                    break;
                }
            }
            Err(e) => {
                return Err(Error::parse_at(&*e.message, input, e.index));
            }
        }
    }
    Ok(parser.take_comments())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_document_has_no_comments() {
        assert_eq!(load_comments("").unwrap(), vec![]);
        assert_eq!(load_comments("k: 1\n").unwrap(), vec![]);
    }

    #[test]
    fn single_line_comment() {
        let cs = load_comments("# hello\n").unwrap();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].text, " hello");
        assert_eq!(cs[0].kind, CommentKind::Line);
        assert_eq!(cs[0].start, 0);
        assert_eq!(cs[0].end, 7);
    }

    #[test]
    fn inline_comment_classification() {
        let cs = load_comments("k: 1  # trailing\n").unwrap();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].kind, CommentKind::Inline);
        assert!(cs[0].text.contains("trailing"));
    }

    #[test]
    fn mixed_leading_and_inline() {
        let yaml = "# top\nk: 1  # tail\n# bottom\n";
        let cs = load_comments(yaml).unwrap();
        assert_eq!(cs.len(), 3);
        assert_eq!(cs[0].kind, CommentKind::Line);
        assert_eq!(cs[1].kind, CommentKind::Inline);
        assert_eq!(cs[2].kind, CommentKind::Line);
    }

    #[test]
    fn indented_line_comment_is_still_line() {
        // A comment indented by spaces on an otherwise-empty line
        // should classify as Line (no preceding content on this line).
        let cs = load_comments("k:\n  # indented\n  v: 1\n").unwrap();
        assert!(!cs.is_empty());
        assert_eq!(cs[0].kind, CommentKind::Line);
    }

    #[test]
    fn source_order_preserved() {
        let yaml = "# a\n# b\n# c\n";
        let cs = load_comments(yaml).unwrap();
        assert_eq!(cs.len(), 3);
        assert!(cs[0].start < cs[1].start);
        assert!(cs[1].start < cs[2].start);
    }
}
