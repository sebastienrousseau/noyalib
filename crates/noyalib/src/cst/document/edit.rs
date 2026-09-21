// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Atomic source splicing and local green-tree repair.

use super::{Document, RepairScope, entry_indent_column, walk_tokens};
use crate::cst::builder::{SubtreeContext, parse_subtree, rebuild_with_splice};
use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::syntax::SyntaxKind;
use crate::error::{Error, Result};
use crate::prelude::*;

impl Document {
    /// Replace the bytes in `start..end` with `replacement` and
    /// re-parse. The caller is responsible for `replacement` being a
    /// syntactically valid fragment in that position; if the spliced
    /// source fails to parse, the original document is left
    /// unchanged and the parse error is returned.
    ///
    /// # Errors
    ///
    /// - `Error::Parse` if the resulting source is not valid YAML.
    /// - `Error::Parse` if `start..end` is out of bounds or not a
    ///   character boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("a: 1\n").unwrap();
    /// let (s, e) = doc.span_at("a").unwrap();
    /// doc.replace_span(s, e, "42").unwrap();
    /// assert_eq!(doc.to_string(), "a: 42\n");
    /// ```
    pub fn replace_span(&mut self, start: usize, end: usize, replacement: &str) -> Result<()> {
        #[cfg(test)]
        if super::fault::splice_should_fail() {
            return Err(Error::Parse("injected splice failure".into()));
        }
        if start > end || end > self.source.len() {
            return Err(Error::Parse(format!(
                "replace_span range {start}..{end} out of bounds (source length {})",
                self.source.len()
            )));
        }
        if !self.source.is_char_boundary(start) || !self.source.is_char_boundary(end) {
            return Err(Error::Parse(format!(
                "replace_span range {start}..{end} is not a character boundary"
            )));
        }
        let mut new_source =
            String::with_capacity(self.source.len() - (end - start) + replacement.len());
        new_source.push_str(&self.source[..start]);
        new_source.push_str(replacement);
        new_source.push_str(&self.source[end..]);

        // A local green-tree repair avoids rebuilding unchanged CST
        // nodes, but it is not a document-level validity proof. Parse
        // the complete source before committing so `Ok(())` can never
        // leave a `Document` whose next typed read panics.
        let new_arc: Arc<str> = Arc::from(new_source.as_str());
        if let Some((new_green, scope)) =
            self.try_local_repair_green(start, end, replacement, &new_source)
        {
            let parsed = crate::parser::parse_exactly_one(&new_source, &self.config)?;
            self.last_repair_scope.set(Some(scope));
            self.source = new_arc;
            self.green = new_green;
            let _ = self.cache.replace(Some(parsed));
            return Ok(());
        }

        // Safety net — full re-parse. Validates the new source and
        // populates everything eagerly.
        self.commit_source(&new_source, RepairScope::Document)
    }

    /// Attempt to repair the green tree locally for the edit
    /// `[start, end) → replacement`. Returns the new tree and the
    /// scope that was successfully repaired, or `None` if escalation
    /// to a full re-parse is required. Pure — does not mutate
    /// `self`.
    fn try_local_repair_green(
        &self,
        start: usize,
        end: usize,
        replacement: &str,
        new_source: &str,
    ) -> Option<(GreenNode, RepairScope)> {
        // Shape guard: any anchor / alias / tag in the affected
        // region forces a Document-scope re-parse so we do not have
        // to reason about cross-document name resolution.
        if region_has_anchor_alias_or_tag(&self.green, start, end)
            || replacement_introduces_anchor_alias_or_tag(replacement)
        {
            return None;
        }

        let delta = replacement.len() as isize - (end as isize - start as isize);
        let candidates = ancestor_candidates(&self.green, start, end);

        // Flow content is kept flat in the green tree, so re-parsing a
        // block ancestor does not validate the structure of a flow
        // collection the edit landed in: `{a: x {y} z, b: 2}` passed the
        // sub-parse and was committed as the document's source (#332).
        // Only the full parse checks flow structure, so escalate to it.
        if candidates
            .iter()
            .any(|c| matches!(c.kind, SyntaxKind::FlowMapping | SyntaxKind::FlowSequence))
        {
            return None;
        }

        for cand in &candidates {
            // Phase A only owns block-collection and block-entry
            // re-parses. Other kinds (scalars, flow collections)
            // are handled by climbing to an ancestor that this
            // ladder rung does support.
            if !is_phase_a_repairable(cand.kind) {
                continue;
            }

            let n_old_start = cand.start;
            let n_old_end = cand.end;
            let n_new_start = n_old_start; // pre-edit start, by construction
            let n_new_end_signed = n_old_end as isize + delta;
            if n_new_end_signed < n_new_start as isize {
                continue;
            }
            let n_new_end = n_new_end_signed as usize;
            // Defensive: make sure the slice is in bounds.
            if n_new_end > new_source.len() {
                continue;
            }
            let fragment = &new_source[n_new_start..n_new_end];
            let indent = entry_indent_column(&self.source, n_old_start);
            let ctx = SubtreeContext::block_at(indent);

            match parse_subtree(fragment, ctx, cand.kind) {
                Ok(new_sub)
                    if new_sub.kind() == cand.kind && new_sub.text_len() == fragment.len() =>
                {
                    let new_root =
                        rebuild_with_splice(&self.green, n_old_start, n_old_end, new_sub);
                    return Some((new_root, scope_for_kind(cand.kind)));
                }
                Ok(_) | Err(_) => {
                    // Shape inversion (kind mismatch), partial
                    // coverage (text_len mismatch — the fragment
                    // spans into sibling territory), or a sub-parse
                    // error. Either way: climb the ladder.
                    continue;
                }
            }
        }
        None
    }
}

