// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! YAML Deserialization.
//!
//! # Examples
//!
//! ```
//! use noyalib::from_str;
//! use std::collections::BTreeMap;
//! let m: BTreeMap<String, i32> = from_str("a: 1\nb: 2\n").unwrap();
//! assert_eq!(m.get("a"), Some(&1));
//! ```

use crate::error::{Error, Result};
use crate::parser::{self};
use crate::prelude::*;
#[cfg(feature = "std")]
use crate::span_context;
use crate::value::Value;
use serde::Deserialize;
#[cfg(feature = "std")]
use std::io;

mod config;
mod deserializer;
pub use config::{DuplicateKeyPolicy, MergeKeyPolicy, ParserConfig, RequireIndent, YamlVersion};
pub use deserializer::Deserializer;
pub(crate) use deserializer::{SpannedMapAccess, is_binary_tag};

/// Deserialize YAML from a `&str` into a typed `T`.
///
/// Default entry point for typed deserialisation. Drives the
/// streaming fast-path when the input matches its eligibility
/// rules (no custom merge-key policy, no ignore-binary-tag mode,
/// no registered policies); otherwise routes through the
/// `Value`-AST loader. The choice is transparent — both paths
/// produce identical results.
///
/// # Errors
///
/// Returns [`Error`](crate::Error) when:
///
/// - `Error::Parse` / `Error::ParseWithLocation` — `s` is not
///   well-formed YAML 1.2 (missing closing bracket, indentation
///   mismatch, invalid escape, …).
/// - `Error::Deserialize` — the document parses but does not
///   match `T`'s shape (wrong scalar type, missing required field
///   on a struct without `#[serde(default)]`, unknown enum
///   variant, …).
/// - `Error::DepthLimit` / `Error::DocumentTooLong` /
///   `Error::AliasLimit` — input exceeds the default
///   [`ParserConfig`] safety budgets. Use
///   [`from_str_with_config`] with [`ParserConfig::strict()`] for
///   tighter limits or with relaxed limits if the defaults
///   reject a known-good document.
/// - `Error::DuplicateKey` — only when `duplicate_key_policy`
///   has been switched to `Error`. The default policy is `Last`,
///   which deduplicates without erroring.
/// - `Error::Custom` — surface for upstream `serde::de::Error`
///   conversions; ordinarily the more specific variants above are
///   produced first.
///
/// # Examples
///
/// ```
/// let n: i32 = noyalib::from_str("42").unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    from_str_with_config(s, &ParserConfig::default())
}

/// Deserialise a YAML document into a target type that may borrow
/// from the input slice (e.g. `&'a str`, `Cow<'a, str>`, structs
/// containing those).
///
/// Where [`from_str`] requires `T: DeserializeOwned + 'static` and
/// therefore can never satisfy `Deserialize<'de> for &'de str`, this
/// function pins the deserialiser's lifetime to the input buffer's
/// lifetime. Plain (unquoted) and unescaped quoted scalars are
/// surfaced via `visit_borrowed_str`, allowing zero-copy borrows.
/// Scalars that required transformation (escape decoding, line
/// folding, alias replay, tag resolution) fall back to owned
/// allocations — see [`crate::borrowed::TransformReason`] for the
/// catalogue of transform causes.
///
/// # Errors
///
/// Returns an error if `s` is not valid YAML, exceeds the default
/// security limits, or cannot be deserialised into `T`. When `T`
/// targets `&'a str` and the YAML scalar required transformation,
/// serde's `&str` visitor surfaces an `invalid type: string,
/// expected a borrowed string` error — switch the target to
/// `Cow<'a, str>` or `String` to accept either form.
///
/// # Examples
///
/// ```
/// use noyalib::from_str_borrowing;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Person<'a> {
///     name: &'a str,
///     role: &'a str,
/// }
///
/// let yaml = "name: noyalib\nrole: parser\n";
/// let p: Person<'_> = from_str_borrowing(yaml).unwrap();
/// assert_eq!(p.name, "noyalib");
/// assert_eq!(p.role, "parser");
/// ```
pub fn from_str_borrowing<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    from_str_borrowing_with_config(s, &ParserConfig::default())
}

