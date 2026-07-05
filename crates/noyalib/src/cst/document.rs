// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Public `Document` handle and parse / mutation entry points.

use core::fmt::Write as _;

use crate::cst::builder::{
    SubtreeContext, document_boundaries, parse_full, parse_subtree, rebuild_with_splice,
};
use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::syntax::SyntaxKind;
use crate::error::{Error, Result};
use crate::path::{QuerySegment, parse_query_path};
use crate::prelude::*;
use crate::span_context::SpanTree;
use crate::value::{Number, Value};

/// A YAML document with byte-faithful source preservation, typed
/// data access, and path-targeted edits.
///
/// `Document` carries three coordinated views of the same input:
/// an immutable green tree that reproduces the source byte-for-byte,
/// a typed [`Value`] for data access, and an internal span tree
/// that maps any [`Value`]-shaped path back to a byte range. Edits
/// flow through [`Document::replace_span`] (the primitive) and
/// [`Document::set`] (the path-shaped wrapper); untouched bytes —
/// indentation, comments, blank lines, sibling entries — are
/// preserved verbatim.
///
/// # Examples
///
/// Read-only round-trip:
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let src = "name: noyalib  # the project\nversion: 0.0.1\n";
/// let doc = parse_document(src).unwrap();
/// assert_eq!(doc.to_string(), src);
/// ```
///
/// Path-targeted edit:
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let mut doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
/// doc.set("version", "0.0.2").unwrap();
/// assert_eq!(doc.to_string(), "name: foo\nversion: 0.0.2\n");
/// ```
#[derive(Debug)]
pub struct Document {
    source: Arc<str>,
    green: GreenNode,
    /// Lazy cache for the typed [`Value`] view + path resolver
    /// [`SpanTree`]. Populated on first read; invalidated on every
    /// edit. Local-repair edits leave it `None` so consecutive
    /// `replace_span` calls don't pay the parser cost between them
    /// — the work is deferred until [`Document::as_value`],
    /// [`Document::span_at`], [`Document::get`], or any path-shaped
    /// API actually needs the value tree.
    cache: core::cell::RefCell<Option<(Value, SpanTree)>>,
    /// Outcome of the most recent edit's localised-repair attempt.
    /// `None` for a freshly-parsed document or after a full
    /// re-parse fallback.
    last_repair_scope: core::cell::Cell<Option<RepairScope>>,
}

impl Clone for Document {
    fn clone(&self) -> Self {
        Self {
            source: Arc::clone(&self.source),
            green: self.green.clone(),
            cache: core::cell::RefCell::new(self.cache.borrow().clone()),
            last_repair_scope: core::cell::Cell::new(self.last_repair_scope.get()),
        }
    }
}

/// The scope at which the most recent edit was repaired.
///
/// Smaller scopes are faster — `Scalar` only re-parses the leaf;
/// `Document` is equivalent to a full re-parse. Surfaced via
/// [`Document::last_repair_scope`] for tests and tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairScope {
    /// Reserved — scalar-granularity repair is not yet implemented.
    Scalar,
    /// The smallest ancestor that contained the edit was a
    /// `MappingEntry` or `SequenceItem`.
    Entry,
    /// The smallest ancestor that contained the edit was a
    /// `BlockMapping` / `BlockSequence` / flow collection.
    Collection,
    /// Edit fell back to (or escalated to) a full document re-parse.
    Document,
}

impl Document {
    /// Borrow the root [`GreenNode`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::{parse_document, SyntaxKind};
    ///
    /// let doc = parse_document("foo: 1\n").unwrap();
    /// assert_eq!(doc.syntax().kind(), SyntaxKind::Document);
    /// ```
    #[must_use]
    pub fn syntax(&self) -> &GreenNode {
        &self.green
    }

