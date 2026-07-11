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
//!
//! ```toml
//! # Cargo.toml — before
//! serde_yaml = "0.9"
//! # Cargo.toml — after
//! noyalib = { version = "0.0.7", features = ["compat-serde-yaml"] }
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
//! # Behavioural divergences from upstream `serde_yaml` 0.9
//!
//! The shim exposes the same surface but is backed by noyalib's
//! deserialiser. Two behaviours differ from upstream — both
//! changes that noyalib intentionally ships with safer defaults:
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
//!   [`doc/MIGRATION-FROM-SERDE-YAML.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/MIGRATION-FROM-SERDE-YAML.md#1-valuetagged-is-a-7th-variant--and-noyalib-preserves-scalar-tags-too)
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
use serde::Serialize;
use serde::de::DeserializeOwned;

// ── Types — re-exported under the serde_yaml names ───────────────────

pub use crate::error::{Error, Location, Result};
pub use crate::value::{Mapping, Number, Sequence, Tag, TaggedValue, Value};

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

/// Deserialize a YAML document into the target type.
///
/// Direct re-export of [`crate::from_str`] — same signature,
/// same behaviour.
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
    T: DeserializeOwned + 'static,
{
    crate::from_str(s)
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
    T: DeserializeOwned + 'static,
{
    crate::from_slice(bytes)
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
pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    R: std::io::Read,
    T: DeserializeOwned + 'static,
{
    crate::from_reader(reader)
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
    T: DeserializeOwned + 'static,
{
    crate::from_value(&value)
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
    T: Serialize,
{
    crate::to_string(value)
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
    T: Serialize,
{
    crate::to_writer(writer, value)
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
    T: Serialize,
{
    crate::to_value(&value)
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
    T: DeserializeOwned + 'static,
{
    crate::load_all_as::<T>(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
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
    fn error_type_re_export_is_noyalib_error() {
        // Compile-time check: `serde_yaml::Error` and
        // `noyalib::Error` are the same type, so callers' existing
        // error-handling code keeps working.
        #[allow(unused_qualifications)]
        fn identity(e: super::Error) -> crate::error::Error {
            e
        }
        // Exercise it at runtime so the coincidence of the two paths is
        // observed, not merely compiled.
        let e = identity(Error::Custom("compat".into()));
        assert!(e.to_string().contains("compat"), "{e}");
    }

    #[test]
    fn deserializer_type_re_exports_under_serde_yaml_name() {
        // Compile-time check: `serde_yaml::Deserializer` resolves to
        // noyalib's own `Deserializer<'_>` so existing call sites
        // that explicitly name the type compile unchanged.
        let v = Value::from(7_i64);
        let de: Deserializer<'_> = Deserializer::new(&v);
        let n: i32 = Deserialize::deserialize(de).unwrap();
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
