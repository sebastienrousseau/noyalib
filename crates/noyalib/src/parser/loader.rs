// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Event-to-Value tree builder with security limits.
//!
//! Converts a stream of [`Event`]s directly into `Vec<(Value, SpanTree)>`.

use crate::de::RequireIndent;
use crate::error::{Error, Result};
use crate::parser::events::Event;
use crate::prelude::*;
#[cfg(feature = "std")]
use crate::span_context::SpanTree;
use crate::value::{Mapping, Number, Tag, TaggedValue, Value};
use indexmap::IndexMap;

/// Maximum number of bytes an expanded alias can account for per document.
/// Prevents billion-laughs style attacks.
const MAX_ALIAS_BYTES: usize = 1024 * 1024 * 32; // 32 MB

/// Overhead in bytes accounted for each node in a mapping or sequence.
const NODE_OVERHEAD: usize = 32;

/// The YAML merge key (`<<`).
const MERGE_KEY: &str = "<<";

/// Configuration for the internal parser, mirroring `ParserConfig`.
#[derive(Debug, Clone)]
pub struct ParseConfig {
    pub max_depth: usize,
    pub max_document_length: usize,
    pub max_alias_expansions: usize,
    pub max_mapping_keys: usize,
    pub max_sequence_length: usize,
    pub max_events: usize,
    pub max_nodes: usize,
    pub max_total_scalar_bytes: usize,
    pub max_documents: usize,
    pub max_merge_keys: usize,
    pub alias_anchor_ratio: Option<f64>,
    pub require_indent: RequireIndent,
    pub duplicate_key_policy: DuplicateKeyPolicy,
    pub strict_booleans: bool,
    pub legacy_booleans: bool,
    pub merge_key_policy: MergeKeyPolicy,
    pub no_schema: bool,
    pub legacy_octal_numbers: bool,
    pub legacy_sexagesimal: bool,
    #[cfg(feature = "lossless-u64")]
    pub lossless_u64_integers: bool,
    pub policies: Vec<Arc<dyn crate::policy::Policy>>,
}

impl Default for ParseConfig {
    fn default() -> Self {
        ParseConfig {
            max_depth: 128,
            max_document_length: 1024 * 1024 * 64, // 64 MB
            max_alias_expansions: 1024,
            max_mapping_keys: 1024 * 64,
            max_sequence_length: 1024 * 64,
            max_events: 1_000_000,
            max_nodes: 250_000,
            max_total_scalar_bytes: 1024 * 1024 * 64,
            max_documents: 1_000,
            max_merge_keys: 10_000,
            alias_anchor_ratio: Some(10.0),
            require_indent: RequireIndent::Unchecked,
            duplicate_key_policy: DuplicateKeyPolicy::default(),
            strict_booleans: false,
            legacy_booleans: false,
            merge_key_policy: MergeKeyPolicy::default(),
            no_schema: false,
            legacy_octal_numbers: false,
            legacy_sexagesimal: false,
            #[cfg(feature = "lossless-u64")]
            lossless_u64_integers: false,
            policies: Vec::new(),
        }
    }
}

impl ParseConfig {
    pub(crate) fn lossless_u64_integers(&self) -> bool {
        #[cfg(feature = "lossless-u64")]
        {
            self.lossless_u64_integers
        }
        #[cfg(not(feature = "lossless-u64"))]
        {
            false
        }
    }
}

impl From<&crate::de::ParserConfig> for ParseConfig {
    fn from(c: &crate::de::ParserConfig) -> Self {
        ParseConfig {
            max_depth: c.max_depth,
            max_document_length: c.max_document_length,
            max_alias_expansions: c.max_alias_expansions,
            max_mapping_keys: c.max_mapping_keys,
            max_sequence_length: c.max_sequence_length,
            max_events: c.max_events,
            max_nodes: c.max_nodes,
            max_total_scalar_bytes: c.max_total_scalar_bytes,
            max_documents: c.max_documents,
            max_merge_keys: c.max_merge_keys,
            alias_anchor_ratio: c.alias_anchor_ratio,
            require_indent: c.require_indent,
            duplicate_key_policy: match c.duplicate_key_policy {
                crate::de::DuplicateKeyPolicy::First => DuplicateKeyPolicy::First,
                crate::de::DuplicateKeyPolicy::Last => DuplicateKeyPolicy::Last,
                crate::de::DuplicateKeyPolicy::Error => DuplicateKeyPolicy::Error,
            },
            strict_booleans: c.strict_booleans,
            legacy_booleans: c.legacy_booleans,
            merge_key_policy: match c.merge_key_policy {
                crate::de::MergeKeyPolicy::Auto => MergeKeyPolicy::Auto,
                crate::de::MergeKeyPolicy::AsOrdinary => MergeKeyPolicy::AsOrdinary,
                crate::de::MergeKeyPolicy::Error => MergeKeyPolicy::Error,
            },
            no_schema: c.no_schema,
            legacy_octal_numbers: c.legacy_octal_numbers,
            legacy_sexagesimal: c.legacy_sexagesimal,
            #[cfg(feature = "lossless-u64")]
            lossless_u64_integers: c.lossless_u64_integers,
            policies: c.policies.clone(),
        }
    }
}

/// Internal mirror of [`crate::MergeKeyPolicy`]; see that type for
/// the full rationale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeKeyPolicy {
    #[default]
    Auto,
    AsOrdinary,
    Error,
}

/// Policy for handling duplicate keys in a YAML mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplicateKeyPolicy {
    /// Use the first occurrence of the key; ignore subsequent ones.
    First,
    /// Use the last occurrence of the key (YAML 1.2 default).
    #[default]
    Last,
    /// Return an error if a duplicate key is encountered.
    Error,
}