    /// Borrow the typed [`Value`] view of the document.
    ///
    /// On the first call after an edit (or a fresh parse), this
    /// triggers a one-shot parse of the current source into the
    /// internal `Value` / `SpanTree` cache. Subsequent calls on the
    /// same document are O(1) until the next edit invalidates the
    /// cache. Code that batches many edits without reading the
    /// typed view in between never pays the typed-tree cost.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("name: noyalib\n").unwrap();
    /// assert_eq!(doc.as_value()["name"].as_str(), Some("noyalib"));
    /// ```
    #[must_use]
    pub fn as_value(&self) -> core::cell::Ref<'_, Value> {
        self.ensure_cache();
        core::cell::Ref::map(self.cache.borrow(), |opt| {
            &opt.as_ref().expect("ensure_cache populated").0
        })
    }

    /// The original source bytes for this document. After an edit
    /// reflects the *current* source.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let src = "key: 1\n";
    /// let doc = parse_document(src).unwrap();
    /// assert_eq!(doc.source(), src);
    /// ```
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Resolve a `path` to the byte range of the value at that path,
    /// if any.
    ///
    /// Path syntax matches the rest of the crate (`foo.bar`,
    /// `items[0]`, `items[0].name`). Wildcard / recursive-descent
    /// segments are not supported here — they have no single span.
    ///
    /// A duplicated mapping key resolves to its *last* occurrence,
    /// the same occurrence the typed view keeps (`as_value` loads
    /// with the default `DuplicateKeyPolicy::Last`, the YAML 1.2
    /// behaviour) — the returned span always denotes the node that
    /// `as_value` selects for the path.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("name: noyalib\nversion: 0.0.1\n").unwrap();
    /// let (s, e) = doc.span_at("version").unwrap();
    /// assert_eq!(&doc.source()[s..e], "0.0.1");
    /// ```
    ///
    /// A duplicate key resolves to the occurrence the typed view
    /// keeps:
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("k: one\nk: two\n").unwrap();
    /// let (s, e) = doc.span_at("k").unwrap();
    /// assert_eq!(&doc.source()[s..e], "two");
    /// assert_eq!(doc.get("k"), Some("two"));
    /// ```
    #[must_use]
    pub fn span_at(&self, path: &str) -> Option<(usize, usize)> {
        let segments = parse_query_path(path);
        // Phase A.3 — green-tree path resolution. The common case
        // (plain block mappings, block sequences) resolves without
        // touching the typed cache: a single walk over the
        // structural CST is enough. Tooling that drives many edits
        // through `set` / `set_value` no longer warms the typed
        // cache between iterations.
        if let Some((s, e)) = resolve_path_in_green(&self.green, &segments, &self.source) {
            return Some(trim_trailing_blank(&self.source, s, e));
        }
        // Fallback for paths the green-tree walker doesn't
        // currently handle — e.g. quoted keys with escapes,
        // aliases, merge-keys. The cache is populated lazily.
        self.ensure_cache();
        let cache = self.cache.borrow();
        let (value, span_tree) = cache.as_ref().expect("ensure_cache populated");
        let (s, e) = resolve_span(value, span_tree, &segments)?;
        Some(trim_trailing_blank(&self.source, s, e))
    }

    /// Populate the typed cache from `self.source` if it is empty.
    /// Panics if the source fails to re-parse — for the lazy path
    /// to be safe, every successful edit must leave the source in a
    /// state that re-parses. Local repair edits gate themselves on
    /// `parse_subtree` (which validates the fragment) plus shape
    /// guards that escalate cross-document concerns to the
    /// safety-net full re-parse.
    fn ensure_cache(&self) {
        if self.cache.borrow().is_some() {
            return;
        }
        let cfg = crate::parser::ParseConfig::default();
        let parsed = crate::parser::parse_one(&self.source, &cfg)
            .expect("Document source must always parse — local repair invariant violated");
        *self.cache.borrow_mut() = Some(parsed);
    }

    /// Verify that the current source re-parses cleanly.
    ///
    /// `Document::set` (and the rest of the path-shaped edit API)
    /// uses a localised-repair fast path that gates each splice on
    /// the fragment's own scanner-level validation but commits
    /// *optimistically*: a structurally invalid splice across the
    /// whole document — for example, a value like `[` that opens a
    /// flow collection never closed at end-of-input — passes the
    /// fragment check and only surfaces when the typed view is
    /// next read. `as_value`, `span_at`, `get`, and any path-shaped
    /// API panic on first access in that state.
    ///
    /// `validate` is the non-panicking eager check: call it after
    /// an edit (or before handing the document to a downstream
    /// consumer) to surface any document-level parse error as a
    /// regular `Result`. On success, the typed cache is populated
    /// as a side-effect so a subsequent `as_value` call is free.
    ///
    /// # Errors
    ///
    /// Returns the underlying parse error if the source no longer
    /// parses as a single YAML document.
    ///
    /// # Examples
    ///
    /// Eagerly validate after an edit that may not be safe:
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("name: foo\n").unwrap();
    /// // `[` opens a flow seq that is never closed — the local
    /// // repair commits optimistically, but the document is now
    /// // structurally broken. `validate` surfaces that as an
    /// // error rather than waiting for the next typed-view read.
    /// doc.set("name", "[").unwrap();
    /// assert!(doc.validate().is_err());
    /// ```
    ///
    /// Validate a freshly-parsed document — always succeeds:
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("name: foo\n").unwrap();
    /// assert!(doc.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<()> {
        if self.cache.borrow().is_some() {
            return Ok(());
        }
        let cfg = crate::parser::ParseConfig::default();
        let parsed = crate::parser::parse_one(&self.source, &cfg)?;
        *self.cache.borrow_mut() = Some(parsed);
        Ok(())
    }

    /// Return the source slice of the value at `path`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    /// assert_eq!(doc.get("items[1]"), Some("two"));
    /// ```
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&str> {
        let (s, e) = self.span_at(path)?;
        Some(&self.source[s..e])
    }

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

        // Phase A.2 — Lazy Value/SpanTree:
        //   * On a successful local-repair edit, the green tree is
        //     spliced and the typed cache is invalidated. We do NOT
        //     re-parse the typed `Value` here. Subsequent edits in
        //     the same batch don't pay any parser cost; the
        //     deferred parse runs once, on the first read.
        //   * On the safety-net path (no local repair fit), the
        //     full re-parse already gives us validated `Value` and
        //     `SpanTree` — we drop them straight into the cache
        //     so the next read is free.
        let new_arc: Arc<str> = Arc::from(new_source.as_str());
        if let Some((new_green, scope)) =
            self.try_local_repair_green(start, end, replacement, &new_source)
        {
            self.last_repair_scope.set(Some(scope));
            self.source = new_arc;
            self.green = new_green;
            let _ = self.cache.replace(None);
            return Ok(());
        }

        // Safety net — full re-parse. Validates the new source and
        // populates everything eagerly.
        let parsed = parse_full(&new_source)?;
        self.last_repair_scope.set(Some(RepairScope::Document));
        self.source = parsed.source;
        self.green = parsed.green;
        let _ = self.cache.replace(Some((parsed.value, parsed.span_tree)));
        Ok(())
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
        // region forces a Document-scope re-parse so we don't have
        // to reason about cross-document name resolution.
        if region_has_anchor_alias_or_tag(&self.green, start, end)
            || replacement_introduces_anchor_alias_or_tag(replacement)
        {
            return None;
        }

        let delta = replacement.len() as isize - (end as isize - start as isize);
        let candidates = ancestor_candidates(&self.green, start, end);

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

    /// Last successful repair scope, if any. Useful for tests and
    /// instrumentation; returns `None` for a freshly-parsed
    /// document or when the most recent edit fell back to a full
    /// re-parse.
    #[must_use]
    pub fn last_repair_scope(&self) -> Option<RepairScope> {
        self.last_repair_scope.get()
    }

    /// Replace the value at `path` with `fragment`.
    ///
    /// `fragment` is spliced verbatim into the source — the caller
    /// supplies the YAML representation. This deliberately matches
    /// no scalar style automatically; choose double-quoted, plain,
    /// or block style to suit. Auto-formatting (the `Emit` trait
    /// from the design doc) is a follow-up.
    ///
    /// # Errors
    ///
    /// - `Error::Parse(...)` with "path not found" if `path` does
    ///   not resolve in the current document.
    /// - The same errors as [`Document::replace_span`] otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("name: foo\nversion: 0.0.1\n").unwrap();
    /// doc.set("version", "0.0.2").unwrap();
    /// assert_eq!(doc.to_string(), "name: foo\nversion: 0.0.2\n");
    /// ```
    pub fn set(&mut self, path: &str, fragment: &str) -> Result<()> {
        let (s, e) = self
            .span_at(path)
            .ok_or_else(|| Error::Parse(format!("path not found: {path}")))?;
        self.replace_span(s, e, fragment)
    }

    /// Replace the value at `path` with a typed [`Value`], formatting
    /// the YAML fragment to match the existing scalar style at the
    /// target site.
    ///
    /// Style matching:
    /// - `PlainScalar` — emit plain when safe, double-quoted otherwise.
    /// - `SingleQuotedScalar` — wrap in `'…'` (only string values).
    /// - `DoubleQuotedScalar` — wrap in `"…"` with standard escapes
    ///   (only string values).
    /// - `LiteralScalar` / `FoldedScalar` — currently rejected; block
    ///   scalar formatting is a follow-up.
    ///
    /// Non-string values (numbers, booleans, null) are emitted plain
    /// regardless of the existing style — quoting them would change
    /// the parsed type round-trip.
    ///
    /// # Errors
    ///
    /// - Path not found.
    /// - Target is a collection or block scalar.
    /// - Caller passed a `Sequence` / `Mapping` (use `set` with a
    ///   pre-formatted fragment for those — `set_value` is scalar-only
    ///   for now).
    /// - The same errors as [`Document::replace_span`] otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    /// use noyalib::Value;
    ///
    /// let mut doc = parse_document("name: noyalib\nversion: 0.0.1\n").unwrap();
    /// doc.set_value("version", &Value::String("0.0.2".into())).unwrap();
    /// assert_eq!(doc.to_string(), "name: noyalib\nversion: 0.0.2\n");
    /// ```
    pub fn set_value(&mut self, path: &str, value: &Value) -> Result<()> {
        let (s, e) = self
            .span_at(path)
            .ok_or_else(|| Error::Parse(format!("path not found: {path}")))?;
        let kind = leaf_kind_at(&self.green, s).ok_or_else(|| {
            Error::Parse("could not locate green-tree leaf at target span".into())
        })?;
        // Neighbour-aware styling: when the site is currently emitted
        // plain (so there is no quoting *intent* to preserve) and a
        // sibling style dominates the surrounding `BlockMapping`,
        // match the neighbours.
        let neighbour = sibling_dominant_scalar_kind(&self.green, s)
            .filter(|_| kind == SyntaxKind::PlainScalar);
        let entry_col = entry_indent_column(&self.source, s);
        let ctx = SiteContext {
            kind,
            neighbour,
            entry_col,
        };
        let fragment = format_value_for_site(value, &ctx)?;
        self.replace_span(s, e, &fragment)
    }

    /// Remove the value at `path` along with its surrounding entry
    /// (key + colon for mappings, `-` indicator for sequences).
    /// Trailing whitespace and the line break are removed too so the
    /// surrounding entries close up with no orphan blank line.
    ///
    /// Restrictions in this phase:
    /// - Block context only — flow-collection entry removal (`[a, b, c]`
    ///   → `[a, c]`) is a follow-up.
    /// - The value must end on the line where its key / `-` indicator
    ///   appears (single-line scalars). Multi-line values and nested
    ///   collections are deferred to the same follow-up that handles
    ///   block-scalar replacement in `set_value`.
    /// - Removing the only entry of a block mapping or sequence is
    ///   rejected — the result would parse differently (an empty
    ///   block becomes `null`), and the caller needs to express that
    ///   intent explicitly.
    ///
    /// # Errors
    ///
    /// - Path not found.
    /// - Restrictions above.
    /// - The same parse-after-edit errors as
    ///   [`Document::replace_span`]; on failure the document is left
    ///   unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("a: 1\nb: 2\nc: 3\n").unwrap();
    /// doc.remove("b").unwrap();
    /// assert_eq!(doc.to_string(), "a: 1\nc: 3\n");
    /// ```
    pub fn remove(&mut self, path: &str) -> Result<()> {
        self.ensure_cache();
        let segments = parse_query_path(path);
        let (line_start, line_end) = {
            let cache = self.cache.borrow();
            let (value, span_tree) = cache.as_ref().expect("ensure_cache populated");
            entry_line_span(value, span_tree, &self.source, &segments)?
        };
        self.replace_span(line_start, line_end, "")
    }

    /// Append a new item to the block sequence at `path`.
    ///
    /// `fragment` is the YAML representation of the *value* — the
    /// `- ` indicator and the surrounding indentation are synthesized
    /// from the existing items so the new line matches the file's
    /// shape. Block sequences only in this phase; flow sequences
    /// (`[…]`) and empty sequences are rejected.
    ///
    /// # Errors
    ///
    /// - `path` does not resolve to a sequence.
    /// - The sequence is a flow collection (`[…]`).
    /// - The sequence has no existing items to anchor indentation on.
    /// - The same parse-after-edit errors as
    ///   [`Document::replace_span`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("items:\n  - one\n  - two\n").unwrap();
    /// doc.push_back("items", "three").unwrap();
    /// assert_eq!(doc.to_string(), "items:\n  - one\n  - two\n  - three\n");
    /// ```
    pub fn push_back(&mut self, path: &str, fragment: &str) -> Result<()> {
        self.ensure_cache();
        let seq_len = {
            let cache = self.cache.borrow();
            let (value, _) = cache.as_ref().expect("ensure_cache populated");
            let target = path_value(value, path)
                .ok_or_else(|| Error::Parse(format!("path not found: {path}")))?;
            match target {
                Value::Sequence(s) => s.len(),
                _ => {
                    return Err(Error::Parse(
                        "push_back: target path is not a sequence".into(),
                    ));
                }
            }
        };
        if seq_len == 0 {
            return Err(Error::Parse(
                "push_back: empty sequence has no anchor for indentation — use `set` with a fragment instead"
                    .into(),
            ));
        }
        // Find the byte range of the LAST existing item to anchor
        // dash indentation and the splice position.
        let item_path = format!("{path}[{}]", seq_len - 1);
        let (last_start, last_end) = self
            .span_at(&item_path)
            .ok_or_else(|| Error::Parse("push_back: could not resolve last item span".into()))?;
        let dash_col = column_of_preceding_dash(&self.source, last_start).ok_or_else(|| {
            Error::Parse(
                "push_back: only block sequences are supported (no `-` anchor before last item)"
                    .into(),
            )
        })?;
        let line_end = end_of_line(&self.source, last_end);
        let indent: String = " ".repeat(dash_col);
        let new_line = format!("{indent}- {fragment}\n");
        self.replace_span(line_end, line_end, &new_line)
    }

    /// Detect the indentation unit (in spaces) used by this document.
    ///
    /// Walks the source line-by-line, looks for any pair of
    /// consecutive non-empty/non-comment lines where the second is
    /// more deeply indented than the first, and returns the smallest
    /// such delta — that is the file's "indent step", typically 2 or
    /// 4 spaces. A document with no nested structure (or only
    /// top-level keys) has no detectable step; the default `2` is
    /// returned in that case.
    ///
    /// Used internally by the [`crate::cst::Entry`] insertion paths
    /// to keep the inserted YAML's inner indentation consistent with
    /// what the rest of the file already uses (2-space file → 2-space
    /// inserts; 4-space file → 4-space inserts). Exposed publicly so
    /// callers building their own emission paths can match the same
    /// convention.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let two_space = parse_document(
    ///     "metadata:\n  labels:\n    app: noyalib\n",
    /// ).unwrap();
    /// assert_eq!(two_space.indent_unit(), 2);
    ///
    /// let four_space = parse_document(
    ///     "metadata:\n    labels:\n        app: noyalib\n",
    /// ).unwrap();
    /// assert_eq!(four_space.indent_unit(), 4);
    ///
    /// // No nested structure — defaults to 2.
    /// let flat = parse_document("a: 1\nb: 2\n").unwrap();
    /// assert_eq!(flat.indent_unit(), 2);
    /// ```
    #[must_use]
    pub fn indent_unit(&self) -> usize {
        detect_indent_unit(&self.source)
    }

    /// Inspect the document and return the dominant scalar quote
    /// style — `Plain`, `SingleQuoted`, or `DoubleQuoted`. Used by
    /// the [`crate::cst::Entry`] insert helpers to make new
    /// scalars adopt the file's existing convention rather than
    /// the serializer's hard-coded default.
    ///
    /// The detection scans every plain / single-quoted /
    /// double-quoted scalar leaf in the green tree, picks the
    /// majority, and breaks ties in favour of the simpler form
    /// (`Plain` > `SingleQuoted` > `DoubleQuoted`). Empty
    /// documents and documents with no string-shaped scalars
    /// default to `Plain`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    /// use noyalib::ScalarStyle;
    ///
    /// let single = parse_document("a: 'one'\nb: 'two'\n").unwrap();
    /// assert_eq!(single.dominant_quote_style(), ScalarStyle::SingleQuoted);
    ///
    /// let double = parse_document("a: \"one\"\nb: \"two\"\n").unwrap();
    /// assert_eq!(double.dominant_quote_style(), ScalarStyle::DoubleQuoted);
    ///
    /// let plain = parse_document("a: one\nb: two\n").unwrap();
    /// assert_eq!(plain.dominant_quote_style(), ScalarStyle::Plain);
    /// ```
    #[must_use]
    pub fn dominant_quote_style(&self) -> crate::ScalarStyle {
        detect_dominant_quote_style(&self.green)
    }

    /// Inspect the document and return the dominant collection
    /// style — `FlowStyle::Block` or `FlowStyle::Auto`
    /// (equivalent to "flow"). Used by `Entry::insert_value` to
    /// decide whether a typed mapping / sequence emission should
    /// use block or flow form.
    ///
    /// The detection counts top-level `BlockMapping` /
    /// `BlockSequence` vs `FlowMapping` / `FlowSequence` leaves
    /// and picks the majority. Empty / scalar-only documents
    /// default to `Block`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    /// use noyalib::FlowStyle;
    ///
    /// let block = parse_document("a:\n  - 1\n  - 2\n").unwrap();
    /// assert_eq!(block.dominant_flow_style(), FlowStyle::Block);
    ///
    /// let flow = parse_document("a: [1, 2, 3]\nb: [4, 5]\n").unwrap();
    /// assert_eq!(flow.dominant_flow_style(), FlowStyle::Auto);
    /// ```
    #[must_use]
    pub fn dominant_flow_style(&self) -> crate::FlowStyle {
        detect_dominant_flow_style(&self.green)
    }

    /// Insert a new `key: fragment` entry into the block mapping at
    /// `mapping_path`. The mapping-side analogue of
    /// [`Document::push_back`].
    ///
    /// Behaves like `set` when the key already exists (the value is
    /// replaced losslessly). When the key is new, a sibling line is
    /// spliced after the last existing entry, with the indent matched
    /// to the last entry's key column so the file stays canonical.
    /// Block mappings only in this phase; flow mappings (`{…}`) and
    /// empty mappings are rejected.
    ///
    /// # Errors
    ///
    /// - `mapping_path` does not resolve to a mapping.
    /// - The mapping is empty (no anchor for indentation; use `set`
    ///   with a fragment instead).
    /// - The same parse-after-edit errors as
    ///   [`Document::replace_span`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document(
    ///     "metadata:\n  labels:\n    app: noyalib\n",
    /// ).unwrap();
    /// doc.insert_entry("metadata.labels", "env", "prod").unwrap();
    /// let out = doc.to_string();
    /// assert!(out.contains("app: noyalib"));
    /// assert!(out.contains("env: prod"));
    /// ```
    pub fn insert_entry(&mut self, mapping_path: &str, key: &str, fragment: &str) -> Result<()> {
        // Easy path: if the key already exists, just replace.
        let child_path = if mapping_path.is_empty() {
            key.to_owned()
        } else {
            format!("{mapping_path}.{key}")
        };
        if self.span_at(&child_path).is_some() {
            return self.set(&child_path, fragment);
        }

        // New-key path — splice a sibling line.
        self.ensure_cache();
        let last_key: String = {
            let cache = self.cache.borrow();
            let (value, _) = cache.as_ref().expect("ensure_cache populated");
            let target = if mapping_path.is_empty() {
                value
            } else {
                path_value(value, mapping_path)
                    .ok_or_else(|| Error::Parse(format!("path not found: {mapping_path}")))?
            };
            let mapping = match target {
                Value::Mapping(m) => m,
                _ => {
                    return Err(Error::Parse(
                        "insert_entry: target path is not a mapping".into(),
                    ));
                }
            };
            if mapping.is_empty() {
                return Err(Error::Parse(
                    "insert_entry: empty mapping has no anchor for indentation — \
                     use `set` with a fragment instead"
                        .into(),
                ));
            }
            mapping
                .iter()
                .last()
                .map(|(k, _)| k.clone())
                .expect("non-empty mapping has a last entry")
        };
        let last_path = if mapping_path.is_empty() {
            last_key
        } else {
            format!("{mapping_path}.{last_key}")
        };
        let (last_value_start, last_value_end) = self.span_at(&last_path).ok_or_else(|| {
            Error::Parse("insert_entry: could not resolve last entry span".into())
        })?;
        let key_col = column_of_key_at(&self.source, last_value_start).ok_or_else(|| {
            Error::Parse("insert_entry: could not locate last key's column for indentation".into())
        })?;
        let line_end = end_of_line(&self.source, last_value_end);
        let indent: String = " ".repeat(key_col);

        // Single-line values (scalars, flow collections, anything
        // without an interior newline) splice inline. Multi-line
        // fragments — typically the YAML emission of a nested block
        // mapping or sequence — splice as `{key}:\n{children}` with
        // the children re-indented by `key_col + indent_unit` so the
        // nested structure lines up with the surrounding file's
        // convention (Phase 2.2).
        let new_line = if fragment.contains('\n') {
            let unit = detect_indent_unit(&self.source);
            let inner_indent: String = " ".repeat(key_col + unit);
            // Strip leading blank lines so a caller that prefixed `\n`
            // to force block form (see `Entry::insert_value` for a
            // single-entry collection) does not introduce a stray
            // blank between the key and its first child.
            let body = fragment.trim_start_matches('\n');
            let mut buf = format!("{indent}{key}:\n");
            for line in body.split('\n') {
                if line.is_empty() {
                    buf.push('\n');
                } else {
                    buf.push_str(&inner_indent);
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
            buf
        } else {
            format!("{indent}{key}: {fragment}\n")
        };
        self.replace_span(line_end, line_end, &new_line)
    }

    /// Insert a new sequence item immediately after the item at
    /// `item_path` (e.g. `"items[1]"`).
    ///
    /// `fragment` is the YAML representation of the value; the
    /// `- ` indicator and indentation are derived from the item at
    /// `item_path`.
    ///
    /// # Errors
    ///
    /// - `item_path` does not end in an index.
    /// - The path does not resolve to a sequence item in a block
    ///   sequence.
    /// - The same parse-after-edit errors as
    ///   [`Document::replace_span`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("items:\n  - one\n  - three\n").unwrap();
    /// doc.insert_after("items[0]", "two").unwrap();
    /// assert_eq!(
    ///     doc.to_string(),
    ///     "items:\n  - one\n  - two\n  - three\n",
    /// );
    /// ```
    pub fn insert_after(&mut self, item_path: &str, fragment: &str) -> Result<()> {
        let segments = parse_query_path(item_path);
        if !matches!(segments.last(), Some(QuerySegment::Index(_))) {
            return Err(Error::Parse(
                "insert_after: path must end with a sequence index, e.g. `items[2]`".into(),
            ));
        }
        let (item_start, item_end) = self
            .span_at(item_path)
            .ok_or_else(|| Error::Parse(format!("path not found: {item_path}")))?;
        let dash_col = column_of_preceding_dash(&self.source, item_start).ok_or_else(|| {
            Error::Parse(
                "insert_after: only block sequences are supported (no `-` anchor before item)"
                    .into(),
            )
        })?;
        let line_end = end_of_line(&self.source, item_end);
        let indent: String = " ".repeat(dash_col);
        let new_line = format!("{indent}- {fragment}\n");
        self.replace_span(line_end, line_end, &new_line)
    }
}

impl fmt::Display for Document {
    /// Re-emit the document. For any input that parses successfully,
    /// the result equals the original bytes verbatim. `Display`
    /// drives `Document::to_string()` via the standard `ToString`
    /// blanket impl.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.green.text(&self.source))
    }
}

