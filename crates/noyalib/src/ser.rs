//! YAML serialization.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::fmt::Write as _;

use serde::ser::{self, Serialize};

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
///   error (custom `serde::ser::Error`, non-string mapping key
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
/// use serde::Serialize;
///
/// #[derive(Serialize)]
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
    T: ?Sized + Serialize,
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
/// use serde::Serialize;
///
/// #[derive(Serialize)]
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
    T: ?Sized + Serialize,
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
    T: ?Sized + Serialize,
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
    T: ?Sized + Serialize,
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
    T: ?Sized + Serialize,
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
    T: ?Sized + Serialize,
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
    T: ?Sized + Serialize,
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
    T: ?Sized + Serialize,
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
    T: ?Sized + Serialize,
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
    T: ?Sized + Serialize,
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
/// **Note**: when `value` is itself a [`Value`] containing
/// [`Value::Tagged`], the round-trip through `serialize` →
/// `Value` is *lossy* on the tag — the standard `Serialize`
/// pipeline routes `Tagged` through `serialize_map` (which is
/// the right shape for serde-bridge interop with `serde_json`
/// etc.) and the YAML-tag wire form is lost. Use
/// [`to_string_value`] / [`to_writer_value`] for tag-preserving
/// emission of a `Value` that may contain `Tagged`.
///
/// # Errors
///
/// - `Error::Serialize` — `T`'s `Serialize` impl returned an
///   error.
/// - `Error::Custom` — surfaces upstream `serde::ser::Error`
///   conversions that don't fit the structured variants.
pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: ?Sized + Serialize,
{
    // No tag-preserving fast-path here: the public `to_value` /
    // `to_string` family keeps `T: ?Sized + Serialize` so callers
    // can serialise structs holding borrowed references. Users
    // holding a [`Value`] who want lossless `Value::Tagged`
    // round-trip should call [`to_string_value`] /
    // [`to_string_value_with_config`] / [`to_writer_value`] —
    // those skip the `Serialize` pipeline entirely.
    value.serialize(Serializer)
}

/// Serialize a [`Value`] directly to a YAML `String`, preserving
/// [`Value::Tagged`] shape losslessly.
///
/// Going through the generic `to_string<T: Serialize>` path
/// routes `Value::Tagged(...)` through `Serializer::serialize_map`
/// (which emits a single-entry mapping for serde-bridge interop),
/// which loses the YAML-tag wire form. This function bypasses the
/// `Serialize` pipeline and writes the YAML-tag prefix directly.
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
            if n.is_nan() {
                output.push_str(".nan");
            } else if n.is_infinite() {
                if *n > 0.0 {
                    output.push_str(".inf");
                } else {
                    output.push_str("-.inf");
                }
            } else {
                #[cfg(feature = "fast-float")]
                {
                    let mut buf = ryu::Buffer::new();
                    output.push_str(buf.format(*n));
                }
                #[cfg(not(feature = "fast-float"))]
                {
                    // `{:?}` preserves float-ness for whole numbers
                    // (`1.0` not `1`) so the YAML round-trips back as
                    // `Number::Float`. Slower than ryu and emits
                    // expanded decimal form for very large magnitudes,
                    // but correct.
                    let _ = write!(output, "{n:?}");
                }
            }
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
                // Write tag followed by value
                output.push_str(tag_str);
                output.push(' ');
                write_value(output, tagged.value(), indent, false, config, depth + 1)?;
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

    // Anything starting with a digit could be interpreted as numeric.
    if rest[0].is_ascii_digit() {
        return true;
    }
    // "." followed by digit: floats like .5
    if rest[0] == b'.' && rest.len() > 1 && rest[1].is_ascii_digit() {
        return true;
    }

    false
}

/// Lookup table: true if the byte requires the string to be quoted.
/// Covers: control chars (except tab), colon, hash, newline, etc.
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

