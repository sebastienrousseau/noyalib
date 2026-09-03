// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Drop-in API surface compatible with `serde_yaml` 0.9.
//!
//! The upstream `serde_yaml` 0.9 crate is unmaintained. This
//! module exposes a name-for-name surface so existing codebases
//! can migrate by editing two lines:
//!
//! ## Why migrate from `serde_yaml`?
//!
//! - **Maintained.** `serde_yaml` 0.9 was archived by its author
//!   in 2024; security advisories and YAML-spec corrections do
//!   not flow into it. noyalib is actively maintained.
//! - **Faster.** noyalib's deserialiser outpaces `serde_yaml_ng`
//!   (the most active fork) by **39 – 64 %** on representative
//!   workloads; the streaming path adds another 22 % on top of
//!   that for large documents. SIMD-accelerated structural
//!   discovery and SWAR decimal parsing pull big-document parses
//!   another 4–9× ahead on the bytes / second metric.
//!   Numbers are reproducible via `cargo bench --bench
//!   comparison`.
//! - **Zero `unsafe`.** noyalib enforces `#![forbid(unsafe_code)]`
//!   across the entire workspace — every line of parser, scanner,
//!   formatter, and CST code is checked at compile time. Audits
//!   that would otherwise need to verify `serde_yaml`'s `unsafe`
//!   blocks evaporate.
//! - **Lossless tooling.** noyalib ships a byte-faithful CST
//!   ([`crate::cst::Document`]) so editing tools can patch a
//!   single value while preserving every comment, indent, and
//!   sibling entry — something the original `serde_yaml` cannot
//!   do at all.
//! - **No dead branch.** The `compat-serde-yaml` shim does
//!   **not** re-introduce the unmaintained crate as a dependency.
//!   Every type the shim exposes is a noyalib-native type
//!   re-exported under the `serde_yaml` name; downstream
//!   `cargo audit` / `cargo deny` never picks up the archived
//!   advisory chain.
//! - **YAML 1.2 spec compliant.** noyalib passes 406/406 cases
//!   in the official YAML 1.2 test suite. `serde_yaml` 0.9
//!   carries known spec deviations that are baked-in for back
//!   compat; noyalib has the freedom to fix them.
//!
//! ## Drop-in migration
//!
//! Zero source changes, via Cargo's package rename and the
//! `noyalib-serde-yaml` companion crate:
//!
//! ```toml
//! # Cargo.toml — the whole migration
//! serde_yaml = { package = "noyalib-serde-yaml", version = "=0.0.29" }
//! ```
//!
//! Or two lines, depending on noyalib directly:
//!
//! ```toml
//! # Cargo.toml — before
//! serde_yaml = "0.9"
//! # Cargo.toml — after
//! noyalib = { version = "0.0", features = ["compat-serde-yaml"] }
//! ```
//!
//! ```rust,ignore
//! // anywhere in the codebase
//! - use serde_yaml::{from_str, to_string, Value};
//! + use noyalib::compat::serde_yaml::{from_str, to_string, Value};
//! ```
//!
//! Every function delegates to the underlying noyalib engine — no
//! double-parsing, no extra allocations, no parser fork. Where
//! `serde_yaml`'s signature differs from noyalib's (the most common
//! case is taking a `Value` by value vs. by reference), this shim
//! provides a thin adapter; everything else is a re-export.
//!
//! # Zero legacy dependencies
//!
//! The shim deliberately **does not depend on the unmaintained
//! `serde_yaml` 0.9 crate**. Every type the shim exposes is a
//! noyalib-native type re-exported under the `serde_yaml` name —
//! you migrate *off* the legacy crate, not into a vendored copy
//! of it. Downstream `cargo audit` / `cargo deny` runs do not
//! pick up the archived advisory chain.
//!
//! # Known surface differences
//!
//! - **`Mapping` is string-keyed.** noyalib's [`Mapping`] uses
//!   `String` keys; `serde_yaml::Mapping` allowed any [`Value`] as
//!   a key. The 99 % case (configuration files, RPC payloads) is
//!   string-keyed and works unchanged. If your code constructs a
//!   `Mapping` with non-string keys, switch to noyalib's
//!   [`crate::MappingAny`] directly — the shim does not re-export
//!   it under the `Mapping` name to keep type errors localised.
//!
//! # Migrating an in-flight `serde_yaml::Value`
//!
//! Mid-migration codebases sometimes still hold an upstream
//! `::serde_yaml::Value` produced by an un-migrated module. The
//! Serde data model is the universal translator: every
//! Serde-compatible value can be funnelled through
//! [`crate::from_value`] / [`crate::to_value`] without depending
//! on the upstream library.
//!
//! ```rust,ignore
//! // Upstream value in flight (un-migrated module hands you one).
//! let upstream: ::serde_yaml::Value = legacy_call();
//!
//! // Lower it into noyalib::Value via the Serde bridge — works
//! // because both ASTs implement `Serialize` / `Deserialize`.
//! let lowered: noyalib::Value = noyalib::to_value(&upstream)?;
//!
//! // Or go straight to a typed struct, skipping the Value AST.
//! let cfg: MyConfig = noyalib::from_value(&noyalib::to_value(&upstream)?)?;
//! ```
//!
//! Going the other direction is just as direct:
//!
//! ```rust,ignore
//! let lifted: ::serde_yaml::Value =
//!     ::serde_yaml::to_value(&my_noyalib_value)?;
//! ```
//!
//! Both directions cost one Serde round-trip — the same wall-clock
//! cost as a hand-written `From` impl on a representative
//! `Value` shape. The benefit: zero dependency on the archived
//! crate.
//!
//! # `Error` parity
//!
//! The `Error::location() -> Option<Location>` /
//! `Location::line()` / `Location::column()` / `Location::index()`
//! chain matches `serde_yaml`'s shape byte-for-byte (1-indexed
//! line and column, 0-indexed byte offset), so existing
//! diagnostic-emitting code that destructures these compiles
//! unchanged.
//!
//! # Migration cookbook
//!
//! ```rust
//! use noyalib::compat::serde_yaml as syml;
//!
//! #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
//! struct Config { name: String, port: u16 }
//!
//! let yaml = "name: noyalib\nport: 8080\n";
//! let cfg: Config = syml::from_str(yaml).unwrap();
//! assert_eq!(cfg, Config { name: "noyalib".into(), port: 8080 });
//!
//! let back = syml::to_string(&cfg).unwrap();
//! let round: Config = syml::from_str(&back).unwrap();
//! assert_eq!(cfg, round);
//! ```
//!
//! # Behavioural parity with upstream `serde_yaml` 0.9
//!
//! Since v0.0.29 the shim is **behavioural**, not just
//! name-compatible: its entry points parse under
//! [`crate::ParserConfig::serde_yaml_compat`] and its [`Error`]
//! renders upstream's wording and locations. The 18-case
//! `serde_yaml` contract suite (`tests/serde_yaml_contract.rs`,
//! expectations captured live from `serde_yaml 0.9.34`) pins:
//! literal `<<` entries with resolved alias values, `0123` as a
//! string and `0b11` as 3, `1e999` as a string, full `u64`
//! precision with one-past-`u64::MAX` refused as
//! `JSON number out of range`, non-scalar keys refused as
//! `invalid type: sequence, expected a string key`, upstream's
//! `repetition limit exceeded` alias budget, and libyaml's
//! error phrasing and end-of-input location convention. One
//! documented partial: a custom tag under `deserialize_any`
//! refuses with upstream's message, anchored at the value rather
//! than the tag.
//!
//! Callers who want noyalib's own spec-strict defaults use the
//! direct API ([`crate::from_str`]); the differences below then
//! apply:
//!
//! - **Custom-tag scalars surface as [`Value::Tagged`]**
//!   instead of being
//!   silently coerced to the inner string. `from_str::<Value>`
//!   on `!Custom 'hello'` returns
//!   `Value::Tagged(Tag("!Custom"), Value::String("hello"))`,
//!   not `Value::String("hello")`. Migrants who previously
//!   exhaustive-matched the six-variant `serde_yaml::Value`
//!   need to either add a `Value::Tagged(_)` arm or call
//!   [`Value::untag`](crate::Value::untag) /
//!   [`Value::untag_ref`](crate::Value::untag_ref) before the
//!   match. See
//!   [`docs/MIGRATION-FROM-SERDE-YAML.md`](https://github.com/sebastienrousseau/noyalib/blob/main/docs/MIGRATION-FROM-SERDE-YAML.md#1-valuetagged-is-a-7th-variant--and-noyalib-preserves-scalar-tags-too)
//!   §1 for the recipe.
//! - **YAML 1.2 strict booleans by default.** `country: NO`
//!   stays `"NO"` (the YAML 1.2 fix to the "Norway problem")
//!   instead of becoming `false`. Opt back into YAML 1.1
//!   resolver semantics via
//!   [`ParserConfig::version`](crate::ParserConfig::version)`(`[`YamlVersion::V1_1`](crate::YamlVersion)`)`
//!   if your existing pipeline depended on the legacy boolean
//!   recognition.
//!
//! Both of these are documented under "Things `noyalib` adds"
//! and "Behavioural differences worth knowing" in the migration
//! guide. Neither is reachable via the existing `serde_yaml`
//! API surface — they are extra information / safer defaults
//! that flow through unchanged for the typed-deserialise path.

