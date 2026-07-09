// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Streaming YAML deserializer that operates directly on parser events.
//!
//! Bypasses the intermediate `Value` AST for typed deserialization,
//! eliminating all intermediate allocations. Anchors and aliases are
//! handled natively via event buffering and replay. The common
//! `<<: *anchor` merge-key pattern is expanded natively.

use crate::prelude::*;

use rustc_hash::{FxHashMap, FxHashSet};
use serde::Deserialize;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use smallvec::SmallVec;

use crate::error::{Error, Result, closest_name};
use crate::parser::{Event, ParseConfig, Parser, ScalarStyle};
use crate::value::Value;
use core::fmt;

/// Sentinel error message used to signal fallback to the Value-based path.
const FALLBACK_SENTINEL: &str = "$__noyalib_streaming_fallback";

/// Buffer size for small collections, tuned for wasm-opt.
#[cfg(feature = "wasm-opt")]
const SMALL_VEC_SIZE: usize = 4;
#[cfg(not(feature = "wasm-opt"))]
const SMALL_VEC_SIZE: usize = 16;

#[derive(Debug, Clone)]
enum BufferedEvent {
    Scalar { value: String, style: ScalarStyle },
    SeqStart,
    SeqEnd,
    MapStart,
    MapEnd,
    Alias { anchor: String },
}

#[derive(Debug, Clone)]
pub(crate) enum Scalar<'a> {
    Null,
    Bool(bool),
    Int(i64),
    #[cfg(feature = "lossless-u64")]
    Uint(u64),
    Float(f64),
    Str(Cow<'a, str>),
}

/// A streaming YAML deserializer operating directly on parser events.
///
/// Bypasses the intermediate `Value` AST for the supported subset of
/// YAML. See the module-level documentation for when to use this type
/// versus [`crate::from_str`].
///
/// # Examples
///
/// ```
/// use noyalib::StreamingDeserializer;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Doc { k: i32 }
///
/// let mut de = StreamingDeserializer::new("k: 42\n");
/// let doc = Doc::deserialize(&mut de).unwrap();
/// assert_eq!(doc.k, 42);
/// ```
pub struct StreamingDeserializer<'a> {
    parser: Parser<'a>,
    input: &'a str,
    config: ParseConfig,
    tag_registry: Option<Arc<crate::TagRegistry>>,
    depth: usize,
    current: Option<Event<'a>>,
    raw_str_mode: bool,
    anchor_events: FxHashMap<String, SmallVec<[BufferedEvent; SMALL_VEC_SIZE]>>,
    anchor_def_spans: FxHashMap<String, usize>,
    replay_stack: Vec<SmallVec<[BufferedEvent; SMALL_VEC_SIZE]>>,
    recording: Option<(String, usize, SmallVec<[BufferedEvent; SMALL_VEC_SIZE]>)>,
    /// Count of alias expansions — bounded by `config.max_alias_expansions`
    /// to prevent billion-laughs style amplification attacks.
    alias_count: usize,
    /// Cumulative alias-expanded byte volume, bounded by
    /// `config.max_document_length` so aliases cannot amplify beyond the
    /// document-length cap.
    alias_bytes: usize,
}

impl fmt::Debug for StreamingDeserializer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamingDeserializer")
            .field("input_len", &self.input.len())
            .field("config", &self.config)
            .field("depth", &self.depth)
            .field("raw_str_mode", &self.raw_str_mode)
            .field("anchor_events_len", &self.anchor_events.len())
            .field("replay_stack_len", &self.replay_stack.len())
            .field("is_recording", &self.recording.is_some())
            .finish()
    }
}

impl<'a> StreamingDeserializer<'a> {
    /// Create a streaming deserializer over the given YAML input using
    /// default parser settings.
    pub fn new(input: &'a str) -> Self {
        Self::with_config(input, ParseConfig::default())
    }

    /// Create a streaming deserializer with a custom parser configuration.
    ///
    /// Accepts anything convertible to the internal `ParseConfig` —
    /// most callers pass a `&crate::ParserConfig`. Use this to tighten
    /// security limits (`max_depth`, `max_document_length`, alias
    /// expansion caps) for untrusted input.
    pub fn with_config<C>(input: &'a str, config: C) -> Self
    where
        C: Into<ParseConfig>,
    {
        StreamingDeserializer {
            parser: Parser::new(input),
            input,
            config: config.into(),
            tag_registry: None,
            depth: 0,
            current: None,
            raw_str_mode: false,
            anchor_events: FxHashMap::default(),
            anchor_def_spans: FxHashMap::default(),
            replay_stack: Vec::new(),
            recording: None,
            alias_count: 0,
            alias_bytes: 0,
        }
    }

    /// Install a [`TagRegistry`](crate::TagRegistry) on this
    /// deserializer. Custom tags listed in the registry will be
    /// stripped on the streaming path instead of forcing a fallback to
    /// the `Value` AST.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{StreamingDeserializer, TagRegistry};
    /// use serde::Deserialize;
    /// use std::sync::Arc;
    ///
    /// #[derive(Deserialize)]
    /// struct Temp(f64);
    ///
    /// let reg = Arc::new(TagRegistry::new().with("!Celsius"));
    /// let mut de = StreamingDeserializer::new("!Celsius 42.0")
    ///     .with_tag_registry(reg);
    /// let t = Temp::deserialize(&mut de).unwrap();
    /// assert_eq!(t.0, 42.0);
    /// ```
    #[must_use]
    pub fn with_tag_registry(mut self, registry: Arc<crate::TagRegistry>) -> Self {
        self.tag_registry = Some(registry);
        self
    }

    fn peek_parser_event(&mut self) -> Result<&Event<'a>> {
        if self.current.is_none() {
            let event = self
                .parser
                .next_event()
                .map_err(|e| Error::parse_at(&*e.message, self.input, e.index))?;
            self.current = Some(event);
        }
        Ok(self.current.as_ref().unwrap())
    }

