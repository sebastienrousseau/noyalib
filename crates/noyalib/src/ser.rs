//! YAML serialization.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::fmt::Write as _;

use crate::error::{Error, Result};
use crate::value::{Mapping, Number, Sequence, Tag, TaggedValue, Value};

/// Flow style preference for collections.
///
/// Controls whether sequences and mappings should use inline (flow) or
/// multi-line (block) style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FlowStyle {
    /// Always use block style (multi-line).
    #[default]
    Block,
    /// Always use flow style (inline, JSON-like).
    Flow,
    /// Automatic: use flow for small collections, block for larger ones.
    Auto,
}

/// Scalar style preference for strings.
///
/// Controls how string values should be quoted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ScalarStyle {
    /// Automatic quoting based on content.
    #[default]
    Auto,
    /// Always use double quotes.
    DoubleQuoted,
    /// Always use single quotes.
    SingleQuoted,
    /// Use literal block style (|) for multiline.
    Literal,
    /// Use folded block style (>) for multiline.
    Folded,
    /// Plain (unquoted) style when possible.
    Plain,
}

/// Configuration options for YAML serialization.
///
/// # Examples
///
/// ```rust
/// use noyalib::{FlowStyle, ScalarStyle, SerializerConfig};
///
/// let config = SerializerConfig::new()
///     .indent(4)
///     .flow_style(FlowStyle::Auto)
///     .scalar_style(ScalarStyle::DoubleQuoted)
///     .document_start(true);
/// ```
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct SerializerConfig {
    /// Number of spaces per indentation level (default: 2).
    pub indent: usize,
    /// Whether to include document start marker `---` (default: false).
    pub document_start: bool,
    /// Whether to include document end marker `...` (default: false).
    pub document_end: bool,
    /// Whether to use block style for multiline strings (default: true).
    pub block_scalars: bool,
    /// Minimum number of newlines to trigger block scalar style (default: 1).
    pub block_scalar_threshold: usize,
    /// Flow style preference for collections (default: Block).
    pub flow_style: FlowStyle,
    /// Scalar style preference for strings (default: Auto).
    pub scalar_style: ScalarStyle,
    /// Maximum number of items in a collection to use flow style in Auto mode
    /// (default: 4).
    pub flow_threshold: usize,
    /// Force-quote all string scalars regardless of content (default: false).
    pub quote_all: bool,
    /// Prefer single quotes over double quotes when a string scalar needs
    /// quoting at all (default: false).
    ///
    /// A string that contains a character only double-quoted style can
    /// carry -- a control character, a tab, or any other code point that
    /// needs an escape sequence -- still gets double-quoted regardless of
    /// this setting, since single-quoted style has no escape mechanism
    /// beyond doubling an embedded `'`.
    pub prefer_single_quotes: bool,
    /// Compact list indentation under mapping keys (default: false).
    ///
    /// When `true`, sequence items under a mapping key align with the key
    /// instead of being indented an extra level.
    pub compact_list_indent: bool,
    /// Line width for folded block scalars (default: 80).
    pub folded_wrap_chars: usize,
    /// Minimum string length before block scalar style is considered (default: 80).
    pub min_fold_chars: usize,
    /// Maximum nesting depth allowed during serialization (default: 128).
    pub max_depth: usize,
}

impl Default for SerializerConfig {
    fn default() -> Self {
        Self {
            indent: 2,
            document_start: false,
            document_end: false,
            block_scalars: true,
            block_scalar_threshold: 1,
            flow_style: FlowStyle::Block,
            scalar_style: ScalarStyle::Auto,
            flow_threshold: 4,
            quote_all: false,
            prefer_single_quotes: false,
            compact_list_indent: false,
            folded_wrap_chars: 80,
            min_fold_chars: 80,
            max_depth: 128,
        }
    }
}

impl SerializerConfig {
    /// Create a new configuration with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indentation width.
    #[must_use]
    pub fn indent(mut self, spaces: usize) -> Self {
        self.indent = spaces;
        self
    }

    /// Enable or disable document start marker `---`.
    #[must_use]
    pub fn document_start(mut self, enabled: bool) -> Self {
        self.document_start = enabled;
        self
    }

    /// Enable or disable document end marker `...`.
    #[must_use]
    pub fn document_end(mut self, enabled: bool) -> Self {
        self.document_end = enabled;
        self
    }

    /// Enable or disable block scalar style for multiline strings.
    #[must_use]
    pub fn block_scalars(mut self, enabled: bool) -> Self {
        self.block_scalars = enabled;
        self
    }

    /// Set minimum newlines to trigger block scalar style.
    #[must_use]
    pub fn block_scalar_threshold(mut self, count: usize) -> Self {
        self.block_scalar_threshold = count;
        self
    }

    /// Set the flow style preference for collections.
    ///
    /// - `FlowStyle::Block`: Always use multi-line block style
    /// - `FlowStyle::Flow`: Always use inline flow style
    /// - `FlowStyle::Auto`: Use flow for small collections
    #[must_use]
    pub fn flow_style(mut self, style: FlowStyle) -> Self {
        self.flow_style = style;
        self
    }

    /// Set the scalar style preference for strings.
    ///
    /// - `ScalarStyle::Auto`: Quote only when necessary
    /// - `ScalarStyle::DoubleQuoted`: Always use double quotes
    /// - `ScalarStyle::SingleQuoted`: Always use single quotes
    /// - `ScalarStyle::Literal`: Use `|` for multiline
    /// - `ScalarStyle::Folded`: Use `>` for multiline
    /// - `ScalarStyle::Plain`: Unquoted when possible
    #[must_use]
    pub fn scalar_style(mut self, style: ScalarStyle) -> Self {
        self.scalar_style = style;
        self
    }

    /// Set the threshold for automatic flow style.
    ///
    /// Collections with this many or fewer items will use flow style
    /// when `flow_style` is set to `Auto`.
    #[must_use]
    pub fn flow_threshold(mut self, threshold: usize) -> Self {
        self.flow_threshold = threshold;
        self
    }

    /// Force-quote all string scalars regardless of content.
    #[must_use]
    pub fn quote_all(mut self, enabled: bool) -> Self {
        self.quote_all = enabled;
        self
    }

    /// Prefer single quotes over double quotes when a string scalar needs
    /// quoting at all.
    ///
    /// A string that needs a character only double-quoted style can carry
    /// (a control character, a tab, or anything else that needs an escape
    /// sequence) still gets double-quoted regardless of this setting.
    /// Output is unchanged when this is left at its default (`false`).
    #[must_use]
    pub fn prefer_single_quotes(mut self, enabled: bool) -> Self {
        self.prefer_single_quotes = enabled;
        self
    }

    /// Enable compact list indentation under mapping keys.
    ///
    /// When enabled, sequence items align with the key rather than
    /// being indented an extra level.
    #[must_use]
    pub fn compact_list_indent(mut self, enabled: bool) -> Self {
        self.compact_list_indent = enabled;
        self
    }

    /// Set the line width for folded block scalars.
    #[must_use]
    pub fn folded_wrap_chars(mut self, chars: usize) -> Self {
        self.folded_wrap_chars = chars;
        self
    }

    /// Set the minimum string length for block scalar style.
    ///
    /// Strings shorter than this threshold will not use block scalar
    /// (`|` / `>`) style, even if they contain newlines.
    #[must_use]
    pub fn min_fold_chars(mut self, chars: usize) -> Self {
        self.min_fold_chars = chars;
        self
    }

    /// Set the maximum nesting depth for serialization.
    #[must_use]
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }
}

/// Serialize a Rust value to a YAML `String`.
///
/// Uses [`SerializerConfig::default`]: 2-space indent, no
/// `---` / `...` markers, block style for collections, auto-style
/// scalars, block scalars enabled.
///
/// # Errors
///
/// Returns [`Error`](crate::Error) when:
///
/// - `Error::Serialize` — `T`'s `Serialize` impl returned an
///   error (custom `serde_core::ser::Error`, non-string mapping key
///   that cannot be coerced, …).
/// - `Error::DepthLimit` — the value graph exceeds
///   `SerializerConfig::max_depth` (default 128). Use
///   [`to_string_with_config`] to raise the cap when serialising
///   a deliberately deep structure.
///
/// `to_string` itself does not perform any I/O and never returns
/// `Error::Io`.
///
/// # Examples
///
/// ```rust
/// #[derive(serde::Serialize)]
/// struct Config {
///     name: String,
///     port: u16,
/// }
///
/// let config = Config {
///     name: "myapp".to_string(),
///     port: 8080,
/// };
///
/// let yaml = noyalib::to_string(&config).unwrap();
/// assert!(yaml.contains("name: myapp"));
/// assert!(yaml.contains("port: 8080"));
/// ```
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ?Sized + serde_core::Serialize,
{
    let v = to_value(value)?;
    value_to_string(&v, &SerializerConfig::default())
}

