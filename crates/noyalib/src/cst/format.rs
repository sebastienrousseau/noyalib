// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Formatter for YAML CST.

use crate::cst::document::parse_document;
use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::syntax::SyntaxKind;
use crate::error::Result;
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
pub fn format(input: &str) -> Result<String> {
    format_with_config(input, &FormatConfig::default())
}

/// Auto-formats a messy YAML file into a canonical style based on the CST,
/// using the provided configuration.
pub fn format_with_config(input: &str, config: &FormatConfig) -> Result<String> {
    if input.trim().is_empty() {
        return Ok(String::new());
    }
    let doc = parse_document(input)?;
    let mut formatter = Formatter::new(input, config);
    formatter.format_node(doc.syntax(), 0)?;
    Ok(formatter.finish())
}

struct Formatter<'a> {
    source: &'a str,
    config: &'a FormatConfig,
    out: String,
    indent_level: usize,
    at_line_start: bool,
    /// Whether the last thing written was a space.
    last_was_space: bool,
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
        if !self.at_line_start {
            self.out.push('\n');
            self.at_line_start = true;
            self.last_was_space = false;
        }
    }

    fn write_raw(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.indent();
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
            SyntaxKind::DocStart => {
                self.write_raw("---");
                self.newline();
            }
            SyntaxKind::DocEnd => {
                self.write_raw("...");
                self.newline();
            }
            SyntaxKind::ColonIndicator => {
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
                self.write_raw(trimmed);
            }
            _ => {}
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
        let mut pos = base;
        let mut saw_colon = false;
        // An explicit key (`? key`) needs the space after its indicator
        // as much as a sequence item needs the one after `-`: `?[a, b]`
        // is a plain scalar, not an explicit key (found by the
        // ultra-complex fixture).
        let mut saw_question = false;
        // A value whose tag or anchor sits alone on the line after the
        // colon (`k:` / `  !!pairs` / `  - a`) is written as `k: !!pairs`:
        // the line break is dropped and the property joins the key line,
        // where it needs no indentation of its own (found by the
        // ultra-complex fixture, which lost the property's indent).
        let children: Vec<&GreenChild> = node.children().collect();
        let mut drop_newline_at: Option<usize> = None;
        if let Some(colon) = children.iter().position(|c| {
            matches!(
                c,
                GreenChild::Token {
                    kind: SyntaxKind::ColonIndicator,
                    ..
                }
            )
        }) {
            let mut i = colon + 1;
            let is_ws = |c: &GreenChild| {
                matches!(
                    c,
                    GreenChild::Token {
                        kind: SyntaxKind::Whitespace,
                        ..
                    }
                )
            };
            while i < children.len() && is_ws(children[i]) {
                i += 1;
            }
            if i < children.len()
                && matches!(
                    children[i],
                    GreenChild::Token {
                        kind: SyntaxKind::Newline,
                        ..
                    }
                )
            {
                let newline = i;
                i += 1;
                while i < children.len() && is_ws(children[i]) {
                    i += 1;
                }
                if i < children.len()
                    && matches!(
                        children[i],
                        GreenChild::Token {
                            kind: SyntaxKind::TagMark | SyntaxKind::AnchorMark,
                            ..
                        }
                    )
                {
                    drop_newline_at = Some(newline);
                }
            }
        }

        for (index, child) in children.iter().enumerate() {
            match child {
                GreenChild::Token { kind, len } => {
                    let text = &self.source[pos..pos + *len as usize];
                    if drop_newline_at == Some(index) {
                        pos += child.text_len();
                        continue;
                    }
                    if (saw_colon || saw_question)
                        && !matches!(
                            kind,
                            SyntaxKind::ColonIndicator
                                | SyntaxKind::Newline
                                | SyntaxKind::Whitespace
                                | SyntaxKind::Comment
                        )
                    {
                        self.ensure_space();
                    }
                    if matches!(kind, SyntaxKind::ColonIndicator) {
                        // An explicit key's value indicator starts its own
                        // line: `? a` / `: b`. On the key's line it would
                        // turn the key into a mapping (`? a: b`).
                        if saw_question && !self.at_line_start {
                            self.newline();
                        }
                        saw_colon = true;
                        saw_question = false;
                    }
                    if matches!(kind, SyntaxKind::QuestionIndicator) {
                        saw_question = true;
                    }
                    self.handle_token(*kind, text);
                }
                GreenChild::Node(inner) => {
                    if saw_question && !saw_colon {
                        self.ensure_space();
                        self.format_node(inner, pos)?;
                    } else if saw_colon {
                        if matches!(
                            inner.kind(),
                            SyntaxKind::BlockMapping | SyntaxKind::BlockSequence
                        ) {
                            self.newline();
                            self.indent_level += 1;
                            self.format_node(inner, pos)?;
                            self.indent_level -= 1;
                        } else {
                            self.ensure_space();
                            self.format_node(inner, pos)?;
                        }
                    } else {
                        self.format_node(inner, pos)?;
                    }
                }
            }
            pos += child.text_len();
        }
        self.newline();
        Ok(())
    }

    fn format_sequence_item(&mut self, node: &GreenNode, base: usize) -> Result<()> {
        let mut pos = base;
        let mut saw_dash = false;

        for child in node.children() {
            match child {
                GreenChild::Token { kind, len } => {
                    let text = &self.source[pos..pos + *len as usize];
                    if saw_dash
                        && !matches!(
                            kind,
                            SyntaxKind::DashIndicator
                                | SyntaxKind::Newline
                                | SyntaxKind::Whitespace
                                | SyntaxKind::Comment
                        )
                    {
                        self.ensure_space();
                    }
                    if matches!(kind, SyntaxKind::DashIndicator) {
                        saw_dash = true;
                    }
                    self.handle_token(*kind, text);
                }
                GreenChild::Node(inner) => {
                    if saw_dash {
                        if matches!(
                            inner.kind(),
                            SyntaxKind::BlockMapping | SyntaxKind::BlockSequence
                        ) {
                            self.ensure_space();
                            self.indent_level += 1;
                            self.format_node(inner, pos)?;
                            self.indent_level -= 1;
                        } else {
                            self.ensure_space();
                            self.format_node(inner, pos)?;
                        }
                    } else {
                        self.format_node(inner, pos)?;
                    }
                }
            }
            pos += child.text_len();
        }
        self.newline();
        Ok(())
    }

    fn write_verbatim(&mut self, node: &GreenNode, base: usize) {
        self.indent();
        let text = node.text(&self.source[base..base + node.text_len()]);
        self.out.push_str(&text);
        self.at_line_start = text.ends_with('\n');
        self.last_was_space = text.ends_with(' ');
    }
}