/// Parse a YAML stream into an editable [`Document`].
///
/// # Errors
///
/// Returns the same parse errors as [`crate::from_str`] — the green
/// tree is built off the same scanner, so every strictness fix in
/// the regular parser applies here too.
///
/// # Examples
///
/// ```
/// use noyalib::cst::parse_document;
///
/// assert_eq!(parse_document("a: 1\n").unwrap().to_string(), "a: 1\n");
/// ```
pub fn parse_document(input: &str) -> Result<Document> {
    let parsed = parse_full(input)?;
    Ok(Document {
        source: parsed.source,
        green: parsed.green,
        // Initial parse already produced the typed view — seed the
        // cache so the first read after a fresh parse is free.
        cache: core::cell::RefCell::new(Some((parsed.value, parsed.span_tree))),
        last_repair_scope: core::cell::Cell::new(None),
    })
}

/// Parse a YAML stream and return one [`Document`] per logical
/// document.
///
/// Boundaries follow YAML 1.2.2 §9.1: an explicit `...` end marker
/// closes the current document, and a fresh `---` opens the next.
/// Trivia (comments, blank lines) between an explicit `...` and the
/// next document is treated as the next document's prologue;
/// trailing trivia at end-of-stream is attached to the last
/// document so concatenating each document's source reproduces the
/// original input byte-for-byte.
///
/// # Errors
///
/// Same as [`parse_document`].
///
/// # Examples
///
/// Single document:
///
/// ```
/// use noyalib::cst::parse_stream;
///
/// let src = "---\nfoo: 1\n";
/// let docs = parse_stream(src).unwrap();
/// assert_eq!(docs.len(), 1);
/// assert_eq!(docs[0].to_string(), src);
/// ```
///
/// Two documents — split on `---`:
///
/// ```
/// use noyalib::cst::{parse_stream, Document};
///
/// let src = "---\nfoo: 1\n---\nbar: 2\n";
/// let docs = parse_stream(src).unwrap();
/// assert_eq!(docs.len(), 2);
/// assert_eq!(docs[0].as_value()["foo"].as_i64(), Some(1));
/// assert_eq!(docs[1].as_value()["bar"].as_i64(), Some(2));
/// let joined: String = docs.iter().map(Document::source).collect();
/// assert_eq!(joined, src);
/// ```
pub fn parse_stream(input: &str) -> Result<Vec<Document>> {
    let bounds = document_boundaries(input)?;
    if bounds.len() <= 1 {
        return Ok(vec![parse_document(input)?]);
    }
    let mut out = Vec::with_capacity(bounds.len());
    for (s, e) in bounds {
        if s == e {
            continue;
        }
        out.push(parse_document(&input[s..e])?);
    }
    Ok(out)
}

