// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Anchor and alias management.
//!
//! YAML's `&name` (anchor) declares a node by name; `*name` (alias)
//! references it. At parse time the loader resolves every alias to
//! the value of the matching anchor, so the typed [`crate::Value`]
//! tree contains independent copies — but in the *source*, the
//! `&name` site and every `*name` site stay distinct. This module
//! gives callers the visibility and primitives needed to manage
//! both halves of that contract.
//!
//! # The propagation contract
//!
//! [`crate::cst::Document::set`] (and every other lossless mutation)
//! edits the source. When the target byte range happens to be inside
//! an anchored value (i.e. the bytes covered by `&name`'s decorated
//! node), the change *propagates* to every `*name` site automatically
//! on the next load — because aliases are pointers, and re-parsing
//! the new source yields the new value at every site that referenced
//! the anchor.
//!
//! Concretely:
//!
//! ```rust
//! use noyalib::cst::parse_document;
//!
//! let src = "\
//! defaults: &cfg
//!   port: 8080
//! server:
//!   <<: *cfg
//!   host: localhost
//! ";
//! let mut doc = parse_document(src).unwrap();
//! doc.set("defaults.port", "9090").unwrap();
//!
//! // The source still has one `&cfg` and one `*cfg` — but the
//! // anchored value is now 9090, so the alias resolves to 9090 too.
//! let v = doc.as_value();
//! assert_eq!(v["server"]["port"].as_i64(), Some(9090));
//! ```
//!
//! # Breaking aliases
//!
//! Sometimes the user wants the *opposite* — independent copies.
//! [`crate::cst::Document::materialise_alias_at`] replaces a `*name`
//! token with the source text of its anchored value, leaving the
//! result independent from any future edits to the anchor. The
//! current scope handles scalar-valued anchors only; multi-line
//! block-valued anchors return a clear "follow-up" error so callers
//! know to fall back to a manual splice.
//!
//! [`crate::cst::Document::materialise_aliases_of`] is the bulk
//! convenience: materialise every alias for one anchor in one call.
//!
//! # Discovery
//!
//! [`crate::cst::Document::anchors`] and
//! [`crate::cst::Document::aliases`] enumerate every `&name` /
//! `*name` lexeme in source order, returning the byte span of each
//! mark and the name. [`crate::cst::Document::aliases_of`] filters
//! aliases by anchor name — useful before deciding propagate vs
//! break.

use crate::cst::document::Document;
use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::syntax::SyntaxKind;
use crate::error::{Error, Result};
use crate::prelude::*;

/// An `&name` anchor declaration discovered in the document source.
///
/// # Examples
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let doc = parse_document("foo: &id1 1\nbar: 2\n").unwrap();
/// let anchors = doc.anchors();
/// assert_eq!(anchors.len(), 1);
/// assert_eq!(anchors[0].name, "id1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorInfo {
    /// The anchor name, without the leading `&`.
    pub name: String,
    /// Byte range of the `&name` lexeme itself in the document source.
    pub mark_span: (usize, usize),
}

/// A `*name` alias reference discovered in the document source.
///
/// # Examples
///
/// ```
/// use noyalib::cst::parse_document;
///
/// let doc = parse_document("foo: &id1 1\nbar: *id1\n").unwrap();
/// let aliases = doc.aliases();
/// assert_eq!(aliases.len(), 1);
/// assert_eq!(aliases[0].name, "id1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasInfo {
    /// The alias name, without the leading `*`.
    pub name: String,
    /// Byte range of the `*name` lexeme itself in the document source.
    pub mark_span: (usize, usize),
}

impl Document {
    /// Every `&name` declaration in this document, in source order.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document(
    ///     "defaults: &cfg\n  port: 8080\nserver:\n  <<: *cfg\n",
    /// ).unwrap();
    /// let anchors = doc.anchors();
    /// assert_eq!(anchors.len(), 1);
    /// assert_eq!(anchors[0].name, "cfg");
    /// ```
    #[must_use]
    pub fn anchors(&self) -> Vec<AnchorInfo> {
        let mut out = Vec::new();
        walk_marks(self.syntax(), self.source(), 0, |kind, span, name| {
            if kind == SyntaxKind::AnchorMark {
                out.push(AnchorInfo {
                    name: name.to_owned(),
                    mark_span: span,
                });
            }
        });
        out
    }

