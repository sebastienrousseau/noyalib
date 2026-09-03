// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Comment-aware read view over a [`crate::cst::Document`].
//!
//! The CST already round-trips comments byte-for-byte through
//! [`Document::set`] and friends — touched spans are rewritten,
//! everything else (indentation, comments, blank lines) is left alone.
//! What this module adds is the *read* side: given a path-shaped
//! query, return the human-authored YAML comments that decorate the
//! node at that path. Tools (linters, AI agents reading config files,
//! IDE plugins) need that to understand what each field *means*.
//!
//! # Definitions
//!
//! - **Inline comment**: a `#`-introduced comment on the *same* line
//!   as the node's content, after that content. `key: val # inline`.
//! - **Leading comments**: a contiguous run of comment-only or blank
//!   lines immediately above the node's first byte. `# pre\nkey: val`
//!   yields one leading comment.
//!
//! # Examples
//!
//! ```
//! use noyalib::cst::parse_document;
//!
//! let src = "# top of file\nname: noyalib  # the project\n# next field\nversion: 0.0.1\n";
//! let doc = parse_document(src).unwrap();
//!
//! let name = doc.comments_at("name");
//! assert_eq!(name.before.len(), 1);
//! assert_eq!(name.before[0].text, " top of file");
//! assert_eq!(name.inline.as_ref().unwrap().text, " the project");
//!
//! let version = doc.comments_at("version");
//! assert_eq!(version.before.len(), 1);
//! assert_eq!(version.before[0].text, " next field");
//! assert!(version.inline.is_none());
//! ```

use crate::comments::Comment;
use crate::cst::Document;
use crate::error::{Error, Result};
use crate::prelude::*;

/// Comments that decorate a single node, organised by their position
/// relative to the node.
///
/// Returned by [`Document::comments_at`]. Both fields are empty /
/// `None` when no comments decorate the queried path or when the path
/// does not resolve to a node in the document.
///
/// # Examples
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let doc = parse_document("port: 8080  # the listen port\n").unwrap();
/// let bundle = doc.comments_at("port");
/// assert!(bundle.before.is_empty());
/// assert_eq!(bundle.inline.unwrap().text, " the listen port");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommentBundle {
    /// Contiguous run of comment lines that appear immediately above
    /// the node, in source order. An interleaved blank line does not
    /// break the run — only another content node does. Empty when
    /// the node has no leading comments.
    pub before: Vec<Comment>,
    /// The trailing comment on the node's own line, if any. `None`
    /// when no `#`-introduced comment follows the node's content on
    /// the same source line. Multi-line nodes (block mappings,
    /// block sequences) do not have an inline comment in this sense
    /// — query individual entries instead.
    pub inline: Option<Comment>,
}

impl CommentBundle {
    /// `true` when the bundle has neither a leading nor an inline
    /// comment. Convenience for the common "decide whether to render"
    /// branch in tooling.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.before.is_empty() && self.inline.is_none()
    }
}

