// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Build the parts of a [`crate::cst::Document`] from input bytes.
//!
//! Green-tree leaves carry their byte length only, with no
//! absolute range. The owning [`crate::cst::Document`] holds the
//! source `Arc<str>`. Splicing a sub-tree only rewrites the path
//! from the root down to the splice target's parent — pre- and
//! post-splice subtrees are reused via cheap `Arc<[GreenChild]>`
//! clones.

use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::is_yaml_blank;
use crate::cst::syntax::SyntaxKind;
use crate::error::{Error, Result};
#[cfg(feature = "std")]
use crate::parser::ParseConfig;
use crate::parser::{
    RecordedToken, RecordedTokenKind, ScannedComment, Scanner, TokenKind, Trivia, TriviaKind,
};
use crate::prelude::*;
#[cfg(feature = "std")]
use crate::span_context::SpanTree;
use crate::value::Value;

/// Outcome of a green-tree-aware parse.
#[cfg(feature = "std")]
pub(crate) struct ParsedDocument {
    pub(crate) green: GreenNode,
    pub(crate) value: Value,
    pub(crate) span_tree: SpanTree,
    pub(crate) source: Arc<str>,
}

/// Parse `input` once for `Value` + `SpanTree` and once for the green
/// tree, under `cfg`. Returns both — the caller wraps them in a
/// `Document`, which keeps `cfg` so every later re-parse of the same
/// source (typed-cache refresh, `validate`, the edit safety net) runs
/// under the limits the document was opened with.
#[cfg(feature = "std")]
pub(crate) fn parse_full(input: &str, cfg: &ParseConfig) -> Result<ParsedDocument> {
    check_document_length(input, cfg)?;
    let (value, span_tree) = crate::parser::parse_exactly_one(input, cfg)?;
    let source: Arc<str> = Arc::from(input);
    let green = build_green_tree(&source, cfg.max_depth)?;
    Ok(ParsedDocument {
        green,
        value,
        span_tree,
        source,
    })
}

/// Refuse a document longer than `cfg.max_document_length`, with the
/// error the typed `&str` entry points return for the same input.
///
/// Every CST parse of a whole document (the initial parse, each
/// document of a stream, and the source an edit would commit) runs
/// this first.
#[cfg(feature = "std")]
pub(crate) fn check_document_length(input: &str, cfg: &ParseConfig) -> Result<()> {
    if input.len() > cfg.max_document_length {
        return Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            cfg.max_document_length
        )));
    }
    Ok(())
}

/// Indentation / flow context for re-parsing a sub-tree.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SubtreeContext {
    /// Column at which the sub-tree begins. Block-collection
    /// content must indent strictly past this column.
    pub(crate) indent: usize,
    /// `0` for block context, non-zero when nested inside flow
    /// brackets. Sub-tree wrapping applies only in block context;
    /// flow contexts pass through verbatim.
    #[allow(dead_code)]
    pub(crate) flow_level: u32,
}

impl SubtreeContext {
    pub(crate) fn block_at(indent: usize) -> Self {
        Self {
            indent,
            flow_level: 0,
        }
    }
}

/// Re-parse a `fragment` of YAML and return a green sub-tree of
/// `expected` kind.
///
/// The strategy is to feed the parser a wrapper that establishes
/// the right indent context — for block-collection kinds we
/// prepend the indent to the first line so all subsequent lines
/// (which already carry their original indent) line up; for
/// entry kinds we additionally append a sentinel sibling so the
/// scanner sees a complete collection. The returned green
/// sub-tree's `text_len` matches `fragment.len()` exactly — that's
/// the contract `try_local_repair_green` checks before splicing.
#[cfg(feature = "std")]
pub(crate) fn parse_subtree(
    fragment: &str,
    ctx: SubtreeContext,
    expected: SyntaxKind,
    max_depth: usize,
) -> Result<GreenNode> {
    use SyntaxKind as S;
    match expected {
        S::BlockMapping | S::BlockSequence => {
            parse_block_collection(fragment, ctx, expected, max_depth)
        }
        S::MappingEntry | S::SequenceItem => parse_block_entry(fragment, ctx, expected, max_depth),
        S::Document => build_green_tree(fragment, max_depth),
        _ => Err(Error::Parse(format!(
            "parse_subtree: unsupported expected kind {expected:?}"
        ))),
    }
}

