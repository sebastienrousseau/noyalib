// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for `de.rs`'s `!include` recursion error edges and
//! `include.rs`'s filesystem read-error path.
//!
//! These close gaps the existing `include_directive.rs` suite leaves
//! open because its assertions only check `is_err()` without pinning
//! the *variant* — e.g. `max_include_depth_caps_recursion` reuses the
//! spec `"infinite"`, so it trips the **cycle** guard and never reaches
//! the **depth** guard (`de.rs` line 607). Each test here asserts the
//! specific failure so the intended region is actually exercised.

#![cfg(feature = "include")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use noyalib::include::{IncludeRequest, IncludeResolver, InputSource};
use noyalib::{Error, ErrorKind, ParserConfig, Result, Value, from_str_with_config};

/// A depth-blowup resolver that hands back a **distinct** spec on
/// every call, so the per-walk `visited` cycle guard never fires and
/// resolution can only be stopped by the depth cap. This is what makes
/// the recursion reach `de.rs`'s `RecursionLimitExceeded` branch (607)
/// rather than the cycle branch (620) the existing test hits.
fn ever_deeper_resolver() -> IncludeResolver {
    let counter = Arc::new(AtomicUsize::new(0));
    IncludeResolver::new(move |_req: IncludeRequest<'_>| -> Result<InputSource> {
        let n = counter.fetch_add(1, Ordering::Relaxed);
        // Each level references a freshly-named child spec.
        Ok(InputSource::new(
            "gen",
            format!("deeper: !include level_{}\n", n + 1),
        ))
    })
}

#[test]
fn depth_cap_yields_recursion_limit_not_cycle() {
    let cfg = ParserConfig::new()
        .include_resolver(ever_deeper_resolver())
        .max_include_depth(4);
    let res: Result<Value> = from_str_with_config("root: !include level_0\n", &cfg);
    let err = res.unwrap_err();
    // The distinguishing assertion: this must be the *depth* guard,
    // not the cycle guard. Cycle errors are `Error::Custom`
    // (ErrorKind::Other); the depth guard is `RecursionLimitExceeded`
    // (ErrorKind::Budget).
    assert!(
        matches!(err, Error::RecursionLimitExceeded { .. }),
        "expected RecursionLimitExceeded, got {err:?}"
    );
    assert_eq!(err.kind(), ErrorKind::Budget);
}

#[test]
fn error_propagates_through_non_include_tagged_wrapper() {
    // A `!custom`-tagged node whose *inner* value contains an
    // `!include` that fails. The walker takes the non-`!include`
    // Tagged branch (`de.rs` line 672), recurses into the wrapped
    // mapping, and the failing include's error must propagate back up
    // through the `?` at line 682.
    let resolver = IncludeResolver::new(|req: IncludeRequest<'_>| -> Result<InputSource> {
        Err(Error::Custom(format!("boom for `{}`", req.spec)))
    });
    let cfg = ParserConfig::new().include_resolver(resolver);
    let yaml = "wrapped: !custom\n  inner: !include child.yaml\n";
    let res: Result<Value> = from_str_with_config(yaml, &cfg);
    let err = res.unwrap_err();
    assert!(
        err.to_string().contains("boom for `child.yaml`"),
        "inner include error must surface through the tagged wrapper: {err}"
    );
}

#[test]
fn error_propagates_through_sequence_element() {
    // The OK sequence path is covered elsewhere; this pins the
    // Err-propagation edge of the sequence arm (`de.rs` line 696):
    // a sequence element that is a failing `!include`.
    let resolver = IncludeResolver::new(|req: IncludeRequest<'_>| -> Result<InputSource> {
        Err(Error::Custom(format!("seq boom for `{}`", req.spec)))
    });
    let cfg = ParserConfig::new().include_resolver(resolver);
    let yaml = "items:\n  - ok\n  - !include broken.yaml\n";
    let res: Result<Value> = from_str_with_config(yaml, &cfg);
    let err = res.unwrap_err();
    assert!(
        err.to_string().contains("seq boom for `broken.yaml`"),
        "failing include inside a sequence must propagate: {err}"
    );
}

/// The read-error path in `include.rs` (`SafeFileResolver`): a path
/// that canonicalises successfully but cannot be read as a file —
/// i.e. it is a **directory**. Exercises lines 289-294 (`fs::read_to_string`
/// error closure), which the "missing file" tests never reach because
/// canonicalisation fails first for a non-existent path.
#[cfg(feature = "include_fs")]
#[test]
fn including_a_directory_surfaces_read_error() {
    use noyalib::include::SafeFileResolver;

    let root = std::env::temp_dir().join("noyalib-cov-de-include-dir");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // `sub` is a directory *inside* root — canonicalises fine, but is
    // not a readable file.
    std::fs::create_dir_all(root.join("sub")).unwrap();

    let cfg = ParserConfig::new().include_resolver(SafeFileResolver::new(&root).into_resolver());
    let res: Result<Value> = from_str_with_config("x: !include sub\n", &cfg);
    let err = res.unwrap_err();
    assert!(
        err.to_string().contains("cannot read"),
        "reading a directory must yield the resolver read-error: {err}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
