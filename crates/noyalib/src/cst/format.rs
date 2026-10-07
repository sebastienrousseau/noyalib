// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Formatter for YAML CST.

use crate::cst::document::{Document, parse_stream_inner};
use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::syntax::SyntaxKind;
use crate::de::ParserConfig;
use crate::error::{Error, Result};
use crate::parser::ParseConfig;
use crate::prelude::*;

/// Configuration for the formatter.
#[derive(Debug, Clone, Copy)]
pub struct FormatConfig {
    /// Number of spaces per indentation level. Defaults to 2.
    pub indent_size: usize,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self { indent_size: 2 }
    }
}

/// Auto-formats a messy YAML file into a canonical style based on the CST.
///
/// This uses the default configuration (2 spaces indentation).
///
/// The output always parses to the same values as the input. A stream
/// the formatter cannot re-layout without changing its meaning (for
/// example a multi-line flow collection whose continuation lines would
/// end up at or left of a re-indented key) is refused with an error
/// rather than rewritten.
///
/// # Errors
///
/// Returns the parse error of an invalid input, a budget error under
/// the default [`ParserConfig`], or an error when the input cannot be
/// re-laid out without changing its meaning.
pub fn format(input: &str) -> Result<String> {
    format_with_config(input, &FormatConfig::default())
}

/// Auto-formats a messy YAML file into a canonical style based on the CST,
/// using the provided configuration.
///
/// The input is parsed under [`ParserConfig::default`]; use
/// [`format_with_parser_config`] to choose the limits.
pub fn format_with_config(input: &str, config: &FormatConfig) -> Result<String> {
    format_with_parser_config(input, config, &ParserConfig::default())
}

/// Auto-formats a YAML stream under the given formatter and parser
/// configuration.
///
/// The input is parsed as by [`crate::cst::parse_stream_with_config`]:
/// `max_stream_bytes`, `max_documents`, `max_document_length` and the
/// other limits of `parser` apply, so a formatter that runs on
/// untrusted input can bound the work it does.
///
/// # Errors
///
/// Returns the parse or budget error the input trips under `parser`,
/// or an error when the input cannot be re-laid out without changing
/// its meaning (see [`format()`]).
///
/// # Examples
///
/// ```
/// use noyalib::cst::{FormatConfig, format_with_parser_config};
/// use noyalib::ParserConfig;
///
/// let strict = ParserConfig::strict();
/// let out = format_with_parser_config("a:   1\n", &FormatConfig::default(), &strict).unwrap();
/// assert_eq!(out, "a: 1\n");
///
/// let one_doc = ParserConfig::new().max_documents(1);
/// let stream = "--- 1\n--- 2\n";
/// assert!(format_with_parser_config(stream, &FormatConfig::default(), &one_doc).is_err());
/// ```
pub fn format_with_parser_config(
    input: &str,
    config: &FormatConfig,
    parser: &ParserConfig,
) -> Result<String> {
    if input.trim().is_empty() {
        return Ok(String::new());
    }
    let parse_config = ParseConfig::from(parser);
    let docs = parse_stream_inner(input, &parse_config, parser.max_stream_bytes)?;
    let mut output = String::with_capacity(input.len());
    for doc in &docs {
        let mut formatter = Formatter::new(doc.source(), config);
        formatter.format_node(doc.syntax(), 0)?;
        output.push_str(&formatter.finish());
    }
    ensure_same_meaning(&docs, &output, &parse_config, parser.max_stream_bytes)?;
    Ok(output)
}

/// Refuse `output` unless it parses, under the same configuration, to
/// the values of `docs`.
///
/// The formatter re-lays out text token by token and does not model
/// every YAML layout rule, so this check is what makes its contract
/// hold: the output means what the input meant, or `format` returns an
/// error and the caller keeps the input.
fn ensure_same_meaning(
    docs: &[Document],
    output: &str,
    config: &ParseConfig,
    max_stream_bytes: usize,
) -> Result<()> {
    let refused = || {
        Error::Parse(
            "format: the input cannot be re-laid out without changing its meaning; \
             it was left unformatted"
                .into(),
        )
    };
    let after = parse_stream_inner(output, config, max_stream_bytes).map_err(|_| refused())?;
    let same = after.len() == docs.len()
        && after
            .iter()
            .zip(docs)
            .all(|(a, b)| *a.as_value() == *b.as_value());
    if same { Ok(()) } else { Err(refused()) }
}

