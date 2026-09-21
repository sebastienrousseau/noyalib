// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `!include` directive support — compose YAML documents from
//! multiple files via the `!include path/to/file.yaml` tag.
//!
//! Two layers:
//!
//! - **`include` feature** (this module's free-standing types) —
//!   defines `IncludeResolver`, `IncludeRequest`, and
//!   `InputSource`. The resolver is a `Send + Sync` closure
//!   stored on [`crate::ParserConfig`]; users wire it up via
//!   [`crate::ParserConfig::include_resolver`].
//!
//! - **`include_fs` feature** (`SafeFileResolver`) — a Unix
//!   capability-rooted filesystem implementation, with a Windows
//!   canonical-root fallback, symlink-policy enforcement
//!   (`SymlinkPolicy`), and max-depth cycle protection.
//!
//! Fragment anchors (`!include file.yaml#name`) resolve the named
//! YAML anchor inside the included document and substitute its
//! value rather than the whole document. Plain `!include
//! file.yaml` substitutes the document root.
//!
//! Cyclic includes (A includes B includes A) are rejected via a
//! per-resolution visited set; the depth ceiling
//! [`crate::ParserConfig::max_include_depth`] (default 24)
//! bounds the recursion.

use crate::error::Result;
use crate::prelude::*;

#[cfg(feature = "include_fs")]
mod fs;
#[cfg(feature = "include_fs")]
#[cfg_attr(docsrs, doc(cfg(feature = "include_fs")))]
pub use fs::{SafeFileResolver, SymlinkPolicy};

/// Describes one `!include` request the loader hands to the
/// resolver.
///
/// The `spec` is the YAML scalar text after `!include` —
/// typically a file path, possibly with a `#anchor` fragment.
/// Resolvers are free to interpret the spec however they like
/// (file path, URL, key in a virtual filesystem); the
/// [`SafeFileResolver`] interprets it as a filesystem path.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct IncludeRequest<'a> {
    /// The path / URL / identifier the user wrote after
    /// `!include`. Includes the optional `#anchor` fragment.
    pub spec: &'a str,
    /// The source identifier of the document making this
    /// request. The top-level document is `0`; nested includes
    /// receive a fresh id from the parser.
    pub from_id: usize,
    /// Inclusion depth (0 = top-level document, 1 = first
    /// nested include, …). Resolvers can refuse to resolve
    /// beyond a certain depth or use this for diagnostics.
    pub depth: usize,
}

/// What a resolver returns: the YAML text plus a stable
/// identifier that downstream layers use for cycle detection
/// and span-source attribution.
///
/// `name` is shown in diagnostic output — typically the
/// canonicalised file path. `bytes` is the YAML text the loader
/// will parse.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct InputSource {
    /// Display name (file path, URL, …).
    pub name: String,
    /// The YAML text to parse.
    pub bytes: String,
}

impl InputSource {
    /// Construct a new [`InputSource`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::include::InputSource;
    /// let s = InputSource::new("config.yaml", "k: 1\n");
    /// assert_eq!(s.name, "config.yaml");
    /// assert_eq!(s.bytes, "k: 1\n");
    /// ```
    #[must_use]
    pub fn new(name: impl Into<String>, bytes: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bytes: bytes.into(),
        }
    }
}

/// Resolver closure stored on [`crate::ParserConfig`].
///
/// Wraps an `Arc<dyn Fn>` so the type is `Clone + Debug` (the
/// underlying `dyn Fn` is not). Construct with
/// [`IncludeResolver::new`].
///
/// `Arc` (not `Box`) keeps configs cheap to clone. The closure
/// is `Send + Sync` so the resolver can be invoked from any
/// thread of a parallel parse.
#[derive(Clone)]
pub struct IncludeResolver(Arc<dyn Fn(IncludeRequest<'_>) -> Result<InputSource> + Send + Sync>);

impl IncludeResolver {
    /// Wrap a closure as an [`IncludeResolver`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::include::{IncludeRequest, IncludeResolver, InputSource};
    /// use noyalib::Result;
    /// let r = IncludeResolver::new(|req: IncludeRequest<'_>| -> Result<InputSource> {
    ///     Ok(InputSource::new(req.spec, "v: 1\n"))
    /// });
    /// let _ = r;
    /// ```
    #[must_use]
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(IncludeRequest<'_>) -> Result<InputSource> + Send + Sync + 'static,
    {
        Self(Arc::new(f))
    }

    /// Invoke the wrapped closure.
    ///
    /// # Errors
    ///
    /// Surfaces whatever the underlying resolver returned.
    pub fn resolve(&self, req: IncludeRequest<'_>) -> Result<InputSource> {
        (self.0)(req)
    }
}

impl fmt::Debug for IncludeResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IncludeResolver")
            .field("ptr", &Arc::as_ptr(&self.0))
            .finish()
    }
}

/// Split `path#fragment` into `(path, Some(fragment))` /
/// `(path, None)`. Used by both the resolver and the post-parse
/// walk so they agree on which characters are path-bytes.
///
/// # Examples
///
/// ```
/// use noyalib::include::split_fragment;
/// assert_eq!(split_fragment("a.yaml#anchor"), ("a.yaml", Some("anchor")));
/// assert_eq!(split_fragment("a.yaml"), ("a.yaml", None));
/// ```
#[must_use]
pub fn split_fragment(spec: &str) -> (&str, Option<&str>) {
    match spec.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (spec, None),
    }
}
