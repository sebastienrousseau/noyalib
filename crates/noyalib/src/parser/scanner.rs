//! YAML 1.2 lexical scanner.
//!
//! Converts a UTF-8 input string into a stream of [`Token`]s.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// VecDeque replaced with Vec + consumed index for better cache locality.

use crate::prelude::*;

/// Byte-offset span in the source input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Span {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// The style of a scalar token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

/// Token kinds emitted by the scanner.
///
/// String-carrying variants use `Cow<'a, str>` so that plain scalars and
/// anchor names can borrow directly from the input, avoiding allocation
/// when the value is immediately resolved to a non-String type.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum TokenKind<'a> {
    #[default]
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    BlockSequenceStart,
    BlockMappingStart,
    BlockEnd,
    FlowSequenceStart,
    FlowSequenceEnd,
    FlowMappingStart,
    FlowMappingEnd,
    BlockEntry,
    FlowEntry,
    Key,
    Value,
    Anchor(Cow<'a, str>),
    Alias(Cow<'a, str>),
    Tag(Cow<'a, str>, Cow<'a, str>),
    Scalar(ScalarStyle, Cow<'a, str>),
}

/// A token with its source span.
#[derive(Debug, Clone, Default)]
pub(crate) struct Token<'a> {
    pub(crate) kind: TokenKind<'a>,
    pub(crate) span: Span,
}

/// Error from the scanner.
#[derive(Debug, Clone)]
pub(crate) struct ScanError {
    pub(crate) message: Cow<'static, str>,
    pub(crate) index: usize,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte offset {}", self.message, self.index)
    }
}

type ScanResult<T> = Result<T, ScanError>;

/// Tracks potential simple keys.
#[derive(Debug, Clone)]
struct SimpleKey {
    possible: bool,
    required: bool,
    token_number: usize,
    index: usize,
    /// True when the simple key is a JSON-like node (quoted scalar or flow
    /// collection).  In flow context, `:` is a valid value indicator after
    /// a JSON-like key even without trailing whitespace.
    json_like: bool,
}

/// Lookup table for bytes that are blanks (space, tab) or line breaks (LF, CR).
/// Index by byte value for O(1) classification — replaces per-call branching.
static IS_BLANK_OR_BREAK: [bool; 256] = {
    let mut t = [false; 256];
    t[b' ' as usize] = true;
    t[b'\t' as usize] = true;
    t[b'\n' as usize] = true;
    t[b'\r' as usize] = true;
    t
};

/// YAML 1.2 lexical scanner.
#[derive(Debug)]
pub(crate) struct Scanner<'a> {
    input: &'a [u8],
    /// The original input as a `&str` — avoids `from_utf8_lossy` on slices.
    input_str: &'a str,
    pos: usize,
    /// The mark position for the current token start.
    mark: usize,
    /// Current column as **byte offset** from the last newline.
    ///
    /// This is a byte offset, not a character count. For ASCII-only content
    /// (including YAML indentation, which is restricted to spaces), byte and
    /// character columns are identical. For multi-byte UTF-8 scalars, the
    /// column reported in `Location` may differ from what editors show.
    /// `Location::from_index()` re-computes the correct character column
    /// when needed (e.g., for error formatting).
    col: usize,
    /// Output token buffer (contiguous for cache locality).
    tokens: Vec<Token<'a>>,
    /// Index of the next token to consume from `tokens`.
    tokens_consumed: usize,
    /// Total tokens produced (including consumed ones).
    tokens_produced: usize,
    /// Block indentation level stack.
    indent: i32,
    /// Block-scope indent history. 8 slots inline covers YAML nesting
    /// depth for the overwhelming majority of real-world documents and
    /// avoids a heap allocation for 32 bytes of data.
    indents: smallvec::SmallVec<[i32; 8]>,
    /// Flow nesting level (0 = block context).
    flow_level: u32,
    /// Simple key tracking stack.
    simple_keys: Vec<SimpleKey>,
    /// Whether a simple key is allowed at the current position.
    simple_key_allowed: bool,
    /// In flow context, `:` is a value indicator when immediately adjacent
    /// to a JSON-like key (double-quoted scalar, `]`, or `}`).  This flag
    /// is set after emitting such tokens and cleared on the next fetch.
    adjacent_value_allowed: bool,
    /// True after an explicit key indicator `?` is emitted.  Allows block
    /// entries (`-`) in the key content even though `simple_key_allowed` is
    /// false (which prevents duplicate Key insertion on `:` later).
    explicit_key_pending: bool,
    /// True once we've emitted StreamStart.
    stream_started: bool,
    /// True once we've emitted StreamEnd.
    stream_ended: bool,
    /// Captured comments in source order. Populated as the scanner
    /// skips comment bytes; readers drain via `take_comments`.
    comments: Vec<ScannedComment>,
    /// Whether a `%YAML` directive has been seen for the current
    /// document (cleared on each `DocumentStart`). Per YAML 1.2.2 §6.8.1
    /// a document may contain at most one `%YAML` directive.
    yaml_directive_seen: bool,
    /// `%TAG` handle → URI-prefix map for the current document. Cleared
    /// at every document boundary (`---`, `...`, stream end). When a
    /// later tag token uses the handle (e.g. `!!int` after
    /// `%TAG !! tag:example.com,2000:app/`), the scanner substitutes
    /// the full URI prefix in place of the handle so the loader sees
    /// the resolved tag without needing directive context (P76L).
    tag_handles: rustc_hash::FxHashMap<String, String>,
    flow_stack: Vec<bool>,
    /// When set, the scanner records inter-token trivia and source-
    /// bearing token spans for the green-tree builder. Off by default
    /// to keep the streaming/AST fast path zero-cost.
    recording: bool,
    trivia: Vec<Trivia>,
    recorded_tokens: Vec<RecordedToken>,
    /// `true` when the most recently emitted token was `:`/`?`/`-`
    /// (a block-collection-opener). The next token may legitimately
    /// appear at a column deeper than the current indent — e.g. the
    /// value of a key on the following line. Cleared as soon as the
    /// next token is dispatched. Used by the indent-rigor check
    /// added for §6.5 / §8.1 strict mode (4HVU, EW3V, …).
    last_token_opens_block: bool,
    /// `true` between a `---` directives-end indicator and the next
    /// line break — the only YAML node that may share that line is a
    /// scalar or flow collection (per §9.1.1). A block-structural
    /// token (`:` / `?` / `-`) here would open a block collection
    /// inline with `---`, which the spec rejects (CXX2, 9KBC).
    doc_start_inline: bool,
    /// `true` once a directive (`%YAML` / `%TAG` / reserved) has been
    /// consumed without an intervening `---`. Per §6.8, directives
    /// must be followed by an explicit `---`; otherwise the document
    /// they decorate never starts (9MMA, B63P).
    pending_directive_needs_doc_start: bool,
    /// `true` between the start of a document's content and the next
    /// `...` document-end marker (or stream end). Used to reject
    /// directives that appear without an intervening `...` to close
    /// the previous document (RHX7, EB22, 9HCY, MUS6:1).
    in_document_body: bool,
    /// Categorical record of the most recently *emitted* token. The
    /// `tokens` vec cannot be inspected reliably for this — slots
    /// past `tokens_consumed` are placeholder `StreamStart` after
    /// `core::mem::take` — so this field is updated in `emit` and
    /// preserved across the consume cycle. Used by the alias-decoration
    /// check (SR86, SU74).
    last_emitted_kind: LastEmitted,
    /// `(line_no, col)` of the most recently emitted node property
    /// (Anchor or Tag) that has not yet been followed by content.
    /// Cleared when a Scalar / Block-or-Flow Start is emitted. Used
    /// to enforce the YAML 1.2.2 §6.9.1 rule that node properties
    /// (and the node they decorate) must be indented strictly more
    /// than the parent block — H7J7 (`key: &x\n!!map\n  a: b`) puts
    /// the tag at column 0 ≤ parent indent, which is invalid.
    pending_property_col: Option<i32>,
}

/// Compact record of the most recent emit, used by guard checks
/// after the underlying `Token` may have been moved out via
/// `core::mem::take`. We only need to know "was it an anchor-or-tag"
/// for the current set of guards; if we ever need richer state, this
/// can grow to a full enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LastEmitted {
    #[default]
    Other,
    Anchor,
    Tag,
}

/// Internal comment record captured by the scanner.
///
/// Public at the crate level so the parser can hand it off to callers;
/// the public API lives in [`crate::comments`].
#[derive(Debug, Clone)]
pub(crate) struct ScannedComment {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: String,
    pub(crate) inline: bool,
}

/// Categorical kind of a piece of inter-token trivia recorded by the
/// scanner when the green-tree path is enabled. Used by `cst::Builder`
/// to materialise leaves that exactly reproduce source bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriviaKind {
    /// Run of inline blanks (spaces and tabs).
    Whitespace,
    /// A single line break (`\n` or `\r\n`).
    Newline,
    /// A UTF-8 byte-order mark consumed at stream start.
    Bom,
    /// A `%YAML` / `%TAG` / reserved directive line. The scanner
    /// validates and consumes these without emitting a token; the CST
    /// builder needs them to reproduce the source.
    Directive,
}