#[cfg(feature = "std")]
fn parse_block_collection(
    fragment: &str,
    ctx: SubtreeContext,
    expected: SyntaxKind,
    max_depth: usize,
) -> Result<GreenNode> {
    let modified = prepend_first_line_indent(fragment, ctx.indent);
    let parsed = build_green_tree(&modified, max_depth)?;
    first_node_of_kind(&parsed, expected).ok_or_else(|| {
        Error::Parse(format!(
            "parse_subtree: re-parsed fragment did not contain a {expected:?} at root"
        ))
    })
}

#[cfg(feature = "std")]
fn parse_block_entry(
    fragment: &str,
    ctx: SubtreeContext,
    expected: SyntaxKind,
    max_depth: usize,
) -> Result<GreenNode> {
    if is_yaml_blank(fragment) {
        return Err(Error::Parse(
            "parse_subtree: empty fragment cannot stand as a block entry".into(),
        ));
    }
    let mut wrapped = prepend_first_line_indent(fragment, ctx.indent);
    if !wrapped.ends_with('\n') {
        wrapped.push('\n');
    }
    // Sentinel sibling at the same column.
    for _ in 0..ctx.indent {
        wrapped.push(' ');
    }
    match expected {
        SyntaxKind::MappingEntry => wrapped.push_str("__noyalib_x__: 0\n"),
        SyntaxKind::SequenceItem => wrapped.push_str("- 0\n"),
        _ => unreachable!("guarded by caller"),
    }
    let parsed = build_green_tree(&wrapped, max_depth)?;
    let parent_kind = match expected {
        SyntaxKind::MappingEntry => SyntaxKind::BlockMapping,
        SyntaxKind::SequenceItem => SyntaxKind::BlockSequence,
        _ => unreachable!("guarded by caller"),
    };
    let parent = first_node_of_kind(&parsed, parent_kind).ok_or_else(|| {
        Error::Parse(format!(
            "parse_subtree: re-parsed entry did not produce a {parent_kind:?}"
        ))
    })?;
    let extracted = parent.children().find_map(|c| match c {
        GreenChild::Node(n) if n.kind() == expected => Some(n.clone()),
        _ => None,
    });
    extracted
        .ok_or_else(|| Error::Parse(format!("parse_subtree: extraction failed for {expected:?}")))
}

/// Prepend `indent` spaces to the first line of `s` if and only if
/// `indent > 0` and the first line does not already start with a
/// space. This equalises a fragment whose first line begins at
/// column 0 with subsequent lines that begin at `indent`.
fn prepend_first_line_indent(s: &str, indent: usize) -> String {
    if indent == 0 || s.starts_with(' ') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + indent);
    for _ in 0..indent {
        out.push(' ');
    }
    out.push_str(s);
    out
}

fn first_node_of_kind(node: &GreenNode, kind: SyntaxKind) -> Option<GreenNode> {
    for child in node.children() {
        if let GreenChild::Node(n) = child {
            if n.kind() == kind {
                return Some(n.clone());
            }
        }
    }
    None
}