impl Document {
    /// Comments decorating the node at `path`, classified by position.
    ///
    /// Returns an empty [`CommentBundle`] when `path` does not
    /// resolve. Path syntax matches [`Document::span_at`] —
    /// `foo.bar`, `items[0]`, `items[0].name`. Wildcard /
    /// recursive-descent segments are not supported (a non-singular
    /// span has no canonical "above" line).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let src = "# A multi-line\n# leading block\nport: 8080  # inline\n";
    /// let doc = parse_document(src).unwrap();
    ///
    /// let b = doc.comments_at("port");
    /// assert_eq!(b.before.len(), 2);
    /// assert_eq!(b.inline.as_ref().unwrap().text, " inline");
    /// ```
    #[must_use]
    pub fn comments_at(&self, path: &str) -> CommentBundle {
        let Some((start, end)) = self.span_at(path) else {
            return CommentBundle::default();
        };

        // Comment scanning is independent of edits — always run on the
        // current source. The comment count is small (one per line at
        // most), so per-call scanning is cheaper than caching.
        let comments = match crate::comments::load_comments(self.source()) {
            Ok(c) => c,
            // A document that successfully parsed cannot fail comment
            // scan — but if it does, treat as no comments rather than
            // bubbling up: this is a read-only convenience API.
            Err(_) => return CommentBundle::default(),
        };

        let src = self.source();
        let line_start_idx = line_start(src, start);
        let line_end_idx = line_end(src, end.saturating_sub(1).max(start));

        let mut bundle = CommentBundle::default();

        // Inline only applies to single-line nodes — a multi-line
        // block (mapping, sequence) does not have a single inline
        // comment of its own; query individual entries instead.
        let is_single_line = !src[start..end].contains('\n');
        if is_single_line {
            for c in &comments {
                if c.start >= end && c.start <= line_end_idx {
                    bundle.inline = Some(c.clone());
                    break;
                }
            }
        }

        // Leading: walk *upwards* from the line containing `start`,
        // collecting comment-only lines and skipping pure-blank lines.
        // Stop at the first line containing non-comment content (or at
        // the start of input).
        let mut cursor = line_start_idx;
        let mut acc: Vec<Comment> = Vec::new();
        while cursor > 0 {
            // `cursor` points at the first byte of a line; step back
            // to the previous line's start.
            let prev_line_end = cursor - 1; // the '\n' that ended the previous line
            let prev_line_start = line_start(src, prev_line_end.saturating_sub(1));
            let line_text = &src[prev_line_start..prev_line_end];
            let trimmed = line_text.trim_start_matches([' ', '\t']);

            if trimmed.is_empty() {
                // Blank line — does not break the run, does not
                // contribute a comment.
                cursor = prev_line_start;
                continue;
            }
            if trimmed.starts_with('#') {
                // Comment-only line. Find the matching scanned comment
                // (we already have spans for all of them).
                if let Some(c) = comments
                    .iter()
                    .find(|c| c.start >= prev_line_start && c.start < prev_line_end)
                {
                    acc.push(c.clone());
                }
                cursor = prev_line_start;
                continue;
            }
            // Content line — stop walking up.
            break;
        }

        // Walked upward, so the natural order is bottom-up; reverse to
        // restore source order.
        acc.reverse();
        bundle.before = acc;

        bundle
    }

    /// Set (or replace) the **inline** comment on the single-line node
    /// at `path` — the `#`-introduced comment that follows the value on
    /// the same line.
    ///
    /// `text` is the comment body without the leading `#`; it renders as
    /// `# <text>` (a single space after `#`, or a bare `#` when `text`
    /// is empty). If the node already has an inline comment, its body is
    /// replaced in place, keeping the existing separating whitespace;
    /// otherwise `  # <text>` is appended after the value.
    ///
    /// Guarded like the other mutators: the edit must re-parse and leave
    /// the document's typed value unchanged (a comment carries no data),
    /// or it is rolled back.
    ///
    /// # Errors
    ///
    /// - `path` does not resolve to a node.
    /// - The node spans multiple lines — it has no inline comment of its
    ///   own; comment its entries instead.
    /// - `text` contains a newline (a comment is a single line).
    /// - The splice would not re-parse or would change data (roll back).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("port: 8080\n").unwrap();
    /// doc.set_inline_comment("port", "the listen port").unwrap();
    /// assert_eq!(doc.source(), "port: 8080  # the listen port\n");
    /// doc.set_inline_comment("port", "changed").unwrap();
    /// assert_eq!(doc.source(), "port: 8080  # changed\n");
    /// ```
    pub fn set_inline_comment(&mut self, path: &str, text: &str) -> Result<()> {
        if text.contains('\n') {
            return Err(Error::Parse(format!(
                "set_inline_comment: comment text for `{path}` contains a newline; \
                 an inline comment is a single line"
            )));
        }
        let Some((start, end)) = self.span_at(path) else {
            return Err(Error::Parse(format!(
                "set_inline_comment: path `{path}` did not resolve to a node"
            )));
        };
        if self.source()[start..end].contains('\n') {
            return Err(Error::Parse(format!(
                "set_inline_comment: `{path}` is a multi-line node and has no inline \
                 comment of its own; comment its individual entries instead"
            )));
        }

        let rendered = if text.is_empty() {
            "#".to_string()
        } else {
            format!("# {text}")
        };
        let existing = self.comments_at(path).inline;
        let snapshot = self.clone();
        let expected = self.as_value().clone();

        let splice = match existing {
            Some(c) => self.replace_span(c.start, c.end, &rendered),
            None => self.replace_span(end, end, &format!("  {rendered}")),
        };
        self.finish_comment_edit("set_inline_comment", path, splice, snapshot, &expected)
    }

