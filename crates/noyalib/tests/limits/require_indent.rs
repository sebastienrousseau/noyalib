// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Surface tests for issue #6 — the `RequireIndent` enum +
//! `ParserConfig::require_indent` builder.
//!
//! Note: enforcement at the scanner layer is deferred to a
//! follow-up PR (the issue itself flags it as the
//! "most invasive Phase 1 item, deferred to separate PR").
//! This file pins the API surface so the follow-up only has
//! to wire the runtime check; the type contract stays stable.

#![allow(missing_docs)]

use noyalib::{ParserConfig, RequireIndent};

#[test]
fn default_is_unchecked() {
    assert_eq!(RequireIndent::default(), RequireIndent::Unchecked);
    assert_eq!(ParserConfig::new().require_indent, RequireIndent::Unchecked);
}

#[test]
fn strict_uses_even() {
    assert_eq!(ParserConfig::strict().require_indent, RequireIndent::Even);
}

#[test]
fn builder_sets_each_variant() {
    let cfg = ParserConfig::new().require_indent(RequireIndent::Even);
    assert_eq!(cfg.require_indent, RequireIndent::Even);

    let cfg = ParserConfig::new().require_indent(RequireIndent::Divisible(4));
    assert_eq!(cfg.require_indent, RequireIndent::Divisible(4));

    let cfg = ParserConfig::new().require_indent(RequireIndent::Uniform(Some(2)));
    assert_eq!(cfg.require_indent, RequireIndent::Uniform(Some(2)));

    let cfg = ParserConfig::new().require_indent(RequireIndent::Uniform(None));
    assert_eq!(cfg.require_indent, RequireIndent::Uniform(None));
}

#[test]
fn variants_are_distinct() {
    assert_ne!(RequireIndent::Unchecked, RequireIndent::Even);
    assert_ne!(RequireIndent::Even, RequireIndent::Divisible(2));
    assert_ne!(
        RequireIndent::Uniform(Some(2)),
        RequireIndent::Uniform(None)
    );
    assert_ne!(
        RequireIndent::Uniform(Some(2)),
        RequireIndent::Uniform(Some(4))
    );
}

#[test]
fn debug_includes_variant_name() {
    let s = format!("{:?}", RequireIndent::Even);
    assert_eq!(s, "Even");
    let s = format!("{:?}", RequireIndent::Divisible(2));
    assert!(s.contains("Divisible"));
    assert!(s.contains('2'));
}

#[test]
fn require_indent_clone_copy_eq() {
    let a = RequireIndent::Divisible(4);
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn debug_for_unchecked_variant() {
    assert_eq!(format!("{:?}", RequireIndent::Unchecked), "Unchecked");
}

#[test]
fn debug_for_uniform_variants() {
    let s = format!("{:?}", RequireIndent::Uniform(Some(2)));
    assert!(s.contains("Uniform"));
    let s = format!("{:?}", RequireIndent::Uniform(None));
    assert!(s.contains("Uniform"));
    assert!(s.contains("None"));
}
