//! YAML 1.2 event-based parser.
//!
//! Converts a stream of [`Token`]s into parsing [`Event`]s.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::IndexMap;
use crate::prelude::*;

use super::scanner::{ScalarStyle, ScanError, Scanner, Span, TokenKind};

/// Parsing events emitted by the parser.
///
/// The lifetime `'a` allows `Scalar::value` to borrow directly from the input
/// when no escaping or line-folding was needed (plain scalar fast path).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum Event<'a> {
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Alias {
        anchor: String,
        span: Span,
    },
    Scalar {
        value: Cow<'a, str>,
        style: ScalarStyle,
        anchor: Option<String>,
        tag: Option<(String, String)>,
        span: Span,
    },
    SequenceStart {
        anchor: Option<String>,
        tag: Option<(String, String)>,
        span: Span,
    },
    SequenceEnd {
        span: Span,
    },
    MappingStart {
        anchor: Option<String>,
        tag: Option<(String, String)>,
        span: Span,
    },
    MappingEnd {
        span: Span,
    },
}

/// Parser states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    StreamStart,
    ImplicitDocumentStart,
    DocumentStart,
    DocumentContent,
    DocumentEnd,
    BlockNode,
    BlockSequenceFirstEntry,
    BlockSequenceEntry,
    IndentlessSequenceEntry,
    BlockMappingFirstKey,
    BlockMappingKey,
    BlockMappingValue,
    FlowSequenceFirstEntry,
    FlowSequenceEntry,
    FlowSequenceEntryMappingKey,
    FlowSequenceEntryMappingValue,
    FlowSequenceEntryMappingEnd,
    FlowMappingFirstKey,
    FlowMappingKey,
    FlowMappingValue,
    FlowMappingEmptyValue,
    End,
}