use crate::prelude::*;

// ── Types — re-exported under the serde_yaml names ───────────────────

pub use crate::error::Location;
pub use crate::value::{Mapping, Number, Sequence, Tag, TaggedValue, Value};

/// Shim result type: [`Result<T, Error>`](core::result::Result) with
/// the shim's own [`Error`].
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// fn parse(s: &str) -> syml::Result<syml::Value> {
///     syml::from_str(s)
/// }
/// assert!(parse("a: 1\n").is_ok());
/// ```
pub type Result<T> = core::result::Result<T, Error>;

/// The shim's error type: a [`crate::Error`] rendered the way
/// `serde_yaml` 0.9 worded and located it.
///
/// Since the behavioural-shim rework this is a newtype, not a
/// re-export of [`crate::Error`] — the payoff is `Display` and
/// [`Error::location`] parity with upstream on the error classes the
/// `serde_yaml` contract exercises:
///
/// - budget refusals read `repetition limit exceeded` /
///   `recursion limit exceeded`, unlocated, exactly as upstream;
/// - a refused non-scalar key reads
///   `invalid type: sequence, expected a string key` at the key's
///   location;
/// - parse errors adopt libyaml's phrasing where the class is
///   recognisable (`did not find expected node content …, while
///   parsing a flow node`; `did not find expected ',' or ']' …,
///   while parsing a flow sequence at …`), and an error at
///   end-of-input reports the line *after* the last one, column 1 —
///   libyaml's EOF convention;
/// - deserialization errors render suffix-style
///   (`field: message at line L column C`).
///
/// Classes without an upstream equivalent keep noyalib's own
/// wording. The inner [`crate::Error`] stays reachable via
/// [`Error::into_inner`] / [`Error::inner`].
#[derive(Debug)]
pub struct Error(Box<ErrorImpl>);