    fn next_parser_event(&mut self) -> Result<Event<'a>> {
        let mut ev = if let Some(ev) = self.current.take() {
            ev
        } else {
            self.parser
                .next_event()
                .map_err(|e| Error::parse_at(&*e.message, self.input, e.index))?
        };
        self.handle_anchor(&mut ev);
        self.maybe_record(&ev);
        Ok(ev)
    }

    fn peek_event(&mut self) -> Result<&Event<'a>> {
        if self.current.is_none() {
            // Drain empty replay frames in a loop, not via tail-recursion.
            // Arbitrary YAML inputs can create deep empty-frame chains
            // during alias/merge expansion that blow the stack on recursion.
            let mut ev_opt = None;
            while let Some(buf) = self.replay_stack.last_mut() {
                if let Some(be) = buf.pop() {
                    ev_opt = Some(self.buffered_to_event(be));
                    break;
                }
                let _ = self.replay_stack.pop();
            }
            let mut ev = if let Some(ev) = ev_opt {
                ev
            } else {
                let mut ev = self
                    .parser
                    .next_event()
                    .map_err(|e| Error::parse_at(&*e.message, self.input, e.index))?;
                if let Event::Alias {
                    ref anchor,
                    ref span,
                } = ev
                {
                    let start = span.start;
                    ev = self.resolve_alias(anchor, start)?;
                }
                ev
            };
            self.handle_anchor(&mut ev);
            self.maybe_record(&ev);
            self.current = Some(ev);
        }
        Ok(self.current.as_ref().unwrap())
    }

    fn next_event(&mut self) -> Result<Event<'a>> {
        if let Some(ev) = self.current.take() {
            return Ok(ev);
        }

        let mut ev_opt = None;
        while let Some(buf) = self.replay_stack.last_mut() {
            if let Some(be) = buf.pop() {
                ev_opt = Some(self.buffered_to_event(be));
                break;
            }
            let _ = self.replay_stack.pop();
        }
        if let Some(mut ev) = ev_opt {
            self.handle_anchor(&mut ev);
            self.maybe_record(&ev);
            return Ok(ev);
        }

        let mut ev = self
            .parser
            .next_event()
            .map_err(|e| Error::parse_at(&*e.message, self.input, e.index))?;
        if let Event::Alias {
            ref anchor,
            ref span,
        } = ev
        {
            let start = span.start;
            ev = self.resolve_alias(anchor, start)?;
        }
        self.handle_anchor(&mut ev);
        self.maybe_record(&ev);
        Ok(ev)
    }

    fn buffered_to_event(&self, be: BufferedEvent) -> Event<'a> {
        let dummy_span = crate::parser::scanner_span_default();
        match be {
            BufferedEvent::Scalar { value, style } => Event::Scalar {
                value: Cow::Owned(value),
                style,
                anchor: None,
                tag: None,
                span: dummy_span,
            },
            BufferedEvent::SeqStart => Event::SequenceStart {
                anchor: None,
                tag: None,
                span: dummy_span,
            },
            BufferedEvent::SeqEnd => Event::SequenceEnd { span: dummy_span },
            BufferedEvent::MapStart => Event::MappingStart {
                anchor: None,
                tag: None,
                span: dummy_span,
            },
            BufferedEvent::MapEnd => Event::MappingEnd { span: dummy_span },
            BufferedEvent::Alias { anchor } => Event::Alias {
                anchor,
                span: dummy_span,
            },
        }
    }

    fn handle_anchor(&mut self, ev: &mut Event<'_>) {
        if self.recording.is_some() {
            return;
        }
        let def_start = match ev {
            Event::Scalar { span, .. }
            | Event::SequenceStart { span, .. }
            | Event::MappingStart { span, .. } => Some(span.start),
            _ => None,
        };
        let anchor_name = match ev {
            Event::Scalar { anchor, .. }
            | Event::SequenceStart { anchor, .. }
            | Event::MappingStart { anchor, .. } => anchor.take(),
            _ => None,
        };
        if let Some(name) = anchor_name {
            if let Some(start) = def_start {
                let _ = self.anchor_def_spans.insert(name.clone(), start);
            }
            self.recording = Some((name, 0, SmallVec::new()));
        }
    }

    fn maybe_record(&mut self, ev: &Event<'_>) {
        if let Some((_, ref mut depth, ref mut buf)) = self.recording {
            match ev {
                Event::Scalar { value, style, .. } => {
                    buf.push(BufferedEvent::Scalar {
                        value: value.to_string(),
                        style: *style,
                    });
                    if *depth == 0 {
                        let (name, _, events) = self.recording.take().unwrap();
                        let _ = self.anchor_events.insert(name, events);
                    }
                }
                Event::SequenceStart { .. } => {
                    buf.push(BufferedEvent::SeqStart);
                    *depth += 1;
                }
                Event::SequenceEnd { .. } => {
                    buf.push(BufferedEvent::SeqEnd);
                    *depth -= 1;
                    if *depth == 0 {
                        let (name, _, events) = self.recording.take().unwrap();
                        let _ = self.anchor_events.insert(name, events);
                    }
                }
                Event::MappingStart { .. } => {
                    buf.push(BufferedEvent::MapStart);
                    *depth += 1;
                }
                Event::MappingEnd { .. } => {
                    buf.push(BufferedEvent::MapEnd);
                    *depth -= 1;
                    if *depth == 0 {
                        let (name, _, events) = self.recording.take().unwrap();
                        let _ = self.anchor_events.insert(name, events);
                    }
                }
                Event::Alias { anchor, .. } => {
                    buf.push(BufferedEvent::Alias {
                        anchor: anchor.clone(),
                    });
                    if *depth == 0 {
                        let (name, _, events) = self.recording.take().unwrap();
                        let _ = self.anchor_events.insert(name, events);
                    }
                }
                _ => {}
            }
        }
    }

    fn resolve_alias(&mut self, name: &str, alias_start: usize) -> Result<Event<'a>> {
        self.alias_count += 1;
        if self.alias_count > self.config.max_alias_expansions {
            return Err(Error::RepetitionLimitExceeded);
        }
        // Estimate cumulative expansion cost (bytes) to guard against
        // billion-laughs style amplification — the same document_length
        // bound applies.
        if let Some(buf_ref) = self.anchor_events.get(name) {
            let bytes: usize = buf_ref
                .iter()
                .map(|ev| match ev {
                    BufferedEvent::Scalar { value, .. } => value.len() + 8,
                    _ => 4,
                })
                .sum();
            self.alias_bytes = self.alias_bytes.saturating_add(bytes);
            if self.alias_bytes > self.config.max_document_length {
                return Err(Error::RepetitionLimitExceeded);
            }
        }
        let buf = self
            .anchor_events
            .get(name)
            .cloned()
            .ok_or_else(|| self.build_unknown_anchor(name, alias_start))?;
        if buf.is_empty() {
            return Err(self.build_unknown_anchor(name, alias_start));
        }
        let mut reversed = buf;
        reversed.reverse();
        let first = reversed.pop().unwrap();
        if !reversed.is_empty() {
            self.replay_stack.push(reversed);
        }
        Ok(self.buffered_to_event(first))
    }

    fn inject_multi_merge_mapping_contents(&mut self, sources: &[(String, usize)]) -> Result<()> {
        let local_buf = self.buffer_rest_of_mapping()?;
        let mut seen_keys = extract_local_keys(&local_buf);
        let mut filtered_sources: SmallVec<[SmallVec<[BufferedEvent; SMALL_VEC_SIZE]>; 2]> =
            SmallVec::new();
        for (name, start) in sources {
            let target_buf = self
                .anchor_events
                .get(name)
                .cloned()
                .ok_or_else(|| self.build_unknown_anchor(name, *start))?;
            let body = extract_mapping_body(&target_buf).ok_or_else(|| self.fallback())?;
            let filtered = filter_merge_entries(body, &seen_keys).ok_or_else(|| self.fallback())?;
            collect_keys(body, &mut seen_keys).ok_or_else(|| self.fallback())?;
            filtered_sources.push(filtered);
        }
        if !local_buf.is_empty() {
            let mut rev = local_buf;
            rev.reverse();
            self.replay_stack.push(rev);
        }
        for mut filtered in filtered_sources.into_iter().rev() {
            if !filtered.is_empty() {
                filtered.reverse();
                self.replay_stack.push(filtered);
            }
        }
        Ok(())
    }

    fn buffer_rest_of_mapping(&mut self) -> Result<SmallVec<[BufferedEvent; SMALL_VEC_SIZE]>> {
        let mut buf = SmallVec::new();
        let mut depth: usize = 0;
        loop {
            let ev = self.next_parser_event()?;
            match ev {
                Event::MappingEnd { .. } => {
                    buf.push(BufferedEvent::MapEnd);
                    if depth == 0 {
                        return Ok(buf);
                    }
                    depth -= 1;
                }
                Event::MappingStart { .. } => {
                    buf.push(BufferedEvent::MapStart);
                    depth += 1;
                }
                Event::SequenceStart { .. } => {
                    buf.push(BufferedEvent::SeqStart);
                    depth += 1;
                }
                Event::SequenceEnd { .. } => {
                    buf.push(BufferedEvent::SeqEnd);
                    depth = depth.saturating_sub(1);
                }
                Event::Scalar { value, style, .. } => buf.push(BufferedEvent::Scalar {
                    value: value.into_owned(),
                    style,
                }),
                Event::Alias { anchor, .. } => buf.push(BufferedEvent::Alias { anchor }),
                _ => {}
            }
        }
    }

    fn build_unknown_anchor(&self, name: &str, alias_start: usize) -> Error {
        let loc = crate::error::Location::from_index(self.input, alias_start);
        let suggestion = closest_name(name, self.anchor_def_spans.keys().map(|s| s.as_str()))
            .and_then(|s| {
                self.anchor_def_spans.get(s).map(|&idx| {
                    (
                        s.to_string(),
                        crate::error::Location::from_index(self.input, idx),
                    )
                })
            });
        Error::UnknownAnchorAt {
            name: name.to_owned(),
            location: loc,
            suggestion,
        }
    }

    fn fallback(&self) -> Error {
        Error::Custom(FALLBACK_SENTINEL.to_owned())
    }

    fn skip_event(&mut self) -> Result<()> {
        let _ = self.next_event()?;
        Ok(())
    }

    fn skip_to_content(&mut self) -> Result<()> {
        loop {
            match self.peek_event()? {
                Event::StreamStart | Event::DocumentStart => {
                    self.skip_event()?;
                }
                _ => return Ok(()),
            }
        }
    }

    fn skip_value(&mut self) -> Result<()> {
        // Iterative traversal — pathologically deep YAML would blow the
        // stack in a recursive implementation.
        let mut balance: i64 = 0;
        loop {
            match self.next_event()? {
                Event::Scalar { .. } | Event::Alias { .. } if balance == 0 => {
                    return Ok(());
                }
                Event::Scalar { .. } | Event::Alias { .. } => {}
                Event::SequenceStart { .. } | Event::MappingStart { .. } => {
                    balance += 1;
                }
                Event::SequenceEnd { .. } | Event::MappingEnd { .. } => {
                    balance -= 1;
                    if balance <= 0 {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
    }

    /// Peek the next event and, if it carries a tag, take it out of the
    /// cached event so subsequent calls see the untagged value. Returns
    /// `None` if no event / no tag. Critical for tag handling: without
    /// this, `StreamingTagMapAccess` would recurse infinitely when its
    /// `next_value_seed` reroutes back through `deserialize_any`.
    fn take_tag_from_current(&mut self) -> Option<(String, String)> {
        let _ = self.peek_event().ok()?;
        match self.current.as_mut() {
            Some(Event::Scalar { tag, .. })
            | Some(Event::SequenceStart { tag, .. })
            | Some(Event::MappingStart { tag, .. }) => tag.take(),
            _ => None,
        }
    }

    /// Is `(handle, suffix)` registered in the tag registry as
    /// strip-through? Matches against the reconstructed full tag
    /// string (`"{handle}{suffix}"`, so `!Celsius` for `("!",
    /// "Celsius")`).
    ///
    /// Core YAML 1.2 tags (`!!str`, `!!int`, `!!bool`, `!!float`,
    /// `!!null`, `!!seq`, `!!map`) are never stripped — their tag
    /// carries semantic information (e.g. `!!str 42` forces string
    /// resolution) that the AST resolver must see. Registering one of
    /// those tags is a no-op.
    fn tag_in_registry(&self, tag: &(String, String)) -> bool {
        self.tag_registry
            .as_ref()
            .is_some_and(|r| tag_is_registry_stripped(&tag.0, &tag.1, r))
    }

    /// Put a tag back onto the currently-cached event. Used to restore
    /// state before returning a fallback error so the AST path sees the
    /// tagged node unchanged.
    fn restore_tag_to_current(&mut self, t: (String, String)) {
        if let Some(
            Event::Scalar { tag, .. }
            | Event::SequenceStart { tag, .. }
            | Event::MappingStart { tag, .. },
        ) = self.current.as_mut()
        {
            *tag = Some(t);
        }
    }

    fn resolve_scalar<'s>(&self, value: &'s str, style: ScalarStyle) -> Scalar<'s> {
        if style == ScalarStyle::Plain {
            resolve_plain_ext(
                value,
                self.config.strict_booleans,
                self.config.legacy_booleans,
                self.config.no_schema,
                self.config.legacy_octal_numbers,
                self.config.legacy_sexagesimal,
                self.config.lossless_u64_integers(),
            )
        } else {
            Scalar::Str(Cow::Borrowed(value))
        }
    }
}

impl<'de> de::Deserializer<'de> for &mut StreamingDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        // `deserialize_any` is used by `Value` and other type-erased
        // visitors — they rely on the AST tag resolver to produce the
        // correct `Value::Tagged(...)` / `Value::String` / `Value::Number`
        // variant for tagged scalars. Restore the tag and fall back.
        if let Some(t) = self.take_tag_from_current() {
            // TagRegistry opt-in: if the tag is registered as
            // strip-through, proceed without restoring so the inner
            // value deserializes directly into the target type.
            if !self.tag_in_registry(&t) {
                self.restore_tag_to_current(t);
                return Err(self.fallback());
            }
        }
        match self.next_event()? {
            Event::Scalar { value, style, .. } => match value {
                Cow::Borrowed(s) => match self.resolve_scalar(s, style) {
                    Scalar::Null => visitor.visit_none(),
                    Scalar::Bool(b) => visitor.visit_bool(b),
                    Scalar::Int(n) => visitor.visit_i64(n),
                    #[cfg(feature = "lossless-u64")]
                    Scalar::Uint(n) => visitor.visit_u64(n),
                    Scalar::Float(f) => visitor.visit_f64(f),
                    Scalar::Str(Cow::Borrowed(b)) => visitor.visit_borrowed_str(b),
                    Scalar::Str(Cow::Owned(o)) => visitor.visit_string(o),
                },
                Cow::Owned(s) => match self.resolve_scalar(&s, style) {
                    Scalar::Null => visitor.visit_none(),
                    Scalar::Bool(b) => visitor.visit_bool(b),
                    Scalar::Int(n) => visitor.visit_i64(n),
                    #[cfg(feature = "lossless-u64")]
                    Scalar::Uint(n) => visitor.visit_u64(n),
                    Scalar::Float(f) => visitor.visit_f64(f),
                    Scalar::Str(_) => visitor.visit_string(s),
                },
            },
            Event::SequenceStart { .. } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                let res = visitor.visit_seq(StreamingSeqAccess {
                    de: self,
                    finished: false,
                    count: 0,
                });
                // Decrement on both Ok and Err so a failed inner
                // visit_seq doesn't leak depth (issue #46).
                self.depth = self.depth.saturating_sub(1);
                res
            }
            Event::MappingStart { .. } => {
                self.depth += 1;
                if self.depth > self.config.max_depth {
                    return Err(Error::RecursionLimitExceeded { depth: self.depth });
                }
                let res = visitor.visit_map(StreamingMapAccess {
                    de: self,
                    finished: false,
                    has_emitted_key: false,
                    key_count: 0,
                    seen_keys: FxHashSet::default(),
                    seen_typed: FxHashMap::default(),
                });
                // Decrement on both Ok and Err so a failed inner
                // visit_map doesn't leak a depth count into the
                // outer scope (issue #46).
                self.depth = self.depth.saturating_sub(1);
                res
            }
            _ => Err(self.fallback()),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        if let Event::Scalar { value, style, .. } = self.next_event()? {
            if let Scalar::Bool(b) = self.resolve_scalar(&value, style) {
                return visitor.visit_bool(b);
            }
        }
        Err(Error::TypeMismatch {
            expected: "bool",
            found: "other".into(),
        })
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        if let Event::Scalar { value, style, .. } = self.next_event()? {
            match self.resolve_scalar(&value, style) {
                Scalar::Int(n) => return visitor.visit_i64(n),
                // Accept whole-number floats ("42.0") as integers so
                // typed struct fields can consume them.
                Scalar::Float(f)
                    if f.is_finite()
                        && f.fract() == 0.0
                        && f >= i64::MIN as f64
                        && f <= i64::MAX as f64 =>
                {
                    return visitor.visit_i64(f as i64);
                }
                _ => {}
            }
        }
        Err(Error::TypeMismatch {
            expected: "integer",
            found: "other".into(),
        })
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        if let Event::Scalar { value, style, .. } = self.next_event()? {
            match self.resolve_scalar(&value, style) {
                Scalar::Int(n) if n >= 0 => return visitor.visit_u64(n as u64),
                #[cfg(feature = "lossless-u64")]
                Scalar::Uint(n) => return visitor.visit_u64(n),
                Scalar::Float(f)
                    if f.is_finite() && f.fract() == 0.0 && f >= 0.0 && f <= u64::MAX as f64 =>
                {
                    return visitor.visit_u64(f as u64);
                }
                _ => {}
            }
        }
        Err(Error::TypeMismatch {
            expected: "unsigned integer",
            found: "other".into(),
        })
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        if let Event::Scalar { value, style, .. } = self.next_event()? {
            match self.resolve_scalar(&value, style) {
                Scalar::Float(f) => return visitor.visit_f64(f),
                Scalar::Int(n) => return visitor.visit_f64(n as f64),
                #[cfg(feature = "lossless-u64")]
                Scalar::Uint(n) => return visitor.visit_f64(n as f64),
                _ => {}
            }
        }
        Err(Error::TypeMismatch {
            expected: "float",
            found: "other".into(),
        })
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        // A tag (`!!str`, `!custom`, …) routes through the AST so the
        // tagged string is resolved correctly.
        if let Some(t) = self.take_tag_from_current() {
            if !self.tag_in_registry(&t) {
                self.restore_tag_to_current(t);
                return Err(self.fallback());
            }
        }
        match self.peek_event()? {
            Event::Scalar { .. } => {
                if let Event::Scalar { value, style, .. } = self.next_event()? {
                    // When the scalar borrows directly from the input
                    // (`Cow::Borrowed`), preserve the `'de` lifetime by
                    // calling `visit_borrowed_str`. This unlocks
                    // zero-copy `Deserialize<'de> for &'de str` without
                    // routing through the `Value` AST.
                    if self.raw_str_mode {
                        return match value {
                            Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                            Cow::Owned(s) => visitor.visit_string(s),
                        };
                    }
                    if style != ScalarStyle::Plain {
                        return match value {
                            Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                            Cow::Owned(s) => visitor.visit_string(s),
                        };
                    }
                    match value {
                        Cow::Borrowed(s) => match self.resolve_scalar(s, style) {
                            Scalar::Str(Cow::Borrowed(b)) => visitor.visit_borrowed_str(b),
                            Scalar::Str(Cow::Owned(o)) => visitor.visit_string(o),
                            _ => Err(Error::TypeMismatch {
                                expected: "string",
                                found: "non-string scalar".into(),
                            }),
                        },
                        Cow::Owned(s) => match self.resolve_scalar(&s, style) {
                            Scalar::Str(_) => visitor.visit_string(s),
                            _ => Err(Error::TypeMismatch {
                                expected: "string",
                                found: "non-string scalar".into(),
                            }),
                        },
                    }
                } else {
                    Err(Error::TypeMismatch {
                        expected: "string",
                        found: "other".into(),
                    })
                }
            }
            // A complex key (sequence / mapping / alias) cannot be
            // delivered as a string through the streaming path without
            // first materialising it — bail so the AST fallback (which
            // stringifies complex keys) takes over.
            _ => Err(self.fallback()),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        if let Event::Scalar { value, style, .. } = self.peek_event()? {
            if *style == ScalarStyle::Plain {
                match &**value {
                    "" | "~" | "null" | "Null" | "NULL" => {
                        self.skip_event()?;
                        return visitor.visit_none();
                    }
                    _ => {}
                }
            }
        }
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        if let Event::Scalar { value, style, .. } = self.next_event()? {
            if let Scalar::Null = self.resolve_scalar(&value, style) {
                return visitor.visit_unit();
            }
        }
        Err(Error::TypeMismatch {
            expected: "null",
            found: "other".into(),
        })
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return Err(self.fallback());
        }
        self.skip_to_content()?;
        if let Some(t) = self.take_tag_from_current() {
            match (t.0.as_str(), t.1.as_str()) {
                ("!!", "int")
                | ("!!", "float")
                | ("!!", "str")
                | ("!!", "bool")
                | ("!!", "null")
                | ("!!", "seq")
                | ("!!", "map") => {}
                _ => {
                    // TagRegistry opt-in: drop the tag and let the
                    // inner value deserialize straight into the
                    // newtype's target type.
                    if self.tag_in_registry(&t) {
                        return visitor.visit_newtype_struct(self);
                    }
                    return visitor.visit_map(StreamingTagMapAccess {
                        de: self,
                        tag: t,
                        done: false,
                    });
                }
            }
        }
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        // Tagged sequences route through the AST fallback so the tag is
        // preserved on the resulting `Value::Tagged(...)`. The registry
        // opts a specific tag out of that behaviour.
        if let Some(t) = self.take_tag_from_current() {
            if !self.tag_in_registry(&t) {
                self.restore_tag_to_current(t);
                return Err(self.fallback());
            }
        }
        if let Event::SequenceStart { .. } = self.next_event()? {
            self.depth += 1;
            if self.depth > self.config.max_depth {
                return Err(Error::RecursionLimitExceeded { depth: self.depth });
            }
            let res = visitor.visit_seq(StreamingSeqAccess {
                de: self,
                finished: false,
                count: 0,
            });
            // Decrement on Ok and Err (issue #46).
            self.depth = self.depth.saturating_sub(1);
            res
        } else {
            Err(Error::TypeMismatch {
                expected: "sequence",
                found: "other".into(),
            })
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        // Tagged mappings route through the AST fallback so the tag is
        // preserved on the resulting `Value::Tagged(...)`. The registry
        // opts a specific tag out of that behaviour.
        if let Some(t) = self.take_tag_from_current() {
            if !self.tag_in_registry(&t) {
                self.restore_tag_to_current(t);
                return Err(self.fallback());
            }
        }
        if let Event::MappingStart { .. } = self.next_event()? {
            self.depth += 1;
            if self.depth > self.config.max_depth {
                return Err(Error::RecursionLimitExceeded { depth: self.depth });
            }
            let res = visitor.visit_map(StreamingMapAccess {
                de: self,
                finished: false,
                has_emitted_key: false,
                key_count: 0,
                seen_keys: FxHashSet::default(),
                seen_typed: FxHashMap::default(),
            });
            // Decrement on Ok and Err (issue #46).
            self.depth = self.depth.saturating_sub(1);
            res
        } else {
            Err(Error::TypeMismatch {
                expected: "mapping",
                found: "other".into(),
            })
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        if let Some(t) = self.take_tag_from_current() {
            match (t.0.as_str(), t.1.as_str()) {
                ("!!", "int")
                | ("!!", "float")
                | ("!!", "str")
                | ("!!", "bool")
                | ("!!", "null")
                | ("!!", "seq")
                | ("!!", "map") => {}
                _ => {
                    return visitor.visit_enum(StreamingTagEnumAccess { de: self, tag: t });
                }
            }
        }
        match self.next_event()? {
            Event::Scalar { value, .. } => {
                visitor.visit_enum(value.into_owned().into_deserializer())
            }
            Event::MappingStart { .. } => {
                if let Event::Scalar { value, .. } = self.next_event()? {
                    visitor.visit_enum(StreamingEnumAccess {
                        de: self,
                        variant: value.into_owned(),
                    })
                } else {
                    Err(Error::TypeMismatch {
                        expected: "variant name",
                        found: "non-scalar".into(),
                    })
                }
            }
            _ => Err(Error::TypeMismatch {
                expected: "enum",
                found: "other".into(),
            }),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        if let Event::Scalar { value, .. } = self.next_event()? {
            return match value {
                Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                Cow::Owned(s) => visitor.visit_string(s),
            };
        }
        Err(Error::TypeMismatch {
            expected: "identifier",
            found: "non-scalar".into(),
        })
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_value()?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        // Accept `null` / `~` / empty scalar — delegate to deserialize_unit.
        self.deserialize_unit(visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // `Spanned<T>` is routed through this method by its Deserialize
        // impl. The streaming path does not carry source-span context,
        // so bail to the AST fallback for any Spanned deserialisation.
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return Err(self.fallback());
        }
        self.deserialize_map(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.skip_to_content()?;
        // YAML 1.2.2 §10.4: a `!!binary`-tagged scalar carries an
        // RFC 4648 base64 payload. Recognise the tag on the current
        // event, decode without buffering, and hand the bytes to the
        // visitor — the AST fallback path is unnecessary here.
        let is_binary = match self.current.as_ref() {
            Some(Event::Scalar { tag: Some(t), .. }) => {
                let full = format!("{}{}", t.0, t.1);
                crate::de::is_binary_tag(&full)
            }
            _ => false,
        };
        if is_binary {
            let _ = self.take_tag_from_current();
            if let Event::Scalar { value, .. } = self.next_event()? {
                return match crate::base64::decode(&value) {
                    Ok(bytes) => visitor.visit_byte_buf(bytes),
                    Err(why) => Err(Error::Deserialize(format!("!!binary: {why}"))),
                };
            }
            return Err(Error::TypeMismatch {
                expected: "string-shaped !!binary content",
                found: "non-scalar".into(),
            });
        }
        if let Event::Scalar { value, style, .. } = self.next_event()? {
            // Mirror the AST path: only string-shaped scalars are
            // accepted as bytes. A plain scalar that resolves to an
            // int / float / bool / null is a type error — the caller
            // wanted bytes, not a number's UTF-8 representation.
            return match self.resolve_scalar(&value, style) {
                Scalar::Str(s) => visitor.visit_bytes(s.as_bytes()),
                Scalar::Null => Err(Error::TypeMismatch {
                    expected: "bytes",
                    found: "null".into(),
                }),
                Scalar::Bool(_) => Err(Error::TypeMismatch {
                    expected: "bytes",
                    found: "bool".into(),
                }),
                Scalar::Int(_) => Err(Error::TypeMismatch {
                    expected: "bytes",
                    found: "integer".into(),
                }),
                #[cfg(feature = "lossless-u64")]
                Scalar::Uint(_) => Err(Error::TypeMismatch {
                    expected: "bytes",
                    found: "integer".into(),
                }),
                Scalar::Float(_) => Err(Error::TypeMismatch {
                    expected: "bytes",
                    found: "float".into(),
                }),
            };
        }
        Err(Error::TypeMismatch {
            expected: "bytes",
            found: "non-scalar".into(),
        })
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    serde::forward_to_deserialize_any! {
        i8 i16 i32 u8 u16 u32 f32 char
        tuple tuple_struct
    }
}

struct StreamingSeqAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
    finished: bool,
    count: usize,
}

impl<'de> SeqAccess<'de> for StreamingSeqAccess<'_, 'de> {
    type Error = Error;
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        // Symmetric guard with `StreamingMapAccess::next_key_seed`
        // — callers that re-invoke `next_element` after the
        // sequence is exhausted must not consume events from the
        // parent context. See issue #46 for the matching map-side
        // failure mode.
        if self.finished {
            return Ok(None);
        }
        if matches!(self.de.peek_event()?, Event::SequenceEnd { .. }) {
            self.de.skip_event()?;
            self.finished = true;
            return Ok(None);
        }
        self.count += 1;
        if self.count > self.de.config.max_sequence_length {
            return Err(Error::Parse(format!(
                "sequence exceeds maximum length of {}",
                self.de.config.max_sequence_length
            )));
        }
        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl Drop for StreamingSeqAccess<'_, '_> {
    fn drop(&mut self) {
        if !self.finished {
            loop {
                match self.de.peek_event() {
                    Ok(Event::SequenceEnd { .. }) => {
                        let _ = self.de.skip_event();
                        break;
                    }
                    Ok(_) => {
                        if self.de.skip_value().is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

/// Convert a resolved streaming `Scalar` into an owned `Value` — used only to
/// canonicalise a mapping key for distinct-typed collision detection, matching
/// the AST loader's untagged-scalar resolution.
fn scalar_to_value(s: Scalar<'_>) -> Value {
    match s {
        Scalar::Null => Value::Null,
        Scalar::Bool(b) => Value::Bool(b),
        Scalar::Int(n) => Value::Number(crate::value::Number::Integer(n)),
        #[cfg(feature = "lossless-u64")]
        Scalar::Uint(n) => Value::Number(crate::value::Number::Unsigned(n)),
        Scalar::Float(f) => Value::Number(crate::value::Number::Float(f)),
        Scalar::Str(s) => Value::String(s.into_owned()),
    }
}

struct StreamingMapAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
    finished: bool,
    has_emitted_key: bool,
    key_count: usize,
    /// Track keys seen at this mapping's top level to enforce
    /// `DuplicateKeyPolicy`. Populated lazily; bypassed when the policy
    /// is `First` (the visitor's own insertion order already matches).
    seen_keys: FxHashSet<String>,
    /// Canonical key string → the typed key `Value` first seen for it, to
    /// detect distinct-typed key collisions (`1` vs `"1"`, `~` vs `"null"`)
    /// in parity with the AST loader. Separate from `DuplicateKeyPolicy`:
    /// a same-string/different-type clash is always a `KeyCollision` error.
    seen_typed: FxHashMap<String, Value>,
}

impl<'de> MapAccess<'de> for StreamingMapAccess<'_, 'de> {
    type Error = Error;
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // Guard against serde visitors (e.g. `noyalib::Value`'s
        // `ValueVisitor::visit_map`) that call `next_key` again
        // after the previous call returned `Ok(None)`. Without
        // this guard the loop reads events from the *parent*
        // mapping and treats them as belonging to the now-empty
        // child — the recursive `deserialize_any` on each spilled
        // value inflates `self.depth` by one per entry and
        // triggers `RecursionLimitExceeded` on otherwise-shallow
        // documents (issue #46).
        if self.finished {
            return Ok(None);
        }
        loop {
            // Use `peek_event` so we see events queued on the replay stack
            // by a prior merge expansion — `peek_parser_event` bypasses it
            // and would read the next raw parser event, skipping the merge.
            let ev = self.de.peek_event()?;
            if matches!(ev, Event::MappingEnd { .. }) {
                self.de.skip_event()?;
                self.finished = true;
                return Ok(None);
            }
            if let Event::Scalar {
                value,
                style: ScalarStyle::Plain,
                ..
            } = ev
            {
                if value == "<<" {
                    if self.has_emitted_key {
                        return Err(self.de.fallback());
                    }
                    self.de.skip_event()?;
                    // The value event is raw-read from the parser so an
                    // `Event::Alias` is visible without auto-resolution.
                    match self.de.peek_parser_event()? {
                        Event::Alias { anchor, span } => {
                            let name = anchor.clone();
                            let start = span.start;
                            self.de.current = None;
                            self.de
                                .inject_multi_merge_mapping_contents(&[(name, start)])?;
                        }
                        Event::SequenceStart { .. } => {
                            self.de.skip_event()?;
                            let mut sources = Vec::new();
                            loop {
                                match self.de.peek_parser_event()? {
                                    Event::SequenceEnd { .. } => {
                                        self.de.skip_event()?;
                                        break;
                                    }
                                    Event::Alias { anchor, span } => {
                                        sources.push((anchor.clone(), span.start));
                                        self.de.skip_event()?;
                                    }
                                    _ => return Err(self.de.fallback()),
                                }
                            }
                            self.de.inject_multi_merge_mapping_contents(&sources)?;
                        }
                        _ => return Err(self.de.fallback()),
                    }
                    continue;
                }
            }
            // Enforce duplicate-key policy when not `Last` (the serde
            // default). The key is peeked as a raw scalar string so policy
            // is applied before the visitor sees it. Extract the key
            // eagerly so the `ev` borrow is released before we touch
            // `self.de` again.
            let key_info = if let Event::Scalar {
                value: key_val,
                style,
                tag,
                ..
            } = ev
            {
                Some((key_val.to_string(), *style, tag.is_none()))
            } else {
                None
            };
            // Distinct-typed key-collision guard (parity with the AST loader):
            // two keys whose canonical map-key string matches but whose typed
            // Value differs (`1` vs `"1"`, `~` vs `"null"`, `true` vs `"true"`)
            // are a `KeyCollision`, independent of `DuplicateKeyPolicy`. This is
            // the guard the streaming path lacked, so `from_str::<map/struct>`
            // and `from_str_borrowing::<Value>` silently collapsed such keys.
            // Untagged scalar keys only; tagged keys keep their prior behaviour.
            if let Some((ref raw, style, true)) = key_info {
                let typed = scalar_to_value(self.de.resolve_scalar(raw, style));
                if let Some(canon) = crate::parser::value_to_key_string(typed.clone()) {
                    match self.seen_typed.get(&canon) {
                        Some(prev) if *prev != typed => {
                            return Err(Error::KeyCollision(canon));
                        }
                        Some(_) => {}
                        None => {
                            let _ = self.seen_typed.insert(canon, typed);
                        }
                    }
                }
            }
            let key_str_opt = key_info.map(|(s, _, _)| s);
            let policy = self.de.config.duplicate_key_policy;
            if let Some(key_str) = key_str_opt {
                if policy != crate::parser::InternalDuplicateKeyPolicy::Last {
                    if self.seen_keys.contains(&key_str) {
                        match policy {
                            crate::parser::InternalDuplicateKeyPolicy::Error => {
                                return Err(Error::DuplicateKey(key_str));
                            }
                            crate::parser::InternalDuplicateKeyPolicy::First => {
                                // Skip duplicate key + value.
                                self.de.skip_value()?;
                                self.de.skip_value()?;
                                continue;
                            }
                            _ => {}
                        }
                    } else {
                        let _ = self.seen_keys.insert(key_str);
                    }
                }
            }
            self.key_count += 1;
            if self.key_count > self.de.config.max_mapping_keys {
                return Err(Error::Parse(format!(
                    "mapping exceeds maximum of {} keys",
                    self.de.config.max_mapping_keys
                )));
            }
            self.de.raw_str_mode = true;
            let res = seed.deserialize(&mut *self.de).map(Some);
            self.de.raw_str_mode = false;
            if res.is_ok() {
                self.has_emitted_key = true;
            }
            return res;
        }
    }
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        // Symmetric guard with `next_key_seed`. Callers that
        // misuse the MapAccess contract (e.g. invoking
        // `next_value` after the keys are exhausted) would
        // otherwise read into the parent mapping's value events
        // — the same `depth` leak documented in issue #46.
        if self.finished {
            return Err(Error::Custom(
                "next_value_seed called after MapAccess exhausted".into(),
            ));
        }
        seed.deserialize(&mut *self.de)
    }
}

impl Drop for StreamingMapAccess<'_, '_> {
    fn drop(&mut self) {
        if !self.finished {
            loop {
                match self.de.peek_event() {
                    Ok(Event::MappingEnd { .. }) => {
                        let _ = self.de.skip_event();
                        break;
                    }
                    Ok(_) => {
                        if self.de.skip_value().is_err() {
                            break;
                        }
                        if self.de.skip_value().is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

struct StreamingEnumAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
    variant: String,
}
impl<'a, 'de> de::EnumAccess<'de> for StreamingEnumAccess<'a, 'de> {
    type Error = Error;
    type Variant = StreamingVariantAccess<'a, 'de>;
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let de = de::value::StringDeserializer::<Error>::new(self.variant);
        let variant = seed.deserialize(de)?;
        Ok((variant, StreamingVariantAccess { de: self.de }))
    }
}

struct StreamingVariantAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
}
impl<'de> de::VariantAccess<'de> for StreamingVariantAccess<'_, 'de> {
    type Error = Error;
    fn unit_variant(self) -> Result<()> {
        let ev = self.de.next_event()?;
        if !matches!(ev, Event::MappingEnd { .. }) {
            self.de.current = Some(ev);
            self.de.skip_value()?;
            if !matches!(self.de.next_event()?, Event::MappingEnd { .. }) {
                return Err(Error::Invalid("expected mapping end".into()));
            }
        }
        Ok(())
    }
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let res = seed.deserialize(&mut *self.de)?;
        if !matches!(self.de.next_event()?, Event::MappingEnd { .. }) {
            return Err(Error::Invalid("expected mapping end".into()));
        }
        Ok(res)
    }
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let res = de::Deserializer::deserialize_seq(&mut *self.de, visitor)?;
        if !matches!(self.de.next_event()?, Event::MappingEnd { .. }) {
            return Err(Error::Invalid("expected mapping end".into()));
        }
        Ok(res)
    }
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let res = de::Deserializer::deserialize_map(&mut *self.de, visitor)?;
        if !matches!(self.de.next_event()?, Event::MappingEnd { .. }) {
            return Err(Error::Invalid("expected mapping end".into()));
        }
        Ok(res)
    }
}

#[allow(dead_code)]
struct StreamingTagMapAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
    tag: (String, String),
    done: bool,
}
impl<'de> MapAccess<'de> for StreamingTagMapAccess<'_, 'de> {
    type Error = Error;
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.done {
            Ok(None)
        } else {
            let full = if self.tag.0 == "!" {
                format!("!{}", self.tag.1)
            } else {
                format!("{}{}", self.tag.0, self.tag.1)
            };
            let de = de::value::StringDeserializer::<Error>::new(full);
            seed.deserialize(de).map(Some)
        }
    }
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.done = true;
        seed.deserialize(&mut *self.de)
    }
}

#[allow(dead_code)]
struct StreamingTagEnumAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
    tag: (String, String),
}
impl<'a, 'de> de::EnumAccess<'de> for StreamingTagEnumAccess<'a, 'de> {
    type Error = Error;
    type Variant = StreamingTagVariantAccess<'a, 'de>;
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let full = if self.tag.0 == "!" {
            format!("!{}", self.tag.1)
        } else {
            format!("{}{}", self.tag.0, self.tag.1)
        };
        let de = de::value::StringDeserializer::<Error>::new(full);
        let variant = seed.deserialize(de)?;
        Ok((variant, StreamingTagVariantAccess { de: self.de }))
    }
}