struct Formatter<'a> {
    source: &'a str,
    config: &'a FormatConfig,
    out: String,
    indent_level: usize,
    at_line_start: bool,
    /// Whether the last thing written was a space.
    last_was_space: bool,
    /// The last token written, ignoring whitespace. A tag, anchor or
    /// alias must be separated from what follows it: `!foo "bar"`
    /// written as `!foo"bar"` is one tag and an empty scalar, and
    /// `*a :` written as `*a:` is an alias named `a:`.
    last_token: Option<SyntaxKind>,
}

impl<'a> Formatter<'a> {
    fn new(source: &'a str, config: &'a FormatConfig) -> Self {
        Self {
            source,
            config,
            out: String::with_capacity(source.len()),
            indent_level: 0,
            at_line_start: true,
            last_was_space: false,
            last_token: None,
        }
    }

    fn finish(self) -> String {
        let mut out = self.out;
        if !out.ends_with('\n') && !out.is_empty() {
            out.push('\n');
        }
        out
    }

    fn indent(&mut self) {
        if self.at_line_start {
            for _ in 0..(self.indent_level * self.config.indent_size) {
                self.out.push(' ');
            }
            self.at_line_start = false;
            self.last_was_space = self.indent_level > 0;
        }
    }

    fn newline(&mut self) {
        if self.at_line_start {
            return;
        }
        // A block scalar's token carries the indentation of the line
        // that follows it, so after writing one the cursor sits on a
        // line holding nothing but spaces. Ending that line would add a
        // blank line, and a keep-chomped scalar counts blank lines as
        // content, so the value would gain a newline. Drop the spaces
        // and we are already at the start of a line (found by
        // formatting the spec-torture corpus).
        let trimmed = self.out.trim_end_matches([' ', '\t']).len();
        let line_is_only_space =
            trimmed < self.out.len() && (trimmed == 0 || self.out.as_bytes()[trimmed - 1] == b'\n');
        if line_is_only_space {
            self.out.truncate(trimmed);
        } else {
            self.out.push('\n');
        }
        self.at_line_start = true;
        self.last_was_space = false;
    }

    fn write_raw(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        // Indentation is written eagerly, so writing it before a token
        // that opens with a line break leaves a line of nothing but
        // spaces. Inside a keep-chomped block scalar that line is
        // content and silently adds a newline to the value (found by
        // formatting the spec-torture corpus).
        if !text.starts_with(['\n', '\r']) {
            self.indent();
        }
        self.out.push_str(text);
        if let Some(last_newline) = text.rfind('\n') {
            let trailing = &text[last_newline + 1..];
            self.at_line_start = trailing.is_empty();
            self.last_was_space = trailing.ends_with(' ');
        } else {
            self.at_line_start = false;
            self.last_was_space = text.ends_with(' ');
        }
    }

    fn ensure_space(&mut self) {
        if !self.at_line_start && !self.last_was_space {
            self.out.push(' ');
            self.last_was_space = true;
        }
    }

    fn format_node(&mut self, node: &GreenNode, base: usize) -> Result<()> {
        match node.kind() {
            SyntaxKind::Document | SyntaxKind::Stream => {
                self.format_children(node, base)?;
            }
            SyntaxKind::BlockMapping => {
                self.format_block_mapping(node, base)?;
            }
            SyntaxKind::BlockSequence => {
                self.format_block_sequence(node, base)?;
            }
            SyntaxKind::MappingEntry => {
                self.format_mapping_entry(node, base)?;
            }
            SyntaxKind::SequenceItem => {
                self.format_sequence_item(node, base)?;
            }
            SyntaxKind::FlowMapping | SyntaxKind::FlowSequence => {
                self.write_verbatim(node, base);
            }
            _ => {
                self.format_children(node, base)?;
            }
        }
        Ok(())
    }