/// Boxed innards, mirroring upstream's own `Error(Box<ErrorImpl>)`
/// shape — keeps `Result<T>` at pointer size on the Err side.
#[derive(Debug)]
struct ErrorImpl {
    inner: crate::error::Error,
    display: String,
    location: Option<Location>,
}

impl Error {
    /// Wrap a noyalib error, rendering it upstream-style against the
    /// input it came from (the input drives the EOF location
    /// convention and the flow-sequence context trailer).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::compat::serde_yaml::Error;
    /// let input = "title: [oops";
    /// let inner = noyalib::from_str::<noyalib::Value>(input).unwrap_err();
    /// let e = Error::from_noyalib_with_input(inner, input);
    /// // libyaml's end-of-input convention: the line after the last.
    /// assert_eq!(e.location().unwrap().line(), 2);
    /// ```
    #[must_use]
    pub fn from_noyalib_with_input(inner: crate::error::Error, input: &str) -> Self {
        let mut location = inner.location();
        // libyaml reports an error at end-of-input on the line after
        // the last one, column 1; noyalib says end-of-last-line.
        if let Some(loc) = location {
            let at_eof = loc.index() >= input.len();
            if at_eof && loc.column() > 1 && parse_shaped(&inner) {
                location = Some(Location::new(loc.line() + 1, 1, loc.index()));
            }
        }
        let display = render_upstream_style(&inner, location, Some(input));
        Self(Box::new(ErrorImpl {
            inner,
            display,
            location,
        }))
    }