/// Inter-token trivia recorded for the green-tree builder. Only
/// populated when [`Scanner::enable_recording`] is set. The fields are
/// only read by the std-only CST builder, so they appear write-only on
/// the alloc-only path — `#[allow(dead_code)]` documents that.
#[cfg_attr(not(feature = "std"), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub(crate) struct Trivia {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: TriviaKind,
}

/// Categorical tag for tokens recorded for the green-tree builder.
/// A simplified mirror of [`TokenKind`] that excludes synthetic tokens
/// — the builder only needs source-bearing tokens plus the
/// zero-length structural events that bracket block collections
/// and implicit-key entry boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordedTokenKind {
    // ── leaf kinds (carry source bytes) ────────────────────────
    DocStart,
    DocEnd,
    DashIndicator,
    QuestionIndicator,
    ColonIndicator,
    Comma,
    OpenBracket,
    CloseBracket,
    OpenBrace,
    CloseBrace,
    AnchorMark,
    AliasMark,
    TagMark,
    PlainScalar,
    SingleQuotedScalar,
    DoubleQuotedScalar,
    LiteralScalar,
    FoldedScalar,

    // ── structural events (zero-length, no source bytes) ───────
    /// Opening of an implicit/explicit block mapping. Drives the
    /// builder to push a `BlockMapping` frame.
    BlockMapStart,
    /// Opening of a block sequence. Drives the builder to push a
    /// `BlockSequence` frame.
    BlockSeqStart,
    /// Closing of the current block collection — the assembler pops
    /// either a `BlockMapping` or a `BlockSequence` based on what is
    /// currently on top of its frame stack.
    BlockEnd,
    /// Zero-length marker the scanner emits before an implicit
    /// mapping key. Drives the builder to open a new `MappingEntry`.
    SyntheticKey,
}

impl RecordedTokenKind {
    fn from_token(kind: &TokenKind<'_>, span_len: usize) -> Option<Self> {
        Some(match kind {
            TokenKind::DocumentStart => Self::DocStart,
            TokenKind::DocumentEnd => Self::DocEnd,
            TokenKind::BlockEntry => Self::DashIndicator,
            TokenKind::FlowEntry => Self::Comma,
            // The scanner emits `Key` for both the explicit `?`
            // indicator and (zero-length) before an implicit key.
            // The span length is the only durable discriminant.
            TokenKind::Key => {
                if span_len == 0 {
                    Self::SyntheticKey
                } else {
                    Self::QuestionIndicator
                }
            }
            TokenKind::Value => Self::ColonIndicator,
            TokenKind::FlowSequenceStart => Self::OpenBracket,
            TokenKind::FlowSequenceEnd => Self::CloseBracket,
            TokenKind::FlowMappingStart => Self::OpenBrace,
            TokenKind::FlowMappingEnd => Self::CloseBrace,
            TokenKind::Anchor(_) => Self::AnchorMark,
            TokenKind::Alias(_) => Self::AliasMark,
            TokenKind::Tag(_, _) => Self::TagMark,
            TokenKind::Scalar(style, _) => match style {
                ScalarStyle::Plain => Self::PlainScalar,
                ScalarStyle::SingleQuoted => Self::SingleQuotedScalar,
                ScalarStyle::DoubleQuoted => Self::DoubleQuotedScalar,
                ScalarStyle::Literal => Self::LiteralScalar,
                ScalarStyle::Folded => Self::FoldedScalar,
            },
            TokenKind::BlockSequenceStart => Self::BlockSeqStart,
            TokenKind::BlockMappingStart => Self::BlockMapStart,
            TokenKind::BlockEnd => Self::BlockEnd,
            TokenKind::StreamStart | TokenKind::StreamEnd => return None,
        })
    }

    /// `true` for variants that produce a green-tree leaf. `false`
    /// for zero-length structural events that only manipulate the
    /// builder's frame stack.
    pub(crate) fn is_leaf(self) -> bool {
        !matches!(
            self,
            Self::BlockMapStart | Self::BlockSeqStart | Self::BlockEnd | Self::SyntheticKey
        )
    }
}

/// Source-bearing token recorded by the scanner for the green-tree
/// builder. Only populated when [`Scanner::enable_recording`] is set.
/// Only read by the std-only CST builder.
#[cfg_attr(not(feature = "std"), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub(crate) struct RecordedToken {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: RecordedTokenKind,
}

impl<'a> Scanner<'a> {
    /// Create a new scanner for the given input.
    pub(crate) fn new(input: &'a str) -> Self {
        // Pre-allocate based on input size heuristics:
        // ~1 token per 8 bytes, ~1 indent level per 64 bytes.
        let estimated_tokens = (input.len() / 8).max(16);
        let estimated_depth = (input.len() / 64).max(4);
        Scanner {
            input: input.as_bytes(),
            input_str: input,
            pos: 0,
            mark: 0,
            col: 0,
            tokens: Vec::with_capacity(estimated_tokens),
            tokens_consumed: 0,
            tokens_produced: 0,
            indent: -1,
            indents: smallvec::SmallVec::with_capacity(estimated_depth),
            flow_level: 0,
            simple_keys: Vec::with_capacity(estimated_depth),
            simple_key_allowed: false,
            adjacent_value_allowed: false,
            explicit_key_pending: false,
            stream_started: false,
            stream_ended: false,
            comments: Vec::new(),
            yaml_directive_seen: false,
            tag_handles: rustc_hash::FxHashMap::default(),
            flow_stack: Vec::new(),
            recording: false,
            trivia: Vec::new(),
            recorded_tokens: Vec::new(),
            // Initial value: at stream start, *any* column is valid
            // for the first token (root may be indented arbitrarily).
            last_token_opens_block: true,
            doc_start_inline: false,
            pending_directive_needs_doc_start: false,
            in_document_body: false,
            last_emitted_kind: LastEmitted::Other,
            pending_property_col: None,
        }
    }

    /// Enable recording of inter-token trivia and source-bearing
    /// token spans for the green-tree builder. Must be called before
    /// any tokens are fetched.
    #[cfg(feature = "std")]
    pub(crate) fn enable_recording(&mut self) {
        self.recording = true;
    }

    /// Drain the recorded inter-token trivia.
    #[cfg(feature = "std")]
    pub(crate) fn take_trivia(&mut self) -> Vec<Trivia> {
        core::mem::take(&mut self.trivia)
    }

    /// Drain the recorded source-bearing tokens.
    #[cfg(feature = "std")]
    pub(crate) fn take_recorded_tokens(&mut self) -> Vec<RecordedToken> {
        core::mem::take(&mut self.recorded_tokens)
    }

    /// Drain captured comments, leaving the scanner's internal buffer
    /// empty. Used by the public [`crate::load_comments`] path.
    pub(crate) fn take_comments(&mut self) -> Vec<ScannedComment> {
        core::mem::take(&mut self.comments)
    }