    /// Every `*name` reference in this document, in source order.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("a: &x 1\nb: *x\nc: *x\n").unwrap();
    /// let aliases = doc.aliases();
    /// assert_eq!(aliases.len(), 2);
    /// ```
    #[must_use]
    pub fn aliases(&self) -> Vec<AliasInfo> {
        let mut out = Vec::new();
        walk_marks(self.syntax(), self.source(), 0, |kind, span, name| {
            if kind == SyntaxKind::AliasMark {
                out.push(AliasInfo {
                    name: name.to_owned(),
                    mark_span: span,
                });
            }
        });
        out
    }

    /// Aliases whose name matches `name`, in source order.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("a: &x 1\nb: &y 2\nc: *x\nd: *y\n").unwrap();
    /// let xs = doc.aliases_of("x");
    /// assert_eq!(xs.len(), 1);
    /// assert_eq!(xs[0].name, "x");
    /// ```
    #[must_use]
    pub fn aliases_of(&self, name: &str) -> Vec<AliasInfo> {
        self.aliases()
            .into_iter()
            .filter(|a| a.name == name)
            .collect()
    }

    /// Replace the `*name` alias whose mark begins at byte
    /// `position` with the source text of the matching `&name`'s
    /// scalar value.
    ///
    /// After the splice, the alias's site holds an independent copy
    /// of the anchored scalar — subsequent edits to the anchored
    /// value do not propagate to it.
    ///
    /// # Errors
    ///
    /// - `position` does not start an `*name` token.
    /// - The named anchor is not declared earlier in source order.
    /// - The anchored value is not a scalar (multi-line block
    ///   collections require manual handling — read the anchor's
    ///   span via [`Self::anchors`] and splice with
    ///   [`Self::replace_span`]).
    /// - The same parse-after-edit errors as
    ///   [`Self::replace_span`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("a: &x 7\nb: *x\n").unwrap();
    /// let alias = doc.aliases()[0].clone();
    /// doc.materialise_alias_at(alias.mark_span.0).unwrap();
    /// assert!(!doc.to_string().contains("*x"));
    /// assert!(doc.to_string().contains("b: 7"));
    /// ```
    pub fn materialise_alias_at(&mut self, position: usize) -> Result<()> {
        let aliases = self.aliases();
        let alias = aliases
            .iter()
            .find(|a| a.mark_span.0 == position)
            .ok_or_else(|| {
                Error::Parse(format!(
                    "materialise_alias_at: no alias mark begins at byte {position}"
                ))
            })?
            .clone();

        // Find the matching anchor declared earlier in source order.
        // YAML 1.2.2 §7.1 says aliases reference the *most recent*
        // matching anchor — we resolve to the closest preceding one.
        let anchor_value_text = self
            .anchored_scalar_text(&alias.name, alias.mark_span.0)?
            .to_owned();

        self.replace_span(alias.mark_span.0, alias.mark_span.1, &anchor_value_text)
    }

    /// Materialise every alias whose name matches `name`. Returns
    /// the count of aliases replaced.
    ///
    /// Aliases are processed in *reverse* source order so each
    /// splice's offsets stay valid for later (earlier in source)
    /// aliases.
    ///
    /// # Errors
    ///
    /// As [`Self::materialise_alias_at`]. The first failing alias
    /// aborts the batch — already-materialised aliases stay
    /// materialised, the rest are unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("a: &x 7\nb: *x\nc: *x\n").unwrap();
    /// let n = doc.materialise_aliases_of("x").unwrap();
    /// assert_eq!(n, 2);
    /// assert!(!doc.to_string().contains('*'));
    /// ```
    pub fn materialise_aliases_of(&mut self, name: &str) -> Result<usize> {
        let mut targets: Vec<usize> = self
            .aliases_of(name)
            .iter()
            .map(|a| a.mark_span.0)
            .collect();
        targets.sort_unstable();
        targets.reverse();
        let total = targets.len();
        for pos in targets {
            self.materialise_alias_at(pos)?;
        }
        Ok(total)
    }