/// Walk a stream of events and return a list of YAML documents.
#[cfg(feature = "std")]
pub(crate) fn load(
    parser: &mut crate::parser::events::Parser<'_>,
    config: &ParseConfig,
    input: &str,
) -> Result<Vec<(Value, SpanTree)>> {
    let mut loader = Loader::new(config);
    loop {
        match parser.next_event() {
            Ok(Event::StreamEnd) => {
                loader.process_event(Event::StreamEnd, input)?;
                break;
            }
            Ok(event) => loader.process_event(event, input)?,
            Err(e) => return Err(Error::parse_at(&*e.message, input, e.index)),
        }
    }
    Ok(loader.into_docs())
}

/// Load the first document from a YAML stream.
#[cfg(feature = "std")]
pub(crate) fn load_one(
    parser: &mut crate::parser::events::Parser<'_>,
    config: &ParseConfig,
    input: &str,
) -> Result<(Value, SpanTree)> {
    let docs = load(parser, config, input)?;
    // An empty YAML stream (whitespace, comments, or `---` with no
    // content) is a valid document whose value is `null` per YAML 1.2.
    Ok(docs
        .into_iter()
        .next()
        .unwrap_or((Value::Null, SpanTree::Leaf(0, 0))))
}

/// Stack frame for the tree builder.
#[cfg(feature = "std")]
#[derive(Debug)]
enum Frame {
    Sequence {
        items: Vec<Value>,
        span_items: Vec<SpanTree>,
        start: usize,
        anchor: Option<String>,
        /// Tag carried by the originating `SequenceStart` event,
        /// if any. Wrapped onto the produced [`Value::Sequence`]
        /// at `SequenceEnd` time so non-core tagged sequences
        /// surface as [`Value::Tagged`] on the deserialise return
        /// path. See [`crate::de::Deserializer::preserve_tags`].
        tag: Option<(String, String)>,
    },
    MappingKey {
        map: Mapping,
        span_entries: Vec<((usize, usize), SpanTree)>,
        /// The original typed key of each inserted entry, in insertion
        /// order (parallel to `map`). Retained to tell a genuine YAML
        /// duplicate (`1`/`1`) apart from a distinct-typed collision
        /// (`1`/`"1"`) once both collapse to the same string key.
        typed_keys: Vec<Value>,
        start: usize,
        anchor: Option<String>,
        merge_values: Vec<Value>,
        /// Tag carried by the originating `MappingStart` event,
        /// if any. Wrapped onto the produced [`Value::Mapping`]
        /// at `MappingEnd` time.
        tag: Option<(String, String)>,
    },
    MappingValue {
        map: Mapping,
        span_entries: Vec<((usize, usize), SpanTree)>,
        typed_keys: Vec<Value>,
        key: String,
        /// The typed key that produced `key`, kept for the collision check.
        key_value: Value,
        key_span: (usize, usize),
        start: usize,
        anchor: Option<String>,
        merge_values: Vec<Value>,
        tag: Option<(String, String)>,
    },
}

/// YAML tree builder with security limits and span tracking.
#[cfg(feature = "std")]
struct Loader<'a> {
    docs: Vec<(Value, SpanTree)>,
    stack: Vec<Frame>,
    anchor_map: IndexMap<String, (Value, SpanTree)>,
    alias_count: usize,
    alias_bytes: usize,
    config: &'a ParseConfig,
    depth: usize,
    in_document: bool,
    /// Total parser events seen (for `max_events`).
    event_count: usize,
    /// Cumulative scalar bytes seen (for `max_total_scalar_bytes`).
    scalar_bytes: usize,
    /// Anchor count (for `alias_anchor_ratio` denominator).
    anchor_count: usize,
    /// Merge-key occurrences (for `max_merge_keys`).
    merge_key_count: usize,
}

#[cfg(feature = "std")]
impl<'a> Loader<'a> {
    fn new(config: &'a ParseConfig) -> Self {
        // Pre-size the loader's mutable buffers with conservative
        // capacity hints so the typical YAML document parses
        // without reallocating any of these vectors. Numbers are
        // empirical from the v0.0.1 benchmark suite — a 100 KB
        // mapping-of-records document fits inside `stack=16` and
        // `anchor_map=4`. Larger documents fall back to growth.
        Loader {
            docs: Vec::with_capacity(1),
            stack: Vec::with_capacity(16),
            anchor_map: IndexMap::with_capacity(4),
            alias_count: 0,
            alias_bytes: 0,
            config,
            depth: 0,
            in_document: false,
            event_count: 0,
            scalar_bytes: 0,
            anchor_count: 0,
            merge_key_count: 0,
        }
    }

    fn into_docs(self) -> Vec<(Value, SpanTree)> {
        self.docs
    }