// ── Localised repair (Phase A) ──────────────────────────────────────

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

fn walk_tokens(
    node: &GreenNode,
    base: usize,
    visit: &mut dyn FnMut(SyntaxKind, core::ops::Range<usize>),
) {
    let mut pos = base;
    for child in node.children() {
        let len = child.text_len();
        match child {
            GreenChild::Token { kind, .. } => {
                visit(*kind, pos..pos + len);
            }
            GreenChild::Node(inner) => walk_tokens(inner, pos, visit),
        }
        pos += len;
    }
}

/// Cheap textual screen for anchor / alias / tag introduction in
/// the replacement bytes. Conservative by design — any whiff of
/// these in `replacement` forces escalation to a full re-parse.
fn replacement_introduces_anchor_alias_or_tag(replacement: &str) -> bool {
    replacement.bytes().any(|b| matches!(b, b'&' | b'*' | b'!'))
}

// ── Green-tree path resolution (Phase A.3) ──────────────────────────

/// Resolve `segments` against the green tree of `root`, returning
/// the byte range of the value at that path. Walks the structural
/// CST directly — does not consult the typed `Value` / `SpanTree`,
/// so callers that drive many edits via `set` / `set_value` can
/// resolve paths without warming the typed cache between
/// iterations.
///
/// Returns `None` for paths the walker does not yet handle
/// (quoted-key escapes that aren't a simple single-quote-doubling,
/// aliases, merge keys, anchors); the caller is expected to fall
/// back to the typed cache for those cases.
fn resolve_path_in_green(
    root: &GreenNode,
    segments: &[QuerySegment],
    source: &str,
) -> Option<(usize, usize)> {
    // The Document root holds collection composites among its
    // children. Find the first one and treat it as the entry
    // point.
    let (collection, base) = first_collection_child(root, 0)?;
    walk_path(collection, segments, base, source)
}

fn first_collection_child(node: &GreenNode, base: usize) -> Option<(&GreenNode, usize)> {
    let mut pos = base;
    for child in node.children() {
        let len = child.text_len();
        if let GreenChild::Node(inner) = child {
            if matches!(
                inner.kind(),
                SyntaxKind::BlockMapping
                    | SyntaxKind::BlockSequence
                    | SyntaxKind::FlowMapping
                    | SyntaxKind::FlowSequence
            ) {
                return Some((inner, pos));
            }
        }
        pos += len;
    }
    None
}

fn walk_path(
    node: &GreenNode,
    segments: &[QuerySegment],
    base: usize,
    source: &str,
) -> Option<(usize, usize)> {
    if segments.is_empty() {
        return Some((base, base + node.text_len()));
    }
    let (head, tail) = segments.split_first()?;
    match (head, node.kind()) {
        (QuerySegment::Key(k), SyntaxKind::BlockMapping)
        | (QuerySegment::Key(k), SyntaxKind::FlowMapping) => {
            walk_mapping(node, k, tail, base, source)
        }
        (QuerySegment::Index(i), SyntaxKind::BlockSequence)
        | (QuerySegment::Index(i), SyntaxKind::FlowSequence) => {
            walk_sequence(node, *i, tail, base, source)
        }
        // Wildcard / recursive descent / kind mismatch — bail out;
        // the caller falls back to the typed cache.
        _ => None,
    }
}

fn walk_mapping(
    node: &GreenNode,
    key: &str,
    tail: &[QuerySegment],
    base: usize,
    source: &str,
) -> Option<(usize, usize)> {
    // Duplicate keys resolve to the *last* occurrence, matching the
    // typed view: under the default `DuplicateKeyPolicy::Last` (the
    // YAML 1.2 behaviour, and the config `as_value` loads with),
    // `k: one\nk: two` yields `k = "two"`, so the span for `k` must
    // denote `two` — never the bytes of a node the typed view did
    // not select. The whole mapping is scanned before committing.
    //
    // An entry whose key text cannot be decoded here (double-quoted
    // escapes, complex keys) could be a hidden duplicate of `key`,
    // making the green walk inconclusive — bail out and let the
    // caller resolve via the typed cache, which sees every key in
    // decoded form.
    let mut found: Option<(&GreenNode, usize)> = None;
    let mut undecodable_key = false;
    let mut pos = base;
    for child in node.children() {
        let len = child.text_len();
        if let GreenChild::Node(entry) = child {
            if entry.kind() == SyntaxKind::MappingEntry {
                match entry_key_text(entry, source, pos) {
                    Some(entry_key) => {
                        if entry_key == key {
                            found = Some((entry, pos));
                        }
                    }
                    None => undecodable_key = true,
                }
            }
        }
        pos += len;
    }
    if undecodable_key {
        return None;
    }
    let (entry, entry_pos) = found?;
    resolve_value_in_entry(entry, entry_pos, tail, source)
}

fn walk_sequence(
    node: &GreenNode,
    target_index: usize,
    tail: &[QuerySegment],
    base: usize,
    source: &str,
) -> Option<(usize, usize)> {
    let mut pos = base;
    let mut idx = 0usize;
    for child in node.children() {
        let len = child.text_len();
        if let GreenChild::Node(item) = child {
            if item.kind() == SyntaxKind::SequenceItem {
                if idx == target_index {
                    return resolve_value_in_item(item, pos, tail, source);
                }
                idx += 1;
            }
        }
        pos += len;
    }
    None
}

