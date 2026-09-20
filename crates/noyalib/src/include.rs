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

impl fmt::Debug for IncludeResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
/// On Unix, the root is opened once as a directory capability. Every
/// subsequent file open is relative to that handle, so renaming or
/// replacing the path used to construct the resolver cannot redirect
/// later reads. Targets canonicalise to a root-relative path and then
/// open every component without following symlinks. Windows retains
/// canonical root checks. Path-traversal attempts (`../../etc/passwd`)
/// and symlink targets outside the root are rejected before content is
/// read.
///
/// # Symlinks
///
/// Controlled by [`SymlinkPolicy`]. On Unix, the default
/// [`SymlinkPolicy::FollowWithinRoot`] resolves symlinks through
/// the directory capability. [`SymlinkPolicy::Reject`] opens each
/// directory component and the final file without following
/// symlinks, avoiding a metadata-then-open race. Windows enforces
/// the same policies through canonical path and metadata checks.
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
    capability: Arc<RootCapability>,
}

#[cfg(feature = "include_fs")]
#[derive(Debug)]
enum RootCapability {
    #[cfg(unix)]
    Ready {
        dir: std::fs::File,
        canonical_root: std::path::PathBuf,
    },
    #[cfg(not(unix))]
    Ready {
        canonical_root: std::path::PathBuf,
    },
    Failed(String),
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
    ///
    /// The root is opened during construction. Because this constructor
    /// retains its historical infallible signature, an open failure is
    /// stored and returned when the resolver is first invoked.
    #[must_use]
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        let root = root.into();
        let capability = match open_root_capability(&root) {
            Ok(capability) => capability,
            Err(error) => RootCapability::Failed(error.to_string()),
        };
        Self {
            root,
            symlink_policy: SymlinkPolicy::default(),
            capability: Arc::new(capability),
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
        let this = self;
        IncludeResolver::new(move |req: IncludeRequest<'_>| this.resolve(req))
    }

    fn resolve(&self, req: IncludeRequest<'_>) -> Result<InputSource> {
        use std::io::Read as _;

        // Strip the optional `#anchor` fragment — the loader
        // handles anchor selection after parse, so the resolver
        // only needs the path portion.
        let (path_part, _frag) = split_fragment(req.spec);
        let relative = normalize_relative_path(path_part).map_err(|message| {
            Error::Custom(format!("include resolver: `{path_part}` {message}"))
        })?;
        let capability = self.root_capability()?;
        let (mut file, display_path) = open_from_root(
            capability,
            &relative,
            self.symlink_policy,
            &self.root,
        )
        .map_err(|e| {
            Error::Custom(format!(
                "include resolver: `{}` escapes sandbox root, contains a rejected symlink, or cannot be opened: {e}",
                self.root.join(&relative).display()
            ))
        })?;
        let mut bytes = String::new();
        let _bytes_read = file.read_to_string(&mut bytes).map_err(|e| {
            Error::Custom(format!(
                "include resolver: cannot read `{}`: {e}",
                display_path.display()
            ))
        })?;
        Ok(InputSource::new(display_path.display().to_string(), bytes))
    }

    fn root_capability(&self) -> Result<&RootCapability> {
        match self.capability.as_ref() {
            ready @ RootCapability::Ready { .. } => Ok(ready),
            RootCapability::Failed(error) => Err(Error::Custom(format!(
                "include resolver: cannot open root `{}`: {error}",
                self.root.display()
            ))),
        }
    }
}

#[cfg(feature = "include_fs")]
fn normalize_relative_path(path: &str) -> core::result::Result<std::path::PathBuf, &'static str> {
    use std::path::Component;

    let mut normalized = std::path::PathBuf::new();
    for component in std::path::Path::new(path).components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                return Err("must be relative to the root");
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err("escapes sandbox root");
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    Ok(normalized)
}