fn scope_for_kind(kind: SyntaxKind) -> RepairScope {
    match kind {
        SyntaxKind::MappingEntry | SyntaxKind::SequenceItem => RepairScope::Entry,
        SyntaxKind::BlockMapping
        | SyntaxKind::BlockSequence
        | SyntaxKind::FlowMapping
        | SyntaxKind::FlowSequence => RepairScope::Collection,
        _ => RepairScope::Document,
    }
}

fn is_phase_a_repairable(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::BlockMapping
            | SyntaxKind::BlockSequence
            | SyntaxKind::MappingEntry
            | SyntaxKind::SequenceItem
    )
}

/// One candidate ancestor for the smallest-scope repair walk.
struct Candidate {
    kind: SyntaxKind,
    start: usize,
    end: usize,
}

/// Walk the green tree once and collect every node ancestor of the
/// edit span `[start, end)`, smallest-first. The Document root is
/// implicitly the last entry — left out here because it always
/// triggers escalation.
fn ancestor_candidates(root: &GreenNode, start: usize, end: usize) -> Vec<Candidate> {
    let mut out = Vec::new();
    collect_ancestors(root, start, end, 0, &mut out);
    // `collect_ancestors` pushes outermost-first; reverse so the
    // smallest scope is tried first.
    out.reverse();
    out
}

fn collect_ancestors(
    node: &GreenNode,
    start: usize,
    end: usize,
    base: usize,
    out: &mut Vec<Candidate>,
) {
    let node_end = base + node.text_len();
    if start >= base && end <= node_end {
        // This node fully contains the edit; record it.
        out.push(Candidate {
            kind: node.kind(),
            start: base,
            end: node_end,
        });
        // Recurse into the containing child.
        let mut pos = base;
        for child in node.children() {
            let len = child.text_len();
            let child_end = pos + len;
            if start >= pos && end <= child_end {
                if let GreenChild::Node(inner) = child {
                    collect_ancestors(inner, start, end, pos, out);
                }
                break;
            }
            pos += len;
        }
    }
}

/// `true` when source bytes in `[start, end)` contain an anchor
/// (`&`), alias (`*`), or tag (`!`) lexeme. Edits overlapping
/// these are escalated to a full re-parse — we do not reason about
/// cross-document name resolution after a localised splice.
fn region_has_anchor_alias_or_tag(root: &GreenNode, start: usize, end: usize) -> bool {
    let mut found = false;
    walk_tokens(root, 0, &mut |kind, range| {
        if range.start >= end || range.end <= start {
            return; // disjoint
        }
        if matches!(
            kind,
            SyntaxKind::AnchorMark | SyntaxKind::AliasMark | SyntaxKind::TagMark
        ) {
            found = true;
        }
    });
    found
}

/// Cheap textual screen for anchor / alias / tag introduction in
/// the replacement bytes. Conservative by design — any whiff of
/// these in `replacement` forces escalation to a full re-parse.
fn replacement_introduces_anchor_alias_or_tag(replacement: &str) -> bool {
    replacement.bytes().any(|b| matches!(b, b'&' | b'*' | b'!'))
}