    fn process_event(&mut self, event: Event<'_>, input: &str) -> Result<()> {
        if !self.config.policies.is_empty() {
            run_event_policies(&event, &self.config.policies)?;
        }
        // ── Budget: total events ─────────────────────────────────
        self.event_count += 1;
        if self.event_count > self.config.max_events {
            return Err(Error::Budget(crate::BudgetBreach::MaxEvents {
                limit: self.config.max_events,
                observed: self.event_count,
            }));
        }
        // ── Budget: cumulative scalar bytes (per-Scalar event) ──
        if let Event::Scalar { value, .. } = &event {
            self.scalar_bytes = self.scalar_bytes.saturating_add(value.len());
            if self.scalar_bytes > self.config.max_total_scalar_bytes {
                return Err(Error::Budget(crate::BudgetBreach::MaxTotalScalarBytes {
                    limit: self.config.max_total_scalar_bytes,
                    observed: self.scalar_bytes,
                }));
            }
        }
        // ── Budget: anchor / alias counters ─────────────────────
        if let Event::Scalar {
            anchor: Some(_), ..
        }
        | Event::SequenceStart {
            anchor: Some(_), ..
        }
        | Event::MappingStart {
            anchor: Some(_), ..
        } = &event
        {
            self.anchor_count = self.anchor_count.saturating_add(1);
        }
        match event {
            Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart => {
                self.in_document = true;
                self.anchor_map.clear();
                self.alias_count = 0;
                self.alias_bytes = 0;
                // Budget: max_documents
                if self.docs.len() + 1 > self.config.max_documents {
                    return Err(Error::Budget(crate::BudgetBreach::MaxDocuments {
                        limit: self.config.max_documents,
                        observed: self.docs.len() + 1,
                    }));
                }
            }
            Event::DocumentEnd => {
                self.in_document = false;
            }
            Event::Alias { anchor, span } => {
                if !self.in_document {
                    return Err(Error::parse_at("alias outside document", input, span.start));
                }
                self.alias_count += 1;
                if self.alias_count > self.config.max_alias_expansions {
                    return Err(Error::RepetitionLimitExceeded);
                }
                // Budget: alias_anchor_ratio heuristic.
                // Trips when aliases vastly outnumber anchors —
                // a billion-laughs amplification fingerprint.
                if let Some(ratio) = self.config.alias_anchor_ratio {
                    let anchors = self.anchor_count.max(1) as f64;
                    if (self.alias_count as f64) > ratio * anchors {
                        return Err(Error::Budget(crate::BudgetBreach::AliasAnchorRatio {
                            ratio,
                            anchors: self.anchor_count,
                            aliases: self.alias_count,
                        }));
                    }
                }

                let (value, span_tree) =
                    self.anchor_map.get(&anchor).cloned().ok_or_else(|| {
                        Error::UnknownAnchorAt {
                            name: anchor.clone(),
                            location: crate::error::Location::from_index(input, span.start),
                            suggestion: None,
                        }
                    })?;

                self.alias_bytes += estimate_value_size(&value);
                // Bound cumulative alias expansion by the document length
                // limit — a classic billion-laughs vector amplifies well
                // beyond the raw input size.
                if self.alias_bytes > self.config.max_document_length
                    || self.alias_bytes > MAX_ALIAS_BYTES
                {
                    return Err(Error::RepetitionLimitExceeded);
                }

                self.push_node(value, span_tree, input)?;
            }
            Event::Scalar {
                value,
                style,
                anchor,
                tag,
                span,
            } => {
                // An empty plain scalar with no anchor or tag is the
                // implicit null the parser synthesizes for an absent
                // block-mapping value or empty sequence item. The span it
                // was handed is the `:` / `-` indicator it followed, not the
                // value's own bytes, so mark the leaf zero-width: the node
                // has no source of its own and `span_at` should report None.
                // Quoted empties carry a non-Plain style and `~` / `null` a
                // non-empty value, so both keep their real spans.
                let is_implicit_empty = value.is_empty()
                    && matches!(style, crate::parser::ScalarStyle::Plain)
                    && anchor.is_none()
                    && tag.is_none();
                let v = if let Some(t) = tag {
                    resolve_tagged_scalar(&t.0, &t.1, &value, self.config.lossless_u64_integers())?
                } else if style != crate::parser::ScalarStyle::Plain {
                    // Quoted/literal/folded scalars always resolve as
                    // strings — YAML schema resolution only applies to
                    // plain scalars.
                    Value::String(value.into_owned())
                } else {
                    match crate::streaming::resolve_plain_ext(
                        &value,
                        self.config.strict_booleans,
                        self.config.legacy_booleans,
                        self.config.no_schema,
                        self.config.legacy_octal_numbers,
                        self.config.legacy_sexagesimal,
                        self.config.lossless_u64_integers(),
                    ) {
                        crate::streaming::Scalar::Null => Value::Null,
                        crate::streaming::Scalar::Bool(b) => Value::Bool(b),
                        crate::streaming::Scalar::Int(i) => Value::Number(Number::Integer(i)),
                        #[cfg(feature = "lossless-u64")]
                        crate::streaming::Scalar::Uint(u) => Value::Number(Number::Unsigned(u)),
                        crate::streaming::Scalar::Float(f) => Value::Number(Number::Float(f)),
                        crate::streaming::Scalar::Str(s) => Value::String(s.into_owned()),
                    }
                };

                let st = if is_implicit_empty {
                    SpanTree::Leaf(span.start, span.start)
                } else {
                    SpanTree::Leaf(span.start, span.end)
                };
                if let Some(name) = anchor {
                    let _ = self.anchor_map.insert(name, (v.clone(), st.clone()));
                }
                self.push_node(v, st, input)?;
            }
            Event::SequenceStart {
                anchor, tag, span, ..
            } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(Frame::Sequence {
                    items: Vec::new(),
                    span_items: Vec::new(),
                    start: span.start,
                    anchor,
                    tag,
                });
            }
            Event::SequenceEnd { span } => {
                self.depth = self.depth.saturating_sub(1);
                if let Some(Frame::Sequence {
                    items,
                    span_items,
                    start,
                    anchor,
                    tag,
                }) = self.stack.pop()
                {
                    let inner = Value::Sequence(items);
                    let v = wrap_with_tag(inner, tag.as_ref());
                    let st = SpanTree::Sequence {
                        start,
                        end: span.end,
                        items: span_items,
                    };
                    if let Some(name) = anchor {
                        let _ = self.anchor_map.insert(name, (v.clone(), st.clone()));
                    }
                    self.push_node(v, st, input)?;
                } else {
                    return Err(Error::parse_at(
                        "unexpected sequence end",
                        input,
                        span.start,
                    ));
                }
            }
            Event::MappingStart {
                anchor, tag, span, ..
            } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(Frame::MappingKey {
                    map: Mapping::new(),
                    span_entries: Vec::new(),
                    typed_keys: Vec::new(),
                    start: span.start,
                    anchor,
                    merge_values: Vec::new(),
                    tag,
                });
            }
            Event::MappingEnd { span } => {
                self.depth = self.depth.saturating_sub(1);
                if let Some(Frame::MappingKey {
                    mut map,
                    span_entries,
                    typed_keys: _,
                    start,
                    anchor,
                    merge_values,
                    tag,
                }) = self.stack.pop()
                {
                    for mv in merge_values {
                        apply_merge(&mut map, mv)?;
                    }

                    let inner = Value::Mapping(map);
                    let v = wrap_with_tag(inner, tag.as_ref());
                    let st = SpanTree::Mapping {
                        start,
                        end: span.end,
                        entries: span_entries,
                    };
                    if let Some(name) = anchor {
                        let _ = self.anchor_map.insert(name, (v.clone(), st.clone()));
                    }
                    self.push_node(v, st, input)?;
                } else {
                    return Err(Error::parse_at("unexpected mapping end", input, span.start));
                }
            }
        }
        Ok(())
    }

    #[inline]
    fn push_node(&mut self, value: Value, span: SpanTree, input: &str) -> Result<()> {
        if self.stack.is_empty() {
            self.docs.push((value, span));
            return Ok(());
        }

        match self.stack.last_mut().unwrap() {
            Frame::Sequence {
                items, span_items, ..
            } => {
                if items.len() >= self.config.max_sequence_length {
                    return Err(Error::Serialize(
                        "sequence length limit exceeded".to_owned(),
                    ));
                }
                items.push(value);
                span_items.push(span);
            }
            Frame::MappingKey {
                map,
                span_entries,
                typed_keys,
                start,
                anchor,
                merge_values,
                tag,
            } => {
                // Retain the typed key before it is coerced to a string, so
                // the insert site can tell a genuine duplicate apart from a
                // distinct-typed collision. The clone is off the hot path
                // (keys only) and only used for the equality check.
                let key_value = value.clone();
                // Coerce scalar keys to strings; complex keys (sequences,
                // mappings) are stringified via their YAML serialization
                // so the final `Mapping<String, Value>` can hold them.
                let key_str = match value_to_key_string(value) {
                    Some(k) => k,
                    None => {
                        return Err(Error::parse_at(
                            "mapping key must be a scalar or representable as string",
                            input,
                            0,
                        ));
                    }
                };
                let key_span = if let SpanTree::Leaf(s, e) = span {
                    (s, e)
                } else {
                    (0, 0)
                };
                let old_map = core::mem::take(map);
                let old_span_entries = core::mem::take(span_entries);
                let old_typed_keys = core::mem::take(typed_keys);
                let old_start = *start;
                let old_anchor = anchor.take();
                let old_merge_values = core::mem::take(merge_values);
                let old_tag = tag.take();

                *self.stack.last_mut().unwrap() = Frame::MappingValue {
                    map: old_map,
                    span_entries: old_span_entries,
                    typed_keys: old_typed_keys,
                    key: key_str,
                    key_value,
                    key_span,
                    start: old_start,
                    anchor: old_anchor,
                    merge_values: old_merge_values,
                    tag: old_tag,
                };
            }
            Frame::MappingValue {
                map,
                span_entries,
                typed_keys,
                key,
                key_value,
                key_span,
                start,
                anchor,
                merge_values,
                tag,
            } => {
                let is_merge = key == MERGE_KEY;
                let merge_treat_as_ordinary =
                    matches!(self.config.merge_key_policy, MergeKeyPolicy::AsOrdinary);
                let merge_reject = matches!(self.config.merge_key_policy, MergeKeyPolicy::Error);
                if is_merge && merge_reject {
                    return Err(Error::Custom(
                        "merge key `<<` rejected by MergeKeyPolicy::Error".to_owned(),
                    ));
                }
                if is_merge && !merge_treat_as_ordinary {
                    self.merge_key_count = self.merge_key_count.saturating_add(1);
                    if self.merge_key_count > self.config.max_merge_keys {
                        return Err(Error::Budget(crate::BudgetBreach::MaxMergeKeys {
                            limit: self.config.max_merge_keys,
                            observed: self.merge_key_count,
                        }));
                    }
                    merge_values.push(value);
                } else {
                    if map.len() >= self.config.max_mapping_keys {
                        return Err(Error::Serialize("mapping key limit exceeded".to_owned()));
                    }
                    // Steal the owned key out of the frame instead of
                    // cloning it on every insert — the frame is replaced
                    // (without `key`) immediately below, so the emptied
                    // slot is discarded. Each branch is the last use of
                    // `key`, so the move is sound.
                    let key = core::mem::take(key);
                    let key_value = core::mem::take(key_value);
                    // Distinct-typed collision: the string key already exists
                    // but was produced by a *different* typed key (e.g. `1`
                    // then `"1"`, or `true` then `"true"`). Collapsing them
                    // would silently drop an entry, so refuse regardless of
                    // DuplicateKeyPolicy. A genuine duplicate (same typed key)
                    // falls through to the policy below.
                    if let Some(idx) = map.get_index_of(&key) {
                        // Nested (not a `let`-chain) to keep the crate's
                        // 1.85 MSRV: `let`-chains stabilized in 1.88.
                        if typed_keys[idx] != key_value {
                            return Err(Error::KeyCollision(key));
                        }
                    }
                    match self.config.duplicate_key_policy {
                        DuplicateKeyPolicy::First => {
                            if !map.contains_key(&key) {
                                let _ = map.insert(key, value);
                                span_entries.push((*key_span, span));
                                typed_keys.push(key_value);
                            }
                        }
                        DuplicateKeyPolicy::Last => {
                            // `IndexMap::insert` keeps a re-inserted key at
                            // its original index, so the parallel span entry
                            // must be replaced in place: pushing a second
                            // entry would leave the stale first-occurrence
                            // span selected for this key and shift the
                            // span pairing of every key that follows the
                            // duplicate by one.
                            if let Some(idx) = map.get_index_of(&key) {
                                let _ = map.insert(key, value);
                                span_entries[idx] = (*key_span, span);
                                // `typed_keys[idx]` is unchanged: a genuine
                                // duplicate carries the same typed key.
                            } else {
                                let _ = map.insert(key, value);
                                span_entries.push((*key_span, span));
                                typed_keys.push(key_value);
                            }
                        }
                        DuplicateKeyPolicy::Error => {
                            if map.contains_key(&key) {
                                return Err(Error::DuplicateKey(key));
                            }
                            let _ = map.insert(key, value);
                            span_entries.push((*key_span, span));
                            typed_keys.push(key_value);
                        }
                    }
                }

                let old_map = core::mem::take(map);
                let old_span_entries = core::mem::take(span_entries);
                let old_typed_keys = core::mem::take(typed_keys);
                let old_start = *start;
                let old_anchor = anchor.take();
                let old_merge_values = core::mem::take(merge_values);
                let old_tag = tag.take();

                *self.stack.last_mut().unwrap() = Frame::MappingKey {
                    map: old_map,
                    span_entries: old_span_entries,
                    typed_keys: old_typed_keys,
                    start: old_start,
                    anchor: old_anchor,
                    merge_values: old_merge_values,
                    tag: old_tag,
                };
            }
        }
        Ok(())
    }
}