    /// Rename every `&old` anchor declaration and every `*old`
    /// alias reference to `new` in one atomic pass. Returns the
    /// total number of touched sites (anchors + aliases).
    ///
    /// Splices run in *reverse source order* so each successive
    /// splice's offsets stay valid for earlier sites. The whole
    /// rename is byte-faithful outside the touched marks —
    /// comments, blank lines, and sibling formatting survive
    /// verbatim.
    ///
    /// # Errors
    ///
    /// - `new` is empty or contains characters that would not be
    ///   accepted as a YAML anchor name (any of the flow
    ///   indicators `,[]{}` or whitespace per YAML 1.2 §6.9.2).
    /// - `old` does not match any anchor or alias in the document
    ///   (so the call is a no-op the user probably did not
    ///   intend) — surfaced as an error rather than a silent
    ///   zero-count.
    /// - `new` already names a *different* anchor in the document
    ///   (unless `new == old`): merging the two would make every
    ///   `*new` alias resolve to the last declaration, silently
    ///   changing the document's meaning, so the rename is refused.
    /// - The same parse-after-edit errors as
    ///   [`crate::cst::Document::replace_span`]. The rename is a
    ///   single atomic splice over the whole document, so it is
    ///   all-or-nothing: on any error the document is left
    ///   byte-for-byte unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document(
    ///     "defaults: &cfg\n  port: 8080\nservice:\n  <<: *cfg\nbackup: *cfg\n",
    /// ).unwrap();
    ///
    /// // Rename `cfg` → `defaults`. The single `&cfg` declaration
    /// // and both `*cfg` references are updated in one call.
    /// let n = doc.rename_anchor("cfg", "defaults").unwrap();
    /// assert_eq!(n, 3); // 1 anchor + 2 aliases
    /// let out = doc.to_string();
    /// assert!(!out.contains("&cfg"));
    /// assert!(!out.contains("*cfg"));
    /// assert!(out.contains("&defaults"));
    /// assert!(out.contains("*defaults"));
    /// ```
    pub fn rename_anchor(&mut self, old: &str, new: &str) -> Result<usize> {
        if !is_valid_anchor_name(new) {
            return Err(Error::Parse(format!(
                "rename_anchor: `{new}` is not a valid YAML anchor name \
                 (must be non-empty and free of flow indicators / whitespace)"
            )));
        }

        // Collect every site (anchor or alias) in source order.
        let anchors = self.anchors();
        // Refuse renaming onto a name already used by a *different*
        // anchor. Two `&new` declarations would make every `*new`
        // alias resolve to the last one (YAML 1.2.2 §7.1), silently
        // changing what the document means — the opposite of a safe
        // refactor. A no-op rename (`old == new`) is exempt.
        if new != old && anchors.iter().any(|a| a.name == new) {
            return Err(Error::Parse(format!(
                "rename_anchor: `&{new}` already declares a different anchor; \
                 renaming `{old}` onto it would change alias resolution"
            )));
        }
        let aliases = self.aliases();
        let mut sites: Vec<(char, (usize, usize))> = anchors
            .iter()
            .filter(|a| a.name == old)
            .map(|a| ('&', a.mark_span))
            .chain(
                aliases
                    .iter()
                    .filter(|a| a.name == old)
                    .map(|a| ('*', a.mark_span)),
            )
            .collect();
        if sites.is_empty() {
            return Err(Error::Parse(format!(
                "rename_anchor: no `&{old}` declaration or `*{old}` reference \
                 found in the document"
            )));
        }
        sites.sort_unstable_by_key(|(_, span)| span.0);

        // Build the new source by stitching together the original
        // bytes between sites and the renamed marker text at each
        // site. A single `replace_span` over the whole document
        // commits the atomic edit — intermediate states that would
        // otherwise have a mismatched anchor / alias name pair
        // (and fail re-parse) are never observed.
        let total = sites.len();
        let original = self.source().to_owned();
        let mut new_source = String::with_capacity(original.len());
        let mut cursor = 0;
        for (marker, (start, end)) in sites {
            new_source.push_str(&original[cursor..start]);
            new_source.push(marker);
            new_source.push_str(new);
            cursor = end;
        }
        new_source.push_str(&original[cursor..]);

        self.replace_span(0, original.len(), &new_source)?;
        Ok(total)
    }