fn write_string(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    let bytes = s.as_bytes();

    // Empty string must be quoted
    if bytes.is_empty() {
        output.push_str("\"\"");
        return;
    }

    // Force-quote all strings when configured
    if config.quote_all {
        write_single_quoted(output, s);
        return;
    }

    // Fast path: short ASCII strings that are clearly safe as plain scalars.
    // Avoids the full lookup table scan for the majority of mapping keys.
    if bytes.len() <= 64
        && bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && !config.block_scalars
        || bytes.iter().all(|&b| b != b'\n')
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

    // Block scalar for multiline strings
    if config.block_scalars {
        let newlines = bytes.iter().filter(|&&b| b == b'\n').count();
        if newlines >= config.block_scalar_threshold {
            write_block_scalar(output, s, indent, config);
            return;
        }
    }

    // Single-pass quoting decision
    let mut needs_quotes = false;
    let mut has_control = false;

    // Check first character
    if bytes[0] < 128 && FIRST_CHAR_QUOTE[bytes[0] as usize] {
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

    // Single pass through interior bytes
    if !needs_quotes {
        for &b in bytes {
            if b < 128 && NEEDS_QUOTE_BYTE[b as usize] {
                if b < 0x20 && b != b'\t' {
                    has_control = true;
                }
                needs_quotes = true;
                // Don't break - we need to know if there are control chars
            }
        }
    }

    if !needs_quotes {
        // Plain scalar - zero-copy output
        output.push_str(s);
        return;
    }

    // Use double quotes for all quoted strings
    let _ = has_control;
    write_double_quoted(output, s);
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
    let bytes = s.as_bytes();
    let mut start = 0;
    for (i, &b) in bytes.iter().enumerate() {
        let esc = match b {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            b'\n' => "\\n",
            b'\r' => "\\r",
            b'\t' => "\\t",
            b'\0' => "\\0",
            c if c < 0x20 && c != b'\t' => {
                // Other control characters: flush and write hex escape
                output.push_str(&s[start..i]);
                let _ = write!(output, "\\x{c:02X}");
                start = i + 1;
                continue;
            }
            _ => continue,
        };
        output.push_str(&s[start..i]);
        output.push_str(esc);
        start = i + 1;
    }
    output.push_str(&s[start..]);
    output.push('"');
}

/// Write a string using YAML literal block scalar style (|).
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
    output.push_str(chomping);

    for line in s.lines() {
        output.push('\n');
        write_indent(output, config.indent * (indent + 1));
        output.push_str(line);
    }

    // s.lines() does not yield trailing empty lines, so we must emit them
    // for the "keep" (+) chomping mode to roundtrip correctly.
    if s.ends_with('\n') {
        // Count trailing newlines
        let trailing = s.len() - s.trim_end_matches('\n').len();
        // lines() already omits one trailing newline in default mode,
        // so for "+" mode we need to emit all trailing newlines explicitly.
        for _ in 0..trailing {
            output.push('\n');
        }
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
        output.push_str("- ");

        match value {
            Value::Mapping(m) if !m.is_empty() => {
                // Write first key-value on same line as dash
                let mut iter = m.iter();
                if let Some((k, v)) = iter.next() {
                    write_string(output, k, indent + 1, config);
                    output.push_str(": ");
                    if matches!(v, Value::Mapping(_) | Value::Sequence(_)) {
                        write_value(output, v, indent + 2, false, config, depth + 1)?;
                    } else {
                        write_value(output, v, indent + 1, false, config, depth + 1)?;
                    }
                }
                // Write remaining key-values
                for (k, v) in iter {
                    output.push('\n');
                    write_indent(output, config.indent * (indent + 1));
                    write_string(output, k, indent + 1, config);
                    output.push_str(": ");
                    if matches!(v, Value::Mapping(_) | Value::Sequence(_)) {
                        write_value(output, v, indent + 2, false, config, depth + 1)?;
                    } else {
                        write_value(output, v, indent + 1, false, config, depth + 1)?;
                    }
                }
            }
            Value::Sequence(_) => {
                write_value(output, value, indent + 1, false, config, depth + 1)?;
            }
            _ => {
                write_value(output, value, indent + 1, false, config, depth + 1)?;
            }
        }
    }
    Ok(())
}

/// Whether a value needs block-style layout (indented on the line after `:`)
/// rather than inline scalar layout. Anchor-wrapped block collections must be
/// treated like the inner collection.
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
        write_string(output, key, indent, config);

        if needs_block_layout(value) {
            output.push(':');
            // Anchor-wrapped block values render as "&idNNN\n  ..." — we
            // need the space between ":" and "&" to keep valid YAML.
            if matches!(
                value,
                Value::Tagged(t) if t.tag().as_str() == crate::fmt::MAGIC_ANCHOR_DEF
            ) {
                output.push(' ');
            }
            // `compact_list_indent`: when on, sequence values
            // under a mapping key align with the key column
            // instead of being bumped one indent level deeper.
            // This is the visual style preferred by some style
            // guides (Kubernetes manifests, GitHub Actions
            // workflows). Mappings and other non-sequence block
            // values keep the standard indent.
            let next_indent = if config.compact_list_indent && matches!(value, Value::Sequence(_)) {
                indent
            } else {
                indent + 1
            };
            write_value(output, value, next_indent, false, config, depth + 1)?;
        } else {
            output.push_str(": ");
            write_value(output, value, indent, false, config, depth + 1)?;
        }
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
        write_string(output, key, 0, config);
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
    output.push_str(chomping);

    for line in s.lines() {
        output.push('\n');
        write_indent(output, config.indent * (indent + 1));
        output.push_str(line);
    }
}

