//! Literal and folded block scalar scanning for the YAML scanner, as a
//! further `impl` block of [`super::Scanner`] beside the flow and
//! quoted scalars in `scalars.rs`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use super::{ScalarStyle, ScanResult, Scanner, TokenKind};
use crate::prelude::*;

/// The block scalar text read so far, with the line breaks not yet
/// placed: whether a break folds depends on the line that follows it.
#[derive(Default)]
struct BlockBody {
    string: String,
    trailing_breaks: String,
    /// The previous content line was more-indented (or spaced), so the
    /// break out of it is preserved rather than folded.
    leading_blank: bool,
}

impl BlockBody {
    /// Place the pending breaks before a content line.
    ///
    /// Apply fold logic. Order of cases:
    ///   * literal style: every break preserved as-is.
    ///   * before any content has been emitted: leading empty
    ///     lines preserved (`b-l-folded` does not fold a leading
    ///     break against the implicit header break).
    ///   * either side is more-indented: every break preserved.
    ///   * single break between regular content lines: fold to ' '.
    ///   * multiple breaks between regular content: drop the
    ///     leading break (the fold-into-empty-line) and keep the
    ///     rest as `\n`s.
    fn place_breaks(&mut self, literal: bool, is_more_indented: bool) {
        if self.trailing_breaks.is_empty() {
            return;
        }
        let preserve_all =
            literal || self.string.is_empty() || is_more_indented || self.leading_blank;
        if preserve_all {
            self.string.push_str(&self.trailing_breaks);
        } else if self.trailing_breaks.len() == 1 {
            self.string.push(' ');
        } else {
            self.string.push_str(&self.trailing_breaks[1..]);
        }
        self.trailing_breaks.clear();
    }

    /// A whitespace-only line whose spaces exceed the content
    /// indentation is a spaced content line, in literal and folded
    /// style alike (YAML 1.2.2 §8.1.1.4, and §8.1.3: a folded scalar
    /// does not fold the breaks around a spaced line). The breaks
    /// before it are kept, the spaces past the indentation are its
    /// text, and the break out of it is kept too.
    fn push_spaced_blank(&mut self, extra: usize) {
        self.string.push_str(&self.trailing_breaks);
        self.trailing_breaks.clear();
        self.string.extend(core::iter::repeat_n(' ', extra));
        self.leading_blank = true;
    }

    /// Apply chomping. YAML 1.2.2 §8.1.1.2:
    ///   `+` (keep): preserve every trailing line break.
    ///   default (clip): a single trailing `\n` if and only if the
    ///     scalar has any content. An empty scalar with `>`/`|` and
    ///     trailing blank lines stays empty.
    ///   `-` (strip): no trailing line break.
    fn chomp(mut self, chomping: i8) -> String {
        match chomping {
            1 => self.string.push_str(&self.trailing_breaks),
            0 if !self.string.is_empty() => self.string.push('\n'),
            _ => {}
        }
        self.string
    }
}