#[allow(dead_code)]
struct StreamingTagVariantAccess<'a, 'de> {
    de: &'a mut StreamingDeserializer<'de>,
}
impl<'de> de::VariantAccess<'de> for StreamingTagVariantAccess<'_, 'de> {
    type Error = Error;
    fn unit_variant(self) -> Result<()> {
        self.de.skip_value()
    }
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}

pub(crate) fn from_str_streaming<T>(s: &str, config: &crate::de::ParserConfig) -> Option<Result<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let parse_config = ParseConfig::from(config);
    if s.len() > parse_config.max_document_length {
        return Some(Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            parse_config.max_document_length
        ))));
    }
    let mut de = StreamingDeserializer::with_config(s, parse_config);
    if let Some(registry) = config.tag_registry.as_ref() {
        de = de.with_tag_registry(Arc::clone(registry));
    }
    let res = T::deserialize(&mut de);
    match res {
        Ok(val) => {
            // Drain remaining events to surface any errors lurking past
            // the point at which `T::deserialize` was satisfied. Without
            // this, lenient inputs like `--- key1: value1\n    key2: value2`
            // would lazily yield `Value::String("key1")` because the
            // serde visitor stops at the first complete node — the
            // subsequent tokens that violate the spec (e.g. "mapping
            // values are not allowed in this context") would never be
            // fetched. Stop *at* `StreamEnd` (querying past it would
            // return a benign "parser has already finished" error);
            // propagate any error encountered before that.
            loop {
                match de.next_event() {
                    Ok(Event::StreamEnd) => break,
                    Ok(Event::DocumentEnd | Event::StreamStart) => continue,
                    Ok(_) => break,
                    Err(e) => return Some(Err(e)),
                }
            }
            Some(Ok(val))
        }
        Err(ref e) => {
            if is_fallback_error(e) {
                None
            } else {
                Some(res)
            }
        }
    }
}