fn write_folded_block(output: &mut String, s: &str, indent: usize, config: &SerializerConfig) {
    let chomping = if s.ends_with('\n') {
        if s.ends_with("\n\n") { "+" } else { "" }
    } else {
        "-"
    };

    output.push('>');
    output.push_str(chomping);

    for line in s.lines() {
        output.push('\n');
        write_indent(output, config.indent * (indent + 1));
        output.push_str(line);
    }
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
pub fn to_string_multi<T: Serialize>(values: &[T]) -> Result<String> {
    to_string_multi_with_config(values, &SerializerConfig::default())
}

/// Serialize an iterable of values as a multi-document YAML
/// string with a custom [`SerializerConfig`].
///
/// # Errors
///
/// All variants documented on [`to_string_with_config`].
pub fn to_string_multi_with_config<T: Serialize>(
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
pub fn to_writer_multi<W: std::io::Write, T: Serialize>(writer: W, values: &[T]) -> Result<()> {
    to_writer_multi_with_config(writer, values, &SerializerConfig::default())
}

/// Serialize multiple values as multi-document YAML to a writer with custom
/// configuration.
///
/// # Errors
///
/// Returns an error if any value cannot be serialized or writing fails.
#[cfg(feature = "std")]
pub fn to_writer_multi_with_config<W: std::io::Write, T: Serialize>(
    writer: W,
    values: &[T],
    config: &SerializerConfig,
) -> Result<()> {
    let s = to_string_multi_with_config(values, config)?;
    let mut writer = writer;
    writer.write_all(s.as_bytes())?;
    Ok(())
}

/// A YAML serializer.
#[derive(Debug, Copy, Clone)]
pub struct Serializer;

impl ser::Serializer for Serializer {
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
        if v <= i64::MAX as u64 {
            return Ok(Value::Number(Number::Integer(v as i64)));
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
        T: ?Sized + Serialize,
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
        T: ?Sized + Serialize,
    {
        // Intercept formatting hint magic names
        match name {
            crate::fmt::MAGIC_FLOW_SEQ
            | crate::fmt::MAGIC_FLOW_MAP
            | crate::fmt::MAGIC_LIT_STR
            | crate::fmt::MAGIC_FOLD_STR
            | crate::fmt::MAGIC_SPACE_AFTER => {
                let inner = value.serialize(Serializer)?;
                Ok(Value::Tagged(Box::new(TaggedValue::new(
                    Tag::new(name),
                    inner,
                ))))
            }
            crate::fmt::MAGIC_COMMENTED => {
                // value is a tuple (inner_value, comment_string)
                let inner = value.serialize(Serializer)?;
                Ok(Value::Tagged(Box::new(TaggedValue::new(
                    Tag::new(name),
                    inner,
                ))))
            }
            crate::fmt::MAGIC_ANCHOR_DEF | crate::fmt::MAGIC_ANCHOR_REF => {
                // ANCHOR_DEF: value serializes as Sequence([String(id), inner]).
                // ANCHOR_REF: value serializes as String(id).
                let inner = value.serialize(Serializer)?;
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
        T: ?Sized + Serialize,
    {
        let mut map = Mapping::new();
        let _ = map.insert(variant.to_owned(), value.serialize(Serializer)?);
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

impl ser::SerializeSeq for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.vec.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Sequence(self.vec))
    }
}

impl ser::SerializeTuple for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeSeq::end(self)
    }
}

/// Serializer for tuple variants.
#[derive(Debug)]
pub struct SerializeTupleVariant {
    name: String,
    vec: Vec<Value>,
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
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

impl ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
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
        T: ?Sized + Serialize,
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error::Serialize("missing key".to_string()))?;
        let _ = self.map.insert(key, value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Mapping(self.map))
    }
}

impl ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
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

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
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
            _ => panic!("Expected RecursionLimitExceeded error, got {:?}", result),
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