fn apply_merge(map: &mut Mapping, merge_value: Value) -> Result<()> {
    match merge_value {
        Value::Mapping(m) => {
            for (k, v) in m {
                if !map.contains_key(&k) {
                    let _ = map.insert(k, v);
                }
            }
        }
        Value::Sequence(s) => {
            for v in s {
                apply_merge(map, v)?;
            }
        }
        Value::Null => {}
        _ => return Err(Error::ScalarInMergeElement),
    }
    Ok(())
}

fn estimate_value_size(v: &Value) -> usize {
    match v {
        Value::Null | Value::Bool(_) | Value::Number(_) => NODE_OVERHEAD,
        Value::String(s) => NODE_OVERHEAD + s.len(),
        Value::Sequence(s) => NODE_OVERHEAD + s.iter().map(estimate_value_size).sum::<usize>(),
        Value::Mapping(m) => {
            NODE_OVERHEAD
                + m.iter()
                    .map(|(k, v)| k.len() + estimate_value_size(v))
                    .sum::<usize>()
        }
        Value::Tagged(tagged) => NODE_OVERHEAD + estimate_value_size(tagged.value()),
    }
}

// ── Span-free loader (no_std path) ──────────────────────────────────────
//
// Only compiled when the `std` feature is disabled. The `std` build
// always uses the span-aware loader above so `Spanned<T>` fields are
// populated correctly.