/// [`from_str_borrowing`] with a custom [`ParserConfig`] — typically
/// to tighten security limits for untrusted input.
///
/// # Errors
///
/// Returns an error if `s` is not valid YAML under the supplied
/// config or cannot be deserialised into `T`.
///
/// # Examples
///
/// ```
/// use noyalib::{from_str_borrowing_with_config, ParserConfig};
/// let cfg = ParserConfig::strict();
/// let s: &str = from_str_borrowing_with_config("hello\n", &cfg).unwrap();
/// assert_eq!(s, "hello");
/// ```
pub fn from_str_borrowing_with_config<'a, T>(s: &'a str, config: &ParserConfig) -> Result<T>
where
    T: Deserialize<'a>,
{
    let parse_config = parser::ParseConfig::from(config);
    if s.len() > parse_config.max_document_length {
        return Err(Error::Parse(format!(
            "document exceeds maximum length of {} bytes",
            parse_config.max_document_length
        )));
    }
    let mut de = crate::streaming::StreamingDeserializer::with_config(s, parse_config);
    if let Some(registry) = config.tag_registry.as_ref() {
        de = de.with_tag_registry(Arc::clone(registry));
    }
    T::deserialize(&mut de)
}

/// Compile-time-ish check: is the deserialise target `T` exactly
/// [`Value`]? Used by [`from_str_with_config`] / [`from_value`]
/// to enable the tag-preserving fast-path on
/// [`Deserializer::deserialize_any`] only when the caller wants a
/// `Value`. For typed targets (`#[derive(Deserialize)] struct`,
/// scalars, enums, …) the standard transparent-tag behaviour
/// stays in place.
///
/// `Value` is `'static`, so [`std::any::TypeId::of`] is well-formed
/// here. The check returns `false` for any other `T`, including
/// `Spanned<Value>` and `Vec<Value>` (where the outer wrapper has
/// a distinct `TypeId`).
fn is_value_target<T: 'static + ?Sized>() -> bool {
    use core::any::TypeId;
    TypeId::of::<T>() == TypeId::of::<Value>()
}

/// Internal typed-deserialise entry that does **not** require
/// `T: 'static` and never engages the tag-preserving fast-path.
///
/// Used by integrations whose external trait signatures expose
/// `T: for<'de> Deserialize<'de>` without a `'static` bound (e.g.
/// the [`figment`] crate's [`figment::Format::from_str`]
/// signature). In those contexts the caller has already
/// type-erased through `T = ProfileFigure` etc., and a tag-
/// preserving Value reconstitution would never apply anyway.
///
/// Mirrors [`from_str_with_config`] in every other respect:
/// streaming fast-path → AST loader fallback → policy walk →
/// `T::deserialize`.
#[cfg(all(feature = "std", feature = "figment"))]
pub(crate) fn from_str_typed_no_tag_preserve<T>(s: &str, config: &ParserConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let stream_eligible = config.merge_key_policy == MergeKeyPolicy::Auto
        && !config.ignore_binary_tag_for_string
        && config.policies.is_empty();
    if stream_eligible {
        if let Some(res) = crate::streaming::from_str_streaming(s, config) {
            return res;
        }
    }
    let parse_config = parser::ParseConfig::from(config);
    let (value, span_tree) = parser::parse_one(s, &parse_config)?;
    for p in &config.policies {
        p.check_value(&value)?;
    }
    let spans = span_context::build_span_map(&value, &span_tree);
    let ctx = span_context::SpanContext {
        spans,
        source: s.into(),
    };
    let _guard = span_context::set_span_context(ctx);
    let de = Deserializer::with_options(
        &value,
        Some(_guard.as_ref()),
        config.ignore_binary_tag_for_string,
    );
    T::deserialize(de)
}