    /// The upstream-shaped location, when the error has one.
    /// 1-based line and column, 0-based byte index — the exact
    /// `serde_yaml::Location` surface.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::compat::serde_yaml as syml;
    /// let err = syml::from_str::<syml::Value>("a: [unclosed").unwrap_err();
    /// let loc = err.location().unwrap();
    /// assert!(loc.line() >= 1 && loc.column() >= 1);
    /// let _: usize = loc.index();
    /// ```
    #[must_use]
    pub fn location(&self) -> Option<Location> {
        self.0.location
    }

    /// Borrow the underlying [`crate::Error`].
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::compat::serde_yaml as syml;
    /// let err = syml::from_str::<syml::Value>("a: [unclosed").unwrap_err();
    /// assert!(matches!(err.inner().kind(), noyalib::ErrorKind::Syntax));
    /// ```
    #[must_use]
    pub fn inner(&self) -> &crate::error::Error {
        &self.0.inner
    }

    /// Unwrap into the underlying [`crate::Error`] — the way back to
    /// noyalib's own diagnostics (miette/ariadne adapters included).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::compat::serde_yaml as syml;
    /// let err = syml::from_str::<syml::Value>("a: [unclosed").unwrap_err();
    /// let inner: noyalib::Error = err.into_inner();
    /// assert!(inner.location().is_some());
    /// ```
    #[must_use]
    pub fn into_inner(self) -> crate::error::Error {
        self.0.inner
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.display)
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.0.inner)
    }
}

impl From<crate::error::Error> for Error {
    fn from(inner: crate::error::Error) -> Self {
        let location = inner.location();
        let display = render_upstream_style(&inner, location, None);
        Self(Box::new(ErrorImpl {
            inner,
            display,
            location,
        }))
    }
}

/// Is this a parse-shaped error (the family libyaml's EOF location
/// convention applies to)?
fn parse_shaped(e: &crate::error::Error) -> bool {
    matches!(
        e.kind(),
        crate::ErrorKind::Syntax | crate::ErrorKind::EndOfStream
    )
}

/// Render a noyalib error the way `serde_yaml` 0.9 would have worded
/// it. Classes without an upstream analogue keep noyalib's wording.
fn render_upstream_style(
    e: &crate::error::Error,
    location: Option<Location>,
    input: Option<&str>,
) -> String {
    use crate::error::Error as E;
    let at = |loc: Option<Location>| -> String {
        loc.map(|l| format!(" at line {} column {}", l.line(), l.column()))
            .unwrap_or_default()
    };
    match e {
        // Upstream's resource-limit wordings, unlocated like upstream.
        E::RepetitionLimitExceeded => "repetition limit exceeded".to_owned(),
        E::Budget(crate::BudgetBreach::AliasAnchorRatio { .. }) => {
            "repetition limit exceeded".to_owned()
        }
        E::RecursionLimitExceeded { .. } => "recursion limit exceeded".to_owned(),
        // `invalid type: sequence, expected a string key` — noyalib's
        // own wording already matches upstream, and upstream carries
        // the position only in `location()`, not in `Display`.
        E::NonScalarKey { .. } => e.to_string(),
        E::IntegerOverflow { path, .. } => {
            let prefix = path
                .as_deref()
                .map(|p| format!("{p}: "))
                .unwrap_or_default();
            format!("{prefix}JSON number out of range{}", at(location))
        }
        E::ParseWithLocation { message, .. } => {
            if message.starts_with("expected a node but found Flow") {
                format!(
                    "did not find expected node content{}, while parsing a flow node",
                    at(location)
                )
            } else if message == "expected ',' or ']' in flow sequence" {
                let ctx = input
                    .and_then(|inp| location.and_then(|l| unmatched_open_bracket(inp, l.index())))
                    .map(|open| {
                        let l = Location::from_index(input.unwrap_or(""), open);
                        format!(
                            ", while parsing a flow sequence at line {} column {}",
                            l.line(),
                            l.column()
                        )
                    })
                    .unwrap_or_default();
                format!("did not find expected ',' or ']'{}{ctx}", at(location))
            } else if message.starts_with("inconsistent indentation") {
                // libyaml reports a value indicator at an impossible
                // column as a misplaced mapping value.
                format!(
                    "mapping values are not allowed in this context{}",
                    at(location)
                )
            } else {
                format!("{message}{}", at(location))
            }
        }
        E::DeserializeWithLocation { message, .. } => {
            // Upstream renders suffix-style: `field: message at line L
            // column C`.
            format!("{message}{}", at(location))
        }
        _ => e.to_string(),
    }
}

