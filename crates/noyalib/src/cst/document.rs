// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Public `Document` handle and parse / mutation entry points.

use core::fmt::Write as _;

use crate::cst::builder::{
    SubtreeContext, document_boundaries, parse_full, parse_subtree, rebuild_with_splice,
};
use crate::cst::emit::{Emit, EmitCtx, emit_key};
use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::syntax::SyntaxKind;
use crate::doc_boundary::strip_bom;
use crate::error::{Error, Result};
use crate::path::{QuerySegment, parse_query_path};
use crate::prelude::*;
use crate::span_context::SpanTree;
use crate::value::{Mapping, Number, Value};

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
            return Some(trim_value_span(&self.source, s, e));
        }
        // Fallback for paths the green-tree walker doesn't
        // currently handle — e.g. quoted keys with escapes,
        // aliases, merge-keys. The cache is populated lazily.
        self.ensure_cache();
        let cache = self.cache.borrow();
        let (value, span_tree) = cache.as_ref().expect("ensure_cache populated");
        // Reads resolve an alias through to its anchor (issue #149); the
        // through-alias flag only matters for writes (see `write_span`).
        let ((s, e), _through_alias) = resolve_span(value, span_tree, &segments)?;
        // A zero-width span is an implicit null, which has no bytes to read
        // (#165). The resolver now hands the position over for the write
        // paths' benefit; discarding it is this reader's job.
        if s == e {
            return None;
        }
        Some(trim_value_span(&self.source, s, e))
    }

    /// Return the byte span of a mapping entry's **key** token, the
    /// read-only companion to [`span_at`](Self::span_at) (which returns
    /// the *value* span). `source()[start..end]` is the key exactly as
    /// written — quotes included for a quoted key.
    ///
    /// This exposes, read-only, the same key site
    /// [`rename_key`](Self::rename_key) rewrites; it is the span tooling
    /// needs to report duplicate keys with positions or to drive a
    /// "rename key" code action without walking the green tree by hand.
    ///
    /// Returns `None` when the path does not resolve to a block-mapping
    /// entry with a simple scalar key — a sequence index, an alias
    /// (`*name`) site (which owns no key bytes of its own), a key
    /// produced by a `<<` merge, or a path that does not resolve at all.
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("name: foo\n\"quoted key\": 1\n").unwrap();
    /// let (s, e) = doc.key_span("name").unwrap();
    /// assert_eq!(&doc.source()[s..e], "name");
    /// let (s, e) = doc.key_span("quoted key").unwrap();
    /// assert_eq!(&doc.source()[s..e], "\"quoted key\"");
    /// assert_eq!(doc.key_span("missing"), None);
    /// ```
    #[must_use]
    pub fn key_span(&self, path: &str) -> Option<(usize, usize)> {
        // A sentinel `new_key` that cannot equal any real sibling, so
        // `entry_key_site`'s duplicate-refusal branch is never taken and
        // it behaves as a pure key-span resolver. Any resolution error
        // (alias / merge-provided / not-a-mapping-entry / not found) and
        // the zero-width span the loader records for a non-scalar key
        // both map to `None`.
        const KEY_SPAN_SENTINEL: &str = "\0\0noyalib::key_span sentinel\0\0";
        let segments = parse_query_path(path);
        if segments.is_empty() {
            return None;
        }
        self.ensure_cache();
        let cache = self.cache.borrow();
        let (value, span_tree) = cache.as_ref().expect("ensure_cache populated");
        match entry_key_site(value, span_tree, &segments, KEY_SPAN_SENTINEL) {
            Ok((s, e)) if s != e => Some((s, e)),
            _ => None,
        }
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
    /// supplies the YAML representation. This deliberately matches no
    /// scalar style automatically; choose double-quoted, plain, or
    /// block style to suit.
    ///
    /// # Prefer [`Document::set_value`] for values
    ///
    /// Verbatim means the fragment is YAML, not text. `set(p, "true")`
    /// writes the boolean, `set(p, "")` writes null, and
    /// `set(p, "v # x")` writes `v` with a comment after it. If you
    /// have a *value* rather than a spelling, [`set_value`] renders it
    /// — quoting, escaping and choosing a block style as needed — so
    /// it reads back as exactly what you passed in.
    ///
    /// [`set_value`]: Document::set_value
    ///
    /// # The fragment cannot reach outside `path`
    ///
    /// A fragment containing a newline could previously give the
    /// document new entries:
    ///
    /// ```text
    /// set("a", "v\nc: 3")  on  "a: 1\nb: 2\n"   ->   a: v
    ///                                                  c: 3
    ///                                                  b: 2
    /// ```
    ///
    /// The re-parse guard could not catch that, because the result is
    /// valid YAML. An oracle now checks that restoring the original
    /// value at `path` reproduces the original document; if the
    /// fragment changed anything elsewhere, the edit is refused and the
    /// document is left untouched. Restructuring the target itself —
    /// scalar to mapping, say — remains allowed.
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
        let (s, e) = self.write_span(path)?;

        // Verbatim splicing is this method's contract, and a fragment
        // that legitimately restructures the *target* — scalar to
        // mapping, say — must keep working. What must not happen is a
        // fragment reaching outside the target:
        //
        //     set("a", "v\nc: 3")  on  "a: 1\nb: 2\n"
        //
        // spliced a newline and gave the document a new key `c`. The
        // re-parse guard cannot see it, because the result is valid
        // YAML — just not the document anyone asked for.
        //
        // The oracle: take the edited document, put the original value
        // back at `path`, and require the result to equal the original
        // document. Anything the fragment changed elsewhere survives
        // that restoration and shows up as a mismatch.
        //
        // Callers wanting a value rendered safely should use
        // `set_value`, which quotes and picks a scalar style.
        self.ensure_cache();
        let segments = parse_query_path(path);
        let before_shape = {
            let cache = self.cache.borrow();
            let (value, _) = cache.as_ref().expect("ensure_cache populated");
            shape_excluding(value, &segments)
        };

        let snapshot = self.clone();
        // Filling in an implicit null: the span abuts the `:` / `-`, so the
        // separator is this writer's to supply. Splicing stays verbatim.
        let filled;
        let fragment = if s == e {
            filled = fill_in(fragment);
            filled.as_str()
        } else {
            fragment
        };
        self.replace_span(s, e, fragment)?;

        // Parse fallibly rather than through `as_value`. A splice that
        // is structurally invalid — `set("name", "[")` — commits
        // optimistically by design and only surfaces via `validate`;
        // forcing a parse here would panic on it and change that
        // documented behaviour. An unparseable result cannot be
        // smuggling extra entries anyway, so there is nothing for this
        // oracle to check.
        let Ok(after_value) = crate::from_str::<Value>(&self.source) else {
            return Ok(());
        };
        let after_shape = shape_excluding(&after_value, &segments);
        if after_shape != before_shape {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "set: the fragment for `{path}` added or removed entries \
                 elsewhere in the document — it was left unchanged. Use \
                 `set_value` to write a value without splicing YAML."
            )));
        }
        Ok(())
    }

    /// Resolve `path` to a byte span for a **write**, refusing when the value
    /// is (or resolves through) an alias reference.
    ///
    /// `span_at` resolves an alias *through* to its anchor's value span (issue
    /// #149) — the right target for a read, but splicing there would rewrite
    /// the **anchor's** bytes, a different key. The green-tree fast path never
    /// yields an alias (it bails on `AliasMark`), so only the typed-cache
    /// fallback can; `resolve_span`'s `through_alias` flag is the single source
    /// of truth for that, so the two paths cannot disagree.
    fn write_span(&self, path: &str) -> Result<(usize, usize)> {
        let segments = parse_query_path(path);
        if let Some((s, e)) = resolve_path_in_green(&self.green, &segments, &self.source) {
            return Ok(trim_value_span(&self.source, s, e));
        }
        self.ensure_cache();
        let cache = self.cache.borrow();
        let (value, span_tree) = cache.as_ref().expect("ensure_cache populated");
        let ((s, e), through_alias) = resolve_span(value, span_tree, &segments)
            .ok_or_else(|| Error::Parse(format!("path not found: {path}")))?;
        if through_alias {
            return Err(Error::Parse(format!(
                "cannot set `{path}`: its value is (or resolves through) an alias \
                 reference; edit the anchor definition or replace the alias explicitly"
            )));
        }
        if s == e {
            return implicit_null_insertion_point(&self.source, s)
                .ok_or_else(|| Error::Parse(format!("path not found: {path}")));
        }
        Ok(trim_value_span(&self.source, s, e))
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
    /// - `LiteralScalar` / `FoldedScalar` — a single-line replacement is
    ///   emitted plain (or quoted when unsafe); a multi-line one is
    ///   re-emitted as a literal block when representable, and refused
    ///   otherwise. Folded style is not yet reproduced — a changed value
    ///   at a `>` site comes back `|` or plain.
    ///
    /// Setting a value equal to the one already loaded is a **no-op**:
    /// the source is left byte-identical, so the author's spelling
    /// (`1.10`, `0x1F`, `~`, an implicit null, a `>-` folded scalar)
    /// survives a save that does not change the value. Equality is
    /// [`Value`]'s own, so whether `1.0` over a loaded `1` is a no-op
    /// follows `Number`'s `PartialEq` for the active features.
    ///
    /// Non-string values (numbers, booleans, null) are emitted plain
    /// regardless of the existing style — quoting them would change
    /// the parsed type round-trip.
    ///
    /// Inside a `[…]` / `{…}` flow collection the same styles apply,
    /// except that a plain spelling is also refused when the string
    /// contains `,` `[` `]` `{` or `}` (structural anywhere in flow
    /// context), and a multi-line string is double-quoted with `\n`
    /// escapes, because block scalars do not exist in flow context:
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    /// use noyalib::Value;
    ///
    /// let mut doc = parse_document("m: {a: 1, b: 2}\n").unwrap();
    /// doc.set_value("m.a", &Value::String("x, y".into())).unwrap();
    /// assert_eq!(doc.to_string(), "m: {a: \"x, y\", b: 2}\n");
    /// doc.set_value("m.b", &Value::String("two\nlines".into())).unwrap();
    /// assert_eq!(doc.to_string(), "m: {a: \"x, y\", b: \"two\\nlines\"}\n");
    /// ```
    ///
    /// # Filling in an implicit null
    ///
    /// An absent block-mapping value (`a:`) or empty sequence item (`- `) has
    /// no bytes to replace, so the value is *inserted* after the `:` / `-`
    /// instead — before any comment on the line, and with no style to inherit,
    /// so the neighbour rule above decides the spelling on its own.
    /// [`span_at`](Self::span_at) still reports `None` there: the node has
    /// nothing to read, which is a separate question from where a write goes.
    ///
    /// # A trailing comment beside a new block literal
    ///
    /// A multi-line string is written as a literal block scalar, which
    /// runs to the end of its last content line. A comment that trailed
    /// the old one-line value (`title: Hello # note`) therefore moves to
    /// the block scalar's header line, where YAML permits one:
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    /// use noyalib::Value;
    ///
    /// let mut doc = parse_document("title: Hello # note\n").unwrap();
    /// doc.set_value("title", &Value::String("multi\nline".into())).unwrap();
    /// assert_eq!(doc.to_string(), "title: |- # note\n  multi\n  line\n");
    /// assert_eq!(doc.as_value()["title"].as_str(), Some("multi\nline"));
    /// ```
    ///
    /// # Collections (#328)
    ///
    /// A `Value::Sequence` / `Value::Mapping` replaces an existing
    /// **collection** node in that node's own style — flow stays flow
    /// (`tags: [a, b]` set to `[a, c]` emits `tags: [a, c]`), block
    /// stays block at the old value's column. The splice is verified
    /// to load back as the document with exactly this path replaced,
    /// or it is rolled back. Replacing a *scalar* with a collection is
    /// still refused: a value that must move onto its own lines is a
    /// layout decision `set` expresses with a fragment.
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    /// use noyalib::{Value, from_str};
    ///
    /// let mut doc = parse_document("tags: [a, b]\nname: x\n").unwrap();
    /// let tags: Value = from_str("[a, c]").unwrap();
    /// doc.set_value("tags", &tags).unwrap();
    /// assert_eq!(doc.to_string(), "tags: [a, c]\nname: x\n");
    /// ```
    ///
    /// # Anchored nodes (#338)
    ///
    /// A write into a value that `*name` alias sites share lands at
    /// every one of them, so it is refused — the same policy
    /// [`rename_key`](Self::rename_key), [`remove`](Self::remove) and
    /// the inserters follow. Call
    /// [`materialise_aliases_of`](Self::materialise_aliases_of) first
    /// to give each site its own copy. Setting a value **equal** to
    /// the current one stays a no-op wherever it points.
    ///
    /// # Errors
    ///
    /// - Path not found.
    /// - The target sits inside an anchored value with live alias
    ///   references.
    /// - Target is a block scalar being replaced by a multi-line
    ///   string it cannot represent.
    /// - Caller passed a `Sequence` / `Mapping` and the target is a
    ///   scalar (use `set` with a pre-formatted fragment to grow a
    ///   scalar into a collection).
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
    ///
    /// // An equal value does not touch the bytes.
    /// let mut doc = parse_document("ratio: 1.10\n").unwrap();
    /// doc.set_value("ratio", &Value::from(1.1_f64)).unwrap();
    /// assert_eq!(doc.to_string(), "ratio: 1.10\n");
    /// ```
    pub fn set_value(&mut self, path: &str, value: &Value) -> Result<()> {
        let (s, e) = self.write_span(path)?;
        // A value equal to the one already there is a no-op, and a no-op
        // must leave the bytes alone: re-rendering an equal value through
        // the formatter rewrites spellings the author chose (`1.10` to
        // `1.1`, `0x1F` to `31`, `+1` to `1`, `~` to `null`, a `>-`
        // folded scalar to a plain one) without changing the loaded
        // document (#337). `write_span` has already vetted the path, so
        // alias refusals and path errors are unaffected.
        {
            self.ensure_cache();
            let cache = self.cache.borrow();
            let (root, _) = cache.as_ref().expect("ensure_cache populated");
            if typed_value_at(root, &parse_query_path(path)) == Some(value) {
                return Ok(());
            }
        }
        // A collection value replaces a collection node in the node's
        // own style — flow stays flow, block stays block (#328). The
        // scalar formatter below cannot spell one, so branch off here.
        if matches!(value, Value::Sequence(_) | Value::Mapping(_)) {
            return self.replace_collection_value(path, value, s, e);
        }
        // One policy for anchored nodes (#338): a write into a value
        // that `*name` sites share lands at every one of them, so it is
        // refused with the same guidance rename_key and the inserters
        // give. (An equal value already returned above: a no-op is
        // harmless wherever it points.)
        self.refuse_inside_aliased_anchor("set_value", path, s)?;
        // An empty span is an implicit null's insertion point, not a value to
        // overwrite: there is no scalar leaf there to read a style from, and
        // no `: ` separator either, since the span starts right after the
        // indicator. Everything else about the site — the neighbours, the
        // column — is read the same way.
        let filling_in = s == e;
        let kind = if filling_in {
            // No existing bytes means no quoting *intent* to preserve, which
            // is exactly the state `PlainScalar` denotes to the neighbour rule
            // below.
            SyntaxKind::PlainScalar
        } else {
            leaf_kind_at(&self.green, s).ok_or_else(|| {
                Error::Parse("could not locate green-tree leaf at target span".into())
            })?
        };
        // Neighbour-aware styling: when the site is currently emitted
        // plain (so there is no quoting *intent* to preserve) and a
        // sibling style dominates the surrounding `BlockMapping`,
        // match the neighbours.
        let neighbour = sibling_dominant_scalar_kind(&self.green, s)
            .filter(|_| kind == SyntaxKind::PlainScalar);
        let entry_col = entry_indent_column(&self.source, s);
        let in_flow = in_flow_collection(&self.green, s);
        let ctx = SiteContext {
            kind,
            neighbour,
            entry_col,
            in_flow,
        };
        let fragment = format_value_for_site(value, &ctx)?;
        // A block literal owns every byte through the end of its last
        // content line, so a comment that trailed the old one-line value
        // would be swallowed into the new value. Move it onto the header
        // line, where YAML allows a comment, and widen the splice to
        // cover it (#333).
        let (fragment, e) = match trailing_comment_span(&self.source, e) {
            Some((comment_start, line_end)) if fragment.starts_with('|') => (
                hoist_comment_onto_header(&fragment, &self.source[comment_start..line_end]),
                line_end,
            ),
            _ => (fragment, e),
        };
        let fragment = if filling_in {
            fill_in(&fragment)
        } else {
            fragment
        };
        self.replace_span(s, e, &fragment)
    }

    /// The collection arm of [`set_value`](Self::set_value) (#328): a
    /// `Value::Sequence` / `Value::Mapping` replaces an existing
    /// collection node in that node's own style. `tags: [a, b]` set to
    /// `[a, c]` stays flow; a block sequence stays block, re-indented
    /// to the old value's column. The candidate document is parsed and
    /// compared against the expected typed value (the document with
    /// exactly this path replaced) *before* the splice, so a rendering
    /// the site cannot hold leaves the source byte-identical.
    ///
    /// Replacing a **scalar** with a collection is still refused — a
    /// value that must move onto its own lines is a layout decision
    /// `set` expresses with a fragment.
    fn replace_collection_value(
        &mut self,
        path: &str,
        value: &Value,
        s: usize,
        e: usize,
    ) -> Result<()> {
        let segments = parse_query_path(path);
        let (expected, tree_span) = {
            self.ensure_cache();
            let cache = self.cache.borrow();
            let (root, span_tree) = cache.as_ref().expect("ensure_cache populated");
            if !matches!(
                typed_value_at(root, &segments),
                Some(Value::Sequence(_) | Value::Mapping(_))
            ) {
                return Err(Error::Parse(
                    "set_value cannot replace a scalar with a collection (use `set` with a \
                     fragment)"
                        .into(),
                ));
            }
            let mut expected = root.clone();
            match path_value_mut(&mut expected, &segments) {
                Some(slot) => *slot = value.clone(),
                None => return Err(Error::Parse(format!("path not found: {path}"))),
            }
            let tree_span = resolve_span(root, span_tree, &segments).map(|(span, _)| span);
            (expected, tree_span)
        };
        // The green resolver mis-spans an indentless block sequence
        // (`tags:\n- a`), so prefer the loader's span tree for
        // collection nodes; then step past any leading indent the span
        // swept up, so the splice starts at the node's first byte and
        // the column below is the node's own.
        let (s, e) = tree_span.map_or((s, e), |(ts, te)| trim_value_span(&self.source, ts, te));
        let s = s + self.source[s..e].bytes().take_while(|&b| b == b' ').count();
        // One policy for anchored nodes (#338): a write into a value
        // that `*name` sites share is refused with the same guidance
        // every other mutator gives.
        self.refuse_inside_aliased_anchor("set_value", path, s)?;
        // The node's own style decides the rendering: a `[` / `{` at
        // the span start, or any flow ancestor, means flow (block
        // collections cannot nest inside flow ones).
        let flow_site = matches!(self.source.as_bytes().get(s), Some(b'[' | b'{'))
            || in_flow_collection(&self.green, s);
        let fragment = if flow_site {
            let cfg = crate::SerializerConfig::new().flow_style(crate::FlowStyle::Flow);
            crate::to_string_value_with_config(value, &cfg)?
                .trim_end_matches('\n')
                .to_owned()
        } else {
            // Block rendering starts at column 0; shift every
            // continuation line to the old value's own column so the
            // fragment sits where the node it replaces sat.
            let line_start = self.source[..s].rfind('\n').map_or(0, |i| i + 1);
            let column = s - line_start;
            let cfg = crate::SerializerConfig::new()
                .indent(self.indent_unit())
                .flow_style(crate::FlowStyle::Block);
            let rendered = crate::to_string_value_with_config(value, &cfg)?;
            let pad = " ".repeat(column);
            rendered
                .trim_end_matches('\n')
                .replace('\n', &format!("\n{pad}"))
        };
        // Oracle before the splice: the candidate must load back as
        // the document with exactly this one path replaced.
        let mut candidate = String::with_capacity(self.source.len() + fragment.len());
        candidate.push_str(&self.source[..s]);
        candidate.push_str(&fragment);
        candidate.push_str(&self.source[e..]);
        let reparsed = parse_document(&candidate).map_err(|err| {
            Error::Parse(format!(
                "set_value: replacing `{path}` with the rendered collection would not \
                 re-parse ({err}); the document was left unchanged"
            ))
        })?;
        if *reparsed.as_value() != expected {
            return Err(Error::Parse(format!(
                "set_value: replacing `{path}` failed the integrity check — the rendered \
                 collection did not load back as the value given; the document was left \
                 unchanged"
            )));
        }
        self.replace_span(s, e, &fragment)
    }

    /// Like [`set_value`](Self::set_value), but creates every missing
    /// mapping level along `path` on the way (#327, ADR-0009).
    ///
    /// A frontmatter writer setting `menu.visible` must not care
    /// whether `menu:` exists yet — the writer it replaces creates it.
    /// `set_path` resolves the deepest existing ancestor and:
    ///
    /// - **whole path exists** — behaves exactly like
    ///   [`set_value`](Self::set_value) (upsert, equal-value no-op,
    ///   scalar-only replacement);
    /// - **a block-mapping ancestor exists** — inserts the remaining
    ///   chain through
    ///   [`insert_entry_value`](Self::insert_entry_value), which owns
    ///   the indentation, quoting, and the typed-oracle guard;
    /// - **the document is empty** (nothing but comments, blank lines,
    ///   or a bare `---`) — appends the rendered chain after the
    ///   existing bytes, so a comment header survives its document's
    ///   first key.
    ///
    /// The style machinery is the same one every `*_value` mutator
    /// uses: quoting stays with [`Emit`], new levels indent at the
    /// document's [`indent_unit`](Self::indent_unit), and the edit is
    /// verified to change exactly the addressed path before it is
    /// kept.
    ///
    /// # Errors
    ///
    /// - An existing path segment resolves to a scalar (`title.x`
    ///   where `title` is a string) or to a null value other than the
    ///   empty document root; the source is left byte-identical.
    /// - A missing segment is a sequence index — `set_path` creates
    ///   mappings, never sequence items.
    /// - The nearest existing ancestor is a flow collection or an
    ///   empty `{}` — the flow inserters are tracked by #338; the
    ///   refusal is clean.
    /// - The same errors as [`set_value`](Self::set_value) /
    ///   [`insert_entry_value`](Self::insert_entry_value) otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    /// use noyalib::Value;
    ///
    /// // Creates the missing `menu:` level.
    /// let mut doc = parse_document("title: x\n").unwrap();
    /// doc.set_path("menu.visible", &Value::Bool(true)).unwrap();
    /// assert_eq!(doc.to_string(), "title: x\nmenu:\n  visible: true\n");
    ///
    /// // An empty document receives its first key.
    /// let mut doc = parse_document("").unwrap();
    /// doc.set_path("menu.visible", &Value::Bool(true)).unwrap();
    /// assert_eq!(doc.to_string(), "menu:\n  visible: true\n");
    ///
    /// // An existing leaf is an ordinary upsert.
    /// let mut doc = parse_document("menu:\n  visible: false\n").unwrap();
    /// doc.set_path("menu.visible", &Value::Bool(true)).unwrap();
    /// assert_eq!(doc.to_string(), "menu:\n  visible: true\n");
    /// ```
    pub fn set_path(&mut self, path: &str, value: &Value) -> Result<()> {
        if let Err(e) = self.validate() {
            return Err(Error::Parse(format!(
                "set_path: the document does not parse, so `{path}` cannot be resolved \
                 ({e}); the document was left unchanged"
            )));
        }
        let segments = parse_query_path(path);
        if segments.is_empty() {
            return Err(Error::Parse(
                "set_path requires a non-empty path (the document root is not an entry)".into(),
            ));
        }
        self.ensure_cache();
        let missing_from = {
            let cache = self.cache.borrow();
            let (root, _) = cache.as_ref().expect("ensure_cache populated");
            first_missing_segment(root, &segments, path)?
        };
        if missing_from == segments.len() {
            return self.set_value(path, value);
        }
        // Everything still to create must be a mapping key: a sequence
        // has no natural "missing item" to materialise.
        for segment in &segments[missing_from..] {
            match segment {
                QuerySegment::Key(_) => {}
                QuerySegment::Index(i) => {
                    return Err(Error::Parse(format!(
                        "set_path: `{path}` needs sequence item [{i}] created, and set_path \
                         creates mappings only — push the item with `push_back_value` first"
                    )));
                }
                QuerySegment::Wildcard | QuerySegment::RecursiveDescent => {
                    return Err(Error::Parse(format!(
                        "set_path: `{path}` contains a wildcard or recursive-descent segment, \
                         which does not address a single entry"
                    )));
                }
            }
        }
        // Wrap the value in one mapping per missing level below the
        // first, innermost out: `menu.theme.dark` over an existing root
        // becomes insert(root, "menu", {theme: {dark: value}}).
        let mut nested = value.clone();
        for segment in segments[missing_from + 1..].iter().rev() {
            let QuerySegment::Key(key) = segment else {
                unreachable!("non-key segments rejected above")
            };
            let mut level = Mapping::new();
            let _ = level.insert(key.clone(), nested);
            nested = Value::Mapping(level);
        }
        let QuerySegment::Key(first_missing) = &segments[missing_from] else {
            unreachable!("non-key segments rejected above")
        };

        let root_is_null = {
            let cache = self.cache.borrow();
            let (root, _) = cache.as_ref().expect("ensure_cache populated");
            matches!(root, Value::Null)
        };
        if missing_from == 0 && root_is_null {
            return self.append_first_entry(path, first_missing, nested);
        }
        let ancestor_path = format_query_prefix(&segments[..missing_from]);
        self.insert_entry_value(&ancestor_path, first_missing, &nested)
    }

    /// The empty-document arm of [`set_path`](Self::set_path): render
    /// the whole new chain and append it after the existing bytes
    /// (comments, blank lines, a bare `---`), so nothing an author
    /// wrote is disturbed. The candidate is parsed and compared to the
    /// expected typed value *before* the document is touched — a null
    /// the author spelled out (`null`, `~`) fails that check and is
    /// refused with the source byte-identical.
    fn append_first_entry(&mut self, path: &str, key: &str, nested: Value) -> Result<()> {
        let mut expected_root = Mapping::new();
        let _ = expected_root.insert(key.to_owned(), nested);
        let expected = Value::Mapping(expected_root);
        let config = crate::SerializerConfig::new().indent(self.indent_unit());
        let mut rendered = crate::to_string_value_with_config(&expected, &config)?;
        if !rendered.ends_with('\n') {
            rendered.push('\n');
        }
        let needs_break = !self.source.is_empty() && !self.source.ends_with('\n');
        let fragment = if needs_break {
            format!("\n{rendered}")
        } else {
            rendered
        };
        let mut candidate = String::with_capacity(self.source.len() + fragment.len());
        candidate.push_str(&self.source);
        candidate.push_str(&fragment);
        let candidate_value: Value = crate::from_str(&candidate).map_err(|e| {
            Error::Parse(format!(
                "set_path: `{path}` cannot be created here — the document's root is not a \
                 mapping and not empty ({e}); the document was left unchanged"
            ))
        })?;
        if candidate_value != expected {
            return Err(Error::Parse(format!(
                "set_path: `{path}` cannot be created here — the document's root already \
                 holds a non-mapping value; the document was left unchanged"
            )));
        }
        let end = self.source.len();
        self.replace_span(end, end, &fragment)
    }

    /// Remove the value at `path` along with its surrounding entry
    /// (key + colon for mappings, `-` indicator for sequences).
    /// Trailing whitespace and the line break are removed too so the
    /// surrounding entries close up with no orphan blank line.
    ///
    /// # What counts as part of the entry
    ///
    /// An entry owns the trivia a reader would say belongs to it, so a
    /// removal leaves no orphan and steals nothing from its neighbours:
    ///
    /// - **Head comment.** A contiguous run of full-line comments
    ///   directly above the entry, at its own indentation, is removed
    ///   with it. Left behind, such a comment does not merely litter —
    ///   it silently becomes documentation for the *next* entry. A blank
    ///   line detaches the run, so a document header set off by one
    ///   survives the removal of the first entry.
    /// - **Kept blank lines.** A keep-chomped (`|+` / `>+`) block
    ///   scalar's trailing blank lines are content, not separation, and
    ///   go with the entry rather than being stranded after it.
    /// - **Trailing comments stay.** A comment *after* the entry's last
    ///   content line lies outside its value span (see
    ///   [`Document::span_at`]) and conventionally documents whatever
    ///   comes next, so it is left in place. A comment *interleaved*
    ///   inside a multi-line value is inside the span and goes with the
    ///   entry.
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// // The comment documenting `database` goes with it …
    /// let mut doc = parse_document("# connection settings\ndatabase:\n  host: x\ncache: 1\n").unwrap();
    /// doc.remove("database").unwrap();
    /// assert_eq!(doc.to_string(), "cache: 1\n");
    ///
    /// // … but one that documents the following entry does not.
    /// let mut doc = parse_document("outer:\n  a: 1\n  # note for next\nnext: 2\n").unwrap();
    /// doc.remove("outer").unwrap();
    /// assert_eq!(doc.to_string(), "  # note for next\nnext: 2\n");
    /// ```
    ///
    /// Coverage (issue #221 sub-ask 4 is complete as of v0.0.23):
    /// - **Multi-line values and nested block collections** are removed
    ///   — the whole entry, from its key / `-` indicator through the
    ///   last line the value owns.
    /// - **Flow-collection members** are removed (`{x: 1, y: 2}` →
    ///   `{y: 2}`, `[1, 2, 3]` → `[1, 3]`). The member's own span goes,
    ///   plus exactly one separator: the comma after it, or — for the
    ///   last member — the comma before it. A separator sitting on
    ///   another line is not matched, so a multi-line flow collection
    ///   refuses rather than splicing something it cannot see.
    /// - **The last entry of a collection** empties that collection
    ///   rather than deleting its bytes: `a:\n  x: 1` becomes
    ///   `a:\n  {}`, and a sole sequence item leaves `[]`. Deleting the
    ///   bytes would leave a dangling `a:`, which re-parses as **null**
    ///   — a type change rather than a removal. The document's trailing
    ///   newline survives.
    ///
    /// Every path except the single-line block fast path is guarded by
    /// an eager re-parse **and** a typed-value oracle (the document
    /// minus exactly this one path); a splice that would change anything
    /// else rolls back and the document is left untouched. The fast path
    /// is kept only where the entry demonstrably owns its whole line,
    /// because the oracle's expectation is wrong for a duplicated key.
    ///
    /// # Errors
    ///
    /// - Path not found.
    /// - The entry sits inside an anchored value with live alias
    ///   references — removing it here would remove it at every
    ///   `*name` site too (#338); call
    ///   [`materialise_aliases_of`](Self::materialise_aliases_of)
    ///   first.
    /// - A flow separator that cannot be located on the member's line.
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
        let removal = {
            let cache = self.cache.borrow();
            let (value, span_tree) = cache.as_ref().expect("ensure_cache populated");
            entry_line_span(value, span_tree, &self.source, &segments, None)?
        };
        // One policy for anchored nodes (#338): an entry inside a value
        // that `*name` sites share disappears from every one of them,
        // so the removal is refused with the same guidance rename_key
        // and the inserters give.
        let removal_start = match &removal {
            Removal::Line { start, .. }
            | Removal::FlowMember { start, .. }
            | Removal::SpanWithinLine { start, .. }
            | Removal::SoleEntry { start, .. } => *start,
        };
        // A whole-line removal starts at the line's first byte — before
        // the indentation, which sits outside the anchored content span
        // — so probe from the entry's first content byte instead.
        let probe = removal_start
            + self.source[removal_start..]
                .bytes()
                .take_while(|&b| b == b' ')
                .count();
        self.refuse_inside_aliased_anchor("remove", path, probe)?;
        // What to splice, and what to put back. Only `Line` can take the
        // unguarded fast path below; the other two always face the oracle,
        // because both edit *inside* a line shared with other data.
        let (line_start, line_end, _multiline, replacement) = match removal {
            Removal::Line {
                start,
                end,
                multiline,
            } => (start, end, multiline, String::new()),
            Removal::FlowMember { start, end } => (start, end, true, String::new()),
            Removal::SpanWithinLine { start, end } => (start, end, true, String::new()),
            Removal::SoleEntry {
                start,
                end,
                empty,
                indent,
            } => (start, end, true, format!("{}{empty}", " ".repeat(indent))),
        };
        // Fast path only when the entry demonstrably owns its line.
        //
        // This used to be `if !multiline`, on the reasoning that
        // deleting one line cannot surprise anyone. A flow collection
        // breaks that: in `a: {x: 1, y: 2}` the entry `a.x` shares a
        // line with its siblings *and* its parent, so "delete the line"
        // removed the whole `a` entry — and for a single-entry
        // document, the whole document — while returning `Ok`. Silent
        // data loss.
        //
        // The test is whether the entry's own key starts the line. If
        // it does, the line is the entry and splicing it is safe. If it
        // does not, something else shares the line and the typed oracle
        // below has to arbitrate.
        //
        // Keeping a fast path at all matters: the oracle compares
        // against "the document with this path absent", which is the
        // wrong expectation for a duplicated key. `remove("k")` on
        // `k: one\nk: two` deletes the winning occurrence and leaves
        // the shadowed one, so the oracle would refuse an edit that is
        // both intended and tested.
        let owns_its_line = matches!(removal, Removal::Line { .. })
            && self
                .key_span(path)
                .is_some_and(|(ks, _)| self.source[line_start..ks].trim().is_empty());
        if !_multiline && owns_its_line {
            return self.replace_span(line_start, line_end, "");
        }

        // Multi-line / nested block value: the splice removes several
        // lines, so guard it with a snapshot, an eager re-parse, and a
        // typed oracle — the document with exactly this path removed.
        let expected = {
            let cache = self.cache.borrow();
            let (value, _) = cache.as_ref().expect("ensure_cache populated");
            expected_after_remove(value, &segments)?
        };
        let snapshot = self.clone();
        if let Err(e) = self.replace_span(line_start, line_end, &replacement) {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "remove: removing `{path}` could not be spliced ({e}); \
                 the document was left unchanged"
            )));
        }
        if let Err(e) = self.validate() {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "remove: removing `{path}` left the document unable to re-parse ({e}); \
                 the document was left unchanged"
            )));
        }
        if *self.as_value() != expected {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "remove: removing `{path}` failed the integrity check — the edit would \
                 change data beyond the removed entry; the document was left unchanged"
            )));
        }
        Ok(())
    }

    /// Rename the key of the mapping entry at `path` to `new_key`,
    /// leaving every other byte — the `:`, the value, whitespace,
    /// comments, and sibling entries — untouched.
    ///
    /// `path` addresses the entry the same way [`Document::set`] and
    /// [`Document::remove`] address it: the path points at the
    /// entry's *value*; the operation rewrites that entry's *key*
    /// token.
    ///
    /// `new_key`'s spelling is *style-matched to the key it
    /// replaces*: a plain key stays plain when `new_key`'s plain
    /// spelling re-parses to exactly that string, a single-quoted
    /// key stays single-quoted, a double-quoted key stays
    /// double-quoted. Quoting is forced only when the plain
    /// spelling would not re-parse to `new_key` (`a: b`, `-flag`,
    /// `8080`, `true`) — a plain site then falls back to double
    /// quotes.
    ///
    /// Renaming a key to its current spelling is a no-op — `Ok(())`
    /// with no bytes modified. "Current spelling" is decided on the
    /// *decoded* key, so a plain `true:` renamed to `"true"` stays
    /// plain rather than being requoted. The guarantee applies to
    /// every path that resolves to a mapping entry; paths that fail
    /// to resolve at all (alias-addressed content, keys produced by
    /// a `<<` merge) report their resolution error instead.
    ///
    /// After the splice the document must re-parse cleanly **and**
    /// its typed value must equal the old value with exactly that
    /// one key renamed — same entry position, same value. If either
    /// check fails, the document is rolled back to its previous
    /// state and an error is returned.
    ///
    /// Restrictions in this phase:
    /// - Both block-mapping and flow-mapping entries rename (#338);
    ///   in flow context a new key whose plain spelling would read as
    ///   flow structure (`,` `[` `]` `{` `}`) is double-quoted.
    /// - The entry's key must be a simple scalar token (plain,
    ///   single-quoted, or double-quoted). Alias keys (`*name :`)
    ///   are rejected. Explicit complex keys (`? [a, b]`) are not
    ///   addressable by the path syntax in the first place — their
    ///   stringified form contains bracket segments, which the path
    ///   parser reads as sequence indices — so they cannot be
    ///   renamed; the surrounding mapping's other entries rename
    ///   normally.
    ///
    /// # Errors
    ///
    /// - Path not found, or it does not address a mapping entry
    ///   (e.g. it ends in a sequence index).
    /// - `path` contains a bracket segment that is not a
    ///   non-negative integer (`servers[web]`) — the shared path
    ///   parser drops such a segment, which would rename the
    ///   *parent* key, so `rename_key` refuses it outright.
    /// - `new_key` is `<<`: the loader treats a `<<` key as a merge
    ///   directive whatever its quote style, so the rename cannot
    ///   round-trip.
    /// - `new_key` contains a non-printable character (any control
    ///   character other than tab, `U+007F`, or a `U+0080..=U+009F`
    ///   C1 control) — YAML's printable set excludes them and no
    ///   scalar style can spell them here.
    /// - Restrictions above.
    /// - The containing mapping already has a *different* entry
    ///   whose key equals `new_key` — the rename would create a
    ///   duplicate and silently change data. Reported separately
    ///   when that sibling comes from a `<<` merge rather than from
    ///   the mapping's own source entries.
    /// - The addressed key has no entry of its own because a `<<`
    ///   merge key produced it — the key lives in the merged
    ///   mapping, so that is where it must be renamed.
    /// - The path is reached *through* an alias (`*name`): the
    ///   bytes at that site belong to the anchor, so the anchor's
    ///   own entry must be renamed instead.
    /// - The entry lies inside an anchored value that has alias
    ///   references — the rename would propagate to every `*name`
    ///   site. Call [`Document::materialise_aliases_of`] first.
    /// - The re-parse / integrity guard above; the document is left
    ///   unchanged.
    /// - The document no longer parses (an earlier edit left it in
    ///   the optimistically-committed broken state — see
    ///   [`Document::validate`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("name: foo  # the project\nversion: 0.0.1\n").unwrap();
    /// doc.rename_key("name", "title").unwrap();
    /// assert_eq!(doc.to_string(), "title: foo  # the project\nversion: 0.0.1\n");
    /// ```
    ///
    /// A new key that is not plain-safe is quoted automatically:
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("name: foo\n").unwrap();
    /// doc.rename_key("name", "a: b").unwrap();
    /// assert_eq!(doc.to_string(), "\"a: b\": foo\n");
    /// assert_eq!(doc.as_value()["a: b"].as_str(), Some("foo"));
    /// ```
    pub fn rename_key(&mut self, path: &str, new_key: &str) -> Result<()> {
        // An earlier edit may have left the document in the
        // optimistically-committed broken state (see `validate`), in
        // which case `ensure_cache` would panic. Surface it as an
        // error instead — `rename_key` returns `Result` and
        // documents no panics.
        self.validate().map_err(|e| {
            Error::Parse(format!(
                "rename_key: the document does not parse, so `{path}` cannot be resolved \
                 ({e}); the document was left unchanged"
            ))
        })?;
        let segments = parse_rename_path(path)?;

        // Spelling refusals that no scalar style can work around,
        // checked before any resolution so the diagnosis names the
        // argument rather than whatever the splice happened to break.
        if new_key == MERGE_KEY_SPELLING {
            return Err(Error::Parse(format!(
                "rename_key: `{MERGE_KEY_SPELLING}` cannot be used as a key name — the loader \
                 treats any `{MERGE_KEY_SPELLING}` key as a merge directive whatever its quote \
                 style, so the renamed entry would not round-trip as a key"
            )));
        }
        if let Some(bad) = first_non_printable(new_key) {
            return Err(Error::Parse(format!(
                "rename_key: the new key contains the non-printable character U+{:04X}, which \
                 is outside YAML's printable character set — mapping keys may not carry control \
                 characters (tab excepted)",
                bad as u32
            )));
        }

        // Resolve the entry's key span via the typed cache — the
        // same resolver family `remove` uses (`entry_line_span`
        // computes this key span and discards its end; here it is
        // the target). The sibling-duplicate refusal happens during
        // resolution, where the containing mapping is at hand.
        let (key_start, key_end) = {
            let cache = self.cache.borrow();
            let (value, span_tree) = cache.as_ref().expect("validate populated the cache");
            entry_key_site(value, span_tree, &segments, new_key)?
        };
        if key_start == key_end {
            // The loader records a zero-width key span for keys that
            // are not a single scalar node — in practice alias keys
            // (`*name :`); an explicit complex key (`? [a, b]`) has
            // one too, but its stringified form is not addressable
            // by the path syntax, so it never reaches here.
            return Err(Error::Parse(format!(
                "rename_key: the key at `{path}` is not a simple scalar token \
                 (alias keys cannot be renamed)"
            )));
        }

        // Green-tree guards: the addressed key must be a scalar
        // token that belongs to a *block* mapping entry.
        let (token_kind, (tok_start, tok_end), parent_kind) =
            token_at_with_parent(&self.green, key_start, 0).ok_or_else(|| {
                Error::Parse(format!(
                    "rename_key: could not locate the key token for `{path}`"
                ))
            })?;

        // The scanner captures a plain scalar at end-of-line with
        // its trailing line break (see `anchored_scalar_text`) — an
        // explicit key (`? foo`) ends its line, so keep separator
        // whitespace out of the splice.
        let (tok_start, tok_end) = trim_trailing_blank(&self.source, tok_start, tok_end);

        // No-op check, decided on the *decoded* key rather than on
        // the spelling `format_key_for_site` would produce: a plain
        // `true:` renamed to `"true"` must stay plain, not be
        // requoted into a different YAML type. It runs before the
        // remaining refusals so a same-name rename is `Ok(())`
        // wherever the entry resolves at all — including inside a
        // flow mapping, whose renames are otherwise a follow-up.
        if let Some(current) = decode_key_token(&self.source[tok_start..tok_end], token_kind) {
            if current == new_key {
                return Ok(());
            }
        }

        // Both block-mapping entries and flow-mapping entries are
        // renameable (#338): the key token is a scalar span either
        // way, and the splice-then-oracle tail below is style-blind.
        if !matches!(
            parent_kind,
            SyntaxKind::MappingEntry | SyntaxKind::FlowMapping
        ) {
            return Err(Error::Parse(format!(
                "rename_key: `{path}` does not address a mapping entry key"
            )));
        }
        if !matches!(
            token_kind,
            SyntaxKind::PlainScalar
                | SyntaxKind::SingleQuotedScalar
                | SyntaxKind::DoubleQuotedScalar
        ) {
            return Err(Error::Parse(format!(
                "rename_key: the key at `{path}` is not a simple scalar token \
                 (alias keys cannot be renamed)"
            )));
        }

        // An entry inside an anchored value is shared with every
        // `*name` site: renaming the key here renames it at all of
        // them, which the integrity oracle would reject as an
        // unrelated "duplicate key". Diagnose the real cause first.
        if let Some((anchor, alias_count)) = self.aliased_anchor_covering(tok_start) {
            return Err(Error::Parse(format!(
                "rename_key: `{path}` is inside the value anchored by `&{anchor}`, which has \
                 {alias_count} alias reference(s) — renaming the key here would rename it at \
                 every `*{anchor}` site too; call `materialise_aliases_of(\"{anchor}\")` first \
                 to give each site its own copy, then rename"
            )));
        }

        // Spell the new key, style-matched to the token it replaces
        // (plain stays plain when the plain spelling re-parses to
        // `new_key`, quoted stays quoted in the same style). In flow
        // context `,` `[` `]` `{` `}` are structural anywhere in a
        // plain scalar, so a plain spelling unsafe there is quoted.
        let replacement = format_key_for_site(new_key, token_kind);
        let replacement = if parent_kind == SyntaxKind::FlowMapping
            && !replacement.starts_with(['"', '\''])
            && !is_plain_safe_in_flow(&replacement)
        {
            format_double_quoted(new_key)
        } else {
            replacement
        };
        if replacement == self.source[tok_start..tok_end] {
            // Spelling-identical after formatting — nothing to splice.
            return Ok(());
        }

        // Snapshot for rollback, and the integrity oracle: the old
        // typed value with exactly this one key renamed in place.
        let snapshot = self.clone();
        let expected = {
            let cache = self.cache.borrow();
            let (value, _) = cache.as_ref().expect("validate populated the cache");
            expected_after_rename(value, &segments, new_key)?
        };

        // Post-splice guards. Every failure below is reported in
        // `rename_key`'s own terms — a raw loader error would say
        // nothing about the path, the new key, or the rollback.
        if let Err(e) = self.replace_span(tok_start, tok_end, &replacement) {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "rename_key: renaming `{path}` to `{new_key}` could not be spliced ({e}); \
                 the document was left unchanged"
            )));
        }

        // Re-parse guard. `replace_span`'s local-repair fast path
        // commits optimistically (see `validate`), so run the eager
        // document-level check here and compare the typed view
        // against the oracle. Roll back on any mismatch.
        if let Err(e) = self.validate() {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "rename_key: renaming `{path}` to `{new_key}` left the document unable to \
                 re-parse ({e}); the document was left unchanged"
            )));
        }
        let matches_expected = *self.as_value() == expected;
        if !matches_expected {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "rename_key: renaming `{path}` to `{new_key}` failed the integrity \
                 check — the edit would change data beyond the single renamed key \
                 (e.g. a duplicate of the old key elsewhere in the mapping); \
                 the document was left unchanged"
            )));
        }
        Ok(())
    }

    /// Swap two items of the block sequence at `path`, exchanging each
    /// item's **whole entry** — its own lines, its head-comment run
    /// included. Every other item, and the surrounding structure, stay
    /// byte-identical.
    ///
    /// An item owns the same range here that [`remove`](Self::remove)
    /// deletes — `owned_entry_range` computes both. That is deliberate:
    /// the two
    /// have to agree about who a comment belongs to, or the same bytes
    /// are the entry's property under one call and the slot's under the
    /// other. A reorder that moved only value bytes would leave each
    /// comment annotating whichever item landed beneath it, silently
    /// and at `Ok`.
    ///
    /// A **flow** sequence has no per-item lines to exchange, so its
    /// members keep the narrower value-span swap.
    ///
    /// Guarded like the other mutators: after the two splices the
    /// document must re-parse **and** its typed value must equal the
    /// original with exactly items `i` and `j` exchanged, or the edit
    /// is rolled back and the document is left untouched.
    ///
    /// Swapping an index with itself, or two items whose values are
    /// already equal, is a no-op that returns `Ok(())`.
    ///
    /// # Errors
    ///
    /// - `path` does not resolve to a sequence.
    /// - `i` or `j` is out of bounds for that sequence.
    /// - The bytes of an item could not be located.
    /// - The splice would not re-parse, or fails the integrity check
    ///   above (both roll back).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("- a\n- b\n- c\n").unwrap();
    /// doc.swap_items("", 0, 2).unwrap();
    /// assert_eq!(doc.source(), "- c\n- b\n- a\n");
    /// ```
    ///
    /// A comment travels with the item it documents:
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("# about one\n- one\n# about two\n- two\n").unwrap();
    /// doc.swap_items("", 0, 1).unwrap();
    /// assert_eq!(doc.source(), "# about two\n- two\n# about one\n- one\n");
    /// ```
    pub fn swap_items(&mut self, path: &str, i: usize, j: usize) -> Result<()> {
        let segments = parse_query_path(path);
        self.ensure_cache();
        let len = {
            let cache = self.cache.borrow();
            let (value, _) = cache.as_ref().expect("ensure_cache populated");
            sequence_len_at(value, &segments, path)?
        };
        if i >= len || j >= len {
            return Err(Error::Parse(format!(
                "swap_items: index out of bounds for the sequence at `{path}` \
                 (length {len}): requested {i} and {j}"
            )));
        }
        if i == j {
            return Ok(());
        }

        let (pi, pj) = (item_child_path(path, i), item_child_path(path, j));
        let span_i = self.span_at(&pi).ok_or_else(|| {
            Error::Parse(format!("swap_items: could not locate item {i} of `{path}`"))
        })?;
        let span_j = self.span_at(&pj).ok_or_else(|| {
            Error::Parse(format!("swap_items: could not locate item {j} of `{path}`"))
        })?;

        // What each range receives. Whole entries when both items own
        // their lines; bare value spans for a flow member, which does
        // not.
        let (range_i, range_j, into_i, into_j) = match self.owned_item_ranges(&pi, &pj) {
            Some((ri, rj)) => {
                let (body_i, term_i) = split_line_terminator(&self.source[ri.0..ri.1]);
                let (body_j, term_j) = split_line_terminator(&self.source[rj.0..rj.1]);
                // Each *position* keeps its own line terminator while the
                // bodies move. The last entry of a document may have none,
                // and carrying the breaks along with the bodies would join
                // two lines into one: `- a\n- b` would swap to `- b- a`.
                (
                    ri,
                    rj,
                    format!("{body_j}{term_i}"),
                    format!("{body_i}{term_j}"),
                )
            }
            None => (
                span_i,
                span_j,
                self.source()[span_j.0..span_j.1].to_string(),
                self.source()[span_i.0..span_i.1].to_string(),
            ),
        };

        // Integrity oracle: the old value with items i and j exchanged.
        let expected = {
            let cache = self.cache.borrow();
            let (value, _) = cache.as_ref().expect("ensure_cache populated");
            expected_after_swap(value, &segments, i, j, path)?
        };

        let snapshot = self.clone();
        // Replace the *later* range first so the earlier range's byte
        // offsets stay valid for the second splice.
        let (lo, hi, lo_text, hi_text) = if range_i.0 < range_j.0 {
            (range_i, range_j, &into_i, &into_j)
        } else {
            (range_j, range_i, &into_j, &into_i)
        };
        for (span, text) in [(hi, hi_text), (lo, lo_text)] {
            if let Err(e) = self.replace_span(span.0, span.1, text) {
                *self = snapshot;
                return Err(Error::Parse(format!(
                    "swap_items: swapping items {i} and {j} of `{path}` could not be \
                     spliced ({e}); the document was left unchanged"
                )));
            }
        }
        if let Err(e) = self.validate() {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "swap_items: swapping items {i} and {j} of `{path}` left the document \
                 unable to re-parse ({e}); the document was left unchanged"
            )));
        }
        if *self.as_value() != expected {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "swap_items: swapping items {i} and {j} of `{path}` failed the integrity \
                 check — the exchange would change data beyond the two items; \
                 the document was left unchanged"
            )));
        }
        Ok(())
    }

    /// The whole-line ranges the items at `pi` and `pj` own, or `None`
    /// when either has no lines of its own.
    ///
    /// `None` is the flow-member case — `[one, two]`, where the items
    /// share a line with each other and with the brackets — and it is
    /// also the safe answer for a path that does not resolve, leaving
    /// the caller on the value-span path it used before.
    fn owned_item_ranges(&self, pi: &str, pj: &str) -> Option<((usize, usize), (usize, usize))> {
        let cache = self.cache.borrow();
        let (value, span_tree) = cache.as_ref()?;
        let owned = |p: &str| match entry_line_span(
            value,
            span_tree,
            &self.source,
            &parse_query_path(p),
            None,
        ) {
            Ok(Removal::Line { start, end, .. }) => Some((start, end)),
            _ => None,
        };
        let (ri, rj) = (owned(pi)?, owned(pj)?);
        // Two distinct items own disjoint ranges — a comment run between
        // them belongs to the one below it, and `owned_value_end` keeps
        // the one above from claiming it. Checked rather than assumed:
        // overlapping ranges would have the second splice write into
        // bytes the first had already moved, which the typed oracle
        // cannot catch because the result still parses.
        if ri.0 < rj.1 && rj.0 < ri.1 {
            return None;
        }
        Some((ri, rj))
    }

    /// Move the item at `from` to index `to` in the block sequence at
    /// `path`, shifting the items in between by one. The move is
    /// applied as a run of adjacent [`swap_items`](Self::swap_items)
    /// steps, so it inherits that method's guarantees — each item's
    /// whole entry moves, its comments with it, structure is preserved,
    /// and each step is guarded — and the whole move is **atomic**: if
    /// any step is refused, the document is rolled back to its state
    /// before the call.
    ///
    /// Moving an index to itself is a no-op that returns `Ok(())`.
    ///
    /// # Errors
    ///
    /// - `path` does not resolve to a sequence.
    /// - `from` or `to` is out of bounds for that sequence.
    /// - Any underlying swap is refused; the document is left unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("- a\n- b\n- c\n- d\n").unwrap();
    /// doc.move_item("", 0, 2).unwrap();
    /// assert_eq!(doc.source(), "- b\n- c\n- a\n- d\n");
    /// ```
    pub fn move_item(&mut self, path: &str, from: usize, to: usize) -> Result<()> {
        let segments = parse_query_path(path);
        self.ensure_cache();
        let len = {
            let cache = self.cache.borrow();
            let (value, _) = cache.as_ref().expect("ensure_cache populated");
            sequence_len_at(value, &segments, path)?
        };
        if from >= len || to >= len {
            return Err(Error::Parse(format!(
                "move_item: index out of bounds for the sequence at `{path}` \
                 (length {len}): from {from}, to {to}"
            )));
        }
        if from == to {
            return Ok(());
        }

        let snapshot = self.clone();
        let mut failure = None;
        if from < to {
            for k in from..to {
                if let Err(e) = self.swap_items(path, k, k + 1) {
                    failure = Some(e);
                    break;
                }
            }
        } else {
            let mut k = from;
            while k > to {
                if let Err(e) = self.swap_items(path, k, k - 1) {
                    failure = Some(e);
                    break;
                }
                k -= 1;
            }
        }
        if let Some(e) = failure {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "move_item: moving item {from} to {to} in `{path}` failed ({e}); \
                 the document was left unchanged"
            )));
        }
        Ok(())
    }

    /// The anchor covering byte `pos` that has at least one `*name`
    /// reference, with that reference count.
    ///
    /// `Document::rename_key` uses this to refuse a rename whose
    /// bytes are shared with alias sites *before* splicing, so the
    /// user gets the anchor's name instead of a downstream integrity
    /// complaint. `None` when `pos` is outside every anchored value,
    /// or the anchors covering it have no aliases (then the rename
    /// is local and safe).
    fn aliased_anchor_covering(&self, pos: usize) -> Option<(String, usize)> {
        for anchor in self.anchors() {
            let Some((start, end)) = anchored_content_span(&self.green, 0, anchor.mark_span.0)
            else {
                continue;
            };
            if pos < start || pos >= end {
                continue;
            }
            let count = self.aliases_of(&anchor.name).len();
            if count > 0 {
                return Some((anchor.name, count));
            }
        }
        None
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
    /// - The fragment changed the document beyond the single item
    ///   asked for — reaching outside the sequence (`"v\nqq: 7"`) or
    ///   smuggling extra items into it (`"v\n  - w"`); the document
    ///   is left unchanged.
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
        let p = path.to_owned();
        let f = fragment.to_owned();
        self.guarded_insert(path, "push_back", InsertGrowth::SeqPlusOne, move |d| {
            d.push_back_inner(&p, &f)
        })
    }

    fn push_back_inner(&mut self, path: &str, fragment: &str) -> Result<()> {
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
        let lead = leading_break_for_splice(&self.source, line_end);
        let nl = document_break(&self.source);
        let new_line = format!("{lead}{indent}- {fragment}{nl}");
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
    /// Only the *fragment* is verbatim YAML. The key is a **name**: a
    /// spelling whose plain form would not re-parse to it (`a: b`,
    /// `[x]`, a leading `- `) is quoted automatically, exactly as
    /// [`Document::rename_key`] documents for its new key, and the
    /// existing-key check reads the mapping's own entries — a key
    /// holding `.` or `[` (`app.io/name`, ubiquitous in Kubernetes
    /// labels) inserts as that literal key rather than resolving
    /// through the path syntax.
    ///
    /// # The fragment cannot reach outside the entry
    ///
    /// After the splice, the document's shape outside `mapping_path`
    /// must be unchanged and the mapping must have gained exactly the
    /// one entry asked for — a fragment or key that smuggles sibling
    /// entries (a line break the splice never intended, `v\rc: 3`) is
    /// refused and the document left untouched, the same oracle
    /// [`Document::set`] and [`Document::push_back`] apply.
    ///
    /// # Errors
    ///
    /// - `mapping_path` does not resolve to a mapping.
    /// - The mapping is empty (no anchor for indentation; use `set`
    ///   with a fragment instead).
    /// - `key` is `<<` (the loader reads any `<<` key as a merge
    ///   directive, whatever its quote style) or carries a
    ///   non-printable character.
    /// - `key` already exists but contains `.` or `[`, which the path
    ///   syntax cannot address to replace its value — `remove` the
    ///   entry and insert it afresh, or splice it with `set`.
    /// - The fragment added or removed entries beyond the one asked
    ///   for (the integrity oracle above); the document is left
    ///   unchanged.
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
        // The key half is a *name*, not YAML. `rename_key` and
        // `insert_entry_value` already refuse the spellings no quote
        // style can carry; the verbatim tier held the last hole — a
        // key holding a line break spliced sibling entries the caller
        // never asked for.
        if key == MERGE_KEY_SPELLING {
            return Err(Error::Parse(format!(
                "insert_entry: `{MERGE_KEY_SPELLING}` cannot be used as a key name — the loader \
                 treats any `{MERGE_KEY_SPELLING}` key as a merge directive whatever its quote \
                 style, so the entry would not round-trip as a key"
            )));
        }
        if let Some(bad) = first_non_printable(key) {
            return Err(Error::Parse(format!(
                "insert_entry: the key contains the non-printable character U+{:04X}, which is \
                 outside YAML's printable character set — mapping keys may not carry control \
                 characters (tab excepted)",
                bad as u32
            )));
        }

        // Existing-key upsert, decided on the mapping's own entries
        // rather than on a composed path: `"{path}.{key}"` means
        // something else entirely for a key holding `.` or `[`
        // (`app.io/name`, ubiquitous in Kubernetes labels), and used
        // to overwrite whatever *nested* entry the composition
        // happened to resolve.
        let child_path = if mapping_path.is_empty() {
            key.to_owned()
        } else {
            format!("{mapping_path}.{key}")
        };
        let addressable = !key.contains('.') && !key.contains('[');
        self.ensure_cache();
        let in_mapping = {
            let cache = self.cache.borrow();
            let (doc_value, _) = cache.as_ref().expect("ensure_cache populated");
            let target = if mapping_path.is_empty() {
                Some(doc_value)
            } else {
                path_value(doc_value, mapping_path)
            };
            matches!(target, Some(Value::Mapping(m)) if m.get(key).is_some())
        };
        if in_mapping && !addressable {
            return Err(Error::Parse(format!(
                "insert_entry: `{mapping_path}` already has a key `{key}`, and a key containing \
                 `.` or `[` cannot be addressed by the path syntax to replace its value — \
                 `remove` the entry and insert it afresh, or splice it with `set`"
            )));
        }
        // A key token of its own means the entry is here (an implicit
        // null included); a key present in the typed view *without*
        // one is inherited through a `<<` merge, and the insert
        // appends an explicit override instead.
        if in_mapping && self.key_span(&child_path).is_some() {
            return self.set(&child_path, fragment);
        }

        let p = mapping_path.to_owned();
        let k = key.to_owned();
        let f = fragment.to_owned();
        self.guarded_insert(
            mapping_path,
            "insert_entry",
            InsertGrowth::MapEntry(key),
            move |d| d.insert_entry_splice(&p, &k, &f),
        )
    }

    /// The new-key splice for [`Document::insert_entry`]: a sibling
    /// line after the mapping's last entry. Runs under
    /// [`Document::guarded_insert`].
    fn insert_entry_splice(&mut self, mapping_path: &str, key: &str, fragment: &str) -> Result<()> {
        // The anchor comes from `mapping_insert_anchor`, which reads
        // the target mapping's own entries out of the span tree; this
        // used to take the last key from the typed view and compose it
        // back into a path string, which no key holding a `.` or `[`
        // survives.
        self.ensure_cache();
        let (key_col, line_end, _) = self.mapping_insert_anchor(mapping_path)?;
        let indent: String = " ".repeat(key_col);
        // The key adopts the site's spelling, quoted when its plain
        // form would not re-parse to the name given — the same
        // courtesy `rename_key` documents for its new key.
        let ctx = self.emit_ctx_at(key_col, mapping_path);
        let key = emit_key(key, &ctx);

        // Single-line values (scalars, flow collections, anything
        // without an interior newline) splice inline. Multi-line
        // fragments — typically the YAML emission of a nested block
        // mapping or sequence — splice as `{key}:\n{children}` with
        // the children re-indented by `key_col + indent_unit` so the
        // nested structure lines up with the surrounding file's
        // convention (Phase 2.2).
        // The fragment is a caller-supplied or generated emission and is
        // always `\n`-separated; only the breaks this splice *adds* take
        // the document's spelling.
        let nl = document_break(&self.source);
        let new_line = if fragment.contains('\n') {
            let unit = detect_indent_unit(&self.source);
            let inner_indent: String = " ".repeat(key_col + unit);
            // Strip leading blank lines so a caller that prefixed `\n`
            // to force block form (see `Entry::insert_value` for a
            // single-entry collection) does not introduce a stray
            // blank between the key and its first child.
            let body = fragment.trim_start_matches('\n');
            let mut buf = format!("{indent}{key}:{nl}");
            for line in body.split('\n') {
                if line.is_empty() {
                    buf.push_str(nl);
                } else {
                    buf.push_str(&inner_indent);
                    buf.push_str(line);
                    buf.push_str(nl);
                }
            }
            buf
        } else {
            format!("{indent}{key}: {fragment}{nl}")
        };
        let lead = leading_break_for_splice(&self.source, line_end);
        self.replace_span(line_end, line_end, &format!("{lead}{new_line}"))
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
    /// - The fragment changed the document beyond the single item
    ///   asked for — the same containment oracle
    ///   [`Document::push_back`] documents; the document is left
    ///   unchanged.
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
        // The container is the parent sequence: `items[2]` -> `items`.
        let container = item_path
            .rfind('[')
            .map_or_else(|| item_path.to_owned(), |b| item_path[..b].to_owned());
        let p = item_path.to_owned();
        let f = fragment.to_owned();
        self.guarded_insert(
            &container,
            "insert_after",
            InsertGrowth::SeqPlusOne,
            move |d| d.insert_after_inner(&p, &f),
        )
    }

    fn insert_after_inner(&mut self, item_path: &str, fragment: &str) -> Result<()> {
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
        let lead = leading_break_for_splice(&self.source, line_end);
        let nl = document_break(&self.source);
        let new_line = format!("{lead}{indent}- {fragment}{nl}");
        self.replace_span(line_end, line_end, &new_line)
    }

    // ── Auto-formatting insertion (the `Emit` tier) ─────────────────

    /// The emission context for a site starting at `column`: the
    /// conventions of the collection being edited, so an insertion looks
    /// like the lines beside it.
    ///
    /// `site` is the path of the collection receiving the entry.
    /// When it holds scalars they decide the quote style — **plain ones
    /// included**. Only when the site offers no evidence at all (an empty
    /// collection) does this fall back to the document-wide vote.
    ///
    /// The document-wide vote is deliberately unchanged. It counts quoted
    /// scalars against each other and ignores plain ones, so one quoted
    /// line anywhere decided the spelling of every later insertion: on a
    /// Kubernetes manifest, `value: "30"` in a container's env block
    /// dictated the spelling of a label four lines from the top (#290).
    /// `EmitCtx`'s own doc says an implementation should "match the file
    /// it is landing in" — landing in is a *site*, and the whole document
    /// was the wrong radius rather than the wrong idea.
    ///
    /// `Document::dominant_quote_style` is public with documented
    /// behaviour and three doctests pinning it, so its meaning stays as
    /// it is; this narrows what *insertion* asks, not what the function
    /// answers.
    fn emit_ctx_at(&self, column: usize, site: &str) -> EmitCtx {
        EmitCtx::new(
            self.quote_style_for_site(site),
            self.dominant_flow_style(),
            self.indent_unit(),
            column,
        )
    }

    /// Quote style for an insertion landing in the collection at `path`,
    /// falling back to the document when that collection has no scalar
    /// values to learn from.
    ///
    /// Reads the collection's entry **values** from the span tree rather
    /// than counting scalar tokens in a byte range. A byte range cannot
    /// tell a key from a value, and mapping keys are almost always plain
    /// — so `a: "one"` / `b: "two"` counts two plain keys against two
    /// quoted values and ties to plain, which is the opposite of what the
    /// site says. Values are the only scalars an insertion imitates.
    fn quote_style_for_site(&self, path: &str) -> crate::ScalarStyle {
        let counts = {
            let cache = self.cache.borrow();
            cache.as_ref().and_then(|(value, tree)| {
                let (_, sub) = resolve_tree(value, tree, &parse_query_path(path))?;
                let mut counts = (0_usize, 0_usize, 0_usize);
                let mut tally = |t: &SpanTree| {
                    // Only leaves are scalars; a nested collection has no
                    // spelling of its own to copy.
                    if let SpanTree::Leaf(start, _) = t {
                        match self.source.as_bytes().get(*start) {
                            Some(b'"') => counts.2 += 1,
                            Some(b'\'') => counts.1 += 1,
                            Some(_) => counts.0 += 1,
                            None => {}
                        }
                    }
                };
                match sub {
                    SpanTree::Mapping { entries, .. } => {
                        for (_, v) in entries {
                            tally(v);
                        }
                    }
                    SpanTree::Sequence { items, .. } => {
                        for item in items {
                            tally(item);
                        }
                    }
                    _ => return None,
                }
                Some(counts)
            })
        };
        match counts {
            Some((plain, single, double)) if plain + single + double > 0 => {
                // Plain needs a *strict* majority. A tie means the site is
                // genuinely mixed — `a: 1` beside `b: 'two'` — and there
                // the existing quoting is the better guide than a
                // preference for bare scalars. #290 is about unrelated
                // lines deciding the spelling, not about mixed sites, and
                // every case it reports has plain winning outright.
                if plain > single && plain > double {
                    crate::ScalarStyle::Plain
                } else if single >= double {
                    crate::ScalarStyle::SingleQuoted
                } else {
                    crate::ScalarStyle::DoubleQuoted
                }
            }
            _ => self.dominant_quote_style(),
        }
    }

    /// Insert `key: value` into the block mapping at `mapping_path`,
    /// formatting **both** halves so they re-parse to exactly the key
    /// and value given.
    ///
    /// The typed counterpart of [`Document::insert_entry`], which
    /// splices its `&str` arguments verbatim: `insert_entry(m, "k",
    /// "a: b")` grows a nested mapping, where
    /// `insert_entry_value(m, "k", "a: b")` inserts the *string*
    /// `"a: b"`. Quoting follows the file's dominant scalar style
    /// except where that style would misrepresent the data, in which
    /// case quoting is forced (see [`Emit`]).
    ///
    /// When `key` already exists its value is replaced in place;
    /// otherwise a sibling line is appended after the mapping's last
    /// entry, indented to match.
    ///
    /// After the splice the document must re-parse **and** its typed
    /// value must equal the pre-edit value with exactly this one entry
    /// set, or the edit is rolled back — the guard the verbatim path
    /// cannot offer, since a fragment that restructures the document
    /// is still valid YAML.
    ///
    /// An existing key is an **upsert**: its value is rewritten in place,
    /// including when that value is an implicit null (`a:`), which is an entry
    /// the mapping already has rather than one to append. A key it only
    /// *inherits* through a `<<` merge has no entry here at all, so an
    /// explicit one is created to override it.
    ///
    /// A new key into a **flow** mapping — `{a: 1}`, `{}`, or a whole
    /// document spelled as one — splices `, key: value` before the
    /// closing brace (#338); only single-line flow mappings accept
    /// inserts.
    ///
    /// # Errors
    ///
    /// - `mapping_path` does not resolve to a mapping, or an empty
    ///   **block**-context mapping leaves no entry to anchor
    ///   indentation on (use [`Document::set`] with a fragment).
    /// - The flow mapping at `mapping_path` spans more than one line.
    /// - `key` is `<<` (the loader reads any `<<` key as a merge
    ///   directive, whatever its quote style) or carries a
    ///   non-printable character.
    /// - The value has no auto-formatted spelling (see [`Emit::emit`]).
    /// - The splice would not re-parse, or fails the integrity check
    ///   above; the document is left unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("labels:\n  app: noyalib\n").unwrap();
    /// doc.insert_entry_value("labels", "version", "8080").unwrap();
    /// // Quoted: the plain spelling would load as a number.
    /// assert_eq!(
    ///     doc.to_string(),
    ///     "labels:\n  app: noyalib\n  version: \"8080\"\n",
    /// );
    /// ```
    pub fn insert_entry_value<E: Emit + ?Sized>(
        &mut self,
        mapping_path: &str,
        key: &str,
        value: &E,
    ) -> Result<()> {
        if let Err(e) = self.validate() {
            return Err(Error::Parse(format!(
                "insert_entry_value: the document does not parse, so `{mapping_path}` cannot \
                 be resolved ({e}); the document was left unchanged"
            )));
        }
        if key == MERGE_KEY_SPELLING {
            return Err(Error::Parse(format!(
                "insert_entry_value: `{MERGE_KEY_SPELLING}` cannot be used as a key name — the \
                 loader treats any `{MERGE_KEY_SPELLING}` key as a merge directive whatever its \
                 quote style, so the entry would not round-trip as a key"
            )));
        }
        if let Some(bad) = first_non_printable(key) {
            return Err(Error::Parse(format!(
                "insert_entry_value: the key contains the non-printable character U+{:04X}, \
                 which is outside YAML's printable character set — mapping keys may not carry \
                 control characters (tab excepted)",
                bad as u32
            )));
        }

        let expected_child = value.expected_value()?;
        let expected = {
            let cache = self.cache.borrow();
            let (doc_value, _) = cache.as_ref().expect("validate populated the cache");
            expected_after_insert_entry(doc_value, mapping_path, key, &expected_child)?
        };

        // Does the mapping already carry this key, and can the path
        // syntax address it? A key holding `.` or `[` — `app.io/name`,
        // ubiquitous in Kubernetes labels — composes into a path that
        // means something else entirely, so it is only safe to *add*
        // one (which needs no path), never to resolve one.
        let addressable = !key.contains('.') && !key.contains('[');
        let in_mapping = {
            let cache = self.cache.borrow();
            let (doc_value, _) = cache.as_ref().expect("validate populated the cache");
            let target = if mapping_path.is_empty() {
                Some(doc_value)
            } else {
                path_value(doc_value, mapping_path)
            };
            matches!(target, Some(Value::Mapping(m)) if m.get(key).is_some())
        };
        if in_mapping && !addressable {
            return Err(Error::Parse(format!(
                "insert_entry_value: `{mapping_path}` already has a key `{key}`, and a key \
                 containing `.` or `[` cannot be addressed by the path syntax to replace its \
                 value — `remove` the entry and insert it afresh, or splice it with `set`"
            )));
        }
        // A key present in the typed view but with no *value* span used to be
        // read as inherited through a `<<` merge — nothing to replace, so an
        // explicit entry overrides it. Since #165 that no longer identifies a
        // merge on its own: an implicit null (`a:`) has no value span either,
        // and appending there produced a second `a` key at `Ok`. The key token
        // separates them. A merged-in key has none, because it is not in this
        // mapping's source at all; an implicit null has one, so the entry is
        // already here and `set` writes into it.
        let child_path = if mapping_path.is_empty() {
            key.to_owned()
        } else {
            format!("{mapping_path}.{key}")
        };
        let existing = if in_mapping && addressable {
            self.span_at(&child_path).or_else(|| {
                self.key_span(&child_path)
                    .and_then(|_| self.write_span(&child_path).ok())
            })
        } else {
            None
        };
        let is_collection = matches!(expected_child, Value::Sequence(_) | Value::Mapping(_));
        if existing.is_some() && is_collection {
            return Err(Error::Parse(format!(
                "insert_entry_value: `{key}` already exists in `{mapping_path}` and its value \
                 is being replaced with a collection — growing a scalar entry into a nested \
                 block is not an in-place edit; `remove` the entry first, or splice the \
                 layout you want with `set`"
            )));
        }

        // A new key into a **flow** mapping — `{a: 1}`, `{}`, or the
        // whole document being one — splices `, key: value` before the
        // closing brace instead of appending a line (#338). Upserts of
        // an existing key fall through: `set` already writes into flow
        // sites.
        if existing.is_none() {
            if let Some((fs, fe)) = self.flow_collection_span(mapping_path, b'{') {
                self.refuse_multiline_flow("insert_entry_value", mapping_path, fs, fe)?;
                self.refuse_inside_aliased_anchor("insert_entry_value", mapping_path, fs)?;
                let ctx = self.emit_ctx_at(0, mapping_path);
                let key_spelling = emit_key(key, &ctx);
                let rendered = Self::emit_flow_member(&expected_child)?;
                let body_is_empty = self.source[fs + 1..fe - 1].trim().is_empty();
                let member = if body_is_empty {
                    format!("{key_spelling}: {rendered}")
                } else {
                    format!(", {key_spelling}: {rendered}")
                };
                let snapshot = self.clone();
                return self.guarded_item_splice(
                    |doc| doc.replace_span(fe - 1, fe - 1, &member),
                    &expected,
                    &snapshot,
                    &format!("insert_entry_value: inserting `{key}` into `{mapping_path}`"),
                );
            }
        }
        // The column the emission indents against, and the byte
        // position the edit touches: an existing key keeps its own
        // column and is rewritten at its value span, a new one takes
        // the last addressable sibling's column and is spliced at the
        // end of that sibling's line.
        let (column, anchor_pos, probe) = match existing {
            Some((start, _)) => (
                column_of_key_at(&self.source, start).ok_or_else(|| {
                    Error::Parse(format!(
                        "insert_entry_value: could not locate the column of the existing key \
                         `{key}` in `{mapping_path}`"
                    ))
                })?,
                start,
                start,
            ),
            None => self.mapping_insert_anchor(mapping_path)?,
        };
        self.refuse_inside_aliased_anchor("insert_entry_value", mapping_path, probe)?;
        // Learn the spelling from the mapping being edited, not the whole
        // document (#290).
        let ctx = self.emit_ctx_at(column, mapping_path);
        let fragment = value.emit(&ctx)?;
        let key_spelling = emit_key(key, &ctx);
        let indent = " ".repeat(column);

        let snapshot = self.clone();
        let spliced = if existing.is_some() {
            // Replace in place. The fragment's continuation lines (a
            // block scalar's body) shift to the existing key's column,
            // landing at `key_col + 2` — the depth `set_value` writes.
            let inline = indent_continuation_lines(&fragment, column, document_break(&self.source));
            self.set(&child_path, &inline)
        } else if is_collection {
            // `key:` then the emission as its children, one indent
            // step in from the key.
            let inner = " ".repeat(column + self.indent_unit());
            let lead = leading_break_for_splice(&self.source, anchor_pos);
            let nl = document_break(&self.source);
            let mut line = format!("{lead}{indent}{key_spelling}:{nl}");
            for body_line in fragment.split('\n') {
                if body_line.is_empty() {
                    line.push_str(nl);
                } else {
                    line.push_str(&inner);
                    line.push_str(body_line);
                    line.push_str(nl);
                }
            }
            self.replace_span(anchor_pos, anchor_pos, &line)
        } else {
            let nl = document_break(&self.source);
            let inline = indent_continuation_lines(&fragment, column, nl);
            let lead = leading_break_for_splice(&self.source, anchor_pos);
            let line = format!("{lead}{indent}{key_spelling}: {inline}{nl}");
            self.replace_span(anchor_pos, anchor_pos, &line)
        };
        if let Err(e) = spliced {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "insert_entry_value: inserting `{key}` into `{mapping_path}` could not be \
                 spliced ({e}); the document was left unchanged"
            )));
        }
        if let Err(e) = self.validate() {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "insert_entry_value: inserting `{key}` into `{mapping_path}` left the document \
                 unable to re-parse ({e}); the document was left unchanged"
            )));
        }
        if *self.as_value() != expected {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "insert_entry_value: inserting `{key}` into `{mapping_path}` failed the \
                 integrity check — the spliced entry did not load back as the value given \
                 (e.g. a key the mapping already inherits through a `{MERGE_KEY_SPELLING}` \
                 merge, or a layout the emitter could not reproduce at this indent); the \
                 document was left unchanged"
            )));
        }
        Ok(())
    }

    /// Append `value` to the block sequence at `path`, formatted so it
    /// re-parses to exactly that value.
    ///
    /// The typed counterpart of [`Document::push_back`], which splices
    /// its `&str` verbatim: `push_back("items", "- x")` grows a nested
    /// sequence, where `push_back_value("items", "- x")` appends the
    /// *string* `"- x"`. Guarded by the same re-parse plus typed-value
    /// oracle as [`Document::insert_entry_value`].
    ///
    /// A **flow** sequence takes `, value` before its closing bracket
    /// instead of a new `- ` line, and `[]` receives its first member
    /// (#338); only single-line flow collections accept inserts.
    ///
    /// # Errors
    ///
    /// - `path` does not resolve to a sequence, or an empty **block**
    ///   sequence leaves no item to anchor indentation on.
    /// - The flow sequence at `path` spans more than one line.
    /// - The value has no auto-formatted spelling (see [`Emit::emit`]).
    /// - The splice would not re-parse, or fails the integrity check;
    ///   the document is left unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("items:\n  - one\n").unwrap();
    /// doc.push_back_value("items", "two: 2").unwrap();
    /// assert_eq!(doc.to_string(), "items:\n  - one\n  - \"two: 2\"\n");
    /// ```
    pub fn push_back_value<E: Emit + ?Sized>(&mut self, path: &str, value: &E) -> Result<()> {
        if let Err(e) = self.validate() {
            return Err(Error::Parse(format!(
                "push_back_value: the document does not parse, so `{path}` cannot be resolved \
                 ({e}); the document was left unchanged"
            )));
        }
        let expected_item = value.expected_value()?;
        let (expected, len) = {
            let cache = self.cache.borrow();
            let (doc_value, _) = cache.as_ref().expect("validate populated the cache");
            let len = sequence_len_at(doc_value, &parse_query_path(path), path)?;
            (
                expected_after_insert_item(doc_value, path, len, &expected_item)?,
                len,
            )
        };
        // A **flow** sequence — `[a, b]` or `[]` — takes `, value`
        // before the closing bracket instead of a new `- ` line (#338).
        if let Some((fs, fe)) = self.flow_collection_span(path, b'[') {
            self.refuse_multiline_flow("push_back_value", path, fs, fe)?;
            self.refuse_inside_aliased_anchor("push_back_value", path, fs)?;
            let rendered = Self::emit_flow_member(&expected_item)?;
            let member = if len == 0 {
                rendered
            } else {
                format!(", {rendered}")
            };
            let snapshot = self.clone();
            return self.guarded_item_splice(
                |doc| doc.replace_span(fe - 1, fe - 1, &member),
                &expected,
                &snapshot,
                &format!("push_back_value: appending to `{path}`"),
            );
        }
        if len == 0 {
            return Err(Error::Parse(format!(
                "push_back_value: the sequence at `{path}` is empty, so it has no item to \
                 anchor indentation on — use `set` with a fragment instead"
            )));
        }
        let (column, anchor_pos) = self.sequence_item_anchor(path, len - 1)?;
        self.refuse_inside_aliased_anchor("push_back_value", path, anchor_pos)?;
        let fragment = self.emit_sequence_item(value, column, path)?;

        let snapshot = self.clone();
        self.guarded_item_splice(
            |doc| doc.push_back(path, &fragment),
            &expected,
            &snapshot,
            &format!("push_back_value: appending to `{path}`"),
        )
    }

    /// Insert `value` immediately after the sequence item at
    /// `item_path` (e.g. `"items[1]"`), formatted so it re-parses to
    /// exactly that value.
    ///
    /// The typed counterpart of [`Document::insert_after`], guarded by
    /// the same re-parse plus typed-value oracle as
    /// [`Document::insert_entry_value`].
    ///
    /// Inside a single-line **flow** sequence the new member follows
    /// the addressed item's own span: `[a, b]` after item 0 becomes
    /// `[a, v, b]` (#338).
    ///
    /// # Errors
    ///
    /// - `item_path` does not end in an index, or does not resolve to
    ///   a sequence item.
    /// - The flow sequence spans more than one line.
    /// - The value has no auto-formatted spelling (see [`Emit::emit`]).
    /// - The splice would not re-parse, or fails the integrity check;
    ///   the document is left unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("items:\n  - one\n  - three\n").unwrap();
    /// doc.insert_after_value("items[0]", "two").unwrap();
    /// assert_eq!(doc.to_string(), "items:\n  - one\n  - two\n  - three\n");
    /// ```
    pub fn insert_after_value<E: Emit + ?Sized>(
        &mut self,
        item_path: &str,
        value: &E,
    ) -> Result<()> {
        if let Err(e) = self.validate() {
            return Err(Error::Parse(format!(
                "insert_after_value: the document does not parse, so `{item_path}` cannot be \
                 resolved ({e}); the document was left unchanged"
            )));
        }
        let segments = parse_query_path(item_path);
        let Some(&QuerySegment::Index(index)) = segments.last() else {
            return Err(Error::Parse(
                "insert_after_value: path must end with a sequence index, e.g. `items[2]`".into(),
            ));
        };
        let seq_path = sequence_parent_path(item_path);
        let expected_item = value.expected_value()?;
        let expected = {
            let cache = self.cache.borrow();
            let (doc_value, _) = cache.as_ref().expect("validate populated the cache");
            expected_after_insert_item(doc_value, &seq_path, index + 1, &expected_item)?
        };
        // Inside a **flow** sequence the new member follows the
        // addressed item's own span: `[a, b]` after item 0 becomes
        // `[a, v, b]` (#338).
        if let Some((fs, fe)) = self.flow_collection_span(&seq_path, b'[') {
            self.refuse_multiline_flow("insert_after_value", item_path, fs, fe)?;
            self.refuse_inside_aliased_anchor("insert_after_value", item_path, fs)?;
            let (_, item_end) = self
                .span_at(item_path)
                .ok_or_else(|| Error::Parse(format!("path not found: {item_path}")))?;
            let rendered = Self::emit_flow_member(&expected_item)?;
            let member = format!(", {rendered}");
            let snapshot = self.clone();
            return self.guarded_item_splice(
                |doc| doc.replace_span(item_end, item_end, &member),
                &expected,
                &snapshot,
                &format!("insert_after_value: inserting after `{item_path}`"),
            );
        }
        let (column, anchor_pos) = self.sequence_item_anchor(&seq_path, index)?;
        self.refuse_inside_aliased_anchor("insert_after_value", item_path, anchor_pos)?;
        let fragment = self.emit_sequence_item(value, column, &seq_path)?;

        let snapshot = self.clone();
        self.guarded_item_splice(
            |doc| doc.insert_after(item_path, &fragment),
            &expected,
            &snapshot,
            &format!("insert_after_value: inserting after `{item_path}`"),
        )
    }

    /// The span of the collection at `path` when it is spelled in
    /// **flow** style opening with `open` (`b'{'` or `b'['`), resolved
    /// through the loader's span tree. `None` for block collections
    /// and unresolvable paths. The span starts exactly at the opening
    /// bracket and ends after the closing one (#338).
    fn flow_collection_span(&self, path: &str, open: u8) -> Option<(usize, usize)> {
        self.ensure_cache();
        let cache = self.cache.borrow();
        let (value, span_tree) = cache.as_ref().expect("caller validated the document");
        let segments = parse_query_path(path);
        let ((s, e), _) = resolve_span(value, span_tree, &segments)?;
        let (s, e) = trim_value_span(&self.source, s, e);
        let s = s + self.source[s..e].bytes().take_while(|&b| b == b' ').count();
        let close = if open == b'{' { b'}' } else { b']' };
        (self.source.as_bytes().get(s) == Some(&open)
            && e > s
            && self.source.as_bytes().get(e - 1) == Some(&close))
        .then_some((s, e))
    }

    /// The one refusal every single-line flow splice shares: a flow
    /// collection spread over several lines has separators this module
    /// cannot see from the span alone, so an insert there is refused
    /// rather than guessed at — the stance `remove` already takes.
    fn refuse_multiline_flow(&self, what: &str, path: &str, s: usize, e: usize) -> Result<()> {
        if self.source[s..e].contains('\n') {
            return Err(Error::Parse(format!(
                "{what}: the flow collection at `{path}` spans more than one line; only \
                 single-line flow collections accept inserts — reformat it, or splice with \
                 `set`"
            )));
        }
        Ok(())
    }

    /// Render `value` as a single-line flow member: collections in
    /// flow style, scalars with the serializer's quoting (a value the
    /// plain spelling would misread is quoted, line breaks force
    /// double quotes).
    fn emit_flow_member(value: &Value) -> Result<String> {
        // A bare string goes through the flow-context speller: the
        // serializer renders a root scalar for block context, where
        // `b, c` is plain-safe — inside `[…]` it is two members.
        if let Value::String(s) = value {
            return Ok(format_string_in_flow(s, SyntaxKind::PlainScalar));
        }
        let cfg = crate::SerializerConfig::new().flow_style(crate::FlowStyle::Flow);
        let rendered = crate::to_string_value_with_config(value, &cfg)?;
        let rendered = rendered.trim_end_matches('\n');
        if rendered.contains('\n') {
            return Err(Error::Parse(
                "cannot spell this value on a single line inside a flow collection".into(),
            ));
        }
        Ok(rendered.to_owned())
    }

    /// Refuse an edit at `pos` when it sits inside a value that is
    /// anchored and aliased elsewhere.
    ///
    /// Such an edit lands at every `*name` site at once, which the
    /// integrity oracle would then report as an unrelated mismatch.
    /// Naming the anchor up front turns a puzzling refusal into an
    /// actionable one — the courtesy `rename_key` already extends.
    fn refuse_inside_aliased_anchor(&self, what: &str, path: &str, pos: usize) -> Result<()> {
        if let Some((anchor, alias_count)) = self.aliased_anchor_covering(pos) {
            return Err(Error::Parse(format!(
                "{what}: `{path}` is inside the value anchored by `&{anchor}`, which has \
                 {alias_count} alias reference(s) — inserting here would insert at every \
                 `*{anchor}` site too; call `materialise_aliases_of(\"{anchor}\")` first to \
                 give each site its own copy, then insert"
            )));
        }
        Ok(())
    }

    /// Run `splice`, then hold it to the re-parse and typed-value
    /// guards shared by the sequence insertion mutators, rolling back
    /// to `snapshot` and reporting in `what`'s terms on any failure.
    fn guarded_item_splice<F>(
        &mut self,
        splice: F,
        expected: &Value,
        snapshot: &Self,
        what: &str,
    ) -> Result<()>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        if let Err(e) = splice(self) {
            *self = snapshot.clone();
            return Err(Error::Parse(format!(
                "{what} could not be spliced ({e}); the document was left unchanged"
            )));
        }
        if let Err(e) = self.validate() {
            *self = snapshot.clone();
            return Err(Error::Parse(format!(
                "{what} left the document unable to re-parse ({e}); the document was left \
                 unchanged"
            )));
        }
        if *self.as_value() != *expected {
            *self = snapshot.clone();
            return Err(Error::Parse(format!(
                "{what} failed the integrity check — the spliced item did not load back as the \
                 value given (e.g. a layout the emitter could not reproduce at this indent); \
                 the document was left unchanged"
            )));
        }
        Ok(())
    }

    /// Emit `value` for a `- ` sequence-item site whose indicator sits
    /// at `column`, carrying any continuation lines to the item's own
    /// content indent so the splice template's single line grows into
    /// a correctly-indented block.
    fn emit_sequence_item<E: Emit + ?Sized>(
        &self,
        value: &E,
        column: usize,
        site: &str,
    ) -> Result<String> {
        let ctx = self.emit_ctx_at(column, site);
        let fragment = value.emit(&ctx)?;
        // `push_back` / `insert_after` splice `{indent}- {fragment}`,
        // so the first line is already placed; every later line must
        // clear the `- ` indicator itself.
        Ok(indent_continuation_lines(
            &fragment,
            column + 2,
            document_break(&self.source),
        ))
    }

    /// The item the insertion mutators anchor against: the column of
    /// item `index`'s `-` indicator in the block sequence at `path`,
    /// and the byte where that item's value starts.
    fn sequence_item_anchor(&self, path: &str, index: usize) -> Result<(usize, usize)> {
        let item_path = item_child_path(path, index);
        let (start, _) = self.span_at(&item_path).ok_or_else(|| {
            Error::Parse(format!(
                "could not locate item {index} of `{path}` to anchor the new item's indentation"
            ))
        })?;
        let column = column_of_preceding_dash(&self.source, start).ok_or_else(|| {
            Error::Parse(format!(
                "only block sequences are supported (no `-` anchor before item {index} of \
                 `{path}`)"
            ))
        })?;
        Ok((column, start))
    }

    /// Where a new sibling entry goes in the block mapping at `path`:
    /// the column of the last addressable entry's key, the end of the
    /// line that entry closes on, and the byte its value starts at.
    /// The first two are the anchor `insert_entry` derives its indent
    /// and splice position from; the third is a probe position that is
    /// definitely *inside* the mapping, for the anchor/alias check.
    fn mapping_insert_anchor(&self, path: &str) -> Result<(usize, usize, usize)> {
        self.ensure_cache();
        let cache = self.cache.borrow();
        let (value, span_tree) = cache.as_ref().expect("caller validated the document");
        let segments = parse_query_path(path);
        let (target, target_tree) = if path.is_empty() {
            (value, span_tree)
        } else {
            resolve_tree(value, span_tree, &segments)
                .ok_or_else(|| Error::Parse(format!("path not found: {path}")))?
        };
        let Value::Mapping(m) = target else {
            return Err(Error::Parse(format!(
                "`{path}` is not a mapping, so it has no entry to anchor a new key on"
            )));
        };
        if m.is_empty() {
            return Err(Error::Parse(format!(
                "`{path}` is an empty mapping, so it has no entry to anchor indentation \
                 on — use `set` with a fragment instead"
            )));
        }
        let SpanTree::Mapping { entries, .. } = target_tree else {
            return Err(Error::Parse(format!(
                "`{path}` is not a mapping in the source, so it has no entry to anchor a \
                 new key on"
            )));
        };
        // Search from the back for an entry with bytes of its own, over
        // the *span tree* rather than by composing each key back into a
        // path string and re-parsing it. A key containing `.` or `[` —
        // the `app.kubernetes.io/name` convention — cannot survive that
        // round trip, so a mapping of such keys used to look as if none
        // of its entries had source bytes at all.
        //
        // Entries the typed view gained through a `<<` merge are absent
        // here by construction: this tree is built from the source.
        let anchor = entries
            .iter()
            .rev()
            .find_map(|((key_start, key_end), child_tree)| {
                let (start, end) = span_tree_bounds(child_tree);
                if start == end {
                    // An implicit null (`b:` with no value) owns no value
                    // bytes, but its *key* is a real line at the right
                    // column — and it is the line a new sibling belongs
                    // after. Taking the column from the key directly:
                    // `column_of_key_at` infers a key's column from a
                    // *value* offset, so it would walk past this one.
                    let line_start = start_of_line(&self.source, *key_start);
                    let bom = if line_start == 0 {
                        strip_bom(self.source.as_bytes())
                    } else {
                        0
                    };
                    let column = key_start.saturating_sub(line_start + bom);
                    return Some((column, *key_end, *key_start));
                }
                let (start, end) = trim_value_span(&self.source, start, end);
                let column = column_of_key_at(&self.source, start)?;
                Some((column, end, start))
            });
        let (column, end, start) = anchor.ok_or_else(|| {
            Error::Parse(format!(
                "no entry of the mapping at `{path}` has source bytes of its own to anchor \
                 indentation on — every entry is inherited through a \
                 `{MERGE_KEY_SPELLING}` merge — use `set` with a fragment instead"
            ))
        })?;
        Ok((column, end_of_line(&self.source, end), start))
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
        (QuerySegment::Key(k), SyntaxKind::BlockMapping | SyntaxKind::FlowMapping) => {
            walk_mapping(node, k, tail, base, source)
        }
        (QuerySegment::Index(i), SyntaxKind::BlockSequence | SyntaxKind::FlowSequence) => {
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
/// Whether a resolved value node is a block (indentation-structured)
/// collection, whose span begins on its own source line.
fn is_block_collection(k: SyntaxKind) -> bool {
    matches!(k, SyntaxKind::BlockMapping | SyntaxKind::BlockSequence)
}

/// Back `start` up over the inline whitespace that indents a value's first
/// line, but only when that value begins its own line (the whitespace run is
/// preceded by a line break or the start of input). A value that shares its
/// line with a `-` / `:` / `{` (e.g. the inner sequence of `- - a`) is left
/// untouched. This makes a block collection's slice uniformly indented — its
/// first line keeps the indentation the following lines already carry — so it
/// re-parses to the selected value instead of silently re-nesting.
fn extend_to_line_start(source: &str, start: usize) -> usize {
    let b = source.as_bytes();
    let mut i = start;
    while i > 0 && matches!(b[i - 1], b' ' | b'\t') {
        i -= 1;
    }
    if i == 0 || matches!(b[i - 1], b'\n' | b'\r') {
        i
    } else {
        start
    }
}

fn resolve_value_in_entry(
    entry: &GreenNode,
    base: usize,
    tail: &[QuerySegment],
    source: &str,
) -> Option<(usize, usize)> {
    let (value_kind, value_range, value_node) = entry_value(entry, base)?;
    if tail.is_empty() {
        // A block collection's node starts at its first key/item token,
        // leaving its first line's indentation just outside the span; widen
        // to the line start so the slice is uniformly indented.
        let start = if is_block_collection(value_kind) {
            extend_to_line_start(source, value_range.0)
        } else {
            value_range.0
        };
        return Some((start, value_range.1));
    }
    // Recursing further requires the value to be a composite.
    let node = value_node?;
    walk_path(node, tail, value_range.0, source)
}

fn resolve_value_in_item(
    item: &GreenNode,
    base: usize,
    tail: &[QuerySegment],
    source: &str,
) -> Option<(usize, usize)> {
    let (value_kind, value_range, value_node) = item_value(item, base)?;
    if tail.is_empty() {
        let start = if is_block_collection(value_kind) {
            extend_to_line_start(source, value_range.0)
        } else {
            value_range.0
        };
        return Some((start, value_range.1));
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
                } else if *kind == SyntaxKind::AliasMark {
                    // An alias reference (`*name`) is a single token with
                    // no value node of its own; its bytes are a dangling
                    // alias that does not re-parse standalone. Bail so
                    // span_at falls back to the typed cache, whose SpanTree
                    // resolves the alias through to its anchor definition's
                    // self-contained value span.
                    return None;
                } else if is_value_property_kind(*kind) {
                    // `!Tag` / `&anchor` prefix — remember the earliest
                    // start and keep scanning for the scalar that follows.
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
                } else if *kind == SyntaxKind::AliasMark {
                    // Alias reference as a sequence item: bail to the typed
                    // cache, which resolves it to the anchor's value span.
                    return None;
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
    // Alias marks are handled separately (they bail the green walk to the
    // typed cache); only anchor/tag definition prefixes stretch the value
    // span to cover the property plus the scalar / node that follows.
    matches!(k, SyntaxKind::AnchorMark | SyntaxKind::TagMark)
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

/// Trim trailing separator whitespace from a *value* span, except for
/// keep-chomped (`|+` / `>+`) block scalars, whose trailing line breaks are
/// content rather than separation. Trimming those would yield a slice that
/// re-parses to a shorter, different value (`"kept\n"` instead of the true
/// `"kept\n\n\n"`).
fn trim_value_span(source: &str, start: usize, end: usize) -> (usize, usize) {
    if is_keep_chomped_block_scalar(source, start, end) {
        (start, end)
    } else {
        trim_trailing_blank(source, start, end)
    }
}

/// The empty span a value would be written into at an implicit null.
///
/// An absent block-mapping value or empty sequence item has no bytes of its
/// own, so there is nothing to *replace* — but there is somewhere to insert,
/// and the loader already records where: the zero-width leaf sits on the `:`
/// or `-` indicator the value would have followed. The insertion point is the
/// byte after it, which is before any trailing comment on the line, so a value
/// written there lands ahead of the comment rather than behind it.
///
/// `None` when `pos` is not one of those two indicators. A zero-width span can
/// in principle arrive from elsewhere, and inserting at a position this
/// function has not identified would splice into the middle of something.
///
/// A caller writing here must supply the separator itself: the span abuts the
/// indicator, and unlike a replacement span there is no `: ` already in the
/// source. [`fill_in`] is that one byte, so the two writers cannot disagree
/// about it.
fn implicit_null_insertion_point(source: &str, pos: usize) -> Option<(usize, usize)> {
    match source.as_bytes().get(pos) {
        Some(b':' | b'-') => Some((pos + 1, pos + 1)),
        _ => None,
    }
}

/// A value written at an [`implicit_null_insertion_point`], separated from the
/// indicator it follows.
fn fill_in(fragment: &str) -> String {
    format!(" {fragment}")
}

/// The `#` comment that follows `pos` on the same line, if only inline
/// whitespace separates them: `(comment_start, line_end)`, where
/// `line_end` excludes the line break. `pos` is the end of a value span
/// (or an implicit null's insertion point), so a `#` reached this way
/// can only be a comment -- the scanner ended the value before it.
fn trailing_comment_span(source: &str, pos: usize) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut i = pos;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'#' {
        return None;
    }
    let mut line_end = i;
    while line_end < bytes.len() && !matches!(bytes[line_end], b'\n' | b'\r') {
        line_end += 1;
    }
    Some((i, line_end))
}

/// Put `comment` after the header of the block literal `fragment`:
/// `|-\n  a` with `# c` becomes `|- # c\n  a`. A fragment without a
/// line break is not a block literal and is returned unchanged.
fn hoist_comment_onto_header(fragment: &str, comment: &str) -> String {
    match fragment.split_once('\n') {
        Some((header, body)) => format!("{header} {comment}\n{body}"),
        None => fragment.to_string(),
    }
}

/// Whether `[start, end)` denotes a keep-chomped block scalar: it begins with
/// a `|` / `>` block indicator carrying a `+` chomping indicator on the header
/// line (`|+`, `>+`, `|+2`, `|2+`). A value span's start is the block
/// indicator itself (the scanner marks it there), and no plain/quoted scalar
/// or collection value begins with a bare `|` / `>`, so this cannot misfire on
/// other node kinds.
fn is_keep_chomped_block_scalar(source: &str, start: usize, end: usize) -> bool {
    let bytes = source.as_bytes();
    // The value span's start may have been widened leftward over an anchor
    // (`&name`) / tag (`!Tag`, `!!str`) property prefix (see `entry_value`), so
    // the block indicator is not necessarily at `start`. Skip those property
    // tokens before inspecting for `|` / `>`, otherwise an anchored/tagged
    // keep-chomped scalar (`key: &anc |+`) is misclassified and its kept
    // trailing blank lines are trimmed.
    let start = skip_value_property_prefix(bytes, start, end);
    if start >= end || (bytes[start] != b'|' && bytes[start] != b'>') {
        return false;
    }
    // A `+` anywhere on the header line (before the first line break) is the
    // keep-chomping indicator.
    for &b in &bytes[start + 1..end] {
        match b {
            b'\n' | b'\r' => return false,
            b'+' => return true,
            _ => {}
        }
    }
    false
}

/// Advance past leading anchor (`&name`) / tag (`!Tag`, `!!str`) property
/// tokens and the whitespace between them, returning the index of the value
/// content proper. Value spans are widened leftward over these properties, so
/// callers inspecting the value's first byte must skip them first.
fn skip_value_property_prefix(bytes: &[u8], mut start: usize, end: usize) -> usize {
    loop {
        while start < end && matches!(bytes[start], b' ' | b'\t') {
            start += 1;
        }
        if start < end && matches!(bytes[start], b'&' | b'!') {
            start += 1;
            while start < end && !matches!(bytes[start], b' ' | b'\t' | b'\n' | b'\r') {
                start += 1;
            }
        } else {
            return start;
        }
    }
}

/// The `(start, end)` bounds of a span tree, transparently unwrapping alias
/// indirection.
fn span_tree_bounds(t: &SpanTree) -> (usize, usize) {
    match t {
        SpanTree::Leaf(s, e) => (*s, *e),
        SpanTree::Sequence { start, end, .. } | SpanTree::Mapping { start, end, .. } => {
            (*start, *end)
        }
        SpanTree::Alias(inner) => span_tree_bounds(inner),
    }
}

/// Resolve `segments` to the `(value, span tree)` pair they address.
///
/// The span-tree twin of [`resolve_span`], for callers that need the
/// addressed node's *structure* rather than its span — the sole one today
/// is `mapping_insert_anchor`, which reads the target mapping's entries
/// directly instead of rebuilding a path string per key.
fn resolve_tree<'a>(
    value: &'a Value,
    span_tree: &'a SpanTree,
    segments: &[QuerySegment],
) -> Option<(&'a Value, &'a SpanTree)> {
    if let SpanTree::Alias(inner) = span_tree {
        return resolve_tree(value, inner, segments);
    }
    let Some((head, tail)) = segments.split_first() else {
        return Some((value, span_tree));
    };
    match (head, value, span_tree) {
        (QuerySegment::Key(k), Value::Mapping(m), SpanTree::Mapping { entries, .. }) => {
            // `m` (an IndexMap) preserves insertion order, matching the
            // parallel order in `entries` (see `span_context::walk`).
            for ((mk, mv), (_, child_tree)) in m.iter().zip(entries.iter()) {
                if mk == k {
                    return resolve_tree(mv, child_tree, tail);
                }
            }
            None
        }
        (QuerySegment::Index(i), Value::Sequence(seq), SpanTree::Sequence { items, .. }) => {
            resolve_tree(seq.get(*i)?, items.get(*i)?, tail)
        }
        _ => None,
    }
}

/// Resolve `segments` to a byte span in the typed cache. The returned `bool`
/// is `true` when resolution passed *through* an alias reference (the span
/// then belongs to the anchor, not the addressed key) — correct to return for
/// a read, but a write must refuse it.
fn resolve_span(
    value: &Value,
    span_tree: &SpanTree,
    segments: &[QuerySegment],
) -> Option<((usize, usize), bool)> {
    // An alias site substitutes the anchor's (value, tree). Resolve against the
    // anchor but flag that the path went through an alias — at any depth, so
    // `ref` and `ref.nested` and `[*a]` are all caught.
    if let SpanTree::Alias(inner) = span_tree {
        return resolve_span(value, inner, segments).map(|(span, _)| (span, true));
    }
    if segments.is_empty() {
        return match span_tree {
            // A zero-width leaf marks an implicit null (an absent
            // block-mapping value or empty sequence item): the node has no
            // source bytes of its own. Its *position* is still the `:` / `-`
            // indicator it followed, which is where a value would be written,
            // so the resolver reports it and each caller decides. `span_at`
            // discards it (#165: an implicit null has no span to read);
            // `write_span` turns it into an insertion point.
            SpanTree::Leaf(s, e) => Some(((*s, *e), false)),
            SpanTree::Sequence { start, end, .. } | SpanTree::Mapping { start, end, .. } => {
                Some(((*start, *end), false))
            }
            SpanTree::Alias(_) => None, // unwrapped above
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
/// How `remove` should splice a given entry out of the source.
///
/// Three shapes, because "delete the entry" means three different byte
/// edits depending on what the entry shares its line with:
///
/// - [`Removal::Line`] — the entry owns whole lines; delete them.
/// - [`Removal::FlowMember`] — the entry lives inside `{…}` / `[…]`
///   alongside its siblings, so only its own span plus one separator
///   may go.
/// - [`Removal::SoleEntry`] — the entry is the last one in its
///   collection. Deleting its bytes would leave `a:` behind, which
///   re-parses as *null*, not as an empty collection — a type change,
///   not a removal. The collection is replaced with an explicit `{}`
///   or `[]` instead.
#[derive(Debug, Clone, Copy)]
enum Removal {
    Line {
        start: usize,
        end: usize,
        multiline: bool,
    },
    FlowMember {
        start: usize,
        end: usize,
    },
    /// The entry's first line is shared with a sequence `-` indicator
    /// (`- name: x`, `- - a`): only the entry's own bytes go, through
    /// its owned range plus the following sibling's indentation, so
    /// the sibling moves up onto the indicator's line. Always guarded
    /// by the typed oracle.
    SpanWithinLine {
        start: usize,
        end: usize,
    },
    SoleEntry {
        start: usize,
        end: usize,
        empty: &'static str,
        /// Columns of indentation to re-emit before `empty`.
        ///
        /// The splice can now start *above* the entry, at the head of its
        /// comment run, so the entry's own leading whitespace is inside
        /// the replaced range and has to be written back — otherwise
        /// `a:\n  # doc\n  x: 1` collapses to `a:\n{}`, which is not
        /// `a`'s value at all.
        indent: usize,
    },
}

/// Widen a flow member's span to take exactly one separator with it,
/// and its whole line when the splice would leave that line blank.
///
/// `{x: 1, y: 2}` minus `x` must become `{y: 2}`, not `{, y: 2}`. The
/// comma *after* the member is preferred; the last member takes the one
/// *before* it instead. A lone member has no separator to take — that is
/// [`Removal::SoleEntry`]'s job, not this one.
///
/// Only spaces and tabs are crossed while looking. A separator parked on
/// another line (a multi-line flow collection) is deliberately not
/// matched: the resulting splice would still be guarded by the typed
/// oracle, so the outcome is a refusal rather than a mangled document.
///
/// The separator walk alone is not the whole answer for a **wrapped**
/// collection, where a member can be the only thing on its line — see
/// [`absorb_emptied_line`].
fn flow_member_range(source: &str, start: usize, end: usize) -> (usize, usize) {
    let bytes = source.as_bytes();

    // Forward: `x: 1, y: 2` → take `, ` after the member.
    let mut i = end;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b',' {
        i += 1;
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        return absorb_emptied_line(source, start, i);
    }

    // Backward: the member is last, so take the `, ` before it.
    let mut j = start;
    while j > 0 && matches!(bytes[j - 1], b' ' | b'\t') {
        j -= 1;
    }
    if j > 0 && bytes[j - 1] == b',' {
        return absorb_emptied_line(source, j - 1, end);
    }

    absorb_emptied_line(source, start, end)
}

/// Give a flow member its whole line when removing it would leave that
/// line holding nothing but indentation.
///
/// A flow collection wrapped over several lines puts one member per
/// line, so splicing out just the member's bytes leaves the line's
/// indentation behind as a whitespace-only line:
///
/// ```text
/// ports: [        remove("ports[0]")     ports: [
///   80,                   ->              ␣␣
///   443,                                  443,
/// ]                                     ]
/// ```
///
/// The result still loads — this is not corruption — but it writes
/// trailing whitespace onto a line that had none, which is what
/// `git diff --check`, `yamllint` and most pre-commit hooks exist to
/// catch. A lossless CST that edits one member should not hand its
/// caller a diff their own lint rejects.
///
/// The condition is deliberately "the member is alone on its line",
/// not "the collection is wrapped". Anything else surviving on the line
/// keeps the line:
///
/// - `ports: [80,` — the opening indicator is on it, so it stays.
/// - `  443]` — the closing indicator is on it, so it stays.
/// - `  80, # http` — the comment is on it, so it stays, and what a
///   comment left behind by a removal *means* stays the caller's
///   question rather than being decided here by a whitespace rule.
///
/// The line terminator goes with the line, `\r\n` included; taking the
/// `\n` and leaving the `\r` would plant a lone CR in a CRLF document.
fn absorb_emptied_line(source: &str, start: usize, end: usize) -> (usize, usize) {
    let bytes = source.as_bytes();

    // Everything before the member on its line must be indentation.
    let line_start = source[..start].rfind('\n').map_or(0, |nl| nl + 1);
    if !source[line_start..start]
        .bytes()
        .all(|b| matches!(b, b' ' | b'\t'))
    {
        return (start, end);
    }

    // And everything after it must be indentation up to the terminator.
    // A `]`, a `#`, or a sibling member here means the line still has
    // content, and the member does not own it.
    let mut i = end;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    match bytes.get(i) {
        Some(b'\n') => (line_start, i + 1),
        Some(b'\r') if bytes.get(i + 1) == Some(&b'\n') => (line_start, i + 2),
        // No terminator to take (end of source): leave the range alone
        // rather than swallow an indent with nothing to fold it into.
        _ => (start, end),
    }
}

/// Is the collection at `start` written in flow style (`{…}` / `[…]`)?
fn is_flow_collection(source: &str, start: usize) -> bool {
    source
        .as_bytes()
        .get(start)
        .is_some_and(|b| matches!(b, b'{' | b'['))
}

/// The collection's span with any trailing whitespace given back.
///
/// A collection's span can run to the end of the line holding its last
/// value, newline included. Overwriting that range with `{}` would take
/// the document's final newline with it — `only: 1\n` becoming `{}`
/// rather than `{}\n`. Harmless to a parser, but this is a lossless CST:
/// a vanished trailing newline is a whole-file diff and trips
/// `.gitattributes` and CI end-of-file checks.
fn collection_span_trimmed(source: &str, start: usize, end: usize) -> (usize, usize) {
    let trimmed = source[..end].trim_end().len();
    (start, trimmed.max(start))
}

/// The span to replace when a collection's **last** entry is removed,
/// plus the indentation the replacement must re-emit.
///
/// Two things the collection's own span does not give us:
///
/// * **The head-comment run.** A collection starts at its first entry's
///   *content*, which is below any comment describing that entry. The
///   `Removal::Line` path owns that run via `absorb_head_comments`, so
///   without this the same comment on the same entry was taken when the
///   entry had a sibling and stranded when it did not — left describing
///   an empty collection (#280). The typed oracle cannot catch it: a
///   comment is not in the typed value.
/// * **The indentation.** Starting the splice at the head of the comment
///   run puts the entry's own leading whitespace inside the replaced
///   range, so it has to be written back or `a:` loses its value.
///
/// A comment detached by a blank line, or at a different column, is not
/// the entry's — `absorb_head_comments` already stops at both.
///
/// `parent_key` is the byte offset of the key this collection is the value
/// of, when it has one. It decides the indentation: a block collection may
/// share its key's column, but the `{}` / `[]` that replaces it may not.
fn sole_entry_range(
    source: &str,
    coll_start: usize,
    coll_end: usize,
    parent_key: Option<usize>,
) -> (usize, usize, usize) {
    let (start, end) = collection_span_trimmed(source, coll_start, coll_end);

    // Flow collections sit inline — `a: {x: 1}` starts at the `{`, part
    // way along a line whose earlier bytes belong to the *key*. There is
    // no head-comment run above such an entry to own, and the bytes
    // before `{` are not indentation to re-emit: treating them as such
    // would rewrite `a: {x: 1}` as `   {}` and lose the key.
    if is_flow_collection(source, coll_start) {
        return (start, end, 0);
    }

    let line_start = start_of_line(source, coll_start);
    let own_indent = coll_start - line_start;

    // A block sequence is allowed to sit at its key's own column — `on:` /
    // `- push`, the GitHub Actions and Ansible idiom. What replaces it is
    // not: `{}` / `[]` is a block *mapping value*, and one that shares its
    // key's column does not re-parse as that key's value. So the entry's
    // own indent is the right answer only when it already clears the key.
    //
    // The comment run is still absorbed at the entry's own column, which is
    // where those comment lines actually sit.
    let indent = match parent_key.map(|key| column_of(source, key)) {
        Some(key_column) if own_indent <= key_column => key_column + 2,
        _ => own_indent,
    };
    (
        absorb_head_comments(source, line_start, own_indent),
        end,
        indent,
    )
}

/// The column a byte offset sits in, with a leading BOM discounted.
///
/// A BOM is zero-width, so `on:` after one is in column 0, not column 3 —
/// counting its bytes toward the column is the mistake #123 fixed in the
/// scanner, and the same one is available here. Everything else that can
/// precede a key on its line (indent spaces, `- `, `? `) is one byte per
/// column, so the subtraction is exact.
fn column_of(source: &str, offset: usize) -> usize {
    let line_start = start_of_line(source, offset);
    let bom = if line_start == 0 {
        strip_bom(source.as_bytes())
    } else {
        0
    };
    offset.saturating_sub(line_start + bom)
}

/// The refusal for a `remove` path segment that names a key the mapping
/// received through a `<<` merge key. Such a key is in the loaded
/// `Value` but has no entry of its own in this mapping's source, so
/// indexing the span-entry list by its position would run past the end
/// (issue #334).
fn merge_provided_key(k: &str) -> Error {
    Error::Parse(format!(
        "remove: key {k:?} was produced by a `<<` merge key and has no entry \
         of its own to remove in this mapping — remove it at the anchor's \
         definition, or override it here explicitly"
    ))
}

fn entry_line_span(
    value: &Value,
    span_tree: &SpanTree,
    source: &str,
    segments: &[QuerySegment],
    parent_key: Option<usize>,
) -> Result<Removal> {
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
        // The key descended through is the parent key of everything below
        // it — which is what the sole-entry arm needs to know how deep an
        // empty collection has to sit. A sequence item is not a key, so
        // descending through one clears it.
        let (child_value, child_tree, child_key) = match (head, value, span_tree) {
            (QuerySegment::Key(k), Value::Mapping(m), SpanTree::Mapping { entries, .. }) => {
                let pos = m
                    .iter()
                    .position(|(mk, _)| mk == k)
                    .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?;
                // Keys past the span-entry list were provided by a `<<`
                // merge key: they exist in the loaded mapping but own no
                // bytes here, so there is nothing to descend into.
                let ((key_start, _), child_tree) =
                    entries.get(pos).ok_or_else(|| merge_provided_key(k))?;
                (
                    m.iter().nth(pos).map(|(_, v)| v).expect("pos in range"),
                    child_tree,
                    Some(*key_start),
                )
            }
            (QuerySegment::Index(i), Value::Sequence(seq), SpanTree::Sequence { items, .. }) => (
                seq.get(*i).ok_or_else(|| {
                    Error::Parse(format!("path not found: index {i} out of bounds"))
                })?,
                items.get(*i).ok_or_else(|| {
                    Error::Parse(format!("path not found: index {i} out of bounds"))
                })?,
                None,
            ),
            _ => return Err(Error::Parse("path not found".into())),
        };
        return entry_line_span(child_value, child_tree, source, tail, child_key);
    }

    // Final segment — locate this entry's key / dash and value.
    match (head, value, span_tree) {
        (
            QuerySegment::Key(k),
            Value::Mapping(m),
            SpanTree::Mapping {
                start: coll_start,
                end: coll_end,
                entries,
            },
        ) => {
            let pos = m
                .iter()
                .position(|(mk, _)| mk == k)
                .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?;
            // The loaded mapping lists merge-provided keys after the
            // explicit ones, and the span tree holds only the explicit
            // entries, so a position past the entry list is a key that
            // `<<` supplied. It owns no bytes in this mapping: refuse
            // before the sole-entry arm below, which would otherwise
            // read a merge-only mapping as "one entry" and replace the
            // `<<` line itself with `{}`.
            let ((key_start, _key_end), child_tree) =
                entries.get(pos).ok_or_else(|| merge_provided_key(k))?;
            // The sole key of a mapping that shares its line with a
            // sequence `-` indicator (`- name: x`): the mapping's bytes
            // are exactly the entry's own, and the item must stay, so
            // the entry is replaced with `{}` in place instead of
            // splicing whole lines (#336).
            if m.len() <= 1 && !is_flow_collection(source, *coll_start) {
                if let Some(((key_start, _), entry_tree)) = entries.first() {
                    if locate_preceding_dash(source, *key_start).is_some() {
                        let (vs, rve) = span_tree_bounds(entry_tree);
                        let end = owned_value_end(source, vs, rve);
                        return Ok(Removal::SoleEntry {
                            start: *key_start,
                            end,
                            empty: "{}",
                            indent: 0,
                        });
                    }
                }
            }
            // Last entry: the collection itself becomes `{}`. Deleting the
            // bytes would leave a dangling `a:` that re-parses as null.
            if m.len() <= 1 {
                let (start, end, indent) =
                    sole_entry_range(source, *coll_start, *coll_end, parent_key);
                return Ok(Removal::SoleEntry {
                    start,
                    end,
                    empty: "{}",
                    indent,
                });
            }
            // An alias value resolves *through* to its anchor, so the span
            // here belongs to the anchor's bytes on some other line — not
            // to this entry. Splicing it would edit a different key, which
            // is why `SpanTree::Alias`'s own doc says a write must refuse.
            //
            // Left unchecked the arithmetic silently degenerated instead:
            // for `a: &x 1` / `b: *x`, the value span (6,7) sits *before*
            // the key at 8, `owned_entry_range` produced an empty range,
            // and the splice removed nothing while returning `Ok`. Found
            // by the `fuzz_editors` shrink invariant.
            if matches!(child_tree, SpanTree::Alias(_)) {
                return Err(Error::Parse(format!(
                    "remove: the value of `{k}` is an alias (`*name`); its source bytes \
                     belong to the anchor, not to this entry, so removing it here would \
                     edit a different key — remove the anchor's entry, or use \
                     `replace_span` deliberately"
                )));
            }
            let (value_start, raw_value_end) = span_tree_bounds(child_tree);
            if is_flow_collection(source, *coll_start) {
                let member_end = owned_value_end(source, value_start, raw_value_end);
                let (s, e) = flow_member_range(source, *key_start, member_end);
                return Ok(Removal::FlowMember { start: s, end: e });
            }
            // A mapping that is a sequence item carries its first key on
            // the `- ` indicator's line. Deleting that entry's whole line
            // would take the indicator -- and so the item -- with it (the
            // splice failed re-parse or the integrity check before, #336).
            // Instead only the entry's own bytes go, through its owned
            // range plus the following sibling's indentation, so the next
            // key moves up beside the indicator.
            if locate_preceding_dash(source, *key_start).is_some() {
                let (_, end, _) = owned_entry_range(source, *key_start, value_start, raw_value_end);
                return Ok(Removal::SpanWithinLine {
                    start: *key_start,
                    end: skip_line_indent(source, end),
                });
            }
            let (start, end, multiline) =
                owned_entry_range(source, *key_start, value_start, raw_value_end);
            Ok(Removal::Line {
                start,
                end,
                multiline,
            })
        }
        (
            QuerySegment::Index(i),
            Value::Sequence(seq),
            SpanTree::Sequence {
                start: coll_start,
                end: coll_end,
                items,
            },
        ) => {
            if *i >= seq.len() {
                return Err(Error::Parse(format!(
                    "path not found: index {i} out of bounds"
                )));
            }
            if seq.len() <= 1 {
                let (start, end, indent) =
                    sole_entry_range(source, *coll_start, *coll_end, parent_key);
                return Ok(Removal::SoleEntry {
                    start,
                    end,
                    empty: "[]",
                    indent,
                });
            }
            let item_tree = items
                .get(*i)
                .ok_or_else(|| Error::Parse(format!("path not found: index {i} out of bounds")))?;
            if matches!(item_tree, SpanTree::Alias(_)) {
                return Err(Error::Parse(format!(
                    "remove: item {i} is an alias (`*name`); its source bytes belong to the \
                     anchor, not to this item, so removing it here would edit different \
                     content"
                )));
            }
            let (value_start, raw_value_end) = span_tree_bounds(item_tree);
            if is_flow_collection(source, *coll_start) {
                // No `-` indicator in flow style; the item's own span is
                // the member.
                let member_end = owned_value_end(source, value_start, raw_value_end);
                let (s, e) = flow_member_range(source, value_start, member_end);
                return Ok(Removal::FlowMember { start: s, end: e });
            }
            // The `-` indicator sits before the value on the same line,
            // separated by inline whitespace. Walk backward to find it.
            // An implicit-null item owns no bytes and its empty span sits
            // past the line break, so the walk may cross one (#336).
            let dash_pos = locate_preceding_dash(source, value_start)
                .or_else(|| {
                    (value_start == raw_value_end)
                        .then(|| locate_dash_at_or_across_break(source, value_start))
                        .flatten()
                })
                .ok_or_else(|| {
                    Error::Parse(
                        "remove: could not locate '-' indicator preceding sequence item".into(),
                    )
                })?;
            // A sequence nested in a sequence shares its first item's
            // line with the enclosing dash (`- - a`). As for a mapping's
            // first key above, only the item's own bytes go (#336).
            if locate_preceding_dash(source, dash_pos).is_some() {
                let (_, end, _) = owned_entry_range(source, dash_pos, value_start, raw_value_end);
                return Ok(Removal::SpanWithinLine {
                    start: dash_pos,
                    end: skip_line_indent(source, end),
                });
            }
            let (start, end, multiline) =
                owned_entry_range(source, dash_pos, value_start, raw_value_end);
            Ok(Removal::Line {
                start,
                end,
                multiline,
            })
        }
        _ => Err(Error::Parse("path not found".into())),
    }
}

/// The whole-line source range an entry owns, plus whether that range
/// spans more than one line (which selects `remove`'s guarded path).
///
/// `entry_start` points at the entry's key (mapping) or `-` indicator
/// (sequence); `value_start..raw_value_end` is its value's span as the
/// span tree reports it.
///
/// An entry owns more than the bytes of its key and value:
///
/// - the contiguous run of full-line comments directly above it, at its
///   own indentation — its *head comment*. Leaving those behind does not
///   merely litter: the comment silently becomes documentation for the
///   *next* entry. A blank line detaches the run, so a document header
///   set off by one is not swept up with the first entry.
/// - a keep-chomped (`|+` / `>+`) block scalar's kept trailing blank
///   lines, which are value content rather than separation. Leaving them
///   behind strands blank lines the removed entry brought with it.
///
/// It does **not** own comment lines that follow its last content line.
/// Those lie outside the value span — [`Document::span_at`] already
/// excludes them — and conventionally document whatever comes next, so
/// removing them would delete something the caller did not address. A
/// comment *interleaved* inside a multi-line value is inside the span and
/// goes with the entry.
fn owned_entry_range(
    source: &str,
    entry_start: usize,
    value_start: usize,
    raw_value_end: usize,
) -> (usize, usize, bool) {
    let bytes = source.as_bytes();
    let value_end = owned_value_end(source, value_start, raw_value_end);

    // Extend through the line break holding the value's last content byte
    // — unless `value_end` already sits on a line boundary, which happens
    // only for a keep-chomped scalar whose kept blank lines end there.
    // Extending then would swallow the following entry's first line.
    let end = if value_end > 0 && bytes[value_end - 1] == b'\n' {
        value_end
    } else {
        end_of_line(source, value_end)
    };

    let first_line_start = start_of_line(source, entry_start);
    let indent = entry_indent_column(source, entry_start);
    let start = absorb_head_comments(source, first_line_start, indent);

    // The single-line fast path in `remove` stays available only for a
    // range that really is one line: an absorbed head comment or a kept
    // blank line makes the splice multi-line and sends it through the
    // re-parse guard.
    let body = &source[start..end];
    let multiline = body.strip_suffix('\n').unwrap_or(body).contains('\n');
    (start, end, multiline)
}

/// Where an entry's value ends for the purposes of
/// [`owned_entry_range`]: the span tree's raw end walked back over
/// separator whitespace and over any comment-only lines beyond the
/// value's last content line.
///
/// A keep-chomped block scalar is returned untouched — its trailing line
/// breaks are content, and trimming them would strand them in the
/// document after the entry is removed.
fn owned_value_end(source: &str, value_start: usize, raw_value_end: usize) -> usize {
    if is_keep_chomped_block_scalar(source, value_start, raw_value_end) {
        return raw_value_end;
    }
    let bytes = source.as_bytes();
    let mut end = raw_value_end;
    loop {
        while end > value_start && matches!(bytes[end - 1], b' ' | b'\t' | b'\n' | b'\r') {
            end -= 1;
        }
        // `end` now sits just past a line's last content byte. If that
        // line holds nothing but a comment, it is trailing trivia rather
        // than value content; drop it and look at the line above. The
        // `line_start > value_start` guard keeps the walk from reaching
        // into the entry's own first line.
        let line_start = start_of_line(source, end);
        if line_start <= value_start || !source[line_start..end].trim_start().starts_with('#') {
            return end;
        }
        end = line_start;
    }
}

/// Split a whole-line range into its content and the line break that
/// ends it, if any.
///
/// Used by [`Document::swap_items`], which exchanges the two bodies but
/// leaves each terminator where it is. The final entry of a document may
/// carry none — `- a\n- b` — and moving the breaks with the bodies would
/// splice that into the single line `- b- a`.
fn split_line_terminator(text: &str) -> (&str, &str) {
    if let Some(body) = text.strip_suffix("\r\n") {
        (body, "\r\n")
    } else if let Some(body) = text.strip_suffix('\n') {
        (body, "\n")
    } else {
        (text, "")
    }
}

/// Walk `start` up over the contiguous run of full-line comments directly
/// above an entry, each beginning at column `indent`.
///
/// Stops at a blank line, a non-comment line, or a comment at a different
/// column — so a comment detached by a blank line stays put, and so does
/// one belonging to an enclosing or nested level.
fn absorb_head_comments(source: &str, mut start: usize, indent: usize) -> usize {
    while start > 0 {
        // `start` is always 0 or one past a `\n`, so the preceding line is
        // `[prev_line_start, start - 1)` with the break excluded.
        let prev_line_start = start_of_line(source, start - 1);
        let line = source[prev_line_start..start - 1].trim_end_matches('\r');
        let content = line.trim_start_matches([' ', '\t']);
        if !content.starts_with('#') || line.len() - content.len() != indent {
            break;
        }
        start = prev_line_start;
    }
    start
}

// ── Key-site resolution (used by `rename_key`) ──────────────────────

/// The YAML merge key. Spelled out here because `rename_key` refuses
/// it as a *new* key name: the loader matches the decoded string, so
/// quoting cannot demote a `<<` key back to an ordinary one.
const MERGE_KEY_SPELLING: &str = "<<";

/// Parse `path` for [`Document::rename_key`] with a stricter bracket
/// rule than the shared [`parse_query_path`].
///
/// `parse_query_path` *drops* a bracket segment whose content is not
/// an index (`servers[web]` collapses to `servers`). For a read that
/// is a harmless miss, but a rename would then rewrite the *parent*
/// key — a silent, destructive edit the caller never asked for. Here
/// the typo is an error naming the offending segment.
fn parse_rename_path(path: &str) -> Result<Vec<QuerySegment>> {
    let mut rest = path;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let close = after.find(']').unwrap_or(after.len());
        let content = &after[..close];
        if content.parse::<usize>().is_err() {
            return Err(Error::Parse(format!(
                "rename_key: `{path}` contains the bracket segment `[{content}]`, which is not \
                 a sequence index — a bracket segment must hold a non-negative integer, and a \
                 mapping key is addressed with dot notation (`parent.child`)"
            )));
        }
        rest = &after[close..];
    }
    Ok(parse_query_path(path))
}

/// The first character of `key` outside YAML's printable set
/// (§5.1 `c-printable`): any control character other than tab,
/// including `U+007F` and the `U+0080..=U+009F` C1 block.
///
/// [`Document::rename_key`] refuses such a key rather than trying to
/// spell it: the double-quoted formatter escapes only `< U+0020`, so
/// a `U+007F` would be spliced raw and the document would carry
/// bytes the YAML spec does not admit.
fn first_non_printable(key: &str) -> Option<char> {
    key.chars().find(|&c| c != '\t' && c.is_control())
}

/// Decode a mapping-key token's source text to the string it
/// denotes, per its quote style. `None` for token kinds that are not
/// a simple scalar (alias marks and the like), which have no decoded
/// spelling to compare against.
///
/// [`Document::rename_key`] compares this against `new_key` to
/// decide the byte-preserving no-op. Comparing *formatted* spellings
/// instead would requote a plain `true:` into `"true":` on a rename
/// to its own name — a data change, since the key's YAML type
/// switches from bool to string.
fn decode_key_token(raw: &str, kind: SyntaxKind) -> Option<String> {
    match kind {
        // A plain scalar's source text is its content, and a key
        // token never spans lines (implicit keys are single-line;
        // an explicit `? foo` key's trailing break is trimmed off
        // before this point).
        SyntaxKind::PlainScalar => Some(raw.to_owned()),
        SyntaxKind::SingleQuotedScalar => decode_single_quoted(raw).map(Cow::into_owned),
        // Double-quoted escapes (`\t`, `é`, …) need the real
        // scalar parser; the token is self-delimiting, so loading it
        // as a bare scalar document yields exactly the decoded key.
        SyntaxKind::DoubleQuotedScalar => {
            let cfg = crate::parser::ParseConfig::default();
            match crate::parser::parse_one_value(raw, &cfg).ok()? {
                Value::String(s) => Some(s),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The byte span of the node an `&name` anchor decorates, given the
/// anchor mark's start offset. The anchored node is the first
/// non-trivia sibling that follows the mark inside the same green
/// node — a scalar token, or a nested collection.
///
/// Returns `None` when the mark is unknown or decorates nothing
/// (an anchored implicit null has no bytes of its own).
fn anchored_content_span(
    node: &GreenNode,
    base: usize,
    mark_start: usize,
) -> Option<(usize, usize)> {
    let mut pos = base;
    let mut seen_mark = false;
    for child in node.children() {
        let len = child.text_len();
        if seen_mark {
            let trivia = matches!(
                child,
                GreenChild::Token { kind, .. }
                    if matches!(
                        kind,
                        SyntaxKind::Whitespace
                            | SyntaxKind::Newline
                            | SyntaxKind::Comment
                            | SyntaxKind::TagMark
                    )
            );
            if !trivia {
                return Some((pos, pos + len));
            }
        } else if pos == mark_start
            && matches!(child, GreenChild::Token { kind, .. } if *kind == SyntaxKind::AnchorMark)
        {
            seen_mark = true;
        }
        pos += len;
    }
    if seen_mark {
        // The mark was the last meaningful child — nothing anchored.
        return None;
    }
    // Not at this level: descend into the child that contains it.
    let mut pos = base;
    for child in node.children() {
        let len = child.text_len();
        if let GreenChild::Node(inner) = child {
            if pos <= mark_start && mark_start < pos + len {
                return anchored_content_span(inner, pos, mark_start);
            }
        }
        pos += len;
    }
    None
}

/// Resolve `segments` to the byte span of the *key* token of the
/// mapping entry it addresses, refusing renames that would collide
/// with an existing sibling key.
///
/// The path addresses the entry the same way `set` / `remove` do —
/// it points at the entry's value; the returned span is the entry's
/// key. Mirrors [`entry_line_span`]'s recursion but keeps the key
/// span that resolver discards.
fn entry_key_site(
    value: &Value,
    span_tree: &SpanTree,
    segments: &[QuerySegment],
    new_key: &str,
) -> Result<(usize, usize)> {
    // An alias site substitutes the anchor's (value, tree) — the
    // same unwrapping `resolve_span` does for reads. A *write* here
    // would splice the anchor's bytes, which belong to a different
    // entry, so refuse with that diagnosis rather than letting the
    // wrapper fall through to the catch-all "path not found".
    if matches!(span_tree, SpanTree::Alias(_)) {
        return Err(Error::Parse(
            "rename_key: the path addresses alias-expanded content — an `*name` site reflects \
             the anchor's entries and owns no key bytes of its own; rename the corresponding \
             entry at the anchor's own definition instead"
                .into(),
        ));
    }

    let (head, tail) = segments.split_first().ok_or_else(|| {
        Error::Parse("rename_key requires a non-empty path addressing a mapping entry".into())
    })?;

    // Recurse into nested mappings / sequences until the segment
    // list identifies the entry whose key is being renamed.
    if !tail.is_empty() {
        let (child_value, child_tree) = match (head, value, span_tree) {
            (QuerySegment::Key(k), Value::Mapping(m), SpanTree::Mapping { entries, .. }) => {
                let pos = m
                    .iter()
                    .position(|(mk, _)| mk == k)
                    .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?;
                let (_, child_tree) = entries.get(pos).ok_or_else(|| {
                    // Keys past the span-entry list were introduced
                    // by a `<<` merge key — they have no source
                    // entry of their own in this mapping. Spelled
                    // exactly as the final-segment arm spells it: an
                    // intermediate segment is the same condition.
                    Error::Parse(format!(
                        "rename_key: key {k:?} was produced by a `<<` merge key and has \
                         no entry of its own to rename in this mapping"
                    ))
                })?;
                (
                    m.iter().nth(pos).map(|(_, v)| v).expect("pos in range"),
                    child_tree,
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
        return entry_key_site(child_value, child_tree, tail, new_key);
    }

    // Final segment — locate this entry's key span in the parent
    // mapping and refuse a rename that would duplicate a sibling.
    match (head, value, span_tree) {
        (QuerySegment::Key(k), Value::Mapping(m), SpanTree::Mapping { entries, .. }) => {
            let pos = m
                .iter()
                .position(|(mk, _)| mk == k)
                .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?;
            if k != new_key && m.contains_key(new_key) {
                // A key beyond the span-entry list came from a `<<`
                // merge: the mapping has no entry of its own by that
                // name, so the result would not be a duplicate — it
                // would be an explicit key *overriding* the merged
                // value. Still refused, but not for that reason.
                let merge_provided = m
                    .get_index_of(new_key)
                    .is_some_and(|idx| idx >= entries.len());
                if merge_provided {
                    return Err(Error::Parse(format!(
                        "rename_key: {new_key:?} is provided by a `<<` merge key in this \
                         mapping — renaming {k:?} to it would create an explicit entry that \
                         overrides the merged value instead of renaming in place"
                    )));
                }
                return Err(Error::Parse(format!(
                    "rename_key: the mapping already has an entry named {new_key:?} — \
                     renaming {k:?} would create a duplicate key"
                )));
            }
            let (key_span, _) = entries.get(pos).ok_or_else(|| {
                Error::Parse(format!(
                    "rename_key: key {k:?} was produced by a `<<` merge key and has \
                     no entry of its own to rename in this mapping"
                ))
            })?;
            Ok(*key_span)
        }
        (QuerySegment::Index(_), _, _) => Err(Error::Parse(
            "rename_key: path must address a mapping entry, not a sequence item".into(),
        )),
        _ => Err(Error::Parse("path not found".into())),
    }
}

/// Locate the token leaf containing byte `target` and return its
/// kind, byte range, and the [`SyntaxKind`] of its immediate green
/// parent. The parent kind lets [`Document::rename_key`] tell a
/// block-mapping key (parent `MappingEntry`) from a flow-mapping
/// key (parent `FlowMapping` — flow content is kept flat, see
/// [`SyntaxKind::FlowMapping`]).
fn token_at_with_parent(
    node: &GreenNode,
    target: usize,
    base: usize,
) -> Option<(SyntaxKind, (usize, usize), SyntaxKind)> {
    let mut pos = base;
    for child in node.children() {
        let len = child.text_len();
        if pos <= target && target < pos + len {
            return match child {
                GreenChild::Token { kind, .. } => Some((*kind, (pos, pos + len), node.kind())),
                GreenChild::Node(inner) => token_at_with_parent(inner, target, pos),
            };
        }
        pos += len;
    }
    None
}

/// YAML spelling for a mapping key that replaces a key token of
/// `kind`, style-matched to the site: a quoted site keeps its quote
/// style, and a plain site stays plain when the plain spelling
/// re-parses to exactly `key` (delegating to [`is_plain_safe`]),
/// falling back to double quotes when it would not.
fn format_key_for_site(key: &str, kind: SyntaxKind) -> String {
    // Single-quoted YAML cannot represent control characters (and a
    // line break inside single quotes folds — the decoded key would
    // differ); fall back to double quotes for those.
    let single_representable = !key.bytes().any(|b| b < 0x20 || b == 0x7F);
    match kind {
        SyntaxKind::SingleQuotedScalar if single_representable => format_single_quoted(key),
        SyntaxKind::DoubleQuotedScalar => format_double_quoted(key),
        _ => {
            if is_plain_safe(key) {
                key.to_owned()
            } else {
                format_double_quoted(key)
            }
        }
    }
}

/// The typed value the document must load to after renaming the
/// entry at `segments` to `new_key`: the pre-edit value with exactly
/// that one key renamed — same entry position, same value. Used as
/// the post-splice integrity oracle by [`Document::rename_key`].
fn expected_after_rename(value: &Value, segments: &[QuerySegment], new_key: &str) -> Result<Value> {
    let (last, parents) = segments.split_last().ok_or_else(|| {
        Error::Parse("rename_key requires a non-empty path addressing a mapping entry".into())
    })?;
    let QuerySegment::Key(old_key) = last else {
        return Err(Error::Parse(
            "rename_key: path must address a mapping entry, not a sequence item".into(),
        ));
    };
    let mut expected = value.clone();
    let mut cur = &mut expected;
    for seg in parents {
        cur = match (seg, cur) {
            (QuerySegment::Key(k), Value::Mapping(m)) => m
                .get_mut(k)
                .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?,
            (QuerySegment::Index(i), Value::Sequence(seq)) => seq
                .get_mut(*i)
                .ok_or_else(|| Error::Parse(format!("path not found: index {i} out of bounds")))?,
            _ => return Err(Error::Parse("path not found".into())),
        };
    }
    let Value::Mapping(m) = cur else {
        return Err(Error::Parse("path not found".into()));
    };
    let mut renamed = Mapping::with_capacity(m.len());
    for (k, v) in m.iter() {
        if k == old_key {
            let _ = renamed.insert(new_key, v.clone());
        } else {
            let _ = renamed.insert(k.clone(), v.clone());
        }
    }
    *m = renamed;
    Ok(expected)
}

/// Build the `items[i]` path for `Document::span_at`, handling the
/// root-sequence case where `path` is empty.
fn item_child_path(path: &str, i: usize) -> String {
    if path.is_empty() {
        format!("[{i}]")
    } else {
        format!("{path}[{i}]")
    }
}

/// Length of the sequence addressed by `segments`, or an error naming
/// `path` if it does not resolve to a sequence.
fn sequence_len_at(value: &Value, segments: &[QuerySegment], path: &str) -> Result<usize> {
    let mut cur = value;
    for seg in segments {
        cur = match (seg, cur) {
            (QuerySegment::Key(k), Value::Mapping(m)) => m
                .get(k)
                .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?,
            (QuerySegment::Index(i), Value::Sequence(seq)) => seq
                .get(*i)
                .ok_or_else(|| Error::Parse(format!("path not found: index {i} out of bounds")))?,
            _ => {
                return Err(Error::Parse(format!(
                    "swap_items: `{path}` does not resolve to a sequence"
                )));
            }
        };
    }
    match cur {
        Value::Sequence(seq) => Ok(seq.len()),
        _ => Err(Error::Parse(format!(
            "swap_items: `{path}` does not address a sequence"
        ))),
    }
}

/// The typed value with items `i` and `j` of the sequence at
/// `segments` exchanged — the integrity oracle for `swap_items`.
fn expected_after_swap(
    value: &Value,
    segments: &[QuerySegment],
    i: usize,
    j: usize,
    path: &str,
) -> Result<Value> {
    let mut expected = value.clone();
    let mut cur = &mut expected;
    for seg in segments {
        cur = match (seg, cur) {
            (QuerySegment::Key(k), Value::Mapping(m)) => m
                .get_mut(k)
                .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?,
            (QuerySegment::Index(idx), Value::Sequence(seq)) => {
                seq.get_mut(*idx).ok_or_else(|| {
                    Error::Parse(format!("path not found: index {idx} out of bounds"))
                })?
            }
            _ => {
                return Err(Error::Parse(format!(
                    "swap_items: `{path}` does not resolve to a sequence"
                )));
            }
        };
    }
    let Value::Sequence(seq) = cur else {
        return Err(Error::Parse(format!(
            "swap_items: `{path}` does not address a sequence"
        )));
    };
    let vi = seq
        .get(i)
        .cloned()
        .ok_or_else(|| Error::Parse(format!("swap_items: index {i} out of bounds")))?;
    let vj = seq
        .get(j)
        .cloned()
        .ok_or_else(|| Error::Parse(format!("swap_items: index {j} out of bounds")))?;
    *seq.get_mut(i).expect("index i checked above") = vj;
    *seq.get_mut(j).expect("index j checked above") = vi;
    Ok(expected)
}

/// The typed value with `key` set to `child` in the mapping at
/// `mapping_path` — the integrity oracle for
/// [`Document::insert_entry_value`].
///
/// An existing key keeps its position (the insertion replaces its
/// value in place, as `insert_entry` does); a new key lands last.
fn expected_after_insert_entry(
    value: &Value,
    mapping_path: &str,
    key: &str,
    child: &Value,
) -> Result<Value> {
    let mut expected = value.clone();
    let cur = if mapping_path.is_empty() {
        &mut expected
    } else {
        path_value_mut(&mut expected, &parse_query_path(mapping_path))
            .ok_or_else(|| Error::Parse(format!("path not found: {mapping_path}")))?
    };
    let Value::Mapping(m) = cur else {
        return Err(Error::Parse(format!(
            "`{mapping_path}` does not address a mapping"
        )));
    };
    let _ = m.insert(key, child.clone());
    Ok(expected)
}

/// The typed value with `item` inserted at `index` of the sequence at
/// `seq_path` — the integrity oracle for
/// [`Document::push_back_value`] and
/// [`Document::insert_after_value`].
fn expected_after_insert_item(
    value: &Value,
    seq_path: &str,
    index: usize,
    item: &Value,
) -> Result<Value> {
    let mut expected = value.clone();
    let cur = if seq_path.is_empty() {
        &mut expected
    } else {
        path_value_mut(&mut expected, &parse_query_path(seq_path))
            .ok_or_else(|| Error::Parse(format!("path not found: {seq_path}")))?
    };
    let Value::Sequence(seq) = cur else {
        return Err(Error::Parse(format!(
            "`{seq_path}` does not address a sequence"
        )));
    };
    if index > seq.len() {
        return Err(Error::Parse(format!(
            "index {index} is past the end of the sequence at `{seq_path}` (length {})",
            seq.len()
        )));
    }
    seq.insert(index, item.clone());
    Ok(expected)
}

/// Mutable analogue of [`path_value`], resolving pre-parsed segments
/// against a `Value` tree.
fn path_value_mut<'a>(value: &'a mut Value, segments: &[QuerySegment]) -> Option<&'a mut Value> {
    let mut cur = value;
    for seg in segments {
        cur = match (seg, cur) {
            (QuerySegment::Key(k), Value::Mapping(m)) => m.get_mut(k)?,
            (QuerySegment::Index(i), Value::Sequence(seq)) => seq.get_mut(*i)?,
            _ => return None,
        };
    }
    Some(cur)
}

/// The sequence path an item path addresses, i.e. `items[2]` →
/// `items`, `[0]` → `` (the root sequence).
fn sequence_parent_path(item_path: &str) -> String {
    match item_path.rfind('[') {
        Some(i) => item_path[..i].to_owned(),
        None => item_path.to_owned(),
    }
}

/// Re-indent every line of `fragment` after the first to `indent`
/// spaces, leaving the first line alone because the splice template
/// has already placed it (after a `- ` indicator or a `key: `).
///
/// Blank lines stay blank — trailing whitespace on an empty line is
/// noise the emitters never introduce deliberately.
fn indent_continuation_lines(fragment: &str, indent: usize, nl: &str) -> String {
    if !fragment.contains('\n') {
        return fragment.to_owned();
    }
    let pad = " ".repeat(indent);
    let mut out = String::with_capacity(fragment.len() + indent * 4);
    for (i, line) in fragment.split('\n').enumerate() {
        if i > 0 {
            // An emission is always `\n`-separated; the breaks it grows
            // when spliced take the document's spelling instead.
            out.push_str(nl);
            if !line.is_empty() {
                out.push_str(&pad);
            }
        }
        out.push_str(line);
    }
    out
}

/// The typed value with the entry at `segments` removed — the integrity
/// oracle for the multi-line `remove` path.
/// A structural fingerprint of `value` with the subtree at `segments`
/// elided.
///
/// Compares *shape* — which keys exist, how long sequences are — and
/// deliberately not scalar contents. That distinction is the whole
/// point:
///
/// - a fragment smuggling `\nc: 3` into the source **adds a key**, which
///   changes the shape and is refused;
/// - editing an anchored value **changes scalars** at existing paths
///   when its aliases are re-read, which leaves the shape identical and
///   is allowed. That is a documented feature, and an earlier
///   value-comparing version of this oracle wrongly rejected it.
impl Document {
    /// Run `edit`, then require the document's shape outside
    /// `container_path` to be unchanged.
    ///
    /// The inserters legitimately change the shape *of the container
    /// they insert into* — that is their job — so the container's
    /// subtree is elided and everything else must match. Without this a
    /// fragment containing a newline escapes:
    ///
    /// ```text
    /// push_back("s", "v\nqq: 7")  on  "s:\n  - 1\nz: 9\n"
    /// ```
    ///
    /// appended `- v` to the sequence *and* gave the document a
    /// top-level `qq`, returning `Ok`, because the result is valid
    /// YAML.
    fn guarded_insert<F>(
        &mut self,
        container_path: &str,
        what: &str,
        growth: InsertGrowth<'_>,
        edit: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        self.ensure_cache();
        let segments = parse_query_path(container_path);
        let (before_shape, before_container) = {
            let cache = self.cache.borrow();
            let (value, _) = cache.as_ref().expect("ensure_cache populated");
            let container = if container_path.is_empty() {
                Some(value)
            } else {
                path_value(value, container_path)
            };
            let before = container.and_then(|c| match (&growth, c) {
                (InsertGrowth::SeqPlusOne, Value::Sequence(s)) => {
                    Some(ContainerBefore::Seq(s.len()))
                }
                (InsertGrowth::MapEntry(_), Value::Mapping(m)) => Some(ContainerBefore::Map(
                    m.keys().map(|k| k.as_str().to_owned()).collect(),
                )),
                _ => None,
            });
            (shape_excluding(value, &segments), before)
        };

        let snapshot = self.clone();
        edit(self)?;

        // Fallible parse: an invalid splice commits optimistically by
        // design and surfaces via `validate`, and cannot be smuggling
        // entries anyway.
        let Ok(after_value) = crate::from_str::<Value>(&self.source) else {
            return Ok(());
        };
        if shape_excluding(&after_value, &segments) != before_shape {
            *self = snapshot;
            return Err(Error::Parse(format!(
                "{what}: the fragment added or removed entries outside \
                 `{container_path}` — the document was left unchanged. Use \
                 the `_value` variant to write a value without splicing YAML."
            )));
        }
        // The shape above elides the container — changing it is the
        // insert's job — so pin that change to exactly the one entry
        // asked for. Without this the smuggling moves *inside*:
        //
        //     push_back("s", "v\n  - w")
        //
        // kept every byte at the container's indent and appended two
        // items at `Ok`.
        if let Some(before) = before_container {
            let after_container = if container_path.is_empty() {
                Some(&after_value)
            } else {
                path_value(&after_value, container_path)
            };
            let grown_exactly = match (&growth, &before, after_container) {
                (InsertGrowth::SeqPlusOne, ContainerBefore::Seq(n), Some(Value::Sequence(s))) => {
                    s.len() == n + 1
                }
                (
                    InsertGrowth::MapEntry(key),
                    ContainerBefore::Map(keys),
                    Some(Value::Mapping(m)),
                ) => {
                    // Expected: the pre-edit keys plus `key`.
                    // Already-present covers the `<<`-override insert,
                    // which replaces an inherited value without
                    // growing the typed view. Order-insensitive: where
                    // a merge places its inherited keys is the
                    // loader's business, not this oracle's.
                    let mut expected: Vec<&str> = keys.iter().map(String::as_str).collect();
                    if !expected.contains(key) {
                        expected.push(key);
                    }
                    let mut after: Vec<&str> = m.keys().map(|k| k.as_str()).collect();
                    expected.sort_unstable();
                    after.sort_unstable();
                    expected == after
                }
                _ => false,
            };
            if !grown_exactly {
                *self = snapshot;
                return Err(Error::Parse(format!(
                    "{what}: the fragment changed `{container_path}` beyond the single \
                     entry asked for — the document was left unchanged. Use the \
                     `_value` variant to write a value without splicing YAML."
                )));
            }
        }
        Ok(())
    }
}

/// What one insert may do to its container — the growth half of
/// [`Document::guarded_insert`]'s oracle.
enum InsertGrowth<'a> {
    /// The container sequence must end up exactly one item longer.
    SeqPlusOne,
    /// The container mapping's keys must become exactly the pre-edit
    /// keys plus this one.
    MapEntry(&'a str),
}

/// The container's pre-edit fingerprint for [`InsertGrowth`].
enum ContainerBefore {
    Seq(usize),
    Map(Vec<String>),
}

fn shape_excluding(value: &Value, segments: &[QuerySegment]) -> String {
    fn walk(v: &Value, skip: &[QuerySegment], out: &mut String) {
        // Reaching the elided path contributes a fixed marker, so the
        // target may change freely — including scalar to mapping.
        if skip.is_empty() {
            out.push_str("<target>");
            return;
        }
        match v {
            Value::Mapping(m) => {
                out.push('{');
                for (k, val) in m {
                    out.push_str(k.as_str());
                    out.push(':');
                    let next = match skip.first() {
                        Some(QuerySegment::Key(sk)) if sk.as_str() == k.as_str() => &skip[1..],
                        _ => &[][..],
                    };
                    if next.len() < skip.len() {
                        walk(val, next, out);
                    } else {
                        walk_all(val, out);
                    }
                    out.push(',');
                }
                out.push('}');
            }
            Value::Sequence(s) => {
                out.push('[');
                for (i, val) in s.iter().enumerate() {
                    let next = match skip.first() {
                        Some(QuerySegment::Index(si)) if *si == i => &skip[1..],
                        _ => &[][..],
                    };
                    if next.len() < skip.len() {
                        walk(val, next, out);
                    } else {
                        walk_all(val, out);
                    }
                    out.push(',');
                }
                out.push(']');
            }
            _ => out.push('_'),
        }
    }

    /// Shape of a subtree with nothing elided.
    fn walk_all(v: &Value, out: &mut String) {
        match v {
            Value::Mapping(m) => {
                out.push('{');
                for (k, val) in m {
                    out.push_str(k.as_str());
                    out.push(':');
                    walk_all(val, out);
                    out.push(',');
                }
                out.push('}');
            }
            Value::Sequence(s) => {
                out.push('[');
                for val in s {
                    walk_all(val, out);
                    out.push(',');
                }
                out.push(']');
            }
            _ => out.push('_'),
        }
    }

    let mut out = String::new();
    walk(value, segments, &mut out);
    out
}

/// The typed value at `segments`, walking mappings by key and sequences
/// by index. `None` when the path does not resolve. Read-only sibling of
/// [`expected_after_remove`]'s navigation loop; used by `set_value`'s
/// no-op short-circuit.
fn typed_value_at<'v>(value: &'v Value, segments: &[QuerySegment]) -> Option<&'v Value> {
    let mut cur = value;
    for seg in segments {
        cur = match (seg, cur) {
            (QuerySegment::Key(k), Value::Mapping(m)) => m.get(k)?,
            (QuerySegment::Index(i), Value::Sequence(seq)) => seq.get(*i)?,
            _ => return None,
        };
    }
    Some(cur)
}

fn expected_after_remove(value: &Value, segments: &[QuerySegment]) -> Result<Value> {
    let (last, parents) = segments
        .split_last()
        .ok_or_else(|| Error::Parse("remove requires a non-empty path".into()))?;
    let mut expected = value.clone();
    let mut cur = &mut expected;
    for seg in parents {
        cur = match (seg, cur) {
            (QuerySegment::Key(k), Value::Mapping(m)) => m
                .get_mut(k)
                .ok_or_else(|| Error::Parse(format!("path not found: missing key {k:?}")))?,
            (QuerySegment::Index(i), Value::Sequence(seq)) => seq
                .get_mut(*i)
                .ok_or_else(|| Error::Parse(format!("path not found: index {i} out of bounds")))?,
            _ => return Err(Error::Parse("path not found".into())),
        };
    }
    match (last, cur) {
        (QuerySegment::Key(k), Value::Mapping(m)) => {
            let mut rebuilt = Mapping::with_capacity(m.len().saturating_sub(1));
            for (mk, mv) in m.iter() {
                if mk != k {
                    let _ = rebuilt.insert(mk.clone(), mv.clone());
                }
            }
            *m = rebuilt;
            Ok(expected)
        }
        (QuerySegment::Index(i), Value::Sequence(seq)) => {
            if *i >= seq.len() {
                return Err(Error::Parse(format!(
                    "path not found: index {i} out of bounds"
                )));
            }
            let _ = seq.remove(*i);
            Ok(expected)
        }
        _ => Err(Error::Parse("path not found".into())),
    }
}

/// Index of the first `segments` entry with no existing node under
/// `root`, walking the typed view. `segments.len()` means the whole
/// path resolves. An existing segment whose value cannot be descended
/// through — a scalar, a null (other than the empty document root), a
/// sequence where a key is asked for — is an error naming it, so
/// [`Document::set_path`] refuses before touching a byte (#327).
fn first_missing_segment(root: &Value, segments: &[QuerySegment], path: &str) -> Result<usize> {
    let mut cursor = root;
    for (i, segment) in segments.iter().enumerate() {
        match segment {
            QuerySegment::Key(key) => match cursor {
                Value::Mapping(map) => match map.get(key.as_str()) {
                    Some(child) => cursor = child,
                    None => return Ok(i),
                },
                // The null *document* has no bytes claiming the root is
                // null; everything is missing. An explicit `null` is
                // told apart later, by the pre-edit candidate check.
                Value::Null if i == 0 => return Ok(0),
                Value::Null => {
                    return Err(Error::Parse(format!(
                        "set_path: `{}` resolves to a null value — filling an implicit \
                         null with a new mapping is a fragment edit for now; splice it \
                         with `set` (`{path}` was not written)",
                        format_query_prefix(&segments[..i]),
                    )));
                }
                _ => {
                    return Err(Error::Parse(format!(
                        "set_path: `{}` resolves to a non-mapping, so `{path}` cannot \
                         descend through it; the document was left unchanged",
                        format_query_prefix(&segments[..i]),
                    )));
                }
            },
            QuerySegment::Index(index) => match cursor {
                Value::Sequence(seq) => match seq.get(*index) {
                    Some(child) => cursor = child,
                    None => return Ok(i),
                },
                _ => {
                    return Err(Error::Parse(format!(
                        "set_path: `{}` resolves to a non-sequence, so `{path}` cannot \
                         index into it; the document was left unchanged",
                        format_query_prefix(&segments[..i]),
                    )));
                }
            },
            QuerySegment::Wildcard | QuerySegment::RecursiveDescent => {
                return Err(Error::Parse(format!(
                    "set_path: `{path}` contains a wildcard or recursive-descent segment, \
                     which does not address a single entry"
                )));
            }
        }
    }
    Ok(segments.len())
}

/// Render `segments` back into the dotted/bracketed path syntax
/// (`a.b[2].c`) so an ancestor prefix can be handed to the existing
/// path-addressed mutators.
fn format_query_prefix(segments: &[QuerySegment]) -> String {
    use core::fmt::Write as _;
    let mut out = String::new();
    for segment in segments {
        match segment {
            QuerySegment::Key(key) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(key);
            }
            QuerySegment::Index(index) => {
                let _ = write!(out, "[{index}]");
            }
            QuerySegment::Wildcard | QuerySegment::RecursiveDescent => {}
        }
    }
    out
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
/// The line break this document uses, for a splice that adds a line.
///
/// A splice derives its indentation from the site rather than assuming
/// two spaces; the terminator deserves the same treatment. Returns
/// `"\r\n"` only when the document is *wholly* CRLF — at least one break,
/// and every `\n` preceded by `\r` — so the answer is the document's
/// convention rather than a guess. An LF document, a mixed one, and a
/// single unterminated line all yield `"\n"`: mixed input has no
/// convention to honour, and picking one would rewrite bytes the caller
/// did not ask about.
fn document_break(source: &str) -> &'static str {
    let bytes = source.as_bytes();
    let mut saw = false;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            if i == 0 || bytes[i - 1] != b'\r' {
                return "\n";
            }
            saw = true;
        }
    }
    if saw { "\r\n" } else { "\n" }
}

/// The line break a splice at `pos` must supply for itself, if any.
///
/// [`end_of_line`] returns the byte after the line's `\n`, or the end
/// of the source when the last line has no terminator. Splicing a new
/// entry at that second position would land it on the tail of the last
/// line (`a: 1  b: 2`), so the new text has to open with a break of its
/// own — in the document's own spelling ([`document_break`]).
/// Everywhere else this is empty.
fn leading_break_for_splice(source: &str, pos: usize) -> &'static str {
    if pos == 0 || source.as_bytes()[pos - 1] == b'\n' {
        ""
    } else {
        document_break(source)
    }
}

/// Position of the first byte of the line containing `pos`: `0`, or the
/// byte just past the preceding `\n`. The mirror of [`end_of_line`].
fn start_of_line(source: &str, pos: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = pos.min(bytes.len());
    while i > 0 && bytes[i - 1] != b'\n' {
        i -= 1;
    }
    i
}

fn end_of_line(source: &str, pos: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = pos;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    if i < bytes.len() { i + 1 } else { i }
}

/// [`locate_preceding_dash`] for an implicit-null item, which owns no
/// bytes: its empty value span sits *at* the `-` indicator itself (the
/// scanner marks the missing value there), or -- when trailing blanks
/// intervene -- past the line break. Both shapes resolve to the dash
/// (#336).
fn locate_dash_at_or_across_break(source: &str, value_start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(value_start) == Some(&b'-') {
        return Some(value_start);
    }
    let mut i = value_start;
    if i > 0 && bytes[i - 1] == b'\n' {
        i -= 1;
        if i > 0 && bytes[i - 1] == b'\r' {
            i -= 1;
        }
    }
    locate_preceding_dash(source, i)
}

/// The position just past the indentation of the line starting at
/// `line_start`: spaces and tabs skipped, stopping at content or the
/// line break.
fn skip_line_indent(source: &str, line_start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = line_start;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    i
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
    /// Whether the leaf sits inside a `[…]` / `{…}` flow collection.
    /// Flow context forbids block scalars and gives `,` `[` `]` `{`
    /// `}` structural meaning anywhere in a plain scalar, so the
    /// spelling rules differ from block context (#332).
    in_flow: bool,
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

pub(super) fn format_number(n: &Number) -> String {
    // `Number`'s `Display` matches the YAML 1.2 plain representation
    // for the integer/float variants we emit here.
    n.to_string()
}

fn format_string_for_site(s: &str, ctx: &SiteContext) -> Result<String> {
    // CR and the Unicode line separators survive only double-quoted,
    // escaped: a literal block normalises `\r` into its own line
    // breaks, and NEL/LS/PS pass through plain or single-quoted styles
    // as raw bytes that read back as line breaks (#335).
    if s.chars().any(|c| {
        matches!(
            c,
            '\r' | '\u{0085}' | '\u{2028}' | '\u{2029}' | '\u{feff}' | '\u{7f}'
        ) || (c < '\u{20}' && c != '\t' && c != '\n')
    }) {
        return Ok(format_double_quoted(s));
    }

    if ctx.in_flow {
        return Ok(format_string_in_flow(s, ctx.kind));
    }
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

/// Spell `s` for a leaf inside a flow collection. Block scalars do
/// not exist in flow context, so a multi-line value is double-quoted
/// with `\n` escapes; a one-line value keeps the leaf's quoting style,
/// and a plain leaf stays plain only when [`is_plain_safe_in_flow`]
/// says the flow indicators cannot misread it (#332).
fn format_string_in_flow(s: &str, kind: SyntaxKind) -> String {
    if s.contains('\n') {
        return format_double_quoted(s);
    }
    match kind {
        SyntaxKind::SingleQuotedScalar => format_single_quoted(s),
        SyntaxKind::DoubleQuotedScalar => format_double_quoted(s),
        _ => {
            if is_plain_safe_in_flow(s) {
                s.to_string()
            } else {
                format_double_quoted(s)
            }
        }
    }
}

/// Whether the leaf at byte `target` has a `FlowMapping` or
/// `FlowSequence` ancestor. Block collections cannot nest inside flow
/// ones, so any flow ancestor means the leaf is in flow context.
fn in_flow_collection(node: &GreenNode, target: usize) -> bool {
    let mut pos = 0;
    for child in node.children() {
        let len = child.text_len();
        if pos <= target && target < pos + len {
            return match child {
                GreenChild::Token { .. } => false,
                GreenChild::Node(inner) => {
                    matches!(
                        inner.kind(),
                        SyntaxKind::FlowMapping | SyntaxKind::FlowSequence
                    ) || in_flow_collection(inner, target - pos)
                }
            };
        }
        pos += len;
    }
    false
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
pub(super) fn can_use_block_literal(s: &str) -> bool {
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
    // A block literal needs at least one content line. `"\n"` strips to
    // nothing, which would emit a header with an empty body —
    //
    //     a: |
    //     b: 2
    //
    // and that does not parse. Double quoting carries it instead. Found
    // by the `set_value` round-trip property test.
    if trimmed.is_empty() {
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
pub(super) fn format_block_literal(s: &str, entry_col: usize) -> String {
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

/// [`is_plain_safe`] for a leaf inside a flow collection, where `,`
/// `[` `]` `{` `}` end or nest a collection wherever they appear in a
/// plain scalar (YAML 1.2 §7.3.3, `ns-plain-safe(c)`), not only at
/// the first byte. `m: {a: x, y}` written for the string `x, y` reads
/// back as two entries; `{a: x {y}}` does not parse at all (#332).
pub(super) fn is_plain_safe_in_flow(s: &str) -> bool {
    is_plain_safe(s)
        && !s
            .bytes()
            .any(|b| matches!(b, b',' | b'[' | b']' | b'{' | b'}'))
}

/// `true` if `s` can be safely emitted as a YAML plain scalar without
/// being misparsed as a different type (bool, null, number) or
/// triggering a structural indicator. Conservative — when in doubt,
/// the caller falls back to a quoted style.
pub(super) fn is_plain_safe(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // NEL/LS/PS read back as line breaks; only double-quoted escapes
    // carry them (#335). A `\r` is caught by the control-byte loop.
    if s.contains(['\u{0085}', '\u{2028}', '\u{2029}', '\u{feff}']) {
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
    // A scalar starting with `...` at column 0 reads as the
    // document-end marker (`-`-leading strings are caught by the
    // first-byte check below; `...` needs its own).
    if s.starts_with("...") {
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
    // Cannot end with `:`. The loop below rejects `": "`, but a colon at
    // the very end has no following byte to catch it there — and in
    // block context a trailing colon reads as a mapping indicator, so
    //
    //     a: a:
    //
    // does not parse. Found by the `set_value` round-trip property
    // test, which proptest minimised to `"a:"`.
    if *bytes.last().unwrap() == b':' {
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

pub(super) fn format_single_quoted(s: &str) -> String {
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

pub(super) fn format_double_quoted(s: &str) -> String {
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
            '\u{0085}' => out.push_str("\\N"),
            '\u{2028}' => out.push_str("\\L"),
            '\u{2029}' => out.push_str("\\P"),
            // A raw BOM must never reach the output — the reader
            // rejects one inside a document (§5.2).
            '\u{feff}' => out.push_str("\\uFEFF"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                let _ = write!(&mut out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
#[cfg(test)]
mod absorb_emptied_line_tests {
    //! Direct unit coverage for @zoosky's #294 helper.
    //!
    //! The integration tests in `tests/cst_remove_wrapped_flow.rs` drive
    //! this through `remove`, which is the behaviour that matters. These
    //! pin the boundary decisions at the byte level, where an off-by-one
    //! is legible — the difference between reclaiming a line and eating
    //! the newline that ends the one above it.

    use super::{absorb_emptied_line, end_of_line, flow_member_range, start_of_line};

    /// Byte range of the first occurrence of `needle` in `hay`.
    fn span(hay: &str, needle: &str) -> (usize, usize) {
        let s = hay.find(needle).expect("needle present");
        (s, s + needle.len())
    }

    #[test]
    fn widens_when_the_member_is_alone_on_its_line() {
        let src = "ports: [\n  80,\n  443,\n]\n";
        let (s, e) = span(src, "80,");
        let (ws, we) = absorb_emptied_line(src, s, e);
        assert_eq!(&src[ws..we], "  80,\n", "takes indentation and terminator");
    }

    #[test]
    fn refuses_when_an_opening_indicator_shares_the_line() {
        let src = "ports: [80,\n  443,\n]\n";
        let (s, e) = span(src, "80,");
        assert_eq!(absorb_emptied_line(src, s, e), (s, e), "unchanged");
    }

    #[test]
    fn refuses_when_a_sibling_shares_the_line() {
        let src = "ports: [\n  80, 443,\n]\n";
        let (s, e) = span(src, "80, ");
        assert_eq!(absorb_emptied_line(src, s, e), (s, e), "unchanged");
    }

    #[test]
    fn refuses_when_a_comment_shares_the_line() {
        let src = "ports: [\n  80, # why\n  443,\n]\n";
        let (s, e) = span(src, "80,");
        assert_eq!(absorb_emptied_line(src, s, e), (s, e), "unchanged");
    }

    #[test]
    fn refuses_when_a_closing_indicator_shares_the_line() {
        let src = "ports: [\n  80,\n  443]\n";
        let (s, e) = span(src, "443");
        assert_eq!(absorb_emptied_line(src, s, e), (s, e), "unchanged");
    }

    #[test]
    fn refuses_at_end_of_input_with_no_terminator() {
        // Nothing to reclaim: widening here would report a range that
        // runs past the last byte of a line that never ended.
        let src = "  80";
        let (s, e) = span(src, "80");
        assert_eq!(absorb_emptied_line(src, s, e), (s, e), "unchanged");
    }

    #[test]
    fn keeps_the_carriage_return_with_the_line() {
        let src = "ports: [\r\n  80,\r\n  443,\r\n]\r\n";
        let (s, e) = span(src, "80,");
        let (ws, we) = absorb_emptied_line(src, s, e);
        assert_eq!(&src[ws..we], "  80,\r\n", "CRLF travels with the line");
    }

    #[test]
    fn a_zero_indent_member_still_widens() {
        let src = "[\n1,\n2,\n]\n";
        let (s, e) = span(src, "1,");
        let (ws, we) = absorb_emptied_line(src, s, e);
        assert_eq!(&src[ws..we], "1,\n");
    }

    #[test]
    fn flow_member_range_composes_the_widening() {
        // The separator is taken first, then the line — the order matters,
        // because widening a range that stops short of the comma would
        // leave the comma stranded on the reclaimed line.
        let src = "ports: [\n  80,\n  443,\n]\n";
        let (s, e) = span(src, "80");
        let (rs, re) = flow_member_range(src, s, e);
        assert_eq!(&src[rs..re], "  80,\n");
    }

    #[test]
    fn flow_member_range_is_unchanged_on_a_single_line() {
        let src = "cfg: {a: 1, b: 2}\n";
        let (s, e) = span(src, "a: 1");
        let (rs, re) = flow_member_range(src, s, e);
        assert_eq!(&src[rs..re], "a: 1, ", "member plus one separator, no line");
    }

    #[test]
    fn line_helpers_agree_with_the_widened_range() {
        // Guards the assumption the helper is built on: the widened range
        // is exactly [start_of_line, end_of_line] of the member.
        let src = "ports: [\n  80,\n  443,\n]\n";
        let (s, e) = span(src, "80,");
        let (ws, we) = absorb_emptied_line(src, s, e);
        assert_eq!(ws, start_of_line(src, s));
        assert_eq!(we, end_of_line(src, e));
    }
}