/// Walk the token stream once and report `(start, end)` byte ranges
/// for each logical YAML document in `input`.
#[cfg(feature = "std")]
pub(crate) fn document_boundaries(input: &str) -> Result<Vec<(usize, usize)>> {
    let mut scanner = Scanner::new(input);
    scanner.enable_recording();
    loop {
        // Keep the scanner's byte position: the typed loaders report
        // the same error at the same place, and a caller needs the
        // line to point at (a directive after an unclosed document,
        // for instance).
        let tok = scanner
            .next_token()
            .map_err(|e| Error::parse_at(&*e.message, input, e.index))?;
        if matches!(tok.kind, TokenKind::StreamEnd) {
            break;
        }
    }
    let toks = scanner.take_recorded_tokens();
    drop(scanner);

    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut cur_start = 0usize;
    let mut has_content = false;
    let mut saw_explicit_end = false;

    for t in &toks {
        match t.kind {
            RecordedTokenKind::DocStart => {
                if has_content && !saw_explicit_end {
                    out.push((cur_start, t.start));
                    cur_start = t.start;
                }
                has_content = true;
                saw_explicit_end = false;
            }
            RecordedTokenKind::DocEnd => {
                // A `...` that closes nothing (the start of the stream,
                // or another `...` right before it) is not a document:
                // the typed loaders yield none for it (the suite's
                // HWV9), so it stays in the range and becomes the next
                // document's prologue, keeping the byte-for-byte
                // guarantee.
                if !has_content {
                    saw_explicit_end = true;
                    continue;
                }
                let bytes = input.as_bytes();
                let mut close = t.end;
                if bytes.get(close) == Some(&b'\r') {
                    close += 1;
                }
                if bytes.get(close) == Some(&b'\n') {
                    close += 1;
                }
                out.push((cur_start, close));
                cur_start = close;
                has_content = false;
                saw_explicit_end = true;
            }
            _ => {
                has_content = true;
                saw_explicit_end = false;
            }
        }
    }

    if cur_start < input.len() {
        if has_content || out.is_empty() {
            out.push((cur_start, input.len()));
        } else if let Some(last) = out.last_mut() {
            last.1 = input.len();
        }
    }

    if out.is_empty() {
        out.push((0, input.len()));
    }
    Ok(out)
}

/// Run the recording scanner over `source` and assemble the result
/// into a green tree. Drains the token stream so any scanner-level
/// error surfaces here rather than later.
///
/// `max_depth` is the document's [`ParseConfig::max_depth`]. The
/// builder refuses a tree that nests deeper than the loader would
/// accept, so a fragment that never reaches the full parse (the
/// local repair behind an edit) cannot build a tree whose recursive
/// walkers exhaust the stack.
pub(crate) fn build_green_tree(source: &str, max_depth: usize) -> Result<GreenNode> {
    let mut scanner = Scanner::new(source);
    scanner.enable_recording();

    loop {
        let tok = scanner
            .next_token()
            .map_err(|e| Error::parse_at(&*e.message, source, e.index))?;
        if matches!(tok.kind, TokenKind::StreamEnd) {
            break;
        }
    }

    let trivia = scanner.take_trivia();
    let tokens = scanner.take_recorded_tokens();
    let comments = scanner.take_comments();
    drop(scanner);

    assemble(trivia, tokens, comments, max_depth)
}

/// Merge the three streams (trivia, tokens, comments) into a nested
/// green tree. Stack-based bracketer: structural events drive
/// frame push/pop, leaf events become children of the current top
/// frame.
fn assemble(
    trivia: Vec<Trivia>,
    tokens: Vec<RecordedToken>,
    comments: Vec<ScannedComment>,
    max_depth: usize,
) -> Result<GreenNode> {
    let mut builder = TreeBuilder::new(max_depth);

    let mut trivia_iter = trivia.into_iter().peekable();
    let mut token_iter = tokens.into_iter().peekable();
    let mut comment_iter = comments.into_iter().peekable();

    loop {
        let nt = trivia_iter.peek().map(|t| t.start);
        let ntok = token_iter.peek().map(|t| t.start);
        let nc = comment_iter.peek().map(|c| c.start);

        match (nt, ntok, nc) {
            (None, None, None) => break,
            (_, Some(tk), c) if min_or_max(c) >= tk && min_or_max(nt) >= tk => {
                let tok = token_iter.next().expect("peeked Some");
                builder.handle_token(&tok)?;
            }
            (Some(t), _, c) if min_or_max(c) >= t => {
                let triv = trivia_iter.next().expect("peeked Some");
                builder.push_leaf(child_from_trivia(&triv)?);
            }
            (_, _, Some(_)) => {
                let cmt = comment_iter.next().expect("peeked Some");
                builder.push_leaf(token_child(SyntaxKind::Comment, cmt.end - cmt.start)?);
            }
            _ => crate::error::invariant_violated(
                "trivia/comment merge: at least one peek was Some by guard",
            ),
        }
    }

    builder.finish()
}