/// Byte offset of the innermost `[` left unclosed at `upto` —
/// the flow-sequence context libyaml names in its error trailer.
/// A display aid only: quote-aware enough for real documents, and
/// absent context simply omits the trailer.
fn unmatched_open_bracket(input: &str, upto: usize) -> Option<usize> {
    let mut stack: Vec<usize> = Vec::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut prev_backslash = false;
    for (i, b) in input.bytes().enumerate().take(upto) {
        if in_double {
            if b == b'"' && !prev_backslash {
                in_double = false;
            }
            prev_backslash = b == b'\\' && !prev_backslash;
            continue;
        }
        if in_single {
            if b == b'\'' {
                in_single = false;
            }
            continue;
        }
        match b {
            b'\'' => in_single = true,
            b'"' => in_double = true,
            b'[' => stack.push(i),
            b']' => {
                let _ = stack.pop();
            }
            _ => {}
        }
    }
    stack.last().copied()
}

// ── `serde_yaml` low-level types ─────────────────────────────────────
//
// `serde_yaml` 0.9 publishes its `Deserializer` / `Serializer` types
// at the crate root for callers that bypass the convenience helpers
// (`from_str`, `to_string`, …). We expose noyalib's own types under
// the same names so existing `serde_yaml::Deserializer` /
// `::Serializer` references compile without modification.

pub use crate::de::Deserializer;
pub use crate::ser::Serializer;

// ── Sub-module namespacing ───────────────────────────────────────────
//
// `serde_yaml` publishes `mapping`, `value`, and `with` sub-modules
// alongside its top-level functions. Migrating code commonly imports
// items via these paths (`use serde_yaml::value::Tag;`,
// `#[serde(with = "serde_yaml::with::singleton_map")]`). We mirror
// the layout so those `use` paths continue to resolve.

/// Sub-module mirroring `serde_yaml::value`.
///
/// `serde_yaml::value::{Value, Mapping, Number, Sequence, Tag,
/// TaggedValue}` are also re-exported at the crate root, but code
/// that imports them via the `value` path keeps working.
pub mod value {
    pub use crate::value::{Mapping, Number, Sequence, Tag, TaggedValue, Value};
}

/// Sub-module mirroring `serde_yaml::mapping`.
///
/// In `serde_yaml` 0.9 this housed the `Mapping` type plus its
/// iterator types. The most common import is `Mapping` itself; we
/// re-export the full set noyalib exposes so user code using the
/// path-form import still resolves.
pub mod mapping {
    pub use crate::value::Mapping;
}

/// Sub-module mirroring `serde_yaml::with`.
///
/// `serde_yaml::with::singleton_map` and its variants are the
/// idiomatic way to control enum representation in `#[serde(with =
/// "...")]` attributes. noyalib's own implementations live under
/// [`crate::with`]; this re-export gives migrants the `serde_yaml`
/// path-form so existing `#[serde(with = "serde_yaml::with::…")]`
/// attributes only need a search-and-replace on the prefix.
pub mod with {
    pub use crate::with::{
        nested_singleton_map, singleton_map, singleton_map_optional, singleton_map_recursive,
        singleton_map_with,
    };
}