/// Serialize a Rust value to a YAML `String` with a custom
/// [`SerializerConfig`].
///
/// # Errors
///
/// Same variant set as [`to_string`]. The active `config`
/// controls which limit-related errors can fire — in particular,
/// raising `max_depth` lets deeper graphs through, lowering it
/// surfaces `Error::DepthLimit` sooner.
///
/// # Examples
///
/// ```rust
/// use noyalib::SerializerConfig;
///
/// #[derive(serde::Serialize)]
/// struct Config {
///     name: String,
///     port: u16,
/// }
///
/// let config = Config {
///     name: "myapp".to_string(),
///     port: 8080,
/// };
///
/// let yaml = noyalib::to_string_with_config(
///     &config,
///     &SerializerConfig::new().indent(4).document_start(true),
/// )
/// .unwrap();
/// assert!(yaml.starts_with("---"));
/// ```
pub fn to_string_with_config<T>(value: &T, config: &SerializerConfig) -> Result<String>
where
    T: ?Sized + serde_core::Serialize,
{
    let v = to_value(value)?;
    value_to_string(&v, config)
}

/// Serialize a Rust value to YAML and write to a
/// [`std::io::Write`] sink.
///
/// Internally serialises to a `String` then writes the bytes to
/// `writer` in a single call.
///
/// # Errors
///
/// - `Error::Io` — the underlying writer returned an I/O error.
/// - All variants documented on [`to_string`].
#[cfg(feature = "std")]
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: std::io::Write,
    T: ?Sized + serde_core::Serialize,
{
    to_writer_with_config(writer, value, &SerializerConfig::default())
}

/// Serialize a Rust value to YAML and write to a
/// [`std::io::Write`] sink, using a custom [`SerializerConfig`].
///
/// # Errors
///
/// - `Error::Io` — the underlying writer returned an I/O error.
/// - All variants documented on [`to_string_with_config`].
#[cfg(feature = "std")]
pub fn to_writer_with_config<W, T>(writer: W, value: &T, config: &SerializerConfig) -> Result<()>
where
    W: std::io::Write,
    T: ?Sized + serde_core::Serialize,
{
    let s = to_string_with_config(value, config)?;
    let mut writer = writer;
    writer.write_all(s.as_bytes())?;
    Ok(())
}

/// Serialize a Rust type to a YAML string with automatic anchor/alias emission
/// for shared `Rc` and `Arc` pointers wrapped in `RcAnchor` / `ArcAnchor`.
///
/// During this call, every `RcAnchor` / `ArcAnchor` whose pointer is seen for
/// the first time emits a YAML anchor (`&idNNN`); every subsequent sighting of
/// the same pointer emits an alias (`*idNNN`). This preserves true DAG
/// structure in the emitted document — `Rc::clone` siblings become alias
/// references instead of duplicated subtrees.
///
/// Pointer identity is tracked via a thread-local scratchpad that is installed
/// for the duration of the call and cleared on return. Plain `to_string`
/// behaviour is unaffected.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized to YAML.
///
/// # Examples
///
/// ```rust
/// use noyalib::{to_string_tracking_shared, RcAnchor};
/// use std::rc::Rc;
///
/// let shared: RcAnchor<String> = RcAnchor::from("hello".to_string());
/// let doc = vec![shared.clone(), shared.clone(), shared];
/// let yaml = to_string_tracking_shared(&doc).unwrap();
/// assert!(yaml.contains("&id001"));
/// assert!(yaml.contains("*id001"));
/// ```
#[cfg(feature = "std")]
pub fn to_string_tracking_shared<T>(value: &T) -> Result<String>
where
    T: ?Sized + serde_core::Serialize,
{
    to_string_tracking_shared_with_config(value, &SerializerConfig::default())
}

/// Serialize with automatic anchor/alias emission and a custom configuration.
///
/// See [`to_string_tracking_shared`] for behaviour.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized to YAML.
#[cfg(feature = "std")]
pub fn to_string_tracking_shared_with_config<T>(
    value: &T,
    config: &SerializerConfig,
) -> Result<String>
where
    T: ?Sized + serde_core::Serialize,
{
    let _scope = crate::anchors::shared_tracking::AnchorScope::enter();
    to_string_with_config(value, config)
}

/// Write YAML to a writer with automatic anchor/alias emission for shared
/// `Rc` / `Arc` pointers.
///
/// See [`to_string_tracking_shared`] for behaviour.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized or writing fails.
#[cfg(feature = "std")]
pub fn to_writer_tracking_shared<W, T>(writer: W, value: &T) -> Result<()>
where
    W: std::io::Write,
    T: ?Sized + serde_core::Serialize,
{
    to_writer_tracking_shared_with_config(writer, value, &SerializerConfig::default())
}

/// Write YAML to a writer with automatic anchor/alias emission and a custom
/// configuration.
///
/// See [`to_string_tracking_shared`] for behaviour.
///
/// # Errors
///
/// Returns an error if the type cannot be serialized or writing fails.
#[cfg(feature = "std")]
pub fn to_writer_tracking_shared_with_config<W, T>(
    writer: W,
    value: &T,
    config: &SerializerConfig,
) -> Result<()>
where
    W: std::io::Write,
    T: ?Sized + serde_core::Serialize,
{
    let s = to_string_tracking_shared_with_config(value, config)?;
    let mut writer = writer;
    writer.write_all(s.as_bytes())?;
    Ok(())
}

/// Serialize a Rust value to YAML and write into a [`core::fmt::Write`]
/// sink — the no_std-friendly counterpart to [`to_writer`].
///
/// # Errors
///
/// - `Error::Serialize` — the destination's `write_str` returned
///   `fmt::Error`, propagated through `Error::Serialize`.
/// - All variants documented on [`to_string`].
pub fn to_fmt_writer<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: fmt::Write,
    T: ?Sized + serde_core::Serialize,
{
    to_fmt_writer_with_config(writer, value, &SerializerConfig::default())
}

/// Serialize a Rust value to YAML and write into a [`core::fmt::Write`]
/// sink, using a custom [`SerializerConfig`].
///
/// # Errors
///
/// - `Error::Serialize` — the destination's `write_str` returned
///   `fmt::Error`.
/// - All variants documented on [`to_string_with_config`].
pub fn to_fmt_writer_with_config<W, T>(
    writer: &mut W,
    value: &T,
    config: &SerializerConfig,
) -> Result<()>
where
    W: fmt::Write,
    T: ?Sized + serde_core::Serialize,
{
    let s = to_string_with_config(value, config)?;
    writer
        .write_str(&s)
        .map_err(|e| Error::Serialize(e.to_string()))
}

/// Serialize a Rust value into a dynamic [`Value`] tree via the
/// Serde data model.
///
/// Use this when the typed serialise output should land in
/// noyalib's [`Value`] for further programmatic editing
/// (`Value::merge`, `Value::interpolate_properties`, …) before a
/// final emit.
///
/// **Note**: [`TaggedValue`]'s `Serialize` impl (and `Value::Tagged`'s own
/// inline serialize arm) route through `serialize_map` with a single
/// entry keyed by the tag string — the right shape for interop with a
/// generic serializer that has no YAML-tag concept, `serde_json` and
/// friends included. This crate's own [`Serializer`] *does* have a tag
/// concept: it recognises that single-entry, `!`-prefixed-key shape when
/// it builds the resulting [`Value`] and reconstructs [`Value::Tagged`],
/// so `to_value`/`to_string` on a `Value` containing `Tagged` round-trips
/// the tag rather than losing it to a degenerate one-entry mapping. Refs
/// #350. For direct emission of a `Value` you already hold, without going
/// through the `Serialize` pipeline at all, see [`to_string_value`] /
/// [`to_writer_value`].
///
/// # Errors
///
/// - `Error::Serialize` — `T`'s `Serialize` impl returned an
///   error.
/// - `Error::Custom` — surfaces upstream `serde_core::ser::Error`
///   conversions that don't fit the structured variants.
pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: ?Sized + serde_core::Serialize,
{
    // The public `to_value` / `to_string` family keeps
    // `T: ?Sized + serde_core::Serialize` so callers can serialise structs
    // holding borrowed references. `Value::Tagged`'s tag survives this path
    // via `SerializeMap::end`'s single-entry-map reconstruction (see its
    // doc comment); users who already hold a `Value` and want to skip the
    // `Serialize` pipeline entirely can still call [`to_string_value`] /
    // [`to_string_value_with_config`] / [`to_writer_value`].
    value.serialize(Serializer)
}

