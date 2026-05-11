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
//! - **`include_fs` feature** (`SafeFileResolver`) — a
//!   filesystem-backed implementation with root-dir sandboxing,
//!   symlink-policy enforcement (`SymlinkPolicy`), and
//!   max-depth cycle protection.
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

#[cfg(feature = "include_fs")]
use crate::error::Error;
use crate::error::Result;
use crate::prelude::*;

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

impl core::fmt::Debug for IncludeResolver {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IncludeResolver")
            .field("ptr", &Arc::as_ptr(&self.0))
            .finish()
    }
}

/// How [`SafeFileResolver`] handles symbolic links it
/// encounters while resolving a path.
///
/// # Examples
///
/// ```
/// use noyalib::include::SymlinkPolicy;
/// assert_eq!(SymlinkPolicy::default(), SymlinkPolicy::FollowWithinRoot);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum SymlinkPolicy {
    /// Follow symlinks that resolve to a path still inside the
    /// resolver's root directory. Reject anything pointing
    /// outside. Default.
    #[default]
    FollowWithinRoot,
    /// Reject all symbolic links regardless of target. Strictest
    /// posture; appropriate for untrusted document graphs.
    Reject,
}

/// Filesystem-backed [`IncludeResolver`] with root-dir
/// sandboxing.
///
/// Behind the `include_fs` Cargo feature (which implies
/// `include` + `std`).
///
/// # Sandboxing
///
/// All resolved paths are canonicalised (via [`std::fs::canonicalize`])
/// and verified to live inside the supplied `root` directory.
/// Path-traversal attempts (`../../etc/passwd`) are caught at
/// the canonicalisation step — the canonical path simply will
/// not have `root` as a prefix, and the resolver errors.
///
/// # Symlinks
///
/// Controlled by [`SymlinkPolicy`]. The default
/// [`SymlinkPolicy::FollowWithinRoot`] follows symlinks but
/// re-applies the root-prefix check against the resolved
/// target. [`SymlinkPolicy::Reject`] errors on any symlink in
/// the path.
///
/// # Examples
///
/// ```no_run
/// use noyalib::include::{SafeFileResolver, SymlinkPolicy};
/// use std::sync::Arc;
///
/// let resolver = SafeFileResolver::new("/srv/configs")
///     .symlink_policy(SymlinkPolicy::Reject)
///     .into_resolver();
/// let cfg = noyalib::ParserConfig::new().include_resolver(resolver);
/// # let _ = cfg;
/// ```
#[cfg(feature = "include_fs")]
#[cfg_attr(docsrs, doc(cfg(feature = "include_fs")))]
#[derive(Debug, Clone)]
pub struct SafeFileResolver {
    root: std::path::PathBuf,
    symlink_policy: SymlinkPolicy,
}

#[cfg(feature = "include_fs")]
impl SafeFileResolver {
    /// Construct a resolver rooted at `root`. All resolved paths
    /// must canonicalise to a descendant of `root`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::include::SafeFileResolver;
    /// let r = SafeFileResolver::new("/srv/configs");
    /// let _ = r;
    /// ```
    #[must_use]
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            root: root.into(),
            symlink_policy: SymlinkPolicy::default(),
        }
    }

    /// Set the [`SymlinkPolicy`].
    #[must_use]
    pub fn symlink_policy(mut self, policy: SymlinkPolicy) -> Self {
        self.symlink_policy = policy;
        self
    }

    /// Convert this configuration into a boxed [`IncludeResolver`]
    /// suitable for [`crate::ParserConfig::include_resolver`].
    #[must_use]
    pub fn into_resolver(self) -> IncludeResolver {
        let this = self.clone();
        IncludeResolver::new(move |req: IncludeRequest<'_>| this.resolve(req))
    }

    fn resolve(&self, req: IncludeRequest<'_>) -> Result<InputSource> {
        use std::fs;
        // Strip the optional `#anchor` fragment — the loader
        // handles anchor selection after parse, so the resolver
        // only needs the path portion.
        let (path_part, _frag) = split_fragment(req.spec);
        let candidate = self.root.join(path_part);

        // Reject paths whose canonical form jumps outside `root`.
        let canon_root = fs::canonicalize(&self.root).map_err(|e| {
            Error::Custom(format!("include resolver: cannot canonicalise root: {e}"))
        })?;
        let canon = fs::canonicalize(&candidate).map_err(|e| {
            Error::Custom(format!(
                "include resolver: cannot canonicalise `{}`: {e}",
                candidate.display()
            ))
        })?;
        if !canon.starts_with(&canon_root) {
            return Err(Error::Custom(format!(
                "include resolver: `{}` escapes sandbox root `{}`",
                canon.display(),
                canon_root.display()
            )));
        }

        if self.symlink_policy == SymlinkPolicy::Reject {
            // `fs::symlink_metadata` returns metadata of the link itself
            // (not the target). If the original (un-canonicalised)
            // path is a symlink the policy rejects.
            let meta = fs::symlink_metadata(&candidate).map_err(|e| {
                Error::Custom(format!(
                    "include resolver: cannot stat `{}`: {e}",
                    candidate.display()
                ))
            })?;
            if meta.file_type().is_symlink() {
                return Err(Error::Custom(format!(
                    "include resolver: symlink rejected by policy: `{}`",
                    candidate.display()
                )));
            }
        }

        let bytes = fs::read_to_string(&canon).map_err(|e| {
            Error::Custom(format!(
                "include resolver: cannot read `{}`: {e}",
                canon.display()
            ))
        })?;
        Ok(InputSource::new(canon.display().to_string(), bytes))
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