// ── Deserialization ──────────────────────────────────────────────────

/// Deserialize a YAML document into the target type — with
/// **`serde_yaml` 0.9's observable behaviour**, not noyalib's
/// defaults.
///
/// The shim parses under [`crate::ParserConfig::serde_yaml_compat`]:
/// `<<` merge keys stay literal entries (alias values resolved),
/// leading-zero integers stay strings and `0b11` is 3, a literal
/// float overflow stays a string, `u64`-range integers keep full
/// precision and one past `u64::MAX` errors, non-scalar keys error,
/// and transitive alias expansion is budgeted exactly as upstream
/// ("repetition limit exceeded"). Callers who want noyalib's own
/// (spec-strict) defaults should use [`crate::from_str`] directly.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let n: i32 = syml::from_str("42").unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: serde_core::de::DeserializeOwned + 'static,
{
    crate::from_str_with_config(s, &crate::ParserConfig::serde_yaml_compat())
        .map_err(|e| Error::from_noyalib_with_input(e, s))
}

/// Deserialize a YAML document from a byte slice.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let n: i32 = syml::from_slice(b"7").unwrap();
/// assert_eq!(n, 7);
/// ```
pub fn from_slice<T>(bytes: &[u8]) -> Result<T>
where
    T: serde_core::de::DeserializeOwned + 'static,
{
    let s = core::str::from_utf8(bytes)
        .map_err(|e| Error::from(crate::error::Error::Parse(format!("invalid UTF-8: {e}"))))?;
    from_str(s)
}

/// Deserialize a YAML document from any [`std::io::Read`] source.
///
/// `serde_yaml::from_reader` and noyalib's `from_reader` have
/// identical signatures, so this is a direct re-export.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let bytes: &[u8] = b"port: 8080\n";
/// let m: std::collections::BTreeMap<String, u16> =
///     syml::from_reader(bytes).unwrap();
/// assert_eq!(m["port"], 8080);
/// ```
#[cfg(feature = "std")]
pub fn from_reader<R, T>(mut reader: R) -> Result<T>
where
    R: std::io::Read,
    T: serde_core::de::DeserializeOwned + 'static,
{
    let mut buf = String::new();
    let _ = reader
        .read_to_string(&mut buf)
        .map_err(|e| Error::from(crate::error::Error::Io(e)))?;
    from_str(&buf)
}

/// Deserialize a typed value from a [`Value`].
///
/// `serde_yaml::from_value` takes the [`Value`] by *value*; noyalib
/// takes it by reference. This adapter accepts the
/// `serde_yaml`-style by-value form so call sites do not need to
/// add an `&` during migration.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let v = syml::Value::Number(syml::Number::Integer(42));
/// let n: i32 = syml::from_value(v).unwrap();
/// assert_eq!(n, 42);
/// ```
pub fn from_value<T>(value: Value) -> Result<T>
where
    T: serde_core::de::DeserializeOwned + 'static,
{
    crate::from_value(&value).map_err(Error::from)
}

// ── Serialization ────────────────────────────────────────────────────

/// Serialize a typed value to a YAML string.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let s = syml::to_string(&42_i32).unwrap();
/// assert!(s.contains("42"));
/// ```
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: serde_core::Serialize,
{
    crate::to_string(value).map_err(Error::from)
}

/// Serialize a typed value to any [`std::io::Write`] sink.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let mut buf: Vec<u8> = Vec::new();
/// syml::to_writer(&mut buf, &42_i32).unwrap();
/// assert!(!buf.is_empty());
/// ```
#[cfg(feature = "std")]
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: std::io::Write,
    T: serde_core::Serialize,
{
    crate::to_writer(writer, value).map_err(Error::from)
}