#[inline]
fn min_or_max(opt: Option<usize>) -> usize {
    opt.unwrap_or(usize::MAX)
}

/// The deepest frame stack the builder accepts for a document whose
/// loader allows `max_depth` levels of YAML nesting.
///
/// A block collection takes two frames per level (the collection and
/// the entry or item that holds the next one); a flow collection takes
/// one. The document frame and the outermost collection add a small
/// constant.
fn frame_limit(max_depth: usize) -> usize {
    crate::parser::budget::effective_max_depth(max_depth)
        .saturating_mul(2)
        .saturating_add(4)
}

struct Frame {
    kind: SyntaxKind,
    children: Vec<GreenChild>,
}

struct TreeBuilder {
    stack: Vec<Frame>,
    max_depth: usize,
}

impl TreeBuilder {
    fn new(max_depth: usize) -> Self {
        let mut stack = Vec::with_capacity(8);
        stack.push(Frame {
            kind: SyntaxKind::Document,
            children: Vec::new(),
        });
        Self { stack, max_depth }
    }

    fn top_kind(&self) -> SyntaxKind {
        self.stack.last().expect("non-empty").kind
    }

    fn push_leaf(&mut self, child: GreenChild) {
        self.stack
            .last_mut()
            .expect("non-empty")
            .children
            .push(child);
    }

    fn push_frame(&mut self, kind: SyntaxKind) -> Result<()> {
        if self.stack.len() >= frame_limit(self.max_depth) {
            return Err(Error::RecursionLimitExceeded {
                depth: self.stack.len(),
            });
        }
        self.stack.push(Frame {
            kind,
            children: Vec::new(),
        });
        Ok(())
    }

    fn pop_frame(&mut self) -> Result<()> {
        if self.stack.len() <= 1 {
            return Ok(());
        }
        let frame = self.stack.pop().expect("len > 1");
        let node = GreenNode::try_new(frame.kind, frame.children).ok_or_else(too_long)?;
        self.push_leaf(GreenChild::Node(node));
        Ok(())
    }

    fn close_open_entry(&mut self) -> Result<()> {
        if matches!(
            self.top_kind(),
            SyntaxKind::MappingEntry | SyntaxKind::SequenceItem
        ) {
            self.pop_frame()?;
        }
        Ok(())
    }

    fn nearest_container_kind(&self) -> SyntaxKind {
        for f in self.stack.iter().rev() {
            match f.kind {
                SyntaxKind::BlockMapping
                | SyntaxKind::BlockSequence
                | SyntaxKind::FlowMapping
                | SyntaxKind::FlowSequence
                | SyntaxKind::Document => return f.kind,
                _ => {}
            }
        }
        SyntaxKind::Document
    }

    /// Open an entry frame of `entry` kind when the nearest container
    /// is `container`, closing the previous entry first.
    fn open_entry_in(&mut self, container: SyntaxKind, entry: SyntaxKind) -> Result<()> {
        if self.nearest_container_kind() == container {
            self.close_open_entry()?;
            self.push_frame(entry)?;
        }
        Ok(())
    }