/// Skip-span loader entry point: parse one document into `Value`
/// without building a `SpanTree`. Available on every target —
/// `std` builds use this from the [`from_str::<Value>`] fast path
/// (`Value::deserialize` never consults the span context, so
/// building one is pure waste). `no_std` builds use it
/// exclusively.
pub(crate) fn load_one_no_spans(input: &str, config: &ParseConfig) -> Result<Value> {
    Ok(load_all_no_spans(input, config)?
        .into_iter()
        .next()
        .unwrap_or(Value::Null))
}

/// Skip-span loader entry point: parse all documents into
/// `Value`s without building `SpanTree`s. See [`load_one_no_spans`].
pub(crate) fn load_all_no_spans(input: &str, config: &ParseConfig) -> Result<Vec<Value>> {
    let mut parser = crate::parser::events::Parser::new(input);
    let mut loader = NoSpanLoader::new(config);
    loop {
        match parser.next_event() {
            Ok(Event::StreamEnd) => {
                loader.process_event(Event::StreamEnd, input)?;
                break;
            }
            Ok(event) => loader.process_event(event, input)?,
            Err(e) => return Err(Error::parse_at(&*e.message, input, e.index)),
        }
    }
    Ok(loader.docs)
}

#[derive(Debug)]
enum NoSpanFrame {
    Sequence {
        items: Vec<Value>,
        anchor: Option<String>,
        tag: Option<(String, String)>,
    },
    MappingKey {
        map: Mapping,
        anchor: Option<String>,
        merge_values: Vec<Value>,
        tag: Option<(String, String)>,
    },
    MappingValue {
        map: Mapping,
        key: String,
        anchor: Option<String>,
        merge_values: Vec<Value>,
        tag: Option<(String, String)>,
    },
}