/// Serialize a typed value to a [`Value`].
///
/// `serde_yaml::to_value` takes by value; noyalib takes by
/// reference. Accepts the `serde_yaml` shape and forwards.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let v = syml::to_value(42_i32).unwrap();
/// assert_eq!(v.as_i64(), Some(42));
/// ```
pub fn to_value<T>(value: T) -> Result<Value>
where
    T: serde_core::Serialize,
{
    crate::to_value(&value).map_err(Error::from)
}

// ── Multi-document streams ───────────────────────────────────────────
//
// `serde_yaml` exposed `Deserializer::from_str(s).into_iter::<T>()`
// for multi-document parsing. noyalib's nearest equivalent is
// `load_all_as`. We expose it under the `serde_yaml` name pattern.

/// Iterate every YAML document in a multi-document stream and
/// deserialize each into `T`. Mirrors the
/// `Deserializer::from_str` chained with `into_iter::<T>()` —
/// the typical `serde_yaml` multi-document idiom.
///
/// # Examples
///
/// ```
/// use noyalib::compat::serde_yaml as syml;
/// let yaml = "1\n---\n2\n---\n3\n";
/// let docs: Vec<i32> = syml::from_str_multi(yaml).unwrap();
/// assert_eq!(docs, vec![1, 2, 3]);
/// ```
pub fn from_str_multi<T>(s: &str) -> Result<Vec<T>>
where
    T: serde_core::de::DeserializeOwned + 'static,
{
    crate::load_all_as::<T>(s).map_err(|e| Error::from_noyalib_with_input(e, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Config {
        name: String,
        port: u16,
    }

    #[test]
    fn from_str_round_trips_typed() {
        let yaml = "name: noyalib\nport: 8080\n";
        let cfg: Config = from_str(yaml).unwrap();
        assert_eq!(
            cfg,
            Config {
                name: "noyalib".into(),
                port: 8080
            }
        );
    }

    #[test]
    fn from_slice_typed() {
        let bytes = b"name: noyalib\nport: 8080\n";
        let cfg: Config = from_slice(bytes).unwrap();
        assert_eq!(cfg.port, 8080);
    }

    #[test]
    fn from_reader_typed() {
        // `serde_yaml::from_reader` drop-in: deserialize straight from
        // an `io::Read` (a `&[u8]` is one).
        let bytes = b"name: noyalib\nport: 8080\n";
        let cfg: Config = from_reader(&bytes[..]).unwrap();
        assert_eq!(
            cfg,
            Config {
                name: "noyalib".into(),
                port: 8080
            }
        );
    }

    #[test]
    fn to_writer_then_from_str_round_trips() {
        // `serde_yaml::to_writer` drop-in: serialize into an
        // `io::Write` sink and confirm the emitted YAML round-trips.
        let cfg = Config {
            name: "noyalib".into(),
            port: 8080,
        };
        let mut buf: Vec<u8> = Vec::new();
        to_writer(&mut buf, &cfg).unwrap();
        let s = String::from_utf8(buf).expect("valid utf-8");
        assert!(s.contains("name: noyalib"), "{s}");
        assert!(s.contains("port: 8080"), "{s}");
        let back: Config = from_str(&s).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn from_value_takes_by_value_like_serde_yaml() {
        let mut m = Mapping::new();
        let _ = m.insert("name", Value::String("noyalib".into()));
        let _ = m.insert("port", Value::Number(Number::Integer(8080)));
        let v = Value::Mapping(m);
        // Note: by *value*, no `&`. This is the `serde_yaml` shape.
        let cfg: Config = from_value(v).unwrap();
        assert_eq!(cfg.port, 8080);
    }

    #[test]
    fn to_value_takes_by_value_like_serde_yaml() {
        let cfg = Config {
            name: "noyalib".into(),
            port: 8080,
        };
        // By value, no `&`.
        let v = to_value(cfg).unwrap();
        match v {
            Value::Mapping(m) => {
                assert_eq!(m.get("name"), Some(&Value::String("noyalib".into())));
            }
            _ => panic!("expected Mapping"),
        }
    }

    #[test]
    fn round_trip_via_to_string_from_str() {
        let cfg = Config {
            name: "noyalib".into(),
            port: 8080,
        };
        let s = to_string(&cfg).unwrap();
        let back: Config = from_str(&s).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn multi_doc_stream() {
        let yaml = "name: a\nport: 1\n---\nname: b\nport: 2\n";
        let docs: Vec<Config> = from_str_multi(yaml).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].port, 1);
        assert_eq!(docs[1].port, 2);
    }

    #[test]
    fn error_type_wraps_noyalib_error() {
        // Since the behavioural-shim rework `serde_yaml::Error` is a
        // newtype rendering upstream's wording; the underlying
        // noyalib error stays reachable for callers that want it.
        let e = Error::from(crate::error::Error::Custom("compat".into()));
        assert!(e.to_string().contains("compat"), "{e}");
        assert!(matches!(e.inner(), crate::error::Error::Custom(_)));
        let _inner: crate::error::Error = e.into_inner();
    }

    #[test]
    fn behavioural_defaults_follow_serde_yaml() {
        // The two spot checks migrants hit first: `<<` stays a
        // literal key, and a leading-zero integer stays a string —
        // both the opposite of noyalib's spec-strict defaults, both
        // exactly what upstream did. The full 18-case contract lives
        // in tests/serde_yaml_contract.rs.
        let v: Value = from_str(
            "a: &a {x: 1}
b:
  <<: *a
",
        )
        .unwrap();
        assert!(v["b"].as_mapping().unwrap().contains_key("<<"));
        let v: Value = from_str(
            "n: 0123
",
        )
        .unwrap();
        assert_eq!(v["n"].as_str(), Some("0123"));
    }

    #[test]
    fn deserializer_type_re_exports_under_serde_yaml_name() {
        // Compile-time check: `serde_yaml::Deserializer` resolves to
        // noyalib's own `Deserializer<'_>` so existing call sites
        // that explicitly name the type compile unchanged.
        let v = Value::from(7_i64);
        let de = Deserializer::new(&v);
        let n: i32 = serde_core::Deserialize::deserialize(de).unwrap();
        assert_eq!(n, 7);
    }

    #[test]
    fn serializer_type_re_exports_under_serde_yaml_name() {
        // Compile-time check: `serde_yaml::Serializer` resolves to
        // noyalib's own `Serializer`. The full streaming-serializer
        // surface is documented on `crate::ser::Serializer`; here we
        // just verify the type is reachable via the compat path.
        let _ = Serializer;
    }

    #[test]
    fn value_submodule_path_resolves() {
        // `use serde_yaml::value::{Value, Mapping, Number};` is a
        // common idiom; verify the path-form import resolves to the
        // same types as the crate-root re-exports.
        use super::value::{Mapping as MappingV, Number as NumberV, Value as ValueV};
        let mut m = MappingV::new();
        let _ = m.insert("k", ValueV::Number(NumberV::Integer(1)));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn mapping_submodule_path_resolves() {
        use super::mapping::Mapping as MappingAlias;
        let m: MappingAlias = MappingAlias::new();
        assert!(m.is_empty());
    }

    #[test]
    fn with_submodule_path_resolves() {
        // Compile-time check: every helper documented on
        // `serde_yaml::with::*` is reachable via
        // `noyalib::compat::serde_yaml::with::*`. No runtime
        // assertion — the import itself is the test.
        #[allow(unused_imports)]
        use super::with::{
            nested_singleton_map, singleton_map, singleton_map_optional, singleton_map_recursive,
            singleton_map_with,
        };
    }

    #[test]
    fn error_exposes_location_line_column() {
        // The `serde_yaml::Error::location()` → `Location::line()`
        // / `Location::column()` chain is the contract that any
        // diagnostic-emitting migrant relies on. noyalib's `Error`
        // exposes the exact same shape, 1-indexed.
        let err = from_str::<Value>("a: [unclosed").unwrap_err();
        let loc = err.location().expect("parse error must carry a location");
        assert!(loc.line() >= 1);
        assert!(loc.column() >= 1);
        let _: usize = loc.index();
    }
}