/// Strict deserialise: like [`from_str`] but errors if `s`
/// contains any keys that the target type `T` does not declare.
///
/// Solves the typo-in-config-key problem — by default, both
/// `serde_yaml` and noyalib silently ignore keys the struct
/// does not know about, so a misspelled `replicass: 3` (with
/// the typo) deserialises into a struct whose `replicas` field
/// stays at its `Default`. `from_str_strict` surfaces those
/// extra keys as a typed `Error::UnknownField` listing every
/// offending path.
///
/// # Errors
///
/// - Any key in the YAML document is not declared on `T`.
/// - Any of the regular [`from_str`] error paths.
///
/// # Examples
///
/// ```
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize)]
/// struct Config {
///     port: u16,
/// }
///
/// // The typo "porrt" is silently ignored by `from_str`. With
/// // `from_str_strict` it surfaces as a typed error.
/// let yaml = "port: 8080\nporrt: 9090\n";
/// assert!(noyalib::from_str::<Config>(yaml).is_ok());
/// assert!(noyalib::from_str_strict::<Config>(yaml).is_err());
/// ```
#[cfg(all(feature = "std", feature = "strict-deserialise"))]
pub fn from_str_strict<T>(s: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    let unknown = std::sync::Mutex::new(Vec::<String>::new());
    let value: Value = from_str_with_config(s, &ParserConfig::default())?;
    let result: Result<T> = serde_ignored::deserialize(&value, |path| {
        unknown
            .lock()
            .expect("from_str_strict: ignored-paths lock poisoned")
            .push(path.to_string());
    });
    let extras = unknown
        .into_inner()
        .expect("from_str_strict: ignored-paths lock poisoned");
    let typed = result?;
    if !extras.is_empty() {
        let msg = if extras.len() == 1 {
            format!("unknown field at `{}`", extras[0])
        } else {
            let joined = extras
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("unknown fields: {joined}")
        };
        return Err(Error::UnknownField(msg));
    }
    Ok(typed)
}

/// Strict deserialise from a byte slice — like [`from_slice`] but
/// errors if the input contains keys the target type `T` does not
/// declare.
///
/// Same semantics as [`from_str_strict`]. Use this when the caller
/// already holds a `&[u8]` (e.g. data read from a buffer or returned
/// by a `bytes` framework) and would otherwise pay the UTF-8
/// validation cost of converting to `&str` twice.
///
/// # Errors
///
/// - The byte slice is not valid UTF-8.
/// - Any key in the YAML document is not declared on `T`.
/// - Any of the regular [`from_slice`] error paths.
///
/// # Examples
///
/// ```
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize)]
/// struct Config {
///     port: u16,
/// }
///
/// let yaml: &[u8] = b"port: 8080\nporrt: 9090\n";
/// assert!(noyalib::from_slice::<Config>(yaml).is_ok());
/// assert!(noyalib::from_slice_strict::<Config>(yaml).is_err());
/// ```
#[cfg(all(feature = "std", feature = "strict-deserialise"))]
pub fn from_slice_strict<T>(b: &[u8]) -> Result<T>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    let s = core::str::from_utf8(b).map_err(|e| Error::Deserialize(e.to_string()))?;
    from_str_strict(s)
}

/// Strict deserialise from an IO reader — like [`from_reader`] but
/// errors if the input contains keys the target type `T` does not
/// declare.
///
/// Same semantics as [`from_str_strict`]. Reads the entire stream
/// into memory before parsing, mirroring [`from_reader`].
///
/// # Errors
///
/// - The reader returns an I/O error or the data is not valid UTF-8.
/// - Any key in the YAML document is not declared on `T`.
/// - Any of the regular [`from_reader`] error paths.
///
/// # Examples
///
/// ```
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize)]
/// struct Config {
///     port: u16,
/// }
///
/// let yaml = b"port: 8080\nporrt: 9090\n".to_vec();
/// assert!(noyalib::from_reader::<_, Config>(&yaml[..]).is_ok());
/// assert!(noyalib::from_reader_strict::<_, Config>(&yaml[..]).is_err());
/// ```
#[cfg(all(feature = "std", feature = "strict-deserialise"))]
pub fn from_reader_strict<R, T>(mut reader: R) -> Result<T>
where
    R: io::Read,
    T: for<'de> Deserialize<'de> + 'static,
{
    let mut s = String::new();
    let _ = reader.read_to_string(&mut s).map_err(Error::Io)?;
    from_str_strict(&s)
}