struct NoSpanLoader<'a> {
    docs: Vec<Value>,
    stack: Vec<NoSpanFrame>,
    anchor_map: IndexMap<String, Value>,
    alias_count: usize,
    alias_bytes: usize,
    config: &'a ParseConfig,
    depth: usize,
    in_document: bool,
}

impl<'a> NoSpanLoader<'a> {
    fn new(config: &'a ParseConfig) -> Self {
        NoSpanLoader {
            docs: Vec::new(),
            stack: Vec::new(),
            anchor_map: IndexMap::new(),
            alias_count: 0,
            alias_bytes: 0,
            config,
            depth: 0,
            in_document: false,
        }
    }

    #[allow(dead_code)] // load_all_no_spans drains `self.docs` directly today.
    fn into_docs(self) -> Vec<Value> {
        self.docs
    }

    fn process_event(&mut self, event: Event<'_>, input: &str) -> Result<()> {
        if !self.config.policies.is_empty() {
            run_event_policies(&event, &self.config.policies)?;
        }
        match event {
            Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart => {
                self.in_document = true;
                self.anchor_map.clear();
            }
            Event::DocumentEnd => {
                self.in_document = false;
            }
            Event::Alias { anchor, span } => {
                self.alias_count += 1;
                if self.alias_count > self.config.max_alias_expansions {
                    return Err(Error::RepetitionLimitExceeded);
                }
                let value = self.anchor_map.get(&anchor).cloned().ok_or_else(|| {
                    Error::UnknownAnchorAt {
                        name: anchor,
                        location: crate::error::Location::from_index(input, span.start),
                        suggestion: None,
                    }
                })?;
                self.alias_bytes += estimate_value_size(&value);
                if self.alias_bytes > MAX_ALIAS_BYTES {
                    return Err(Error::RepetitionLimitExceeded);
                }
                self.push_value(value)?;
            }
            Event::Scalar {
                value,
                style,
                anchor,
                tag,
                ..
            } => {
                let v = if let Some(t) = tag {
                    resolve_tagged_scalar(&t.0, &t.1, &value, self.config.lossless_u64_integers())?
                } else if style != crate::parser::ScalarStyle::Plain {
                    Value::String(value.into_owned())
                } else {
                    match crate::streaming::resolve_plain_ext(
                        &value,
                        self.config.strict_booleans,
                        self.config.legacy_booleans,
                        self.config.no_schema,
                        self.config.legacy_octal_numbers,
                        self.config.legacy_sexagesimal,
                        self.config.lossless_u64_integers(),
                    ) {
                        crate::streaming::Scalar::Null => Value::Null,
                        crate::streaming::Scalar::Bool(b) => Value::Bool(b),
                        crate::streaming::Scalar::Int(i) => Value::Number(Number::Integer(i)),
                        #[cfg(feature = "lossless-u64")]
                        crate::streaming::Scalar::Uint(u) => Value::Number(Number::Unsigned(u)),
                        crate::streaming::Scalar::Float(f) => Value::Number(Number::Float(f)),
                        crate::streaming::Scalar::Str(s) => Value::String(s.into_owned()),
                    }
                };
                if let Some(name) = anchor {
                    let _ = self.anchor_map.insert(name, v.clone());
                }
                self.push_value(v)?;
            }
            Event::SequenceStart { anchor, tag, .. } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(NoSpanFrame::Sequence {
                    items: Vec::new(),
                    anchor,
                    tag,
                });
            }
            Event::SequenceEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                if let Some(NoSpanFrame::Sequence { items, anchor, tag }) = self.stack.pop() {
                    let inner = Value::Sequence(items);
                    let v = wrap_with_tag(inner, tag.as_ref());
                    if let Some(name) = anchor {
                        let _ = self.anchor_map.insert(name, v.clone());
                    }
                    self.push_value(v)?;
                }
            }
            Event::MappingStart { anchor, tag, .. } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                self.stack.push(NoSpanFrame::MappingKey {
                    map: Mapping::new(),
                    anchor,
                    merge_values: Vec::new(),
                    tag,
                });
            }
            Event::MappingEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                if let Some(NoSpanFrame::MappingKey {
                    mut map,
                    anchor,
                    merge_values,
                    tag,
                }) = self.stack.pop()
                {
                    for mv in merge_values {
                        apply_merge(&mut map, mv)?;
                    }
                    let inner = Value::Mapping(map);
                    let v = wrap_with_tag(inner, tag.as_ref());
                    if let Some(name) = anchor {
                        let _ = self.anchor_map.insert(name, v.clone());
                    }
                    self.push_value(v)?;
                }
            }
        }
        Ok(())
    }

    fn push_value(&mut self, value: Value) -> Result<()> {
        if self.stack.is_empty() {
            self.docs.push(value);
            return Ok(());
        }
        match self.stack.last_mut().unwrap() {
            NoSpanFrame::Sequence { items, .. } => {
                items.push(value);
            }
            NoSpanFrame::MappingKey {
                map,
                anchor,
                merge_values,
                tag,
            } => {
                if let Some(key) = value_to_key_string(value) {
                    let old_map = core::mem::take(map);
                    let old_anchor = anchor.take();
                    let old_merge_values = core::mem::take(merge_values);
                    let old_tag = tag.take();
                    *self.stack.last_mut().unwrap() = NoSpanFrame::MappingValue {
                        map: old_map,
                        key,
                        anchor: old_anchor,
                        merge_values: old_merge_values,
                        tag: old_tag,
                    };
                }
            }
            NoSpanFrame::MappingValue {
                map,
                key,
                anchor,
                merge_values,
                tag,
            } => {
                let is_merge = key == MERGE_KEY;
                let merge_treat_as_ordinary =
                    matches!(self.config.merge_key_policy, MergeKeyPolicy::AsOrdinary);
                let merge_reject = matches!(self.config.merge_key_policy, MergeKeyPolicy::Error);
                if is_merge && merge_reject {
                    return Err(Error::Custom(
                        "merge key `<<` rejected by MergeKeyPolicy::Error".to_owned(),
                    ));
                }
                if is_merge && !merge_treat_as_ordinary {
                    merge_values.push(value);
                } else {
                    // Steal the owned key out of the frame instead of
                    // cloning it — the frame is overwritten (without
                    // `key`) immediately below, so the emptied slot is
                    // discarded.
                    let _ = map.insert(core::mem::take(key), value);
                }
                let old_map = core::mem::take(map);
                let old_anchor = anchor.take();
                let old_merge_values = core::mem::take(merge_values);
                let old_tag = tag.take();
                *self.stack.last_mut().unwrap() = NoSpanFrame::MappingKey {
                    map: old_map,
                    anchor: old_anchor,
                    merge_values: old_merge_values,
                    tag: old_tag,
                };
            }
        }
        Ok(())
    }
}