fn is_fallback_error(e: &Error) -> bool {
    match e {
        Error::Custom(msg) => msg == FALLBACK_SENTINEL,
        _ => false,
    }
}

/// Is the tag `{handle}{suffix}` registered for strip-through in `registry`?
///
/// Core YAML 1.2 tags (`!!int`, `!!float`, `!!str`, `!!bool`, `!!null`,
/// `!!seq`, `!!map`) are never stripped — their tag carries semantic
/// resolution the loader/streamer must see, so registering one is a no-op.
/// Shared by the streaming deserializer and the AST loader so both agree on
/// exactly what a registry strips (three-loader parity).
pub(crate) fn tag_is_registry_stripped(
    handle: &str,
    suffix: &str,
    registry: &crate::TagRegistry,
) -> bool {
    if matches!(
        (handle, suffix),
        ("!!", "int")
            | ("!!", "float")
            | ("!!", "str")
            | ("!!", "bool")
            | ("!!", "null")
            | ("!!", "seq")
            | ("!!", "map")
    ) {
        return false;
    }
    registry.contains(&format!("{handle}{suffix}"))
}

/// Resolve a plain scalar according to YAML 1.2's implicit-typing
/// rules, with three `ParserConfig` toggles exposed:
///
/// - `no_schema` — when `true`, keep every plain scalar as a
///   string. Useful for schema-strict pipelines where YAML's
///   implicit resolution of `yes` → `true` or `null` → `Null`
///   produces unwanted ambiguity.
/// - `legacy_octal` — when `true`, accept YAML 1.1-style bare
///   `0`-prefix octal literals (e.g. `0644`) in addition to the
///   YAML 1.2 `0o644` form. Off by default to honour YAML 1.2's
///   stricter integer schema.
/// - `legacy_sexagesimal` — when `true`, accept YAML 1.1-style
///   colon-separated base-60 numbers (`60:00` → 3 600,
///   `1:30:00` → 5 400). Off by default; YAML 1.2 dropped the
///   sexagesimal schema.
/// - `lossless_u64` — when `true` and the `lossless-u64` feature is
///   enabled, resolve unsigned integer scalars up to `u64::MAX`
///   before considering a float fallback.
pub(crate) fn resolve_plain_ext(
    s: &str,
    strict: bool,
    legacy: bool,
    no_schema: bool,
    legacy_octal: bool,
    legacy_sexagesimal: bool,
    lossless_u64: bool,
) -> Scalar<'_> {
    if no_schema {
        // Schema-strict mode: every plain scalar surfaces as a
        // string so callers see exactly what the file said.
        return Scalar::Str(Cow::Borrowed(s));
    }
    match s {
        "" | "~" | "null" | "Null" | "NULL" => Scalar::Null,
        "true" => Scalar::Bool(true),
        "false" => Scalar::Bool(false),
        "True" | "TRUE" if !strict => Scalar::Bool(true),
        "False" | "FALSE" if !strict => Scalar::Bool(false),
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => Scalar::Float(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => Scalar::Float(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => Scalar::Float(f64::NAN),
        "yes" | "Yes" | "YES" | "y" | "Y" if legacy => Scalar::Bool(true),
        "no" | "No" | "NO" | "n" | "N" if legacy => Scalar::Bool(false),
        "on" | "On" | "ON" if !strict && legacy => Scalar::Bool(true),
        "off" | "Off" | "OFF" if !strict && legacy => Scalar::Bool(false),
        _ => {
            if let Some(n) = parse_integer(s, legacy_octal, lossless_u64) {
                match n {
                    ParsedInteger::Signed(n) => Scalar::Int(n),
                    #[cfg(feature = "lossless-u64")]
                    ParsedInteger::Unsigned(n) => Scalar::Uint(n),
                }
            } else if lossless_u64 && looks_like_integer_literal(s, legacy_octal) {
                Scalar::Str(Cow::Borrowed(s))
            } else if legacy_sexagesimal {
                if let Some(n) = parse_sexagesimal_int(s) {
                    Scalar::Int(n)
                } else if let Some(f) = parse_sexagesimal_float(s) {
                    Scalar::Float(f)
                } else if let Ok(f) = s.parse::<f64>() {
                    Scalar::Float(f)
                } else {
                    Scalar::Str(Cow::Borrowed(s))
                }
            } else if let Ok(f) = s.parse::<f64>() {
                Scalar::Float(f)
            } else {
                Scalar::Str(Cow::Borrowed(s))
            }
        }
    }
}

fn looks_like_integer_literal(s: &str, legacy_octal: bool) -> bool {
    let b = s.as_bytes();
    if b.is_empty() {
        return false;
    }
    if b.len() > 2 && b[0] == b'0' && (bytes_to_char(b[1]) == 'x' || bytes_to_char(b[1]) == 'X') {
        return b[2..].iter().all(|c| c.is_ascii_hexdigit());
    }
    if b.len() > 2 && b[0] == b'0' && (bytes_to_char(b[1]) == 'o' || bytes_to_char(b[1]) == 'O') {
        return b[2..].iter().all(|c| (b'0'..=b'7').contains(c));
    }
    if legacy_octal && b.len() >= 2 && b[0] == b'0' {
        return b[1..].iter().all(|c| (b'0'..=b'7').contains(c));
    }
    let start = if b[0] == b'+' || b[0] == b'-' { 1 } else { 0 };
    start < b.len() && b[start..].iter().all(u8::is_ascii_digit)
}

enum ParsedInteger {
    Signed(i64),
    #[cfg(feature = "lossless-u64")]
    Unsigned(u64),
}

/// Parse a YAML 1.1 sexagesimal integer like `60:00` or
/// `-1:30:00` — colon-separated base-60 components evaluated
/// left-to-right. Returns `None` if the shape doesn't fit:
///
/// - At least one `:` is required (otherwise this overlaps with
///   plain integer parsing).
/// - Each component must be one or more decimal digits.
/// - Components other than the first must be `00..=59` to avoid
///   accepting nonsense like `1:99`.
/// - The leading sign (`-` or `+`) applies to the whole value.
fn parse_sexagesimal_int(s: &str) -> Option<i64> {
    let (sign, rest) = match s.as_bytes().first() {
        Some(b'-') => (-1i64, &s[1..]),
        Some(b'+') => (1i64, &s[1..]),
        _ => (1i64, s),
    };
    if !rest.contains(':') {
        return None;
    }
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    let mut total: i64 = 0;
    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: i64 = part.parse().ok()?;
        if idx > 0 && n >= 60 {
            return None;
        }
        total = total.checked_mul(60)?.checked_add(n)?;
    }
    sign.checked_mul(total)
}