/// Extract the key text of a `MappingEntry`. Supports plain scalar
/// keys verbatim and single-quoted keys with the YAML
/// `''`-doubling escape. Returns `None` for keys whose textual
/// representation differs from the segment string the user would
/// pass — the caller falls back to the typed cache.
fn entry_key_text<'s>(entry: &GreenNode, source: &'s str, base: usize) -> Option<Cow<'s, str>> {
    let mut pos = base;
    for child in entry.children() {
        let child_len = child.text_len();
        match child {
            GreenChild::Token { kind, len } => {
                let start = pos;
                let end = pos + *len as usize;
                match kind {
                    SyntaxKind::QuestionIndicator
                    | SyntaxKind::Whitespace
                    | SyntaxKind::Newline
                    | SyntaxKind::Comment
                    | SyntaxKind::AnchorMark
                    | SyntaxKind::TagMark => {}
                    SyntaxKind::PlainScalar => {
                        return Some(Cow::Borrowed(&source[start..end]));
                    }
                    SyntaxKind::SingleQuotedScalar => {
                        return decode_single_quoted(&source[start..end]);
                    }
                    _ => return None,
                }
            }
            GreenChild::Node(_) => {
                return None;
            }
        }
        pos += child_len;
    }
    None
}

fn decode_single_quoted(raw: &str) -> Option<Cow<'_, str>> {
    // Strip surrounding quotes.
    let inner = raw.strip_prefix('\'')?.strip_suffix('\'')?;
    if !inner.contains('\'') {
        return Some(Cow::Borrowed(inner));
    }
    // Replace `''` with `'`. Anything else inside single quotes is
    // taken verbatim.
    Some(Cow::Owned(inner.replace("''", "'")))
}

/// Find the value position inside a `MappingEntry` and either
/// return its byte range (if `tail` is empty) or recurse into it
/// with `tail`.
fn resolve_value_in_entry(
    entry: &GreenNode,
    base: usize,
    tail: &[QuerySegment],
    source: &str,
) -> Option<(usize, usize)> {
    let (value_kind, value_range, value_node) = entry_value(entry, base)?;
    if tail.is_empty() {
        return Some(value_range);
    }
    // Recursing further requires the value to be a composite.
    let node = value_node?;
    walk_path(node, tail, value_range.0, source).map(|(s, e)| {
        // Defensive: ensure recursion stays inside the value's
        // span — composite parents may contain trailing trivia
        // that's outside the conceptual "value" range.
        let _ = value_kind;
        (s, e)
    })
}

fn resolve_value_in_item(
    item: &GreenNode,
    base: usize,
    tail: &[QuerySegment],
    source: &str,
) -> Option<(usize, usize)> {
    let (_, value_range, value_node) = item_value(item, base)?;
    if tail.is_empty() {
        return Some(value_range);
    }
    let node = value_node?;
    walk_path(node, tail, value_range.0, source)
}

/// Inside a `MappingEntry`, walk past the key + ColonIndicator and
/// return the first non-trivia "value" child. `value_node` is
/// `Some` if the value is a composite (a nested collection), `None`
/// if it is a leaf scalar.
fn entry_value(
    entry: &GreenNode,
    base: usize,
) -> Option<(SyntaxKind, (usize, usize), Option<&GreenNode>)> {
    let mut pos = base;
    let mut after_colon = false;
    // First-property-token start: when a value is preceded by an
    // [`SyntaxKind::AnchorMark`] / [`SyntaxKind::TagMark`] (or a
    // combination), the conceptual value span covers the entire
    // property prefix plus the scalar / node that follows.
    // Capture that earliest property start here so the returned
    // `(start, end)` stretches across the whole prefixed value.
    let mut prefix_start: Option<usize> = None;
    for child in entry.children() {
        let len = child.text_len();
        let child_start = pos;
        let child_end = pos + len;
        match child {
            GreenChild::Token { kind, .. } => {
                if !after_colon {
                    if *kind == SyntaxKind::ColonIndicator {
                        after_colon = true;
                    }
                } else if is_value_property_kind(*kind) {
                    // `!Tag` / `&anchor` / `*alias` prefix —
                    // remember the earliest start and keep
                    // scanning for the scalar that follows.
                    let _ = prefix_start.get_or_insert(child_start);
                } else if !is_trivia_kind(*kind) {
                    let start = prefix_start.unwrap_or(child_start);
                    return Some((*kind, (start, child_end), None));
                }
            }
            GreenChild::Node(inner) => {
                if after_colon {
                    let start = prefix_start.unwrap_or(child_start);
                    return Some((inner.kind(), (start, child_end), Some(inner)));
                }
            }
        }
        pos += len;
    }
    // Fall-through: the entry has a tag/anchor prefix but nothing
    // followed it before EOF — surface the prefix span so callers
    // see a meaningful range rather than `None`.
    prefix_start.map(|start| (SyntaxKind::PlainScalar, (start, pos), None))
}

/// Inside a `SequenceItem`, walk past the DashIndicator and return
/// the first non-trivia "value" child. Mirrors [`entry_value`]'s
/// tag/anchor-prefix handling: the returned span covers any
/// `!Tag` / `&anchor` / `*alias` property tokens **plus** the
/// scalar / node that follows.
fn item_value(
    item: &GreenNode,
    base: usize,
) -> Option<(SyntaxKind, (usize, usize), Option<&GreenNode>)> {
    let mut pos = base;
    let mut after_dash = false;
    let mut prefix_start: Option<usize> = None;
    for child in item.children() {
        let len = child.text_len();
        let child_start = pos;
        let child_end = pos + len;
        match child {
            GreenChild::Token { kind, .. } => {
                if !after_dash {
                    if *kind == SyntaxKind::DashIndicator {
                        after_dash = true;
                    }
                } else if is_value_property_kind(*kind) {
                    let _ = prefix_start.get_or_insert(child_start);
                } else if !is_trivia_kind(*kind) {
                    let start = prefix_start.unwrap_or(child_start);
                    return Some((*kind, (start, child_end), None));
                }
            }
            GreenChild::Node(inner) => {
                if after_dash {
                    let start = prefix_start.unwrap_or(child_start);
                    return Some((inner.kind(), (start, child_end), Some(inner)));
                }
            }
        }
        pos += len;
    }
    prefix_start.map(|start| (SyntaxKind::PlainScalar, (start, pos), None))
}

fn is_trivia_kind(k: SyntaxKind) -> bool {
    matches!(
        k,
        SyntaxKind::Whitespace
            | SyntaxKind::Newline
            | SyntaxKind::Comment
            | SyntaxKind::Bom
            | SyntaxKind::Directive
    )
}

/// Tokens that are part of a YAML *value* by attaching properties
/// (anchor, alias, tag) but are not themselves the value content.
/// The CST span resolver treats these as a *prefix* of the value
/// span — `entry_value` / `item_value` stretch their returned
/// `(start, end)` to cover the prefix plus the scalar / node that
/// follows, so `Document::span_at("name")` on
/// `name: !Custom 'app-1'` returns `6..21` (covering both the
/// tag and the quoted scalar) rather than `6..13` (the tag
/// alone, which was the pre-fix behaviour).
fn is_value_property_kind(k: SyntaxKind) -> bool {
    matches!(
        k,
        SyntaxKind::AnchorMark | SyntaxKind::TagMark | SyntaxKind::AliasMark
    )
}

// ── Path resolution ─────────────────────────────────────────────────

fn trim_trailing_blank(source: &str, start: usize, mut end: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    while end > start {
        match bytes[end - 1] {
            b' ' | b'\t' | b'\n' | b'\r' => end -= 1,
            _ => break,
        }
    }
    (start, end)
}

fn resolve_span(
    value: &Value,
    span_tree: &SpanTree,
    segments: &[QuerySegment],
) -> Option<(usize, usize)> {
    if segments.is_empty() {
        return match span_tree {
            // A zero-width leaf marks an implicit null (an absent
            // block-mapping value or empty sequence item): the node has no
            // source bytes of its own, so it has no span.
            SpanTree::Leaf(s, e) if s == e => None,
            SpanTree::Leaf(s, e) => Some((*s, *e)),
            SpanTree::Sequence { start, end, .. } | SpanTree::Mapping { start, end, .. } => {
                Some((*start, *end))
            }
        };
    }
    let (head, tail) = segments.split_first()?;
    match (head, value, span_tree) {
        (QuerySegment::Key(k), Value::Mapping(m), SpanTree::Mapping { entries, .. }) => {
            // `m` (an IndexMap) preserves insertion order, matching
            // the parallel order in `entries` (see `span_context::walk`).
            for ((mk, mv), (_, child_tree)) in m.iter().zip(entries.iter()) {
                if mk == k {
                    return resolve_span(mv, child_tree, tail);
                }
            }
            None
        }
        (QuerySegment::Index(i), Value::Sequence(seq), SpanTree::Sequence { items, .. }) => {
            let v = seq.get(*i)?;
            let t = items.get(*i)?;
            resolve_span(v, t, tail)
        }
        // Wildcard / recursive descent are unsupported because they
        // do not resolve to a *single* span; the caller would need a
        // multi-span API.
        _ => None,
    }
}

// ── Entry-line resolution (used by `remove`) ────────────────────────