/// Coerce a `Value` into a mapping-key string. Scalars stringify naturally;
/// sequences and mappings use a deterministic inline YAML-like representation
/// so the parser can still build a `Mapping<String, Value>` from YAML with
/// complex keys (common in the official YAML Test Suite).
fn value_to_key_string(value: Value) -> Option<String> {
    use core::fmt::Write as _;
    match value {
        Value::String(s) => Some(s),
        Value::Bool(b) => Some(if b { "true".into() } else { "false".into() }),
        Value::Null => Some("null".into()),
        Value::Number(Number::Integer(n)) => {
            #[cfg(feature = "fast-int")]
            {
                Some(itoa::Buffer::new().format(n).to_owned())
            }
            #[cfg(not(feature = "fast-int"))]
            {
                Some(n.to_string())
            }
        }
        #[cfg(feature = "lossless-u64")]
        Value::Number(Number::Unsigned(n)) => {
            #[cfg(feature = "fast-int")]
            {
                Some(itoa::Buffer::new().format(n).to_owned())
            }
            #[cfg(not(feature = "fast-int"))]
            {
                Some(n.to_string())
            }
        }
        Value::Number(Number::Float(n)) => {
            #[cfg(feature = "fast-float")]
            {
                Some(ryu::Buffer::new().format(n).to_owned())
            }
            #[cfg(not(feature = "fast-float"))]
            {
                // `{:?}` keeps `1.0` printable as `1.0` (not `1`)
                // so the resulting key string is unambiguously a
                // float on round-trip.
                Some(format!("{n:?}"))
            }
        }
        Value::Tagged(t) => value_to_key_string(t.value().clone()),
        Value::Sequence(seq) => {
            let mut s = String::from("[");
            for (i, v) in seq.into_iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                let _ = write!(s, "{}", value_to_key_string(v).unwrap_or_default());
            }
            s.push(']');
            Some(s)
        }
        Value::Mapping(m) => {
            let mut s = String::from("{");
            for (i, (k, v)) in m.into_iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&k);
                s.push_str(": ");
                let _ = write!(s, "{}", value_to_key_string(v).unwrap_or_default());
            }
            s.push('}');
            Some(s)
        }
    }
}

/// Wrap `inner` (a Sequence or Mapping `Value`) in
/// [`Value::Tagged`] when the originating event carried a custom
/// (non-core) tag. Core YAML 1.2 tags (`!!seq`, `!!map`) are
/// stripped — they are no-ops on a sequence/mapping anyway and
/// `!!seq` / `!!map` would otherwise leak into the deserialise
/// return path as redundant metadata.
fn wrap_with_tag(inner: Value, tag: Option<&(String, String)>) -> Value {
    let Some((handle, suffix)) = tag else {
        return inner;
    };
    // `!!seq` / `!!map` (and the explicit URI form) are
    // pure-metadata core tags on collections; the `Sequence` or
    // `Mapping` variant of `Value` already conveys "seq" /
    // "map" — wrapping in `Tagged` would only confuse downstream
    // matches that key on the variant.
    let is_core_collection =
        (handle == "!!" || handle == "tag:yaml.org,2002:") && (suffix == "seq" || suffix == "map");
    if is_core_collection {
        return inner;
    }
    Value::Tagged(Box::new(TaggedValue::new(
        Tag::new(concat_str(handle, suffix)),
        inner,
    )))
}

/// Concatenate two `&str` into a fresh `String` with exactly the
/// right capacity. Skips the `format!`/`fmt::Arguments` machinery
/// that allocates an intermediate buffer; on a tagged-scalar-heavy
/// document this is hit once per scalar.
#[inline]
fn concat_str(a: &str, b: &str) -> String {
    let mut s = String::with_capacity(a.len() + b.len());
    s.push_str(a);
    s.push_str(b);
    s
}

