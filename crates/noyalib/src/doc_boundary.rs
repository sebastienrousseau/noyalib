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

use crate::error::{BudgetBreach, Error, Result};
use crate::prelude::Vec;

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
    marker_at_or_after(bytes, start.max(1))
}

/// Search for a document marker at or after `start`.
fn marker_at_or_after(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 3 <= bytes.len() {
        if is_doc_marker_at(bytes, i) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// A fallible, allocation-free stream of document slices.
///
/// Boundaries are discovered only as the consumer requests another item. This
/// keeps marker-heavy inputs from allocating storage proportional to the
/// document count and lets parallel consumers bound in-flight work to their
/// worker count.
pub(crate) struct DocumentStream<'a> {
    input: &'a str,
    pending: Option<(usize, usize)>,
    current_start: Option<usize>,
    search_from: usize,
    max_documents: usize,
    yielded: usize,
    exhausted: bool,
}

impl<'a> DocumentStream<'a> {
    /// Construct a stream that reports the first document beyond
    /// `max_documents` as a typed budget error.
    #[must_use]
    pub(crate) fn new(input: &'a str, max_documents: usize) -> Self {
        let first_marker = marker_at_or_after(input.as_bytes(), 0);
        let (pending, current_start, search_from) = match first_marker {
            None if input.trim().is_empty() => (None, None, input.len()),
            None => (Some((0, input.len())), None, input.len()),
            Some(marker) if marker > 0 && prologue_has_content(&input[..marker]) => {
                (Some((0, marker)), Some(marker), marker + 3)
            }
            Some(marker) => (None, Some(0), marker + 3),
        };

        Self {
            input,
            pending,
            current_start,
            search_from,
            max_documents,
            yielded: 0,
            exhausted: false,
        }
    }

    fn emit(&mut self, start: usize, end: usize) -> Result<&'a str> {
        let observed = self.yielded.saturating_add(1);
        if observed > self.max_documents {
            self.exhausted = true;
            return Err(Error::Budget(BudgetBreach::MaxDocuments {
                limit: self.max_documents,
                observed,
            }));
        }
        self.yielded = observed;
        Ok(&self.input[start..end])
    }
}

impl<'a> Iterator for DocumentStream<'a> {
    type Item = Result<&'a str>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        if let Some((start, end)) = self.pending.take() {
            return Some(self.emit(start, end));
        }

        let Some(start) = self.current_start else {
            self.exhausted = true;
            return None;
        };
        if let Some(marker) = marker_at_or_after(self.input.as_bytes(), self.search_from) {
            self.current_start = Some(marker);
            self.search_from = marker + 3;
            return Some(self.emit(start, marker));
        }

        self.current_start = None;
        self.exhausted = true;
        let trailing = &self.input[start..];
        if start < self.input.len() && !trailing.trim_end().is_empty() {
            return Some(self.emit(start, self.input.len()));
        }
        None
    }
}

impl core::iter::FusedIterator for DocumentStream<'_> {}

/// Validate the document-count budget without retaining markers or slices.
///
/// Parallel consumers use this before scheduling work so an oversized stream
/// cannot execute a valid prefix before the eventual budget error is known.
pub(crate) fn validate_document_budget(input: &str, max_documents: usize) -> Result<()> {
    for document in DocumentStream::new(input, max_documents) {
        let _ = document?;
    }
    Ok(())
}

/// Split `input` into per-document `&str` slices on column-0
/// `---` markers. Empty trailing slices are omitted.
///
/// A leading implicit document (content before the first `---`)
/// becomes the first slice; each subsequent slice starts at its
/// `---` marker (asymmetric trimming made the previous copies
/// disagree on offsets — keeping the marker is the convention
/// that round-trips cleanly through `from_str_with_config`).
#[must_use]
pub(crate) fn split_documents(input: &str) -> Vec<&str> {
    DocumentStream::new(input, usize::MAX)
        .map_while(Result::ok)
        .collect()
}

/// Split a stream while enforcing a document-count budget.
///
/// The scanner probes one marker beyond the configured limit so an
/// oversized stream is rejected rather than silently truncating the tail.
pub(crate) fn split_documents_checked(input: &str, max_documents: usize) -> Result<Vec<&str>> {
    DocumentStream::new(input, max_documents).collect()
}

/// Whether the text before the first marker is an implicit document.
/// Comments, blank lines, and directives belong to the explicit document
/// opened by that marker.
fn prologue_has_content(pre: &str) -> bool {
    pre.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('%')
    })
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
        let docs = DocumentStream::new("---\na: 1\n---\nb: 2\n", 16)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(docs, vec!["---\na: 1\n", "---\nb: 2\n"]);
    }

    #[test]
    fn crlf_terminated_markers_are_recognised() {
        // The previous copies in recovery/parallel/tokio_async
        // each missed at least one of these.
        let docs = DocumentStream::new("---\r\na: 1\r\n---\r\nb: 2\r\n", 16)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs.concat(), "---\r\na: 1\r\n---\r\nb: 2\r\n");
    }

    #[test]
    fn mid_line_dashes_are_not_markers() {
        let docs = DocumentStream::new("a: ---\nb: 2\n", 16)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(docs, vec!["a: ---\nb: 2\n"]);
    }

    #[test]
    fn marker_at_eof_is_recognised() {
        // `---` as the very last bytes — no terminator after.
        let docs = DocumentStream::new("a: 1\n---", 16)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(docs, vec!["a: 1\n", "---"]);
    }

    #[test]
    fn next_marker_skips_leading() {
        assert_eq!(next_marker_after(b"---\na: 1\n", 0), None);
        assert_eq!(next_marker_after(b"---\na: 1\n---\nb: 2\n", 0), Some(9));
    }

    #[test]
    fn checked_split_rejects_document_overflow() {
        let error = split_documents_checked("---\na: 1\n---\na: 2\n", 1).unwrap_err();
        assert!(matches!(
            error,
            Error::Budget(BudgetBreach::MaxDocuments {
                limit: 1,
                observed: 2
            })
        ));
    }

    #[test]
    fn comments_before_a_leading_marker_stay_with_the_first_document() {
        let docs = split_documents_checked("# heading\n---\na: 1\n", 1).unwrap();
        assert_eq!(docs, vec!["# heading\n---\na: 1\n"]);
    }

    #[test]
    fn document_stream_preserves_exact_partitions() {
        let cases = [
            "a: 1\n---\nb: 2\n",
            "# heading\n---\r\na: 1\r\n---\r\nb: 2\r\n",
            "---\n---\n---",
            "content\n---",
        ];
        for input in cases {
            let docs = DocumentStream::new(input, usize::MAX)
                .collect::<Result<Vec<_>>>()
                .unwrap();
            assert_eq!(docs.concat(), input);
        }
    }

    #[test]
    fn document_stream_reports_only_one_budget_error() {
        let mut docs = DocumentStream::new("---\na: 1\n---\na: 2\n", 1);
        assert!(docs.next().unwrap().is_ok());
        assert!(matches!(
            docs.next().unwrap(),
            Err(Error::Budget(BudgetBreach::MaxDocuments {
                limit: 1,
                observed: 2
            }))
        ));
        assert!(docs.next().is_none());
    }
}