    /// Push a closing bracket and pop its frame when it is the top.
    fn close_flow(&mut self, frame: SyntaxKind, leaf: GreenChild) -> Result<()> {
        self.push_leaf(leaf);
        if self.top_kind() == frame {
            self.pop_frame()?;
        }
        Ok(())
    }

    fn handle_token(&mut self, tok: &RecordedToken) -> Result<()> {
        use RecordedTokenKind as R;
        use SyntaxKind as S;

        let leaf = || token_child(leaf_kind(tok.kind), tok.end - tok.start);
        match tok.kind {
            R::BlockMapStart => self.push_frame(S::BlockMapping),
            R::BlockSeqStart => self.push_frame(S::BlockSequence),
            R::BlockEnd => {
                self.close_open_entry()?;
                if matches!(self.top_kind(), S::BlockMapping | S::BlockSequence) {
                    self.pop_frame()?;
                }
                Ok(())
            }
            R::SyntheticKey => self.open_entry_in(S::BlockMapping, S::MappingEntry),
            R::QuestionIndicator => {
                self.open_entry_in(S::BlockMapping, S::MappingEntry)?;
                self.push_leaf(leaf()?);
                Ok(())
            }
            R::DashIndicator => {
                self.open_entry_in(S::BlockSequence, S::SequenceItem)?;
                self.push_leaf(leaf()?);
                Ok(())
            }
            R::OpenBrace | R::OpenBracket => {
                let frame = if tok.kind == R::OpenBrace {
                    S::FlowMapping
                } else {
                    S::FlowSequence
                };
                self.push_frame(frame)?;
                self.push_leaf(leaf()?);
                Ok(())
            }
            R::CloseBrace => self.close_flow(S::FlowMapping, leaf()?),
            R::CloseBracket => self.close_flow(S::FlowSequence, leaf()?),
            _ => {
                self.push_leaf(leaf()?);
                Ok(())
            }
        }
    }

    fn finish(mut self) -> Result<GreenNode> {
        while self.stack.len() > 1 {
            self.pop_frame()?;
        }
        let root = self.stack.pop().expect("Document frame");
        GreenNode::try_new(SyntaxKind::Document, root.children).ok_or_else(too_long)
    }
}

/// The green-tree kind of a recorded token's leaf. Structural tokens
/// that only open or close a frame have no leaf of their own; they
/// map to the document kind, which the caller never pushes as a leaf.
fn leaf_kind(kind: RecordedTokenKind) -> SyntaxKind {
    use RecordedTokenKind as R;
    use SyntaxKind as S;
    match kind {
        R::QuestionIndicator => S::QuestionIndicator,
        R::DashIndicator => S::DashIndicator,
        R::OpenBrace => S::OpenBrace,
        R::CloseBrace => S::CloseBrace,
        R::OpenBracket => S::OpenBracket,
        R::CloseBracket => S::CloseBracket,
        R::DocStart => S::DocStart,
        R::DocEnd => S::DocEnd,
        R::ColonIndicator => S::ColonIndicator,
        R::Comma => S::Comma,
        R::AnchorMark => S::AnchorMark,
        R::AliasMark => S::AliasMark,
        R::TagMark => S::TagMark,
        R::PlainScalar => S::PlainScalar,
        R::SingleQuotedScalar => S::SingleQuotedScalar,
        R::DoubleQuotedScalar => S::DoubleQuotedScalar,
        R::LiteralScalar => S::LiteralScalar,
        R::FoldedScalar => S::FoldedScalar,
        R::BlockMapStart | R::BlockSeqStart | R::BlockEnd | R::SyntheticKey => S::Document,
    }
}

/// The error for a token or tree whose length does not fit the
/// green tree's `u32` length field.
fn too_long() -> Error {
    Error::Parse("CST input exceeds the 4 GiB green-tree length limit".into())
}