/// Deserialize YAML from a `&str` with a custom [`ParserConfig`].
///
/// Use this when the defaults need overriding — common reasons:
/// untrusted input ([`ParserConfig::strict()`]), pipeline-specific
/// limits ([`ParserConfig::max_depth`]), YAML 1.1 compatibility
/// ([`ParserConfig::version`]), or custom safe-YAML
/// [`policy::Policy`](crate::policy::Policy) enforcement.
///
/// # Errors
///
/// Same variant set as [`from_str`]. The active `config` controls
/// which limit-related errors can fire:
///
/// - When `config.duplicate_key_policy == DuplicateKeyPolicy::Error`,
///   any duplicate mapping key returns `Error::DuplicateKey`.
/// - When the input exceeds the configured `max_depth`,
///   `max_document_length`, `max_alias_expansions`,
///   `max_mapping_keys`, or `max_sequence_length`, the matching
///   `Error::*Limit` variant is returned.
/// - When `config.policies` contains a policy that rejects the
///   document, `Error::Deserialize` is returned with the policy's
///   diagnostic.
///
/// # Examples
///
/// ```
/// use noyalib::{from_str_with_config, ParserConfig};
/// let cfg = ParserConfig::strict();
/// let n: i32 = from_str_with_config("7", &cfg).unwrap();
/// assert_eq!(n, 7);
/// ```
pub fn from_str_with_config<T>(s: &str, config: &ParserConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    // Try streaming path first (faster, no intermediate Value AST).
    // The streaming path bakes in YAML 1.2 semantics:
    // - `<<: *alias` merges natively;
    // - `!!binary` is propagated as a typed tag.
    // When the caller asked for a non-default behaviour on either
    // axis, route through the AST loader so the relevant toggle
    // takes effect. Active `properties` interpolation also disables
    // the streaming path so the post-parse substitution walk runs.
    let stream_eligible = config.merge_key_policy == MergeKeyPolicy::Auto
        && !config.ignore_binary_tag_for_string
        && config.policies.is_empty()
        && properties_inactive(config)
        && includes_inactive(config);
    if stream_eligible {
        if let Some(res) = crate::streaming::from_str_streaming(s, config) {
            return res;
        }
    }

    let parse_config = parser::ParseConfig::from(config);

    // Skip-span + zero-rewalk fast path: when T == Value, the AST
    // we just parsed *is* the answer. The default path would
    // (a) build a `SpanTree` that `Value::deserialize` never
    // consults, then (b) hand the parsed `Value` back to serde,
    // which walks it a second time and rebuilds an identical
    // `Value` via `ValueVisitor::visit_seq` / `visit_map` — pure
    // waste. Instead, parse via `parse_one_value` (no SpanTree)
    // and downcast the `Value` directly into `T`. The downcast
    // is the safe stdlib `Box<dyn Any>::downcast::<T>()` and is
    // provably correct because `is_value_target::<T>()` already
    // verified `TypeId::of::<T>() == TypeId::of::<Value>()`.
    if is_value_target::<T>() {
        let mut value = parser::parse_one_value(s, &parse_config)?;
        apply_includes(&mut value, config)?;
        apply_properties(&mut value, config)?;
        for p in &config.policies {
            p.check_value(&value)?;
        }
        let boxed: Box<dyn core::any::Any> = Box::new(value);
        // SAFETY-by-construction: `is_value_target::<T>()` already
        // verified `TypeId::of::<T>() == TypeId::of::<Value>()`, so
        // the boxed `Value` downcasts to `T` infallibly. `.expect()`
        // documents the invariant; the path is provably unreachable.
        let downcast: Box<T> = boxed
            .downcast::<T>()
            .expect("is_value_target proved T == Value");
        return Ok(*downcast);
    }

    #[cfg(feature = "std")]
    {
        let (mut value, span_tree) = parser::parse_one(s, &parse_config)?;
        apply_includes(&mut value, config)?;
        apply_properties(&mut value, config)?;
        for p in &config.policies {
            p.check_value(&value)?;
        }
        let spans = span_context::build_span_map(&value, &span_tree);
        let ctx = span_context::SpanContext {
            spans,
            source: s.into(),
        };
        let _guard = span_context::set_span_context(ctx);
        let de = Deserializer::with_options(
            &value,
            Some(_guard.as_ref()),
            config.ignore_binary_tag_for_string,
        );
        T::deserialize(de)
    }

    #[cfg(not(feature = "std"))]
    {
        let value = parser::parse_one_value(s, &parse_config)?;
        let de = Deserializer::with_options(&value, None, config.ignore_binary_tag_for_string);
        T::deserialize(de)
    }
}