/// Serialize a [`Value`] directly to a YAML `String`, preserving
/// [`Value::Tagged`] shape losslessly.
///
/// This function bypasses the `Serialize` pipeline entirely and writes
/// the YAML-tag prefix directly. [`to_string`]/[`to_value`] also preserve
/// `Value::Tagged` when `T` is (or contains) a `Value` — see the note on
/// [`to_value`] — but this one skips the round trip through `Serializer`
/// altogether.
///
/// Use this whenever you hold a `Value` that may contain
/// `Value::Tagged` and want the emitted YAML to round-trip back
/// into an equivalent `Value::Tagged`.
///
/// # Errors
///
/// All variants documented on [`to_string`].
///
/// # Examples
///
/// ```
/// use noyalib::{from_str, to_string_value, Value};
/// let v: Value = from_str("!Color '#ff8800'\n").unwrap();
/// assert!(matches!(v, Value::Tagged(_)));
/// let s = to_string_value(&v).unwrap();
/// // Re-parsing yields an equivalent `Value::Tagged`.
/// let back: Value = from_str(&s).unwrap();
/// assert!(matches!(back, Value::Tagged(_)));
/// ```
pub fn to_string_value(value: &Value) -> Result<String> {
    value_to_string(value, &SerializerConfig::default())
}

/// Serialize a [`Value`] to a YAML `String` with a custom
/// [`SerializerConfig`], preserving [`Value::Tagged`] shape
/// losslessly. See [`to_string_value`] for the rationale.
///
/// # Errors
///
/// All variants documented on [`to_string_with_config`].
pub fn to_string_value_with_config(value: &Value, config: &SerializerConfig) -> Result<String> {
    value_to_string(value, config)
}

/// Write a [`Value`] to an [`std::io::Write`] sink, preserving
/// [`Value::Tagged`] shape losslessly. See [`to_string_value`]
/// for the rationale.
///
/// # Errors
///
/// - `Error::Io` — the underlying writer returned an I/O error.
/// - All variants documented on [`to_string`].
#[cfg(feature = "std")]
pub fn to_writer_value<W>(writer: W, value: &Value) -> Result<()>
where
    W: std::io::Write,
{
    to_writer_value_with_config(writer, value, &SerializerConfig::default())
}

/// Write a [`Value`] to an [`std::io::Write`] sink with a custom
/// [`SerializerConfig`], preserving [`Value::Tagged`] shape
/// losslessly. See [`to_string_value`] for the rationale.
///
/// # Errors
///
/// - `Error::Io` — the underlying writer returned an I/O error.
/// - All variants documented on [`to_string_with_config`].
#[cfg(feature = "std")]
pub fn to_writer_value_with_config<W>(
    writer: W,
    value: &Value,
    config: &SerializerConfig,
) -> Result<()>
where
    W: std::io::Write,
{
    let s = to_string_value_with_config(value, config)?;
    let mut writer = writer;
    writer.write_all(s.as_bytes())?;
    Ok(())
}

fn value_to_string(value: &Value, config: &SerializerConfig) -> Result<String> {
    let mut output = String::with_capacity(estimate_yaml_size(value));
    if config.document_start {
        output.push_str("---\n");
    }
    write_value(&mut output, value, 0, true, config, 0)?;
    if config.document_end {
        output.push_str("\n...");
    }
    Ok(output)
}

fn estimate_yaml_size(value: &Value) -> usize {
    match value {
        Value::Null => 4,
        Value::Bool(_) => 5,
        Value::Number(_) => 12,
        Value::String(s) => s.len() + 4,
        Value::Sequence(seq) => 4 + seq.iter().map(|v| estimate_yaml_size(v) + 4).sum::<usize>(),
        Value::Mapping(map) => {
            4 + map
                .iter()
                .map(|(k, v)| k.len() + estimate_yaml_size(v) + 6)
                .sum::<usize>()
        }
        Value::Tagged(t) => 20 + estimate_yaml_size(t.value()),
    }
}

/// Write `total_spaces` space characters to `output` without heap allocation.
#[inline]
fn write_indent(output: &mut String, total_spaces: usize) {
    const SPACES: &str = "                                                                ";
    // 64 spaces - covers indent up to depth 32 with indent=2
    let mut remaining = total_spaces;
    while remaining > 0 {
        let n = remaining.min(SPACES.len());
        output.push_str(&SPACES[..n]);
        remaining -= n;
    }
}

fn write_value(
    output: &mut String,
    value: &Value,
    indent: usize,
    is_root: bool,
    config: &SerializerConfig,
    depth: usize,
) -> Result<()> {
    if depth > config.max_depth {
        return Err(Error::RecursionLimitExceeded { depth });
    }
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        Value::Number(Number::Integer(n)) => {
            #[cfg(feature = "fast-int")]
            {
                let mut buf = itoa::Buffer::new();
                output.push_str(buf.format(*n));
            }
            #[cfg(not(feature = "fast-int"))]
            {
                let _ = write!(output, "{n}");
            }
        }
        #[cfg(feature = "lossless-u64")]
        Value::Number(Number::Unsigned(n)) => {
            #[cfg(feature = "fast-int")]
            {
                let mut buf = itoa::Buffer::new();
                output.push_str(buf.format(*n));
            }
            #[cfg(not(feature = "fast-int"))]
            {
                let _ = write!(output, "{n}");
            }
        }
        Value::Number(Number::Float(n)) => {
            // Shared with `Number`'s `Display` impl (see #348) so the
            // two never disagree on how a float prints.
            let _ = crate::value::write_float(output, *n);
        }
        Value::String(s) => write_string(output, s, indent, config),
        Value::Sequence(seq) => write_sequence(output, seq, indent, is_root, config, depth)?,
        Value::Mapping(map) => write_mapping(output, map, indent, is_root, config, depth)?,
        Value::Tagged(tagged) => {
            let tag_str = tagged.tag().as_str();
            if tag_str.starts_with("__noya_") {
                write_internal_tag(
                    output,
                    tag_str,
                    tagged.value(),
                    indent,
                    is_root,
                    config,
                    depth,
                )?;
            } else {
                // Write the tag, then its payload. A scalar payload sits on
                // this same line after a space (`!tag value`); a non-empty
                // mapping/sequence payload starts block layout on the next
                // line instead, with no trailing space after the tag (that
                // space would never be followed by anything, so it would
                // survive only as trailing whitespace). `indent` here is
                // already the slot this caller computed for the *whole*
                // tagged value via `needs_block_layout`/`indicator_takes_a_space`
                // above (see `write_mapping`/`write_sequence`), so the
                // payload is written at that same `indent`, not one deeper.
                // A tag whose body holds characters the shorthand
                // spelling cannot carry — flow indicators, blanks, or
                // an interior `!` (a handle separator there) — is
                // emitted in the verbatim form `!<...>`, which
                // re-parses to exactly the stored tag (`!<!str>` is
                // `!!str`). Emitting it raw produced YAML that split
                // at the first such byte: `!<tag:example.com,2026:x>`
                // re-emitted as shorthand died at the comma (found by
                // fuzz_roundtrip).
                // …and a tag body NO spelling can carry — a control
                // character (the scanner rejects those in shorthand
                // and verbatim forms alike; a tab is one) or a `>`
                // (verbatim's terminator, rejected in shorthand as a
                // non-URI char) — resolves the serde-model ambiguity
                // the other way: in the serde data model a tagged
                // value is indistinguishable from the single-entry
                // mapping keyed by its `!`-leading spelling, so emit
                // that mapping with a quoted key and let it re-parse
                // as what it is (found by fuzz_roundtrip on the key
                // `"!\t"`).
                if tag_str.bytes().any(|b| b < 0x20 || b == 0x7f || b == b'>') {
                    write_key_string(output, tag_str, indent, config);
                    output.push(':');
                    let inner = tagged.value();
                    if indicator_takes_a_space(inner) {
                        output.push(' ');
                    }
                    write_value(output, inner, indent, false, config, depth + 1)?;
                    return Ok(());
                }
                let shorthand_body = tag_str
                    .strip_prefix("!!")
                    .or_else(|| tag_str.strip_prefix('!'));
                let needs_verbatim = shorthand_body.is_some_and(|body| {
                    body.bytes().any(|b| {
                        matches!(b, b',' | b'[' | b']' | b'{' | b'}' | b'!' | b' ' | b'\t')
                    })
                });
                if needs_verbatim {
                    output.push_str("!<");
                    output.push_str(&tag_str[1..]);
                    output.push('>');
                } else {
                    output.push_str(tag_str);
                }
                let inner = tagged.value();
                if indicator_takes_a_space(inner) {
                    output.push(' ');
                }
                write_value(output, inner, indent, false, config, depth + 1)?;
            }
        }
    }
    Ok(())
}