/// Parse a YAML 1.1 sexagesimal float like `60:00.5`. Same shape
/// as the integer variant but the *last* component may be a
/// decimal float (`.5`, `5.5`).
fn parse_sexagesimal_float(s: &str) -> Option<f64> {
    let (sign, rest) = match s.as_bytes().first() {
        Some(b'-') => (-1.0f64, &s[1..]),
        Some(b'+') => (1.0f64, &s[1..]),
        _ => (1.0f64, s),
    };
    if !rest.contains(':') {
        return None;
    }
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    let last_idx = parts.len() - 1;
    let mut total: f64 = 0.0;
    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty() {
            return None;
        }
        let last_with_decimal = idx == last_idx && part.contains('.');
        if !last_with_decimal && !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: f64 = part.parse().ok()?;
        if idx > 0 && n >= 60.0 {
            return None;
        }
        total = total * 60.0 + n;
    }
    Some(sign * total)
}

fn parse_integer(s: &str, legacy_octal: bool, lossless_u64: bool) -> Option<ParsedInteger> {
    let b = s.as_bytes();
    if b.is_empty() {
        return None;
    }
    if b.len() > 2 && b[0] == b'0' && (bytes_to_char(b[1]) == 'x' || bytes_to_char(b[1]) == 'X') {
        return parse_radix_integer(&s[2..], 16, lossless_u64);
    }
    if b.len() > 2 && b[0] == b'0' && (bytes_to_char(b[1]) == 'o' || bytes_to_char(b[1]) == 'O') {
        return parse_radix_integer(&s[2..], 8, lossless_u64);
    }
    // YAML 1.1-style bare `0`-prefix octal — only when explicitly
    // opted in. The leading `0` must be followed by an octal digit
    // (0-7) so we don't misclassify `08` (decimal eight) as an
    // invalid octal.
    if legacy_octal
        && b.len() >= 2
        && b[0] == b'0'
        && b[1..].iter().all(|&c| c.is_ascii_digit() && c <= b'7')
    {
        return parse_radix_integer(&s[1..], 8, lossless_u64);
    }
    let start = if b[0] == b'+' || b[0] == b'-' { 1 } else { 0 };
    if start >= b.len() {
        return None;
    }
    if b[start..].iter().all(|&c| c.is_ascii_digit()) {
        // SIMD-friendly SWAR decimal parse — bit-for-bit equivalent
        // to `s.parse::<i64>()` but processes 8 digits per cycle on
        // the inner pipeline, beating the stdlib byte-by-byte loop
        // on data-heavy workloads (telemetry, IDs, port numbers).
        if let Some(n) = crate::simd::parse_decimal_i64(b) {
            return Some(ParsedInteger::Signed(n));
        }
        #[cfg(feature = "lossless-u64")]
        if lossless_u64 && b[0] != b'-' {
            let digits = if b[0] == b'+' { &b[1..] } else { b };
            return crate::simd::parse_decimal_u64(digits).map(ParsedInteger::Unsigned);
        }
        None
    } else {
        None
    }
}