/// Returns `true` when no `${KEY}` substitution table is active —
/// keeps the streaming fast-path eligible. Always `true` under
/// `no_std` (the field doesn't exist).
#[cfg(feature = "std")]
#[inline]
fn properties_inactive(config: &ParserConfig) -> bool {
    config.properties.is_none()
}

#[cfg(not(feature = "std"))]
#[inline]
fn properties_inactive(_config: &ParserConfig) -> bool {
    true
}

/// Walk the `Value` tree and substitute every `${name}` placeholder
/// against `config.properties`. No-op when no map is installed; in
/// `no_std` builds the function is also a no-op (the field doesn't
/// exist). Honours `strict_properties` for the missing-key
/// behaviour, but always propagates *syntax* errors from the
/// substitution walk (invalid characters, unterminated `${...}`,
/// malformed `:-default`) regardless of mode.
#[cfg(feature = "std")]
fn apply_properties(value: &mut Value, config: &ParserConfig) -> Result<()> {
    if let Some(props) = config.properties.as_ref() {
        let action = if config.strict_properties {
            crate::value::MissingAction::Error(false)
        } else {
            crate::value::MissingAction::Empty
        };
        value.interpolate_inner(
            &|name| match props.get(name) {
                Some(v) => crate::value::ResolveOutcome::Found(v.clone()),
                None => crate::value::ResolveOutcome::Missing,
            },
            action,
        )?;
    }
    Ok(())
}

#[cfg(not(feature = "std"))]
#[inline]
fn apply_properties(_value: &mut Value, _config: &ParserConfig) -> Result<()> {
    Ok(())
}

/// Returns `true` when no `!include` resolver is installed —
/// keeps the streaming fast-path eligible.
#[cfg(feature = "include")]
#[inline]
fn includes_inactive(config: &ParserConfig) -> bool {
    config.include_resolver.is_none()
}

#[cfg(not(feature = "include"))]
#[inline]
fn includes_inactive(_config: &ParserConfig) -> bool {
    true
}

/// Walk the `Value` tree, find every `Value::Tagged(!include, _)`
/// node, and replace it with the resolver's output. Cyclic
/// includes are detected via a per-walk visited set; depth is
/// bounded by `config.max_include_depth`. No-op when no resolver
/// is installed.
#[cfg(feature = "include")]
fn apply_includes(value: &mut Value, config: &ParserConfig) -> Result<()> {
    if let Some(resolver) = config.include_resolver.as_ref() {
        let parse_config = parser::ParseConfig::from(config);
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut next_id: usize = 1;
        resolve_includes_recursive(
            value,
            resolver,
            &parse_config,
            config.max_include_depth,
            0,
            0,
            &mut visited,
            &mut next_id,
        )?;
    }
    Ok(())
}