    /// Remove the **inline** comment on the node at `path`, if any,
    /// taking the separating whitespace with it so no trailing space is
    /// left. A no-op returning `Ok(())` when the node has no inline
    /// comment (or the path does not resolve).
    ///
    /// Guarded and rolled back exactly like
    /// [`set_inline_comment`](Self::set_inline_comment).
    ///
    /// # Errors
    ///
    /// - The removal would not re-parse or would change data (rolls
    ///   back). A missing comment or path is a no-op, not an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("port: 8080  # noise\n").unwrap();
    /// doc.remove_inline_comment("port").unwrap();
    /// assert_eq!(doc.source(), "port: 8080\n");
    /// ```
    pub fn remove_inline_comment(&mut self, path: &str) -> Result<()> {
        let Some((_start, end)) = self.span_at(path) else {
            return Ok(());
        };
        let Some(c) = self.comments_at(path).inline else {
            return Ok(());
        };
        let snapshot = self.clone();
        let expected = self.as_value().clone();
        // The value ends at `end`; the bytes from there to `c.end` are
        // the separating whitespace plus the `# …` comment.
        let splice = self.replace_span(end, c.end, "");
        self.finish_comment_edit("remove_inline_comment", path, splice, snapshot, &expected)
    }

    /// Set (or replace) the **leading** comment block above the
    /// single-line mapping entry at `path` — the run of comment lines
    /// that `comments_at(path).before` reports.
    ///
    /// `text` becomes one comment line per `\n`-separated segment, each
    /// rendered at the key's indentation as `# <segment>` (a bare `#`
    /// for an empty segment). An existing leading block is replaced in
    /// place; otherwise the block is inserted immediately above the
    /// entry's line.
    ///
    /// Scope: block **mapping keys** on a single line (where the key
    /// token, and therefore the entry's own line and indent, are
    /// unambiguous). Multi-line / nested entries and sequence items are
    /// a follow-up — `comments_at` does not attribute a leading block to
    /// them unambiguously. Guarded like the other mutators: the edit
    /// must re-parse and leave the typed value unchanged, or it rolls
    /// back.
    ///
    /// # Errors
    ///
    /// - `path` does not address a single-line block-mapping key.
    /// - The splice would not re-parse or would change data (roll back).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("port: 8080\n").unwrap();
    /// doc.set_leading_comment("port", "the listen port").unwrap();
    /// assert_eq!(doc.source(), "# the listen port\nport: 8080\n");
    /// doc.set_leading_comment("port", "line one\nline two").unwrap();
    /// assert_eq!(doc.source(), "# line one\n# line two\nport: 8080\n");
    /// ```
    pub fn set_leading_comment(&mut self, path: &str, text: &str) -> Result<()> {
        let (key_start, entry_line_start, indent) = self.leading_comment_site(path)?;
        let _ = key_start;
        let nl = comment_line_break(self.source());
        let rendered: String = text
            .split('\n')
            .map(|line| {
                if line.is_empty() {
                    format!("{indent}#{nl}")
                } else {
                    format!("{indent}# {line}{nl}")
                }
            })
            .collect();

        let before = self.comments_at(path).before;
        let snapshot = self.clone();
        let expected = self.as_value().clone();

        let splice = match (before.first(), before.last()) {
            (Some(first), Some(last)) => {
                let block_start = line_start(self.source(), first.start);
                let block_end = line_end(self.source(), last.start) + 1;
                self.replace_span(block_start, block_end, &rendered)
            }
            _ => self.replace_span(entry_line_start, entry_line_start, &rendered),
        };
        self.finish_comment_edit("set_leading_comment", path, splice, snapshot, &expected)
    }

