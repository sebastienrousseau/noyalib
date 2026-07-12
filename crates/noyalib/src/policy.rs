// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Pluggable parser policies for "Safe YAML" enforcement.
//!
//! A [`Policy`](crate::policy::Policy) inspects parser events as
//! the document is loaded and rejects any that violate
//! organisational constraints — common examples are "no anchors",
//! "no custom tags", or "no scalar larger than N bytes". Policies
//! fire on the AST loader path; if any policy is registered the
//! streaming fast-path is bypassed automatically so the policy
//! contract is honoured everywhere.
//!
//! Built-in policies live in this module:
//!
//! - [`DenyAnchors`](crate::policy::DenyAnchors) — reject any
//!   document that defines or dereferences an anchor.
//! - [`DenyTags`](crate::policy::DenyTags) — reject any tagged
//!   scalar / collection.
//! - [`MaxScalarLength`](crate::policy::MaxScalarLength) — cap
//!   individual scalar length in bytes.
//!
//! Custom policies implement the trait directly. Stateful policies
//! that need to mutate during a parse should hold their state
//! behind interior mutability ([`std::sync::Mutex`] or equivalent).
//!
//! # Examples
//!
//! ```
//! use noyalib::{from_str_with_config, ParserConfig, Value};
//! use noyalib::policy::DenyAnchors;
//!
//! let cfg = ParserConfig::new().with_policy(DenyAnchors);
//! let res: Result<Value, _> =
//!     from_str_with_config("k: &x 1\nv: *x\n", &cfg);
//! assert!(res.is_err(), "DenyAnchors must reject anchored input");
//! ```

use crate::error::{Error, Result};
use crate::prelude::*;
use crate::value::Value;

/// Kind of parser event handed to a policy.
///
/// # Examples
///
/// ```
/// use noyalib::policy::PolicyEventKind;
/// assert_eq!(PolicyEventKind::Scalar, PolicyEventKind::Scalar);
/// assert_ne!(PolicyEventKind::Scalar, PolicyEventKind::Alias);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEventKind {
    /// A scalar value (plain, quoted, literal, or folded).
    Scalar,
    /// The start of a sequence (block or flow).
    SequenceStart,
    /// The start of a mapping (block or flow).
    MappingStart,
    /// An alias dereference (`*name`).
    Alias,
}

/// Lightweight projection of a parser event for policy inspection.
///
/// `PolicyEvent` borrows from the parser's internal event so
/// policies can inspect the anchor name, tag URI, and scalar text
/// without taking ownership.
///
/// # Examples
///
/// ```
/// use noyalib::policy::{PolicyEvent, PolicyEventKind};
/// let ev = PolicyEvent {
///     kind: PolicyEventKind::Scalar,
///     anchor: None,
///     tag: Some("!!str"),
///     scalar: Some("hello"),
/// };
/// assert_eq!(ev.kind, PolicyEventKind::Scalar);
/// assert_eq!(ev.scalar, Some("hello"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PolicyEvent<'a> {
    /// The kind of event.
    pub kind: PolicyEventKind,
    /// The anchor on this node, if any (the `&name` after the `&`).
    pub anchor: Option<&'a str>,
    /// The fully-reconstructed tag on this node, if any (e.g.
    /// `"!!str"`, `"!Custom"`, or `"tag:yaml.org,2002:binary"`).
    pub tag: Option<&'a str>,
    /// The decoded scalar content (escapes already resolved), present
    /// only for [`PolicyEventKind::Scalar`].
    pub scalar: Option<&'a str>,
}

/// Pluggable "Safe YAML" policy.
///
/// Implementors override either or both check methods. The default
/// implementations accept everything, so a policy that only cares
/// about the post-parse value can leave [`Policy::check_event`]
/// alone.
///
/// # Examples
///
/// ```
/// use noyalib::policy::{Policy, PolicyEvent, PolicyEventKind};
/// use noyalib::{from_str_with_config, ParserConfig, Value, Result, Error};
///
/// #[derive(Debug, Default)]
/// struct DenyTabs;
/// impl Policy for DenyTabs {
///     fn check_event(&self, ev: PolicyEvent<'_>) -> Result<()> {
///         if ev.kind == PolicyEventKind::Scalar
///             && ev.scalar.is_some_and(|s| s.contains('\t'))
///         {
///             return Err(Error::Deserialize("tab in scalar".into()));
///         }
///         Ok(())
///     }
/// }
///
/// let cfg = ParserConfig::new().with_policy(DenyTabs);
/// let res: Result<Value> = from_str_with_config("k: \"a\\tb\"\n", &cfg);
/// assert!(res.is_err());
/// ```
pub trait Policy: Send + Sync + fmt::Debug {
    /// Inspect a parser event during loading. Return `Err(...)` to
    /// abort the parse with a diagnostic.
    fn check_event(&self, _event: PolicyEvent<'_>) -> Result<()> {
        Ok(())
    }

