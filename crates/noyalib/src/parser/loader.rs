// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Event-to-Value tree builder with security limits.
//!
//! Converts a stream of [`Event`]s directly into `Vec<(Value, SpanTree)>`.

use crate::de::RequireIndent;
use crate::error::{Error, Result};
use crate::parser::budget;
use crate::parser::events::Event;
use crate::parser::meter::{AliasCost, Meter};
use crate::prelude::IndexMap;
use crate::prelude::*;
#[cfg(feature = "std")]
use crate::span_context::SpanTree;
use crate::value::{Mapping, Number, Tag, TaggedValue, Value};

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
    pub leading_zero_integer_strings: bool,
    pub legacy_binary_numbers: bool,
    pub float_overflow_strings: bool,
    pub integer_overflow_errors: bool,
    pub non_scalar_key_policy: crate::de::NonScalarKeyPolicy,
    pub alias_jump_event_factor: Option<usize>,
    #[cfg(feature = "lossless-u64")]
    pub lossless_u64_integers: bool,
    /// Mirrors `ParserConfig::plain_scalar_strings`. When `true`, a
    /// plain scalar deserializes into a `String`/`char` target as its
    /// literal text regardless of its resolved schema type.
    pub plain_scalar_strings: bool,
    pub policies: Vec<Arc<dyn crate::policy::Policy>>,
    /// Tags registered for strip-through. When set, the loader resolves a
    /// registered tag's scalar as if untagged (matching the streaming path)
    /// instead of producing a [`Value::Tagged`].
    pub tag_registry: Option<Arc<crate::TagRegistry>>,
    /// Stream-wide budget counters shared with the other documents of
    /// the same stream; see [`crate::parser::meter::StreamTally`].
    pub(crate) stream_tally: Option<Arc<crate::parser::meter::StreamTally>>,
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
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
            leading_zero_integer_strings: false,
            legacy_binary_numbers: false,
            float_overflow_strings: false,
            integer_overflow_errors: false,
            non_scalar_key_policy: crate::de::NonScalarKeyPolicy::Stringify,
            alias_jump_event_factor: None,
            #[cfg(feature = "lossless-u64")]
            lossless_u64_integers: false,
            plain_scalar_strings: false,
            policies: Vec::new(),
            tag_registry: None,
            stream_tally: None,
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
        Self {
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
            leading_zero_integer_strings: c.leading_zero_integer_strings,
            legacy_binary_numbers: c.legacy_binary_numbers,
            float_overflow_strings: c.float_overflow_strings,
            integer_overflow_errors: c.integer_overflow_errors,
            non_scalar_key_policy: c.non_scalar_key_policy,
            alias_jump_event_factor: c.alias_jump_event_factor,
            #[cfg(feature = "lossless-u64")]
            lossless_u64_integers: c.lossless_u64_integers,
            plain_scalar_strings: c.plain_scalar_strings,
            policies: c.policies.clone(),
            tag_registry: c.tag_registry.clone(),
            stream_tally: c.stream_tally.clone(),
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
            Err(e) => return Err(e.into_error(input, config.max_events)),
        }
    }
    Ok(loader.into_docs())
}

/// Load exactly one document and error if the stream carries more than
/// one.
///
/// Used by the single-document deserialisation and CST entry points. A
/// stream carrying more than one document must use `from_str_multi`,
/// `document::load_all`, or `cst::parse_stream` instead. See #351. A
/// single document with a leading `---` or trailing `...` marker is
/// unaffected: it still produces exactly one entry in `docs`.
#[cfg(feature = "std")]
pub(crate) fn load_exactly_one(
    parser: &mut crate::parser::events::Parser<'_>,
    config: &ParseConfig,
    input: &str,
) -> Result<(Value, SpanTree)> {
    let docs = load(parser, config, input)?;
    if docs.len() > 1 {
        return Err(Error::MoreThanOneDocument);
    }
    Ok(docs
        .into_iter()
        .next()
        .unwrap_or((Value::Null, SpanTree::Leaf(0, 0))))
}

/// What the key-collision check needs to remember about a key.
///
/// Introduced in v0.0.14 as `Vec<Value>`, cloning every mapping key so that
/// two keys which stringify identically (`1` and `"1"`) could still be told
/// apart. That clone was one `String` allocation per key on every document —
/// measured at +39% total heap on a frontmatter corpus — and for string keys
/// it recorded nothing the check could use: two string keys that collide on
/// their string form are equal by definition.
///
/// So a string key stores only the fact that it was a string. Anything else
/// keeps the full value, which for scalars is a cheap copy and for compound
/// keys is rare (and rejected outright under `NonScalarKeyPolicy::Error`).
#[derive(Debug, Clone, PartialEq, Default)]
enum KeyShape {
    /// Placeholder left behind by `mem::take`; never compared.
    #[default]
    /// A `Value::String` key. Content deliberately not stored.
    Str,
    /// Any other key, kept whole for the typed comparison.
    ///
    /// Boxed so the enum is pointer-sized rather than `Value`-sized. This
    /// lives in every mapping frame, and the per-parse frame stack takes
    /// its first allocation at four frames: v0.0.14 grew `NoSpanFrame`
    /// from 176 to 280 bytes, pushing that allocation from the 1 KiB size
    /// class to 2 KiB. Boxing the rare arm gives most of that back.
    Other(Box<Value>),
}

/// Sentinel returned by [`KeyShape::at`] for an unmaterialised index.
///
/// A `static` rather than a `const` inside the method: a `const` item in a
/// function body is an outer item, where `Self` is not in scope, and the
/// lint that forbids naming the type inside its own impl applies there.
static KEY_SHAPE_STR: KeyShape = KeyShape::Str;

impl KeyShape {
    fn of(value: &Value) -> Self {
        match value {
            Value::String(_) => Self::Str,
            other => Self::Other(Box::new(other.clone())),
        }
    }

    /// The shape recorded for map index `idx`.
    ///
    /// `typed_keys` is materialised lazily: while every key seen so far is
    /// a string, it stays empty and every index reads as `Str`. That is
    /// what makes the collision check free for the ordinary document — the
    /// eager `Vec` grew through four reallocations per mapping (256 → 512
    /// → 1024 → 2048 bytes) and was the whole of a +39% heap regression
    /// measured against v0.0.13, on documents that never had a non-string
    /// key to record.
    fn at(typed_keys: &[Self], idx: usize) -> &Self {
        typed_keys.get(idx).unwrap_or(&KEY_SHAPE_STR)
    }

