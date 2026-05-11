// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Declarative builders for the public config types.
//!
//! Pairs with the chained-setter builders on [`crate::ParserConfig`]
//! and [`crate::SerializerConfig`]: each `field: value` entry is
//! translated to a call to the same-named builder method on the
//! `new()` baseline, so the result is byte-identical to writing the
//! chain by hand. Zero runtime overhead — the expansion is a single
//! struct construction the optimiser folds away.
//!
//! # Examples
//!
//! ```
//! use noyalib::parser_config;
//!
//! let cfg = parser_config! {
//!     max_depth: 64,
//!     strict_booleans: true,
//! };
//! assert_eq!(cfg.max_depth, 64);
//! assert!(cfg.strict_booleans);
//! ```
//!
//! ```
//! use noyalib::serializer_config;
//!
//! let cfg = serializer_config! {
//!     indent: 4,
//!     quote_all: true,
//! };
//! ```
//!
//! Both macros accept zero entries (yielding the same value as
//! `ParserConfig::new()` / `SerializerConfig::new()`) and a
//! trailing comma after the last entry.

/// Construct a [`crate::ParserConfig`] from a field-value list.
///
/// Equivalent to chaining the named builder methods on
/// [`crate::ParserConfig::new()`]. Empty input returns the
/// `new()` baseline. Trailing comma is permitted.
///
/// # Examples
///
/// ```
/// use noyalib::parser_config;
///
/// // Empty form — equivalent to `ParserConfig::new()`.
/// let _ = parser_config! {};
///
/// // Full chain.
/// let cfg = parser_config! {
///     max_depth: 32,
///     max_alias_expansions: 200,
///     strict_booleans: true,
/// };
/// assert_eq!(cfg.max_depth, 32);
/// assert_eq!(cfg.max_alias_expansions, 200);
/// assert!(cfg.strict_booleans);
/// ```
#[macro_export]
macro_rules! parser_config {
    () => { $crate::ParserConfig::new() };
    ( $( $field:ident : $value:expr ),+ $(,)? ) => {{
        $crate::ParserConfig::new()
            $( .$field($value) )+
    }};
}

/// Construct a [`crate::SerializerConfig`] from a field-value list.
///
/// Equivalent to chaining the named builder methods on
/// [`crate::SerializerConfig::new()`]. Empty input returns the
/// `new()` baseline. Trailing comma is permitted.
///
/// # Examples
///
/// ```
/// use noyalib::serializer_config;
///
/// // Empty form — equivalent to `SerializerConfig::new()`.
/// let _ = serializer_config! {};
///
/// // Full chain.
/// let cfg = serializer_config! {
///     indent: 4,
///     quote_all: true,
/// };
/// ```
#[macro_export]
macro_rules! serializer_config {
    () => { $crate::SerializerConfig::new() };
    ( $( $field:ident : $value:expr ),+ $(,)? ) => {{
        $crate::SerializerConfig::new()
            $( .$field($value) )+
    }};
}