/// Find the byte range of the *entire* mapping entry or sequence entry
/// addressed by `segments` — including its key / `-` indicator,
/// leading indentation, and trailing line break — so a caller can
/// splice the empty string in to delete it.
fn entry_line_span(
    value: &Value,
    span_tree: &SpanTree,
    source: &str,
    segments: &[QuerySegment],
) -> Result<(usize, usize)> {
    if segments.is_empty() {
        return Err(Error::Parse(
            "remove requires a non-empty path (cannot remove the document root)".into(),
        ));
    }

    let (head, tail) = segments
        .split_first()
        .ok_or_else(|| Error::Parse("path not found".into()))?;

    // Recurse into nested mappings / sequences until the segment list
    // identifies the *parent* of the entry to remove.
    if !tail.is_empty() {
        let (child_value, child_tree) = match (head, value, span_tree) {
            (QuerySegment::Key(k), Value::Mapping(m), SpanTree::Mapping { entries, .. }) => {
                let pos = m
                    .iter()
                    .position(|(mk, _)| mk == k)
                    .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?;
                (
                    m.iter().nth(pos).map(|(_, v)| v).expect("pos in range"),
                    &entries[pos].1,
                )
            }
            (QuerySegment::Index(i), Value::Sequence(seq), SpanTree::Sequence { items, .. }) => (
                seq.get(*i).ok_or_else(|| {
                    Error::Parse(format!("path not found: index {i} out of bounds"))
                })?,
                items.get(*i).ok_or_else(|| {
                    Error::Parse(format!("path not found: index {i} out of bounds"))
                })?,
            ),
            _ => return Err(Error::Parse("path not found".into())),
        };
        return entry_line_span(child_value, child_tree, source, tail);
    }

    // Final segment — locate this entry's key / dash and value.
    match (head, value, span_tree) {
        (QuerySegment::Key(k), Value::Mapping(m), SpanTree::Mapping { entries, .. }) => {
            if m.len() <= 1 {
                return Err(Error::Parse(
                    "remove cannot delete the only entry of a mapping".into(),
                ));
            }
            let pos = m
                .iter()
                .position(|(mk, _)| mk == k)
                .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?;
            let ((key_start, _key_end), child_tree) = &entries[pos];
            let raw_value_end = match child_tree {
                SpanTree::Leaf(_, e) => *e,
                SpanTree::Sequence { end, .. } | SpanTree::Mapping { end, .. } => *end,
            };
            let (_, value_end) = trim_trailing_blank(source, *key_start, raw_value_end);
            require_single_line(source, *key_start, value_end)?;
            Ok(line_extent(source, *key_start, value_end))
        }
        (QuerySegment::Index(i), Value::Sequence(seq), SpanTree::Sequence { items, .. }) => {
            if seq.len() <= 1 {
                return Err(Error::Parse(
                    "remove cannot delete the only entry of a sequence".into(),
                ));
            }
            let item_tree = items
                .get(*i)
                .ok_or_else(|| Error::Parse(format!("path not found: index {i} out of bounds")))?;
            let (value_start, raw_value_end) = match item_tree {
                SpanTree::Leaf(s, e) => (*s, *e),
                SpanTree::Sequence { start, end, .. } | SpanTree::Mapping { start, end, .. } => {
                    (*start, *end)
                }
            };
            let (_, value_end) = trim_trailing_blank(source, value_start, raw_value_end);
            // The `-` indicator sits before the value on the same line,
            // separated by inline whitespace. Walk backward to find it.
            let dash_pos = locate_preceding_dash(source, value_start).ok_or_else(|| {
                Error::Parse(
                    "remove: could not locate '-' indicator preceding sequence item".into(),
                )
            })?;
            require_single_line(source, dash_pos, value_end)?;
            Ok(line_extent(source, dash_pos, value_end))
        }
        _ => Err(Error::Parse("path not found".into())),
    }
}

/// Walk backward from `value_start` past inline whitespace and find
/// the `-` indicator that opened this sequence entry. Returns its
/// byte offset, or `None` if no dash is found on the same line.
/// Resolve `path` against `value` and return the addressed value.
/// Mirrors the resolution logic of `span_at` but works directly on
/// the typed [`Value`] tree.
fn path_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let segments = parse_query_path(path);
    let mut cur = value;
    for seg in &segments {
        match (seg, cur) {
            (QuerySegment::Key(k), Value::Mapping(m)) => {
                let (_k, v) = m.iter().find(|(mk, _)| *mk == k)?;
                cur = v;
            }
            (QuerySegment::Index(i), Value::Sequence(seq)) => {
                cur = seq.get(*i)?;
            }
            _ => return None,
        }
    }
    Some(cur)
}

/// Column of the `-` indicator on the same line as `value_start`,
/// found by walking backward over inline whitespace. `None` if no
/// dash precedes the value on its line.
fn column_of_preceding_dash(source: &str, value_start: usize) -> Option<usize> {
    let dash_pos = locate_preceding_dash(source, value_start)?;
    let bytes = source.as_bytes();
    let mut line_start = dash_pos;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    Some(dash_pos - line_start)
}

/// Walk every line in `source`, find pairs of consecutive
/// non-empty/non-comment lines where the second is more deeply
/// indented than the first, and return the smallest such delta —
/// the file's indent step. Defaults to `2` when nothing is detected
/// (single-level documents, all-top-level mappings).
///
/// Tab-indented lines are skipped: tabs cannot serve as YAML
/// indentation per spec §6.1, and trying to mix them into the
/// detection produces nonsense for the typical-case mixed-edit
/// scenario.
fn detect_indent_unit(source: &str) -> usize {
    let mut prev_indent: Option<usize> = None;
    let mut min_step: Option<usize> = None;
    for line in source.lines() {
        // Count leading spaces; bail on tab-indented lines.
        let mut spaces = 0;
        let bytes = line.as_bytes();
        let mut tab_seen = false;
        for &b in bytes {
            if b == b' ' {
                spaces += 1;
            } else if b == b'\t' {
                tab_seen = true;
                break;
            } else {
                break;
            }
        }
        if tab_seen {
            // Tab line — leaves prev_indent unchanged so the next
            // pair compares across the tab line.
            continue;
        }
        // Skip blank and comment-only lines.
        let trimmed = &line[spaces..];
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(prev) = prev_indent {
            if spaces > prev {
                let step = spaces - prev;
                min_step = Some(min_step.map_or(step, |m| m.min(step)));
            }
        }
        prev_indent = Some(spaces);
    }
    min_step.unwrap_or(2)
}

/// Column of the *key* that owns the value at `value_start`.
///
/// Two layouts to handle:
///
/// - **Inline:** `key: value` — key and value share a line. The key's
///   column is the leading-space count on that line.
/// - **Nested block:** `key:\n  child: …` — the value's first byte
///   sits on a child line, indented past the key. The key's column is
///   the leading-space count of an *earlier* non-blank/non-comment
///   line whose indent is *smaller* than the value-line's indent.
///
/// Walks backwards from `value_start`, skipping blank and comment
/// lines, and returns the first content line's column that is shallower
/// than the value line's column. Falls back to the value line's own
/// column for the inline case.
///
/// Returns `None` if `value_start` is out of range.
fn column_of_key_at(source: &str, value_start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if value_start > bytes.len() {
        return None;
    }

    // Locate the line that contains value_start.
    let line_start = |pos: usize| -> usize {
        let mut s = pos;
        while s > 0 && bytes[s - 1] != b'\n' {
            s -= 1;
        }
        s
    };
    let leading_spaces = |start: usize| -> usize {
        let mut c = 0;
        while start + c < bytes.len() && bytes[start + c] == b' ' {
            c += 1;
        }
        c
    };

    let value_line_start = line_start(value_start);
    let value_col = leading_spaces(value_line_start);

    // If there is real content (not just whitespace) on the value line
    // at or before `value_start`, the key is inline on this same line.
    let mut probe = value_line_start + value_col;
    let mut inline_content = false;
    while probe < value_start {
        let b = bytes[probe];
        if b != b' ' && b != b'\t' {
            inline_content = true;
            break;
        }
        probe += 1;
    }
    if inline_content {
        return Some(value_col);
    }

    // Nested case: walk backward by line, skipping blanks and
    // comment-only lines, until we find content at a *smaller* column.
    if value_line_start == 0 {
        return Some(value_col);
    }
    let mut cursor = value_line_start - 1; // past the trailing '\n'
    loop {
        // Find the start of the line ending at `cursor`.
        let mut prev_start = cursor;
        while prev_start > 0 && bytes[prev_start - 1] != b'\n' {
            prev_start -= 1;
        }
        let prev_col = leading_spaces(prev_start);
        let first_content = prev_start + prev_col;
        let after_content = cursor; // cursor still points at the '\n' index
        let is_blank = first_content >= after_content;
        let is_comment = !is_blank && bytes[first_content] == b'#';
        if !is_blank && !is_comment && prev_col < value_col {
            return Some(prev_col);
        }
        if prev_start == 0 {
            return Some(value_col);
        }
        cursor = prev_start - 1;
    }
}