/// Fast check whether a plain scalar would be interpreted as a number by a YAML
/// parser.  This is intentionally over-inclusive to ensure roundtrip safety —
/// it's cheaper to quote a few extra strings than to lose data.
fn looks_like_number(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    // YAML special float literals (case variants)
    if matches!(
        s,
        ".inf"
            | ".Inf"
            | ".INF"
            | "+.inf"
            | "+.Inf"
            | "+.INF"
            | "-.inf"
            | "-.Inf"
            | "-.INF"
            | ".nan"
            | ".NaN"
            | ".NAN"
    ) {
        return true;
    }

    // Skip any leading signs (yaml-rust2 is permissive with e.g. "++1")
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    if i >= bytes.len() {
        return false;
    }

    let rest = &bytes[i..];

    // A digit, or "." and a digit (floats like .5), can open a number.
    // That is the cheap filter; the parser's own resolver has the last
    // word, so digit-leading text it keeps as a string (`2026-12-31`,
    // `1.2.3`, `3rd`, `1/2`) is written plain, as the author wrote it.
    // The resolver sees the text after the signs: a permissive reader
    // (yaml-rust2 accepts `++1`) may take stacked signs as one, so what
    // follows them decides.
    let candidate =
        rest[0].is_ascii_digit() || (rest[0] == b'.' && rest.len() > 1 && rest[1].is_ascii_digit());
    candidate && resolves_as_non_string(&s[i..])
}

/// The parser's verdict on a plain scalar: would it read back as a
/// number, a boolean, or null rather than a string?
///
/// Runs the resolver the loaders and the streaming path share, with the
/// YAML 1.1 legacy forms enabled (`0`-prefixed octals, sexagesimals) so a
/// string is quoted whenever any reader configuration would turn it into
/// something else.
fn resolves_as_non_string(s: &str) -> bool {
    !matches!(
        crate::streaming::resolve_plain_ext(s, false, true, false, true, true, false),
        crate::streaming::Scalar::Str(_)
    )
}

/// A `:` ends a plain scalar only when a space, a tab, a flow indicator,
/// or the end of the text follows it (YAML 1.2 plain scalars), so
/// `word:count`, `10:00:00Z`, and `http://` stay plain.
fn colon_ends_plain(bytes: &[u8], i: usize) -> bool {
    match bytes.get(i + 1) {
        None => true,
        Some(&next) => matches!(next, b' ' | b'\t' | b',' | b'[' | b']' | b'{' | b'}'),
    }
}

/// A `#` starts a comment only at the start of the text or after
/// whitespace, so `a#b` stays plain.
fn hash_starts_comment(bytes: &[u8], i: usize) -> bool {
    i == 0 || matches!(bytes[i - 1], b' ' | b'\t')
}

/// Lookup table: true if the byte can require the string to be quoted.
/// Covers: control chars (except tab), colon, hash, newline, etc. A colon
/// or a hash is then judged in context by `colon_ends_plain` and
/// `hash_starts_comment`.
static NEEDS_QUOTE_BYTE: [bool; 128] = {
    let mut t = [false; 128];
    // Control characters (except tab 0x09)
    let mut i = 0u8;
    while i < 0x20 {
        if i != b'\t' {
            t[i as usize] = true;
        }
        i += 1;
    }
    // YAML structural characters
    t[b':' as usize] = true;
    t[b'#' as usize] = true;
    t[b'\n' as usize] = true;
    t[b'\r' as usize] = true;
    t[b'\0' as usize] = true;
    t
};

/// Characters that require quoting when they appear as the first character.
static FIRST_CHAR_QUOTE: [bool; 128] = {
    let mut t = [false; 128];
    t[b' ' as usize] = true;
    t[b'-' as usize] = true;
    t[b'&' as usize] = true;
    t[b'*' as usize] = true;
    t[b'!' as usize] = true;
    t[b'|' as usize] = true;
    t[b'>' as usize] = true;
    t[b'%' as usize] = true;
    t[b'@' as usize] = true;
    t[b'`' as usize] = true;
    t[b'{' as usize] = true;
    t[b'}' as usize] = true;
    t[b'[' as usize] = true;
    t[b']' as usize] = true;
    t[b',' as usize] = true;
    t[b'?' as usize] = true;
    t[b'\'' as usize] = true;
    t[b'"' as usize] = true;
    t
};

/// A character only double-quoted style can carry faithfully: CR, NEL
/// (U+0085), LS (U+2028), PS (U+2029) — and a BOM (U+FEFF). A literal
/// block scalar normalises `\r` into the block's own line breaks, and
/// the three Unicode separators pass through plain and single-quoted
/// styles as raw bytes that 1.1-era parsers (and this crate's own
/// reader) fold as line breaks, so a round trip changes the string
/// (#335). A raw BOM is worse: the reader must not accept one inside
/// a document at all (§5.2), and a string-leading BOM emitted plain
/// is stream-skipped on re-parse, reinterpreting the rest of the
/// scalar as markup (found by fuzz_roundtrip).
fn needs_double_quoted_escape(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(
            c,
            '\r' | '\u{0085}' | '\u{2028}' | '\u{2029}' | '\u{feff}' | '\u{7f}'
        ) || (c < '\u{20}' && c != '\t' && c != '\n')
    })
}

/// Write a mapping key. Keys are implicit (`key: value`) in every
/// form this serializer emits, and an implicit key must fit on one
/// line — the block scalar styles are not grammar there at all — so
/// a string the value writer would render as a `|`/`>` block (any
/// string holding a line break) is written double-quoted with
/// escapes instead. Found by fuzz_roundtrip: a multi-line key
/// emitted as a `|-` block produced YAML that no longer parsed
/// ("expected block mapping key or end").
fn write_key_string(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    if s.contains('\n') {
        write_double_quoted(output, s);
    } else {
        write_string(output, s, indent, config);
    }
}