    fn format_children(&mut self, node: &GreenNode, base: usize) -> Result<()> {
        let mut pos = base;
        for child in node.children() {
            match child {
                GreenChild::Node(inner) => {
                    self.format_node(inner, pos)?;
                }
                GreenChild::Token { kind, len } => {
                    self.handle_token(*kind, &self.source[pos..pos + *len as usize]);
                }
            }
            pos += child.text_len();
        }
        Ok(())
    }

    fn handle_token(&mut self, kind: SyntaxKind, text: &str) {
        match kind {
            SyntaxKind::Comment => {
                self.ensure_space();
                self.write_raw(text.trim_end());
                self.newline();
            }
            SyntaxKind::Newline => {
                self.newline();
            }
            SyntaxKind::Whitespace | SyntaxKind::Bom => {
                // Skip
            }
            SyntaxKind::DocStart | SyntaxKind::DocEnd => {
                // A marker is only a marker at the start of a line:
                // `Document...` is a plain scalar.
                self.newline();
                self.write_raw(if kind == SyntaxKind::DocStart {
                    "---"
                } else {
                    "..."
                });
                self.newline();
            }
            SyntaxKind::ColonIndicator => {
                if self.after_node_property() {
                    self.ensure_space();
                }
                self.write_raw(":");
            }
            SyntaxKind::DashIndicator => {
                if !self.at_line_start {
                    self.newline();
                }
                self.write_raw("-");
            }
            _ if kind.is_token() => {
                let trimmed = if matches!(kind, SyntaxKind::PlainScalar) {
                    text.trim()
                } else {
                    text
                };
                self.separate_from_property();
                self.write_raw(trimmed);
                // A scalar token can carry the line break that ends
                // it (`: a` / `: b` under an empty key); keep it.
                if trimmed.len() < text.len() && text.trim_end_matches([' ', '\t']).ends_with('\n')
                {
                    self.newline();
                }
            }
            _ => {}
        }
        if kind != SyntaxKind::Whitespace {
            self.last_token = Some(kind);
        }
    }

    /// Whether the last token was a tag, anchor or alias.
    fn after_node_property(&self) -> bool {
        matches!(
            self.last_token,
            Some(SyntaxKind::TagMark | SyntaxKind::AnchorMark | SyntaxKind::AliasMark)
        )
    }

    /// Keep the space between a tag, anchor or indicator and the
    /// content that follows it (`: a` is a value, `:a` a plain scalar).
    fn separate_from_property(&mut self) {
        if matches!(
            self.last_token,
            Some(
                SyntaxKind::TagMark
                    | SyntaxKind::AnchorMark
                    | SyntaxKind::ColonIndicator
                    | SyntaxKind::DashIndicator
                    | SyntaxKind::QuestionIndicator
            )
        ) {
            self.ensure_space();
        }
    }

    fn format_block_mapping(&mut self, node: &GreenNode, base: usize) -> Result<()> {
        let mut pos = base;
        for child in node.children() {
            match child {
                GreenChild::Node(inner) if inner.kind() == SyntaxKind::MappingEntry => {
                    self.format_mapping_entry(inner, pos)?;
                }
                GreenChild::Token { kind, len } => {
                    self.handle_token(*kind, &self.source[pos..pos + *len as usize]);
                }
                GreenChild::Node(inner) => {
                    self.format_node(inner, pos)?;
                }
            }
            pos += child.text_len();
        }
        Ok(())
    }

    fn format_block_sequence(&mut self, node: &GreenNode, base: usize) -> Result<()> {
        let mut pos = base;
        for child in node.children() {
            match child {
                GreenChild::Node(inner) if inner.kind() == SyntaxKind::SequenceItem => {
                    self.format_sequence_item(inner, pos)?;
                }
                GreenChild::Token { kind, len } => {
                    self.handle_token(*kind, &self.source[pos..pos + *len as usize]);
                }
                GreenChild::Node(inner) => {
                    self.format_node(inner, pos)?;
                }
            }
            pos += child.text_len();
        }
        Ok(())
    }