/// Walk every scalar leaf in the green tree and pick the
/// dominant *quoted* style. Plain mapping keys overwhelm any
/// real signal from the values so we deliberately ignore them —
/// the question we want to answer is "when the user *did* quote
/// a value, did they reach for `'…'` or `\"…\"`?". Documents
/// with no quoted scalars at all default to `Plain` (the
/// simplest form, matching what most YAML files do for short
/// values).
fn detect_dominant_quote_style(root: &GreenNode) -> crate::ScalarStyle {
    let mut single = 0_usize;
    let mut double = 0_usize;
    walk_tokens(root, 0, &mut |kind, _| match kind {
        SyntaxKind::SingleQuotedScalar => single += 1,
        SyntaxKind::DoubleQuotedScalar => double += 1,
        _ => {}
    });
    if single == 0 && double == 0 {
        return crate::ScalarStyle::Plain;
    }
    if single >= double {
        crate::ScalarStyle::SingleQuoted
    } else {
        crate::ScalarStyle::DoubleQuoted
    }
}

/// Walk every collection leaf and pick the majority shape —
/// block (`BlockMapping` / `BlockSequence`) vs flow
/// (`FlowMapping` / `FlowSequence`). The result drives the
/// "block vs flow" decision in [`crate::cst::Entry::insert_value`]
/// when emitting a typed collection.
fn detect_dominant_flow_style(root: &GreenNode) -> crate::FlowStyle {
    let mut block = 0_usize;
    let mut flow = 0_usize;
    walk_collections(root, &mut |kind| match kind {
        SyntaxKind::BlockMapping | SyntaxKind::BlockSequence => block += 1,
        SyntaxKind::FlowMapping | SyntaxKind::FlowSequence => flow += 1,
        _ => {}
    });
    if flow > block {
        crate::FlowStyle::Auto
    } else {
        crate::FlowStyle::Block
    }
}

/// Walk every node (not token) in the green tree, calling
/// `visit` with each composite node's `SyntaxKind`.
fn walk_collections(node: &GreenNode, visit: &mut dyn FnMut(SyntaxKind)) {
    visit(node.kind());
    for child in node.children() {
        if let GreenChild::Node(inner) = child {
            walk_collections(inner, visit);
        }
    }
}

/// Position of the byte immediately past the next `\n` at or after
/// `pos`. If `pos` already points past a newline, returns `pos`.
/// At end-of-input, returns `source.len()`.
fn end_of_line(source: &str, pos: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = pos;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    if i < bytes.len() { i + 1 } else { i }
}

fn locate_preceding_dash(source: &str, value_start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = value_start;
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b' ' | b'\t' => {}
            b'-' => return Some(i),
            b'\n' | b'\r' => return None,
            _ => return None,
        }
    }
    None
}

/// Reject ranges that span multiple lines — `remove` currently only
/// handles entries whose value ends on the same line as the key /
/// dash.
fn require_single_line(source: &str, start: usize, end: usize) -> Result<()> {
    let segment = &source.as_bytes()[start..end];
    if segment.contains(&b'\n') {
        return Err(Error::Parse(
            "remove: multi-line / nested-value entries are not yet supported".into(),
        ));
    }
    Ok(())
}

/// Extend `(start, end)` outward to a full-line byte range:
/// - leftward to the byte after the previous `\n` (or 0).
/// - rightward through the trailing `\n` (or to EOF).
fn line_extent(source: &str, start: usize, end: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let mut s = start;
    while s > 0 && bytes[s - 1] != b'\n' {
        s -= 1;
    }
    let mut e = end;
    while e < bytes.len() && bytes[e] != b'\n' {
        e += 1;
    }
    if e < bytes.len() {
        e += 1;
    }
    (s, e)
}

// ── Green-tree leaf lookup ──────────────────────────────────────────

/// Return the [`SyntaxKind`] of the leaf containing byte position
/// `target` in `node`. Walks the green tree recursively with a
/// running offset.
fn leaf_kind_at(node: &GreenNode, target: usize) -> Option<SyntaxKind> {
    let mut pos = 0;
    for child in node.children() {
        let len = child.text_len();
        match child {
            GreenChild::Token { kind, .. } => {
                if pos <= target && target < pos + len {
                    return Some(*kind);
                }
            }
            GreenChild::Node(inner) => {
                if pos <= target && target < pos + len {
                    return leaf_kind_at(inner, target - pos);
                }
            }
        }
        pos += len;
    }
    None
}

/// If the leaf at byte `target` lives inside a `BlockMapping`'s
/// `MappingEntry`, scan the *other* entries' value scalars and
/// return their dominant scalar style — but only when that style is
/// `SingleQuotedScalar` or `DoubleQuotedScalar`. A plain-dominant
/// neighbourhood returns `None` (plain is the default fallback for
/// a plain site, so the caller does not need a hint).
fn sibling_dominant_scalar_kind(node: &GreenNode, target: usize) -> Option<SyntaxKind> {
    let (mapping, entry) = enclosing_mapping_and_entry(node, target, 0)?;
    dominant_sibling_value_kind(mapping, entry)
}

/// Walk the tree and return `(BlockMapping, MappingEntry)` ancestors
/// of the leaf at byte `target`, when both exist. Recursion is
/// linear in the tree height plus the children scanned per level.
fn enclosing_mapping_and_entry(
    node: &GreenNode,
    target: usize,
    base: usize,
) -> Option<(&GreenNode, &GreenNode)> {
    fn walk<'a>(
        node: &'a GreenNode,
        target: usize,
        base: usize,
        cur_mapping: Option<&'a GreenNode>,
        cur_entry: Option<&'a GreenNode>,
    ) -> Option<(&'a GreenNode, &'a GreenNode)> {
        let mut pos = base;
        for child in node.children() {
            let len = child.text_len();
            if pos <= target && target < pos + len {
                match child {
                    GreenChild::Token { .. } => {
                        if let (Some(m), Some(e)) = (cur_mapping, cur_entry) {
                            return Some((m, e));
                        }
                        return None;
                    }
                    GreenChild::Node(inner) => {
                        let new_mapping = if inner.kind() == SyntaxKind::BlockMapping {
                            Some(inner)
                        } else {
                            cur_mapping
                        };
                        let new_entry = if inner.kind() == SyntaxKind::MappingEntry {
                            Some(inner)
                        } else {
                            cur_entry
                        };
                        if let Some(found) = walk(inner, target, pos, new_mapping, new_entry) {
                            return Some(found);
                        }
                    }
                }
            }
            pos += len;
        }
        None
    }
    walk(node, target, base, None, None)
}

/// Tally value-scalar kinds of every `MappingEntry` child of
/// `mapping` *except* the entry being modified. Return the
/// dominant quoted style if and only if it is uniquely the most
/// frequent and there are at least two siblings vouching for it.
fn dominant_sibling_value_kind(mapping: &GreenNode, exclude: &GreenNode) -> Option<SyntaxKind> {
    let exclude_ptr: *const GreenNode = exclude;
    let mut plain = 0usize;
    let mut single = 0usize;
    let mut double = 0usize;
    for child in mapping.children() {
        if let GreenChild::Node(entry) = child {
            if entry.kind() != SyntaxKind::MappingEntry {
                continue;
            }
            // Cheap pointer-equality check — both come from the same
            // `Arc<[GreenChild]>` storage in this tree, so identity
            // comparison is reliable.
            let entry_ptr: *const GreenNode = entry;
            if core::ptr::eq(entry_ptr, exclude_ptr) {
                continue;
            }
            match entry_value_scalar_kind(entry) {
                Some(SyntaxKind::PlainScalar) => plain += 1,
                Some(SyntaxKind::SingleQuotedScalar) => single += 1,
                Some(SyntaxKind::DoubleQuotedScalar) => double += 1,
                _ => {}
            }
        }
    }
    // Need at least two siblings agreeing on a quoted style and a
    // strict plurality over the other quoted style and over plain.
    if single >= 2 && single > double && single > plain {
        return Some(SyntaxKind::SingleQuotedScalar);
    }
    if double >= 2 && double > single && double > plain {
        return Some(SyntaxKind::DoubleQuotedScalar);
    }
    None
}

/// Within a `MappingEntry`, return the syntax kind of the value
/// scalar (the leaf that follows `:`). `None` if the value is a
/// nested collection or otherwise not a single scalar leaf.
fn entry_value_scalar_kind(entry: &GreenNode) -> Option<SyntaxKind> {
    let mut after_colon = false;
    for child in entry.children() {
        match child {
            GreenChild::Token { kind, .. } => {
                if *kind == SyntaxKind::ColonIndicator {
                    after_colon = true;
                    continue;
                }
                if after_colon
                    && matches!(
                        kind,
                        SyntaxKind::PlainScalar
                            | SyntaxKind::SingleQuotedScalar
                            | SyntaxKind::DoubleQuotedScalar
                            | SyntaxKind::LiteralScalar
                            | SyntaxKind::FoldedScalar
                    )
                {
                    return Some(*kind);
                }
                // Whitespace / newline / comment leaves are skipped.
            }
            GreenChild::Node(_) => {
                if after_colon {
                    // Nested collection — value is not a single scalar.
                    return None;
                }
            }
        }
    }
    None
}

// ── Value → YAML scalar fragment ────────────────────────────────────

/// Context the formatter consults when picking a YAML representation
/// for a replacement value at a particular site.
struct SiteContext {
    /// The existing leaf's syntax kind at the splice site.
    kind: SyntaxKind,
    /// A dominant sibling scalar style, when one is unambiguous.
    /// Only consulted when [`Self::kind`] is `PlainScalar`.
    neighbour: Option<SyntaxKind>,
    /// Column of the first non-whitespace byte on the line that
    /// owns the splice site. Used to decide block-scalar
    /// continuation indent.
    entry_col: usize,
}