    /// Look up `name` as the closest `&name` anchor declared at byte
    /// position `<= before` in source order, and return the source
    /// text of its decorated scalar value. Returns an error if the
    /// name is unknown or the decorated value is not a scalar.
    ///
    /// The CST scanner sometimes emits scalar tokens whose text
    /// includes the trailing line break (a plain scalar at the end
    /// of a line is captured as `"7\n"` rather than `"7"`); we trim
    /// trailing whitespace before classifying so a `7\n`-bearing
    /// scalar is correctly recognised as scalar, not multi-line.
    fn anchored_scalar_text(&self, name: &str, before: usize) -> Result<&str> {
        let source = self.source();
        let mut chosen: Option<(usize, usize)> = None;
        walk_anchor_value_spans(
            self.syntax(),
            source,
            0,
            |anchor_name, mark_span, value_span| {
                if anchor_name == name && mark_span.0 < before {
                    // Last writer in source order wins — YAML 1.2.2 §7.1
                    // resolves to the most recent matching anchor.
                    chosen = Some(value_span);
                }
            },
        );
        let (vs, ve) = chosen.ok_or_else(|| {
            Error::Parse(format!(
                "materialise_alias_at: no `&{name}` anchor declared before byte {before}"
            ))
        })?;
        let raw = &source[vs..ve];
        let trimmed = raw.trim_end_matches(['\n', '\r', ' ', '\t']);
        if trimmed.contains('\n') {
            return Err(Error::Parse(format!(
                "materialise_alias_at: anchor `&{name}` decorates a multi-line block value — \
                 only scalar-valued anchors are materialisable in this scope. \
                 Use `Document::anchors()` + `Document::replace_span()` for manual block splicing."
            )));
        }
        if trimmed.is_empty() {
            return Err(Error::Parse(format!(
                "materialise_alias_at: anchor `&{name}` decorates an empty value"
            )));
        }
        Ok(trimmed)
    }
}

/// Callback signature for [`walk_marks`]: `(kind, mark_span, name)`.
type MarkVisitor<'a> = dyn FnMut(SyntaxKind, (usize, usize), &str) + 'a;

/// Walk every `&name` / `*name` token in `node`, calling `visit`
/// with `(kind, mark_span, name)` for each.
fn walk_marks(
    node: &GreenNode,
    source: &str,
    base: usize,
    mut visit: impl FnMut(SyntaxKind, (usize, usize), &str),
) {
    walk_marks_inner(node, source, base, &mut visit);
}

fn walk_marks_inner(node: &GreenNode, source: &str, base: usize, visit: &mut MarkVisitor<'_>) {
    let mut pos = base;
    for child in node.children() {
        let len = child.text_len();
        match child {
            GreenChild::Token { kind, .. } => {
                if matches!(kind, SyntaxKind::AnchorMark | SyntaxKind::AliasMark) {
                    let span = (pos, pos + len);
                    // Lexeme is `&name` or `*name` — name skips the
                    // marker byte. Both `&` and `*` are single-byte
                    // ASCII, so `pos + 1` is always a char boundary.
                    let name = &source[pos + 1..pos + len];
                    visit(*kind, span, name);
                }
            }
            GreenChild::Node(inner) => walk_marks_inner(inner, source, pos, visit),
        }
        pos += len;
    }
}

/// Callback signature for [`walk_anchor_value_spans`]:
/// `(name, mark_span, value_span)`.
type AnchorValueVisitor<'a> = dyn FnMut(&str, (usize, usize), (usize, usize)) + 'a;