    /// Remove the **leading** comment block above the mapping entry at
    /// `path`, if any. A no-op returning `Ok(())` when there is none (or
    /// the path does not address a single-line mapping key).
    ///
    /// Guarded and rolled back exactly like
    /// [`set_leading_comment`](Self::set_leading_comment).
    ///
    /// # Errors
    ///
    /// - The removal would not re-parse or would change data (rolls
    ///   back).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("# noise\n# more\nport: 8080\n").unwrap();
    /// doc.remove_leading_comment("port").unwrap();
    /// assert_eq!(doc.source(), "port: 8080\n");
    /// ```
    pub fn remove_leading_comment(&mut self, path: &str) -> Result<()> {
        // A path that is not a single-line mapping key simply has no
        // leading block this method owns — treat as a no-op.
        if self.leading_comment_site(path).is_err() {
            return Ok(());
        }
        let before = self.comments_at(path).before;
        let (Some(first), Some(last)) = (before.first(), before.last()) else {
            return Ok(());
        };
        let block_start = line_start(self.source(), first.start);
        let block_end = line_end(self.source(), last.start) + 1;
        let snapshot = self.clone();
        let expected = self.as_value().clone();
        let splice = self.replace_span(block_start, block_end, "");
        self.finish_comment_edit("remove_leading_comment", path, splice, snapshot, &expected)
    }

    /// Resolve `path` to a single-line block-mapping key and return its
    /// key-token start, its line start, and the indent (whitespace)
    /// prefix of that line. `Err` for anything that is not such a key.
    fn leading_comment_site(&self, path: &str) -> Result<(usize, usize, String)> {
        let Some((key_start, _key_end)) = self.key_span(path) else {
            return Err(Error::Parse(format!(
                "leading comment: `{path}` does not address a block-mapping key"
            )));
        };
        let Some((vstart, vend)) = self.span_at(path) else {
            return Err(Error::Parse(format!(
                "leading comment: `{path}` did not resolve to a value"
            )));
        };
        if self.source()[vstart..vend].contains('\n') {
            return Err(Error::Parse(format!(
                "leading comment: `{path}` is a multi-line entry; leading-comment \
                 mutation is limited to single-line mapping keys in this phase"
            )));
        }
        let ls = line_start(self.source(), key_start);
        let indent = self.source()[ls..key_start].to_string();
        Ok((key_start, ls, indent))
    }

    /// Shared tail for the comment mutators: apply the re-parse and
    /// value-unchanged guard, rolling back to `snapshot` on any failure.
    fn finish_comment_edit(
        &mut self,
        op: &str,
        path: &str,
        splice: Result<()>,
        snapshot: Self,
        expected: &crate::Value,
    ) -> Result<()> {
        if let Err(e) = splice {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "{op}: editing the comment on `{path}` could not be spliced ({e}); \
                 the document was left unchanged"
            )));
        }
        if let Err(e) = self.validate() {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "{op}: editing the comment on `{path}` left the document unable to \
                 re-parse ({e}); the document was left unchanged"
            )));
        }
        if *self.as_value() != *expected {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "{op}: editing the comment on `{path}` changed the document's data; \
                 the document was left unchanged"
            )));
        }
        Ok(())
    }
}

#[inline]
fn line_start(src: &str, byte: usize) -> usize {
    let bytes = src.as_bytes();
    let mut i = byte.min(bytes.len());
    while i > 0 && bytes[i - 1] != b'\n' {
        i -= 1;
    }
    i
}

#[inline]
fn line_end(src: &str, byte: usize) -> usize {
    let bytes = src.as_bytes();
    let mut i = byte.min(bytes.len().saturating_sub(1));
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

/// Where a comment sits relative to the node it decorates.
///
/// Mirrors the two positions [`CommentBundle`] reports, so a read with
/// [`Document::comments_at`] and a write with
/// [`Document::set_comment`] address the same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentPosition {
    /// The trailing comment on the node's own line.
    Inline,
    /// The contiguous run of comment lines immediately above the node.
    Before,
}

impl Document {
    /// Set the comment at `path` in the given position.
    ///
    /// Replaces an existing comment, or writes a new one when there is
    /// none. `text` is the comment body without the leading `#`; a
    /// single leading space is added when `text` does not already begin
    /// with whitespace, so `set_comment(p, Inline, "note")` yields
    /// `# note`.
    ///
    /// For [`CommentPosition::Before`], a `text` containing newlines
    /// becomes one `#` line per line, each at the node's own
    /// indentation.
    ///
    /// The edit goes through [`Document::replace_span`], so it inherits
    /// the same guard: an edit that would make the document re-parse
    /// differently is rejected rather than written.
    ///
    /// # Errors
    ///
    /// - `path` does not resolve to a node.
    /// - The resulting document would not re-parse to the same value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::{parse_document, CommentPosition};
    ///
    /// let mut doc = parse_document("port: 8080\n").unwrap();
    /// doc.set_comment("port", CommentPosition::Inline, "listen port").unwrap();
    /// assert_eq!(doc.source(), "port: 8080  # listen port\n");
    /// ```
    pub fn set_comment(&mut self, path: &str, position: CommentPosition, text: &str) -> Result<()> {
        let p = path.to_owned();
        let txt = text.to_owned();
        self.guarded_comment_edit("set_comment", move |d| {
            d.set_comment_inner(&p, position, &txt)
        })
    }