fn parse_radix_integer(s: &str, radix: u32, lossless_u64: bool) -> Option<ParsedInteger> {
    #[cfg(not(feature = "lossless-u64"))]
    let _ = lossless_u64;
    if let Ok(n) = i64::from_str_radix(s, radix) {
        return Some(ParsedInteger::Signed(n));
    }
    #[cfg(feature = "lossless-u64")]
    if lossless_u64 {
        return u64::from_str_radix(s, radix)
            .ok()
            .map(ParsedInteger::Unsigned);
    }
    #[allow(unreachable_code)]
    None
}

fn bytes_to_char(b: u8) -> char {
    b as char
}

fn extract_mapping_body(buf: &[BufferedEvent]) -> Option<&[BufferedEvent]> {
    if buf.len() < 2
        || !matches!(buf.first(), Some(BufferedEvent::MapStart))
        || !matches!(buf.last(), Some(BufferedEvent::MapEnd))
    {
        None
    } else {
        Some(&buf[1..buf.len() - 1])
    }
}

fn collect_keys(body: &[BufferedEvent], seen: &mut FxHashSet<String>) -> Option<()> {
    let mut i = 0;
    while i < body.len() {
        if let BufferedEvent::Scalar { value, .. } = &body[i] {
            let _ = seen.insert(value.clone());
        } else {
            return None;
        }
        i += 1;
        if i < body.len() {
            let len = skip_buffered_value(&body[i..]);
            if len == 0 {
                return None;
            }
            i += len;
        } else {
            return None;
        }
    }
    Some(())
}