#[cfg(all(feature = "include_fs", unix))]
fn open_root_capability(root: &std::path::Path) -> std::io::Result<RootCapability> {
    use rustix::fs::{Mode, OFlags};

    let fd = rustix::fs::open(
        root,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let dir = std::fs::File::from(fd);
    #[cfg(target_vendor = "apple")]
    let canonical_root = {
        use std::os::unix::ffi::OsStringExt as _;

        let path = rustix::fs::getpath(&dir)?;
        std::path::PathBuf::from(std::ffi::OsString::from_vec(path.into_bytes()))
    };
    #[cfg(not(target_vendor = "apple"))]
    let canonical_root = std::fs::canonicalize(root)?;

    Ok(RootCapability::Ready {
        dir,
        canonical_root,
    })
}

#[cfg(all(feature = "include_fs", not(unix)))]
fn open_root_capability(root: &std::path::Path) -> std::io::Result<RootCapability> {
    Ok(RootCapability::Ready {
        canonical_root: std::fs::canonicalize(root)?,
    })
}

#[cfg(all(feature = "include_fs", unix))]
fn open_from_root(
    capability: &RootCapability,
    relative: &std::path::Path,
    policy: SymlinkPolicy,
    _root_label: &std::path::Path,
) -> std::io::Result<(std::fs::File, std::path::PathBuf)> {
    let RootCapability::Ready {
        dir,
        canonical_root,
    } = capability
    else {
        unreachable!("failed root capabilities are rejected before open")
    };
    let active_root = current_root_path(dir).unwrap_or_else(|_| canonical_root.clone());
    let (open_path, identity) = if policy == SymlinkPolicy::FollowWithinRoot {
        let canonical = std::fs::canonicalize(active_root.join(relative))?;
        let beneath = canonical.strip_prefix(&active_root).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "resolved path escapes sandbox root",
            )
        })?;
        (beneath.to_path_buf(), canonical)
    } else {
        (relative.to_path_buf(), active_root.join(relative))
    };
    let file = open_relative_nofollow(dir, &open_path)?;
    Ok((file, identity))
}

#[cfg(all(feature = "include_fs", unix, target_vendor = "apple"))]
fn current_root_path(root: &std::fs::File) -> std::io::Result<std::path::PathBuf> {
    use std::os::unix::ffi::OsStringExt as _;

    let path = rustix::fs::getpath(root)?;
    Ok(std::path::PathBuf::from(std::ffi::OsString::from_vec(
        path.into_bytes(),
    )))
}

#[cfg(all(
    feature = "include_fs",
    unix,
    any(target_os = "linux", target_os = "android")
))]
fn current_root_path(root: &std::fs::File) -> std::io::Result<std::path::PathBuf> {
    use std::os::fd::AsRawFd as _;

    std::fs::read_link(format!("/proc/self/fd/{}", root.as_raw_fd()))
}

#[cfg(all(
    feature = "include_fs",
    unix,
    not(any(target_vendor = "apple", target_os = "linux", target_os = "android"))
))]
fn current_root_path(_root: &std::fs::File) -> std::io::Result<std::path::PathBuf> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "the target cannot recover a path from a directory handle",
    ))
}

#[cfg(all(feature = "include_fs", not(unix)))]
fn open_from_root(
    capability: &RootCapability,
    relative: &std::path::Path,
    policy: SymlinkPolicy,
    _root_label: &std::path::Path,
) -> std::io::Result<(std::fs::File, std::path::PathBuf)> {
    let RootCapability::Ready { canonical_root } = capability else {
        unreachable!("failed root capabilities are rejected before open")
    };
    let canonical = std::fs::canonicalize(canonical_root.join(relative))?;
    if !canonical.starts_with(canonical_root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "resolved path escapes sandbox root",
        ));
    }
    if policy == SymlinkPolicy::Reject && path_contains_symlink(canonical_root, relative)? {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "symlink rejected by policy",
        ));
    }
    Ok((std::fs::File::open(&canonical)?, canonical))
}

#[cfg(all(feature = "include_fs", unix))]
fn open_relative_nofollow(
    root: &std::fs::File,
    relative: &std::path::Path,
) -> std::io::Result<std::fs::File> {
    use rustix::fs::{Mode, OFlags};

    let file_name = relative.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "include path names no file",
        )
    })?;
    let mut parent = rustix::fs::openat(
        root,
        ".",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    if let Some(ancestors) = relative.parent() {
        for component in ancestors.components() {
            parent = rustix::fs::openat(
                &parent,
                component.as_os_str(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )?;
        }
    }
    let fd = rustix::fs::openat(
        &parent,
        file_name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    Ok(std::fs::File::from(fd))
}

#[cfg(all(feature = "include_fs", not(unix)))]
fn path_contains_symlink(
    root: &std::path::Path,
    relative: &std::path::Path,
) -> std::io::Result<bool> {
    let mut candidate = root.to_path_buf();
    for component in relative.components() {
        candidate.push(component.as_os_str());
        if std::fs::symlink_metadata(&candidate)?
            .file_type()
            .is_symlink()
        {
            return Ok(true);
        }
    }
    Ok(false)
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