    fn set_comment_inner(
        &mut self,
        path: &str,
        position: CommentPosition,
        text: &str,
    ) -> Result<()> {
        // A comment body cannot contain a line break: a bare `\r` (or
        // `\r\n`) ends the comment token in YAML, so the remainder of
        // the text would land *outside* the comment and change — or
        // break — the document (found by fuzz_editors: an inline text
        // of "t[[\r---" split the document in two). `Before` accepts
        // `\n` by documented design (one `#` line per line) but the
        // same carriage-return breakout applies to each line.
        let breaking = match position {
            CommentPosition::Inline => text.contains('\n') || text.contains('\r'),
            CommentPosition::Before => text.contains('\r'),
        };
        if breaking {
            return Err(Error::Parse(format!(
                "set_comment: comment text for `{path}` contains a line \
                 break that would end the comment token and leak into the \
                 document; inline comments take a single line, and Before \
                 comments split on `\n` only"
            )));
        }
        let Some((start, end)) = self.span_at(path) else {
            return Err(Error::Parse(format!(
                "set_comment: path `{path}` does not resolve"
            )));
        };
        let bundle = self.comments_at(path);

        match position {
            CommentPosition::Inline => {
                let body = normalise_body(text);
                if let Some(existing) = bundle.inline {
                    self.replace_span(existing.start, existing.end, &format!("#{body}"))
                } else {
                    // No inline comment yet: append at the end of the
                    // node's own line, before the *whole* break. Landing
                    // on the `\n` of a `\r\n` would splice between the
                    // two and strand a lone `\r`.
                    let line_end = line_break_start(self.source(), end);
                    self.replace_span(line_end, line_end, &format!("  #{body}"))
                }
            }
            CommentPosition::Before => {
                let indent = indent_of_line_containing(self.source(), start);
                let nl = comment_line_break(self.source());
                let block: String = text
                    .split('\n')
                    .map(|l| format!("{indent}#{}{nl}", normalise_body(l)))
                    .collect();
                if let (Some(first), Some(last)) = (bundle.before.first(), bundle.before.last()) {
                    // Replace the existing run, including the newline
                    // that terminates its last line.
                    let end_of_run = line_end_from(self.source(), last.end) + 1;
                    let start_of_run = line_start_from(self.source(), first.start);
                    self.replace_span(start_of_run, end_of_run.min(self.source().len()), &block)
                } else {
                    let line_start = line_start_from(self.source(), start);
                    self.replace_span(line_start, line_start, &block)
                }
            }
        }
    }

    /// Remove the comment at `path` in the given position.
    ///
    /// A no-op when there is no comment there. For
    /// [`CommentPosition::Inline`] the whitespace separating the
    /// comment from the node's content goes with it, so no trailing
    /// spaces are left behind. For [`CommentPosition::Before`] the
    /// whole run of comment lines is removed.
    ///
    /// # Errors
    ///
    /// As [`Document::set_comment`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::{parse_document, CommentPosition};
    ///
    /// let mut doc = parse_document("port: 8080  # note\n").unwrap();
    /// doc.remove_comment("port", CommentPosition::Inline).unwrap();
    /// assert_eq!(doc.source(), "port: 8080\n");
    /// ```
    pub fn remove_comment(&mut self, path: &str, position: CommentPosition) -> Result<()> {
        let p = path.to_owned();
        self.guarded_comment_edit("remove_comment", move |doc| {
            doc.remove_comment_inner(&p, position)
        })
    }