fn write_string(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    let bytes = s.as_bytes();

    // Empty string must be quoted
    if bytes.is_empty() {
        if config.prefer_single_quotes {
            output.push_str("''");
        } else {
            output.push_str("\"\"");
        }
        return;
    }

    // Force-quote all strings when configured. Single-quoted style has
    // no escapes, so a string only double-quoted style can carry still
    // falls back regardless of the setting.
    if config.quote_all {
        if needs_double_quoted_escape(s) {
            write_double_quoted(output, s);
        } else {
            write_single_quoted(output, s);
        }
        return;
    }

    // A scalar starting with `...` emitted at the start of a line
    // reads back as the document-end marker (explicit-key emission
    // places keys at column 0), so it can never go plain — same
    // family as the `-` first-byte rule below, which already covers
    // `---` (found by fuzz_roundtrip on a `? ...` explicit key).
    if s.starts_with("...") {
        if needs_double_quoted_escape(s) {
            write_double_quoted(output, s);
        } else {
            write_single_quoted(output, s);
        }
        return;
    }

    // Fast path: short ASCII strings that are clearly safe as plain scalars.
    // Avoids the full lookup table scan for the majority of mapping keys.
    //
    // The intent is: short, alnum-bounded, no newline. All four conditions
    // below are ANDed — `||` binds looser than `&&`, and an earlier version
    // of this guard read `a && b && c && !config.block_scalars ||
    // no_newline`, which let *every* newline-free string take the fast path
    // regardless of its first byte (`"-"` slipped through unquoted and
    // re-parsed as a block sequence entry, not a scalar). The first/last
    // alnum checks already exclude every `FIRST_CHAR_QUOTE` member (none of
    // them are alphanumeric) and tab (also not alphanumeric), but the
    // explicit `FIRST_CHAR_QUOTE` check is kept here too as defense in
    // depth against the alnum check alone being loosened later.
    if bytes.len() <= 64
        && bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes.iter().all(|&b| b != b'\n')
        && !(bytes[0] < 128 && FIRST_CHAR_QUOTE[bytes[0] as usize])
    {
        let safe = bytes.iter().all(|&b| {
            b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.' || b == b'/'
        });
        if safe
            && !matches!(
                s,
                "true"
                    | "false"
                    | "null"
                    | "~"
                    | "True"
                    | "False"
                    | "TRUE"
                    | "FALSE"
                    | "Null"
                    | "NULL"
            )
            && !looks_like_number(s)
        {
            output.push_str(s);
            return;
        }
    }

    // Block scalar for multiline strings -- unless the string carries a
    // character a block scalar cannot represent: `str::lines` and the
    // block's own line breaks erase a `\r` (#335).
    if config.block_scalars && !needs_double_quoted_escape(s) {
        let newlines = bytes.iter().filter(|&&b| b == b'\n').count();
        if newlines >= config.block_scalar_threshold {
            write_block_scalar(output, s, indent, config);
            return;
        }
    }

    // CR and the Unicode line separators are representable only with
    // double-quoted escapes (#335).
    if needs_double_quoted_escape(s) {
        write_double_quoted(output, s);
        return;
    }

    // Single-pass quoting decision
    let mut needs_quotes = false;
    let mut has_control = false;

    // Check first character
    if bytes[0] < 128 && FIRST_CHAR_QUOTE[bytes[0] as usize] {
        needs_quotes = true;
    }

    // A leading or trailing tab must quote. `NEEDS_QUOTE_BYTE` deliberately
    // excludes tab so an *interior* tab stays unescaped in a plain scalar,
    // but YAML 1.2 still requires quoting when a plain scalar's content
    // starts or ends in white space (tab included), or the boundary is
    // lost on re-parse.
    if bytes[0] == b'\t' || bytes[bytes.len() - 1] == b'\t' {
        needs_quotes = true;
    }

    // Check last character (trailing space)
    if bytes[bytes.len() - 1] == b' ' {
        needs_quotes = true;
    }

    // Reserved words
    if !needs_quotes {
        needs_quotes = matches!(
            s,
            "true" | "false" | "null" | "~" | "True" | "False" | "TRUE" | "FALSE" | "Null" | "NULL"
        ) || looks_like_number(s);
    }

    // Single pass through interior bytes. A colon or a hash counts only
    // where YAML gives it meaning; see `colon_ends_plain` and
    // `hash_starts_comment`.
    if !needs_quotes {
        for (i, &b) in bytes.iter().enumerate() {
            if b >= 128 || !NEEDS_QUOTE_BYTE[b as usize] {
                continue;
            }
            if (b == b':' && !colon_ends_plain(bytes, i))
                || (b == b'#' && !hash_starts_comment(bytes, i))
            {
                continue;
            }
            if b < 0x20 && b != b'\t' {
                has_control = true;
            }
            needs_quotes = true;
            // Don't break - we need to know if there are control chars
        }
    }

    if !needs_quotes {
        // Plain scalar - zero-copy output
        output.push_str(s);
        return;
    }

    if config.prefer_single_quotes && single_quote_safe(s) {
        write_single_quoted(output, s);
        return;
    }

    // Use double quotes for all quoted strings
    let _ = has_control;
    write_double_quoted(output, s);
}

/// Whether `s` can be represented as a YAML single-quoted scalar with no
/// escapes beyond doubling an embedded `'`.
///
/// Single-quoted style has no escape mechanism at all besides doubling the
/// quote character itself — a literal backslash, double quote, `#`, `:`,
/// and so on all pass straight through unescaped. What it *cannot* carry is
/// a control character (tab, newline, carriage return, and the rest of the
/// C0/C1 ranges) or any other non-printable code point: those need one of
/// double-quoted style's escape sequences, so a string containing one must
/// fall back to double-quoted even when `prefer_single_quotes` is set.
fn single_quote_safe(s: &str) -> bool {
    !s.chars().any(char::is_control)
}

/// Write a single-quoted string, escaping embedded single quotes.
fn write_single_quoted(output: &mut String, s: &str) {
    output.push('\'');
    for c in s.chars() {
        if c == '\'' {
            output.push_str("''");
        } else {
            output.push(c);
        }
    }
    output.push('\'');
}

/// Write a double-quoted string with bulk-copy between escape points.
fn write_double_quoted(output: &mut String, s: &str) {
    output.push('"');
    let mut start = 0;
    for (i, c) in s.char_indices() {
        let esc = match c {
            '"' => "\\\"",
            '\\' => "\\\\",
            '\n' => "\\n",
            '\r' => "\\r",
            '\t' => "\\t",
            '\0' => "\\0",
            // Named escapes for the non-ASCII line-break characters
            // (YAML 1.2 section 5.7): emitted raw they read back as
            // line breaks in 1.1-era parsers and in this crate's own
            // reader (#335).
            '\u{0085}' => "\\N",
            '\u{2028}' => "\\L",
            '\u{2029}' => "\\P",
            // A raw BOM must never reach the output stream — the
            // reader rejects one inside a document (§5.2).
            '\u{feff}' => "\\uFEFF",
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                // Other control characters: flush and write hex escape
                output.push_str(&s[start..i]);
                let _ = write!(output, "\\x{:02X}", c as u32);
                start = i + 1;
                continue;
            }
            _ => continue,
        };
        output.push_str(&s[start..i]);
        output.push_str(esc);
        start = i + c.len_utf8();
    }
    output.push_str(&s[start..]);
    output.push('"');
}

/// Write a string using YAML literal block scalar style (|).
/// Write a block scalar's content lines at `indent + 1`.
///
/// An empty line gets **no** indentation. The block's indent is detected from
/// its first non-empty line, so an empty one has nothing to say; writing the
/// indent anyway leaves it standing as trailing whitespace on a line that
/// holds nothing, which `git diff --check` and `yamllint` reject.
///
/// This must stay one function. It had three identical copies — `|` auto, `|`
/// explicit and `>` — and the empty-line rule was missing from all three, so
/// fixing any one of them would have left the other two writing it.
fn write_block_scalar_body(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    for line in s.lines() {
        output.push('\n');
        if !line.is_empty() {
            write_indent(output, config.indent * (indent + 1));
            output.push_str(line);
        }
    }
}

/// The explicit indentation indicator digit(s), if the block needs one.
///
/// YAML 1.2.2 §8.1.1.1: a literal/folded block scalar's indentation is
/// normally *auto-detected* from its first non-empty content line. That
/// detection breaks when the first content line itself starts with a
/// space or a tab — the leading whitespace gets folded into the detected
/// indentation, inflating it past what later, less-indented lines carry,
/// which a parser then rejects as inconsistent indentation. The fix is an
/// explicit indentation indicator between the block style character
/// (`|`/`>`) and the chomping indicator, stating the indentation as a
/// number of columns *beyond the parent node's own indentation*.
///
/// [`write_block_scalar_body`] always places content `config.indent`
/// columns beyond the indentation this function's caller was handed for
/// this value's slot (the parent node's own indentation) — so whenever an
/// indicator is needed, its value is exactly `config.indent`, independent
/// of nesting depth.
fn block_scalar_indent_indicator(s: &str, config: &SerializerConfig) -> String {
    let first_content_line = s.lines().find(|line| !line.is_empty());
    match first_content_line {
        Some(line) if line.starts_with(' ') || line.starts_with('\t') => config.indent.to_string(),
        _ => String::new(),
    }
}

fn write_block_scalar(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    // Determine chomping indicator based on trailing newlines
    let chomping = if s.ends_with('\n') {
        if s.ends_with("\n\n") {
            "+" // Keep all trailing newlines
        } else {
            "" // Keep single trailing newline (default)
        }
    } else {
        "-" // Strip trailing newlines
    };

    output.push('|');
    output.push_str(&block_scalar_indent_indicator(s, config));
    output.push_str(chomping);

    write_block_scalar_body(output, s, indent, config);

    // `str::lines()` never yields an extra empty element for the string's
    // *final* line terminator, but every other trailing blank line DOES
    // get its own element (and so its own newline from the loop above).
    // That means the body loop always emits exactly one newline fewer
    // than `s` actually ends with, regardless of how many trailing
    // newlines there are: for `"text\n"` the loop emits none after
    // "text" (`.lines()` is just `["text"]`), for `"text\n\n"` it emits
    // one (`["text", ""]`), for `"text\n\n\n"` it emits two
    // (`["text", "", ""]`), and so on -- one behind `s`'s own count every
    // time. So exactly one more newline (never a count derived from `s`)
    // closes the gap.
    //
    // The previous version pushed `s.len() - s.trim_end_matches('\n').len()`
    // newlines here -- the *full* trailing-newline count -- which double
    // counted every trailing newline past the first and grew the string by
    // one extra `\n` on every serialize/parse round trip.
    if s.ends_with('\n') {
        output.push('\n');
    }
}