    /// Records `shape` as the entry for the key just inserted at `idx`.
    ///
    /// Stays empty for as long as it can. The first non-string key back-
    /// fills `Str` for every earlier index so positions stay parallel to
    /// the map from that point on.
    fn record(typed_keys: &mut Vec<Self>, idx: usize, shape: Self) {
        if typed_keys.is_empty() && shape == Self::Str {
            return;
        }
        if typed_keys.len() < idx {
            typed_keys.resize(idx, Self::Str);
        }
        typed_keys.push(shape);
    }
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
        /// `true` when the start token is `[` — a flow sequence's
        /// end span comes from its `]` token; a block sequence has
        /// no end token, so its span is sealed at its last item
        /// (#375: the SequenceEnd event carries the *next* token's
        /// span for block shapes, which produced indicator-only
        /// spans for value-position sequences).
        flow: bool,
    },
    MappingKey {
        map: Mapping,
        span_entries: Vec<((usize, usize), SpanTree)>,
        /// The original typed key of each inserted entry, in insertion
        /// order (parallel to `map`). Retained to tell a genuine YAML
        /// duplicate (`1`/`1`) apart from a distinct-typed collision
        /// (`1`/`"1"`) once both collapse to the same string key.
        typed_keys: Vec<KeyShape>,
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
        typed_keys: Vec<KeyShape>,
        key: String,
        /// The typed key that produced `key`, kept for the collision check.
        key_value: KeyShape,
        /// Whether `key` is eligible to be read as a `<<` merge key.
        ///
        /// Only a **plain** `<<` scalar is: the YAML merge type gives
        /// `tag:yaml.org,2002:merge` to a plain `<<`, while a quoted `"<<"`
        /// and an alias resolving to the string `<<` both resolve to
        /// `...:str`. `key` is the *stringified* key, identical in every
        /// case, so the distinction has to be carried from where the
        /// scalar's presentation was still known.
        key_may_merge: bool,
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
    /// Anchored nodes, each with the cost of expanding it, measured once
    /// when the anchor closes so an alias is charged before it is copied.
    anchor_map: IndexMap<String, (Value, SpanTree, AliasCost)>,
    /// Source byte-index of each anchor's definition (parity with
    /// the streaming path's `anchor_def_spans`) — powers the
    /// "did you mean …?" affordance on `Error::UnknownAnchorAt`.
    anchor_def_spans: IndexMap<String, usize>,
    /// Anchors defined in earlier documents of the stream (name to
    /// byte index of the last definition). Anchors do not cross `---`
    /// (YAML 1.2.2 §3.2.2.2); when an alias names one of these the
    /// error says so instead of only "unknown anchor".
    earlier_anchor_defs: IndexMap<String, usize>,
    /// Every budget counter, charged in the order all loaders share.
    meter: Meter,
    config: &'a ParseConfig,
    depth: usize,
    in_document: bool,
}

#[cfg(feature = "std")]
impl<'a> Loader<'a> {
    /// The dotted path of the mapping being filled, from the frames that
    /// enclose it: the key of each open mapping and the index of each
    /// open sequence. The innermost frame is the one being filled; its
    /// key has been taken by the caller, which appends it itself.
    fn parent_path(&self) -> Option<String> {
        let enclosing = &self.stack[..self.stack.len().saturating_sub(1)];
        frames_path(enclosing.iter().map(|f| match f {
            Frame::MappingValue { key, .. } => Some(key.clone()),
            Frame::Sequence { items, .. } => Some(items.len().to_string()),
            _ => None,
        }))
    }

    /// The located duplicate-key refusal for `key`, which begins at
    /// `key_start` in `input` (#378).
    fn duplicate_key_at(&self, key: String, key_start: usize, input: &str) -> Error {
        duplicate_key_at(key, self.parent_path(), key_start, input)
    }

    /// The located key-collision refusal for `key` (#378).
    fn key_collision_at(&self, key: String, key_start: usize, input: &str) -> Error {
        key_collision_at(key, self.parent_path(), key_start, input)
    }

