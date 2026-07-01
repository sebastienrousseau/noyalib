// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Workspace-private `---` document-boundary scanner.
//!
//! Three modules used to ship their own copy of this routine
//! ([`crate::parallel::split`], `crate::recovery::split_documents`,
//! `crate::tokio_async::find_doc_boundary`). They had subtly
//! different CRLF / leading-marker / trailing-marker semantics —
//! Windows-edited inputs round-tripped via one and not the other.
//!
//! This module centralises the scanner so the CRLF, BOM, and
//! `---`-spam DoS guards live in exactly one place.
//!
//! The scanner recognises a `---` document-start marker if and
//! only if all of:
//!
//! * the three bytes are at the start of input, or follow a
//!   `\n` or `\r\n` line break;
//! * the byte after the marker (if any) is whitespace
//!   (`\n`, `\r`, ` `, `\t`) or end-of-input.
//!
//! This matches the YAML 1.2.2 §9.1.2 `c-directives-end` grammar
//! and the strict-parser's own boundary detection.

#![allow(dead_code)]

use crate::prelude::{Vec, vec};

/// UTF-8 BOM byte sequence (`U+FEFF`).
pub(crate) const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Strip a leading UTF-8 BOM if present and return the remaining
/// byte offset. The offset is `0` when the BOM was absent and
/// `3` when it was present — never any other value.
#[inline]
#[must_use]
pub(crate) fn strip_bom(bytes: &[u8]) -> usize {
    if bytes.starts_with(&BOM) { 3 } else { 0 }
}

/// `true` if the byte at `i` opens a column-0 `---` directive-end
/// marker — i.e. the run `bytes[i..i+3]` is exactly `b"---"`
/// **and** the preceding byte (if any) is `\n` or `\r`
/// **and** the following byte (if any) is whitespace or end of
/// input.
///
/// Returns `false` for out-of-range `i` so callers can use it
/// unchecked inside scan loops.
#[inline]
#[must_use]
pub(crate) fn is_doc_marker_at(bytes: &[u8], i: usize) -> bool {
    if i + 3 > bytes.len() {
        return false;
    }
    if &bytes[i..i + 3] != b"---" {
        return false;
    }
    let preceded_by_break = i == 0 || matches!(bytes[i - 1], b'\n' | b'\r');
    if !preceded_by_break {
        return false;
    }
    if i + 3 == bytes.len() {
        return true;
    }
    matches!(bytes[i + 3], b'\n' | b'\r' | b' ' | b'\t')
}

/// Collect every column-0 `---` marker offset in `bytes`, bounded
/// by `max_markers` so a hostile `---`-spam input cannot drive
/// unbounded `Vec` growth.
///
/// On overflow the scan stops at `max_markers` markers and the
/// returned `Vec` has exactly that length. Callers that want a
/// hard error on overflow should compare `out.len() == max_markers`
/// and decide whether the input is suspicious.
///
/// `max_markers == 0` yields an empty `Vec` without scanning.
#[must_use]
pub(crate) fn scan_markers(bytes: &[u8], max_markers: usize) -> Vec<usize> {
    let mut out = Vec::new();
    if max_markers == 0 {
        return out;
    }
    let mut i = 0;
    while i + 3 <= bytes.len() {
        if is_doc_marker_at(bytes, i) {
            out.push(i);
            if out.len() >= max_markers {
                break;
            }
            i += 3;
            continue;
        }
        i += 1;
    }
    out
}

/// Search `bytes` for the **next** column-0 `---` marker starting
/// at offset `start`. Returns the offset of the first byte of the
/// marker, or `None` when no marker is present.
///
/// `start == 0` does **not** match a leading `---` — that's the
/// start of the first document, not a boundary between two of
/// them. Use [`is_doc_marker_at`] directly if a leading marker
/// must be considered.
#[must_use]
pub(crate) fn next_marker_after(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start.max(1);
    while i + 3 <= bytes.len() {
        if is_doc_marker_at(bytes, i) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Split `input` into per-document `&str` slices on column-0
/// `---` markers, bounded by `max_markers` to defeat `---`-spam
/// inputs. Empty trailing slices are omitted.
///
/// A leading implicit document (content before the first `---`)
/// becomes the first slice; each subsequent slice starts at its
/// `---` marker (asymmetric trimming made the previous copies
/// disagree on offsets — keeping the marker is the convention
/// that round-trips cleanly through `from_str_with_config`).
#[must_use]
pub(crate) fn split_documents(input: &str, max_markers: usize) -> Vec<&str> {
    let bytes = input.as_bytes();
    let markers = scan_markers(bytes, max_markers);

    if markers.is_empty() {
        return if input.trim().is_empty() {
            Vec::new()
        } else {
            vec![input]
        };
    }

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

    #[test]
    fn bom_strip_round_trip() {
        assert_eq!(strip_bom(b""), 0);
        assert_eq!(strip_bom(b"a: 1\n"), 0);
        assert_eq!(strip_bom(b"\xEF\xBB\xBFa: 1\n"), 3);
    }

    #[test]
    fn lf_terminated_markers() {
        let m = scan_markers(b"---\na: 1\n---\nb: 2\n", 16);
        assert_eq!(m, vec![0, 9]);
    }

    #[test]
    fn crlf_terminated_markers_are_recognised() {
        // The previous copies in recovery/parallel/tokio_async
        // each missed at least one of these.
        let m = scan_markers(b"---\r\na: 1\r\n---\r\nb: 2\r\n", 16);
        assert_eq!(m, vec![0, 11]);
    }

    #[test]
    fn mid_line_dashes_are_not_markers() {
        let m = scan_markers(b"a: ---\nb: 2\n", 16);
        assert!(m.is_empty());
    }

    #[test]
    fn marker_at_eof_is_recognised() {
        // `---` as the very last bytes — no terminator after.
        let m = scan_markers(b"a: 1\n---", 16);
        assert_eq!(m, vec![5]);
    }

    #[test]
    fn marker_cap_truncates() {
        let input = b"---\n---\n---\n---\n---\n";
        let m = scan_markers(input, 2);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn next_marker_skips_leading() {
        assert_eq!(next_marker_after(b"---\na: 1\n", 0), None);
        assert_eq!(next_marker_after(b"---\na: 1\n---\nb: 2\n", 0), Some(9));
    }

    #[test]
    fn marker_cap_zero_yields_empty() {
        assert!(scan_markers(b"---\n---\n", 0).is_empty());
    }
}
