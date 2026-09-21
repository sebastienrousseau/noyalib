// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Atomic batches of byte-range CST edits.

use super::{Document, RepairScope};
use crate::error::{Error, Result};
use crate::prelude::*;

#[derive(Debug)]
struct PlannedEdit {
    start: usize,
    end: usize,
    replacement: String,
}

/// A deferred, atomic batch of byte-range edits to a [`Document`].
///
/// Ranges refer to the unchanged source exposed by [`Self::source`]. Edits
/// may be queued in any order, but their ranges must not overlap and two
/// edits cannot begin at the same byte. The target document is borrowed but
/// not modified until [`Self::commit`] validates the assembled source as one
/// YAML document.
///
/// The session intentionally exposes no typed reads. Resolve every required
/// span before starting the session, queue the replacements, and commit once.
/// This avoids the repeated complete-document validation required by
/// independent [`Document::replace_span`] calls.
#[derive(Debug)]
#[must_use = "dropping an EditSession aborts its queued edits"]
pub struct EditSession<'a> {
    document: &'a mut Document,
    edits: Vec<PlannedEdit>,
}

impl<'a> EditSession<'a> {
    pub(super) fn new(document: &'a mut Document) -> Self {
        Self {
            document,
            edits: Vec::new(),
        }
    }

    /// Borrow the unchanged source whose byte offsets this session uses.
    #[must_use]
    pub fn source(&self) -> &str {
        self.document.source()
    }

    /// Return the number of replacements queued for commit.
    #[must_use]
    pub fn len(&self) -> usize {
        self.edits.len()
    }

    /// Return `true` when no replacements are queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Queue a replacement measured against [`Self::source`].
    ///
    /// Insertions use an empty range (`start == end`). Adjacent ranges are
    /// accepted, while overlapping ranges and multiple edits beginning at the
    /// same byte are rejected because their application order would be
    /// ambiguous.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parse`] when the range is out of bounds, is not on
    /// UTF-8 character boundaries, or conflicts with an already queued edit.
    /// YAML syntax is validated once, by [`Self::commit`].
    pub fn replace_span(
        &mut self,
        start: usize,
        end: usize,
        replacement: impl Into<String>,
    ) -> Result<()> {
        let source = self.source();
        if start > end || end > source.len() {
            return Err(Error::Parse(format!(
                "edit-session range {start}..{end} out of bounds (source length {})",
                source.len()
            )));
        }
        if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
            return Err(Error::Parse(format!(
                "edit-session range {start}..{end} is not a character boundary"
            )));
        }
        if self
            .edits
            .iter()
            .any(|queued| queued.start == start || (start < queued.end && queued.start < end))
        {
            return Err(Error::Parse(format!(
                "edit-session range {start}..{end} conflicts with a queued edit"
            )));
        }
        self.edits.push(PlannedEdit {
            start,
            end,
            replacement: replacement.into(),
        });
        Ok(())
    }

    /// Validate and atomically commit every queued replacement.
    ///
    /// The candidate is assembled without mutating the target document, then
    /// validated and rebuilt as coordinated source, green-tree, value, and
    /// span views. A parse or budget error leaves the target unchanged.
    ///
    /// # Errors
    ///
    /// Returns the parser error produced by the complete candidate document,
    /// including an error when the edits introduce a second YAML document.
    pub fn commit(mut self) -> Result<()> {
        if self.edits.is_empty() {
            return Ok(());
        }

        self.edits.sort_unstable_by_key(|edit| edit.start);
        let source = self.document.source();
        let final_len = self.edits.iter().try_fold(source.len(), |length, edit| {
            length
                .checked_sub(edit.end - edit.start)
                .and_then(|length| length.checked_add(edit.replacement.len()))
        });
        let Some(final_len) = final_len else {
            return Err(Error::Parse(
                "edit-session candidate length exceeds addressable memory".into(),
            ));
        };

        let mut candidate = String::with_capacity(final_len);
        let mut cursor = 0;
        for edit in &self.edits {
            candidate.push_str(&source[cursor..edit.start]);
            candidate.push_str(&edit.replacement);
            cursor = edit.end;
        }
        candidate.push_str(&source[cursor..]);

        self.document
            .commit_source(&candidate, RepairScope::Document)
    }

    /// Abort the session and leave the target document unchanged.
    pub fn abort(self) {}
}
