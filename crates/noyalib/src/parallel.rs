// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Parallel multi-document YAML parsing — the "MapReduce" path.
//!
//! For massive multi-document streams (telemetry logs, audit
//! exports, Kubernetes-resource snapshots, anything emitting `---`-
//! separated documents at scale), even the fastest single-threaded
//! parser is bounded by one CPU core. This module pre-scans the
//! input on the main thread, splits it into per-document slices,
//! then dispatches each document to a Rayon worker.
//!
//! Gated behind the `parallel` Cargo feature.
//!
//! # Linear scaling
//!
//! The pre-scan runs in `O(input_len)` with no allocation; the
//! parse-per-document work is the dominant cost and parallelises
//! naturally across cores. Expect near-linear speedup with the
//! number of cores up to the point where document size starts to
//! dominate (very large single documents see less benefit because
//! one document still parses on one thread).
//!
//! # Document-boundary contract
//!
//! The pre-scanner recognises `---` document-start markers that
//! begin at column 0 and are followed by `\n`, `\r`, ` `, `\t`, or
//! end-of-input. This matches the YAML 1.2.2 §9.1.2 grammar for
//! `c-directives-end`. The scanner does **not** recognise:
//!
//! - `---` inside a literal (`|`) or folded (`>`) block scalar
//!   that is column-0-aligned (extremely rare in practice; the
//!   YAML spec does not actually permit such a literal because
//!   block scalars must indent past the parent).
//! - `...` document-end markers — they are advisory in YAML 1.2,
//!   and the document-start scan picks up the next document
//!   anyway.
//!
//! Inputs that violate the column-0 rule fall back to the
//! conservative single-document slice (everything before the next
//! valid `---` is treated as one document).
//!
//! # Examples
//!
//! ```
//! # #[cfg(feature = "parallel")] {
//! let yaml = "---\nid: 1\n---\nid: 2\n---\nid: 3\n";
//! #[derive(serde::Deserialize, Debug)]
//! struct Record { id: u32 }
//! let records: Vec<Record> = noyalib::parallel::parse(yaml).unwrap();
//! assert_eq!(records.len(), 3);
//! # }
//! ```
//!
//! # API shape
//!
//! - [`parse`](crate::parallel::parse) — typed deserialise into `Vec<T>`.
//! - [`values`](crate::parallel::values) — dynamic-tree variant returning `Vec<Value>`.
//! - [`split`](crate::parallel::split) — standalone document-boundary
//!   pre-scanner for callers driving their own concurrency primitives.
//!
//! Names are kept short on purpose — the `parallel` namespace
//! already encodes the concurrency contract, so the function
//! verb stays single-word: `parallel::parse` reads as one
//! sentence.

use crate::error::Result;
use rayon::prelude::*;
use serde::de::DeserializeOwned;

/// Deserialise every YAML document in `input` into `T`, parsing
/// in parallel via Rayon's global thread pool.
///
/// `T` must be `Send` because the per-document parses run on
/// arbitrary worker threads. The result is collected back into a
/// `Vec<T>` in document order.
///
/// Reads as `noyalib::parallel::parse::<MyType>(input)` —
/// concurrency lives in the namespace, the verb is one word.
///
/// # Errors
///
/// - Any document fails to parse — the first error is returned;
///   sibling documents may still complete in parallel but their
///   results are discarded.
///
/// # Examples
///
/// ```
/// let yaml = "---\nport: 80\n---\nport: 443\n";
/// #[derive(serde::Deserialize, Debug, PartialEq)]
/// struct Service { port: u16 }
/// let v: Vec<Service> = noyalib::parallel::parse(yaml).unwrap();
/// assert_eq!(v[0].port, 80);
/// assert_eq!(v[1].port, 443);
/// ```
pub fn parse<T>(input: &str) -> Result<Vec<T>>
where
    T: DeserializeOwned + Send + 'static,
{
    let chunks = split(input);
    chunks
        .par_iter()
        .map(|chunk| crate::from_str::<T>(chunk))
        .collect::<Result<Vec<T>>>()
}

/// Dynamic-tree variant of [`parse`]: returns a
/// [`Vec<crate::Value>`]. Use when the caller wants to route
/// documents to different typed handlers post-parse.
///
/// # Examples
///
/// ```
/// use noyalib::Value;
/// let yaml = "---\na: 1\n---\nb: 2\n";
/// let docs: Vec<Value> = noyalib::parallel::values(yaml).unwrap();
/// assert_eq!(docs.len(), 2);
/// assert_eq!(docs[0]["a"].as_i64(), Some(1));
/// assert_eq!(docs[1]["b"].as_i64(), Some(2));
/// ```
pub fn values(input: &str) -> Result<Vec<crate::Value>> {
    parse::<crate::Value>(input)
}