    /// Fetch the next token from the scanner.
    pub(crate) fn next_token(&mut self) -> ScanResult<Token<'a>> {
        // Ensure we have at least one token buffered.
        while self.needs_more_tokens() {
            self.fetch_next_token()?;
        }
        if self.tokens_consumed < self.tokens.len() {
            // Move the token out instead of cloning — avoids heap-allocating
            // copies of owned Strings inside Scalar/Anchor/Alias/Tag variants.
            let t = core::mem::take(&mut self.tokens[self.tokens_consumed]);
            // Record tokens for the green-tree builder when
            // recording is enabled. Two flavours land in this
            // stream: source-bearing leaves (scalars, indicators,
            // braces, …) and zero-length structural events
            // (BlockMapStart / BlockSeqStart / BlockEnd /
            // SyntheticKey) used to bracket composites. Stream
            // start / end remain filtered.
            if self.recording {
                let span_len = t.span.end.saturating_sub(t.span.start);
                if let Some(kind) = RecordedTokenKind::from_token(&t.kind, span_len) {
                    // Reject zero-length leaves — the scanner can
                    // legitimately emit a leaf-kind token with a
                    // collapsed span when it represents a synthetic
                    // boundary (e.g., the implicit-Key `Key` is mapped
                    // to `SyntheticKey` above; anything else with a
                    // zero span is a defensive no-op).
                    if !(kind.is_leaf() && span_len == 0) {
                        self.recorded_tokens.push(RecordedToken {
                            start: t.span.start,
                            end: t.span.end,
                            kind,
                        });
                    }
                }
            }
            self.tokens_consumed += 1;
            self.tokens_produced += 1;
            // Compact when we've consumed enough to avoid unbounded growth.
            // Use a higher threshold to amortize the O(n) shift cost.
            if self.tokens_consumed > 256 {
                drop(self.tokens.drain(..self.tokens_consumed));
                self.tokens_consumed = 0;
            }
            return Ok(t);
        }
        Err(self.error("unexpected end of token stream"))
    }

    fn needs_more_tokens(&self) -> bool {
        if self.stream_ended {
            return false;
        }
        if self.tokens_consumed >= self.tokens.len() {
            return true;
        }
        // Fast path: if no simple key is possible, no need to scan the list.
        // In most YAML, simple_keys has 0-2 entries with possible=true.
        let next_token = self.tokens_produced;
        self.simple_keys
            .iter()
            .any(|sk| sk.possible && sk.token_number == next_token)
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    #[inline]
    fn peek(&self) -> u8 {
        if self.pos < self.input.len() {
            self.input[self.pos]
        } else {
            0
        }
    }

    #[inline]
    fn peek_at(&self, offset: usize) -> u8 {
        let idx = self.pos + offset;
        if idx < self.input.len() {
            self.input[idx]
        } else {
            0
        }
    }

    #[inline]
    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    #[inline]
    fn advance(&mut self) {
        if self.pos < self.input.len() {
            if self.input[self.pos] == b'\n' {
                self.col = 0;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    #[inline]
    fn advance_by(&mut self, n: usize) {
        let end = (self.pos + n).min(self.input.len());
        let slice = &self.input[self.pos..end];
        // Fast path: no newlines in the slice (common for scalar content).
        match slice.iter().rposition(|&b| b == b'\n') {
            Some(last_nl) => {
                // Column resets at the last newline, then counts remaining bytes.
                self.col = slice.len() - last_nl - 1;
            }
            None => {
                self.col += slice.len();
            }
        }
        self.pos = end;
    }

    #[inline]
    fn column(&self) -> usize {
        self.col
    }

    /// Slice the original `&str` input without allocation.
    ///
    /// Defensive: clamps both indices to the input length and snaps
    /// any mid-char-boundary index back to the previous code-point
    /// boundary. This protects the panic-free contract documented
    /// in [`POLICIES.md` §8] even when an upstream bug leaves the
    /// scanner position mid-`u8` after a defective advance pattern
    /// — the scanner may raise an error with a degraded slice but
    /// never panics on `&str[..]` indexing.
    ///
    /// On well-formed input the clamping is a no-op (zero overhead).
    ///
    /// [`POLICIES.md` §8]: https://github.com/sebastienrousseau/noyalib/blob/main/doc/POLICIES.md#8-panic-policy
    #[inline]
    fn slice_str(&self, start: usize, end: usize) -> &'a str {
        let len = self.input_str.len();
        let end = end.min(len);
        let start = start.min(end);
        let start = floor_char_boundary(self.input_str, start);
        let end = floor_char_boundary(self.input_str, end);
        &self.input_str[start..end]
    }

    fn error(&self, msg: &'static str) -> ScanError {
        ScanError {
            message: Cow::Borrowed(msg),
            index: self.pos,
        }
    }

    fn emit(&mut self, kind: TokenKind<'a>) {
        // Document-content tokens establish that we are inside a
        // document body. Subsequent directives without an
        // intervening `...` will be rejected (RHX7, EB22, 9HCY,
        // MUS6:1). Stream and document boundary tokens themselves
        // do not count as content.
        if !matches!(
            kind,
            TokenKind::StreamStart
                | TokenKind::StreamEnd
                | TokenKind::DocumentStart
                | TokenKind::DocumentEnd
                | TokenKind::BlockEnd
        ) {
            self.in_document_body = true;
        }
        self.last_emitted_kind = match &kind {
            TokenKind::Anchor(_) => LastEmitted::Anchor,
            TokenKind::Tag(_, _) => LastEmitted::Tag,
            _ => LastEmitted::Other,
        };
        // Track unsatisfied node-property column (Anchor/Tag awaiting
        // content). Cleared on actual content emission so siblings on
        // future entries don't see stale state.
        match &kind {
            TokenKind::Anchor(_) | TokenKind::Tag(_, _) if self.pending_property_col.is_none() => {
                self.pending_property_col = Some(self.column() as i32);
            }
            TokenKind::Anchor(_) | TokenKind::Tag(_, _) => {}
            TokenKind::Scalar(_, _)
            | TokenKind::FlowSequenceStart
            | TokenKind::FlowMappingStart
            | TokenKind::BlockSequenceStart
            | TokenKind::BlockMappingStart
            | TokenKind::Alias(_) => {
                self.pending_property_col = None;
            }
            _ => {}
        }
        let span = Span {
            start: self.mark,
            end: self.pos,
        };
        self.tokens.push(Token { kind, span });
    }

    fn insert_token(&mut self, index: usize, kind: TokenKind<'a>, span: Span) {
        // `insert_token` is used to back-patch synthetic
        // BlockMappingStart / BlockSequenceStart / Key tokens for
        // simple-key promotion. These also count as document
        // content for the directive-after-content guard.
        if !matches!(kind, TokenKind::BlockEnd) {
            self.in_document_body = true;
        }
        self.tokens
            .insert(self.tokens_consumed + index, Token { kind, span });
    }

    // ── Whitespace / Newline ─────────────────────────────────────────────

    #[inline]
    fn is_blank(c: u8) -> bool {
        c == b' ' || c == b'\t'
    }

    #[inline]
    fn is_break(c: u8) -> bool {
        c == b'\n' || c == b'\r'
    }

    #[inline]
    fn is_blank_or_break(c: u8) -> bool {
        IS_BLANK_OR_BREAK[c as usize]
    }

    fn skip_blank(&mut self) {
        // `simd::match_prefix_len` is the polarity-correct sibling of
        // `clean_prefix_len`: counts the leading run that IS in the
        // needle set (vs. clean_prefix_len which counts the run that
        // ISN'T). Commit 1e7dace shipped a regression by passing the
        // wrong polarity here; using the explicitly-named match_prefix_len
        // primitive eliminates the bug class.
        let count = crate::simd::match_prefix_len(&self.input[self.pos..], b" \t");
        if count > 0 {
            self.col += count; // blanks never contain newlines
            self.pos += count;
        }
    }

    fn reject_illegal_tab(&self, start: usize, indicator: &'static str) -> ScanResult<()> {
        let span = &self.input[start..self.pos];
        if !span.contains(&b'\t') {
            return Ok(());
        }

        let next = self.peek();
        let is_structural = matches!(next, b'-' | b'?' | b':')
            && (self.pos + 1 >= self.input.len() || Self::is_blank_or_break(self.peek_at(1)));
        if !is_structural {
            return Ok(());
        }
        Err(ScanError {
            message: Cow::Owned(format!(
                "tabs are not allowed as separation or indentation after {indicator}"
            )),
            index: start,
        })
    }

    /// Non-advancing variant of [`reject_illegal_tab`]. Inspects the
    /// blanks starting at `self.pos` *without* moving the cursor — the
    /// blanks must remain available to the trivia recorder and the
    /// next plain-scalar reader. Mirrors the same rule: tabs that
    /// precede another block-structural indicator (`-`, `?`, `:`) are
    /// indentation tabs and rejected; tabs that precede content are
    /// fine (A2M4 spec example 6.2).
    fn check_illegal_tab_lookahead(&self, indicator: &'static str) -> ScanResult<()> {
        let mut p = self.pos;
        let mut saw_tab = false;
        while p < self.input.len() && Self::is_blank(self.input[p]) {
            if self.input[p] == b'\t' {
                saw_tab = true;
            }
            p += 1;
        }
        if !saw_tab {
            return Ok(());
        }
        let next = if p < self.input.len() {
            self.input[p]
        } else {
            0
        };
        let next_after = if p + 1 < self.input.len() {
            self.input[p + 1]
        } else {
            0
        };
        let is_structural = matches!(next, b'-' | b'?' | b':')
            && (p + 1 >= self.input.len() || Self::is_blank_or_break(next_after));
        if !is_structural {
            return Ok(());
        }
        Err(ScanError {
            message: Cow::Owned(format!(
                "tabs are not allowed as separation or indentation after {indicator}"
            )),
            index: self.pos,
        })
    }
    fn skip_line(&mut self) {
        let c = self.peek();
        if c == b'\r' && self.peek_at(1) == b'\n' {
            self.advance_by(2);
        } else if Self::is_break(c) {
            self.advance();
        }
    }

    fn skip_to_next_token(&mut self) -> ScanResult<()> {
        loop {
            // Whether the `#` we're about to process (if any) sits at
            // the start of a line (after only whitespace) or trails
            // real content on the same line.
            let inline = self.col > 0;
            // Skip whitespace (tabs are only allowed in some contexts).
            let blank_start = self.pos;
            self.skip_blank();
            if self.recording && self.pos > blank_start {
                self.trivia.push(Trivia {
                    start: blank_start,
                    end: self.pos,
                    kind: TriviaKind::Whitespace,
                });
            }

            // Skip comment — bulk-scan to next line break, capturing
            // the span and text for callers that want to read it back.
            if self.peek() == b'#' {
                // Per YAML 1.2.2 §6.6: a `#` starts a comment only when
                // preceded by whitespace, a line break, or the start of
                // the input. Look at the byte immediately before the `#`
                // — if it's any non-whitespace content character, this
                // is an inline `#` adjacent to prior content (e.g.
                // `"value"# bad`) and is not a valid comment indicator.
                if self.pos > 0 {
                    let prev = self.input[self.pos - 1];
                    if !Self::is_blank_or_break(prev) {
                        return Err(self.error(
                            "comment indicator '#' must be preceded by a space, tab, or line break",
                        ));
                    }
                }
                let comment_start = self.pos;
                let remaining = &self.input[self.pos..];
                // SIMD: comment text runs to the next line break.
                // memchr2 dispatches to SSE2 / NEON for the bulk-scan
                // and is materially faster than a byte-by-byte
                // `iter().position` on long comments.
                let end = memchr::memchr2(b'\n', b'\r', remaining).unwrap_or(remaining.len());
                let comment_end = comment_start + end;
                // `#` itself is at `comment_start`; the text starts
                // one byte later. Skip the `#` but keep any following
                // space so reconstruction preserves formatting.
                let text_start = comment_start + 1;
                let text = self.input_str[text_start..comment_end].to_owned();
                self.comments.push(ScannedComment {
                    start: comment_start,
                    end: comment_end,
                    text,
                    inline,
                });
                self.col += end;
                self.pos += end;
            }

            // Skip line break.
            if Self::is_break(self.peek()) {
                let break_start = self.pos;
                self.skip_line();
                if self.recording {
                    self.trivia.push(Trivia {
                        start: break_start,
                        end: self.pos,
                        kind: TriviaKind::Newline,
                    });
                }
                // The `---` line is over — block content may now
                // appear on subsequent lines under normal indent rules.
                self.doc_start_inline = false;
                // Anchors / tags only decorate a node on the *same*
                // line; once a line break is crossed, the node they
                // decorate is whatever appears after, which may be a
                // collection that *contains* an alias key. Clearing
                // `Anchor` / `Tag` here makes the alias-decoration
                // guard fire only on direct adjacency (SR86: `&b *a`)
                // and not on legitimate line-broken structures
                // (26DV: `&node3\n  *alias1: scalar3`).
                if matches!(
                    self.last_emitted_kind,
                    LastEmitted::Anchor | LastEmitted::Tag
                ) {
                    self.last_emitted_kind = LastEmitted::Other;
                }
                // Per YAML 1.2.2 §7.4 (Flow Collections): flow content
                // continuation across a line break must be indented
                // strictly more than the surrounding block — otherwise
                // it would be ambiguous with sibling block content
                // (9C9N). Skip when the new line is empty (only
                // blanks before the next break); compare against the
                // content column (after leading blanks), not the
                // line-start column.
                if self.flow_level > 0 && self.indent >= 0 {
                    // Count *spaces* first, then any blanks (tab or
                    // space). Per YAML 1.2.2 §6.1 only spaces count
                    // toward indentation; tabs that appear AFTER the
                    // space-prefix are valid inline separation
                    // (6HB6 line 16: `  <tab>Still by two`). Tabs
                    // BEFORE any space (i.e. at column 0 of a flow
                    // continuation line) ARE invalid indentation
                    // (Y79Y sub-case 4).
                    let mut look = self.pos;
                    while look < self.input.len() && self.input[look] == b' ' {
                        look += 1;
                    }
                    let space_indent_col = (look - self.pos) as i32;
                    while look < self.input.len() && Self::is_blank(self.input[look]) {
                        look += 1;
                    }
                    let line_has_content =
                        look < self.input.len() && !Self::is_break(self.input[look]);
                    if line_has_content && space_indent_col <= self.indent {
                        return Err(self.error(
                            "flow content must be indented more than the surrounding block",
                        ));
                    }
                }
                // In block context, allow simple key at line start.
                if self.flow_level == 0 {
                    self.simple_key_allowed = true;
                    // After a line break in block context, reject tabs as
                    // indentation — but only when the tab precedes actual
                    // content.  Tabs on otherwise-empty lines (tab followed
                    // by line break or EOF) are harmless whitespace.
                    if self.peek() == b'\t' {
                        // Scan ahead past the tab(s) and any following
                        // whitespace to see if content follows.
                        let mut look = self.pos;
                        while look < self.input.len() && Self::is_blank(self.input[look]) {
                            look += 1;
                        }
                        // If content follows (not a line break / EOF), the
                        // tab is being used as indentation which YAML forbids.
                        if look < self.input.len() && !Self::is_break(self.input[look]) {
                            return Err(self.error("tab characters are not allowed as indentation"));
                        }
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    // ── Indentation ──────────────────────────────────────────────────────

    fn roll_indent(
        &mut self,
        column: i32,
        number: Option<usize>,
        kind: TokenKind<'a>,
        mark: usize,
    ) {
        if self.flow_level > 0 {
            return;
        }
        if self.indent < column {
            self.indents.push(self.indent);
            self.indent = column;
            let span = Span {
                start: mark,
                end: mark,
            };
            match number {
                Some(n) => {
                    let idx = n - self.tokens_produced;
                    self.insert_token(idx, kind, span);
                }
                None => {
                    self.tokens.push(Token { kind, span });
                }
            }
        }
    }

    #[inline]
    fn unroll_indent(&mut self, column: i32) {
        if self.flow_level > 0 {
            return;
        }
        while self.indent > column {
            self.indent = self.indents.pop().unwrap_or(-1);
            self.emit(TokenKind::BlockEnd);
        }
    }

    // ── Simple keys ──────────────────────────────────────────────────────

    #[inline]
    fn save_simple_key(&mut self) {
        self.save_simple_key_ext(false);
    }

    #[inline]
    fn save_simple_key_ext(&mut self, json_like: bool) {
        if !self.simple_key_allowed {
            return;
        }
        let required = self.flow_level == 0 && self.indent == self.column() as i32;
        let sk = SimpleKey {
            possible: true,
            required,
            token_number: self.tokens_produced + (self.tokens.len() - self.tokens_consumed),
            index: self.pos,
            json_like,
        };
        // Inline remove_simple_key for the common case (not required).
        if let Some(last) = self.simple_keys.last_mut() {
            last.possible = false;
            *last = sk;
        }
    }

    #[inline]
    fn remove_simple_key(&mut self) -> ScanResult<()> {
        if let Some(sk) = self.simple_keys.last() {
            if sk.possible && sk.required {
                return Err(ScanError {
                    message: Cow::Borrowed("simple key was required but not found"),
                    index: sk.index,
                });
            }
        }
        if let Some(sk) = self.simple_keys.last_mut() {
            sk.possible = false;
        }
        Ok(())
    }

    fn stale_simple_keys(&mut self) -> ScanResult<()> {
        // In this implementation we don't enforce the 1024-char limit for
        // simple keys in block context (yaml-rust2 also relaxes this).
        Ok(())
    }

    // ── Main dispatch ────────────────────────────────────────────────────

    fn fetch_next_token(&mut self) -> ScanResult<()> {
        if !self.stream_started {
            return self.fetch_stream_start();
        }

        // `adjacent_value_allowed` is valid only when the very next byte is
        // `:` with no intervening whitespace.  Capture and clear it here;
        // `skip_to_next_token` may consume whitespace which breaks adjacency.
        let pos_before_skip = self.pos;
        let adj = self.adjacent_value_allowed;
        self.adjacent_value_allowed = false;

        self.skip_to_next_token()?;
        // If any whitespace was consumed, adjacency is broken.
        let adjacent_value = adj && self.pos == pos_before_skip;
        self.stale_simple_keys()?;
        self.unroll_indent(self.column() as i32);

        // Indent-rigor check (YAML 1.2.2 §6.5 / §8.1): after
        // unrolling, a column deeper than the current block indent
        // is only legal when the previous token opened a block
        // (`:` / `?` / `-`). The flag is set in `fetch_value` /
        // `fetch_key` / `fetch_block_entry` and cleared at the end
        // of node-emitting fetchers (scalar, alias, flow open) —
        // node-property tokens (anchor, tag) leave it alone so the
        // block-open intent flows through to the actual node.
        // Catches 4HVU, EW3V, DMG6, N4JP, U44R-class cases.
        if self.flow_level == 0
            && self.indent >= 0
            && (self.column() as i32) > self.indent
            && !self.last_token_opens_block
            && self.simple_key_allowed
            && !self.is_eof()
        {
            return Err(self.error(
                "inconsistent indentation: token at a column that does not match \
                 any open block scope",
            ));
        }

        self.mark = self.pos;

        if self.is_eof() {
            return self.fetch_stream_end();
        }

        let c = self.peek();

        // Check for document indicators at column 0.
        if self.column() == 0 {
            if c == b'-'
                && self.peek_at(1) == b'-'
                && self.peek_at(2) == b'-'
                && (self.pos + 3 >= self.input.len() || Self::is_blank_or_break(self.peek_at(3)))
            {
                return self.fetch_document_indicator(true);
            }
            if c == b'.'
                && self.peek_at(1) == b'.'
                && self.peek_at(2) == b'.'
                && (self.pos + 3 >= self.input.len() || Self::is_blank_or_break(self.peek_at(3)))
            {
                return self.fetch_document_indicator(false);
            }
        }

        // `-`, `?`, `:` at the very end of the input (no trailing
        // newline) are still indicators per YAML 1.2 — a single `-`
        // becomes a block sequence with one null entry, a single `:`
        // becomes a mapping with empty key→null (SM9W). `peek_at(1)`
        // returns 0 past EOF, which fails `is_blank_or_break`, so an
        // explicit EOF check is required here.
        let next_is_terminator = self.pos + 1 >= self.input.len();

        match c {
            b'[' => self.fetch_flow_collection_start(true),
            b'{' => self.fetch_flow_collection_start(false),
            b']' => self.fetch_flow_collection_end(true),
            b'}' => self.fetch_flow_collection_end(false),
            b',' => self.fetch_flow_entry(),
            b'-' if next_is_terminator
                || Self::is_blank_or_break(self.peek_at(1))
                || (self.flow_level > 0
                    && (self.peek_at(1) == b','
                        || self.peek_at(1) == b']'
                        || self.peek_at(1) == b'}')) =>
            {
                self.fetch_block_entry()
            }
            b'?' if next_is_terminator
                || Self::is_blank_or_break(self.peek_at(1))
                || (self.flow_level > 0
                    && (self.peek_at(1) == b','
                        || self.peek_at(1) == b']'
                        || self.peek_at(1) == b'}')) =>
            {
                self.fetch_key()
            }
            b':' if next_is_terminator
                || Self::is_blank_or_break(self.peek_at(1))
                || (self.flow_level > 0
                    && (self.peek_at(1) == b','
                        || self.peek_at(1) == b']'
                        || self.peek_at(1) == b'}'))
                // Adjacent value: `:` immediately after a JSON-like key
                // (quoted scalar, `]`, or `}`) in flow context — no space needed.
                || (self.flow_level > 0 && adjacent_value)
                // JSON-like simple key: `:` after a pending simple key that
                // was a quoted scalar or flow collection, even across whitespace.
                || (self.flow_level > 0
                    && self.simple_keys.last().is_some_and(|sk| sk.possible && sk.json_like)) =>
            {
                self.fetch_value()
            }
            b'*' => self.fetch_alias(),
            b'&' => self.fetch_anchor(),
            b'!' => self.fetch_tag(),
            b'|' if self.flow_level == 0 => self.fetch_block_scalar(true),
            b'>' if self.flow_level == 0 => self.fetch_block_scalar(false),
            b'\'' => self.fetch_quoted_scalar(false),
            b'"' => self.fetch_quoted_scalar(true),
            b'%' if self.column() == 0 => self.fetch_directive(),
            // BOM at start
            0xEF if self.pos == 0 && self.peek_at(1) == 0xBB && self.peek_at(2) == 0xBF => {
                self.advance_by(3);
                Ok(())
            }
            _ => self.fetch_plain_scalar(),
        }
    }

    // ── Token fetchers ───────────────────────────────────────────────────

    fn fetch_stream_start(&mut self) -> ScanResult<()> {
        self.stream_started = true;
        self.simple_key_allowed = true;
        self.simple_keys.push(SimpleKey {
            possible: false,
            required: false,
            token_number: 0,
            index: 0,
            json_like: false,
        });
        // Skip BOM if present.
        if self.pos + 2 < self.input.len()
            && self.input[self.pos] == 0xEF
            && self.input[self.pos + 1] == 0xBB
            && self.input[self.pos + 2] == 0xBF
        {
            let bom_start = self.pos;
            self.advance_by(3);
            if self.recording {
                self.trivia.push(Trivia {
                    start: bom_start,
                    end: self.pos,
                    kind: TriviaKind::Bom,
                });
            }
        }
        self.mark = self.pos;
        self.emit(TokenKind::StreamStart);
        Ok(())
    }

    fn fetch_stream_end(&mut self) -> ScanResult<()> {
        // Per YAML 1.2.2 §6.8: a directive must be followed by an
        // explicit `---` document-start indicator. Reaching stream
        // end with a pending directive means no document was ever
        // opened for it — invalid (9MMA, B63P).
        if self.pending_directive_needs_doc_start {
            return Err(
                self.error("directive must be followed by an explicit '---' document indicator")
            );
        }
        // Force-close any open blocks.
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        self.stream_ended = true;
        self.emit(TokenKind::StreamEnd);
        Ok(())
    }

    fn fetch_directive(&mut self) -> ScanResult<()> {
        // Per YAML 1.2.2 §6.8 / §9.1.2: a directive must not appear
        // after document content without an intervening `...`. The
        // previous document needs an explicit footer first (RHX7,
        // EB22, 9HCY, MUS6:1).
        if self.in_document_body {
            return Err(
                self.error("directive must be preceded by '...' to close the previous document")
            );
        }
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        // Directives must be followed by an explicit `---`. Record
        // that we owe one; cleared on `DocumentStart`, asserted at
        // stream end (9MMA, B63P).
        self.pending_directive_needs_doc_start = true;

        let directive_start = self.pos;
        // Skip the leading `%` and parse the directive name. Stop at
        // `#` as well so the post-validation comment-whitespace check
        // can flag `%foo#bad`-style packing.
        self.advance();
        let name_start = self.pos;
        while !self.is_eof() && !Self::is_blank_or_break(self.peek()) && self.peek() != b'#' {
            self.advance();
        }
        let name = self.slice_str(name_start, self.pos).to_owned();

        // Per YAML 1.2.2 §6.8.1: only one `%YAML` directive per document.
        // We accept the questionable `%YAML 1.1 1.2` form (ZYU8: extra
        // numeric token is "valid YAML according to the 1.2 productions,
        // just not usefully valid") but reject clearly-malformed
        // alphabetic trailing content like `%YAML 1.2 foo` (H7TQ).
        if name == "TAG" {
            self.skip_blank();
            if self.peek() == b'!' {
                let h_start = self.pos;
                while !self.is_eof() && !Self::is_blank_or_break(self.peek()) {
                    self.advance();
                }
                let handle = self.slice_str(h_start, self.pos).to_owned();
                self.skip_blank();
                let p_start = self.pos;
                while !self.is_eof() && !Self::is_blank_or_break(self.peek()) {
                    self.advance();
                }
                let prefix = self.slice_str(p_start, self.pos).to_owned();
                let _ = self.tag_handles.insert(handle, prefix);
            }
        } else if name == "YAML" {
            if self.yaml_directive_seen {
                return Err(
                    self.error("duplicate %YAML directive (at most one allowed per document)")
                );
            }
            self.yaml_directive_seen = true;
            self.skip_blank();
            // Same `#` stopping rule as the directive-name loop —
            // `1.1#...` (MUS6:0) packs a comment indicator straight
            // against the version digits and must fail validation.
            while !self.is_eof() && !Self::is_blank_or_break(self.peek()) && self.peek() != b'#' {
                self.advance();
            }
            self.skip_blank();
            if !self.is_eof() && !Self::is_break(self.peek()) && self.peek() != b'#' {
                let extra = self.peek();
                if !extra.is_ascii_digit() && extra != b'.' {
                    return Err(self.error("unexpected non-numeric argument on %YAML directive"));
                }
            }
        }

        // Per YAML 1.2.2 §6.6: a `#` on the directive line introduces
        // a trailing comment only when preceded by whitespace.
        // `%YAML 1.1#...` (MUS6:0) packs `#` directly against the
        // version digits and is invalid.
        if self.peek() == b'#' && self.pos > 0 && !Self::is_blank_or_break(self.input[self.pos - 1])
        {
            return Err(self.error(
                "comment indicator '#' on a directive line must be preceded by a space or tab",
            ));
        }

        // Skip to end of line — directive contents past validation are
        // not interpreted further (consumers don't need the version).
        if let Some(pos) = memchr::memchr2(b'\n', b'\r', &self.input[self.pos..]) {
            self.advance_by(pos);
        } else {
            self.pos = self.input.len();
        }
        if self.recording {
            self.trivia.push(Trivia {
                start: directive_start,
                end: self.pos,
                kind: TriviaKind::Directive,
            });
        }
        Ok(())
    }

    fn fetch_document_indicator(&mut self, is_start: bool) -> ScanResult<()> {
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        self.mark = self.pos;
        self.advance_by(3);
        if is_start {
            self.emit(TokenKind::DocumentStart);
            self.doc_start_inline = true;

            // YAML 1.2.2 §6.8.1: "The scope of a directive is the
            // document that follows it." Directives declared *just
            // before* this `---` apply to the document we are now
            // opening, so we must NOT clear them on `is_start` — they
            // were just registered (`pending_directive_needs_doc_start`
            // is the signal). They are cleared at `...` (DocEnd) and
            // also at a subsequent `---` if the previous document
            // ended implicitly (RHX7, EB22, 9HCY, MUS6:1, P76L). The
            // implicit-transition case is detected by `in_document_body`
            // still being true when we reach a new `---`.
            if self.in_document_body && !self.pending_directive_needs_doc_start {
                self.yaml_directive_seen = false;
                self.tag_handles.clear();
            }
            self.pending_directive_needs_doc_start = false;
            self.in_document_body = false;
            // NOTE: do NOT `self.skip_blank()` here. After `---`, the
            // following blanks are part of the next token's leading
            // separation (a plain scalar, flow indicator, etc.) — the
            // regular `skip_to_next_token()` invoked by the next
            // `fetch_*` call records them as trivia for the CST and
            // folds them into the scalar's leading whitespace for the
            // streaming/AST path. Eagerly consuming them here advances
            // `self.pos` without recording trivia, which causes the
            // CST's plain-scalar token span to come out one byte short
            // on its tail (cst_round_trip 27NA, 6FWR, 753E, …).
        } else {
            self.emit(TokenKind::DocumentEnd);
            // Directives are scoped to a single document.
            self.yaml_directive_seen = false;
            self.in_document_body = false;
            self.tag_handles.clear();
            // Per YAML 1.2.2 §6.8: a `...` document-end marker may be
            // followed by inline whitespace and an optional comment,
            // then a line break. Validate that here via *lookahead* —
            // we must not advance `self.pos`, because every byte
            // between this token and the next must be visible to the
            // CST trivia recorder (`skip_to_next_token` runs before
            // the next `fetch_*`). Eagerly `skip_blank()`-ing here
            // silently consumes those bytes and causes the next token's
            // recorded span to come out one byte short on its tail
            // (cst_round_trip RTP8).
            let mut look = self.pos;
            while look < self.input.len() && Self::is_blank(self.input[look]) {
                look += 1;
            }
            if self.input[self.pos..look].contains(&b'\t') {
                return Err(self.error("tabs are not allowed as separation after document marker"));
            }
            if look < self.input.len()
                && !Self::is_break(self.input[look])
                && self.input[look] != b'#'
            {
                return Err(self.error("unexpected content after document-end marker '...'"));
            }
        }
        Ok(())
    }

    fn fetch_flow_collection_start(&mut self, is_seq: bool) -> ScanResult<()> {
        // Flow collections are JSON-like: `[...]` and `{...}` can act as
        // mapping keys with adjacent `:`.
        self.save_simple_key_ext(true);
        self.flow_level += 1;
        // Push a new simple-key context for this flow level so that a `:`
        // inside the collection does not retroactively consume a simple key
        // that started *outside* the collection (e.g. `[` itself).
        self.simple_keys.push(SimpleKey {
            possible: false,
            required: false,
            token_number: 0,
            index: 0,
            json_like: false,
        });
        self.simple_key_allowed = true;
        self.mark = self.pos;
        self.advance();
        if is_seq {
            self.emit(TokenKind::FlowSequenceStart);
            self.flow_stack.push(true);
        } else {
            self.emit(TokenKind::FlowMappingStart);
            self.flow_stack.push(false);
        }
        // The collection itself is the node that the pending block-
        // open targeted; clear so subsequent tokens inside the flow
        // are checked against their own scope.
        self.last_token_opens_block = false;
        Ok(())
    }

    fn fetch_flow_collection_end(&mut self, is_seq: bool) -> ScanResult<()> {
        // Per YAML 1.2.2 §7.4: `]` / `}` may only close an open flow
        // collection of the matching kind. A stray closing indicator
        // outside any flow context (e.g. `[a, b] ]`) is invalid.
        if self.flow_level == 0 {
            return Err(self.error(if is_seq {
                "unexpected ']' outside of any flow sequence"
            } else {
                "unexpected '}' outside of any flow mapping"
            }));
        }
        self.remove_simple_key()?;
        // Pop the simple-key context that was pushed when this flow
        // collection was opened.
        let _ = self.simple_keys.pop();
        self.flow_level -= 1;
        let _ = self.flow_stack.pop();
        self.simple_key_allowed = false;
        self.mark = self.pos;
        self.advance();
        if is_seq {
            self.emit(TokenKind::FlowSequenceEnd);
        } else {
            self.emit(TokenKind::FlowMappingEnd);
        }
        // `]` and `}` end JSON-like nodes — a following `:` is an
        // adjacent value indicator when still inside a flow context.
        if self.flow_level > 0 {
            self.adjacent_value_allowed = true;
        }
        Ok(())
    }

    fn fetch_flow_entry(&mut self) -> ScanResult<()> {
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        self.mark = self.pos;
        self.advance();
        self.emit(TokenKind::FlowEntry);
        Ok(())
    }

    /// Per YAML 1.2.2 §9.1.1, the only YAML node that may share a
    /// line with the `---` directives-end indicator is a scalar or
    /// flow collection. Opening a block collection on the same line
    /// (via `:` / `?` / `-` outside any `{}`/`[]`) is invalid —
    /// `---` followed by `key: value` is *not* a one-line block
    /// mapping (CXX2, 9KBC).
    fn reject_block_inline_with_doc_start(&self, indicator: &'static str) -> ScanResult<()> {
        if self.doc_start_inline {
            return Err(ScanError {
                message: Cow::Owned(format!(
                    "{indicator} cannot open a block collection on the same line as '---'"
                )),
                index: self.pos,
            });
        }
        Ok(())
    }

    /// Per YAML 1.2.2 §7.3.2 / §7.3.3, continuation lines of a
    /// multi-line quoted scalar in *block* context must be indented
    /// strictly more than the parent block's indent — otherwise the
    /// scalar's continuation could be confused with a sibling at the
    /// parent level. Called by each quoted-scalar break handler
    /// after the trailing line break and any leading blanks have
    /// been consumed; `pos` therefore sits at the first content
    /// byte of the new line. Skipped in flow context, where the
    /// indent rule is governed by `s-flow-line-prefix(n)` and
    /// already enforced by the surrounding flow scaffolding.
    fn require_quoted_continuation_indent(&self, style: &'static str) -> ScanResult<()> {
        if self.flow_level != 0 {
            return Ok(());
        }
        if self.is_eof() || Self::is_break(self.peek()) {
            // Trailing whitespace / blank line — the closing quote
            // (or another break) handles termination.
            return Ok(());
        }
        if (self.col as i32) <= self.indent {
            return Err(ScanError {
                message: Cow::Owned(format!(
                    "{style} continuation must be indented more than the parent block"
                )),
                index: self.pos,
            });
        }
        Ok(())
    }

    /// Variant of [`Self::require_quoted_continuation_indent`] that
    /// uses an externally-computed *space-only* indent column instead
    /// of `self.col` (which counts tabs). Per YAML 1.2.2 §6.1 only
    /// spaces count as indentation; this catches the case where the
    /// continuation line's leading whitespace is `\t…` — `self.col`
    /// would be 1 (visually past the parent block's column 0 mapping)
    /// but `space_indent` is 0, correctly flagging it as under-indented
    /// (DK95 sub-case 2).
    fn require_quoted_continuation_indent_spaces(
        &self,
        style: &'static str,
        space_indent: i32,
    ) -> ScanResult<()> {
        if self.flow_level != 0 {
            return Ok(());
        }
        if self.is_eof() || Self::is_break(self.peek()) {
            return Ok(());
        }
        if space_indent <= self.indent {
            return Err(ScanError {
                message: Cow::Owned(format!(
                    "{style} continuation must be indented more than the parent block"
                )),
                index: self.pos,
            });
        }
        Ok(())
    }

    /// Per YAML 1.2.2 §6.8 / §6.7, a `---` or `...` indicator at column 0
    /// terminates the surrounding document. A multi-line quoted scalar
    /// that crosses such a marker is invalid (the indicator is not
    /// content; it would prematurely close the document). Called from
    /// each quoted-scalar break handler immediately after a line break
    /// has been consumed and `pos` sits at column 0 of the next line.
    fn reject_doc_marker_in_quoted(&self, style: &'static str) -> ScanResult<()> {
        if self.col != 0 || self.is_eof() {
            return Ok(());
        }
        let p0 = self.peek();
        if p0 != b'-' && p0 != b'.' {
            return Ok(());
        }
        if self.peek_at(1) != p0 || self.peek_at(2) != p0 {
            return Ok(());
        }
        if self.pos + 3 < self.input.len() && !Self::is_blank_or_break(self.peek_at(3)) {
            return Ok(());
        }
        Err(ScanError {
            message: Cow::Owned(format!(
                "document marker '{}{}{}' is not allowed inside a {style} scalar",
                p0 as char, p0 as char, p0 as char,
            )),
            index: self.pos,
        })
    }

    /// After a block-structural indicator (`-`, `?`, `:`), verify the
    /// separation does not end in a tab immediately followed by another
    /// structural indicator. Per YAML 1.2.2 §6.1 tabs are valid as
    /// inline whitespace (spec example 6.3: `:\t bar` is fine), but a
    /// tab cannot stand in for indentation when the next token would
    /// itself open a new block scope — that is the Y79Y-class issue.
    fn _reject_tab_indent_after_indicator(&self, indicator: &'static str) -> ScanResult<()> {
        if self.flow_level != 0 {
            return Ok(());
        }
        let mut look = self.pos;
        let mut last_ws: u8 = 0;
        while look < self.input.len() && Self::is_blank(self.input[look]) {
            last_ws = self.input[look];
            look += 1;
        }
        if last_ws != b'\t' || look >= self.input.len() {
            return Ok(());
        }
        let next = self.input[look];
        // Only reject when the following content is itself a structural
        // token whose position is interpreted as indentation. Plain
        // content after a tab is permitted (it is folded as scalar
        // whitespace, not indentation).
        let is_structural = matches!(next, b'-' | b'?' | b':')
            && (look + 1 >= self.input.len() || Self::is_blank_or_break(self.input[look + 1]));
        if !is_structural {
            return Ok(());
        }
        Err(ScanError {
            message: Cow::Owned(format!(
                "tab character cannot precede a block-structural indicator after {indicator}"
            )),
            index: self.pos,
        })
    }

    fn fetch_block_entry(&mut self) -> ScanResult<()> {
        if self.flow_level == 0 {
            if !self.simple_key_allowed && !self.explicit_key_pending {
                return Err(self.error("block sequence entries are not allowed in this context"));
            }
            self.reject_block_inline_with_doc_start("'-'")?;
            let col = self.column() as i32;
            self.roll_indent(col, None, TokenKind::BlockSequenceStart, self.pos);
        }
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        self.mark = self.pos;
        self.advance();
        // YAML 1.2.2 §6.1: tabs may appear as inline separation before
        // content but not as indentation before another structural
        // indicator. `- \tfoo` and `-\tfoo` are valid (A2M4); only
        // `-\t-` and similar cascades are rejected. The lookahead
        // variant must not advance `self.pos` — the CST trivia
        // recorder needs those blanks (cst_round_trip).
        if self.flow_level == 0 {
            self.check_illegal_tab_lookahead("'-'")?;
        }
        self.emit(TokenKind::BlockEntry);
        self.last_token_opens_block = true;
        Ok(())
    }

    fn fetch_key(&mut self) -> ScanResult<()> {
        if self.flow_level == 0 {
            if !self.simple_key_allowed {
                return Err(self.error("mapping keys are not allowed in this context"));
            }
            self.reject_block_inline_with_doc_start("'?'")?;
            let col = self.column() as i32;
            self.roll_indent(col, None, TokenKind::BlockMappingStart, self.pos);
        }
        self.remove_simple_key()?;
        // In *block* context, allow a simple key on the same line as
        // `?` so `? key: value` parses as an explicit-key mapping
        // whose key is the single-pair mapping `{key: value}` (V9D5
        // spec example 8.19). The cross-line case `? key\n: value`
        // is still safe: the existing fetch_value rule-1 logic
        // invalidates the simple key when a newline separates it from
        // the `:`, preventing duplicate Key emission. In *flow*
        // context (`{? foo\n bar : baz`), `?` continues to disable
        // simple-key tracking — flow's own folded-key parser handles
        // multi-line keys without the simple-key promotion path.
        if self.flow_level == 0 {
            self.simple_key_allowed = true;
            self.explicit_key_pending = true;
        } else {
            self.simple_key_allowed = false;
        }
        self.mark = self.pos;
        self.advance();
        // Same A2M4-style relaxation as `fetch_block_entry`.
        if self.flow_level == 0 {
            self.check_illegal_tab_lookahead("'?'")?;
        }
        self.emit(TokenKind::Key);
        self.last_token_opens_block = true;
        Ok(())
    }

    fn fetch_value(&mut self) -> ScanResult<()> {
        // Check if there's a pending simple key.
        if let Some(sk) = self.simple_keys.last().cloned() {
            if sk.possible {
                // Two distinct YAML 1.2.2 §7.4.2 rules conflated as
                // "implicit key" violations:
                //
                //   (rule 2)  the key itself spans a `\n` —
                //             `"c\n d": 1` (7LBH/D49Q) or
                //             `c\n d: 1` (G7JE). Error.
                //
                //   (rule 1)  the key ends on one line and `:` lands
                //             on the next — `&b b\n: *a` (6M2F). The
                //             key is *single-line* but the `:` is for
                //             a *different* (empty implicit) pair.
                //             Invalidate, fall through to else.
                //
                // The latest emitted token's source span is the
                // simple key's actual end; trimming its trailing
                // whitespace strips the `\n` the multi-line plain
                // scalar reader consumes during termination.
                // Clamp `key_end` to be at least `sk.index` — degenerate
                // streams (`:\n*\n…`) can leave the simple-key tracker
                // ahead of every emitted token's span end, in which
                // case `tokens.last().span.end < sk.index` and the
                // slice below would panic with "starts at X but ends
                // at Y (X > Y)". An empty slice is the correct content
                // for "no key seen yet" — the implicit-key-spans-newline
                // check below will see an empty buffer and fall through.
                let key_end = self
                    .tokens
                    .last()
                    .map(|t| t.span.end)
                    .unwrap_or(sk.index)
                    .max(sk.index);
                let key_end_trimmed = {
                    let mut e = key_end;
                    while e > sk.index
                        && matches!(
                            self.input.get(e - 1).copied(),
                            Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
                        )
                    {
                        e -= 1;
                    }
                    e
                };

                let key_content = &self.input[sk.index..key_end_trimmed];
                let key_has_newline = key_content.iter().any(|&b| b == b'\n' || b == b'\r');

                if key_has_newline && self.flow_level == 0 {
                    // When `?` introduced the key, the simple-key
                    // tracker is permitted to span newlines (the key
                    // is *explicit*, not implicit). Invalidate the
                    // tracker and fall through to the else branch so
                    // the `:` is emitted as the explicit-key value
                    // indicator (JTV5, M5DY). Without `?`, this is
                    // the genuine "implicit key spans newlines" error.
                    if self.explicit_key_pending {
                        if let Some(last) = self.simple_keys.last_mut() {
                            last.possible = false;
                        }
                        return Ok(());
                    }
                    return Err(self.error(
                        "implicit mapping key in block context cannot span multiple lines",
                    ));
                }

                let between_has_newline = self.input[key_end_trimmed..self.pos]
                    .iter()
                    .any(|&b| b == b'\n' || b == b'\r');

                if between_has_newline {
                    if self.flow_level == 0 {
                        // Rule 1: Key ends on line N, ':' on line N+1.
                        // Invalidate the simple key so it doesn't get retroactively
                        // converted to a mapping, but don't error (6M2F).
                        if let Some(last) = self.simple_keys.last_mut() {
                            last.possible = false;
                        }
                        return Ok(());
                    } else {
                        // In flow context, Rule 1 is generally allowed (4MUZ).
                        // BUT in a flow sequence (where we are looking for a
                        // single-pair mapping), the colon must be on the same
                        // line as the key (DK4H, ZXT5).
                        let in_flow_seq = self.flow_stack.last().copied().unwrap_or(false);

                        if in_flow_seq {
                            return Err(self.error("implicit mapping key in flow sequence must be on the same line as the colon"));
                        }

                        if key_has_newline
                            && (self.pos == 0
                                || (self.input[self.pos - 1] != b' '
                                    && self.input[self.pos - 1] != b'\t'))
                        {
                            return Err(self
                                .error("implicit mapping key cannot span multiple lines (flow)"));
                        }
                    }
                }
            }

            if sk.possible {
                // Insert Key token before the simple key.
                let idx = sk.token_number - self.tokens_produced;
                let span = Span {
                    start: sk.index,
                    end: sk.index,
                };
                self.insert_token(idx, TokenKind::Key, span);

                // Roll indent for block mapping.
                if self.flow_level == 0 {
                    self.reject_block_inline_with_doc_start("':'")?;
                    let line_start = self.input[..sk.index]
                        .iter()
                        .rposition(|&b| b == b'\n')
                        .map_or(0, |nl| nl + 1);
                    let leading = &self.input[line_start..sk.index];
                    // YAML 1.2.2 §6.1: block-mapping key indentation
                    // must be spaces only. A tab in the leading
                    // whitespace before a block-mapping key (DK95
                    // sub-case 7: `  \tb: 2`) is an indentation tab.
                    // Plain-scalar continuation (DK95 sub-case 1
                    // `\tbar` after `foo:`) is unaffected — no Key
                    // promotion happens for the continuation line.
                    if leading.contains(&b'\t') {
                        return Err(self.error(
                            "tab characters are not allowed in block-mapping key indentation",
                        ));
                    }
                    let col = (sk.index - line_start) as i32;
                    self.roll_indent(
                        col,
                        Some(sk.token_number),
                        TokenKind::BlockMappingStart,
                        sk.index,
                    );
                }

                if let Some(last) = self.simple_keys.last_mut() {
                    last.possible = false;
                }
                self.simple_key_allowed = false;
            } else {
                // No simple key. Must be a complex value indicator.
                if self.flow_level == 0 {
                    if !self.simple_key_allowed && !self.explicit_key_pending {
                        return Err(self.error("mapping values are not allowed in this context"));
                    }
                    let col = self.column() as i32;
                    self.roll_indent(col, None, TokenKind::BlockMappingStart, self.pos);
                }
                self.simple_key_allowed = self.flow_level == 0;
            }
        } else {
            // No simple key tracking. Must be a complex value.
            if self.flow_level == 0 && !self.simple_key_allowed && !self.explicit_key_pending {
                return Err(self.error("mapping values are not allowed in this context"));
            }
            if self.flow_level == 0 {
                let col = self.column() as i32;
                self.roll_indent(col, None, TokenKind::BlockMappingStart, self.pos);
            }
            self.simple_key_allowed = self.flow_level == 0;
        }

        self.explicit_key_pending = false;
        self.mark = self.pos;
        self.advance();
        let start_val = self.pos;
        self.skip_blank();
        if self.flow_level == 0 {
            self.reject_illegal_tab(start_val, "':'")?;
        }
        self.emit(TokenKind::Value);
        self.last_token_opens_block = true;
        Ok(())
    }

    fn fetch_anchor(&mut self) -> ScanResult<()> {
        if self.flow_level == 0 {
            self.check_pending_property_indent()?;
        }
        self.save_simple_key();
        self.simple_key_allowed = false;
        self.mark = self.pos;
        self.advance(); // skip '&'
        let name = self.scan_anchor_name()?;
        self.emit(TokenKind::Anchor(name));
        Ok(())
    }

    /// If a previous Anchor/Tag has not yet been followed by content
    /// and we are about to emit *another* node-property token at a
    /// column that is no greater than the current block indent, the
    /// new property is at parent's level — which YAML 1.2.2 §6.9.1
    /// disallows (a node and its properties must be indented strictly
    /// more than the surrounding block). Used by H7J7.
    fn check_pending_property_indent(&self) -> ScanResult<()> {
        if self.pending_property_col.is_some() && (self.column() as i32) <= self.indent {
            return Err(self.error(
                "node properties (anchor/tag) must be indented more than the parent block",
            ));
        }
        Ok(())
    }

    fn fetch_alias(&mut self) -> ScanResult<()> {
        // Per YAML 1.2.2 §7.1: aliases are complete references, so
        // node properties (anchors and tags) cannot decorate them
        // (SR86, SU74).
        if matches!(
            self.last_emitted_kind,
            LastEmitted::Anchor | LastEmitted::Tag
        ) {
            return Err(self.error(
                "alias cannot be decorated with an anchor or tag — aliases are complete references",
            ));
        }
        self.save_simple_key();
        self.simple_key_allowed = false;
        self.mark = self.pos;
        self.advance(); // skip '*'
        let name = self.scan_anchor_name()?;
        self.emit(TokenKind::Alias(name));
        // An alias is a complete node — close any pending block-open
        // intent so the next dispatch is checked against the indent.
        self.last_token_opens_block = false;
        Ok(())
    }

    fn scan_anchor_name(&mut self) -> ScanResult<Cow<'a, str>> {
        /// Maximum length of an anchor or alias name (in bytes).
        const MAX_ANCHOR_NAME_LEN: usize = 1024;

        let start = self.pos;
        while !self.is_eof() {
            if self.pos - start > MAX_ANCHOR_NAME_LEN {
                return Err(self.error("anchor name exceeds maximum length of 1024 bytes"));
            }
            let c = self.peek();
            // Per YAML 1.2.2 §6.9.2: ns-anchor-char = ns-char - c-flow-indicator.
            // Terminators are whitespace and flow indicators only. `:` is part
            // of the anchor name; structural ambiguity with value separators
            // is resolved by requiring whitespace before the separator.
            if Self::is_blank_or_break(c)
                || c == b','
                || c == b'['
                || c == b']'
                || c == b'{'
                || c == b'}'
            {
                break;
            }
            self.advance();
        }
        if self.pos == start {
            return Err(self.error("expected anchor or alias name"));
        }
        Ok(Cow::Borrowed(self.slice_str(start, self.pos)))
    }

    fn fetch_tag(&mut self) -> ScanResult<()> {
        /// Maximum length of a tag URI (in bytes).
        const MAX_TAG_LEN: usize = 1024;

        if self.flow_level == 0 {
            self.check_pending_property_indent()?;
        }

        self.save_simple_key();
        self.simple_key_allowed = false;
        self.mark = self.pos;
        self.advance(); // skip '!'

        let handle: Cow<'a, str>;
        let suffix: Cow<'a, str>;

        if self.peek() == b'<' {
            // Verbatim tag: !<...>
            handle = Cow::Borrowed("!");
            self.advance(); // skip '<'
            let start = self.pos;
            while !self.is_eof() && self.peek() != b'>' {
                if self.pos - start > MAX_TAG_LEN {
                    return Err(self.error("tag URI exceeds maximum length of 1024 bytes"));
                }
                self.advance();
            }
            suffix = Cow::Borrowed(self.slice_str(start, self.pos));
            if self.peek() == b'>' {
                self.advance();
            }
        } else if self.peek() == b'!' {
            // Secondary tag handle `!!suffix`.
            handle = Cow::Borrowed("!!");
            self.advance();
            let start = self.pos;
            while !self.is_eof()
                && !Self::is_blank_or_break(self.peek())
                && self.peek() != b','
                && self.peek() != b'['
                && self.peek() != b']'
                && self.peek() != b'{'
                && self.peek() != b'}'
            {
                if self.pos - start > MAX_TAG_LEN {
                    return Err(self.error("tag suffix exceeds maximum length of 1024 bytes"));
                }
                self.advance();
            }
            suffix = Cow::Borrowed(self.slice_str(start, self.pos));
        } else {
            // Primary `!suffix` OR named `!handle!suffix`. The two
            // forms are distinguished by whether a second `!` appears
            // before the next separator: scan up to that separator, and
            // if a `!` was crossed, treat the bytes up to and including
            // it as the handle.
            let start = self.pos;
            let mut second_bang: Option<usize> = None;
            while !self.is_eof()
                && !Self::is_blank_or_break(self.peek())
                && self.peek() != b','
                && self.peek() != b'['
                && self.peek() != b']'
                && self.peek() != b'{'
                && self.peek() != b'}'
            {
                if self.peek() == b'!' && second_bang.is_none() {
                    second_bang = Some(self.pos);
                }
                if self.pos - start > MAX_TAG_LEN {
                    return Err(self.error("tag suffix exceeds maximum length of 1024 bytes"));
                }
                self.advance();
            }
            if let Some(bang_pos) = second_bang {
                // Named handle: `!handle!suffix` — handle includes the
                // second `!`, suffix is what follows it.
                handle = Cow::Owned(format!("!{}!", self.slice_str(start, bang_pos)));
                suffix = Cow::Borrowed(self.slice_str(bang_pos + 1, self.pos));
            } else {
                handle = Cow::Borrowed("!");
                suffix = Cow::Borrowed(self.slice_str(start, self.pos));
            }
        }

        // Per YAML 1.2.2 §6.9.1, a tag URI is followed by separation
        // (whitespace or a line break) before the next node. The
        // collection openers `{` and `[` packed directly against the
        // tag are always malformed — `!invalid{}tag` (LHL4) — because
        // they would have to start a *nested* node and the tag has no
        // separator.
        //
        // The terminators `,` / `]` / `}` are different: in flow
        // context they validly mark the end of a tagged *empty* scalar
        // (`!!str,` and `!!str]`/`!!str}` — WZ62). Only treat them as
        // malformed outside flow.
        let next = self.peek();
        let is_malformed = matches!(next, b'{' | b'[')
            || (self.flow_level == 0 && matches!(next, b'}' | b']' | b','));
        if is_malformed {
            return Err(self.error(
                "tag must be followed by whitespace or a line break, not a flow indicator",
            ));
        }

        // Apply any active `%TAG handle prefix` directive: replace
        // the handle with the declared URI prefix so the loader sees
        // the resolved tag (P76L spec example 6.19). Default handles
        // `!` and `!!` are always available; *named* handles
        // (`!foo!`) must be declared in a `%TAG` directive for the
        // current document — otherwise reject (QLJ7 spec example 6.21).
        let resolved_handle = if let Some(prefix) = self.tag_handles.get(handle.as_ref()) {
            Cow::Owned(prefix.clone())
        } else {
            let h: &str = handle.as_ref();
            // A *named* handle has the form `!foo!` — at least one
            // character between the two bangs. The primary `!` and
            // the secondary `!!` are not "named" — they have implicit
            // default URI prefixes per YAML 1.2.2 §6.8.2.
            let is_named = h.len() > 2 && h.starts_with('!') && h.ends_with('!');
            if is_named {
                return Err(self.error(
                    "named tag handle is not declared by a %TAG directive in this document",
                ));
            }
            handle
        };

        self.emit(TokenKind::Tag(resolved_handle, suffix));
        Ok(())
    }

    // ── Scalars ──────────────────────────────────────────────────────────

    fn fetch_plain_scalar(&mut self) -> ScanResult<()> {
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

    fn fetch_quoted_scalar(&mut self, double: bool) -> ScanResult<()> {
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

    fn fetch_block_scalar(&mut self, literal: bool) -> ScanResult<()> {
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

/// Snap `index` to the nearest code-point boundary at or below it.
///
/// Stable polyfill for `str::floor_char_boundary`, which is
/// nightly-only as of Rust 1.94. Used by [`Scanner::slice_str`] to
/// keep the panic-free contract under defective upstream advance
/// patterns. Defensive — on well-formed input this is a no-op.
fn floor_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(index) {
        index = index.saturating_sub(1);
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_two_key_mapping() {
        let input = "name: test\nitem: Unit\n";
        let mut scanner = Scanner::new(input);
        let mut tokens = Vec::new();
        loop {
            let result = scanner.next_token();
            match result {
                Ok(t) => {
                    let is_end = matches!(t.kind, TokenKind::StreamEnd);
                    tokens.push(format!("{:?}", t.kind));
                    if is_end {
                        break;
                    }
                }
                Err(e) => {
                    panic!("Scanner error after tokens {tokens:#?}: {e}");
                }
            }
        }
        // Expected: StreamStart, BlockMappingStart, Key, Scalar(name),
        //           Value, Scalar(test), Key, Scalar(item), Value, Scalar(Unit),
        //           BlockEnd, StreamEnd
        let expected_contains = vec![
            "StreamStart",
            "BlockMappingStart",
            "Key",
            "Scalar(Plain, \"name\")",
            "Value",
            "Scalar(Plain, \"test\")",
            "Key",
            "Scalar(Plain, \"item\")",
            "Value",
            "Scalar(Plain, \"Unit\")",
            "BlockEnd",
            "StreamEnd",
        ];
        for (i, exp) in expected_contains.iter().enumerate() {
            assert!(
                i < tokens.len() && tokens[i].contains(exp),
                "Token {i}: expected to contain {exp:?}, got {:?}\nAll tokens: {tokens:#?}",
                tokens.get(i)
            );
        }
    }
}