/// Whether a value can be safely rendered inside an `Auto`-mode flow
/// collection. A flow collection may only contain scalars and other flow
/// collections — a nested *block* collection would produce invalid YAML
/// (`[a, - x]`). So in `Auto` mode the whole subtree must stay within the
/// flow threshold for any ancestor to flow; otherwise we fall back to block.
///
/// `Tagged` values are conservatively treated as block-only: they carry
/// anchors, custom tags, and the internal block-scalar/anchor magic tags that
/// have no valid flow representation here.
fn auto_flow_eligible(value: &Value, config: &SerializerConfig) -> bool {
    match value {
        Value::Sequence(s) => {
            s.len() <= config.flow_threshold && s.iter().all(|v| auto_flow_eligible(v, config))
        }
        Value::Mapping(m) => {
            m.len() <= config.flow_threshold && m.iter().all(|(_, v)| auto_flow_eligible(v, config))
        }
        Value::Tagged(_) => false,
        _ => true,
    }
}

/// Decide whether a collection of `len` items holding `values` should render
/// in flow style under the active `config`.
fn use_flow<'a, I>(len: usize, values: impl Fn() -> I, config: &SerializerConfig) -> bool
where
    I: Iterator<Item = &'a Value>,
{
    match config.flow_style {
        FlowStyle::Block => false,
        FlowStyle::Flow => true,
        FlowStyle::Auto => {
            len <= config.flow_threshold && values().all(|v| auto_flow_eligible(v, config))
        }
    }
}

fn write_sequence(
    output: &mut String,
    seq: &Sequence,
    indent: usize,
    is_root: bool,
    config: &SerializerConfig,
    depth: usize,
) -> Result<()> {
    if seq.is_empty() {
        output.push_str("[]");
        return Ok(());
    }

    if use_flow(seq.len(), || seq.iter(), config) {
        return write_flow_sequence(output, seq, config, depth);
    }

    for (i, value) in seq.iter().enumerate() {
        if i > 0 || !is_root {
            output.push('\n');
            write_indent(output, config.indent * indent);
        }
        output.push('-');

        match value {
            Value::Mapping(m) if !m.is_empty() => {
                // The item's first key shares the dash's line, so the dash
                // always takes its space here whatever the *value* looks
                // like — `- key: 1` and `- key:` alike.
                output.push(' ');
                for (j, (k, v)) in m.iter().enumerate() {
                    if j > 0 {
                        output.push('\n');
                        write_indent(output, config.indent * (indent + 1));
                    }
                    write_key_string(output, k, indent + 1, config);
                    output.push(':');
                    if indicator_takes_a_space(v) {
                        output.push(' ');
                    }
                    // `compact_list_indent`: a sequence value starts at its
                    // own key's indentation (`indent + 1`, matching `k`'s
                    // own column) rather than one level deeper — the same
                    // rule `write_mapping` applies, extended to a mapping
                    // that is itself a sequence item. Every other
                    // block-layout value (a mapping, or a sequence with the
                    // option off) still gets the extra level.
                    let next_indent = if needs_block_layout(v) {
                        if config.compact_list_indent && matches!(v, Value::Sequence(_)) {
                            indent + 1
                        } else {
                            indent + 2
                        }
                    } else {
                        indent + 1
                    };
                    write_value(output, v, next_indent, false, config, depth + 1)?;
                }
            }
            Value::Sequence(inner) if config.compact_list_indent && !inner.is_empty() => {
                // `compact_list_indent`: a sequence item that is itself a
                // sequence is written inline (`- - a`), with the nested
                // dash sharing this item's own dash's line the same way a
                // nested mapping's first key does above. Continuation
                // elements align with that nested dash (`indent + 1`).
                output.push(' ');
                write_sequence(output, inner, indent + 1, true, config, depth + 1)?;
            }
            _ => {
                if indicator_takes_a_space(value) {
                    output.push(' ');
                }
                write_value(output, value, indent + 1, false, config, depth + 1)?;
            }
        }
    }
    Ok(())
}

/// Whether the indicator introducing `value` — the `:` after a key, or a
/// sequence item's `-` — must be followed by a space.
///
/// An inline scalar needs one (`key: 1`, `- 1`). A block collection does not:
/// it begins on the *next* line, so the space would be left dangling at the
/// end of this one. That is invisible, it re-parses identically, and it is
/// exactly what `git diff --check` and `yamllint`'s `trailing-spaces` reject.
///
/// The exception is an anchor-wrapped block value, which renders as
/// `&idNNN\n  ...`: the `&` is on *this* line, so the space is real
/// separation rather than leftovers. A regular (user) tag is the same
/// story: `!tag\n  ...` also has visible text — the tag name — on this
/// line, so the space is real separation there too.
///
/// [`write_mapping`] has always applied this rule; [`write_sequence`] carried
/// its own copy of the key-writing and did not, which is the whole of the bug
/// this function exists to stop recurring. One rule, one place, both callers.
fn indicator_takes_a_space(value: &Value) -> bool {
    !needs_block_layout(value)
        || matches!(
            value,
            Value::Tagged(t) if t.tag().as_str() == crate::fmt::MAGIC_ANCHOR_DEF
                || !t.tag().as_str().starts_with("__noya_")
        )
}

/// Whether a value needs block-style layout (indented on the line after `:`)
/// rather than inline scalar layout. Anchor-wrapped block collections must be
/// treated like the inner collection; a regular (user) tag likewise needs
/// block layout exactly when *its* payload does (a tagged mapping/sequence
/// under a key must be indented one level deeper, the same as an untagged
/// one — see [`write_value`]'s `Value::Tagged` arm).
fn needs_block_layout(v: &Value) -> bool {
    match v {
        Value::Mapping(m) => !m.is_empty(),
        Value::Sequence(s) => !s.is_empty(),
        Value::Tagged(t) if t.tag().as_str() == crate::fmt::MAGIC_ANCHOR_DEF => {
            if let Value::Sequence(seq) = t.value() {
                if seq.len() == 2 {
                    return needs_block_layout(&seq[1]);
                }
            }
            false
        }
        Value::Tagged(t) if !t.tag().as_str().starts_with("__noya_") => {
            needs_block_layout(t.value())
        }
        _ => false,
    }
}

fn write_mapping(
    output: &mut String,
    map: &Mapping,
    indent: usize,
    is_root: bool,
    config: &SerializerConfig,
    depth: usize,
) -> Result<()> {
    if map.is_empty() {
        output.push_str("{}");
        return Ok(());
    }

    if use_flow(map.len(), || map.iter().map(|(_, v)| v), config) {
        return write_flow_mapping(output, map, config, depth);
    }

    for (i, (key, value)) in map.iter().enumerate() {
        if i > 0 || !is_root {
            output.push('\n');
            write_indent(output, config.indent * indent);
        }
        write_key_string(output, key, indent, config);

        output.push(':');
        if indicator_takes_a_space(value) {
            output.push(' ');
        }
        let next_indent = if needs_block_layout(value) {
            // `compact_list_indent`: when on, sequence values
            // under a mapping key align with the key column
            // instead of being bumped one indent level deeper.
            // This is the visual style preferred by some style
            // guides (Kubernetes manifests, GitHub Actions
            // workflows). Mappings and other non-sequence block
            // values keep the standard indent.
            if config.compact_list_indent && matches!(value, Value::Sequence(_)) {
                indent
            } else {
                indent + 1
            }
        } else {
            indent
        };
        write_value(output, value, next_indent, false, config, depth + 1)?;
    }
    Ok(())
}

