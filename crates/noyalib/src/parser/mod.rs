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
#[cfg(feature = "std")]
pub(crate) fn parse_one(input: &str, config: &ParseConfig) -> Result<(Value, SpanTree)> {
    let mut parser = Parser::new(input);
    loader::load_one(&mut parser, config, input)
}

/// Parse a single YAML document into a `Value` without building a `SpanTree`.
///
/// Available on every target. Callers that don't need span data
/// (e.g. `from_str::<Value>` — `Value` has no span field, so spans
/// are always discarded) should prefer this over [`parse_one`] to
/// avoid the per-node `SpanTree` allocation and the subsequent
/// `build_span_map` walk. `no_std` builds use this exclusively
/// because `SpanTree` requires `std`-only types.
pub(crate) fn parse_one_value(input: &str, config: &ParseConfig) -> Result<Value> {
    loader::load_one_no_spans(input, config)
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