/// Walk every `&name` token and call `visit` with the anchor's name,
/// the mark span, and the byte span of the decorated value (the
/// first non-trivia, non-property sibling that follows the anchor in
/// its parent node). For anchors at the end of their parent with no
/// content sibling, the value span is collapsed to the mark's end
/// byte (an empty slice) — handled as "not a scalar" by callers.
fn walk_anchor_value_spans(
    root: &GreenNode,
    source: &str,
    base: usize,
    mut visit: impl FnMut(&str, (usize, usize), (usize, usize)),
) {
    walk_anchor_value_spans_inner(root, source, base, &mut visit);
}

fn walk_anchor_value_spans_inner(
    node: &GreenNode,
    source: &str,
    base: usize,
    visit: &mut AnchorValueVisitor<'_>,
) {
    let children: Vec<&GreenChild> = node.children().collect();
    let mut pos = base;
    let mut child_starts: Vec<usize> = Vec::with_capacity(children.len());
    for c in &children {
        child_starts.push(pos);
        pos += c.text_len();
    }

    for (i, child) in children.iter().enumerate() {
        let child_start = child_starts[i];
        if let GreenChild::Token { kind, len } = child {
            if *kind == SyntaxKind::AnchorMark {
                let len_u = *len as usize;
                let mark_span = (child_start, child_start + len_u);
                let name = &source[child_start + 1..child_start + len_u];
                let value_span = decorated_value_span(&children, &child_starts, i);
                visit(name, mark_span, value_span);
            }
        }
        if let GreenChild::Node(inner) = child {
            walk_anchor_value_spans_inner(inner, source, child_start, visit);
        }
    }
}

/// Given a parent's `children` and their absolute starting byte
/// positions, plus the index of an `AnchorMark` within them, return
/// the byte span of the value the anchor decorates: the first
/// non-trivia, non-property sibling that follows.
fn decorated_value_span(
    children: &[&GreenChild],
    starts: &[usize],
    anchor_idx: usize,
) -> (usize, usize) {
    let anchor_end = starts[anchor_idx] + children[anchor_idx].text_len();
    for j in (anchor_idx + 1)..children.len() {
        let kind = match children[j] {
            GreenChild::Token { kind, .. } => Some(*kind),
            GreenChild::Node(inner) => Some(inner.kind()),
        };
        let Some(kind) = kind else { continue };
        if is_trivia_or_property(kind) {
            continue;
        }
        let start = starts[j];
        let len = children[j].text_len();
        return (start, start + len);
    }
    // Anchor at end of parent with no content sibling — produce an
    // empty span anchored at the mark's end. Callers that read
    // `source[start..end]` will see "" and treat it as not-a-scalar.
    (anchor_end, anchor_end)
}

/// `true` when `name` is a valid YAML anchor name per §6.9.2 —
/// non-empty, no flow indicators (`,[]{}`), no whitespace.
fn is_valid_anchor_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.bytes().all(|b| {
        !matches!(
            b,
            b',' | b'[' | b']' | b'{' | b'}' | b' ' | b'\t' | b'\r' | b'\n'
        )
    })
}

