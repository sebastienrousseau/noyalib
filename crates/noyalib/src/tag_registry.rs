// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Streaming-path registry for custom YAML tag pass-through.
//!
//! The default streaming deserializer routes values carrying a non-core
//! tag (anything outside `!!int`, `!!str`, `!!bool`, …) through the AST
//! fallback so the tag survives as `Value::Tagged(...)`. That's correct
//! when a caller is reading into `Value`, but it forces an unnecessary
//! allocation when the caller already knows the tag maps to a concrete
//! Rust type.
//!
//! [`TagRegistry`] lets a caller declare which custom tags should
//! instead be *stripped* on the streaming path so the inner scalar,
//! sequence, or mapping deserializes directly into the target type —
//! no AST detour, no `#[serde(rename = "!…")]` dance.
//!
//! # Examples
//!
//! ```
//! use noyalib::{from_str_with_config, ParserConfig, TagRegistry};
//! use std::sync::Arc;
//!
//! #[derive(Debug, serde::Deserialize, PartialEq)]
//! struct Celsius(f64);
//!
//! let registry = Arc::new(TagRegistry::new().with("!Celsius"));
//! let cfg = ParserConfig::new().tag_registry(registry);
//!
//! // Without the registry, `!Celsius 42.5` becomes Value::Tagged and a
//! // plain `f64` deserialize fails. With the registry, the tag is
//! // stripped on the streaming path and the scalar deserializes
//! // directly into the target type.
//! let c: Celsius = from_str_with_config("!Celsius 42.5", &cfg).unwrap();
//! assert_eq!(c, Celsius(42.5));
//! ```
//!
//! # When you still need `#[serde(rename = "!tag")]`
//!
//! The registry strips tags; it does not dispatch. If you're reading a
//! tagged value into an *enum* where the tag chooses the variant
//! (`!bang` → `Msg::Bang`, `!quiet` → `Msg::Quiet`), keep using
//! `#[serde(rename)]` — serde itself needs the tag string to pick the
//! variant. The registry is for "I already know the type; just get out
//! of my way" newtype/unit-style use cases (the robotics, scientific,
//! and units scenarios in particular).

use crate::prelude::*;
use crate::prelude::FxHashSet;

/// A set of custom YAML tags that the streaming deserializer should
/// strip and pass through instead of routing via the AST fallback.
///
/// Clone-cheap (just a set of owned strings); typically wrapped in an
/// `Arc` and shared across calls. See the [module-level
/// documentation](crate::tag_registry) for semantics.
///
/// # Examples
///
/// ```
/// use noyalib::TagRegistry;
/// let mut reg = TagRegistry::new();
/// let _ = reg.register("!Celsius");
/// let _ = reg.register("!Meters");
/// assert!(reg.contains("!Celsius"));
/// assert_eq!(reg.len(), 2);
/// ```
#[derive(Debug, Clone, Default)]
pub struct TagRegistry {
    known: FxHashSet<String>,
}

impl TagRegistry {
    /// Construct an empty registry.
    ///
    /// # Examples
    ///
    /// ```
    /// let reg = noyalib::TagRegistry::new();
    /// assert!(reg.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark `tag` as known. Accepts both handle-prefixed forms
    /// (`!Celsius`, `!!custom`) and verbatim tag text. Returns `&mut
    /// self` so callers can chain registrations.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut reg = noyalib::TagRegistry::new();
    /// let _ = reg.register("!Celsius").register("!Fahrenheit");
    /// assert!(reg.contains("!Celsius"));
    /// assert!(reg.contains("!Fahrenheit"));
    /// ```
    pub fn register(&mut self, tag: impl Into<String>) -> &mut Self {
        let _ = self.known.insert(tag.into());
        self
    }

    /// Chainable `register` that consumes `self` — convenient when
    /// building a registry inline.
    ///
    /// # Examples
    ///
    /// ```
    /// let reg = noyalib::TagRegistry::new()
    ///     .with("!Celsius")
    ///     .with("!Meters");
    /// assert_eq!(reg.len(), 2);
    /// ```
    #[must_use]
    pub fn with(mut self, tag: impl Into<String>) -> Self {
        let _ = self.known.insert(tag.into());
        self
    }

    /// Is `tag` registered as a strip-through tag?
    ///
    /// # Examples
    ///
    /// ```
    /// let reg = noyalib::TagRegistry::new().with("!Celsius");
    /// assert!(reg.contains("!Celsius"));
    /// assert!(!reg.contains("!Fahrenheit"));
    /// ```
    #[must_use]
    pub fn contains(&self, tag: &str) -> bool {
        self.known.contains(tag)
    }

    /// Number of tags registered.
    ///
    /// # Examples
    ///
    /// ```
    /// let reg = noyalib::TagRegistry::new().with("!A").with("!B");
    /// assert_eq!(reg.len(), 2);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.known.len()
    }

    /// Returns `true` when no tags are registered.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(noyalib::TagRegistry::new().is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }

    /// Remove every registration.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut reg = noyalib::TagRegistry::new().with("!A");
    /// reg.clear();
    /// assert!(reg.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.known.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        assert!(TagRegistry::new().is_empty());
    }

    #[test]
    fn register_and_contains() {
        let mut reg = TagRegistry::new();
        let _ = reg.register("!Celsius");
        assert!(reg.contains("!Celsius"));
        assert!(!reg.contains("!Fahrenheit"));
    }

    #[test]
    fn with_chain() {
        let reg = TagRegistry::new().with("!a").with("!b").with("!c");
        assert_eq!(reg.len(), 3);
    }

    #[test]
    fn clear_empties() {
        let mut reg = TagRegistry::new().with("!a");
        reg.clear();
        assert!(reg.is_empty());
    }

    #[test]
    fn duplicates_collapse() {
        let mut reg = TagRegistry::new();
        let _ = reg.register("!x");
        let _ = reg.register("!x");
        assert_eq!(reg.len(), 1);
    }
}