    /// Inspect the fully-built [`Value`] tree. Runs after the AST
    /// is assembled and before deserialisation continues. Return
    /// `Err(...)` to reject the document.
    fn check_value(&self, _value: &Value) -> Result<()> {
        Ok(())
    }
}

/// Reject any document that defines an anchor (`&name`) or
/// dereferences one (`*name`).
///
/// Aliases are a known billion-laughs vector and a major
/// readability hazard in audited configs; many enterprise pipelines
/// disable them outright.
///
/// # Examples
///
/// ```
/// use noyalib::policy::DenyAnchors;
/// use noyalib::{from_str_with_config, ParserConfig, Value};
/// let cfg = ParserConfig::new().with_policy(DenyAnchors);
/// let res: Result<Value, _> = from_str_with_config("k: &x 1\nv: *x\n", &cfg);
/// assert!(res.is_err());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct DenyAnchors;

impl Policy for DenyAnchors {
    fn check_event(&self, event: PolicyEvent<'_>) -> Result<()> {
        if let Some(name) = event.anchor {
            return Err(Error::Deserialize(format!(
                "policy DenyAnchors: anchor `&{name}` is not allowed"
            )));
        }
        if event.kind == PolicyEventKind::Alias {
            return Err(Error::Deserialize(
                "policy DenyAnchors: alias dereference is not allowed".into(),
            ));
        }
        Ok(())
    }
}

/// Reject any document carrying a custom (non-default) tag.
///
/// Default YAML 1.2 core tags (`!!str`, `!!int`, `!!bool`,
/// `!!float`, `!!null`, `!!seq`, `!!map`, `!!binary`) are still
/// permitted — only user-defined tags trigger rejection. Useful in
/// configs where downstream consumers do not understand custom tag
/// resolution.
///
/// # Examples
///
/// ```
/// use noyalib::policy::DenyTags;
/// use noyalib::{from_str_with_config, ParserConfig, Value};
/// let cfg = ParserConfig::new().with_policy(DenyTags);
/// let bad: Result<Value, _> = from_str_with_config("k: !Custom 1\n", &cfg);
/// assert!(bad.is_err());
/// // Core tags are still allowed.
/// let ok: Value = from_str_with_config("k: !!str 1\n", &cfg).unwrap();
/// assert!(matches!(ok, Value::Mapping(_)));
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct DenyTags;

impl Policy for DenyTags {
    fn check_event(&self, event: PolicyEvent<'_>) -> Result<()> {
        if let Some(tag) = event.tag {
            if !is_core_tag(tag) {
                return Err(Error::Deserialize(format!(
                    "policy DenyTags: tag `{tag}` is not allowed"
                )));
            }
        }
        Ok(())
    }
}

/// Cap the byte length of any individual scalar.
///
/// Counts the decoded scalar content (escape sequences resolved),
/// *not* the post-resolution typed value; numeric / boolean scalars
/// are measured by their textual representation. Helpful for
/// resource-constrained pipelines that
/// cannot trust upstream input size.
///
/// # Examples
///
/// ```
/// use noyalib::policy::MaxScalarLength;
/// use noyalib::{from_str_with_config, ParserConfig, Value};
/// let cfg = ParserConfig::new().with_policy(MaxScalarLength(8));
/// let ok: Value = from_str_with_config("k: short\n", &cfg).unwrap();
/// assert!(matches!(ok, Value::Mapping(_)));
/// let long: Result<Value, _> =
///     from_str_with_config("k: this-is-too-long\n", &cfg);
/// assert!(long.is_err());
/// ```
#[derive(Debug, Clone, Copy)]
pub struct MaxScalarLength(pub usize);

impl Policy for MaxScalarLength {
    fn check_event(&self, event: PolicyEvent<'_>) -> Result<()> {
        if let Some(s) = event.scalar {
            if s.len() > self.0 {
                return Err(Error::Deserialize(format!(
                    "policy MaxScalarLength: scalar of {} bytes exceeds limit of {}",
                    s.len(),
                    self.0
                )));
            }
        }
        Ok(())
    }
}

fn is_core_tag(tag: &str) -> bool {
    matches!(
        tag,
        "!!str"
            | "!!int"
            | "!!bool"
            | "!!float"
            | "!!null"
            | "!!seq"
            | "!!map"
            | "!!binary"
            | "tag:yaml.org,2002:str"
            | "tag:yaml.org,2002:int"
            | "tag:yaml.org,2002:bool"
            | "tag:yaml.org,2002:float"
            | "tag:yaml.org,2002:null"
            | "tag:yaml.org,2002:seq"
            | "tag:yaml.org,2002:map"
            | "tag:yaml.org,2002:binary"
    )
}