/// YAML event-based parser.
#[derive(Debug)]
pub(crate) struct Parser<'a> {
    scanner: Scanner<'a>,
    /// Parser state stack. Each `State` is a 1-byte enum; 16 slots inline
    /// is plenty for realistic nesting and costs just 16 bytes of stack,
    /// avoiding a heap allocation per parse.
    states: smallvec::SmallVec<[State; 16]>,
    state: State,
    /// Current peeked token kind + span (if any).
    current_kind: Option<TokenKind<'a>>,
    current_span: Span,
    has_current: bool,
    /// Anchor name registry.
    marks: IndexMap<String, usize>,
    next_anchor_id: usize,
    /// `true` once the first document in the stream has ended. Per
    /// YAML 1.2.2 §9.1.2, implicit documents are only allowed as the
    /// first document; any subsequent document must start with an
    /// explicit `---` (or follow `...`). Used to reject stray content
    /// at the end of a document (BS4K, KS4U-class).
    first_document_ended: bool,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Parser {
            scanner: Scanner::new(input),
            states: smallvec::SmallVec::new(),
            state: State::StreamStart,
            current_kind: None,
            current_span: Span::default(),
            has_current: false,
            marks: IndexMap::default(),
            next_anchor_id: 0,
            first_document_ended: false,
        }
    }

    /// Drain captured comments. Used by the public
    /// [`crate::load_comments`] API. Returns comments in source order.
    pub(crate) fn take_comments(&mut self) -> Vec<crate::comments::Comment> {
        self.scanner
            .take_comments()
            .into_iter()
            .map(|c| crate::comments::Comment {
                text: c.text,
                start: c.start,
                end: c.end,
                kind: if c.inline {
                    crate::comments::CommentKind::Inline
                } else {
                    crate::comments::CommentKind::Line
                },
            })
            .collect()
    }

    pub(crate) fn next_event(&mut self) -> Result<Event<'a>, ScanError> {
        match self.state {
            State::StreamStart => self.parse_stream_start(),
            State::ImplicitDocumentStart => self.parse_document_start(true),
            State::DocumentStart => self.parse_document_start(false),
            State::DocumentContent => self.parse_document_content(),
            State::DocumentEnd => self.parse_document_end(),
            State::BlockNode => self.parse_node(true, false),
            State::BlockSequenceFirstEntry => self.parse_block_sequence_entry(true),
            State::BlockSequenceEntry => self.parse_block_sequence_entry(false),
            State::IndentlessSequenceEntry => self.parse_indentless_sequence_entry(),
            State::BlockMappingFirstKey => self.parse_block_mapping_key(true),
            State::BlockMappingKey => self.parse_block_mapping_key(false),
            State::BlockMappingValue => self.parse_block_mapping_value(),
            State::FlowSequenceFirstEntry => self.parse_flow_sequence_entry(true),
            State::FlowSequenceEntry => self.parse_flow_sequence_entry(false),
            State::FlowSequenceEntryMappingKey => self.parse_flow_sequence_entry_mapping_key(),
            State::FlowSequenceEntryMappingValue => self.parse_flow_sequence_entry_mapping_value(),
            State::FlowSequenceEntryMappingEnd => self.parse_flow_sequence_entry_mapping_end(),
            State::FlowMappingFirstKey => self.parse_flow_mapping_key(true),
            State::FlowMappingKey => self.parse_flow_mapping_key(false),
            State::FlowMappingValue => self.parse_flow_mapping_value(false),
            State::FlowMappingEmptyValue => self.parse_flow_mapping_value(true),
            State::End => Err(ScanError {
                message: Cow::Borrowed("parser has already finished"),
                index: 0,
            }),
        }
    }

    /// Ensure the current token is buffered and return its kind + span.
    fn peek(&mut self) -> Result<(&TokenKind<'a>, Span), ScanError> {
        if !self.has_current {
            let t = self.scanner.next_token()?;
            self.current_kind = Some(t.kind);
            self.current_span = t.span;
            self.has_current = true;
        }
        Ok((
            self.current_kind
                .as_ref()
                .expect("internal: current_kind set by peek"),
            self.current_span,
        ))
    }

    /// Peek just the kind (for matching).
    ///
    /// NOTE: This clones the `TokenKind`, including any owned `String` in
    /// `Scalar`/`Anchor`/`Alias`/`Tag` variants. Prefer `peek_is()` or
    /// `take()` when you only need the discriminant or will consume the token.
    fn peek_kind(&mut self) -> Result<TokenKind<'a>, ScanError> {
        let (kind, _) = self.peek()?;
        Ok(kind.clone())
    }

    /// Check whether the peeked token matches a discriminant without cloning.
    #[inline]
    fn peek_is(&mut self, f: fn(&TokenKind<'_>) -> bool) -> Result<bool, ScanError> {
        let (kind, _) = self.peek()?;
        Ok(f(kind))
    }

    /// Consume the current token and return its kind + span.
    fn take(&mut self) -> Result<(TokenKind<'a>, Span), ScanError> {
        if self.has_current {
            self.has_current = false;
            Ok((
                self.current_kind
                    .take()
                    .expect("internal: current_kind set when has_current"),
                self.current_span,
            ))
        } else {
            let t = self.scanner.next_token()?;
            Ok((t.kind, t.span))
        }
    }

    /// Consume the current token, discarding it.
    fn skip(&mut self) -> Result<(), ScanError> {
        let _ = self.take()?;
        Ok(())
    }

    fn pop_state(&mut self) -> State {
        self.states.pop().unwrap_or(State::End)
    }

    fn empty_scalar(&self, span: Span) -> Event<'a> {
        Event::Scalar {
            value: Cow::Borrowed(""),
            style: ScalarStyle::Plain,
            anchor: None,
            tag: None,
            span,
        }
    }

    // ── State handlers ───────────────────────────────────────────────────

    fn parse_stream_start(&mut self) -> Result<Event<'a>, ScanError> {
        self.skip()?; // StreamStart
        self.state = State::ImplicitDocumentStart;
        Ok(Event::StreamStart)
    }

    fn parse_document_start(&mut self, implicit: bool) -> Result<Event<'a>, ScanError> {
        // Skip any document end markers (`...`).
        while self.peek_is(|k| matches!(k, TokenKind::DocumentEnd))? {
            self.skip()?;
        }

        if self.peek_is(|k| matches!(k, TokenKind::StreamEnd))? {
            self.skip()?;
            self.state = State::End;
            return Ok(Event::StreamEnd);
        }

        if self.peek_is(|k| matches!(k, TokenKind::DocumentStart))? {
            self.skip()?;
            self.state = State::DocumentContent;
            return Ok(Event::DocumentStart);
        }

        // YAML 1.2.2 §9.1.2: subsequent documents must begin with an
        // explicit `---`. Reaching this point with `first_document_ended`
        // means there is content after the first document but neither
        // a `---` indicator nor end-of-stream — that is stray and
        // invalid (BS4K, KS4U).
        if self.first_document_ended {
            let span = self.current_span;
            return Err(ScanError {
                message: Cow::Borrowed(
                    "stray content after document — subsequent documents must start with '---'",
                ),
                index: span.start,
            });
        }

        if implicit {
            self.state = State::BlockNode;
            self.states.push(State::DocumentEnd);
        } else {
            self.state = State::DocumentContent;
        }
        Ok(Event::DocumentStart)
    }

    fn parse_document_content(&mut self) -> Result<Event<'a>, ScanError> {
        let _ = self.peek()?;
        let span = self.current_span;
        if self.peek_is(|k| {
            matches!(
                k,
                TokenKind::DocumentEnd | TokenKind::DocumentStart | TokenKind::StreamEnd
            )
        })? {
            self.state = State::DocumentEnd;
            Ok(self.empty_scalar(span))
        } else {
            self.states.push(State::DocumentEnd);
            self.parse_node(true, false)
        }
    }

    fn parse_document_end(&mut self) -> Result<Event<'a>, ScanError> {
        let saw_explicit_end = self.peek_is(|k| matches!(k, TokenKind::DocumentEnd))?;
        if saw_explicit_end {
            self.skip()?;
        }
        self.marks.clear();
        self.next_anchor_id = 0;
        self.state = State::DocumentStart;
        // Only mark the "subsequent docs must be explicit" flag when
        // the current document ended *implicitly* (no `...`). A
        // bare implicit document after `...` is allowed (7Z25); only
        // stray content with no boundary at all is invalid (BS4K,
        // KS4U).
        if !saw_explicit_end {
            self.first_document_ended = true;
        }
        Ok(Event::DocumentEnd)
    }

    fn parse_node(&mut self, block: bool, indentless: bool) -> Result<Event<'a>, ScanError> {
        // Peek once to get the span; use take() to extract owned data only
        // when the token is actually consumed — avoids cloning Strings.
        let _ = self.peek()?;
        let span = self.current_span;

        let mut anchor: Option<String> = None;
        let mut tag: Option<(String, String)> = None;

        // Parse optional anchor — take() to move the Cow out; convert to owned
        // for the Event boundary.
        if self.peek_is(|k| matches!(k, TokenKind::Anchor(_)))? {
            if let (TokenKind::Anchor(name), _) = self.take()? {
                let owned = name.into_owned();
                let _ = self.marks.insert(owned.clone(), self.next_anchor_id);
                self.next_anchor_id += 1;
                anchor = Some(owned);
            }
            // Check for tag after anchor.
            if self.peek_is(|k| matches!(k, TokenKind::Tag(_, _)))? {
                if let (TokenKind::Tag(h, s), _) = self.take()? {
                    tag = Some((h.into_owned(), s.into_owned()));
                }
            }
        } else if self.peek_is(|k| matches!(k, TokenKind::Tag(_, _)))? {
            if let (TokenKind::Tag(h, s), _) = self.take()? {
                tag = Some((h.into_owned(), s.into_owned()));
            }
            // Check for anchor after tag.
            if self.peek_is(|k| matches!(k, TokenKind::Anchor(_)))? {
                if let (TokenKind::Anchor(name), _) = self.take()? {
                    let owned = name.into_owned();
                    let _ = self.marks.insert(owned.clone(), self.next_anchor_id);
                    self.next_anchor_id += 1;
                    anchor = Some(owned);
                }
            }
        }

        // A node's span includes its properties: `span` above is the
        // first token of the node, which is the anchor/tag token when
        // properties are present — so widening the emitted span back
        // to `span.start` anchors Spanned<T> and error locations at
        // the `!tag`/`&anchor`, matching serde_yaml/libyaml marks
        // (the serde_yaml contract's custom-explicit-tag case pins
        // this at 1:8:7).
        let props_start = if anchor.is_some() || tag.is_some() {
            Some(span.start)
        } else {
            None
        };
        let widen = |s: Span| match props_start {
            Some(p) if p < s.start => Span {
                start: p,
                end: s.end,
            },
            _ => s,
        };

        // Alias — take() moves the Cow; convert to owned for the Event.
        if self.peek_is(|k| matches!(k, TokenKind::Alias(_)))? {
            let (kind, alias_span) = self.take()?;
            if let TokenKind::Alias(name) = kind {
                self.state = self.pop_state();
                return Ok(Event::Alias {
                    anchor: name.into_owned(),
                    span: alias_span,
                });
            }
        }

        // Main node dispatch — take() for Scalar to move the String.
        let _ = self.peek()?;
        let tok_span = self.current_span;
        let kind_ref = self
            .current_kind
            .as_ref()
            .expect("internal: peek() above guarantees current_kind");

        match kind_ref {
            TokenKind::Scalar(_, _) => {
                let (kind, scalar_span) = self.take()?;
                if let TokenKind::Scalar(style, value) = kind {
                    self.state = self.pop_state();
                    Ok(Event::Scalar {
                        value,
                        style,
                        anchor,
                        tag,
                        span: widen(scalar_span),
                    })
                } else {
                    crate::error::invariant_violated(
                        "outer match guarded TokenKind::Scalar; take() must return the same",
                    )
                }
            }
            TokenKind::FlowSequenceStart => {
                self.skip()?;
                self.state = State::FlowSequenceFirstEntry;
                Ok(Event::SequenceStart {
                    anchor,
                    tag,
                    span: widen(tok_span),
                })
            }
            TokenKind::FlowMappingStart => {
                self.skip()?;
                self.state = State::FlowMappingFirstKey;
                Ok(Event::MappingStart {
                    anchor,
                    tag,
                    span: widen(tok_span),
                })
            }
            TokenKind::BlockSequenceStart if block => {
                self.skip()?;
                self.state = State::BlockSequenceFirstEntry;
                Ok(Event::SequenceStart {
                    anchor,
                    tag,
                    span: widen(tok_span),
                })
            }
            TokenKind::BlockMappingStart if block => {
                self.skip()?;
                self.state = State::BlockMappingFirstKey;
                Ok(Event::MappingStart {
                    anchor,
                    tag,
                    span: widen(tok_span),
                })
            }
            // Indentless block sequence: `BlockEntry` without a preceding
            // `BlockSequenceStart` — the `-` is at the same indent as the
            // containing mapping key.
            TokenKind::BlockEntry if indentless || (anchor.is_some() || tag.is_some()) => {
                self.state = State::IndentlessSequenceEntry;
                Ok(Event::SequenceStart {
                    anchor,
                    tag,
                    span: widen(tok_span),
                })
            }
            _ => {
                if anchor.is_some() || tag.is_some() {
                    self.state = self.pop_state();
                    Ok(Event::Scalar {
                        value: Cow::Borrowed(""),
                        style: ScalarStyle::Plain,
                        anchor,
                        tag,
                        span,
                    })
                } else if indentless {
                    self.state = self.pop_state();
                    Ok(self.empty_scalar(span))
                } else {
                    let kind = self.peek_kind()?;
                    Err(ScanError {
                        message: Cow::Owned(format!("expected a node but found {kind:?}")),
                        index: span.start,
                    })
                }
            }
        }
    }

    // ── Block sequences ──────────────────────────────────────────────────

    fn parse_block_sequence_entry(&mut self, _first: bool) -> Result<Event<'a>, ScanError> {
        let _ = self.peek()?;
        let span = self.current_span;

        if self.peek_is(|k| matches!(k, TokenKind::BlockEntry))? {
            self.skip()?;
            if self.peek_is(|k| matches!(k, TokenKind::BlockEntry | TokenKind::BlockEnd))? {
                self.state = State::BlockSequenceEntry;
                Ok(self.empty_scalar(span))
            } else {
                self.states.push(State::BlockSequenceEntry);
                self.parse_node(true, false)
            }
        } else if self.peek_is(|k| matches!(k, TokenKind::BlockEnd))? {
            self.skip()?;
            self.state = self.pop_state();
            Ok(Event::SequenceEnd { span })
        } else {
            Err(ScanError {
                message: Cow::Borrowed("expected block sequence entry or end"),
                index: span.start,
            })
        }
    }

    fn parse_indentless_sequence_entry(&mut self) -> Result<Event<'a>, ScanError> {
        let _ = self.peek()?;
        let span = self.current_span;

        if self.peek_is(|k| matches!(k, TokenKind::BlockEntry))? {
            self.skip()?;
            if self.peek_is(|k| {
                matches!(
                    k,
                    TokenKind::BlockEntry | TokenKind::Key | TokenKind::Value | TokenKind::BlockEnd
                )
            })? {
                self.state = State::IndentlessSequenceEntry;
                Ok(self.empty_scalar(span))
            } else {
                self.states.push(State::IndentlessSequenceEntry);
                self.parse_node(true, false)
            }
        } else {
            self.state = self.pop_state();
            Ok(Event::SequenceEnd { span })
        }
    }

    // ── Block mappings ───────────────────────────────────────────────────

    fn parse_block_mapping_key(&mut self, _first: bool) -> Result<Event<'a>, ScanError> {
        let _ = self.peek()?;
        let span = self.current_span;

        if self.peek_is(|k| matches!(k, TokenKind::Key))? {
            self.skip()?;
            if self
                .peek_is(|k| matches!(k, TokenKind::Key | TokenKind::Value | TokenKind::BlockEnd))?
            {
                self.state = State::BlockMappingValue;
                Ok(self.empty_scalar(span))
            } else {
                self.states.push(State::BlockMappingValue);
                self.parse_node(true, true)
            }
        } else if self.peek_is(|k| matches!(k, TokenKind::BlockEnd))? {
            self.skip()?;
            self.state = self.pop_state();
            Ok(Event::MappingEnd { span })
        } else if self.peek_is(|k| matches!(k, TokenKind::Value))? {
            // Bare `:` without `?` — implicit empty key.  Emit the empty key
            // scalar and transition to the value phase (which will consume `:`)
            self.state = State::BlockMappingValue;
            Ok(self.empty_scalar(span))
        } else if self.peek_is(|k| {
            matches!(
                k,
                TokenKind::BlockSequenceStart
                    | TokenKind::BlockEntry
                    | TokenKind::BlockMappingStart
            )
        })? {
            // Compact block collection as mapping value at the same indent level.
            // Treat as if we saw an implicit empty key followed by this value.
            // This handles patterns like: `key:\n- item` where `-` is at the same indent.
            self.state = self.pop_state();
            Ok(Event::MappingEnd { span })
        } else {
            Err(ScanError {
                message: Cow::Borrowed("expected block mapping key or end"),
                index: span.start,
            })
        }
    }

    fn parse_block_mapping_value(&mut self) -> Result<Event<'a>, ScanError> {
        let _ = self.peek()?;
        let span = self.current_span;

        if self.peek_is(|k| matches!(k, TokenKind::Value))? {
            self.skip()?;
            if self
                .peek_is(|k| matches!(k, TokenKind::Key | TokenKind::Value | TokenKind::BlockEnd))?
            {
                self.state = State::BlockMappingKey;
                Ok(self.empty_scalar(span))
            } else {
                self.states.push(State::BlockMappingKey);
                self.parse_node(true, true)
            }
        } else {
            self.state = State::BlockMappingKey;
            Ok(self.empty_scalar(span))
        }
    }

    // ── Flow sequences ───────────────────────────────────────────────────

    fn parse_flow_sequence_entry(&mut self, first: bool) -> Result<Event<'a>, ScanError> {
        if !first {
            let _ = self.peek()?;
            let span = self.current_span;
            if self.peek_is(|k| matches!(k, TokenKind::FlowEntry))? {
                self.skip()?;
            } else if !self.peek_is(|k| matches!(k, TokenKind::FlowSequenceEnd))? {
                return Err(ScanError {
                    message: Cow::Borrowed("expected ',' or ']' in flow sequence"),
                    index: span.start,
                });
            }
        }

        let _ = self.peek()?;
        let span = self.current_span;

        if self.peek_is(|k| matches!(k, TokenKind::FlowSequenceEnd))? {
            self.skip()?;
            self.state = self.pop_state();
            return Ok(Event::SequenceEnd { span });
        }

        if self.peek_is(|k| matches!(k, TokenKind::Key))? {
            self.skip()?;
            self.state = State::FlowSequenceEntryMappingKey;
            self.states.push(State::FlowSequenceEntry);
            return Ok(Event::MappingStart {
                anchor: None,
                tag: None,
                span,
            });
        }

        // A bare `Value` (`:`) without a preceding `Key` means an implicit
        // empty-key mapping pair, e.g. `[ : value ]` (CFD4). Start a
        // mapping and route through the *Key* phase so the empty key is
        // emitted as a scalar event before the value — otherwise the
        // loader sees a MappingStart followed directly by the value
        // and treats the value as the key (which then has no value),
        // producing "unexpected mapping end".
        if self.peek_is(|k| matches!(k, TokenKind::Value))? {
            self.state = State::FlowSequenceEntryMappingKey;
            self.states.push(State::FlowSequenceEntry);
            return Ok(Event::MappingStart {
                anchor: None,
                tag: None,
                span,
            });
        }

        self.states.push(State::FlowSequenceEntry);
        self.parse_node(false, false)
    }

    fn parse_flow_sequence_entry_mapping_key(&mut self) -> Result<Event<'a>, ScanError> {
        let _ = self.peek()?;
        let span = self.current_span;

        if self.peek_is(|k| {
            matches!(
                k,
                TokenKind::Value | TokenKind::FlowEntry | TokenKind::FlowSequenceEnd
            )
        })? {
            self.state = State::FlowSequenceEntryMappingValue;
            Ok(self.empty_scalar(span))
        } else {
            self.states.push(State::FlowSequenceEntryMappingValue);
            self.parse_node(false, false)
        }
    }

    fn parse_flow_sequence_entry_mapping_value(&mut self) -> Result<Event<'a>, ScanError> {
        let _ = self.peek()?;
        let span = self.current_span;

        if self.peek_is(|k| matches!(k, TokenKind::Value))? {
            self.skip()?;
            if !self.peek_is(|k| matches!(k, TokenKind::FlowEntry | TokenKind::FlowSequenceEnd))? {
                self.states.push(State::FlowSequenceEntryMappingEnd);
                return self.parse_node(false, false);
            }
        }

        self.state = State::FlowSequenceEntryMappingEnd;
        Ok(self.empty_scalar(span))
    }

    fn parse_flow_sequence_entry_mapping_end(&mut self) -> Result<Event<'a>, ScanError> {
        let _ = self.peek()?;
        let span = self.current_span;
        self.state = self.pop_state();
        Ok(Event::MappingEnd { span })
    }

    // ── Flow mappings ────────────────────────────────────────────────────

    fn parse_flow_mapping_key(&mut self, first: bool) -> Result<Event<'a>, ScanError> {
        if !first {
            let _ = self.peek()?;
            let span = self.current_span;
            if self.peek_is(|k| matches!(k, TokenKind::FlowEntry))? {
                self.skip()?;
            } else if !self.peek_is(|k| matches!(k, TokenKind::FlowMappingEnd))? {
                return Err(ScanError {
                    message: Cow::Borrowed("expected ',' or '}' in flow mapping"),
                    index: span.start,
                });
            }
        }

        let _ = self.peek()?;
        let span = self.current_span;

        if self.peek_is(|k| matches!(k, TokenKind::FlowMappingEnd))? {
            self.skip()?;
            self.state = self.pop_state();
            return Ok(Event::MappingEnd { span });
        }

        if self.peek_is(|k| matches!(k, TokenKind::Key))? {
            self.skip()?;
            let _ = self.peek()?;
            let next_span = self.current_span;
            if !self.peek_is(|k| {
                matches!(
                    k,
                    TokenKind::Value | TokenKind::FlowEntry | TokenKind::FlowMappingEnd
                )
            })? {
                self.states.push(State::FlowMappingValue);
                return self.parse_node(false, false);
            }
            self.state = State::FlowMappingValue;
            return Ok(self.empty_scalar(next_span));
        }

        // A bare `Value` (`:`) without a preceding `Key` means the key is
        // empty, e.g. `{ : bar }`.  Emit an empty scalar for the key and
        // proceed directly to the value phase.
        if self.peek_is(|k| matches!(k, TokenKind::Value))? {
            self.state = State::FlowMappingValue;
            return Ok(self.empty_scalar(span));
        }

        // Implicit key.
        self.states.push(State::FlowMappingEmptyValue);
        self.parse_node(false, false)
    }

    fn parse_flow_mapping_value(&mut self, empty: bool) -> Result<Event<'a>, ScanError> {
        let _ = self.peek()?;
        let span = self.current_span;

        if empty {
            self.state = State::FlowMappingKey;
            return Ok(self.empty_scalar(span));
        }

        if self.peek_is(|k| matches!(k, TokenKind::Value))? {
            self.skip()?;
            if !self.peek_is(|k| matches!(k, TokenKind::FlowEntry | TokenKind::FlowMappingEnd))? {
                self.states.push(State::FlowMappingKey);
                return self.parse_node(false, false);
            }
        }

        self.state = State::FlowMappingKey;
        Ok(self.empty_scalar(span))
    }
}
