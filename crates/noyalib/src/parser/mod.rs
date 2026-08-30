//! Native YAML 1.2 parser.
//!
//! This module provides a complete YAML 1.2 Core Schema parser that builds
//! `Value` trees directly, with full control over security limits, duplicate
//! key handling, and alias expansion tracking.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

mod events;
mod loader;
mod scanner;

pub(crate) use events::{Event, Parser};
pub(crate) use loader::{
    DuplicateKeyPolicy as InternalDuplicateKeyPolicy, ParseConfig, value_to_key_string,
};
pub(crate) use scanner::ScalarStyle;
// CST builder is the only consumer; gate the re-exports to match.
#[cfg(feature = "std")]
pub(crate) use scanner::{
    RecordedToken, RecordedTokenKind, ScannedComment, Scanner, TokenKind, Trivia, TriviaKind,
};

/// Returns a default (zero) `Span` for use in synthesized events.
pub(crate) fn scanner_span_default() -> scanner::Span {
    scanner::Span::default()
}

use crate::error::Result;
// `prelude::*` brings `Vec` into scope on no_std builds; on std it
// duplicates the std prelude. The `#[allow]` suppresses the
// unused-import warning that fires only under std + the strict
// `-D unused` workspace lint.
#[allow(unused_imports)]
use crate::prelude::*;
#[cfg(feature = "std")]
use crate::span_context::SpanTree;
use crate::value::Value;

/// Parse a YAML string into a list of `(Value, SpanTree)` documents.
#[cfg(feature = "std")]
pub(crate) fn parse(input: &str, config: &ParseConfig) -> Result<Vec<(Value, SpanTree)>> {
    let mut parser = Parser::new(input);
    loader::load(&mut parser, config, input)
}

/// Parse a single YAML document from a string.
///
/// Silently discards any document past the first — see
/// [`loader::load_one`]. Deserialise entry points use
/// [`parse_exactly_one`] instead.
#[cfg(feature = "std")]
pub(crate) fn parse_one(input: &str, config: &ParseConfig) -> Result<(Value, SpanTree)> {
    let mut parser = Parser::new(input);
    loader::load_one(&mut parser, config, input)
}

/// Like [`parse_one`], but errors if the stream carries more than one
/// document. Used by `from_str` / `from_str_with_config`'s AST path
/// (see #351).
#[cfg(feature = "std")]
pub(crate) fn parse_exactly_one(input: &str, config: &ParseConfig) -> Result<(Value, SpanTree)> {
    let mut parser = Parser::new(input);
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
#[cfg(feature = "std")]
pub(crate) fn parse_one_value(input: &str, config: &ParseConfig) -> Result<Value> {
    loader::load_one_no_spans(input, config)
}

/// Parse a single YAML document into a `Value` without building a
/// `SpanTree`, erroring if the stream carries more than one document.
///
/// Available on every target — `std` builds use this from
/// `from_str::<Value>`'s fast path (`Value` has no span field, so a
/// `SpanTree` would be pure waste) and from the `no_std` AST path;
/// `no_std` builds use it exclusively. See #351.
pub(crate) fn parse_exactly_one_value(input: &str, config: &ParseConfig) -> Result<Value> {
    loader::load_exactly_one_no_spans(input, config)
}

/// Parse all YAML documents into `Value`s without building `SpanTree`s.
///
/// Used by `document::load_all_*` on the no_std target where
/// `SpanTree` is unavailable. The std target uses the
/// span-aware sibling `parse(...)` and discards span data on
/// the `Value`-target fast path via `load_one_no_spans` instead.
#[cfg(not(feature = "std"))]
pub(crate) fn parse_all_values(input: &str, config: &ParseConfig) -> Result<Vec<Value>> {
    loader::load_all_no_spans(input, config)
}