fn is_trivia_or_property(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Whitespace
            | SyntaxKind::Newline
            | SyntaxKind::Comment
            | SyntaxKind::Bom
            | SyntaxKind::Directive
            | SyntaxKind::TagMark
            | SyntaxKind::AnchorMark
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parse_document;

    #[test]
    fn anchors_listed_in_source_order() {
        let src = "a: &one 1\nb: &two 2\nc: 3\n";
        let doc = parse_document(src).unwrap();
        let anchors = doc.anchors();
        assert_eq!(anchors.len(), 2);
        assert_eq!(anchors[0].name, "one");
        assert_eq!(anchors[1].name, "two");
        // Mark spans must point at the `&name` lexeme.
        let (s, e) = anchors[0].mark_span;
        assert_eq!(&src[s..e], "&one");
    }

    #[test]
    fn aliases_listed_in_source_order() {
        let src = "a: &one 1\nb: *one\nc: *one\n";
        let doc = parse_document(src).unwrap();
        let aliases = doc.aliases();
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0].name, "one");
        assert_eq!(aliases[1].name, "one");
        let (s, e) = aliases[0].mark_span;
        assert_eq!(&src[s..e], "*one");
    }

    #[test]
    fn aliases_of_filters_by_name() {
        let src = "a: &x 1\nb: &y 2\nc: *x\nd: *y\ne: *x\n";
        let doc = parse_document(src).unwrap();
        assert_eq!(doc.aliases_of("x").len(), 2);
        assert_eq!(doc.aliases_of("y").len(), 1);
        assert_eq!(doc.aliases_of("missing").len(), 0);
    }

    #[test]
    fn no_anchors_no_aliases() {
        let doc = parse_document("a: 1\nb: 2\n").unwrap();
        assert!(doc.anchors().is_empty());
        assert!(doc.aliases().is_empty());
    }

    #[test]
    fn anchor_on_block_value_is_visible() {
        let src = "defaults: &cfg\n  port: 8080\n  host: db1\n";
        let doc = parse_document(src).unwrap();
        let anchors = doc.anchors();
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "cfg");
    }

    #[test]
    fn materialise_replaces_alias_with_anchor_text() {
        let src = "a: &x 7\nb: *x\n";
        let mut doc = parse_document(src).unwrap();
        let pos = doc.aliases()[0].mark_span.0;
        doc.materialise_alias_at(pos).unwrap();
        let out = doc.to_string();
        assert_eq!(out, "a: &x 7\nb: 7\n");
        assert!(doc.aliases().is_empty(), "alias must be gone, got: {out}");
    }

    #[test]
    fn materialise_with_quoted_scalar() {
        let src = "a: &x \"hello world\"\nb: *x\n";
        let mut doc = parse_document(src).unwrap();
        let pos = doc.aliases()[0].mark_span.0;
        doc.materialise_alias_at(pos).unwrap();
        assert_eq!(
            doc.to_string(),
            "a: &x \"hello world\"\nb: \"hello world\"\n"
        );
    }

    #[test]
    fn materialise_aliases_of_handles_multiple_in_one_call() {
        let src = "a: &x 7\nb: *x\nc: *x\nd: *x\n";
        let mut doc = parse_document(src).unwrap();
        let n = doc.materialise_aliases_of("x").unwrap();
        assert_eq!(n, 3);
        assert!(doc.aliases().is_empty());
        assert_eq!(doc.anchors().len(), 1);
    }

    #[test]
    fn materialise_block_anchor_errors_with_actionable_message() {
        let src = "defaults: &cfg\n  port: 8080\n  host: db1\nserver: *cfg\n";
        let mut doc = parse_document(src).unwrap();
        let pos = doc.aliases()[0].mark_span.0;
        let err = doc.materialise_alias_at(pos).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("multi-line") && msg.contains("scalar-valued"),
            "error must point at the limitation, got: {msg}"
        );
        // Document is unchanged.
        assert_eq!(doc.to_string(), src);
    }

    #[test]
    fn materialise_unknown_position_errors() {
        let mut doc = parse_document("a: &x 7\nb: *x\n").unwrap();
        let err = doc.materialise_alias_at(0).unwrap_err();
        assert!(err.to_string().contains("no alias mark begins at byte 0"));
    }

    #[test]
    fn edits_to_anchored_value_propagate_to_aliases_on_reload() {
        // The propagation contract documented in this module's
        // rustdoc — set() on the anchor's value updates every alias
        // site automatically because aliases are pointers.
        let src = "\
defaults: &cfg
  port: 8080
server:
  <<: *cfg
  host: localhost
";
        let mut doc = parse_document(src).unwrap();
        doc.set("defaults.port", "9090").unwrap();
        let v = doc.as_value();
        // The merge-key alias resolved to the new anchored value.
        assert_eq!(v["server"]["port"].as_i64(), Some(9090));
        assert_eq!(v["defaults"]["port"].as_i64(), Some(9090));
        // Source still has exactly one anchor and one alias.
        assert_eq!(doc.anchors().len(), 1);
        assert_eq!(doc.aliases().len(), 1);
    }
}
