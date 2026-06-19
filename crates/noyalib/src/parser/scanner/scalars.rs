//! Scalar scanning for the YAML scanner: plain, single/double-quoted
//! (with escape decoding), and literal/folded block scalars.
//!
//! These are methods of [`super::Scanner`], split into their own file
//! as a second `impl` block to keep scanner.rs navigable. A child
//! module can read the parent `Scanner`'s private fields, so the move
//! is purely organisational.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use super::{ScalarStyle, ScanError, ScanResult, Scanner, TokenKind};
use crate::prelude::*;

impl Scanner<'_> {
    // ── Scalars ──────────────────────────────────────────────────────────

    pub(super) fn fetch_plain_scalar(&mut self) -> ScanResult<()> {
        self.save_simple_key();
        self.simple_key_allowed = false;
        self.mark = self.pos;

        // ── Fast path: single-line plain scalar (no line folding) ────
        // Most scalars in real YAML are single-line values like `key: value`.
        // Detect this case and emit directly from the input slice without
        // allocating `whitespace` or entering the multiline folding loop.
        {
            let remaining = &self.input[self.pos..];
            let in_flow = self.flow_level > 0;
            let mut len = 0;
            let mut is_single_line = true;

            // Find first line break or comment. A `#` is only a comment
            // when preceded by whitespace (YAML 1.2 rule).
            //
            // Use memchr for large remaining slices (>32 bytes) where SIMD
            // pays off; fall back to a simple scan for short scalars.
            let mut search_end = remaining.len();
            if remaining.len() > 32 {
                let mut offset = 0;
                while let Some(p) = memchr::memchr3(b'\n', b'\r', b'#', &remaining[offset..]) {
                    let abs = offset + p;
                    if remaining[abs] == b'#' {
                        // Only a comment if preceded by whitespace.
                        if abs > 0 && Self::is_blank(remaining[abs - 1]) {
                            search_end = abs;
                            break;
                        }
                        // Not a comment — keep scanning after this `#`.
                        offset = abs + 1;
                        continue;
                    }
                    // Line break found.
                    search_end = abs;
                    is_single_line = false;
                    break;
                }
            } else {
                for (i, &b) in remaining.iter().enumerate() {
                    if b == b'\n' || b == b'\r' {
                        search_end = i;
                        is_single_line = false;
                        break;
                    }
                    if b == b'#' && i > 0 && Self::is_blank(remaining[i - 1]) {
                        search_end = i;
                        break;
                    }
                }
            }

            // Hot-path SIMD: every byte that isn't in the boundary
            // candidate set just increments `len` after the existing
            // checks — those bytes are pure scalar interior. Skip
            // straight to the next candidate byte via the SIMD-routed
            // `clean_prefix_len` (memchr arity 1/2/3, SWAR for 4+);
            // the per-candidate state-dependent rules below stay
            // unchanged. Pure additive optimisation — semantics are
            // bit-exact with the byte-by-byte loop.
            let candidate_set: &[u8] = if in_flow { b": \t,[]{}" } else { b": \t" };
            while len < search_end {
                let skip =
                    crate::simd::clean_prefix_len(&remaining[len..search_end], candidate_set);
                len += skip;
                if len >= search_end {
                    break;
                }
                let c = remaining[len];
                if c == b':' {
                    let next = if len + 1 < remaining.len() {
                        remaining[len + 1]
                    } else {
                        0
                    };
                    if Self::is_blank_or_break(next)
                        || (in_flow && (next == b',' || next == b']' || next == b'}'))
                    {
                        break;
                    }
                }
                if in_flow && (c == b',' || c == b'[' || c == b']' || c == b'{' || c == b'}') {
                    break;
                }
                if c == b' ' || c == b'\t' {
                    // Check if this is trailing whitespace before a break or terminator.
                    let mut j = len + 1;
                    while j < remaining.len() && (remaining[j] == b' ' || remaining[j] == b'\t') {
                        j += 1;
                    }
                    if j >= remaining.len() || Self::is_break(remaining[j]) || remaining[j] == b'#'
                    {
                        // Trailing whitespace — trim and break
                        break;
                    }
                    if remaining[j] == b':'
                        && (j + 1 >= remaining.len()
                            || Self::is_blank_or_break(remaining[j + 1])
                            || (in_flow
                                && (remaining[j + 1] == b','
                                    || remaining[j + 1] == b']'
                                    || remaining[j + 1] == b'}')))
                    {
                        break;
                    }
                    len = j;
                    continue;
                }
                len += 1;
            }

            // The fast path can also fire when the scalar terminates
            // before the next newline (e.g. on a `:`, trailing
            // whitespace, or comment), even though `is_single_line`
            // was flipped to `false` because *some* newline exists
            // farther down the input. The line-folding slow path is
            // only required when the scalar runs right up to the
            // newline and might continue on the next line — i.e.
            // `len == search_end` *and* `search_end` landed on a
            // line break. Whenever `len < search_end` we know the
            // scalar is fully bounded on the current line, so the
            // input slice can be emitted directly as
            // `Cow::Borrowed`. This unblocks zero-copy
            // `Deserialize<'de> for &'de str` for the typical
            // `key: value\n` shape.
            let scalar_terminates_on_line = len < search_end;
            if (is_single_line || scalar_terminates_on_line) && len > 0 {
                // Strip trailing inline whitespace before a flow
                // indicator (`}`, `]`, `,`). The inner-scan loop
                // folds those blanks into `len` (line ~2134 above)
                // because they may precede more content on the
                // current line. When they do not, the slow path
                // would emit just the content; mirror that here so
                // the borrowed slice matches the owned-buffer
                // result byte-for-byte. The scanner position still
                // advances past the whitespace so downstream tokens
                // line up.
                let mut content_len = len;
                while content_len > 0 && matches!(remaining[content_len - 1], b' ' | b'\t') {
                    content_len -= 1;
                }
                if content_len > 0 {
                    let s = Cow::Borrowed(self.slice_str(self.pos, self.pos + content_len));
                    self.advance_by(len);
                    self.emit(TokenKind::Scalar(ScalarStyle::Plain, s));
                    self.last_token_opens_block = false;
                    return Ok(());
                }
            }
        }

        // ── Slow path: multiline plain scalar with line folding ──────
        let scalar_start = self.pos;
        let mut content_end = self.pos;
        // Track whether the scalar is built from a single contiguous
        // run of input bytes (no folded line breaks). If so we can
        // emit `Cow::Borrowed(slice)` instead of allocating an owned
        // `String`. Flipped to `false` the first time the
        // line-folding branch synthesises a space / newline that
        // does not exist verbatim in the input.
        let mut single_chunk = true;
        let mut string = String::new();
        let mut leading_blanks = false;
        let mut whitespace = String::new();
        let indent = self.indent + 1;
        let in_flow = self.flow_level > 0;

        loop {
            // Skip to the end of the current run — scan directly from the
            // byte slice for cache-line-friendly sequential access.
            let mut length = 0;
            let remaining = &self.input[self.pos..];

            loop {
                if length >= remaining.len() {
                    break;
                }
                let c = remaining[length];

                // Check for ':' followed by blank/flow-indicator.
                if c == b':' {
                    let next = if length + 1 < remaining.len() {
                        remaining[length + 1]
                    } else {
                        0
                    };
                    if Self::is_blank_or_break(next)
                        || (in_flow && (next == b',' || next == b']' || next == b'}'))
                    {
                        break;
                    }
                }

                if in_flow && (c == b',' || c == b'[' || c == b']' || c == b'{' || c == b'}') {
                    break;
                }

                if Self::is_blank_or_break(c) {
                    break;
                }
                // `#` is a comment indicator when preceded by whitespace, or
                // at the start of a content segment (where prior whitespace
                // was already consumed by the outer loop).
                if c == b'#' && (length == 0 || Self::is_blank(remaining[length - 1])) {
                    break;
                }

                // Check for document indicators at start of line.
                if length == 0 && self.column() == 0 {
                    if c == b'-'
                        && self.peek_at(1) == b'-'
                        && self.peek_at(2) == b'-'
                        && (self.pos + 3 >= self.input.len()
                            || Self::is_blank_or_break(self.peek_at(3)))
                    {
                        break;
                    }
                    if c == b'.'
                        && self.peek_at(1) == b'.'
                        && self.peek_at(2) == b'.'
                        && (self.pos + 3 >= self.input.len()
                            || Self::is_blank_or_break(self.peek_at(3)))
                    {
                        break;
                    }
                }

                length += 1;
            }

            if length == 0 && !leading_blanks {
                break;
            }

            // Append characters.
            if length > 0 {
                if leading_blanks {
                    // Crossing a line break and synthesising folded
                    // whitespace means the emitted string no longer
                    // matches the input slice byte-for-byte. Switch
                    // to the owned-buffer path.
                    single_chunk = false;
                    // Handle line joins.
                    if let Some(stripped) = whitespace.strip_prefix('\n') {
                        if stripped.is_empty() {
                            string.push(' ');
                        } else {
                            // Multiple line breaks.
                            string.push_str(stripped);
                        }
                    } else {
                        string.push_str(&whitespace);
                    }
                    whitespace.clear();
                } else if !whitespace.is_empty() {
                    // Inline whitespace — already part of the input
                    // slice between the previous content_end and
                    // self.pos, so `single_chunk` stays true.
                    string.push_str(&whitespace);
                    whitespace.clear();
                }

                string.push_str(self.slice_str(self.pos, self.pos + length));
                self.advance_by(length);
                content_end = self.pos;
            }

            // Skip whitespace/newlines between plain scalar content.
            if !Self::is_blank_or_break(self.peek()) {
                break;
            }

            whitespace.clear();

            // Consume blanks and breaks.
            while Self::is_blank(self.peek()) {
                whitespace.push(self.peek() as char);
                self.advance();
            }

            if Self::is_break(self.peek()) {
                leading_blanks = true;
                whitespace.clear();
                // Consume runs of `break (blanks? break)*` so a line that
                // is only whitespace between two breaks is recorded as an
                // empty line (one extra `\n` in `whitespace`) rather than
                // collapsed silently. Mirrors quoted-scalar handling.
                loop {
                    let c = self.peek();
                    if c == b'\r' && self.peek_at(1) == b'\n' {
                        whitespace.push('\n');
                        self.advance_by(2);
                    } else {
                        whitespace.push('\n');
                        self.advance();
                    }
                    while Self::is_blank(self.peek()) {
                        self.advance();
                    }
                    if !Self::is_break(self.peek()) {
                        break;
                    }
                }

                if self.flow_level == 0 && (self.column() as i32) < indent {
                    break;
                }
            }
            // else: inline blanks between words — continue scanning.
        }

        if string.is_empty() {
            return Err(self.error("unexpected character in YAML stream"));
        }

        // If the plain scalar ended with the scanner at a new line in block
        // context (e.g. the value was followed by a newline that was consumed
        // during line folding), simple keys must be allowed again so that a
        // following key at the same indent level can be recognised.
        if self.flow_level == 0 && leading_blanks {
            self.simple_key_allowed = true;
        }

        // Borrow when the entire scalar is a single contiguous run of
        // input bytes (no line-folding synthesis). Else fall back to
        // the owned buffer that was accumulated above.
        let value = if single_chunk {
            Cow::Borrowed(self.slice_str(scalar_start, content_end))
        } else {
            Cow::Owned(string)
        };
        self.emit(TokenKind::Scalar(ScalarStyle::Plain, value));
        self.last_token_opens_block = false;
        Ok(())
    }

    pub(super) fn fetch_quoted_scalar(&mut self, double: bool) -> ScanResult<()> {
        // Quoted scalars are JSON-like: mark for adjacent value detection.
        self.save_simple_key_ext(true);
        self.simple_key_allowed = false;
        self.mark = self.pos;

        let string = if double {
            self.scan_double_quoted_scalar()?
        } else {
            self.scan_single_quoted_scalar()?
        };

        let style = if double {
            ScalarStyle::DoubleQuoted
        } else {
            ScalarStyle::SingleQuoted
        };
        self.emit(TokenKind::Scalar(style, Cow::Owned(string)));
        self.last_token_opens_block = false;
        // Both single- and double-quoted scalars are JSON-like nodes:
        // a following `:` is an adjacent value indicator in flow context.
        if self.flow_level > 0 {
            self.adjacent_value_allowed = true;
        }
        Ok(())
    }

    fn scan_single_quoted_scalar(&mut self) -> ScanResult<String> {
        self.advance(); // skip opening '

        let mut string = String::new();
        let mut whitespace = String::new();
        let mut leading_break = false;

        loop {
            if self.is_eof() {
                return Err(self.error("unterminated single-quoted string"));
            }

            match self.peek() {
                b'\'' => {
                    if self.peek_at(1) == b'\'' {
                        // Escaped single quote — flush pending fold/buffered whitespace.
                        if leading_break {
                            if whitespace.is_empty() {
                                string.push(' ');
                            } else {
                                string.push_str(&whitespace);
                            }
                            leading_break = false;
                        } else if !whitespace.is_empty() {
                            string.push_str(&whitespace);
                        }
                        whitespace.clear();
                        string.push('\'');
                        self.advance_by(2);
                    } else {
                        // End of string. Flush pending fold; trailing
                        // whitespace before the close-quote on the same
                        // line is preserved (mirrors double-quoted).
                        if leading_break {
                            if whitespace.is_empty() {
                                string.push(' ');
                            } else {
                                string.push_str(&whitespace);
                            }
                        } else if !whitespace.is_empty() {
                            string.push_str(&whitespace);
                        }
                        self.advance();
                        return Ok(string);
                    }
                }
                c if Self::is_break(c) => {
                    // Per YAML 1.2.2 §7.3.2: trailing whitespace before a
                    // break is stripped, and an empty line between content
                    // contributes a preserved `\n`. Mirrors the
                    // double-quoted handler — each break is processed in
                    // its own iteration so blanks-between-breaks are
                    // recognised as empty lines.
                    if leading_break {
                        whitespace.push('\n');
                    } else {
                        whitespace.clear();
                        leading_break = true;
                    }

                    if self.peek() == b'\r' && self.peek_at(1) == b'\n' {
                        self.advance_by(2);
                    } else {
                        self.advance();
                    }

                    self.reject_doc_marker_in_quoted("single-quoted")?;

                    while Self::is_blank(self.peek()) {
                        self.advance();
                    }

                    self.require_quoted_continuation_indent("single-quoted")?;
                }
                _ => {
                    // Flush any pending fold/whitespace before content.
                    if leading_break {
                        if whitespace.is_empty() {
                            string.push(' ');
                        } else {
                            string.push_str(&whitespace);
                        }
                        whitespace.clear();
                        leading_break = false;
                    } else if !whitespace.is_empty() {
                        string.push_str(&whitespace);
                        whitespace.clear();
                    }

                    // Spaces and tabs adjacent to a break or close-quote
                    // are buffered as candidate-trailing-whitespace; spaces
                    // adjacent to content are flushed and read inline.
                    if Self::is_blank(self.peek()) {
                        let start = self.pos;
                        while Self::is_blank(self.peek()) {
                            self.advance();
                        }
                        if Self::is_break(self.peek()) || self.peek() == b'\'' {
                            whitespace.push_str(self.slice_str(start, self.pos));
                            continue;
                        }
                        string.push_str(self.slice_str(start, self.pos));
                    } else {
                        // Bulk-copy a content run up to the next interesting
                        // byte. All needles are ASCII (<0x80) so they never
                        // appear as UTF-8 continuation bytes — slicing on
                        // a needle hit is always char-boundary safe.
                        let start = self.pos;
                        let len =
                            crate::simd::clean_prefix_len(&self.input[self.pos..], b"'\n\r \t");
                        let len = if len == 0 { 1 } else { len };
                        self.advance_by(len);
                        string.push_str(self.slice_str(start, self.pos));
                    }
                }
            }
        }
    }

    fn scan_double_quoted_scalar(&mut self) -> ScanResult<String> {
        self.advance(); // skip opening "

        let mut string = String::new();
        let mut whitespace = String::new();
        let mut leading_break = false;

        loop {
            if self.is_eof() {
                return Err(self.error("unterminated double-quoted string"));
            }

            match self.peek() {
                b'"' => {
                    // Flush any pending whitespace before closing.
                    if leading_break {
                        if whitespace.is_empty() {
                            string.push(' ');
                        } else {
                            string.push_str(&whitespace);
                        }
                    } else if !whitespace.is_empty() {
                        string.push_str(&whitespace);
                    }
                    self.advance();
                    return Ok(string);
                }
                b'\\' => {
                    // Flush pending whitespace.
                    if leading_break {
                        if whitespace.is_empty() {
                            string.push(' ');
                        } else {
                            string.push_str(&whitespace);
                        }
                        whitespace.clear();
                        leading_break = false;
                    } else if !whitespace.is_empty() {
                        string.push_str(&whitespace);
                        whitespace.clear();
                    }

                    self.advance(); // skip '\'
                    if self.is_eof() {
                        return Err(self.error("unexpected end of input in escape sequence"));
                    }
                    let escaped = self.peek();
                    self.advance();
                    match escaped {
                        b'0' => string.push('\0'),
                        b'a' => string.push('\x07'),
                        b'b' => string.push('\x08'),
                        b't' | b'\t' => string.push('\t'),
                        b'n' => string.push('\n'),
                        b'v' => string.push('\x0B'),
                        b'f' => string.push('\x0C'),
                        b'r' => string.push('\r'),
                        b'e' => string.push('\x1B'),
                        b' ' => string.push(' '),
                        b'"' => string.push('"'),
                        b'/' => string.push('/'),
                        b'\\' => string.push('\\'),
                        b'N' => string.push('\u{0085}'), // NEL
                        b'_' => string.push('\u{00A0}'), // NBSP
                        b'L' => string.push('\u{2028}'), // LS
                        b'P' => string.push('\u{2029}'), // PS
                        b'x' => {
                            let ch = self.scan_hex_escape(2)?;
                            string.push(ch);
                        }
                        b'u' => {
                            // JSON-style UTF-16 surrogate pair escape:
                            // `𝄞` encodes U+1D11E (𝄞). When
                            // we see a high surrogate, peek for a
                            // following `\uXXXX` low surrogate and pair
                            // them. Lone or reversed surrogates fall
                            // through to `scan_hex_escape_pair`'s
                            // existing rejection path.
                            let ch = self.scan_unicode_4()?;
                            string.push(ch);
                        }
                        b'U' => {
                            let ch = self.scan_hex_escape(8)?;
                            string.push(ch);
                        }
                        b'\r' | b'\n' => {
                            // Line break escape — fold.
                            if escaped == b'\r' && self.peek() == b'\n' {
                                self.advance();
                            }
                            // Skip leading whitespace on next line.
                            while Self::is_blank(self.peek()) {
                                self.advance();
                            }
                        }
                        _ => {
                            return Err(ScanError {
                                message: Cow::Owned(format!(
                                    "unknown escape character '\\{}'",
                                    escaped as char
                                )),
                                index: self.pos - 1,
                            });
                        }
                    }
                }
                c if Self::is_break(c) => {
                    // Line folding in double-quoted scalars per YAML 1.2.2
                    // §7.3.2 / §6.5: trailing whitespace before a break is
                    // stripped; an "empty line" (a line containing only
                    // whitespace, *or* nothing) between content lines
                    // contributes a preserved `\n`. Each break is handled
                    // in its own loop iteration so blanks-between-breaks
                    // are recognised as empty lines.
                    if leading_break {
                        // We're already in a break sequence — the previous
                        // iteration ended on a break and its trailing
                        // blanks have been consumed below. Reaching another
                        // break means the line in between was empty.
                        whitespace.push('\n');
                    } else {
                        // First break of a sequence — discard any buffered
                        // trailing whitespace before this break.
                        whitespace.clear();
                        leading_break = true;
                    }

                    if self.peek() == b'\r' && self.peek_at(1) == b'\n' {
                        self.advance_by(2);
                    } else {
                        self.advance();
                    }

                    self.reject_doc_marker_in_quoted("double-quoted")?;

                    // YAML 1.2.2 §6.1: only spaces count as
                    // indentation. Count leading *spaces* before any
                    // tab so the continuation-indent check uses the
                    // space-only column, not `self.col` (which counts
                    // tabs as columns and would mask a tab-as-indent
                    // bug in DK95 sub-case 2).
                    let space_indent = {
                        let mut n = 0;
                        while self.input.get(self.pos + n).copied() == Some(b' ') {
                            n += 1;
                        }
                        n as i32
                    };

                    // Skip leading blanks on the new line.
                    while Self::is_blank(self.peek()) {
                        self.advance();
                    }

                    self.require_quoted_continuation_indent_spaces("double-quoted", space_indent)?;
                }
                _ => {
                    if leading_break {
                        if whitespace.is_empty() {
                            string.push(' ');
                        } else {
                            string.push_str(&whitespace);
                        }
                        whitespace.clear();
                        leading_break = false;
                    } else if !whitespace.is_empty() {
                        string.push_str(&whitespace);
                        whitespace.clear();
                    }

                    // Handle whitespace chars specially for folding.
                    if Self::is_blank(self.peek()) {
                        let start = self.pos;
                        while Self::is_blank(self.peek()) {
                            self.advance();
                        }
                        if Self::is_break(self.peek())
                            || self.peek() == b'"'
                            || self.peek() == b'\\'
                        {
                            whitespace.push_str(self.slice_str(start, self.pos));
                            continue;
                        }
                        string.push_str(self.slice_str(start, self.pos));
                    } else {
                        // Bulk-copy a content run up to the next interesting
                        // byte. All needles are ASCII (<0x80) so they never
                        // appear as UTF-8 continuation bytes — slicing on
                        // a needle hit is always char-boundary safe.
                        let start = self.pos;
                        let len =
                            crate::simd::clean_prefix_len(&self.input[self.pos..], b"\"\\\n\r \t");
                        let len = if len == 0 { 1 } else { len };
                        self.advance_by(len);
                        string.push_str(self.slice_str(start, self.pos));
                    }
                }
            }
        }
    }

    fn scan_hex_escape(&mut self, digits: usize) -> ScanResult<char> {
        let start = self.pos;
        for _ in 0..digits {
            if self.is_eof() || !self.peek().is_ascii_hexdigit() {
                return Err(ScanError {
                    message: Cow::Owned(format!("expected {digits} hex digits in escape sequence")),
                    index: start,
                });
            }
            self.advance();
        }
        let hex_str = self.slice_str(start, self.pos);
        let code =
            u32::from_str_radix(hex_str, 16).map_err(|_| self.error("invalid hex escape"))?;
        char::from_u32(code).ok_or_else(|| ScanError {
            message: Cow::Owned(format!("invalid Unicode code point U+{code:04X}")),
            index: start,
        })
    }

    /// `\uXXXX` escape with JSON-style UTF-16 surrogate pairing.
    ///
    /// Reads four hex digits. If the value is a high surrogate
    /// (`U+D800..=U+DBFF`), peeks for an immediately-following
    /// `\uXXXX` low-surrogate (`U+DC00..=U+DFFF`) and combines the
    /// pair into a single supplementary-plane code point per the
    /// UTF-16 algorithm. A lone or reversed surrogate is rejected
    /// with the same error shape as a single bad `\uD800`.
    ///
    /// Lifted out of `scan_hex_escape` so the 2-digit (`\xXX`),
    /// 8-digit (`\UXXXXXXXX`), and tag-decode call sites keep the
    /// strict "no surrogate halves" invariant.
    fn scan_unicode_4(&mut self) -> ScanResult<char> {
        let start = self.pos;
        for _ in 0..4 {
            if self.is_eof() || !self.peek().is_ascii_hexdigit() {
                return Err(ScanError {
                    message: Cow::Owned("expected 4 hex digits in escape sequence".into()),
                    index: start,
                });
            }
            self.advance();
        }
        let hex_str = self.slice_str(start, self.pos);
        let code =
            u32::from_str_radix(hex_str, 16).map_err(|_| self.error("invalid hex escape"))?;

        // Fast path: not a surrogate at all.
        if let Some(ch) = char::from_u32(code) {
            return Ok(ch);
        }

        // Surrogate territory (U+D800..=U+DFFF).
        const HIGH_LO: u32 = 0xD800;
        const HIGH_HI: u32 = 0xDBFF;
        const LOW_LO: u32 = 0xDC00;
        const LOW_HI: u32 = 0xDFFF;

        if (HIGH_LO..=HIGH_HI).contains(&code) && self.peek() == b'\\' && self.peek_at(1) == b'u' {
            // Tentatively consume `\u`. If the following 4 digits do
            // not pair, error out at the original `\uD8XX` position.
            let pair_start = self.pos;
            self.advance_by(2);
            let low_start = self.pos;
            for _ in 0..4 {
                if self.is_eof() || !self.peek().is_ascii_hexdigit() {
                    return Err(ScanError {
                        message: Cow::Owned(
                            "high surrogate must be followed by a `\\uXXXX` low surrogate".into(),
                        ),
                        index: pair_start,
                    });
                }
                self.advance();
            }
            let low_hex = self.slice_str(low_start, self.pos);
            let low =
                u32::from_str_radix(low_hex, 16).map_err(|_| self.error("invalid hex escape"))?;
            if !(LOW_LO..=LOW_HI).contains(&low) {
                return Err(ScanError {
                    message: Cow::Owned(format!(
                        "high surrogate U+{code:04X} not followed by a low surrogate (got U+{low:04X})"
                    )),
                    index: pair_start,
                });
            }
            // High in [D800, DBFF] and low in [DC00, DFFF] yields
            // combined in [0x10000, 0x10FFFF] — always a valid
            // supplementary-plane code point, so `from_u32` cannot
            // return `None` here. `expect` documents the invariant.
            let combined = 0x10000 + ((code - HIGH_LO) << 10) + (low - LOW_LO);
            return Ok(char::from_u32(combined)
                .expect("surrogate pair math always yields a valid supplementary code point"));
        }

        // Lone surrogate (high without follow-up, or low surrogate
        // appearing first) — reject with the canonical error shape.
        Err(ScanError {
            message: Cow::Owned(format!("invalid Unicode code point U+{code:04X}")),
            index: start,
        })
    }

    pub(super) fn fetch_block_scalar(&mut self, literal: bool) -> ScanResult<()> {
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        self.mark = self.pos;

        let string = self.scan_block_scalar(literal)?;
        let style = if literal {
            ScalarStyle::Literal
        } else {
            ScalarStyle::Folded
        };
        self.emit(TokenKind::Scalar(style, Cow::Owned(string)));
        self.last_token_opens_block = false;
        Ok(())
    }

    fn scan_block_scalar(&mut self, literal: bool) -> ScanResult<String> {
        self.advance(); // skip '|' or '>'

        // Parse optional chomping indicator and indentation indicator.
        let mut chomping: i8 = 0; // 0 = clip, 1 = keep, -1 = strip
        let mut increment: usize = 0;

        // Check for chomping/indent indicators in either order.
        for _ in 0..2 {
            if !self.is_eof() {
                match self.peek() {
                    b'+' => {
                        chomping = 1;
                        self.advance();
                    }
                    b'-' => {
                        chomping = -1;
                        self.advance();
                    }
                    c if c.is_ascii_digit() && c != b'0' => {
                        increment = (c - b'0') as usize;
                        self.advance();
                    }
                    _ => break,
                }
            }
        }

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
                self.advance();
            }
        }

        // Consume the line break.
        if Self::is_break(self.peek()) {
            self.skip_line();
        }

        // Determine the indentation level and validate leading empty lines.
        let mut max_leading_empty_spaces = 0;
        let mut detected = 0;
        let mut has_content = false;
        let save_pos = self.pos;
        let save_col = self.col;
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
            detected = spaces;
            has_content = true;
            break;
        }
        self.pos = save_pos;
        self.col = save_col;

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

        // YAML 1.2.2 §8.1.1.2: If any leading empty line contains more spaces than
        // the indentation level, it is an error.
        if max_leading_empty_spaces > block_indent {
            return Err(self.error("a leading all-space line must not have too many spaces"));
        }

        // Read the block scalar content.
        let mut string = String::new();
        let mut trailing_breaks = String::new();
        let mut leading_blank = false;

        while !self.is_eof() {
            // Document boundary terminates the block scalar (matters when
            // `block_indent == 0`; otherwise the indent check below handles it).
            if self.col == 0 {
                let p0 = self.peek();
                let is_marker_byte = p0 == b'-' || p0 == b'.';
                if is_marker_byte
                    && self.peek_at(1) == p0
                    && self.peek_at(2) == p0
                    && (self.pos + 3 >= self.input.len()
                        || Self::is_blank_or_break(self.peek_at(3)))
                {
                    break;
                }
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
                return Err(
                    self.error("tab characters are not allowed as block-scalar indentation")
                );
            }

            if spaces < block_indent && !Self::is_break(self.peek()) && !self.is_eof() {
                // End of block scalar.
                break;
            }

            // Empty line (blank-only or break-only) — record and continue
            // before any fold decision so empty lines accumulate as `\n`s
            // in `trailing_breaks` rather than being treated as content.
            //
            // For *literal* style, a whitespace-only line whose leading
            // spaces exceed `block_indent` carries content: per YAML
            // 1.2.2 §8.1.1.4, every character at or beyond the content
            // indentation is preserved literally. The exception is
            // *leading* whitespace-only lines (before any real content
            // has been emitted) — those are part of the leading
            // empty-line region and contribute only their `\n`, not
            // their indent characters.
            if Self::is_break(self.peek()) || self.is_eof() {
                let extra = spaces.saturating_sub(block_indent);
                if !Self::is_break(self.peek()) {
                    // EOF reached after counting `spaces` blanks. Treat
                    // a whitespace-only trailing line as if it had a
                    // synthetic line break so the chomping pass below
                    // sees the same shape it would for the
                    // newline-terminated case (L24T spec test).
                    if literal && extra > 0 && !string.is_empty() {
                        if !trailing_breaks.is_empty() {
                            string.push_str(&trailing_breaks);
                            trailing_breaks.clear();
                        }
                        for _ in 0..extra {
                            string.push(' ');
                        }
                        trailing_breaks.push('\n');
                    }
                    break;
                }
                if literal && extra > 0 && !string.is_empty() {
                    if !trailing_breaks.is_empty() {
                        string.push_str(&trailing_breaks);
                        trailing_breaks.clear();
                    }
                    for _ in 0..extra {
                        string.push(' ');
                    }
                }
                trailing_breaks.push('\n');
                self.skip_line();
                continue;
            }

            // Determine more-indented status of the current content line.
            // YAML 1.2.2 §8.1.1.5: a line is "more-indented" if it has
            // extra leading spaces beyond `block_indent`, or if its first
            // non-leading-space character is a tab. The break(s) into and
            // out of a more-indented line are preserved (not folded).
            let extra = spaces.saturating_sub(block_indent);
            let starts_with_tab = self.peek() == b'\t';
            let is_more_indented = extra > 0 || starts_with_tab;

            // Apply fold logic. Order of cases:
            //   * literal style: every break preserved as-is.
            //   * before any content has been emitted: leading empty
            //     lines preserved (`b-l-folded` does not fold a leading
            //     break against the implicit header break).
            //   * either side is more-indented: every break preserved.
            //   * single break between regular content lines: fold to ' '.
            //   * multiple breaks between regular content: drop the
            //     leading break (the fold-into-empty-line) and keep the
            //     rest as `\n`s.
            if !trailing_breaks.is_empty() {
                let preserve_all =
                    literal || string.is_empty() || is_more_indented || leading_blank;
                if preserve_all {
                    string.push_str(&trailing_breaks);
                } else if trailing_breaks.len() == 1 {
                    string.push(' ');
                } else {
                    string.push_str(&trailing_breaks[1..]);
                }
                trailing_breaks.clear();
            }

            for _ in 0..extra {
                string.push(' ');
            }

            leading_blank = is_more_indented;

            // Read content of the line.
            while !self.is_eof() && !Self::is_break(self.peek()) {
                let start = self.pos;
                self.advance();
                while self.pos < self.input.len() && (self.input[self.pos] & 0xC0) == 0x80 {
                    self.advance();
                }
                string.push_str(self.slice_str(start, self.pos));
            }

            // Consume the line break.
            if Self::is_break(self.peek()) {
                trailing_breaks.push('\n');
                self.skip_line();
            }
        }

        // Apply chomping. YAML 1.2.2 §8.1.1.2:
        //   `+` (keep): preserve every trailing line break.
        //   default (clip): a single trailing `\n` if and only if the
        //     scalar has any content. An empty scalar with `>`/`|` and
        //     trailing blank lines stays empty.
        //   `-` (strip): no trailing line break.
        match chomping {
            1 => string.push_str(&trailing_breaks),
            0 if !string.is_empty() => string.push('\n'),
            _ => {}
        }

        Ok(string)
    }
}