/// Split `input` into per-document byte slices on YAML 1.2 `---`
/// markers. Single-pass `O(input.len())`. Public so callers that
/// drive their own concurrency primitives (async tasks, custom
/// thread pools) can reuse the same boundary scan.
///
/// # Examples
///
/// ```
/// let docs = noyalib::parallel::split("---\na: 1\n---\nb: 2\n");
/// assert_eq!(docs.len(), 2);
/// ```
#[must_use]
pub fn split(input: &str) -> Vec<&str> {
    let bytes = input.as_bytes();
    let mut markers: Vec<usize> = Vec::new();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let at_line_start = i == 0 || bytes[i - 1] == b'\n' || bytes[i - 1] == b'\r';
        if at_line_start && &bytes[i..i + 3] == b"---" {
            let next_ok =
                i + 3 >= bytes.len() || matches!(bytes[i + 3], b'\n' | b'\r' | b' ' | b'\t');
            if next_ok {
                markers.push(i);
                // Skip past the marker to avoid re-matching it on
                // the next iteration.
                i += 3;
                continue;
            }
        }
        i += 1;
    }

    if markers.is_empty() {
        // No document marker — treat the whole input as one
        // document. Skip the empty case.
        return if input.is_empty() {
            Vec::new()
        } else {
            vec![input]
        };
    }

    // Build slices between successive markers. Anything before the
    // first marker is also a document (it's the preamble of an
    // implicit first doc when the input starts with content
    // followed by `---`).
    let mut docs: Vec<&str> = Vec::with_capacity(markers.len() + 1);
    if markers[0] > 0 {
        let pre = input[..markers[0]].trim();
        if !pre.is_empty() {
            docs.push(&input[..markers[0]]);
        }
    }
    for window in markers.windows(2) {
        docs.push(&input[window[0]..window[1]]);
    }
    let last = *markers.last().unwrap();
    if last < input.len() {
        let trailing = &input[last..];
        if !trailing.trim_end().is_empty() {
            docs.push(trailing);
        }
    }
    docs
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn split_separates_three_records() {
        let yaml = "---\nid: 1\n---\nid: 2\n---\nid: 3\n";
        let docs = split(yaml);
        assert_eq!(docs.len(), 3);
        assert!(docs[0].contains("id: 1"));
        assert!(docs[1].contains("id: 2"));
        assert!(docs[2].contains("id: 3"));
    }

    #[test]
    fn split_handles_no_separators() {
        let yaml = "single: doc\n";
        let docs = split(yaml);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0], yaml);
    }

    #[test]
    fn split_handles_empty_input() {
        assert!(split("").is_empty());
    }

    #[test]
    fn split_handles_implicit_first_doc() {
        // Input starts with content, then `---`, then another doc.
        // The preamble is the implicit first document.
        let yaml = "name: a\n---\nname: b\n";
        let docs = split(yaml);
        assert_eq!(docs.len(), 2);
        assert!(docs[0].contains("name: a"));
        assert!(docs[1].contains("name: b"));
    }

    #[test]
    fn split_ignores_dashes_mid_line() {
        // `---` not at column 0 is part of a scalar, not a marker.
        let yaml = "key: value---suffix\n";
        let docs = split(yaml);
        assert_eq!(docs.len(), 1);
        assert!(docs[0].contains("value---suffix"));
    }

    #[test]
    fn split_requires_post_marker_whitespace() {
        // `---foo` is *not* a document start — the marker must be
        // followed by whitespace or end-of-input. (This is a
        // simplification of the spec but matches every real YAML
        // emitter we've encountered.)
        let yaml = "key: a\n---foo\nkey: b\n";
        let docs = split(yaml);
        assert_eq!(docs.len(), 1, "got: {docs:?}");
    }

    // Rayon's global thread pool pulls in `crossbeam-epoch`, whose
    // epoch-based reclamation performs integer-to-pointer casts that
    // Miri rejects under `-Zmiri-strict-provenance` (see the weekly
    // `soak-miri` job). The provenance issue is entirely inside the
    // third-party dep — noyalib's own call sites stay
    // `#![forbid(unsafe_code)]` — so the rayon-invoking tests are
    // skipped under Miri while every other test still runs. Mirrors
    // `scripts/miri.sh`, which excludes rayon from the focused sweep
    // for the same reason.
    #[cfg_attr(
        miri,
        ignore = "rayon/crossbeam-epoch uses int-to-ptr casts unsupported under -Zmiri-strict-provenance"
    )]
    #[test]
    fn parse_round_trips_typed_records() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Record {
            id: u32,
        }
        let yaml = "---\nid: 1\n---\nid: 2\n---\nid: 3\n";
        let records: Vec<Record> = parse(yaml).unwrap();
        assert_eq!(
            records,
            vec![Record { id: 1 }, Record { id: 2 }, Record { id: 3 }]
        );
    }

    #[cfg_attr(
        miri,
        ignore = "rayon/crossbeam-epoch uses int-to-ptr casts unsupported under -Zmiri-strict-provenance"
    )]
    #[test]
    fn values_yields_value_per_document() {
        let yaml = "---\na: 1\n---\nb: 2\n";
        let docs = values(yaml).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0]["a"].as_i64(), Some(1));
        assert_eq!(docs[1]["b"].as_i64(), Some(2));
    }

    #[cfg_attr(
        miri,
        ignore = "rayon/crossbeam-epoch uses int-to-ptr casts unsupported under -Zmiri-strict-provenance"
    )]
    #[test]
    fn parse_propagates_first_error() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Record {
            id: u32,
        }
        // Second doc has unparseable content.
        let yaml = "---\nid: 1\n---\nid: [\n";
        let res: Result<Vec<Record>> = parse(yaml);
        assert!(res.is_err());
    }

    #[cfg_attr(
        miri,
        ignore = "rayon/crossbeam-epoch uses int-to-ptr casts unsupported under -Zmiri-strict-provenance"
    )]
    #[test]
    fn parse_matches_sequential_for_correctness() {
        // Stress test: 50 small documents. The parallel and
        // sequential implementations must produce bit-for-bit
        // identical results.
        let mut yaml = String::new();
        for i in 0..50 {
            yaml.push_str(&format!("---\nid: {i}\nname: record-{i}\n"));
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Record {
            id: u32,
            name: String,
        }
        let parallel: Vec<Record> = parse(&yaml).unwrap();
        let sequential: Vec<Record> = crate::load_all_as(&yaml).unwrap();
        assert_eq!(parallel, sequential);
    }
}