#[cfg(not(feature = "include"))]
#[inline]
fn apply_includes(_value: &mut Value, _config: &ParserConfig) -> Result<()> {
    Ok(())
}

#[cfg(feature = "include")]
#[allow(clippy::too_many_arguments)]
fn resolve_includes_recursive(
    value: &mut Value,
    resolver: &crate::include::IncludeResolver,
    parse_config: &parser::ParseConfig,
    max_depth: usize,
    depth: usize,
    from_id: usize,
    visited: &mut std::collections::HashSet<String>,
    next_id: &mut usize,
) -> Result<()> {
    if depth > max_depth {
        return Err(Error::RecursionLimitExceeded { depth });
    }
    match value {
        Value::Tagged(boxed) => {
            if boxed.tag().as_str() == "!include" {
                let spec = match boxed.value().as_str() {
                    Some(s) => s.to_string(),
                    None => {
                        return Err(Error::Custom(
                            "!include directive expects a scalar string spec".into(),
                        ));
                    }
                };
                if !visited.insert(spec.clone()) {
                    return Err(Error::Custom(format!(
                        "!include cycle detected: `{spec}` already in resolution chain"
                    )));
                }
                let (path, fragment) = crate::include::split_fragment(&spec);
                let req = crate::include::IncludeRequest {
                    spec: &spec,
                    from_id,
                    depth,
                };
                let source = resolver.resolve(req)?;
                let id = *next_id;
                *next_id += 1;
                let mut included = parser::parse_one_value(&source.bytes, parse_config)?;
                // Recurse into the included document's own
                // `!include` nodes — depth + 1.
                resolve_includes_recursive(
                    &mut included,
                    resolver,
                    parse_config,
                    max_depth,
                    depth + 1,
                    id,
                    visited,
                    next_id,
                )?;
                // Fragment selection: if `spec` was `foo.yaml#anchor`,
                // narrow to the named anchor inside the included
                // document. Fragments resolve against mapping keys
                // (the conventional YAML "anchor-as-key" pattern);
                // see `examples/include_directive.rs`.
                if let Some(frag) = fragment {
                    if let Some(map) = included.as_mapping() {
                        match map.get(frag) {
                            Some(v) => *value = v.clone(),
                            None => {
                                return Err(Error::Custom(format!(
                                    "!include fragment `#{frag}` not found in `{path}`"
                                )));
                            }
                        }
                    } else {
                        return Err(Error::Custom(format!(
                            "!include fragment `#{frag}` requires a mapping-shaped \
                             included document; `{path}` is not a mapping"
                        )));
                    }
                } else {
                    *value = included;
                }
                let _ = visited.remove(&spec);
            } else {
                resolve_includes_recursive(
                    boxed.value_mut(),
                    resolver,
                    parse_config,
                    max_depth,
                    depth,
                    from_id,
                    visited,
                    next_id,
                )?;
            }
        }
        Value::Sequence(seq) => {
            for v in seq {
                resolve_includes_recursive(
                    v,
                    resolver,
                    parse_config,
                    max_depth,
                    depth,
                    from_id,
                    visited,
                    next_id,
                )?;
            }
        }
        Value::Mapping(map) => {
            for v in map.values_mut() {
                resolve_includes_recursive(
                    v,
                    resolver,
                    parse_config,
                    max_depth,
                    depth,
                    from_id,
                    visited,
                    next_id,
                )?;
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
    Ok(())
}

/// Deserialize YAML from a byte slice.
///
/// Convenience wrapper that validates `b` is UTF-8 then forwards
/// to [`from_str`]. Use when the caller already holds a `&[u8]`
/// (a buffer, a network frame, a `bytes::Bytes`) and would
/// otherwise have to round-trip through `String`.
///
/// # Errors
///
/// - `Error::Deserialize` — `b` is not valid UTF-8.
/// - All variants documented on [`from_str`].
///
/// # Examples
///
/// ```
/// let n: i32 = noyalib::from_slice(b"42").unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn from_slice<T>(b: &[u8]) -> Result<T>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    let s = core::str::from_utf8(b).map_err(|e| Error::Deserialize(e.to_string()))?;
    from_str(s)
}

/// Deserialize YAML from a byte slice with a custom [`ParserConfig`].
///
/// # Errors
///
/// - `Error::Deserialize` — `b` is not valid UTF-8.
/// - All variants documented on [`from_str_with_config`].
///
/// # Examples
///
/// ```
/// use noyalib::{from_slice_with_config, ParserConfig};
/// let cfg = ParserConfig::new();
/// let n: i32 = from_slice_with_config(b"7", &cfg).unwrap();
/// assert_eq!(n, 7);
/// ```
pub fn from_slice_with_config<T>(b: &[u8], config: &ParserConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    let s = core::str::from_utf8(b).map_err(|e| Error::Deserialize(e.to_string()))?;
    from_str_with_config(s, config)
}

/// Deserialize YAML from an [`std::io::Read`] source.
///
/// Reads the entire stream into memory before parsing — YAML's
/// data model is not streamable past document boundaries, so this
/// function trades incremental I/O for a single, simple `Result`.
/// For very large multi-document streams prefer
/// [`crate::parallel::parse`] (with the `parallel` feature) which
/// scans document boundaries on the input thread and parses each
/// document in parallel.
///
/// # Errors
///
/// - `Error::Io` — the underlying reader returns an I/O error
///   while filling the buffer.
/// - All variants documented on [`from_str`].
///
/// # Examples
///
/// ```
/// let yaml = b"42".to_vec();
/// let n: i32 = noyalib::from_reader(&yaml[..]).unwrap();
/// assert_eq!(n, 42);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    R: io::Read,
    T: for<'de> Deserialize<'de> + 'static,
{
    from_reader_with_config(reader, &ParserConfig::default())
}

/// Deserialize YAML from an [`std::io::Read`] source with a custom
/// [`ParserConfig`].
///
/// # Errors
///
/// - `Error::Io` — the underlying reader returns an I/O error.
/// - All variants documented on [`from_str_with_config`].
///
/// # Examples
///
/// ```
/// use noyalib::{from_reader_with_config, ParserConfig};
/// let cfg = ParserConfig::new();
/// let bytes = b"7".to_vec();
/// let n: i32 = from_reader_with_config(&bytes[..], &cfg).unwrap();
/// assert_eq!(n, 7);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn from_reader_with_config<R, T>(mut reader: R, config: &ParserConfig) -> Result<T>
where
    R: io::Read,
    T: for<'de> Deserialize<'de> + 'static,
{
    let mut s = String::new();
    let _ = reader.read_to_string(&mut s).map_err(Error::Io)?;
    from_str_with_config(&s, config)
}

/// Deserialize a [`Value`] into a typed `T` via Serde's data model.
///
/// Useful for second-pass conversion when the first pass parsed
/// into the dynamic [`Value`] tree and a typed view is now needed
/// for a sub-tree.
///
/// # Errors
///
/// - `Error::Deserialize` — `value` does not match `T`'s shape.
/// - `Error::Custom` — surfaces upstream `serde::de::Error`
///   conversions that don't fit the structured variants.
///
/// # Examples
///
/// ```
/// use noyalib::{from_value, Value};
/// let v = Value::from(42_i64);
/// let n: i32 = from_value(&v).unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn from_value<T>(value: &Value) -> Result<T>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    // Zero-rewalk fast path: when T == Value, the answer is just
    // `value.clone()`. The default `T::deserialize(de)` route
    // walks `value` and reconstructs an identical Value via
    // `ValueVisitor::visit_seq`/`visit_map` — pure waste.
    if is_value_target::<T>() {
        let cloned = value.clone();
        let boxed: Box<dyn core::any::Any> = Box::new(cloned);
        let downcast: Box<T> = boxed
            .downcast::<T>()
            .expect("is_value_target proved T == Value");
        return Ok(*downcast);
    }
    T::deserialize(Deserializer::new(value))
}