fn write_internal_tag(
    output: &mut String,
    tag: &str,
    value: &Value,
    indent: usize,
    is_root: bool,
    config: &SerializerConfig,
    depth: usize,
) -> Result<()> {
    match tag {
        crate::fmt::MAGIC_FLOW_SEQ => {
            if let Value::Sequence(seq) = value {
                write_flow_sequence(output, seq, config, depth)?;
            } else {
                write_value(output, value, indent, is_root, config, depth)?;
            }
        }
        crate::fmt::MAGIC_FLOW_MAP => {
            if let Value::Mapping(map) = value {
                write_flow_mapping(output, map, config, depth)?;
            } else {
                write_value(output, value, indent, is_root, config, depth)?;
            }
        }
        crate::fmt::MAGIC_LIT_STR => {
            if let Value::String(s) = value {
                write_literal_block(output, s, indent, config);
            } else {
                write_value(output, value, indent, is_root, config, depth)?;
            }
        }
        crate::fmt::MAGIC_FOLD_STR => {
            if let Value::String(s) = value {
                write_folded_block(output, s, indent, config);
            } else {
                write_value(output, value, indent, is_root, config, depth)?;
            }
        }
        crate::fmt::MAGIC_COMMENTED => {
            // value is a sequence [inner_value, comment_string]
            if let Value::Sequence(seq) = value {
                if seq.len() == 2 {
                    write_value(output, &seq[0], indent, is_root, config, depth)?;
                    if let Value::String(comment) = &seq[1] {
                        output.push_str(" # ");
                        output.push_str(comment);
                    }
                } else {
                    write_value(output, value, indent, is_root, config, depth)?;
                }
            } else {
                write_value(output, value, indent, is_root, config, depth)?;
            }
        }
        crate::fmt::MAGIC_SPACE_AFTER => {
            write_value(output, value, indent, is_root, config, depth)?;
            output.push('\n');
        }
        crate::fmt::MAGIC_ANCHOR_DEF => {
            // value is a sequence [String(id), inner_value]. Emit "&id" before
            // the inner value. For block collections the inner starts on a new
            // line; for scalars it follows on the same line.
            if let Value::Sequence(seq) = value {
                if seq.len() == 2 {
                    if let Value::String(id) = &seq[0] {
                        let inner = &seq[1];
                        output.push('&');
                        output.push_str(id);
                        match inner {
                            Value::Mapping(m) if !m.is_empty() => {
                                output.push('\n');
                                write_indent(output, config.indent * indent);
                                // `is_root = true` suppresses the leading newline
                                // inside write_mapping so the anchor line and
                                // the first key are correctly adjacent.
                                write_mapping(output, m, indent, true, config, depth + 1)?;
                            }
                            Value::Sequence(s) if !s.is_empty() => {
                                output.push('\n');
                                write_indent(output, config.indent * indent);
                                write_sequence(output, s, indent, true, config, depth + 1)?;
                            }
                            _ => {
                                output.push(' ');
                                write_value(output, inner, indent, false, config, depth + 1)?;
                            }
                        }
                    }
                }
            }
        }
        crate::fmt::MAGIC_ANCHOR_REF => {
            // value is String(id). Emit "*id".
            if let Value::String(id) = value {
                output.push('*');
                output.push_str(id);
            }
        }
        _ => {
            // Unknown internal tag — fall through to regular output
            write_value(output, value, indent, is_root, config, depth)?;
        }
    }
    Ok(())
}

fn write_flow_sequence(
    output: &mut String,
    seq: &Sequence,
    config: &SerializerConfig,
    depth: usize,
) -> Result<()> {
    output.push('[');
    for (i, value) in seq.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        write_value(output, value, 0, false, config, depth + 1)?;
    }
    output.push(']');
    Ok(())
}

fn write_flow_mapping(
    output: &mut String,
    map: &Mapping,
    config: &SerializerConfig,
    depth: usize,
) -> Result<()> {
    output.push('{');
    for (i, (key, value)) in map.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        write_key_string(output, key, 0, config);
        output.push_str(": ");
        write_value(output, value, 0, false, config, depth + 1)?;
    }
    output.push('}');
    Ok(())
}

fn write_literal_block(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    let chomping = if s.ends_with('\n') {
        if s.ends_with("\n\n") { "+" } else { "" }
    } else {
        "-"
    };

    output.push('|');
    output.push_str(&block_scalar_indent_indicator(s, config));
    output.push_str(chomping);

    write_block_scalar_body(output, s, indent, config);
}

fn write_folded_block(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    let chomping = if s.ends_with('\n') {
        if s.ends_with("\n\n") { "+" } else { "" }
    } else {
        "-"
    };

    output.push('>');
    output.push_str(&block_scalar_indent_indicator(s, config));
    output.push_str(chomping);

    write_block_scalar_body(output, s, indent, config);
}

/// Serialize an iterable of values as a multi-document YAML
/// string with `---` document-start markers between each.
///
/// # Errors
///
/// All variants documented on [`to_string`]; the first failing
/// document short-circuits and returns its error — earlier
/// documents are not emitted.
///
/// # Examples
///
/// ```rust
/// let docs = vec![1, 2, 3];
/// let yaml = noyalib::to_string_multi(&docs).unwrap();
/// assert!(yaml.contains("---"));
/// ```
pub fn to_string_multi<T: serde_core::Serialize>(values: &[T]) -> Result<String> {
    to_string_multi_with_config(values, &SerializerConfig::default())
}

/// Serialize an iterable of values as a multi-document YAML
/// string with a custom [`SerializerConfig`].
///
/// # Errors
///
/// All variants documented on [`to_string_with_config`].
pub fn to_string_multi_with_config<T: serde_core::Serialize>(
    values: &[T],
    config: &SerializerConfig,
) -> Result<String> {
    let mut output = String::new();
    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        output.push_str("---\n");
        let v = to_value(value)?;
        write_value(&mut output, &v, 0, true, config, 0)?;
        output.push('\n');
    }
    Ok(output)
}

/// Serialize multiple values as multi-document YAML to a writer.
///
/// # Errors
///
/// Returns an error if any value cannot be serialized or writing fails.
#[cfg(feature = "std")]
pub fn to_writer_multi<W, T>(writer: W, values: &[T]) -> Result<()>
where
    W: std::io::Write,
    T: serde_core::Serialize,
{
    to_writer_multi_with_config(writer, values, &SerializerConfig::default())
}

/// Serialize multiple values as multi-document YAML to a writer with custom
/// configuration.
///
/// # Errors
///
/// Returns an error if any value cannot be serialized or writing fails.
#[cfg(feature = "std")]
pub fn to_writer_multi_with_config<W, T>(
    writer: W,
    values: &[T],
    config: &SerializerConfig,
) -> Result<()>
where
    W: std::io::Write,
    T: serde_core::Serialize,
{
    let s = to_string_multi_with_config(values, config)?;
    let mut writer = writer;
    writer.write_all(s.as_bytes())?;
    Ok(())
}

/// A YAML serializer.
#[derive(Debug, Copy, Clone)]
pub struct Serializer;