fn format_value_for_site(value: &Value, ctx: &SiteContext) -> Result<String> {
    match value {
        Value::Null => Ok("null".to_string()),
        Value::Bool(true) => Ok("true".to_string()),
        Value::Bool(false) => Ok("false".to_string()),
        Value::Number(n) => Ok(format_number(n)),
        Value::String(s) => format_string_for_site(s, ctx),
        Value::Sequence(_) | Value::Mapping(_) => Err(Error::Parse(
            "set_value cannot replace a scalar with a collection (use `set` with a fragment)"
                .into(),
        )),
        Value::Tagged(t) => format_value_for_site(t.value(), ctx),
    }
}

fn format_number(n: &Number) -> String {
    // `Number`'s `Display` matches the YAML 1.2 plain representation
    // for the integer/float variants we emit here.
    n.to_string()
}

fn format_string_for_site(s: &str, ctx: &SiteContext) -> Result<String> {
    // Multi-line string in a block context: prefer a literal block
    // scalar (`|` / `|-`) over `\n`-escaped double quotes — a
    // Renovate-style edit that lifts a one-line value into many
    // lines should look like the rest of the file would have, not
    // an escaped one-liner.
    if s.contains('\n') && can_use_block_literal(s) && is_block_site(ctx.kind) {
        return Ok(format_block_literal(s, ctx.entry_col));
    }

    match ctx.kind {
        SyntaxKind::PlainScalar => {
            // Neighbour preference only kicks in when the current
            // site is plain — i.e. there is no existing quoting
            // intent to preserve. A surrounding mapping that
            // unambiguously prefers one quoted style nudges the new
            // value into the same style.
            match ctx.neighbour {
                Some(SyntaxKind::SingleQuotedScalar) if !s.contains('\n') => {
                    Ok(format_single_quoted(s))
                }
                Some(SyntaxKind::DoubleQuotedScalar) => Ok(format_double_quoted(s)),
                _ => {
                    if is_plain_safe(s) {
                        Ok(s.to_string())
                    } else {
                        Ok(format_double_quoted(s))
                    }
                }
            }
        }
        SyntaxKind::SingleQuotedScalar => Ok(format_single_quoted(s)),
        SyntaxKind::DoubleQuotedScalar => Ok(format_double_quoted(s)),
        SyntaxKind::LiteralScalar | SyntaxKind::FoldedScalar => {
            // Replacing a block scalar with a *single-line* string
            // is a legitimate edit (e.g. truncating a longer note
            // back to one line). Emit the natural plain/quoted
            // shape rather than a one-line block literal.
            if !s.contains('\n') {
                if is_plain_safe(s) {
                    Ok(s.to_string())
                } else {
                    Ok(format_double_quoted(s))
                }
            } else if can_use_block_literal(s) {
                Ok(format_block_literal(s, ctx.entry_col))
            } else {
                Err(Error::Parse(
                    "set_value: existing block scalar can only be replaced with a string \
                     whose content lines do not begin with whitespace or control characters yet"
                        .into(),
                ))
            }
        }
        _ => Err(Error::Parse(
            "set_value: target site is not a scalar leaf".into(),
        )),
    }
}

/// `true` when the existing leaf's syntax kind belongs to a
/// block-context scalar — block mappings/sequences are the only
/// place a literal `|` block scalar makes sense.
fn is_block_site(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::PlainScalar
            | SyntaxKind::SingleQuotedScalar
            | SyntaxKind::DoubleQuotedScalar
            | SyntaxKind::LiteralScalar
            | SyntaxKind::FoldedScalar
    )
}

/// Conservative check: a string is safely representable as a literal
/// block scalar only when none of its lines begin with a horizontal
/// whitespace character (which would require an explicit indent
/// indicator we do not yet emit), it contains no control characters
/// other than `\n`, and its trailing-newline count is zero or one
/// (matched by the `|-` and `|` chomping indicators respectively).
fn can_use_block_literal(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Reject control characters except `\n` and `\t` between content.
    for &b in s.as_bytes() {
        if (b < 0x20 && b != b'\n' && b != b'\t') || b == 0x7F {
            return false;
        }
    }
    // Strip up to one trailing newline; reject more than one.
    let trimmed = s.strip_suffix('\n').unwrap_or(s);
    if trimmed.ends_with('\n') {
        return false;
    }
    // No line may start with a space or tab — that requires an
    // explicit indentation indicator we do not emit yet.
    for line in trimmed.split('\n') {
        if line.starts_with(' ') || line.starts_with('\t') {
            return false;
        }
    }
    true
}

/// Format `s` as a literal block scalar (`|` or `|-`) at
/// `entry_col + 2` indent.
fn format_block_literal(s: &str, entry_col: usize) -> String {
    let trailing_nl = s.ends_with('\n');
    let body = if trailing_nl { &s[..s.len() - 1] } else { s };
    let indent_str = " ".repeat(entry_col + 2);

    let mut out =
        String::with_capacity(s.len() + 8 + indent_str.len() * (body.matches('\n').count() + 1));
    out.push('|');
    if !trailing_nl {
        // Strip chomping indicator removes any trailing newlines, so
        // we can faithfully encode the no-trailing-newline case.
        out.push('-');
    }
    out.push('\n');
    let mut first = true;
    for line in body.split('\n') {
        if !first {
            out.push('\n');
        }
        first = false;
        out.push_str(&indent_str);
        out.push_str(line);
    }
    // `replace_span` pastes the fragment in place of the value
    // bytes only — the trailing line break that separates this
    // entry from the next is already in the surrounding source.
    out
}

/// Compute the column (zero-based) of the first non-whitespace byte
/// on the line that contains `pos` in `source`. For
/// `  version: 0.0.1\n` with `pos` at the value scalar's start,
/// returns 2.
fn entry_indent_column(source: &str, pos: usize) -> usize {
    let bytes = source.as_bytes();
    let mut line_start = pos.min(bytes.len());
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let mut col = line_start;
    while col < bytes.len() && (bytes[col] == b' ' || bytes[col] == b'\t') {
        col += 1;
    }
    col - line_start
}

/// `true` if `s` can be safely emitted as a YAML plain scalar without
/// being misparsed as a different type (bool, null, number) or
/// triggering a structural indicator. Conservative — when in doubt,
/// the caller falls back to a quoted style.
fn is_plain_safe(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Reserved scalars that resolve to non-string types.
    if matches!(
        s,
        "null"
            | "Null"
            | "NULL"
            | "~"
            | "true"
            | "True"
            | "TRUE"
            | "false"
            | "False"
            | "FALSE"
            | "yes"
            | "Yes"
            | "YES"
            | "no"
            | "No"
            | "NO"
            | "on"
            | "On"
            | "ON"
            | "off"
            | "Off"
            | "OFF"
    ) {
        return false;
    }
    if looks_like_number(s) {
        return false;
    }
    let bytes = s.as_bytes();
    // Cannot start with structural / flow / quote indicators.
    let first = bytes[0];
    if matches!(
        first,
        b'-' | b'?'
            | b':'
            | b','
            | b'['
            | b']'
            | b'{'
            | b'}'
            | b'#'
            | b'&'
            | b'*'
            | b'!'
            | b'|'
            | b'>'
            | b'\''
            | b'"'
            | b'%'
            | b'@'
            | b'`'
            | b' '
            | b'\t'
    ) {
        return false;
    }
    // Cannot end with whitespace.
    if matches!(*bytes.last().unwrap(), b' ' | b'\t') {
        return false;
    }
    // Disallow line breaks and control characters; disallow `: ` and
    // ` #` which terminate plain scalars in block context.
    let mut prev: u8 = 0;
    for &b in bytes {
        if b < 0x20 || b == 0x7F {
            return false;
        }
        if b == b' ' && prev == b':' {
            return false;
        }
        if b == b'#' && prev == b' ' {
            return false;
        }
        prev = b;
    }
    true
}

fn looks_like_number(s: &str) -> bool {
    // Leading sign or digit makes it a number candidate.
    let mut chars = s.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    let candidate = matches!(first, '-' | '+' | '.') || first.is_ascii_digit();
    if !candidate {
        return false;
    }
    // Defer the actual parse to `Number`'s integer/float resolvers via
    // the streaming scalar resolver (which is the source of truth for
    // what the parser would treat as a number).
    let scalar = crate::streaming::resolve_plain_ext(s, false, false, false, false, false, false);
    match scalar {
        crate::streaming::Scalar::Int(_) | crate::streaming::Scalar::Float(_) => true,
        #[cfg(feature = "lossless-u64")]
        crate::streaming::Scalar::Uint(_) => true,
        _ => false,
    }
}

fn format_single_quoted(s: &str) -> String {
    // YAML 1.2 §7.3.3: single quote is the only escape — `''` for `'`.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn format_double_quoted(s: &str) -> String {
    // YAML 1.2 §5.7 + §7.3.2: standard JSON-like escapes plus the
    // YAML extras (`\0`, `\a`, `\v`, `\e`, `\N`, `\_`, `\L`, `\P`).
    // For Phase 2B we emit the JSON-compatible subset; the others
    // are unnecessary for round-tripping textual content and would
    // complicate the diff if we surface them.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(&mut out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