/// Resolve a tagged scalar into a typed `Value`. Handles the YAML 1.2
/// core schema tags (`!!int`, `!!float`, `!!bool`, `!!null`, `!!str`)
/// and any custom tag falls through to the `Tagged` wrapper.
fn resolve_tagged_scalar(
    handle: &str,
    suffix: &str,
    value: &str,
    lossless_u64: bool,
) -> Result<Value> {
    // Canonicalize tag: handle `!!foo` (secondary) → `tag:yaml.org,2002:foo`.
    let is_core = handle == "!!"
        || handle == "tag:yaml.org,2002:"
        || (handle == "!" && matches!(suffix, "int" | "float" | "bool" | "null" | "str"));
    if is_core {
        match suffix {
            "int" => {
                // Accept decimal, hex (0x), octal (0o), with optional sign.
                let trimmed = value.trim();
                let parsed = if let Some(rest) = trimmed
                    .strip_prefix("0x")
                    .or_else(|| trimmed.strip_prefix("0X"))
                {
                    parse_tagged_integer(rest, 16, lossless_u64)
                } else if let Some(rest) = trimmed
                    .strip_prefix("0o")
                    .or_else(|| trimmed.strip_prefix("0O"))
                {
                    parse_tagged_integer(rest, 8, lossless_u64)
                } else {
                    parse_tagged_decimal_integer(trimmed, lossless_u64)
                };
                parsed.ok_or_else(|| Error::FailedToParseNumber(format!("!!int {value}")))
            }
            "float" => {
                let trimmed = value.trim();
                match trimmed {
                    ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
                        Ok(Value::Number(Number::Float(f64::INFINITY)))
                    }
                    "-.inf" | "-.Inf" | "-.INF" => {
                        Ok(Value::Number(Number::Float(f64::NEG_INFINITY)))
                    }
                    ".nan" | ".NaN" | ".NAN" => Ok(Value::Number(Number::Float(f64::NAN))),
                    _ => trimmed
                        .parse::<f64>()
                        .map(|f| Value::Number(Number::Float(f)))
                        .map_err(|_| Error::FailedToParseNumber(format!("!!float {value}"))),
                }
            }
            "bool" => match value.trim() {
                "true" | "True" | "TRUE" => Ok(Value::Bool(true)),
                "false" | "False" | "FALSE" => Ok(Value::Bool(false)),
                _ => Err(Error::FailedToParseNumber(format!("!!bool {value}"))),
            },
            "null" => match value.trim() {
                "" | "~" | "null" | "Null" | "NULL" => Ok(Value::Null),
                _ => Err(Error::FailedToParseNumber(format!("!!null {value}"))),
            },
            "str" => Ok(Value::String(value.to_owned())),
            _ => Ok(Value::Tagged(Box::new(TaggedValue::new(
                Tag::new(concat_str(handle, suffix)),
                Value::String(value.to_owned()),
            )))),
        }
    } else {
        Ok(Value::Tagged(Box::new(TaggedValue::new(
            Tag::new(concat_str(handle, suffix)),
            Value::String(value.to_owned()),
        ))))
    }
}

fn parse_tagged_decimal_integer(trimmed: &str, lossless_u64: bool) -> Option<Value> {
    #[cfg(not(feature = "lossless-u64"))]
    let _ = lossless_u64;
    if let Ok(n) = trimmed.parse::<i64>() {
        return Some(Value::Number(Number::Integer(n)));
    }
    #[cfg(feature = "lossless-u64")]
    if lossless_u64 {
        return trimmed
            .parse::<u64>()
            .ok()
            .map(|n| Value::Number(Number::Unsigned(n)));
    }
    #[allow(unreachable_code)]
    None
}

fn parse_tagged_integer(rest: &str, radix: u32, lossless_u64: bool) -> Option<Value> {
    #[cfg(not(feature = "lossless-u64"))]
    let _ = lossless_u64;
    if let Ok(n) = i64::from_str_radix(rest, radix) {
        return Some(Value::Number(Number::Integer(n)));
    }
    #[cfg(feature = "lossless-u64")]
    if lossless_u64 {
        return u64::from_str_radix(rest, radix)
            .ok()
            .map(|n| Value::Number(Number::Unsigned(n)));
    }
    #[allow(unreachable_code)]
    None
}

/// Run every registered policy against this parser event. The
/// loader calls this on each event before further processing; the
/// first policy to reject aborts the parse.
fn run_event_policies(
    event: &Event<'_>,
    policies: &[Arc<dyn crate::policy::Policy>],
) -> Result<()> {
    use crate::policy::{PolicyEvent, PolicyEventKind};
    let (kind, anchor, tag, scalar) = match event {
        Event::Scalar {
            value, anchor, tag, ..
        } => {
            let tag_str = tag.as_ref().map(|(h, s)| format!("{h}{s}"));
            (
                Some(PolicyEventKind::Scalar),
                anchor.as_deref(),
                tag_str,
                Some(value.as_ref()),
            )
        }
        Event::SequenceStart { anchor, tag, .. } => {
            let tag_str = tag.as_ref().map(|(h, s)| format!("{h}{s}"));
            (
                Some(PolicyEventKind::SequenceStart),
                anchor.as_deref(),
                tag_str,
                None,
            )
        }
        Event::MappingStart { anchor, tag, .. } => {
            let tag_str = tag.as_ref().map(|(h, s)| format!("{h}{s}"));
            (
                Some(PolicyEventKind::MappingStart),
                anchor.as_deref(),
                tag_str,
                None,
            )
        }
        Event::Alias { .. } => (
            // `Event::Alias.anchor` carries the *target* anchor name,
            // not a fresh definition — surface this as a pure Alias
            // kind without an `anchor` field so policies can
            // distinguish "this node is anchored" from "this node
            // dereferences an existing anchor".
            Some(PolicyEventKind::Alias),
            None,
            None,
            None,
        ),
        _ => (None, None, None, None),
    };
    if let Some(kind) = kind {
        let projected = PolicyEvent {
            kind,
            anchor,
            tag: tag.as_deref(),
            scalar,
        };
        for p in policies {
            p.check_event(projected)?;
        }
    }
    Ok(())
}
