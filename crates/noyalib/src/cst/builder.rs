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
    let (value, span_tree) = crate::parser::parse_one(input, cfg)?;
    let source: Arc<str> = Arc::from(input);
    let green = build_green_tree(&source)?;
    Ok(ParsedDocument {
        green,
        value,
        span_tree,
        source,
    })
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
) -> Result<GreenNode> {
    use SyntaxKind as S;
    match expected {
        S::BlockMapping | S::BlockSequence => parse_block_collection(fragment, ctx, expected),
        S::MappingEntry | S::SequenceItem => parse_block_entry(fragment, ctx, expected),
        S::Document => {
            let arc: Arc<str> = Arc::from(fragment);
            build_green_tree(&arc)
        }
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
) -> Result<GreenNode> {
    let modified = prepend_first_line_indent(fragment, ctx.indent);
    let arc: Arc<str> = Arc::from(modified.as_str());
    let parsed = build_green_tree(&arc)?;
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
) -> Result<GreenNode> {
    if fragment.trim().is_empty() {
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
    let arc: Arc<str> = Arc::from(wrapped.as_str());
    let parsed = build_green_tree(&arc)?;
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
        let tok = scanner
            .next_token()
            .map_err(|e| Error::Parse(e.message.into_owned()))?;
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
pub(crate) fn build_green_tree(source: &str) -> Result<GreenNode> {
    let mut scanner = Scanner::new(source);
    scanner.enable_recording();

    loop {
        let tok = scanner
            .next_token()
            .map_err(|e| Error::Parse(e.message.into_owned()))?;
        if matches!(tok.kind, TokenKind::StreamEnd) {
            break;
        }
    }

    let trivia = scanner.take_trivia();
    let tokens = scanner.take_recorded_tokens();
    let comments = scanner.take_comments();
    drop(scanner);

    Ok(assemble(trivia, tokens, comments))
}

/// Merge the three streams (trivia, tokens, comments) into a nested
/// green tree. Stack-based bracketer: structural events drive
/// frame push/pop, leaf events become children of the current top
/// frame.
fn assemble(
    trivia: Vec<Trivia>,
    tokens: Vec<RecordedToken>,
    comments: Vec<ScannedComment>,
) -> GreenNode {
    let mut builder = TreeBuilder::new();

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
                builder.handle_token(tok);
            }
            (Some(t), _, c) if min_or_max(c) >= t => {
                let triv = trivia_iter.next().expect("peeked Some");
                builder.push_leaf(child_from_trivia(triv));
            }
            (_, _, Some(_)) => {
                let cmt = comment_iter.next().expect("peeked Some");
                builder.push_leaf(child_from_comment(cmt));
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

struct Frame {
    kind: SyntaxKind,
    children: Vec<GreenChild>,
}

struct TreeBuilder {
    stack: Vec<Frame>,
}

impl TreeBuilder {
    fn new() -> Self {
        let mut stack = Vec::with_capacity(8);
        stack.push(Frame {
            kind: SyntaxKind::Document,
            children: Vec::new(),
        });
        Self { stack }
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

    fn push_frame(&mut self, kind: SyntaxKind) {
        self.stack.push(Frame {
            kind,
            children: Vec::new(),
        });
    }

    fn pop_frame(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }
        let frame = self.stack.pop().expect("len > 1");
        let node = GreenNode::new(frame.kind, frame.children);
        self.push_leaf(GreenChild::Node(node));
    }

    fn close_open_entry(&mut self) {
        if matches!(
            self.top_kind(),
            SyntaxKind::MappingEntry | SyntaxKind::SequenceItem
        ) {
            self.pop_frame();
        }
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

    fn handle_token(&mut self, tok: RecordedToken) {
        use RecordedTokenKind as R;
        use SyntaxKind as S;

        match tok.kind {
            R::BlockMapStart => self.push_frame(S::BlockMapping),
            R::BlockSeqStart => self.push_frame(S::BlockSequence),
            R::BlockEnd => {
                self.close_open_entry();
                if matches!(self.top_kind(), S::BlockMapping | S::BlockSequence) {
                    self.pop_frame();
                }
            }
            R::SyntheticKey => {
                if matches!(self.nearest_container_kind(), S::BlockMapping) {
                    self.close_open_entry();
                    self.push_frame(S::MappingEntry);
                }
            }
            R::QuestionIndicator => {
                if matches!(self.nearest_container_kind(), S::BlockMapping) {
                    self.close_open_entry();
                    self.push_frame(S::MappingEntry);
                }
                self.push_leaf(leaf_token(S::QuestionIndicator, tok.end - tok.start));
            }
            R::DashIndicator => {
                if matches!(self.nearest_container_kind(), S::BlockSequence) {
                    self.close_open_entry();
                    self.push_frame(S::SequenceItem);
                }
                self.push_leaf(leaf_token(S::DashIndicator, tok.end - tok.start));
            }
            R::OpenBrace => {
                self.push_frame(S::FlowMapping);
                self.push_leaf(leaf_token(S::OpenBrace, tok.end - tok.start));
            }
            R::CloseBrace => {
                self.push_leaf(leaf_token(S::CloseBrace, tok.end - tok.start));
                if matches!(self.top_kind(), S::FlowMapping) {
                    self.pop_frame();
                }
            }
            R::OpenBracket => {
                self.push_frame(S::FlowSequence);
                self.push_leaf(leaf_token(S::OpenBracket, tok.end - tok.start));
            }
            R::CloseBracket => {
                self.push_leaf(leaf_token(S::CloseBracket, tok.end - tok.start));
                if matches!(self.top_kind(), S::FlowSequence) {
                    self.pop_frame();
                }
            }
            R::DocStart => self.push_leaf(leaf_token(S::DocStart, tok.end - tok.start)),
            R::DocEnd => self.push_leaf(leaf_token(S::DocEnd, tok.end - tok.start)),
            R::ColonIndicator => {
                self.push_leaf(leaf_token(S::ColonIndicator, tok.end - tok.start));
            }
            R::Comma => self.push_leaf(leaf_token(S::Comma, tok.end - tok.start)),
            R::AnchorMark => self.push_leaf(leaf_token(S::AnchorMark, tok.end - tok.start)),
            R::AliasMark => self.push_leaf(leaf_token(S::AliasMark, tok.end - tok.start)),
            R::TagMark => self.push_leaf(leaf_token(S::TagMark, tok.end - tok.start)),
            R::PlainScalar => self.push_leaf(leaf_token(S::PlainScalar, tok.end - tok.start)),
            R::SingleQuotedScalar => {
                self.push_leaf(leaf_token(S::SingleQuotedScalar, tok.end - tok.start));
            }
            R::DoubleQuotedScalar => {
                self.push_leaf(leaf_token(S::DoubleQuotedScalar, tok.end - tok.start));
            }
            R::LiteralScalar => self.push_leaf(leaf_token(S::LiteralScalar, tok.end - tok.start)),
            R::FoldedScalar => self.push_leaf(leaf_token(S::FoldedScalar, tok.end - tok.start)),
        }
    }

    fn finish(mut self) -> GreenNode {
        while self.stack.len() > 1 {
            self.pop_frame();
        }
        let root = self.stack.pop().expect("Document frame");
        GreenNode::new(SyntaxKind::Document, root.children)
    }
}

fn leaf_token(kind: SyntaxKind, len: usize) -> GreenChild {
    GreenChild::Token {
        kind,
        len: u32::try_from(len).expect("token length exceeds 4 GiB cap"),
    }
}

fn child_from_trivia(t: Trivia) -> GreenChild {
    let kind = match t.kind {
        TriviaKind::Whitespace => SyntaxKind::Whitespace,
        TriviaKind::Newline => SyntaxKind::Newline,
        TriviaKind::Bom => SyntaxKind::Bom,
        TriviaKind::Directive => SyntaxKind::Directive,
    };
    GreenChild::Token {
        kind,
        len: u32::try_from(t.end - t.start).expect("trivia length exceeds 4 GiB cap"),
    }
}

fn child_from_comment(c: ScannedComment) -> GreenChild {
    GreenChild::Token {
        kind: SyntaxKind::Comment,
        len: u32::try_from(c.end - c.start).expect("comment length exceeds 4 GiB cap"),
    }
}

/// Splice `spliced` into `old_root` at the position currently
/// occupied by the unique node spanning `[splice_old_start,
/// splice_old_end)` of the same kind. Walks only the path from the
/// root down to the splice target's parent — sibling subtrees on
/// the path are reused via `Arc<[GreenChild]>` clones, not
/// rebuilt.
///
/// Returns the new root. Time is `O(depth × siblings_per_level)` —
/// independent of the total tree size.
#[cfg(feature = "std")]
pub(crate) fn rebuild_with_splice(
    old_root: &GreenNode,
    splice_old_start: usize,
    splice_old_end: usize,
    spliced: GreenNode,
) -> GreenNode {
    splice_recursive(old_root, splice_old_start, splice_old_end, spliced, 0)
}

#[cfg(feature = "std")]
fn splice_recursive(
    node: &GreenNode,
    splice_old_start: usize,
    splice_old_end: usize,
    spliced: GreenNode,
    base: usize,
) -> GreenNode {
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
                        splice_recursive(inner, splice_old_start, splice_old_end, s, child_start);
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

    GreenNode::new(node.kind(), new_children)
}
