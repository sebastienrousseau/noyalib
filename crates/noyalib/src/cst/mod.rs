// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Side-table CST (concrete syntax tree) for lossless round-tripping.
//!
//! This module is the implementation of the design described in
//! `docs/design/green-tree.md`. It exposes a `Document` type that
//! parses YAML byte-faithfully — every byte of the input is retained
//! as a green-tree leaf — so that
//! `parse_document(s).unwrap().to_string()` is byte-identical to `s`
//! for any input the parser accepts.
//!
//! The `Value` API (`from_str`, `to_string`, `StreamingDeserializer`)
//! is unchanged. Trivia capture is enabled only on this path; the
//! fast path pays no extra cost.
//!
//! # Current scope
//!
//! - **Read access.** [`Document::as_value`](crate::cst::Document::as_value)
//!   for a typed view, [`Document::span_at`](crate::cst::Document::span_at)
//!   / [`Document::get`](crate::cst::Document::get) for byte-range
//!   lookups by `path`, and
//!   [`Document::syntax`](crate::cst::Document::syntax) for the green
//!   tree itself.
//! - **Mutation.** [`Document::replace_span`](crate::cst::Document::replace_span)
//!   (primitive byte replacement) and
//!   [`Document::set`](crate::cst::Document::set) (path-targeted, the
//!   wrapper most callers want). Both re-parse on edit and reject
//!   the change if the spliced source is invalid YAML, leaving the
//!   document untouched.
//!
//! The green tree itself is still a flat sequence of leaves under a
//! single `Document` parent — sufficient for byte-faithful
//! round-tripping and for the span-based edit primitive. Hierarchical
//! nesting (per-mapping / per-sequence parent nodes) and an `Emit`
//! trait that auto-formats replacement values are tracked as
//! follow-ups in `docs/design/green-tree.md`.
//!
//! # Examples
//!
//! ```
//! use noyalib::cst::parse_document;
//!
//! let src = "name: noyalib  # the project\nversion: 0.0.1\n";
//! let doc = parse_document(src).unwrap();
//! assert_eq!(doc.to_string(), src);
//! ```
//!
//! # Multi-document streams
//!
//! Use [`parse_stream`](crate::cst::parse_stream) for inputs
//! containing `---` / `...` separators — one
//! [`Document`](crate::cst::Document) per logical YAML document,
//! with each slice covering the exact bytes of that document so
//! concatenation reproduces the input verbatim:
//!
//! ```
//! use noyalib::cst::{parse_stream, Document};
//!
//! let src = "---\nfoo: 1\n...\n---\nbar: 2\n";
//! let docs = parse_stream(src).unwrap();
//! assert_eq!(docs.len(), 2);
//! assert_eq!(docs[0].as_value()["foo"].as_i64(), Some(1));
//! assert_eq!(docs[1].as_value()["bar"].as_i64(), Some(2));
//! let joined: String = docs.iter().map(Document::source).collect();
//! assert_eq!(joined, src);
//! ```

mod anchor;
mod annotated;
mod builder;
#[cfg(feature = "validate-schema")]
mod coerce;
mod document;
mod entry;
mod format;
mod green;
mod syntax;

pub use anchor::{AliasInfo, AnchorInfo};
pub use annotated::CommentBundle;
#[cfg(feature = "validate-schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "validate-schema")))]
pub use coerce::coerce_to_schema;
pub use document::{Document, RepairScope, parse_document, parse_stream};
pub use entry::Entry;
pub use format::{FormatConfig, format, format_with_config};
pub use green::{GreenChild, GreenNode};
pub use syntax::SyntaxKind;
