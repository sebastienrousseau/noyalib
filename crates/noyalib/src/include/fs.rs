// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Capability-rooted filesystem support for `!include`.

use super::{IncludeRequest, IncludeResolver, InputSource, split_fragment};
use crate::error::{Error, Result};
use crate::prelude::*;

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
///
/// let resolver = SafeFileResolver::new("/srv/configs")
///     .symlink_policy(SymlinkPolicy::Reject)
///     .into_resolver();
/// let cfg = noyalib::ParserConfig::new().include_resolver(resolver);
/// # let _ = cfg;
/// ```
#[derive(Debug, Clone)]
pub struct SafeFileResolver {
    root: std::path::PathBuf,
    symlink_policy: SymlinkPolicy,
    capability: Arc<RootCapability>,
}

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

        // Strip the optional `#anchor` fragment. The loader handles anchor
        // selection after parse, so the resolver only needs the path portion.
        let (path_part, _fragment) = split_fragment(req.spec);
        let relative = normalize_relative_path(path_part).map_err(|message| {
            Error::Custom(format!("include resolver: `{path_part}` {message}"))
        })?;
        let capability = self.root_capability()?;
        let (mut file, display_path) =
            open_from_root(capability, &relative, self.symlink_policy).map_err(|error| {
                Error::Custom(format!(
                    "include resolver: `{}` escapes sandbox root, contains a rejected symlink, or cannot be opened: {error}",
                    self.root.join(&relative).display()
                ))
            })?;
        let mut bytes = String::new();
        let _bytes_read = file.read_to_string(&mut bytes).map_err(|error| {
            Error::Custom(format!(
                "include resolver: cannot read `{}`: {error}",
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

#[cfg(unix)]
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

#[cfg(not(unix))]
fn open_root_capability(root: &std::path::Path) -> std::io::Result<RootCapability> {
    Ok(RootCapability::Ready {
        canonical_root: std::fs::canonicalize(root)?,
    })
}

#[cfg(unix)]
fn open_from_root(
    capability: &RootCapability,
    relative: &std::path::Path,
    policy: SymlinkPolicy,
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

#[cfg(all(unix, target_vendor = "apple"))]
fn current_root_path(root: &std::fs::File) -> std::io::Result<std::path::PathBuf> {
    use std::os::unix::ffi::OsStringExt as _;

    let path = rustix::fs::getpath(root)?;
    Ok(std::path::PathBuf::from(std::ffi::OsString::from_vec(
        path.into_bytes(),
    )))
}

#[cfg(all(unix, any(target_os = "linux", target_os = "android")))]
fn current_root_path(root: &std::fs::File) -> std::io::Result<std::path::PathBuf> {
    use std::os::fd::AsRawFd as _;

    std::fs::read_link(format!("/proc/self/fd/{}", root.as_raw_fd()))
}

#[cfg(all(
    unix,
    not(any(target_vendor = "apple", target_os = "linux", target_os = "android"))
))]
fn current_root_path(_root: &std::fs::File) -> std::io::Result<std::path::PathBuf> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "the target cannot recover a path from a directory handle",
    ))
}

#[cfg(not(unix))]
fn open_from_root(
    capability: &RootCapability,
    relative: &std::path::Path,
    policy: SymlinkPolicy,
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

#[cfg(unix)]
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

#[cfg(not(unix))]
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