    fn remove_comment_inner(&mut self, path: &str, position: CommentPosition) -> Result<()> {
        if self.span_at(path).is_none() {
            return Err(Error::Parse(format!(
                "remove_comment: path `{path}` does not resolve"
            )));
        }
        let bundle = self.comments_at(path);
        match position {
            CommentPosition::Inline => {
                let Some(c) = bundle.inline else {
                    return Ok(());
                };
                // Take the run of whitespace before the `#` too.
                let from = trim_back_whitespace(self.source(), c.start);
                self.replace_span(from, c.end, "")
            }
            CommentPosition::Before => {
                let (Some(first), Some(last)) = (bundle.before.first(), bundle.before.last())
                else {
                    return Ok(());
                };
                let start_of_run = line_start_from(self.source(), first.start);
                let end_of_run =
                    (line_end_from(self.source(), last.end) + 1).min(self.source().len());
                self.replace_span(start_of_run, end_of_run, "")
            }
        }
    }
}

impl Document {
    /// Run a comment edit, then require the document's *value* to be
    /// unchanged.
    ///
    /// Comments are trivia: an edit to them that alters what the
    /// document means is a bug by definition. Enforcing that as an
    /// invariant beats reasoning about every context, because the
    /// contexts are not obvious. A folded block scalar is the case that
    /// found this:
    ///
    /// ```text
    /// >        set_comment("", Inline, "")        >  #
    /// ```
    ///
    /// Appending `  #` to a block scalar does not write a comment — the
    /// text becomes scalar *content*, and the value changes from `">"`
    /// to `">  #"`. The `fuzz_editors` target found it within a minute
    /// of existing.
    fn guarded_comment_edit<F>(&mut self, what: &str, edit: F) -> Result<()>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        // Both loads run under the document's own configuration, so a
        // document that only opens with a relaxed budget is guarded
        // too instead of silently skipping the check.
        let before = crate::parser::parse_one_value(self.source(), self.config()).ok();
        let snapshot = self.clone();
        edit(self)?;

        if let Some(before) = before {
            let unchanged = matches!(
                crate::parser::parse_one_value(self.source(), self.config()),
                Ok(a) if a == before
            );
            if !unchanged {
                *self = snapshot;
                return Err(Error::Parse(format!(
                    "{what}: the edit would change the document's value, not just its \
                     comments — it was left unchanged. This happens where a `#` is not \
                     a comment, such as inside a block scalar."
                )));
            }
        }
        Ok(())
    }
}

/// Give a comment body exactly one leading space unless it already
/// starts with whitespace, so `"note"` and `" note"` both render as
/// `# note`.
fn normalise_body(text: &str) -> String {
    if text.starts_with(char::is_whitespace) || text.is_empty() {
        text.to_owned()
    } else {
        format!(" {text}")
    }
}

/// Byte index of the newline ending the line containing `idx`, or the
/// end of the source when the last line is unterminated.
fn line_end_from(src: &str, idx: usize) -> usize {
    src[idx..].find('\n').map_or(src.len(), |n| idx + n)
}

/// Byte index where the line break ending the line containing `idx`
/// *begins* — the `\r` of a `\r\n`, not the `\n`.
///
/// [`line_end_from`] answers "where is the `\n`", which is the right
/// question for reading a line and the wrong one for appending to it: an
/// insertion there lands between the `\r` and the `\n`.
fn line_break_start(src: &str, idx: usize) -> usize {
    let end = line_end_from(src, idx);
    if end > idx && src.as_bytes()[end - 1] == b'\r' {
        end - 1
    } else {
        end
    }
}

/// The line break to give a comment line this edit adds.
///
/// `"\r\n"` only when the document is wholly CRLF — every `\n` preceded
/// by a `\r`. A mixed document has no convention to honour, so it keeps
/// the `"\n"` default rather than being rewritten to a guess.
fn comment_line_break(src: &str) -> &'static str {
    let bytes = src.as_bytes();
    let mut saw = false;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            if i == 0 || bytes[i - 1] != b'\r' {
                return "\n";
            }
            saw = true;
        }
    }
    if saw { "\r\n" } else { "\n" }
}

/// Byte index just after the newline preceding `idx`, i.e. the start of
/// the line containing it.
fn line_start_from(src: &str, idx: usize) -> usize {
    src[..idx].rfind('\n').map_or(0, |n| n + 1)
}

