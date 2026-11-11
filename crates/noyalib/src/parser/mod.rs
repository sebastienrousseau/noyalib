//! Native YAML 1.2 parser.
//!
//! This module provides a complete YAML 1.2 Core Schema parser that builds
//! `Value` trees directly, with full control over security limits, duplicate
//! key handling, and alias expansion tracking.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

pub(crate) mod budget;
mod events;
mod loader;
pub(crate) mod meter;
mod scanner;

pub(crate) use events::{Event, Parser};
pub(crate) use loader::{
    DuplicateKeyPolicy as InternalDuplicateKeyPolicy, MergeKeyPolicy as InternalMergeKeyPolicy,
    ParseConfig, parse_legacy_binary, value_to_key_string,
};
pub(crate) use scanner::{ScalarStyle, Scanner, TokenKind};
// CST builder is the only consumer; gate the re-exports to match.
pub(crate) use scanner::{RecordedToken, RecordedTokenKind, ScannedComment, Trivia, TriviaKind};

/// Returns a default (zero) `Span` for use in synthesized events.
pub(crate) fn scanner_span_default() -> scanner::Span {
    scanner::Span::default()
}

/// Decode a plain scalar's spelling without resolving its schema type.
/// Properties can prefix a scalar's span; quoted and block scalars keep
/// their existing decoded `Value::String` behavior in the deserializer.
pub(crate) fn plain_scalar_text(input: &str) -> Option<Cow<'_, str>> {
    let mut scanner = Scanner::new(input);
    loop {
        match scanner.next_token().ok()?.kind {
            TokenKind::StreamStart | TokenKind::Anchor(_) => {}
            TokenKind::Tag(handle, suffix)
                if handle.as_ref() == "!" && suffix.as_ref() == "include" =>
            {
                return None;
            }
            TokenKind::Tag(_, _) => {}
            TokenKind::Scalar(ScalarStyle::Plain, text) => return Some(text),
            _ => return None,
        }
    }
}

use crate::error::Result;
// `prelude::*` brings `Vec` into scope on no_std builds; on std it
// duplicates the std prelude. The `#[allow]` suppresses the
// unused-import warning that fires only under std + the strict
// `-D unused` workspace lint.
#[allow(unused_imports)]
use crate::prelude::*;
use crate::span_context::SpanTree;
use crate::value::Value;

/// Parse a YAML string into a list of `(Value, SpanTree)` documents.
pub(crate) fn parse(input: &str, config: &ParseConfig) -> Result<Vec<(Value, SpanTree)>> {
    let mut parser = Parser::with_max_events(input, config.max_events);
    loader::load(&mut parser, config, input)
}

/// Parse exactly one YAML document, erroring if the stream carries
/// more than one. Used by single-document deserialisation and CST
/// entry points (see #351).
pub(crate) fn parse_exactly_one(input: &str, config: &ParseConfig) -> Result<(Value, SpanTree)> {
    let mut parser = Parser::with_max_events(input, config.max_events);
    loader::load_exactly_one(&mut parser, config, input)
}

/// Parse a single YAML document into a `Value` without building a
/// `SpanTree`, silently discarding any document past the first.
///
/// The deserialise entry points (`from_str`, `from_str_with_config`)
/// use the checked [`parse_exactly_one_value`] instead; this
/// unchecked form's sole remaining caller is
/// `cst::document::decode_key_token`, which only ever feeds it a
/// single scalar token — `std`-only, like the rest of the `cst`
/// module. See [`loader::load_one_no_spans`].
pub(crate) fn parse_one_value(input: &str, config: &ParseConfig) -> Result<Value> {
    loader::load_one_no_spans(input, config)
}

/// Parse a single YAML document into a `Value` without building a
/// `SpanTree`, erroring if the stream carries more than one document.
///
/// Available on every target — `std` builds use this from
/// `from_str::<Value>`'s fast path (`Value` has no span field, so a
/// `SpanTree` would be pure waste) and from the `no_std` AST path;
/// `no_std` builds use it unless typed plain-scalar strings need their
/// source spans. See #351 and #494.
pub(crate) fn parse_exactly_one_value(input: &str, config: &ParseConfig) -> Result<Value> {
    loader::load_exactly_one_no_spans(input, config)
}

/// Parse all YAML documents into `Value`s without building `SpanTree`s.
///
/// Used by the ordinary no_std document loaders. Typed plain-scalar
/// strings can opt into the span-aware sibling to retain source text. The std target uses the
/// span-aware sibling `parse(...)` and discards span data on
/// the `Value`-target fast path via `load_one_no_spans` instead.
#[cfg(not(feature = "std"))]
pub(crate) fn parse_all_values(input: &str, config: &ParseConfig) -> Result<Vec<Value>> {
    loader::load_all_no_spans(input, config)
}