    /// Charge and expand `*anchor`, met at `alias_start`.
    ///
    /// Every budget is charged from the anchor's stored cost before the
    /// anchored tree is cloned, so a refused expansion allocates nothing.
    fn resolve_alias(
        &mut self,
        anchor: &str,
        alias_start: usize,
        input: &str,
    ) -> Result<(Value, SpanTree)> {
        if !self.in_document {
            return Err(Error::parse_at(
                "alias outside document",
                input,
                alias_start,
            ));
        }
        self.meter.charge_alias(self.config)?;
        let Some((value, span_tree, cost)) = self.anchor_map.get(anchor) else {
            return Err(missing_anchor(
                anchor,
                alias_start,
                input,
                &self.anchor_def_spans,
                &self.earlier_anchor_defs,
            ));
        };
        self.meter.charge_expansion(cost, self.depth, self.config)?;
        Ok((value.clone(), span_tree.clone()))
    }

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
            anchor_def_spans: IndexMap::with_capacity(4),
            earlier_anchor_defs: IndexMap::new(),
            meter: Meter::new(config),
            config,
            depth: 0,
            in_document: false,
        }
    }

    fn into_docs(self) -> Vec<(Value, SpanTree)> {
        self.docs
    }

    fn process_event(&mut self, event: Event<'_>, input: &str) -> Result<()> {
        self.meter.charge_event(&event, self.config)?;
        match event {
            Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart => {
                self.in_document = true;
                self.anchor_map.clear();
                self.earlier_anchor_defs
                    .extend(self.anchor_def_spans.drain(..));
            }
            Event::DocumentEnd => {
                self.in_document = false;
            }
            Event::Alias { anchor, span } => {
                let (value, span_tree) = self.resolve_alias(&anchor, span.start, input)?;
                // Wrap the anchor's cloned tree so span resolution can tell it
                // reached this value *through* an alias — a read resolves
                // through (issue #149), a write must refuse (would splice the
                // anchor's bytes, a different key).
                // An alias is never a merge key either. The merge tag is
                // resolved from a plain `<<` *scalar*; `<<: *x` where `*x`
                // happens to resolve to the string `\"<<\"` is an ordinary
                // key whose value is that string.
                self.push_node(value, SpanTree::Alias(Box::new(span_tree)), input, false)?;
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
                // Decide merge-key eligibility here, while the scalar's style
                // is still in hand — `resolve_untagged_scalar` returns a
                // `Value::String("<<")` for a plain and a quoted `<<` alike.
                let is_plain_merge_candidate = matches!(style, crate::parser::ScalarStyle::Plain);
                // serde_yaml-profile: a plain decimal integer past
                // `u64::MAX` is refused instead of degrading to an
                // approximate f64.
                if self.config.integer_overflow_errors
                    && is_plain_merge_candidate
                    && tag.is_none()
                    && overflows_u64_decimal(&value)
                {
                    return Err(Error::IntegerOverflow {
                        location: Some(crate::error::Location::from_index(input, span.start)),
                        path: frames_path(self.stack.iter().map(|f| match f {
                            Frame::MappingValue { key, .. } => Some(key.clone()),
                            Frame::Sequence { items, .. } => Some(items.len().to_string()),
                            _ => None,
                        })),
                    });
                }
                let v =
                    if let Some(t) = tag {
                        if self.config.tag_registry.as_ref().is_some_and(|r| {
                            crate::streaming::tag_is_registry_stripped(&t.0, &t.1, r)
                        }) {
                            // Registered tag: strip through and resolve as if
                            // untagged, exactly as the streaming path does.
                            resolve_untagged_scalar(value, style, self.config)
                        } else {
                            resolve_tagged_scalar(
                                &t.0,
                                &t.1,
                                &value,
                                self.config.lossless_u64_integers(),
                            )?
                        }
                    } else {
                        resolve_untagged_scalar(value, style, self.config)
                    };

                let st = if is_implicit_empty {
                    SpanTree::Leaf(span.start, span.start)
                } else {
                    SpanTree::Leaf(span.start, span.end)
                };
                if let Some(name) = anchor {
                    let _ = self.anchor_def_spans.insert(name.clone(), span.start);
                    let cost = AliasCost::of_value(&v);
                    let _ = self.anchor_map.insert(name, (v.clone(), st.clone(), cost));
                }
                self.push_node(v, st, input, is_plain_merge_candidate)?;
            }
            Event::SequenceStart {
                anchor, tag, span, ..
            } => {
                self.depth += 1;
                if budget::depth_exceeded(self.depth, self.config.max_depth) {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                if let Some(name) = anchor.as_ref() {
                    let _ = self.anchor_def_spans.insert(name.clone(), span.start);
                }
                self.stack.push(Frame::Sequence {
                    items: Vec::new(),
                    span_items: Vec::new(),
                    start: span.start,
                    anchor,
                    tag,
                    flow: starts_with_flow_bracket(input, span.start),
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
                    flow,
                }) = self.stack.pop()
                {
                    let inner = Value::Sequence(items);
                    let v = wrap_with_tag(inner, tag.as_ref(), self.config.tag_registry.as_deref());
                    // #375: a block sequence has no end token — the
                    // event's span belongs to whatever token follows
                    // the dedent — so its span is sealed at its last
                    // item. Flow keeps the `]` from the event; an
                    // empty block sequence keeps the event span.
                    let end = if flow {
                        span.end
                    } else {
                        span_items.last().map_or(span.end, span_tree_end).max(start)
                    };
                    let st = SpanTree::Sequence {
                        start,
                        end,
                        items: span_items,
                    };
                    if let Some(name) = anchor {
                        let cost = AliasCost::of_value(&v);
                        let _ = self.anchor_map.insert(name, (v.clone(), st.clone(), cost));
                    }
                    // A sequence or mapping is never the string `<<`.
                    self.push_node(v, st, input, false)?;
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
                if budget::depth_exceeded(self.depth, self.config.max_depth) {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                if let Some(name) = anchor.as_ref() {
                    let _ = self.anchor_def_spans.insert(name.clone(), span.start);
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
                    let v = wrap_with_tag(inner, tag.as_ref(), self.config.tag_registry.as_deref());
                    let st = SpanTree::Mapping {
                        start,
                        end: span.end,
                        entries: span_entries,
                    };
                    if let Some(name) = anchor {
                        let cost = AliasCost::of_value(&v);
                        let _ = self.anchor_map.insert(name, (v.clone(), st.clone(), cost));
                    }
                    // A sequence or mapping is never the string `<<`.
                    self.push_node(v, st, input, false)?;
                } else {
                    return Err(Error::parse_at("unexpected mapping end", input, span.start));
                }
            }
        }
        Ok(())
    }

    #[inline]
    /// Push a completed node into the enclosing frame.
    ///
    /// `may_be_merge_key` says whether this value is *eligible* to be read
    /// as a `<<` merge key. Only a **plain** scalar is: the YAML merge type
    /// gives `tag:yaml.org,2002:merge` to a plain `<<`, while a quoted
    /// `"<<"` resolves to `tag:yaml.org,2002:str` and is an ordinary key.
    /// By the time a scalar reaches here it is a `Value::String("<<")`
    /// either way, so the distinction has to be carried in rather than
    /// re-derived.
    fn push_node(
        &mut self,
        value: Value,
        span: SpanTree,
        input: &str,
        may_be_merge_key: bool,
    ) -> Result<()> {
        if self.stack.is_empty() {
            self.docs.push((value, span));
            return Ok(());
        }

        match self.stack.last_mut().unwrap() {
            Frame::Sequence {
                items, span_items, ..
            } => {
                if items.len() >= self.config.max_sequence_length {
                    return Err(Error::Budget(crate::BudgetBreach::MaxSequenceLength {
                        limit: self.config.max_sequence_length,
                        observed: items.len() + 1,
                    }));
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
                // distinct-typed collision. Skip the clone when the key is
                // a merge key (`<<`) that will be buffered rather than
                // inserted — merge values bypass the collision check, so
                // the clone would be pure waste on `<<`-heavy documents.
                let is_buffered_merge_key = may_be_merge_key
                    && matches!(&value, Value::String(s) if s == MERGE_KEY)
                    && !matches!(self.config.merge_key_policy, MergeKeyPolicy::AsOrdinary);
                // A buffered merge key is recorded as a `null` shape, exactly
                // as it was recorded as `Value::Null` before, so a merge key
                // and a literal `null` key still compare equal here.
                let key_value = if is_buffered_merge_key {
                    KeyShape::Other(Box::new(Value::Null))
                } else {
                    KeyShape::of(&value)
                };
                // serde_yaml-profile: refuse a non-scalar key outright
                // instead of stringifying it.
                if matches!(
                    self.config.non_scalar_key_policy,
                    crate::de::NonScalarKeyPolicy::Error
                ) && matches!(&value, Value::Sequence(_) | Value::Mapping(_))
                {
                    let kind = if matches!(&value, Value::Sequence(_)) {
                        "sequence"
                    } else {
                        "mapping"
                    };
                    return Err(Error::NonScalarKey {
                        kind,
                        location: Some(crate::error::Location::from_index(
                            input,
                            span_tree_start(&span),
                        )),
                    });
                }
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
                    key_may_merge: may_be_merge_key,
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
                key_may_merge,
                key_span,
                start,
                anchor,
                merge_values,
                tag,
            } => {
                let is_merge = *key_may_merge && key == MERGE_KEY;
                let merge_treat_as_ordinary =
                    matches!(self.config.merge_key_policy, MergeKeyPolicy::AsOrdinary);
                let merge_reject = matches!(self.config.merge_key_policy, MergeKeyPolicy::Error);
                if is_merge && merge_reject {
                    return Err(Error::Custom(
                        "merge key `<<` rejected by MergeKeyPolicy::Error".to_owned(),
                    ));
                }
                if is_merge && !merge_treat_as_ordinary {
                    self.meter.charge_merge_key(self.config)?;
                    merge_values.push(value);
                } else {
                    if map.len() >= self.config.max_mapping_keys {
                        return Err(Error::Budget(crate::BudgetBreach::MaxMappingKeys {
                            limit: self.config.max_mapping_keys,
                            observed: map.len() + 1,
                        }));
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
                        // 1.86 MSRV: `let`-chains stabilized in 1.88.
                        if *KeyShape::at(typed_keys, idx) != key_value {
                            let key_start = key_span.0;
                            return Err(self.key_collision_at(key, key_start, input));
                        }
                    }
                    match self.config.duplicate_key_policy {
                        DuplicateKeyPolicy::First => {
                            if !map.contains_key(&key) {
                                let _ = map.insert(key, value);
                                span_entries.push((*key_span, span));
                                KeyShape::record(typed_keys, map.len() - 1, key_value);
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
                                KeyShape::record(typed_keys, map.len() - 1, key_value);
                            }
                        }
                        DuplicateKeyPolicy::Error => {
                            if map.contains_key(&key) {
                                let key_start = key_span.0;
                                return Err(self.duplicate_key_at(key, key_start, input));
                            }
                            let _ = map.insert(key, value);
                            span_entries.push((*key_span, span));
                            KeyShape::record(typed_keys, map.len() - 1, key_value);
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

// ── Span-free loader (no_std path) ──────────────────────────────────────
//
// Only compiled when the `std` feature is disabled. The `std` build
// always uses the span-aware loader above so `Spanned<T>` fields are
// populated correctly.

/// Skip-span loader entry point: parse one document into `Value`
/// without building a `SpanTree`, silently discarding any document
/// past the first.
///
/// The deserialise entry points use the checked
/// [`load_exactly_one_no_spans`] instead; this unchecked form's sole
/// remaining caller is `cst::document::decode_key_token`, `std`-only
/// like the rest of the `cst` module.
#[cfg(feature = "std")]
pub(crate) fn load_one_no_spans(input: &str, config: &ParseConfig) -> Result<Value> {
    Ok(load_all_no_spans(input, config)?
        .into_iter()
        .next()
        .unwrap_or(Value::Null))
}

/// Like [`load_one_no_spans`], but errors if the stream carries more
/// than one document.
///
/// Used by `from_str` / `from_str_with_config`'s `Value` bypass and
/// its `no_std` AST path — see the matching [`load_exactly_one`] and
/// #351.
pub(crate) fn load_exactly_one_no_spans(input: &str, config: &ParseConfig) -> Result<Value> {
    let docs = load_all_no_spans(input, config)?;
    if docs.len() > 1 {
        return Err(Error::MoreThanOneDocument);
    }
    Ok(docs.into_iter().next().unwrap_or(Value::Null))
}

/// Skip-span loader entry point: parse all documents into
/// `Value`s without building `SpanTree`s. See [`load_one_no_spans`].
pub(crate) fn load_all_no_spans(input: &str, config: &ParseConfig) -> Result<Vec<Value>> {
    let mut parser = crate::parser::events::Parser::with_max_events(input, config.max_events);
    let mut loader = NoSpanLoader::new(config);
    loop {
        match parser.next_event() {
            Ok(Event::StreamEnd) => {
                loader.process_event(Event::StreamEnd, input)?;
                break;
            }
            Ok(event) => loader.process_event(event, input)?,
            Err(e) => return Err(e.into_error(input, config.max_events)),
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
        /// Byte offset of the sequence's first byte — carried so a
        /// completed composite used as a mapping key can locate a
        /// `NonScalarKey` refusal.
        start: usize,
    },
    MappingKey {
        map: Mapping,
        /// Byte offset of the mapping's first byte (see
        /// `Sequence::start`).
        start: usize,
        // Parallel to `map`: retains the *typed* value that produced
        // each string key so the value-arm's collision check can tell
        // a distinct-typed collision (`1` vs `"1"`) apart from a
        // genuine duplicate (`1` twice). Mirrors the span-full loader.
        typed_keys: Vec<KeyShape>,
        anchor: Option<String>,
        merge_values: Vec<Value>,
        tag: Option<(String, String)>,
    },
    MappingValue {
        map: Mapping,
        /// The enclosing mapping's own first byte, riding along so the
        /// key-state frame can be rebuilt with it (see
        /// `MappingKey::start`).
        start: usize,
        typed_keys: Vec<KeyShape>,
        key: String,
        // The typed key value the current `key` string was derived
        // from; consumed by the collision check in the value arm.
        key_value: KeyShape,
        /// Whether `key` may be read as a `<<` merge key — see the same
        /// field on [`Frame::MappingValue`].
        key_may_merge: bool,
        /// Byte offset of the key, so a duplicate or a collision can be
        /// reported where it is (#378); the only position this loader
        /// keeps.
        key_start: usize,
        anchor: Option<String>,
        merge_values: Vec<Value>,
        tag: Option<(String, String)>,
    },
}

struct NoSpanLoader<'a> {
    docs: Vec<Value>,
    stack: Vec<NoSpanFrame>,
    /// Anchored values with their expansion cost (see [`Loader`]).
    anchor_map: IndexMap<String, (Value, AliasCost)>,
    // Source byte-index of each anchor's definition, keyed by name.
    // Populated alongside `anchor_map` so an unknown-alias error can
    // point at the closest known definition — the same "did you mean
    // `&logger`?" affordance the streaming path already offers.
    anchor_def_spans: IndexMap<String, usize>,
    /// Anchors defined in earlier documents of the stream (name to
    /// byte index of the last definition). Anchors do not cross `---`
    /// (YAML 1.2.2 §3.2.2.2); when an alias names one of these the
    /// error says so instead of only "unknown anchor".
    earlier_anchor_defs: IndexMap<String, usize>,
    /// Every budget counter, shared with the span-full loader's order.
    meter: Meter,
    config: &'a ParseConfig,
    depth: usize,
    in_document: bool,
}

impl<'a> NoSpanLoader<'a> {
    /// The dotted path of the mapping being filled; see
    /// [`Loader::parent_path`].
    fn parent_path(&self) -> Option<String> {
        let enclosing = &self.stack[..self.stack.len().saturating_sub(1)];
        frames_path(enclosing.iter().map(|f| match f {
            NoSpanFrame::MappingValue { key, .. } => Some(key.clone()),
            NoSpanFrame::Sequence { items, .. } => Some(items.len().to_string()),
            _ => None,
        }))
    }

    /// The located duplicate-key refusal (#378); see
    /// [`Loader::duplicate_key_at`].
    fn duplicate_key_at(&self, key: String, key_start: usize, input: &str) -> Error {
        duplicate_key_at(key, self.parent_path(), key_start, input)
    }

    /// The located key-collision refusal (#378).
    fn key_collision_at(&self, key: String, key_start: usize, input: &str) -> Error {
        key_collision_at(key, self.parent_path(), key_start, input)
    }

    fn new(config: &'a ParseConfig) -> Self {
        NoSpanLoader {
            docs: Vec::new(),
            stack: Vec::new(),
            anchor_map: IndexMap::default(),
            anchor_def_spans: IndexMap::default(),
            earlier_anchor_defs: IndexMap::default(),
            meter: Meter::new(config),
            config,
            depth: 0,
            in_document: false,
        }
    }

    /// Charge and expand `*anchor`; see [`Loader::resolve_alias`].
    fn resolve_alias(&mut self, anchor: &str, alias_start: usize, input: &str) -> Result<Value> {
        if !self.in_document {
            return Err(Error::parse_at(
                "alias outside document",
                input,
                alias_start,
            ));
        }
        self.meter.charge_alias(self.config)?;
        let Some((value, cost)) = self.anchor_map.get(anchor) else {
            return Err(missing_anchor(
                anchor,
                alias_start,
                input,
                &self.anchor_def_spans,
                &self.earlier_anchor_defs,
            ));
        };
        self.meter.charge_expansion(cost, self.depth, self.config)?;
        Ok(value.clone())
    }

    #[allow(dead_code)] // load_all_no_spans drains `self.docs` directly today.
    fn into_docs(self) -> Vec<Value> {
        self.docs
    }

    fn process_event(&mut self, event: Event<'_>, input: &str) -> Result<()> {
        self.meter.charge_event(&event, self.config)?;
        match event {
            Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart => {
                self.in_document = true;
                self.anchor_map.clear();
                self.earlier_anchor_defs
                    .extend(self.anchor_def_spans.drain(..));
            }
            Event::DocumentEnd => {
                self.in_document = false;
            }
            Event::Alias { anchor, span } => {
                let value = self.resolve_alias(&anchor, span.start, input)?;
                self.push_value(value, false, span.start, input)?;
            }
            Event::Scalar {
                value,
                style,
                anchor,
                tag,
                span,
            } => {
                // serde_yaml-profile: a plain decimal integer past
                // `u64::MAX` is refused instead of degrading to an
                // approximate f64.
                if self.config.integer_overflow_errors
                    && matches!(style, crate::parser::ScalarStyle::Plain)
                    && tag.is_none()
                    && overflows_u64_decimal(&value)
                {
                    return Err(Error::IntegerOverflow {
                        location: Some(crate::error::Location::from_index(input, span.start)),
                        path: frames_path(self.stack.iter().map(|f| match f {
                            NoSpanFrame::MappingValue { key, .. } => Some(key.clone()),
                            NoSpanFrame::Sequence { items, .. } => Some(items.len().to_string()),
                            _ => None,
                        })),
                    });
                }
                let v =
                    if let Some(t) = tag {
                        if self.config.tag_registry.as_ref().is_some_and(|r| {
                            crate::streaming::tag_is_registry_stripped(&t.0, &t.1, r)
                        }) {
                            // Registered tag: strip through and resolve as if
                            // untagged, exactly as the streaming path does.
                            resolve_untagged_scalar(value, style, self.config)
                        } else {
                            resolve_tagged_scalar(
                                &t.0,
                                &t.1,
                                &value,
                                self.config.lossless_u64_integers(),
                            )?
                        }
                    } else {
                        resolve_untagged_scalar(value, style, self.config)
                    };
                // Merge-key eligibility is decided here, while the scalar's
                // presentation is still known — `resolve_untagged_scalar`
                // yields `Value::String("<<")` for a plain and a quoted `<<`
                // alike.
                let is_plain_merge_candidate = matches!(style, crate::parser::ScalarStyle::Plain);
                if let Some(name) = anchor {
                    let _ = self.anchor_def_spans.insert(name.clone(), span.start);
                    let cost = AliasCost::of_value(&v);
                    let _ = self.anchor_map.insert(name, (v.clone(), cost));
                }
                self.push_value(v, is_plain_merge_candidate, span.start, input)?;
            }
            Event::SequenceStart { anchor, tag, span } => {
                self.depth += 1;
                if budget::depth_exceeded(self.depth, self.config.max_depth) {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                if let Some(name) = anchor.as_ref() {
                    let _ = self.anchor_def_spans.insert(name.clone(), span.start);
                }
                self.stack.push(NoSpanFrame::Sequence {
                    items: Vec::new(),
                    anchor,
                    tag,
                    start: span.start,
                });
            }
            Event::SequenceEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                if let Some(NoSpanFrame::Sequence {
                    items,
                    anchor,
                    tag,
                    start,
                }) = self.stack.pop()
                {
                    let inner = Value::Sequence(items);
                    let v = wrap_with_tag(inner, tag.as_ref(), self.config.tag_registry.as_deref());
                    if let Some(name) = anchor {
                        let cost = AliasCost::of_value(&v);
                        let _ = self.anchor_map.insert(name, (v.clone(), cost));
                    }
                    self.push_value(v, false, start, input)?;
                }
            }
            Event::MappingStart { anchor, tag, span } => {
                self.depth += 1;
                if budget::depth_exceeded(self.depth, self.config.max_depth) {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                if let Some(name) = anchor.as_ref() {
                    let _ = self.anchor_def_spans.insert(name.clone(), span.start);
                }
                self.stack.push(NoSpanFrame::MappingKey {
                    map: Mapping::new(),
                    start: span.start,
                    typed_keys: Vec::new(),
                    anchor,
                    merge_values: Vec::new(),
                    tag,
                });
            }
            Event::MappingEnd { .. } => {
                self.depth = self.depth.saturating_sub(1);
                if let Some(NoSpanFrame::MappingKey {
                    mut map,
                    start,
                    typed_keys: _,
                    anchor,
                    merge_values,
                    tag,
                }) = self.stack.pop()
                {
                    for mv in merge_values {
                        apply_merge(&mut map, mv)?;
                    }
                    let inner = Value::Mapping(map);
                    let v = wrap_with_tag(inner, tag.as_ref(), self.config.tag_registry.as_deref());
                    if let Some(name) = anchor {
                        let cost = AliasCost::of_value(&v);
                        let _ = self.anchor_map.insert(name, (v.clone(), cost));
                    }
                    self.push_value(v, false, start, input)?;
                }
            }
        }
        Ok(())
    }

    /// Push a completed value into the enclosing frame.
    ///
    /// `may_be_merge_key` mirrors `push_node` on the span-tracking loader:
    /// only a **plain** `<<` scalar is a merge key, and by the time a value
    /// arrives here it is a `Value::String("<<")` however it was written.
    fn push_value(
        &mut self,
        value: Value,
        may_be_merge_key: bool,
        node_start: usize,
        input: &str,
    ) -> Result<()> {
        if self.stack.is_empty() {
            self.docs.push(value);
            return Ok(());
        }
        match self.stack.last_mut().unwrap() {
            NoSpanFrame::Sequence { items, .. } => {
                if items.len() >= self.config.max_sequence_length {
                    return Err(Error::Budget(crate::BudgetBreach::MaxSequenceLength {
                        limit: self.config.max_sequence_length,
                        observed: items.len() + 1,
                    }));
                }
                items.push(value);
            }
            NoSpanFrame::MappingKey {
                map,
                start,
                typed_keys,
                anchor,
                merge_values,
                tag,
            } => {
                // Retain the typed key before coercing to a string so
                // the value arm can distinguish a distinct-typed
                // collision (`1` vs `"1"`) from a genuine duplicate.
                // Skip the clone on merge keys that will be buffered
                // rather than inserted; merge values bypass the
                // collision check, so the clone would be waste.
                let is_buffered_merge_key = may_be_merge_key
                    && matches!(&value, Value::String(s) if s == MERGE_KEY)
                    && !matches!(self.config.merge_key_policy, MergeKeyPolicy::AsOrdinary);
                // A buffered merge key is recorded as a `null` shape, exactly
                // as it was recorded as `Value::Null` before, so a merge key
                // and a literal `null` key still compare equal here.
                let key_value = if is_buffered_merge_key {
                    KeyShape::Other(Box::new(Value::Null))
                } else {
                    KeyShape::of(&value)
                };
                // serde_yaml-profile: refuse a non-scalar key outright
                // instead of stringifying it.
                if matches!(
                    self.config.non_scalar_key_policy,
                    crate::de::NonScalarKeyPolicy::Error
                ) && matches!(&value, Value::Sequence(_) | Value::Mapping(_))
                {
                    let kind = if matches!(&value, Value::Sequence(_)) {
                        "sequence"
                    } else {
                        "mapping"
                    };
                    return Err(Error::NonScalarKey {
                        kind,
                        location: Some(crate::error::Location::from_index(input, node_start)),
                    });
                }
                if let Some(key) = value_to_key_string(value) {
                    let old_map = core::mem::take(map);
                    let old_typed_keys = core::mem::take(typed_keys);
                    let old_anchor = anchor.take();
                    let old_merge_values = core::mem::take(merge_values);
                    let old_tag = tag.take();
                    *self.stack.last_mut().unwrap() = NoSpanFrame::MappingValue {
                        key_may_merge: may_be_merge_key,
                        map: old_map,
                        start: *start,
                        typed_keys: old_typed_keys,
                        key,
                        key_value,
                        key_start: node_start,
                        anchor: old_anchor,
                        merge_values: old_merge_values,
                        tag: old_tag,
                    };
                }
            }
            NoSpanFrame::MappingValue {
                map,
                start,
                typed_keys,
                key,
                key_value,
                key_may_merge,
                key_start,
                anchor,
                merge_values,
                tag,
            } => {
                let is_merge = *key_may_merge && key == MERGE_KEY;
                let merge_treat_as_ordinary =
                    matches!(self.config.merge_key_policy, MergeKeyPolicy::AsOrdinary);
                let merge_reject = matches!(self.config.merge_key_policy, MergeKeyPolicy::Error);
                if is_merge && merge_reject {
                    return Err(Error::Custom(
                        "merge key `<<` rejected by MergeKeyPolicy::Error".to_owned(),
                    ));
                }
                if is_merge && !merge_treat_as_ordinary {
                    self.meter.charge_merge_key(self.config)?;
                    merge_values.push(value);
                } else {
                    if map.len() >= self.config.max_mapping_keys {
                        return Err(Error::Budget(crate::BudgetBreach::MaxMappingKeys {
                            limit: self.config.max_mapping_keys,
                            observed: map.len() + 1,
                        }));
                    }
                    // Steal the owned key out of the frame instead of
                    // cloning it — the frame is overwritten (without
                    // `key`) immediately below, so the emptied slot is
                    // discarded.
                    let key = core::mem::take(key);
                    let key_value = core::mem::take(key_value);
                    // Distinct-typed collision: the string key already
                    // exists but was produced by a different typed key
                    // (e.g. `1` then `"1"`). Silently collapsing these
                    // would drop an entry — refuse regardless of
                    // DuplicateKeyPolicy so the fast `Value` path has
                    // the same guard as the span-full loader.
                    if let Some(idx) = map.get_index_of(&key) {
                        if *KeyShape::at(typed_keys, idx) != key_value {
                            let key_start = *key_start;
                            return Err(self.key_collision_at(key, key_start, input));
                        }
                        // Genuine duplicate: apply the caller's
                        // policy. Mirrors the span-full loader arms.
                        match self.config.duplicate_key_policy {
                            DuplicateKeyPolicy::First => {
                                // Keep the first occurrence: no-op.
                            }
                            DuplicateKeyPolicy::Last => {
                                let _ = map.insert(key, value);
                            }
                            DuplicateKeyPolicy::Error => {
                                let key_start = *key_start;
                                return Err(self.duplicate_key_at(key, key_start, input));
                            }
                        }
                    } else {
                        let _ = map.insert(key, value);
                        KeyShape::record(typed_keys, map.len() - 1, key_value);
                    }
                    debug_assert!(
                        typed_keys.is_empty() || typed_keys.len() == map.len(),
                        "typed_keys must be empty or parallel to map"
                    );
                }
                let old_map = core::mem::take(map);
                let old_typed_keys = core::mem::take(typed_keys);
                let old_anchor = anchor.take();
                let old_merge_values = core::mem::take(merge_values);
                let old_tag = tag.take();
                *self.stack.last_mut().unwrap() = NoSpanFrame::MappingKey {
                    map: old_map,
                    start: *start,
                    typed_keys: old_typed_keys,
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
/// `[-+]?0[0-9]+` — a decimal spelling with a leading zero (octal in
/// YAML 1.1, decimal in 1.2; libyaml resolved it as neither).
fn is_leading_zero_decimal(s: &str) -> bool {
    let d = s.strip_prefix(['+', '-']).unwrap_or(s);
    d.len() >= 2 && d.starts_with('0') && d.bytes().all(|b| b.is_ascii_digit())
}

/// YAML 1.1 binary integer: `[-+]?0b[0-1_]+`.
fn parse_legacy_binary(s: &str) -> Option<i64> {
    let (negative, rest) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let digits = rest
        .strip_prefix("0b")
        .or_else(|| rest.strip_prefix("0B"))?;
    if digits.is_empty() || digits.bytes().all(|b| b == b'_') {
        return None;
    }
    let mut n: i64 = 0;
    for b in digits.bytes() {
        match b {
            b'_' => {}
            b'0' | b'1' => {
                n = n.checked_mul(2)?.checked_add(i64::from(b - b'0'))?;
            }
            _ => return None,
        }
    }
    Some(if negative { -n } else { n })
}

/// One of the explicit infinity spellings the resolver accepts.
fn is_inf_spelling(s: &str) -> bool {
    matches!(
        s,
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" | "-.inf" | "-.Inf" | "-.INF"
    )
}

/// A plain decimal integer (optional `+`) that does not fit `u64`.
fn overflows_u64_decimal(s: &str) -> bool {
    let d = s.strip_prefix('+').unwrap_or(s);
    !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()) && d.parse::<u64>().is_err()
}

/// Byte offset where a span tree's node begins.
#[cfg(feature = "std")]
fn span_tree_start(span: &SpanTree) -> usize {
    match span {
        SpanTree::Leaf(s, _)
        | SpanTree::Sequence { start: s, .. }
        | SpanTree::Mapping { start: s, .. } => *s,
        SpanTree::Alias(inner) => span_tree_start(inner),
    }
}

/// `true` when the node beginning at `start` opens with `[` once
/// its properties are skipped. Since v0.0.30 a node's event span
/// starts at its `&anchor`/`!tag` properties, so flow detection has
/// to scan past them: property tokens run to whitespace (verbatim
/// tags to `>`), and a plain scalar can never begin with `&` or
/// `!`, so the scan is unambiguous.
#[cfg(feature = "std")]
fn starts_with_flow_bracket(input: &str, start: usize) -> bool {
    let bytes = input.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'!' if bytes.get(i + 1) == Some(&b'<') => {
                while i < bytes.len() && bytes[i] != b'>' {
                    i += 1;
                }
                i += 1;
            }
            b'&' | b'!' => {
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
            }
            b if b.is_ascii_whitespace() => i += 1,
            b => return b == b'[',
        }
    }
    false
}

/// Byte offset where a span tree's node ends.
#[cfg(feature = "std")]
fn span_tree_end(span: &SpanTree) -> usize {
    match span {
        SpanTree::Leaf(_, e)
        | SpanTree::Sequence { end: e, .. }
        | SpanTree::Mapping { end: e, .. } => *e,
        SpanTree::Alias(inner) => span_tree_end(inner),
    }
}

/// Dotted path from a loader frame stack — the field the currently
/// arriving value belongs to, for error messages that name it the
/// way serde's own path tracking would (`server.port`, `items.3`).
fn frames_path(segments: impl Iterator<Item = Option<String>>) -> Option<String> {
    let parts: Vec<_> = segments.flatten().collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    }
}

/// The dotted path of the entry `key` under the mapping at `parent`.
fn entry_path(parent: Option<String>, key: &str) -> String {
    match parent {
        Some(parent) => format!("{parent}.{key}"),
        None => key.to_owned(),
    }
}

/// The located duplicate-key refusal (#378): the entry's dotted path,
/// the enclosing mapping's path with `key` appended, and the key's
/// position in `input`.
fn duplicate_key_at(key: String, parent: Option<String>, key_start: usize, input: &str) -> Error {
    Error::DuplicateKeyAt {
        path: entry_path(parent, &key),
        location: crate::error::Location::from_index(input, key_start),
        key,
    }
}

/// The located key-collision refusal (#378), shaped like
/// [`duplicate_key_at`].
fn key_collision_at(key: String, parent: Option<String>, key_start: usize, input: &str) -> Error {
    Error::KeyCollisionAt {
        path: entry_path(parent, &key),
        location: crate::error::Location::from_index(input, key_start),
        key,
    }
}

pub(crate) fn value_to_key_string(value: Value) -> Option<String> {
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
            // Special-value floats need spec-shaped spellings so a
            // YAML `nan:` or `.inf:` key round-trips as the string
            // it was written as. Rust's Debug prints these as `NaN`
            // and `inf`; ryu prints them as `NaN` and `inf` too —
            // both diverge from the resolver's accepted plain forms
            // (`nan`, `inf`, `-inf`) that keyed lookups typically
            // use. Canonicalise to lowercase-plain here so keys
            // survive `Value` deserialization.
            if n.is_nan() {
                return Some("nan".into());
            }
            if n.is_infinite() {
                return Some(if n.is_sign_negative() {
                    "-inf".into()
                } else {
                    "inf".into()
                });
            }
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
fn wrap_with_tag(
    inner: Value,
    tag: Option<&(String, String)>,
    registry: Option<&crate::TagRegistry>,
) -> Value {
    let Some((handle, suffix)) = tag else {
        return inner;
    };
    // A tag registered for strip-through drops the tag and exposes the bare
    // collection, matching what the streaming path yields.
    if registry.is_some_and(|r| crate::streaming::tag_is_registry_stripped(handle, suffix, r)) {
        return inner;
    }
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
/// Resolve a scalar as if it carried no tag: quoted / literal / folded →
/// `String`; plain → YAML 1.2 schema resolution (via the shared
/// `resolve_plain_ext`, so the two loaders and the streaming path agree).
/// This is also the strip-through result for a tag registered in the active
/// [`TagRegistry`], keeping AST and streaming byte-for-byte equivalent.
fn resolve_untagged_scalar(
    value: Cow<'_, str>,
    style: crate::parser::ScalarStyle,
    config: &ParseConfig,
) -> Value {
    if style != crate::parser::ScalarStyle::Plain {
        // Quoted/literal/folded scalars always resolve as strings — YAML
        // schema resolution only applies to plain scalars.
        return Value::String(value.into_owned());
    }
    // serde_yaml-profile scalar quirks (both loader paths; the shim
    // configuration disqualifies the streaming fast path, and these
    // flags are part of that disqualification):
    if config.leading_zero_integer_strings && is_leading_zero_decimal(&value) {
        return Value::String(value.into_owned());
    }
    if config.legacy_binary_numbers {
        if let Some(n) = parse_legacy_binary(&value) {
            return Value::Number(Number::Integer(n));
        }
    }
    match crate::streaming::resolve_plain_ext(
        &value,
        config.strict_booleans,
        config.legacy_booleans,
        config.no_schema,
        config.legacy_octal_numbers,
        config.legacy_sexagesimal,
        config.lossless_u64_integers(),
    ) {
        crate::streaming::Scalar::Null => Value::Null,
        crate::streaming::Scalar::Bool(b) => Value::Bool(b),
        crate::streaming::Scalar::Int(i) => Value::Number(Number::Integer(i)),
        #[cfg(feature = "lossless-u64")]
        crate::streaming::Scalar::Uint(u) => Value::Number(Number::Unsigned(u)),
        crate::streaming::Scalar::Float(f) => {
            // serde_yaml-profile: a *literal* spelling that overflows
            // f64 (`1e999`) stays a string; the explicit infinity
            // spellings keep their float values.
            if config.float_overflow_strings && f.is_infinite() && !is_inf_spelling(&value) {
                Value::String(value.into_owned())
            } else {
                Value::Number(Number::Float(f))
            }
        }
        crate::streaming::Scalar::Str(s) => Value::String(s.into_owned()),
    }
}

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
                parsed.ok_or_else(|| Error::FailedToParseNumber(int_hint(value)))
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

/// The refusal for an alias whose anchor is not available.
///
/// Either the anchor is still open (the alias points inside the node
/// being built: YAML allows a cyclic graph, a `Value` tree cannot hold
/// one), or it is unknown in this document, in which case the error
/// suggests the closest defined name or says the anchor belongs to an
/// earlier document (anchors do not cross `---`, YAML 1.2.2 §3.2.2.2).
fn missing_anchor(
    anchor: &str,
    alias_start: usize,
    input: &str,
    defined: &IndexMap<String, usize>,
    earlier: &IndexMap<String, usize>,
) -> Error {
    let at = |idx: usize| crate::error::Location::from_index(input, idx);
    if let Some(&defined_at) = defined.get(anchor) {
        return Error::parse_at(
            format!(
                "alias `*{anchor}` points at `&{anchor}`, still being defined at {}; a self-referential node cannot be represented as a tree",
                at(defined_at)
            ),
            input,
            alias_start,
        );
    }
    let suggestion = crate::error::closest_name(anchor, defined.keys().map(String::as_str))
        .and_then(|s| defined.get(s).map(|&idx| (s.to_string(), at(idx))))
        .or_else(|| {
            earlier
                .get(anchor)
                .map(|&idx| (anchor.to_string(), at(idx)))
        });
    Error::UnknownAnchorAt {
        name: anchor.to_string(),
        location: at(alias_start),
        suggestion,
    }
}

#[cfg(test)]
mod tests {
    /// Mapping frames must not grow past the size class they occupy.
    ///
    /// The per-parse frame stack takes its first allocation at four
    /// frames. v0.0.14 grew `NoSpanFrame` from 176 to 280 bytes, which
    /// pushed that allocation from the 1 KiB class into 2 KiB and was the
    /// last piece of a +39% heap regression measured against v0.0.13.
    /// Boxing `KeyShape::Other` brought it back under. These bounds fail
    /// the build the next time a field is added without thinking about
    /// where it lands.
    #[test]
    fn mapping_frames_stay_within_their_size_class() {
        let no_span = size_of::<NoSpanFrame>();
        let span = size_of::<Frame>();
        assert!(
            no_span * 4 <= 1024,
            "NoSpanFrame is {no_span} bytes; four frames = {} > 1 KiB",
            no_span * 4
        );
        assert!(span <= 280, "Frame is {span} bytes (bound 280)");
    }

    /// The typed-key record stays unallocated while every key is a string.
    ///
    /// This is what makes the key-collision check free for the ordinary
    /// document. Before it, every key was cloned into a `Vec<Value>` that
    /// grew through four reallocations per mapping.
    #[test]
    fn typed_keys_stay_empty_for_string_keys() {
        let mut keys: Vec<KeyShape> = Vec::new();
        for idx in 0..64 {
            KeyShape::record(&mut keys, idx, KeyShape::Str);
        }
        assert!(keys.is_empty(), "string keys must not materialise the vec");
        assert_eq!(keys.capacity(), 0, "no allocation may have happened");
        assert_eq!(*KeyShape::at(&keys, 17), KeyShape::Str);
    }

    /// The first non-string key back-fills so indices stay parallel.
    #[test]
    fn first_typed_key_backfills_earlier_positions() {
        let mut keys: Vec<KeyShape> = Vec::new();
        for idx in 0..3 {
            KeyShape::record(&mut keys, idx, KeyShape::Str);
        }
        let one = KeyShape::Other(Box::new(Value::Number(1.into())));
        KeyShape::record(&mut keys, 3, one.clone());
        assert_eq!(keys.len(), 4);
        assert_eq!(*KeyShape::at(&keys, 0), KeyShape::Str);
        assert_eq!(*KeyShape::at(&keys, 3), one);
        // A string key that collides on text with the typed key differs.
        assert_ne!(*KeyShape::at(&keys, 3), KeyShape::Str);
    }

    use super::*;

    // Regression (v0.0.14 review): NoSpanLoader must zero the alias budget
    // at each DocumentStart, like the span-full Loader. A multi-document
    // stream whose documents are each within budget must not be rejected
    // because the per-document counts accumulate across the stream (a
    // std/no_std divergence, since the no-span loader is the no_std default).
    #[test]
    fn no_span_loader_resets_alias_budget_per_document() {
        let src = "\
p: &x 1
q: *x
r: *x
---
p: &y 2
q: *y
r: *y
---
p: &z 3
q: *z
r: *z
";
        // Each document uses exactly 2 aliases (== the limit); three
        // documents sum to 6. Pre-fix the no-span loader accumulated and
        // tripped RepetitionLimitExceeded partway through the second doc.
        let config = ParseConfig {
            max_alias_expansions: 2,
            ..ParseConfig::default()
        };
        let docs = load_all_no_spans(src, &config)
            .expect("each document is within the per-document alias budget");
        assert_eq!(docs.len(), 3);

        // Cross-path parity: the span-full loader already resets per
        // document, so it accepts the identical stream under the identical
        // budget. Both paths must agree.
        let via_span_full =
            crate::parser::parse(src, &config).expect("span-full loader accepts the same stream");
        assert_eq!(via_span_full.len(), 3);
    }
}

#[cfg(test)]
mod merge_key_eligibility_tests {
    //! Unit coverage for "only a plain `<<` is a merge key".
    //!
    //! The integration matrix in `tests/merge_key_plain_only.rs` drives
    //! this through documents on both the streaming and AST paths, which
    //! is the behaviour that matters. These pin the decision at the level
    //! where it is actually made, because the interesting property is a
    //! negative one: by the time a key reaches the mapping arms it is a
    //! `Value::String("<<")` whatever its presentation was, so the *only*
    //! thing keeping a quoted `"<<"` from merging is the flag threaded in
    //! from the scalar site.
    //!
    //! Two loaders exist — the span-tracking one and the no-span one —
    //! each with its own frame enum and its own pair of merge checks.
    //! Four sites in total, which is why the first attempt at this fix
    //! patched one of them and changed nothing observable.

    use super::{MERGE_KEY, MergeKeyPolicy, Value};

    /// Reproduces the eligibility decision made at both `MappingKey` arms.
    fn buffered_as_merge(value: &Value, may_be_merge_key: bool, policy: MergeKeyPolicy) -> bool {
        may_be_merge_key
            && matches!(value, Value::String(s) if s == MERGE_KEY)
            && !matches!(policy, MergeKeyPolicy::AsOrdinary)
    }

    /// Reproduces the decision made at both `MappingValue` arms.
    fn treated_as_merge(key: &str, key_may_merge: bool) -> bool {
        key_may_merge && key == MERGE_KEY
    }

    #[test]
    fn a_plain_double_angle_is_eligible() {
        let v = Value::String(MERGE_KEY.to_owned());
        assert!(buffered_as_merge(&v, true, MergeKeyPolicy::Auto));
        assert!(treated_as_merge(MERGE_KEY, true));
    }

    #[test]
    fn a_non_plain_double_angle_is_not_eligible() {
        // The quoted case: same `Value`, same key string, flag false.
        let v = Value::String(MERGE_KEY.to_owned());
        assert!(!buffered_as_merge(&v, false, MergeKeyPolicy::Auto));
        assert!(!treated_as_merge(MERGE_KEY, false));
    }

    #[test]
    fn the_value_alone_cannot_distinguish_them() {
        // The reason the flag has to be threaded at all: presentation is
        // gone, so both spellings are literally the same value here.
        let plain = Value::String(MERGE_KEY.to_owned());
        let quoted = Value::String(MERGE_KEY.to_owned());
        assert_eq!(plain, quoted, "identical once resolved");
        assert!(buffered_as_merge(&plain, true, MergeKeyPolicy::Auto));
        assert!(!buffered_as_merge(&quoted, false, MergeKeyPolicy::Auto));
    }

    #[test]
    fn an_ordinary_key_is_never_eligible_however_the_flag_is_set() {
        let v = Value::String("not-a-merge".to_owned());
        assert!(!buffered_as_merge(&v, true, MergeKeyPolicy::Auto));
        assert!(!treated_as_merge("not-a-merge", true));
    }

    #[test]
    fn as_ordinary_policy_suppresses_even_a_plain_double_angle() {
        let v = Value::String(MERGE_KEY.to_owned());
        assert!(!buffered_as_merge(&v, true, MergeKeyPolicy::AsOrdinary));
    }

    #[test]
    fn a_non_string_key_is_never_eligible() {
        // `<<` as an integer or sequence key cannot be a merge key.
        assert!(!buffered_as_merge(&Value::Null, true, MergeKeyPolicy::Auto));
        assert!(!buffered_as_merge(
            &Value::Sequence(vec![]),
            true,
            MergeKeyPolicy::Auto
        ));
    }

    #[test]
    fn eligibility_is_required_not_merely_sufficient() {
        // Both halves must hold. Neither the flag alone nor the spelling
        // alone may promote a key to a merge instruction.
        assert!(!treated_as_merge("x", true), "flag alone is not enough");
        assert!(
            !treated_as_merge(MERGE_KEY, false),
            "spelling alone is not enough"
        );
        assert!(treated_as_merge(MERGE_KEY, true), "both together");
    }
}

/// Explain an `!!int` that does not match YAML 1.2's integer form.
///
/// The two shapes people reach for are YAML 1.1's: a `0b` binary
/// literal and `_` digit separators. YAML 1.2's core schema dropped
/// both, so a parser that follows 1.2 has to refuse them under an
/// explicit `!!int`. Saying which one it is saves the reader a trip to
/// the specification.
fn int_hint(value: &str) -> String {
    let trimmed = value.trim();
    let negative = trimmed.starts_with('-');
    let unsigned = trimmed.strip_prefix(['-', '+']).unwrap_or(trimmed);
    if unsigned.starts_with("0b") || unsigned.starts_with("0B") {
        let digits = &unsigned[2..];
        if !digits.is_empty() && digits.bytes().all(|b| matches!(b, b'0' | b'1')) {
            if let Ok(n) = i64::from_str_radix(digits, 2) {
                let n = if negative { -n } else { n };
                return format!(
                    "!!int {trimmed}: YAML 1.2 has no binary literal, `0b` was YAML 1.1; write {n}"
                );
            }
        }
    }
    if unsigned.contains('_') {
        let stripped: String = trimmed.chars().filter(|c| *c != '_').collect();
        if stripped.parse::<i64>().is_ok() {
            return format!(
                "!!int {trimmed}: YAML 1.2 has no digit separators, `_` was YAML 1.1; write {stripped}"
            );
        }
    }
    format!("!!int {trimmed}")
}