impl Scanner<'_> {
    pub(super) fn fetch_block_scalar(&mut self, literal: bool) -> ScanResult<()> {
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        self.mark = self.pos;

        let string = self.scan_block_scalar(literal)?;
        // §5.1 c-printable applies to block scalar content too — the
        // breaks are already folded to `\n` and tab is legal, so the
        // shared printable check fits exactly (found by the
        // serde_yaml parity fuzzer on `>-\x07`).
        self.check_scalar_printable(&string)?;
        let style = if literal {
            ScalarStyle::Literal
        } else {
            ScalarStyle::Folded
        };
        self.emit(TokenKind::Scalar(style, Cow::Owned(string)));
        self.last_token_opens_block = false;
        Ok(())
    }
    /// Whether the bytes at the cursor are a document marker (`---` or
    /// `...`) followed by a blank, a break, or the end of input.
    fn at_document_marker(&self) -> bool {
        let p0 = self.peek();
        (p0 == b'-' || p0 == b'.')
            && self.peek_at(1) == p0
            && self.peek_at(2) == p0
            && (self.pos + 3 >= self.input.len() || Self::is_blank_or_break(self.peek_at(3)))
    }

    fn scan_block_scalar(&mut self, literal: bool) -> ScanResult<String> {
        self.advance(); // skip '|' or '>'
        let (chomping, increment) = self.scan_block_header()?;
        let block_indent = self.detect_block_indent(increment)?;

        // Read the block scalar content.
        let mut body = BlockBody::default();
        while !self.is_eof() && self.scan_block_line(&mut body, literal, block_indent, increment)? {
        }
        Ok(body.chomp(chomping))
    }

    /// Parse optional chomping indicator and indentation indicator.
    /// Returns `(chomping, increment)`: chomping 0 = clip, 1 = keep,
    /// -1 = strip; an increment of 0 means auto-detected indentation.
    fn scan_block_indicators(&mut self) -> (i8, usize) {
        let mut chomping: i8 = 0; // 0 = clip, 1 = keep, -1 = strip
        let mut increment: usize = 0;

        // Check for chomping/indent indicators in either order.
        for _ in 0..2 {
            if self.is_eof() {
                break;
            }
            match self.peek() {
                b'+' => chomping = 1,
                b'-' => chomping = -1,
                c if c.is_ascii_digit() && c != b'0' => increment = (c - b'0') as usize,
                _ => break,
            }
            self.advance();
        }
        (chomping, increment)
    }

    /// Read the rest of a block scalar header after the indicators, up
    /// to and including its line break.
    fn scan_block_header(&mut self) -> ScanResult<(i8, usize)> {
        let (chomping, increment) = self.scan_block_indicators();

        // Per YAML 1.2.2 §8.1.1.1, the explicit indentation indicator
        // is a single digit 1..9. `0` is invalid (zero indent), and a
        // second digit (e.g. `|10`) is also invalid (the indicator is
        // a single digit). Anything still hanging on the header that
        // isn't blank/break/comment is malformed.
        let next = self.peek();
        if next.is_ascii_digit() {
            return Err(self.error(
                "invalid block scalar indentation indicator (must be a single digit 1..9)",
            ));
        }

        // Skip to end of line (including optional comment). Per
        // YAML 1.2.2 §6.6, an inline `#` must be preceded by a space or
        // tab — `>#` or `|2#` is invalid because the comment indicator
        // is adjacent to the header content.
        let pos_before_blank = self.pos;
        while Self::is_blank(self.peek()) {
            self.advance();
        }
        if self.peek() == b'#' {
            if self.pos == pos_before_blank {
                return Err(self.error("comment indicator '#' must be preceded by a space or tab"));
            }
            while !self.is_eof() && !Self::is_break(self.peek()) {
                if Self::is_raw_control(self.peek()) {
                    return Err(self.error("comment contains a raw control character"));
                }
                self.advance();
            }
        }

        // Per §8.1.1 the header line ends here: anything left that is
        // not a break (or end of input) is malformed — content starts
        // on the NEXT line, never on the header's (found by the
        // serde_yaml parity fuzzer on `>-\n`, where a literal `\n`
        // after the header was silently read as content).
        if !self.is_eof() && !Self::is_break(self.peek()) {
            return Err(
                self.error("block scalar header must be followed by a comment or a line break")
            );
        }

        // Consume the line break.
        if Self::is_break(self.peek()) {
            self.skip_line();
        }
        Ok((chomping, increment))
    }

    /// Look ahead, without consuming, for the first non-empty line.
    /// Returns `(detected, has_content, max_leading_empty_spaces)`.
    ///
    /// The first non-empty line counts as the scalar's first content
    /// line only when it is part of the scalar: more indented than
    /// the parent node, and not a document marker at column 0. A
    /// scalar whose lines are all blank (`- |+` followed by a line
    /// of spaces, then `---`) has no content line at all, and its
    /// blank lines are trailing breaks, not a leading-line
    /// indentation error (found by the suite-stream permutations).
    fn peek_first_content_indent(&mut self) -> (usize, bool, usize) {
        let mut max_leading_empty_spaces = 0;
        let mut detected = 0;
        let mut has_content = false;
        let save_pos = self.pos;
        let save_col = self.col;
        let parent_indent = self.indent;
        loop {
            let mut spaces = 0;
            while self.peek() == b' ' {
                spaces += 1;
                self.advance();
            }
            if Self::is_break(self.peek()) {
                max_leading_empty_spaces = max_leading_empty_spaces.max(spaces);
                self.skip_line();
                continue;
            }
            if self.is_eof() {
                break;
            }
            let belongs_to_scalar =
                spaces as i32 > parent_indent && !(spaces == 0 && self.at_document_marker());
            if belongs_to_scalar {
                detected = spaces;
                has_content = true;
            }
            break;
        }
        self.pos = save_pos;
        self.col = save_col;
        (detected, has_content, max_leading_empty_spaces)
    }

    /// Determine the indentation level and validate leading empty lines.
    fn detect_block_indent(&mut self, increment: usize) -> ScanResult<usize> {
        let (detected, has_content, max_leading_empty_spaces) = self.peek_first_content_indent();

        let block_indent = if increment > 0 {
            if self.indent >= 0 {
                self.indent as usize + increment
            } else {
                increment
            }
        } else {
            let min_indent = if self.indent >= 0 {
                self.indent as usize + 1
            } else {
                // Root-level block scalar: content can start at column 0
                // (parent indent is -1, so any column ≥ 0 is more indented).
                0
            };
            let actual_detected = if has_content {
                detected
            } else {
                max_leading_empty_spaces
            };
            actual_detected.max(min_indent)
        };

        // YAML 1.2.2 §8.1.1.1: with auto-detected indentation it is an
        // error for a leading empty line to hold more spaces than the
        // first non-empty line. With an explicit indentation indicator
        // the level is given, so such a line is not empty: its spaces
        // past the indentation are content, read below (#384).
        if increment == 0 && max_leading_empty_spaces > block_indent {
            return Err(self.error("a leading all-space line must not have too many spaces"));
        }
        Ok(block_indent)
    }

    /// Read one line of block scalar content into `body`. Returns
    /// `false` when the scalar ends at this line.
    fn scan_block_line(
        &mut self,
        body: &mut BlockBody,
        literal: bool,
        block_indent: usize,
        increment: usize,
    ) -> ScanResult<bool> {
        // Document boundary terminates the block scalar (matters when
        // `block_indent == 0`; otherwise the indent check below handles it).
        if self.col == 0 && self.at_document_marker() {
            return Ok(false);
        }

        // Count leading spaces.
        let mut spaces = 0;
        while self.peek() == b' ' {
            spaces += 1;
            self.advance();
        }

        // YAML 1.2.2 §6.1: tabs MUST NOT serve as indentation. If
        // we're below the established block indent and the next
        // byte is a tab (not a line break / EOF), the user is
        // attempting to use the tab as further indentation —
        // reject (Y79Y sub-case 1).
        if spaces < block_indent && self.peek() == b'\t' {
            return Err(self.error("tab characters are not allowed as block-scalar indentation"));
        }

        if spaces < block_indent && !Self::is_break(self.peek()) && !self.is_eof() {
            // End of block scalar.
            return Ok(false);
        }

        let extra = spaces.saturating_sub(block_indent);
        if Self::is_break(self.peek()) || self.is_eof() {
            return Ok(self.scan_block_blank_line(body, extra, increment > 0));
        }
        self.scan_block_content_line(body, literal, extra);
        Ok(true)
    }

    /// Empty line (blank-only or break-only) — record and continue
    /// before any fold decision so empty lines accumulate as `\n`s
    /// in `trailing_breaks` rather than being treated as content.
    ///
    /// A whitespace-only line whose leading spaces exceed
    /// `block_indent` carries content: per YAML 1.2.2 §8.1.1.4, every
    /// character at or beyond the content indentation is preserved,
    /// and the line is a spaced line in folded style
    /// ([`BlockBody::push_spaced_blank`]). The exception is *leading*
    /// whitespace-only lines (before any real content has been
    /// emitted) under auto-detected indentation — those are part of
    /// the leading empty-line region and contribute only their `\n`,
    /// not their indent characters. With an explicit indentation
    /// indicator the level is known before any content, so a leading
    /// line's surplus spaces are content too (#384).
    ///
    /// Returns `false` when the line ends the input.
    fn scan_block_blank_line(
        &mut self,
        body: &mut BlockBody,
        extra: usize,
        explicit_indent: bool,
    ) -> bool {
        let leading_spaces_are_content = explicit_indent || !body.string.is_empty();
        let spaced = extra > 0 && leading_spaces_are_content;
        if spaced {
            body.push_spaced_blank(extra);
        }
        if !Self::is_break(self.peek()) {
            // EOF reached after counting `spaces` blanks. Treat
            // a whitespace-only trailing line as if it had a
            // synthetic line break so the chomping pass below
            // sees the same shape it would for the
            // newline-terminated case (L24T spec test).
            if spaced {
                body.trailing_breaks.push('\n');
            }
            return false;
        }
        body.trailing_breaks.push('\n');
        self.skip_line();
        true
    }

    /// Read a content line: place the pending breaks, then the line's
    /// extra indentation and text, then its break.
    fn scan_block_content_line(&mut self, body: &mut BlockBody, literal: bool, extra: usize) {
        // Determine more-indented status of the current content line.
        // YAML 1.2.2 §8.1.1.5: a line is "more-indented" if it has
        // extra leading spaces beyond `block_indent`, or if its first
        // non-leading-space character is a tab. The break(s) into and
        // out of a more-indented line are preserved (not folded).
        let starts_with_tab = self.peek() == b'\t';
        let is_more_indented = extra > 0 || starts_with_tab;

        body.place_breaks(literal, is_more_indented);
        body.string.extend(core::iter::repeat_n(' ', extra));
        body.leading_blank = is_more_indented;

        // Read content of the line.
        while !self.is_eof() && !Self::is_break(self.peek()) {
            let start = self.pos;
            self.advance();
            while self.pos < self.input.len() && (self.input[self.pos] & 0xC0) == 0x80 {
                self.advance();
            }
            body.string.push_str(self.slice_str(start, self.pos));
        }

        // Consume the line break.
        if Self::is_break(self.peek()) {
            body.trailing_breaks.push('\n');
            self.skip_line();
        }
    }
}
