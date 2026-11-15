// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Coverage for `de.rs`'s `!include` recursion error edges and
//! `include.rs`'s filesystem read-error path.
//!
//! Assert error variants to distinguish the depth cap from the cycle guard.

#![cfg(feature = "include")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use noyalib::include::{IncludeRequest, IncludeResolver, InputSource};
use noyalib::{Error, ErrorKind, ParserConfig, Result, Value, from_str_with_config};

/// Distinct child specs reach the depth cap without triggering cycle detection.
fn ever_deeper_resolver() -> IncludeResolver {
    let counter = Arc::new(AtomicUsize::new(0));
    IncludeResolver::new(move |_req: IncludeRequest<'_>| -> Result<InputSource> {
        let n = counter.fetch_add(1, Ordering::Relaxed);
        // Each level references a freshly-named child spec.
        Ok(InputSource::new(
            format!("gen-{n}"),
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
    // The failing include must propagate through the custom tag's inner mapping.
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
    // A failing include must propagate through a sequence element.
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

/// The unreadable-input path in `SafeFileResolver`: a path that resolves
/// successfully but cannot be consumed as a file because it is a directory.
/// Unix may reject the later read while Windows may reject the initial open;
/// both must retain the same public error classification.
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