fn skip_buffered_value(slice: &[BufferedEvent]) -> usize {
    if slice.is_empty() {
        return 0;
    }
    match &slice[0] {
        BufferedEvent::Scalar { .. } | BufferedEvent::Alias { .. } => 1,
        BufferedEvent::SeqStart => {
            let mut d = 1;
            let mut i = 1;
            while i < slice.len() && d > 0 {
                match &slice[i] {
                    BufferedEvent::SeqStart => d += 1,
                    BufferedEvent::SeqEnd => d -= 1,
                    _ => {}
                }
                i += 1;
            }
            i
        }
        BufferedEvent::MapStart => {
            let mut d = 1;
            let mut i = 1;
            while i < slice.len() && d > 0 {
                match &slice[i] {
                    BufferedEvent::MapStart => d += 1,
                    BufferedEvent::MapEnd => d -= 1,
                    _ => {}
                }
                i += 1;
            }
            i
        }
        _ => 1,
    }
}

fn extract_local_keys(buf: &[BufferedEvent]) -> FxHashSet<String> {
    let mut keys = FxHashSet::default();
    let mut d: usize = 0;
    let mut key = true;
    for ev in buf {
        match ev {
            BufferedEvent::Scalar { value, .. } => {
                if d == 0 {
                    if key {
                        let _ = keys.insert(value.clone());
                    }
                    key = !key;
                }
            }
            BufferedEvent::Alias { .. } => {
                if d == 0 {
                    key = !key;
                }
            }
            BufferedEvent::MapStart | BufferedEvent::SeqStart => d += 1,
            BufferedEvent::MapEnd | BufferedEvent::SeqEnd => {
                if d == 1 {
                    key = true;
                }
                d = d.saturating_sub(1);
            }
        }
    }
    keys
}

fn filter_merge_entries(
    inner: &[BufferedEvent],
    local: &FxHashSet<String>,
) -> Option<SmallVec<[BufferedEvent; SMALL_VEC_SIZE]>> {
    let mut out = SmallVec::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        let key = if let BufferedEvent::Scalar { value, .. } = &inner[i] {
            value.clone()
        } else {
            return None;
        };
        let start = i;
        i += 1;
        if i >= inner.len() {
            return None;
        }
        let len = skip_buffered_value(&inner[i..]);
        if len == 0 {
            return None;
        }
        let end = i + len;
        if !local.contains(&key) {
            for ev in &inner[start..end] {
                out.push(ev.clone());
            }
        }
        i = end;
    }
    Some(out)
}