    fn format_mapping_entry(&mut self, node: &GreenNode, base: usize) -> Result<()> {
        // A value whose tag or anchor sits alone on the line after the
        // colon (`k:` / `  !!pairs` / `  - a`) is written as `k: !!pairs`:
        // the line break is dropped and the property joins the key line,
        // where it needs no indentation of its own (found by the
        // ultra-complex fixture, which lost the property's indent).
        let children: Vec<&GreenChild> = node.children().collect();
        let drop_newline_at = property_on_next_line(&children);
        let mut state = EntryState::default();
        let mut pos = base;
        for (index, child) in children.iter().enumerate() {
            match child {
                GreenChild::Token { kind, len } if drop_newline_at != Some(index) => {
                    let text = &self.source[pos..pos + *len as usize];
                    self.entry_token(&mut state, *kind, text);
                }
                GreenChild::Token { .. } => {}
                GreenChild::Node(inner) => self.entry_node(&mut state, inner, pos)?,
            }
            pos += child.text_len();
        }
        self.end_entry(&state);
        Ok(())
    }

    /// Write one token of a mapping entry.
    fn entry_token(&mut self, state: &mut EntryState, kind: SyntaxKind, text: &str) {
        // An explicit key (`? key`) needs the space after its indicator
        // as much as a sequence item needs the one after `-`: `?[a, b]`
        // is a plain scalar, not an explicit key (found by the
        // ultra-complex fixture).
        if (state.saw_colon || state.saw_question) && !is_layout_token(kind) {
            if state.saw_colon {
                self.indent_value_on_next_line(state);
            }
            self.ensure_space();
        }
        match kind {
            SyntaxKind::ColonIndicator => {
                // An explicit key's value indicator starts its own
                // line: `? a` / `: b`. On the key's line it would
                // turn the key into a mapping (`? a: b`).
                if state.saw_question && !self.at_line_start {
                    self.newline();
                }
                state.saw_colon = true;
                state.saw_question = false;
            }
            SyntaxKind::QuestionIndicator => state.saw_question = true,
            SyntaxKind::Newline | SyntaxKind::Comment if state.saw_colon => {
                state.value_on_next_line = true;
            }
            _ => {}
        }
        self.handle_token(kind, text);
    }

    /// Write one child node of a mapping entry.
    fn entry_node(&mut self, state: &mut EntryState, inner: &GreenNode, pos: usize) -> Result<()> {
        let block = matches!(
            inner.kind(),
            SyntaxKind::BlockMapping | SyntaxKind::BlockSequence
        );
        if state.saw_question && !state.saw_colon {
            self.ensure_space();
            // A mapping used as an explicit key continues on
            // the lines below the `?`, and those lines must
            // be indented past it or they become entries of
            // the surrounding mapping instead. Without this
            // the key is torn apart and the value is lost
            // (found by formatting the spec-torture corpus).
            let nested = inner.kind() == SyntaxKind::BlockMapping;
            return self.format_nested(inner, pos, nested);
        }
        if !state.saw_colon {
            return self.format_node(inner, pos);
        }
        if block {
            self.newline();
            return self.format_nested(inner, pos, true);
        }
        self.indent_value_on_next_line(state);
        self.ensure_space();
        self.format_node(inner, pos)
    }

    /// A scalar or flow value that starts on the line below its key
    /// (`key:` / `  value`) must stay indented past the key, or it
    /// becomes a key of its own.
    fn indent_value_on_next_line(&mut self, state: &mut EntryState) {
        if state.value_on_next_line && !state.value_indented {
            self.indent_level += 1;
            state.value_indented = true;
        }
    }