/// Walk back over spaces and tabs immediately before `idx`.
fn trim_back_whitespace(src: &str, idx: usize) -> usize {
    let bytes = src.as_bytes();
    let mut i = idx;
    while i > 0 && (bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') {
        i -= 1;
    }
    i
}

/// The leading whitespace of the line containing `idx`.
fn indent_of_line_containing(src: &str, idx: usize) -> String {
    let start = line_start_from(src, idx);
    src[start..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parse_document;

    #[test]
    fn inline_comment_on_simple_value() {
        let doc = parse_document("port: 8080  # the listen port\n").unwrap();
        let b = doc.comments_at("port");
        assert!(b.before.is_empty());
        assert_eq!(b.inline.as_ref().unwrap().text, " the listen port");
    }

    #[test]
    fn leading_single_comment() {
        let doc = parse_document("# pre\nkey: val\n").unwrap();
        let b = doc.comments_at("key");
        assert_eq!(b.before.len(), 1);
        assert_eq!(b.before[0].text, " pre");
        assert!(b.inline.is_none());
    }

    #[test]
    fn leading_multi_with_blank_lines_preserves_run() {
        let doc = parse_document(
            "# first\n\
             \n\
             # second\n\
             key: val\n",
        )
        .unwrap();
        let b = doc.comments_at("key");
        assert_eq!(b.before.len(), 2);
        assert_eq!(b.before[0].text, " first");
        assert_eq!(b.before[1].text, " second");
    }

    #[test]
    fn content_line_breaks_leading_run() {
        let doc = parse_document(
            "name: noyalib\n\
             # this comment belongs to version, not name\n\
             version: 0.0.1\n",
        )
        .unwrap();
        let name = doc.comments_at("name");
        assert!(name.before.is_empty());
        let version = doc.comments_at("version");
        assert_eq!(version.before.len(), 1);
        assert!(version.before[0].text.contains("belongs to version"));
    }

    #[test]
    fn nested_path_inline_comment() {
        let doc = parse_document(
            "server:\n\
             \x20 host: localhost  # bind address\n\
             \x20 port: 8080\n",
        )
        .unwrap();
        let host = doc.comments_at("server.host");
        assert_eq!(host.inline.as_ref().unwrap().text, " bind address");
        let port = doc.comments_at("server.port");
        assert!(port.inline.is_none());
    }

    #[test]
    fn unknown_path_returns_empty_bundle() {
        let doc = parse_document("a: 1\n").unwrap();
        let b = doc.comments_at("nonexistent");
        assert!(b.is_empty());
    }

    #[test]
    fn comments_survive_lossless_edit() {
        let mut doc = parse_document(
            "# version is bumped by Renovate\n\
             version: 0.0.1  # do not edit by hand\n",
        )
        .unwrap();
        doc.set("version", "0.0.2").unwrap();
        let b = doc.comments_at("version");
        assert_eq!(b.before.len(), 1);
        assert_eq!(b.before[0].text, " version is bumped by Renovate");
        assert_eq!(b.inline.as_ref().unwrap().text, " do not edit by hand");
        assert!(doc.to_string().contains("version: 0.0.2"));
        assert!(doc.to_string().contains("# version is bumped by Renovate"));
        assert!(doc.to_string().contains("# do not edit by hand"));
    }

    #[test]
    fn multiline_block_does_not_inherit_child_inline() {
        // Bug guard: querying a multi-line block must not return the
        // inline comment of its last child entry. The user's mental
        // model of `comments_at("server")` is "comments on `server`",
        // not "comments on `server.port`".
        let doc =
            parse_document("server:\n  host: localhost\n  port: 8080  # main HTTP port\n").unwrap();
        let server = doc.comments_at("server");
        assert!(
            server.inline.is_none(),
            "block must not inherit child inline"
        );
        let port = doc.comments_at("server.port");
        assert_eq!(port.inline.as_ref().unwrap().text, " main HTTP port");
    }

    #[test]
    fn sequence_item_inline_comment() {
        let doc = parse_document(
            "items:\n\
             \x20 - one  # the first\n\
             \x20 - two  # the second\n",
        )
        .unwrap();
        let first = doc.comments_at("items[0]");
        assert_eq!(first.inline.as_ref().unwrap().text, " the first");
        let second = doc.comments_at("items[1]");
        assert_eq!(second.inline.as_ref().unwrap().text, " the second");
    }
}