fn token_child(kind: SyntaxKind, len: usize) -> Result<GreenChild> {
    let len = u32::try_from(len).map_err(|_| too_long())?;
    Ok(GreenChild::Token { kind, len })
}

fn child_from_trivia(t: &Trivia) -> Result<GreenChild> {
    let kind = match t.kind {
        TriviaKind::Whitespace => SyntaxKind::Whitespace,
        TriviaKind::Newline => SyntaxKind::Newline,
        TriviaKind::Bom => SyntaxKind::Bom,
        TriviaKind::Directive => SyntaxKind::Directive,
    };
    token_child(kind, t.end - t.start)
}

/// Splice `spliced` into `old_root` at the position currently
/// occupied by the unique node spanning `[splice_old_start,
/// splice_old_end)` of the same kind. Walks only the path from the
/// root down to the splice target's parent — sibling subtrees on
/// the path are reused via `Arc<[GreenChild]>` clones, not
/// rebuilt.
///
/// Returns the new root, or `None` when the result would exceed the
/// green tree's 4 GiB length limit. Time is `O(depth × siblings_per_level)` —
/// independent of the total tree size.
#[cfg(feature = "std")]
pub(crate) fn rebuild_with_splice(
    old_root: &GreenNode,
    splice_old_start: usize,
    splice_old_end: usize,
    spliced: GreenNode,
) -> Option<GreenNode> {
    splice_recursive(old_root, splice_old_start, splice_old_end, spliced, 0)
}

#[cfg(feature = "std")]
fn splice_recursive(
    node: &GreenNode,
    splice_old_start: usize,
    splice_old_end: usize,
    spliced: GreenNode,
    base: usize,
) -> Option<GreenNode> {
    let mut new_children = Vec::with_capacity(node.children().count());
    let mut pos = base;
    let mut consumed = false;
    let mut spliced_opt = Some(spliced);

    for child in node.children() {
        let len = child.text_len();
        let child_start = pos;
        let child_end = pos + len;

        if !consumed
            && child_start == splice_old_start
            && child_end == splice_old_end
            && matches!(child, GreenChild::Node(n)
                if Some(n.kind()) == spliced_opt.as_ref().map(|s| s.kind()))
        {
            // Exact match — replace.
            let s = spliced_opt.take().expect("checked Some above");
            new_children.push(GreenChild::Node(s));
            consumed = true;
        } else if !consumed && child_start <= splice_old_start && child_end >= splice_old_end {
            // Recurse into the only child that contains the
            // splice target.
            match child {
                GreenChild::Node(inner) => {
                    let s = spliced_opt.take().expect("path-unique splice target");
                    let new_inner =
                        splice_recursive(inner, splice_old_start, splice_old_end, s, child_start)?;
                    new_children.push(GreenChild::Node(new_inner));
                    consumed = true;
                }
                GreenChild::Token { .. } => {
                    // Defensive: a leaf can't contain a node.
                    new_children.push(child.clone());
                }
            }
        } else {
            // Pre- or post-splice — same `Arc`-backed reference.
            new_children.push(child.clone());
        }
        pos += len;
    }

    GreenNode::try_new(node.kind(), new_children)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_over_u32_max_is_an_error_not_a_panic() {
        let over = u32::MAX as usize + 1;
        assert!(token_child(SyntaxKind::PlainScalar, over).is_err());
        assert!(token_child(SyntaxKind::PlainScalar, u32::MAX as usize).is_ok());
    }

    #[test]
    fn a_node_over_u32_max_is_refused() {
        let big = GreenChild::Token {
            kind: SyntaxKind::PlainScalar,
            len: u32::MAX,
        };
        let one = GreenChild::Token {
            kind: SyntaxKind::PlainScalar,
            len: 1,
        };
        assert!(GreenNode::try_new(SyntaxKind::Document, vec![big.clone()]).is_some());
        assert!(GreenNode::try_new(SyntaxKind::Document, vec![big, one]).is_none());
    }
}