    fn end_entry(&mut self, state: &EntryState) {
        if state.value_indented {
            self.indent_level -= 1;
        }
        self.newline();
    }

    /// Format `inner`, one level deeper when `deeper` is set.
    fn format_nested(&mut self, inner: &GreenNode, pos: usize, deeper: bool) -> Result<()> {
        if !deeper {
            return self.format_node(inner, pos);
        }
        self.indent_level += 1;
        let result = self.format_node(inner, pos);
        self.indent_level -= 1;
        result
    }

    fn format_sequence_item(&mut self, node: &GreenNode, base: usize) -> Result<()> {
        let mut pos = base;
        let mut state = EntryState::default();

        for child in node.children() {
            match child {
                GreenChild::Token { kind, len } => {
                    let text = &self.source[pos..pos + *len as usize];
                    self.item_token(&mut state, *kind, text);
                }
                GreenChild::Node(inner) if state.saw_colon => {
                    // `saw_colon` stands for "saw the dash" here.
                    let block = matches!(
                        inner.kind(),
                        SyntaxKind::BlockMapping | SyntaxKind::BlockSequence
                    );
                    if !block {
                        self.indent_value_on_next_line(&mut state);
                    }
                    self.ensure_space();
                    self.format_nested(inner, pos, block)?;
                }
                GreenChild::Node(inner) => self.format_node(inner, pos)?,
            }
            pos += child.text_len();
        }
        self.end_entry(&state);
        Ok(())
    }

    /// Write one token of a sequence item.
    fn item_token(&mut self, state: &mut EntryState, kind: SyntaxKind, text: &str) {
        if state.saw_colon && !is_layout_token(kind) && kind != SyntaxKind::DashIndicator {
            self.indent_value_on_next_line(state);
            self.ensure_space();
        }
        match kind {
            SyntaxKind::DashIndicator => state.saw_colon = true,
            SyntaxKind::Newline | SyntaxKind::Comment if state.saw_colon => {
                state.value_on_next_line = true;
            }
            _ => {}
        }
        self.handle_token(kind, text);
    }

    fn write_verbatim(&mut self, node: &GreenNode, base: usize) {
        self.separate_from_property();
        self.indent();
        let text = node.text(&self.source[base..base + node.text_len()]);
        self.out.push_str(&text);
        self.at_line_start = text.ends_with('\n');
        self.last_was_space = text.ends_with(' ');
        self.last_token = Some(node.kind());
    }
}

/// Where a mapping entry is in its layout.
#[derive(Default)]
struct EntryState {
    /// The value indicator (or, for a sequence item, the dash) has
    /// been written.
    saw_colon: bool,
    /// An explicit-key indicator has been written and its value
    /// indicator has not.
    saw_question: bool,
    /// A line break or comment followed the value indicator.
    value_on_next_line: bool,
    /// The value was indented one level for being on its own line.
    value_indented: bool,
}

/// Tokens that carry layout only and never need a separating space.
fn is_layout_token(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::ColonIndicator
            | SyntaxKind::Newline
            | SyntaxKind::Whitespace
            | SyntaxKind::Comment
    )
}

/// The index of the line break after an entry's colon when the next
/// thing on the following line is a tag or anchor (`k:` / `  !!pairs`).
fn property_on_next_line(children: &[&GreenChild]) -> Option<usize> {
    let is = |c: &GreenChild, kinds: &[SyntaxKind]| matches!(c, GreenChild::Token { kind, .. } if kinds.contains(kind));
    let colon = children
        .iter()
        .position(|c| is(c, &[SyntaxKind::ColonIndicator]))?;
    let mut rest = children[colon + 1..]
        .iter()
        .enumerate()
        .filter(|(_, c)| !is(c, &[SyntaxKind::Whitespace]));
    let (offset, newline) = rest.next()?;
    if !is(newline, &[SyntaxKind::Newline]) {
        return None;
    }
    let (_, next) = rest.next()?;
    is(next, &[SyntaxKind::TagMark, SyntaxKind::AnchorMark]).then_some(colon + 1 + offset)
}