impl serde_core::ser::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeSeq;
    type SerializeTuple = SerializeSeq;
    type SerializeTupleStruct = SerializeSeq;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Value> {
        Ok(Value::Number(Number::Integer(v)))
    }

    fn serialize_u8(self, v: u8) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Value> {
        if let Ok(v) = i64::try_from(v) {
            return Ok(Value::Number(Number::Integer(v)));
        }
        // Values above `i64::MAX` require the `lossless-u64` feature.
        // Without it the `Number::Unsigned` variant does not exist and
        // there is no lossless representation to fall back to — return
        // an explicit serialise-time error so callers can surface the
        // limit to their users.
        #[cfg(feature = "lossless-u64")]
        {
            Ok(Value::Number(Number::Unsigned(v)))
        }
        #[cfg(not(feature = "lossless-u64"))]
        {
            Err(Error::Serialize(format!(
                "u64 value {v} exceeds i64::MAX and cannot be represented losslessly; \
                 enable the `lossless-u64` Cargo feature to opt in to unsigned integer support"
            )))
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Value> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Value> {
        Ok(Value::Number(Number::Float(v)))
    }

    fn serialize_char(self, v: char) -> Result<Value> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::String(v.to_owned()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
        // YAML 1.2.2 §10.4: byte buffers serialise as a `!!binary`
        // tagged scalar carrying the RFC 4648 base64 encoding of
        // the payload. This is the round-trip partner of the
        // `deserialize_bytes` path that recognises `!!binary` and
        // base64-decodes on demand. Holds for any `serde_bytes`
        // wrapper (`ByteBuf`, `Bytes`) and any `&[u8]` /
        // `Vec<u8>`-shaped target the caller annotates with
        // `#[serde(with = "serde_bytes")]`.
        let encoded = crate::base64::encode(v);
        Ok(Value::Tagged(Box::new(TaggedValue::new(
            Tag::new("!!binary"),
            Value::String(encoded),
        ))))
    }

    fn serialize_none(self) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + serde_core::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Value>
    where
        T: ?Sized + serde_core::Serialize,
    {
        // Intercept formatting hint magic names
        match name {
            crate::fmt::MAGIC_FLOW_SEQ
            | crate::fmt::MAGIC_FLOW_MAP
            | crate::fmt::MAGIC_LIT_STR
            | crate::fmt::MAGIC_FOLD_STR
            | crate::fmt::MAGIC_SPACE_AFTER => {
                let inner = value.serialize(Self)?;
                Ok(Value::Tagged(Box::new(TaggedValue::new(
                    Tag::new(name),
                    inner,
                ))))
            }
            crate::fmt::MAGIC_COMMENTED => {
                // value is a tuple (inner_value, comment_string)
                let inner = value.serialize(Self)?;
                Ok(Value::Tagged(Box::new(TaggedValue::new(
                    Tag::new(name),
                    inner,
                ))))
            }
            crate::fmt::MAGIC_ANCHOR_DEF | crate::fmt::MAGIC_ANCHOR_REF => {
                // ANCHOR_DEF: value serializes as Sequence([String(id), inner]).
                // ANCHOR_REF: value serializes as String(id).
                let inner = value.serialize(Self)?;
                Ok(Value::Tagged(Box::new(TaggedValue::new(
                    Tag::new(name),
                    inner,
                ))))
            }
            _ => value.serialize(self),
        }
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + serde_core::Serialize,
    {
        let mut map = Mapping::new();
        let _ = map.insert(variant.to_owned(), value.serialize(Self)?);
        Ok(Value::Mapping(map))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeSeq {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(SerializeTupleVariant {
            name: variant.to_owned(),
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeMap {
            map: Mapping::new(),
            key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(SerializeStructVariant {
            name: variant.to_owned(),
            map: Mapping::new(),
        })
    }
}

/// Serializer for sequences.
#[derive(Debug)]
pub struct SerializeSeq {
    vec: Vec<Value>,
}

impl serde_core::ser::SerializeSeq for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde_core::Serialize,
    {
        self.vec.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Sequence(self.vec))
    }
}

impl serde_core::ser::SerializeTuple for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde_core::Serialize,
    {
        serde_core::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        serde_core::ser::SerializeSeq::end(self)
    }
}

impl serde_core::ser::SerializeTupleStruct for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde_core::Serialize,
    {
        serde_core::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        serde_core::ser::SerializeSeq::end(self)
    }
}

/// Serializer for tuple variants.
#[derive(Debug)]
pub struct SerializeTupleVariant {
    name: String,
    vec: Vec<Value>,
}

impl serde_core::ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde_core::Serialize,
    {
        self.vec.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut map = Mapping::new();
        let _ = map.insert(self.name, Value::Sequence(self.vec));
        Ok(Value::Mapping(map))
    }
}

/// Serializer for maps.
#[derive(Debug)]
pub struct SerializeMap {
    map: Mapping,
    key: Option<String>,
}

impl serde_core::ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + serde_core::Serialize,
    {
        let key_value = key.serialize(Serializer)?;
        let key_str = match key_value {
            Value::String(s) => s,
            Value::Number(Number::Integer(n)) => n.to_string(),
            #[cfg(feature = "lossless-u64")]
            Value::Number(Number::Unsigned(n)) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => return Err(Error::Serialize("map key must be a string".to_string())),
        };
        self.key = Some(key_str);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde_core::Serialize,
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error::Serialize("missing key".to_string()))?;
        let _ = self.map.insert(key, value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        // `TaggedValue::serialize` (and `Value::Tagged`'s own inline
        // serialize arm) route through this exact `serialize_map(Some(1))`
        // + one `serialize_entry` shape -- that single-entry-map wire form
        // is the documented, unchanged shape for interop with a generic
        // serializer that has no YAML-tag concept (`serde_json` and
        // friends). Our own serializer *does* have a tag concept, so
        // recognise that shape here and reconstruct `Value::Tagged`
        // instead of losing the tag to a degenerate one-entry mapping.
        // Refs #350.
        let is_tag_shaped = self.map.len() == 1
            && self
                .map
                .iter()
                .next()
                .is_some_and(|(k, _)| k.starts_with('!'));
        if is_tag_shaped {
            let (key, value) = self
                .map
                .into_iter()
                .next()
                .expect("is_tag_shaped confirmed exactly one entry");
            return Ok(Value::Tagged(Box::new(TaggedValue::new(
                Tag::new(key),
                value,
            ))));
        }
        Ok(Value::Mapping(self.map))
    }
}

impl serde_core::ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + serde_core::Serialize,
    {
        let _ = self
            .map
            .insert(key.to_owned(), value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Mapping(self.map))
    }
}

/// Serializer for struct variants.
#[derive(Debug)]
pub struct SerializeStructVariant {
    name: String,
    map: Mapping,
}

impl serde_core::ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + serde_core::Serialize,
    {
        let _ = self
            .map
            .insert(key.to_owned(), value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut map = Mapping::new();
        let _ = map.insert(self.name, Value::Mapping(self.map));
        Ok(Value::Mapping(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn test_serialization_recursion_limit() {
        let mut root = Value::Sequence(vec![Value::Null]);
        for _ in 0..200 {
            root = Value::Sequence(vec![root]);
        }

        let config = SerializerConfig::default().max_depth(128);
        let result = to_string_with_config(&root, &config);

        match result {
            Err(Error::RecursionLimitExceeded { depth }) => assert!(depth > 128),
            _ => panic!("Expected RecursionLimitExceeded error, got {result:?}"),
        }
    }

    // Regression for #84: `flow_style` was stored but never consulted by the
    // emit path, so `FlowStyle::Flow` / `Auto` silently produced block output.
    #[test]
    fn test_flow_style_flow_emits_inline_collections() {
        let seq = Value::Sequence(vec![
            Value::from(0),
            Value::from(1),
            Value::from(2),
            Value::from(3),
            Value::from(4),
        ]);
        let mut map = Mapping::new();
        let _ = map.insert("a", Value::from(1));
        let _ = map.insert("b", Value::from(2));
        let _ = map.insert("c", Value::from(3));
        let map = Value::Mapping(map);

        let config = SerializerConfig::new().flow_style(FlowStyle::Flow);
        assert_eq!(
            to_string_with_config(&seq, &config).unwrap().trim_end(),
            "[0, 1, 2, 3, 4]"
        );
        assert_eq!(
            to_string_with_config(&map, &config).unwrap().trim_end(),
            "{a: 1, b: 2, c: 3}"
        );
    }

    #[test]
    fn test_flow_style_auto_respects_threshold() {
        let small = Value::Sequence((0..3).map(Value::from).collect());
        let large = Value::Sequence((0..10).map(Value::from).collect());

        let config = SerializerConfig::new()
            .flow_style(FlowStyle::Auto)
            .flow_threshold(4);

        // Small collection (<= threshold) flows inline.
        assert_eq!(
            to_string_with_config(&small, &config).unwrap().trim_end(),
            "[0, 1, 2]"
        );
        // Large collection (> threshold) stays block.
        assert!(
            to_string_with_config(&large, &config)
                .unwrap()
                .starts_with("- 0")
        );
    }

    #[test]
    fn test_flow_style_auto_falls_back_when_child_exceeds_threshold() {
        // Outer has 2 items (<= threshold) but the inner sequence has 10
        // (> threshold). Flowing the outer would emit an invalid block child
        // inside flow, so Auto must keep the outer in block style.
        let inner = Value::Sequence((0..10).map(Value::from).collect());
        let outer = Value::Sequence(vec![Value::from(0), inner]);

        let config = SerializerConfig::new()
            .flow_style(FlowStyle::Auto)
            .flow_threshold(4);
        let out = to_string_with_config(&outer, &config).unwrap();
        assert!(out.starts_with("- 0"), "outer should stay block: {out:?}");
    }

    #[test]
    fn test_flow_style_block_is_default_unchanged() {
        let seq = Value::Sequence((0..3).map(Value::from).collect());
        // Default config (Block) and the no-config helper both stay block.
        assert!(to_string(&seq).unwrap().starts_with("- 0"));
        assert!(
            to_string_with_config(&seq, &SerializerConfig::new())
                .unwrap()
                .starts_with("- 0")
        );
    }
}
