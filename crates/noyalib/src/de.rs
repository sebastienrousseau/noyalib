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
use crate::span_context;
use crate::value::{Number, Value};
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
#[cfg(feature = "std")]
use std::io;

/// Which version of the YAML specification the resolver follows.
///
/// YAML 1.2 (the default) and 1.1 differ in their plain-scalar
/// resolution table:
///
/// | Form | 1.2 (core schema) | 1.1 |
/// |---|---|---|
/// | `yes` / `no` / `on` / `off` | string | bool |
/// | `0644` | int 644 (decimal) | int 420 (octal) |
/// | `60:00` | string | int 3 600 (sexagesimal) |
/// | `.nan` / `.inf` | float | float (same) |
/// | `true` / `false` | bool | bool (same) |
///
/// Selecting a version is a preset over the three `legacy_*` flags;
/// see [`ParserConfig::version`] for the full mapping.
///
/// # Examples
///
/// ```
/// use noyalib::{from_str_with_config, ParserConfig, Value, YamlVersion};
///
/// let cfg = ParserConfig::new().version(YamlVersion::V1_1);
/// let v: Value = from_str_with_config("yes", &cfg).unwrap();
/// assert_eq!(v, Value::Bool(true));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum YamlVersion {
    /// YAML 1.2 (2009) core schema. Default. Strict `true` / `false`
    /// booleans; no bare octal; no sexagesimal.
    #[default]
    V1_2,
    /// YAML 1.1 (2005). Broad resolver: `yes` / `no` / `on` / `off`
    /// are booleans; `0644` is octal; `60:00` is sexagesimal.
    V1_1,
}

/// Deserialization configuration.
///
/// All fields are public, but the struct is annotated
/// [`#[non_exhaustive]`][nex] so that adding a new budget or
/// policy in a future minor release is **not** a breaking change.
/// Construct with [`ParserConfig::new`] / [`ParserConfig::strict`]
/// / [`ParserConfig::default`] (preferred) or with the
/// `..ParserConfig::default()` struct-update form; do not
/// construct from an exhaustive struct-literal outside this
/// crate.
///
/// [nex]: https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute
///
/// # Examples
///
/// ```
/// use noyalib::ParserConfig;
/// let cfg = ParserConfig::new().max_depth(64);
/// assert_eq!(cfg.max_depth, 64);
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ParserConfig {
    /// Which YAML specification version to honour during plain-scalar
    /// resolution.
    ///
    /// YAML 1.2 (default) follows the **core schema** — strict
    /// `true`/`false` booleans, no bare `0`-prefix octal, no
    /// sexagesimal `60:00` integers. YAML 1.1 broadens the resolver
    /// to accept all of those legacy forms.
    ///
    /// Setting this to [`YamlVersion::V1_1`] is equivalent to flipping
    /// every `legacy_*` flag (`legacy_booleans`, `legacy_octal_numbers`,
    /// `legacy_sexagesimal`) on at once. The `legacy_*` flags remain
    /// available for fine-grained overrides — version selection sets a
    /// preset, individual flags refine it.
    pub yaml_version: YamlVersion,
    /// Maximum recursion depth allowed during parsing (default: 128).
    pub max_depth: usize,
    /// Maximum length of a single YAML document in bytes (default: 64 MB).
    pub max_document_length: usize,
    /// Maximum number of times a single anchor can be expanded (default: 1024).
    pub max_alias_expansions: usize,
    /// Maximum number of keys allowed in a single mapping (default: 64k).
    pub max_mapping_keys: usize,
    /// Maximum number of elements allowed in a single sequence (default: 64k).
    pub max_sequence_length: usize,
    /// Maximum total parser events emitted across the input
    /// (default: 1 000 000). Caps event-stream amplification
    /// independent of recursion depth or alias count. Trips
    /// [`crate::Error::Budget`] with
    /// [`crate::BudgetBreach::MaxEvents`].
    pub max_events: usize,
    /// Maximum total `Value` nodes built into the AST
    /// (default: 250 000). Trips
    /// [`crate::BudgetBreach::MaxNodes`].
    pub max_nodes: usize,
    /// Maximum cumulative scalar-byte count across the document
    /// (default: 64 MB). Distinct from
    /// [`Self::max_document_length`] (input size) — this caps
    /// scalar payload after alias expansion. Trips
    /// [`crate::BudgetBreach::MaxTotalScalarBytes`].
    pub max_total_scalar_bytes: usize,
    /// Maximum number of documents in a multi-document stream
    /// (default: 1 000). Trips
    /// [`crate::BudgetBreach::MaxDocuments`].
    pub max_documents: usize,
    /// Maximum number of merge-key (`<<`) entries across the
    /// document (default: 10 000). Trips
    /// [`crate::BudgetBreach::MaxMergeKeys`].
    pub max_merge_keys: usize,
    /// Optional alias-to-anchor ratio heuristic for detecting
    /// billion-laughs amplification patterns
    /// (default: `Some(10.0)`). When more than `ratio × anchors`
    /// aliases have been resolved, the parser trips
    /// [`crate::BudgetBreach::AliasAnchorRatio`]. Set to `None`
    /// to disable.
    pub alias_anchor_ratio: Option<f64>,
    /// How to handle duplicate keys in a mapping (default: Last, per YAML 1.2).
    pub duplicate_key_policy: DuplicateKeyPolicy,
    /// If true, only `true` and `false` (lowercase) are accepted as booleans.
    pub strict_booleans: bool,
    /// If true, accepts YAML 1.1 booleans like `yes`, `no`, `on`, `off`.
    pub legacy_booleans: bool,
    /// Optional registry of custom tags to strip on the streaming path.
    ///
    /// See [`TagRegistry`](crate::TagRegistry) for the full rationale.
    /// `None` (default) preserves the legacy behaviour of routing every
    /// custom-tagged value through the AST fallback.
    pub tag_registry: Option<Arc<crate::TagRegistry>>,
    /// How the YAML merge key (`<<`) should be handled.
    ///
    /// See [`MergeKeyPolicy`] for the available policies. The
    /// default is [`MergeKeyPolicy::Auto`] — the YAML 1.2 spec
    /// behaviour where `<<:` triggers automatic mapping merge.
    pub merge_key_policy: MergeKeyPolicy,
    /// When `true`, plain scalars are *never* resolved to
    /// `null` / `bool` / `int` / `float` — every plain scalar
    /// becomes a string. Useful for schema-strict pipelines that
    /// require the user to quote intent explicitly. Default
    /// `false`.
    pub no_schema: bool,
    /// When `true`, accept YAML 1.1-style bare `0`-prefix octal
    /// literals (e.g. `0644` parsed as 420) in addition to the
    /// YAML 1.2 `0o644` form. Default `false` to honour the YAML
    /// 1.2 schema.
    pub legacy_octal_numbers: bool,
    /// When `true`, deserializing `!!binary "ABCD"` into a
    /// [`String`] target yields the literal base64 source string
    /// (`"ABCD"`) rather than rejecting on tag mismatch. The
    /// canonical bytes path (`Vec<u8>`,
    /// `serde_bytes::ByteBuf`) still decodes the base64 payload
    /// either way. Useful for migrations from Python pyyaml-style
    /// applications that treat the tag as advisory. Default
    /// `false`.
    pub ignore_binary_tag_for_string: bool,
    /// When `true`, accept YAML 1.1-style **sexagesimal** numbers
    /// (`60:00`, `1:30:00`) as integers. The colon-separated
    /// digits are interpreted in base 60: each component is
    /// multiplied by an increasing power of 60, summed left to
    /// right. `60:00` → 3 600; `1:30:00` → 5 400. Negative values
    /// (`-1:30:00`) and partial signs are honoured.
    ///
    /// Off by default to honour the YAML 1.2 schema. Useful for
    /// migrations from YAML 1.1 / Ruby / pyyaml configs that use
    /// the legacy time-of-day notation.
    pub legacy_sexagesimal: bool,
    /// Indentation-validation mode. See [`RequireIndent`].
    /// Default: [`RequireIndent::Unchecked`] — accept any
    /// well-formed YAML indent.
    pub require_indent: RequireIndent,
    /// Pluggable "Safe YAML" policies, run during parsing.
    ///
    /// Each [`Policy`](crate::policy::Policy) inspects parser
    /// events and the post-parse [`Value`] tree; any policy
    /// returning `Err(...)` aborts the parse with that diagnostic.
    /// Empty by default.
    ///
    /// Use [`ParserConfig::with_policy`] to register a policy.
    /// When at least one policy is present the streaming fast-path
    /// is bypassed automatically so the policy contract holds for
    /// every code path.
    pub policies: Vec<Arc<dyn crate::policy::Policy>>,
    /// `${KEY}` / `${KEY:-default}` substitution table consulted
    /// after parsing every document.
    ///
    /// Each scalar in the resulting [`Value`] tree is walked and
    /// any `${name}` placeholder is replaced with the property of
    /// that name. Supported syntax:
    ///
    /// - `${name}` — substitute, error or pass through depending
    ///   on [`Self::strict_properties`]
    /// - `${name:-default}` — substitute, falling back to
    ///   `default` when `name` is missing (always silent, never
    ///   surfaces in errors)
    /// - `${{` — literal `${` (escape for the open delimiter)
    /// - `$$` — literal `$`
    /// - `}}` — literal `}`
    ///
    /// `None` (default) disables the substitution pass entirely;
    /// the parser is unchanged. Setting a non-empty map forces the
    /// AST fallback so the post-parse walk runs uniformly across
    /// every typed target.
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub properties: Option<Arc<std::collections::HashMap<String, String>>>,
    /// When `true`, an unknown `${name}` placeholder (no entry in
    /// [`Self::properties`] and no `:-default` fallback) aborts
    /// the parse with [`Error::Custom`].
    /// When `false` (default), unknown placeholders are replaced
    /// with the empty string — the lossy semantics matching
    /// [`Value::interpolate_properties_lossy`](crate::Value::interpolate_properties_lossy).
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub strict_properties: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        ParserConfig {
            yaml_version: YamlVersion::V1_2,
            max_depth: 128,
            max_document_length: 1024 * 1024 * 64, // 64 MB
            max_alias_expansions: 1024,
            max_mapping_keys: 1024 * 64,
            max_sequence_length: 1024 * 64,
            max_events: 1_000_000,
            max_nodes: 250_000,
            max_total_scalar_bytes: 1024 * 1024 * 64, // 64 MB
            max_documents: 1_000,
            max_merge_keys: 10_000,
            alias_anchor_ratio: Some(10.0),
            duplicate_key_policy: DuplicateKeyPolicy::default(),
            strict_booleans: false,
            legacy_booleans: false,
            tag_registry: None,
            merge_key_policy: MergeKeyPolicy::default(),
            no_schema: false,
            legacy_octal_numbers: false,
            ignore_binary_tag_for_string: false,
            legacy_sexagesimal: false,
            require_indent: RequireIndent::Unchecked,
            policies: Vec::new(),
            #[cfg(feature = "std")]
            properties: None,
            #[cfg(feature = "std")]
            strict_properties: false,
        }
    }
}

impl ParserConfig {
    /// Create a new configuration with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new();
    /// assert_eq!(cfg.max_depth, 128);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strict configuration (YAML 1.2 strict) with tighter
    /// security limits suitable for untrusted input.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::strict();
    /// assert_eq!(cfg.max_depth, 64);
    /// ```
    #[must_use]
    pub fn strict() -> Self {
        ParserConfig {
            yaml_version: YamlVersion::V1_2,
            max_depth: 64,
            max_document_length: 1024 * 1024, // 1 MB
            max_alias_expansions: 100,
            max_mapping_keys: 1024,
            max_sequence_length: 1024,
            max_events: 100_000,
            max_nodes: 25_000,
            max_total_scalar_bytes: 1024 * 1024, // 1 MB
            max_documents: 100,
            max_merge_keys: 1_000,
            alias_anchor_ratio: Some(5.0),
            strict_booleans: true,
            legacy_booleans: false,
            duplicate_key_policy: DuplicateKeyPolicy::Error,
            tag_registry: None,
            merge_key_policy: MergeKeyPolicy::default(),
            no_schema: false,
            legacy_octal_numbers: false,
            ignore_binary_tag_for_string: false,
            legacy_sexagesimal: false,
            require_indent: RequireIndent::Even,
            policies: Vec::new(),
            #[cfg(feature = "std")]
            properties: None,
            #[cfg(feature = "std")]
            strict_properties: true,
        }
    }

    /// Install a `${KEY}` substitution table consulted after
    /// parsing.
    ///
    /// Each scalar in the resulting [`Value`] tree is walked and
    /// any `${name}` placeholder is replaced with the property of
    /// that name. Pairs with [`Self::strict_properties`] to choose
    /// between erroring or silently empty-substituting on unknown
    /// keys, and with `${name:-default}` syntax for inline
    /// defaults.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str_with_config, ParserConfig, Value};
    /// use std::collections::HashMap;
    /// use std::sync::Arc;
    ///
    /// let mut props = HashMap::new();
    /// props.insert("HOST".to_string(), "localhost".to_string());
    /// let cfg = ParserConfig::new().properties(Arc::new(props));
    /// let v: Value = from_str_with_config("url: http://${HOST}/", &cfg).unwrap();
    /// assert_eq!(v["url"].as_str(), Some("http://localhost/"));
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    #[must_use]
    pub fn properties(
        mut self,
        properties: Arc<std::collections::HashMap<String, String>>,
    ) -> Self {
        self.properties = Some(properties);
        self
    }

    /// Toggle strict-mode placeholder resolution.
    ///
    /// When `true`, an unknown `${name}` (no map entry, no
    /// `:-default` fallback) aborts the parse. When `false`
    /// (default), unknown placeholders are replaced with the empty
    /// string — useful for environment-style configs where missing
    /// variables should silently degrade.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str_with_config, ParserConfig, Value};
    /// use std::collections::HashMap;
    /// use std::sync::Arc;
    ///
    /// let cfg = ParserConfig::new()
    ///     .properties(Arc::new(HashMap::new()))
    ///     .strict_properties(true);
    /// let res: Result<Value, _> = from_str_with_config("x: ${MISSING}", &cfg);
    /// assert!(res.is_err());
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    #[must_use]
    pub fn strict_properties(mut self, strict: bool) -> Self {
        self.strict_properties = strict;
        self
    }

    /// Select the YAML specification version the resolver should
    /// honour.
    ///
    /// Selecting [`YamlVersion::V1_1`] is a *preset* over the three
    /// `legacy_*` flags — equivalent to:
    ///
    /// ```text
    /// cfg.legacy_booleans      = true;  // yes / no / on / off
    /// cfg.legacy_octal_numbers = true;  // 0644 → octal
    /// cfg.legacy_sexagesimal   = true;  // 60:00 → 3600
    /// ```
    ///
    /// Selecting [`YamlVersion::V1_2`] resets those three flags to
    /// `false` so callers can revert to strict 1.2 mode without
    /// re-creating the config from scratch. Other fields (limits,
    /// policies, merge-key behaviour) are unaffected.
    ///
    /// Fine-grained overrides (e.g. "1.1 booleans but reject octal
    /// `0644`") work as expected: call `version` first, then flip
    /// individual flags.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str_with_config, ParserConfig, Value, YamlVersion};
    ///
    /// let cfg = ParserConfig::new().version(YamlVersion::V1_1);
    /// // YAML 1.1 booleans
    /// let v: Value = from_str_with_config("on", &cfg).unwrap();
    /// assert_eq!(v, Value::Bool(true));
    /// // YAML 1.1 octal
    /// let v: Value = from_str_with_config("0644", &cfg).unwrap();
    /// assert_eq!(v, Value::from(420_i64));
    /// // YAML 1.1 sexagesimal
    /// let v: Value = from_str_with_config("1:30", &cfg).unwrap();
    /// assert_eq!(v, Value::from(90_i64));
    /// ```
    #[must_use]
    pub fn version(mut self, version: YamlVersion) -> Self {
        self.yaml_version = version;
        match version {
            YamlVersion::V1_1 => {
                self.legacy_booleans = true;
                self.legacy_octal_numbers = true;
                self.legacy_sexagesimal = true;
            }
            YamlVersion::V1_2 => {
                self.legacy_booleans = false;
                self.legacy_octal_numbers = false;
                self.legacy_sexagesimal = false;
            }
        }
        self
    }

    /// Register a [`Policy`](crate::policy::Policy) to enforce
    /// during parsing.
    ///
    /// Multiple policies may be registered; they all run in
    /// registration order, and the first error short-circuits the
    /// parse. When any policy is present the streaming fast-path
    /// is bypassed so the policy contract is enforced uniformly.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{from_str_with_config, ParserConfig, Value};
    /// use noyalib::policy::DenyAnchors;
    ///
    /// let cfg = ParserConfig::new().with_policy(DenyAnchors);
    /// let res: Result<Value, _> =
    ///     from_str_with_config("a: &x 1\nb: *x\n", &cfg);
    /// assert!(res.is_err());
    /// ```
    #[must_use]
    pub fn with_policy<P>(mut self, policy: P) -> Self
    where
        P: crate::policy::Policy + 'static,
    {
        self.policies.push(Arc::new(policy));
        self
    }

    /// Set the maximum recursion depth.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_depth(32);
    /// assert_eq!(cfg.max_depth, 32);
    /// ```
    #[must_use]
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set the maximum document length.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_document_length(1024);
    /// assert_eq!(cfg.max_document_length, 1024);
    /// ```
    #[must_use]
    pub fn max_document_length(mut self, len: usize) -> Self {
        self.max_document_length = len;
        self
    }

    /// Set the maximum alias expansions.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_alias_expansions(50);
    /// assert_eq!(cfg.max_alias_expansions, 50);
    /// ```
    #[must_use]
    pub fn max_alias_expansions(mut self, expansions: usize) -> Self {
        self.max_alias_expansions = expansions;
        self
    }

    /// Set the maximum number of mapping keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_mapping_keys(100);
    /// assert_eq!(cfg.max_mapping_keys, 100);
    /// ```
    #[must_use]
    pub fn max_mapping_keys(mut self, max: usize) -> Self {
        self.max_mapping_keys = max;
        self
    }

    /// Set the maximum sequence length.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_sequence_length(100);
    /// assert_eq!(cfg.max_sequence_length, 100);
    /// ```
    #[must_use]
    pub fn max_sequence_length(mut self, max: usize) -> Self {
        self.max_sequence_length = max;
        self
    }

    /// Set the maximum total parser-event budget.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_events(50_000);
    /// assert_eq!(cfg.max_events, 50_000);
    /// ```
    #[must_use]
    pub fn max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }

    /// Set the maximum total `Value` node budget.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_nodes(10_000);
    /// assert_eq!(cfg.max_nodes, 10_000);
    /// ```
    #[must_use]
    pub fn max_nodes(mut self, max: usize) -> Self {
        self.max_nodes = max;
        self
    }

    /// Set the maximum cumulative scalar-byte budget.
    ///
    /// Distinct from [`Self::max_document_length`] — this caps
    /// scalar bytes after alias expansion.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_total_scalar_bytes(8 * 1024 * 1024);
    /// assert_eq!(cfg.max_total_scalar_bytes, 8 * 1024 * 1024);
    /// ```
    #[must_use]
    pub fn max_total_scalar_bytes(mut self, max: usize) -> Self {
        self.max_total_scalar_bytes = max;
        self
    }

    /// Set the maximum document count for multi-document streams.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_documents(64);
    /// assert_eq!(cfg.max_documents, 64);
    /// ```
    #[must_use]
    pub fn max_documents(mut self, max: usize) -> Self {
        self.max_documents = max;
        self
    }

    /// Set the maximum merge-key count budget.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().max_merge_keys(1_000);
    /// assert_eq!(cfg.max_merge_keys, 1_000);
    /// ```
    #[must_use]
    pub fn max_merge_keys(mut self, max: usize) -> Self {
        self.max_merge_keys = max;
        self
    }

    /// Set the indentation-validation mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{ParserConfig, RequireIndent};
    /// let cfg = ParserConfig::new().require_indent(RequireIndent::Even);
    /// assert_eq!(cfg.require_indent, RequireIndent::Even);
    /// ```
    #[must_use]
    pub fn require_indent(mut self, mode: RequireIndent) -> Self {
        self.require_indent = mode;
        self
    }

    /// Set the alias-to-anchor ratio heuristic.
    ///
    /// Pass `Some(ratio)` to enable the billion-laughs guard,
    /// `None` to disable.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().alias_anchor_ratio(Some(20.0));
    /// assert_eq!(cfg.alias_anchor_ratio, Some(20.0));
    /// ```
    #[must_use]
    pub fn alias_anchor_ratio(mut self, ratio: Option<f64>) -> Self {
        self.alias_anchor_ratio = ratio;
        self
    }

    /// Set the duplicate key policy.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{DuplicateKeyPolicy, ParserConfig};
    /// let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    /// assert_eq!(cfg.duplicate_key_policy, DuplicateKeyPolicy::Error);
    /// ```
    #[must_use]
    pub fn duplicate_key_policy(mut self, policy: DuplicateKeyPolicy) -> Self {
        self.duplicate_key_policy = policy;
        self
    }

    /// Enable or disable strict booleans.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().strict_booleans(true);
    /// assert!(cfg.strict_booleans);
    /// ```
    #[must_use]
    pub fn strict_booleans(mut self, strict: bool) -> Self {
        self.strict_booleans = strict;
        self
    }

    /// Enable or disable legacy booleans.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().legacy_booleans(true);
    /// assert!(cfg.legacy_booleans);
    /// ```
    #[must_use]
    pub fn legacy_booleans(mut self, legacy: bool) -> Self {
        self.legacy_booleans = legacy;
        self
    }

    /// Attach a [`TagRegistry`](crate::TagRegistry) so the streaming
    /// deserializer strips listed custom tags instead of routing them
    /// through the AST.
    ///
    /// See the [`tag_registry`](crate::tag_registry) module
    /// documentation for when to use this versus `#[serde(rename)]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{ParserConfig, TagRegistry};
    /// use std::sync::Arc;
    /// let reg = Arc::new(TagRegistry::new().with("!Celsius"));
    /// let cfg = ParserConfig::new().tag_registry(Arc::clone(&reg));
    /// assert!(cfg.tag_registry.is_some());
    /// ```
    #[must_use]
    pub fn tag_registry(mut self, registry: Arc<crate::TagRegistry>) -> Self {
        self.tag_registry = Some(registry);
        self
    }

    /// Set the policy for handling the YAML merge key (`<<`).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{MergeKeyPolicy, ParserConfig};
    /// let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::AsOrdinary);
    /// assert_eq!(cfg.merge_key_policy, MergeKeyPolicy::AsOrdinary);
    /// ```
    #[must_use]
    pub fn merge_key_policy(mut self, policy: MergeKeyPolicy) -> Self {
        self.merge_key_policy = policy;
        self
    }

    /// Toggle schema-free plain-scalar resolution. When `true`,
    /// every plain scalar becomes a string regardless of whether
    /// it would normally resolve to `null`, `bool`, integer, or
    /// float.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().no_schema(true);
    /// assert!(cfg.no_schema);
    /// ```
    #[must_use]
    pub fn no_schema(mut self, no_schema: bool) -> Self {
        self.no_schema = no_schema;
        self
    }

    /// Toggle YAML 1.1-style bare `0`-prefix octal parsing
    /// (e.g. `0644` → 420). Off by default; YAML 1.2 requires the
    /// `0o` prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().legacy_octal_numbers(true);
    /// assert!(cfg.legacy_octal_numbers);
    /// ```
    #[must_use]
    pub fn legacy_octal_numbers(mut self, on: bool) -> Self {
        self.legacy_octal_numbers = on;
        self
    }

    /// Toggle the migration-helper behaviour where
    /// `!!binary "ABCD"` deserializes into a [`String`] target as
    /// the literal base64 source string. The canonical bytes
    /// path (`Vec<u8>`, `serde_bytes::ByteBuf`) is unaffected —
    /// it always decodes the base64 payload.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().ignore_binary_tag_for_string(true);
    /// assert!(cfg.ignore_binary_tag_for_string);
    /// ```
    #[must_use]
    pub fn ignore_binary_tag_for_string(mut self, on: bool) -> Self {
        self.ignore_binary_tag_for_string = on;
        self
    }

    /// Toggle YAML 1.1-style sexagesimal number parsing
    /// (`60:00` → 3 600). Off by default; YAML 1.2 dropped the
    /// sexagesimal schema, so plain `1:30:00` would otherwise
    /// surface as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::ParserConfig;
    /// let cfg = ParserConfig::new().legacy_sexagesimal(true);
    /// assert!(cfg.legacy_sexagesimal);
    /// ```
    #[must_use]
    pub fn legacy_sexagesimal(mut self, on: bool) -> Self {
        self.legacy_sexagesimal = on;
        self
    }
}

/// Policy for handling the YAML merge key (`<<`) during parsing.
///
/// YAML 1.2 §10.2 defines `<<` as a "merge key" that, when used as
/// a mapping key, splices the value's mapping (or sequence of
/// mappings) into the enclosing mapping. The variants below let
/// callers opt out of that behaviour.
///
/// # Examples
///
/// Indentation-validation mode for the YAML scanner.
///
/// Issue #6 surface — every block-context indent transition is
/// classified per the chosen mode. The default
/// ([`RequireIndent::Unchecked`]) accepts any well-formed YAML
/// indent (the YAML 1.2 spec mandate), which is what every
/// other Rust YAML parser does.
///
/// Stricter modes are useful in pipelines that require uniform
/// indentation house style (linters, formatters, reviewer
/// gates) — a config file with mixed `2`-space and `4`-space
/// indent passes by-spec but fails consistency review.
///
/// # Variants
///
/// - [`RequireIndent::Unchecked`] (default) — by-spec mode.
/// - [`RequireIndent::Even`] — every indent delta must be even.
/// - `RequireIndent::Divisible(N)` — every indent delta must
///   be divisible by `N`.
/// - `RequireIndent::Uniform(Some(N))` — every indent delta
///   must equal `N`. `None` means "auto-detect from the first
///   delta and require the rest to match it".
///
/// # Examples
///
/// ```
/// use noyalib::{ParserConfig, RequireIndent};
/// let cfg = ParserConfig::new().require_indent(RequireIndent::Even);
/// assert_eq!(cfg.require_indent, RequireIndent::Even);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum RequireIndent {
    /// Accept any indent transition the YAML 1.2 spec allows.
    /// Default.
    #[default]
    Unchecked,
    /// Indent delta must be even (`2`, `4`, `6`, …). The most
    /// common house-style.
    Even,
    /// Indent delta must be divisible by `N`.
    Divisible(usize),
    /// `Some(N)`: every indent delta must equal `N`.
    /// `None`: the first delta sets the standard for the
    /// document; subsequent deltas must match it.
    Uniform(Option<usize>),
}

/// ```
/// use noyalib::MergeKeyPolicy;
/// assert_eq!(MergeKeyPolicy::default(), MergeKeyPolicy::Auto);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MergeKeyPolicy {
    /// Apply the YAML 1.2 merge-key semantics — `<<:` keys trigger
    /// automatic merge of the value into the enclosing mapping.
    /// Default.
    #[default]
    Auto,
    /// Treat `<<` as an ordinary string key. The mapping retains a
    /// literal `<<` entry whose value is whatever the YAML
    /// document supplied. Useful when round-tripping configuration
    /// files that happen to contain a `<<` key for non-merge
    /// reasons.
    AsOrdinary,
    /// Reject any document that contains a `<<` key with
    /// [`crate::Error::Custom`]. Useful for schema-strict pipelines
    /// where merge keys are forbidden.
    Error,
}

/// Policy for handling duplicate keys in a YAML mapping.
///
/// # Examples
///
/// ```
/// use noyalib::DuplicateKeyPolicy;
/// assert_eq!(DuplicateKeyPolicy::default(), DuplicateKeyPolicy::Last);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum DuplicateKeyPolicy {
    /// Use the first occurrence of the key; ignore subsequent ones.
    First,
    /// Use the last occurrence of the key (YAML 1.2 default).
    #[default]
    Last,
    /// Return an error if a duplicate key is encountered.
    Error,
}

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
        && properties_inactive(config);
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

/// A YAML deserializer.
///
/// # Examples
///
/// ```
/// use noyalib::{Deserializer, Value};
/// use serde::Deserialize;
/// let v = Value::from(42_i64);
/// let de = Deserializer::new(&v);
/// let n: i32 = Deserialize::deserialize(de).unwrap();
/// assert_eq!(n, 42);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Deserializer<'de> {
    pub(crate) value: &'de Value,
    pub(crate) span_ctx: Option<&'de span_context::SpanContext>,
    /// Per-call flag mirroring
    /// [`ParserConfig::ignore_binary_tag_for_string`]. When `true`,
    /// `!!binary "ABCD"` deserializes into `String` as the literal
    /// `"ABCD"` (no base64 decode). Default `false` preserves YAML
    /// 1.2 semantics.
    pub(crate) ignore_binary_tag_for_string: bool,
    /// When `true`, [`Value::Tagged`] is surfaced through the
    /// magic-key [`crate::value::TagPreservingMapAccess`] so the
    /// outer [`Value::deserialize`] visitor can reconstruct
    /// `Value::Tagged(...)` losslessly. When `false` (default), a
    /// tagged scalar is unwrapped to its inner value — the
    /// transparent behaviour every typed `T::deserialize` expects.
    ///
    /// Set automatically by [`from_str_with_config`] /
    /// [`from_value`] when the caller's `T` is `Value` (detected
    /// via [`std::any::TypeId`]). Threaded through every
    /// `descend()` site so nested tagged values inside a
    /// `Mapping` / `Sequence` also survive.
    pub(crate) preserve_tags: bool,
}

impl<'de> Deserializer<'de> {
    /// Create a new deserializer from a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::{Deserializer, Value};
    /// let v = Value::from(1_i64);
    /// let _de = Deserializer::new(&v);
    /// ```
    #[must_use]
    pub fn new(value: &'de Value) -> Self {
        Deserializer {
            value,
            span_ctx: None,
            ignore_binary_tag_for_string: false,
            preserve_tags: false,
        }
    }

    /// Create a new deserializer from a value with an associated span context.
    ///
    /// The span context carries source-location information used to attach
    /// line/column details to errors and `Spanned<T>` fields. This
    /// constructor is primarily used internally by `from_str`; most callers
    /// should prefer [`Deserializer::new`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Constructed internally by from_str — external callers use Deserializer::new.
    /// use noyalib::Deserializer;
    /// # let value = unimplemented!();
    /// # let span_ctx = unimplemented!();
    /// let _de = Deserializer::with_span_context(value, span_ctx);
    /// ```
    #[must_use]
    pub fn with_span_context(value: &'de Value, span_ctx: &'de span_context::SpanContext) -> Self {
        Deserializer {
            value,
            span_ctx: Some(span_ctx),
            ignore_binary_tag_for_string: false,
            preserve_tags: false,
        }
    }

    /// Pass-through constructor for the
    /// [`crate::ParserConfig::ignore_binary_tag_for_string`] flag.
    /// Used internally by [`from_str_with_config`] when the caller
    /// has opted in to the migration helper.
    pub(crate) fn with_options(
        value: &'de Value,
        span_ctx: Option<&'de span_context::SpanContext>,
        ignore_binary_tag_for_string: bool,
    ) -> Self {
        Deserializer {
            value,
            span_ctx,
            ignore_binary_tag_for_string,
            preserve_tags: false,
        }
    }

    /// Internal constructor used by [`from_str_with_config`] /
    /// [`from_value`] when the caller's `T` is detected as
    /// [`Value`] via [`std::any::TypeId`]. Sets `preserve_tags`
    /// so [`Value::Tagged`] survives the data-binding return
    /// path. See `Deserializer::preserve_tags` for the contract.
    pub(crate) fn with_options_preserving_tags(
        value: &'de Value,
        span_ctx: Option<&'de span_context::SpanContext>,
        ignore_binary_tag_for_string: bool,
    ) -> Self {
        Deserializer {
            value,
            span_ctx,
            ignore_binary_tag_for_string,
            preserve_tags: true,
        }
    }

    /// Construct a child deserializer for `value`, propagating the
    /// span context and every per-call config toggle from `self`.
    /// Used by every descent site (struct field, sequence element,
    /// tagged inner value) so the toggles survive the walk.
    pub(crate) fn descend(&self, value: &'de Value) -> Self {
        Deserializer {
            value,
            span_ctx: self.span_ctx,
            ignore_binary_tag_for_string: self.ignore_binary_tag_for_string,
            preserve_tags: self.preserve_tags,
        }
    }

    fn wrap_err<T>(&self, res: Result<T>) -> Result<T> {
        match res {
            Err(Error::Deserialize(msg)) => {
                if let Some(ctx) = self.span_ctx {
                    let ptr: *const Value = self.value;
                    let addr = ptr as usize;
                    if let Some(span) = ctx.spans.get(&addr) {
                        return Err(Error::deserialize_at(msg, &ctx.source, span.0));
                    }
                }
                Err(Error::Deserialize(msg))
            }
            _ => res,
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => self.wrap_err(visitor.visit_none()),
            Value::Bool(b) => self.wrap_err(visitor.visit_bool(*b)),
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_i64(*n)),
            Value::Number(Number::Float(n)) => self.wrap_err(visitor.visit_f64(*n)),
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            Value::Sequence(_) => self.deserialize_seq(visitor),
            Value::Mapping(_) => self.deserialize_map(visitor),
            Value::Tagged(tagged) => {
                if self.preserve_tags {
                    // Tag-preserving path (`from_str::<Value>` /
                    // `from_value::<Value>`): surface the tag via
                    // a magic-key MapAccess so the outer
                    // `Value::deserialize` visitor reconstructs
                    // `Value::Tagged(...)` losslessly.
                    self.wrap_err(visitor.visit_map(crate::value::TagPreservingMapAccess::new(
                        tagged.tag().as_str(),
                        tagged.value(),
                    )))
                } else {
                    // Default path: typed targets see through the
                    // tag transparently — `#[derive(Deserialize)]
                    // struct Foo { x: i32 }` against `!Foo {x: 1}`
                    // yields `Foo { x: 1 }`.
                    let de = self.descend(tagged.value());
                    de.deserialize_any(visitor)
                }
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Bool(b) => self.wrap_err(visitor.visit_bool(*b)),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "bool",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_i64(*n)),
            Value::Number(Number::Float(n))
                if n.fract() == 0.0
                    && *n >= i64::MIN as f64
                    && *n <= i64::MAX as f64
                    && !n.is_nan() =>
            {
                self.wrap_err(visitor.visit_i64(*n as i64))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "integer",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(Number::Integer(n)) if *n >= 0 => {
                self.wrap_err(visitor.visit_u64(*n as u64))
            }
            Value::Number(Number::Float(n))
                if n.fract() == 0.0 && *n >= 0.0 && *n <= u64::MAX as f64 && !n.is_nan() =>
            {
                self.wrap_err(visitor.visit_u64(*n as u64))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "unsigned integer",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(Number::Float(n)) => self.wrap_err(visitor.visit_f64(*n)),
            Value::Number(Number::Integer(n)) => self.wrap_err(visitor.visit_f64(*n as f64)),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "float",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) if s.chars().count() == 1 => {
                self.wrap_err(visitor.visit_char(s.chars().next().unwrap()))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "char",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            // Migration helper: when the source declared
            // `!!binary "ABCD"` and the caller opted in to
            // `ignore_binary_tag_for_string`, surface the literal
            // source string rather than rejecting on tag mismatch.
            // The base64 encoding stays as the user-facing value;
            // the application layer can decode (or not) as it
            // sees fit.
            Value::Tagged(boxed)
                if self.ignore_binary_tag_for_string && is_binary_tag(boxed.tag().as_str()) =>
            {
                match boxed.value() {
                    Value::String(s) => self.wrap_err(visitor.visit_str(s)),
                    other => self.wrap_err(Err(Error::TypeMismatch {
                        expected: "string-shaped !!binary content",
                        found: type_name(other),
                    })),
                }
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "string",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) => self.wrap_err(visitor.visit_bytes(s.as_bytes())),
            // YAML 1.2.2 §10.4: `!!binary` carries an RFC 4648
            // base64-encoded payload. Decode on demand when a serde
            // target asks for bytes / a byte buffer (Vec<u8>,
            // serde_bytes::ByteBuf, &[u8] via owned visit).
            Value::Tagged(boxed) if is_binary_tag(boxed.tag().as_str()) => match boxed.value() {
                Value::String(s) => match crate::base64::decode(s) {
                    Ok(bytes) => self.wrap_err(visitor.visit_byte_buf(bytes)),
                    Err(why) => self.wrap_err(Err(Error::Deserialize(format!("!!binary: {why}")))),
                },
                other => self.wrap_err(Err(Error::TypeMismatch {
                    expected: "string-shaped !!binary content",
                    found: type_name(other),
                })),
            },
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "bytes",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => self.wrap_err(visitor.visit_none()),
            _ => self.wrap_err(visitor.visit_some(self)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => self.wrap_err(visitor.visit_unit()),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "null",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return visitor.visit_map(SpannedMapAccess::new(self.value, self.span_ctx));
        }
        self.wrap_err(visitor.visit_newtype_struct(self))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Sequence(seq) => {
                self.wrap_err(visitor.visit_seq(ValueSeqAccess::from_de(&self, seq)))
            }
            // Tagged values are transparent for typed `deserialize_*`
            // calls — `Vec<T>::deserialize` against `!List [1, 2, 3]`
            // (which now surfaces as `Tagged(Sequence(...))` per the
            // tag-preserving loader) sees through the wrapper.
            Value::Tagged(tagged) => self.descend(tagged.value()).deserialize_seq(visitor),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "sequence",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Mapping(map) => {
                self.wrap_err(visitor.visit_map(ValueMapAccess::from_de(&self, map)))
            }
            // Tagged values are transparent for typed
            // `deserialize_*` calls — `HashMap::deserialize`
            // against `!!set { Mark, Sammy }` (which now surfaces
            // as `Tagged(Mapping(...))` per the tag-preserving
            // loader) sees through the wrapper.
            Value::Tagged(tagged) => self.descend(tagged.value()).deserialize_map(visitor),
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "mapping",
                found: type_name(self.value),
            })),
        }
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
        if name == crate::spanned::SPANNED_TYPE_NAME {
            return visitor.visit_map(SpannedMapAccess::new(self.value, self.span_ctx));
        }
        self.deserialize_map(visitor)
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
        match self.value {
            Value::String(variant) => {
                let de: de::value::StrDeserializer<'de, Error> =
                    variant.as_str().into_deserializer();
                self.wrap_err(visitor.visit_enum(de))
            }
            Value::Mapping(map) if map.len() == 1 => {
                let (variant, value) = map.iter().next().unwrap();
                self.wrap_err(visitor.visit_enum(EnumAccess {
                    variant,
                    value,
                    span_ctx: self.span_ctx,
                }))
            }
            _ => self.wrap_err(Err(Error::TypeMismatch {
                expected: "string or single-key mapping",
                found: type_name(self.value),
            })),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(s) => self.wrap_err(visitor.visit_str(s)),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.wrap_err(visitor.visit_unit())
    }
}

pub(crate) struct ValueSeqAccess<'de> {
    iter: core::slice::Iter<'de, Value>,
    span_ctx: Option<&'de span_context::SpanContext>,
    ignore_binary_tag_for_string: bool,
    /// Mirror of the parent [`Deserializer::preserve_tags`] so
    /// nested `Value::Tagged(...)` nodes inside a sequence
    /// survive the data-binding return path.
    preserve_tags: bool,
}

impl<'de> ValueSeqAccess<'de> {
    pub(crate) fn from_de(de: &Deserializer<'de>, seq: &'de [Value]) -> Self {
        ValueSeqAccess {
            iter: seq.iter(),
            span_ctx: de.span_ctx,
            ignore_binary_tag_for_string: de.ignore_binary_tag_for_string,
            preserve_tags: de.preserve_tags,
        }
    }
}

impl<'de> SeqAccess<'de> for ValueSeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                let de = Deserializer {
                    value,
                    span_ctx: self.span_ctx,
                    ignore_binary_tag_for_string: self.ignore_binary_tag_for_string,
                    preserve_tags: self.preserve_tags,
                };
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }
}

pub(crate) struct ValueMapAccess<'de> {
    iter: indexmap::map::Iter<'de, String, Value>,
    value: Option<&'de Value>,
    span_ctx: Option<&'de span_context::SpanContext>,
    ignore_binary_tag_for_string: bool,
    /// Mirror of the parent [`Deserializer::preserve_tags`] so
    /// nested `Value::Tagged(...)` nodes inside a mapping survive
    /// the data-binding return path. See `de::Deserializer` docs.
    preserve_tags: bool,
}

impl<'de> ValueMapAccess<'de> {
    pub(crate) fn from_de(de: &Deserializer<'de>, map: &'de crate::value::Mapping) -> Self {
        ValueMapAccess {
            iter: map.iter(),
            value: None,
            span_ctx: de.span_ctx,
            ignore_binary_tag_for_string: de.ignore_binary_tag_for_string,
            preserve_tags: de.preserve_tags,
        }
    }

    /// Build the child [`Deserializer`] used to read each map
    /// value — propagates every per-call toggle including
    /// `preserve_tags`.
    fn child_de(&self, value: &'de Value) -> Deserializer<'de> {
        Deserializer {
            value,
            span_ctx: self.span_ctx,
            ignore_binary_tag_for_string: self.ignore_binary_tag_for_string,
            preserve_tags: self.preserve_tags,
        }
    }
}

impl<'de> MapAccess<'de> for ValueMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let de = self.child_de(value);
                let key_de: de::value::StrDeserializer<'de, Error> =
                    key.as_str().into_deserializer();
                de.wrap_err(seed.deserialize(key_de).map(Some))
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => {
                let de = self.child_de(value);
                let res = seed.deserialize(de);
                de.wrap_err(res)
            }
            None => Err(de::Error::custom("value is missing")),
        }
    }
}

struct EnumAccess<'de> {
    variant: &'de str,
    value: &'de Value,
    span_ctx: Option<&'de span_context::SpanContext>,
}

impl<'de> de::EnumAccess<'de> for EnumAccess<'de> {
    type Error = Error;
    type Variant = VariantAccess<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let de: de::value::StrDeserializer<'de, Error> = self.variant.into_deserializer();
        let variant = seed.deserialize(de)?;
        let visitor = VariantAccess {
            value: self.value,
            span_ctx: self.span_ctx,
        };
        Ok((variant, visitor))
    }
}

struct VariantAccess<'de> {
    value: &'de Value,
    span_ctx: Option<&'de span_context::SpanContext>,
}

impl<'de> de::VariantAccess<'de> for VariantAccess<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        Deserialize::deserialize(de)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        seed.deserialize(de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        de::Deserializer::deserialize_seq(de, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let de = if let Some(ctx) = self.span_ctx {
            Deserializer::with_span_context(self.value, ctx)
        } else {
            Deserializer::new(self.value)
        };
        de::Deserializer::deserialize_map(de, visitor)
    }
}

pub(crate) struct SpannedMapAccess<'de> {
    value: &'de Value,
    span_ctx: Option<&'de span_context::SpanContext>,
    fields: core::slice::Iter<'static, &'static str>,
}

impl<'de> SpannedMapAccess<'de> {
    pub(crate) fn new(value: &'de Value, span_ctx: Option<&'de span_context::SpanContext>) -> Self {
        SpannedMapAccess {
            value,
            span_ctx,
            fields: crate::spanned::SPANNED_FIELDS.iter(),
        }
    }
}

impl<'de> MapAccess<'de> for SpannedMapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.fields.next() {
            Some(field) => {
                use serde::de::value::BorrowedStrDeserializer;
                let de: BorrowedStrDeserializer<'_, Error> = BorrowedStrDeserializer::new(field);
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        use crate::spanned::*;
        let last_field = SPANNED_FIELDS[SPANNED_FIELDS.len() - 1 - (self.fields.len())];

        if last_field == SPANNED_FIELD_VALUE {
            let de = if let Some(ctx) = self.span_ctx {
                Deserializer::with_span_context(self.value, ctx)
            } else {
                Deserializer::new(self.value)
            };
            return de.wrap_err(seed.deserialize(de));
        }

        let ptr: *const Value = self.value;
        let addr = ptr as usize;
        let span = self.span_ctx.and_then(|ctx| ctx.spans.get(&addr));
        let loc = if let Some(s) = span {
            crate::error::Location::from_index(&self.span_ctx.unwrap().source, s.0)
        } else {
            crate::error::Location::default()
        };
        let end_loc = if let Some(s) = span {
            crate::error::Location::from_index(&self.span_ctx.unwrap().source, s.1)
        } else {
            crate::error::Location::default()
        };

        let val = match last_field {
            SPANNED_FIELD_START_LINE => loc.line(),
            SPANNED_FIELD_START_COLUMN => loc.column(),
            SPANNED_FIELD_START_INDEX => loc.index(),
            SPANNED_FIELD_END_LINE => end_loc.line(),
            SPANNED_FIELD_END_COLUMN => end_loc.column(),
            SPANNED_FIELD_END_INDEX => end_loc.index(),
            _ => crate::error::invariant_violated(
                "spanned-field index outside the SPANNED_FIELDS array",
            ),
        };

        seed.deserialize(val.into_deserializer())
    }
}

/// True if `tag` names the YAML 1.2 binary tag, in any of the forms
/// the scanner / loader may produce: shorthand `!!binary`, suffix
/// `binary` (post-handle-stripping), or the canonical full URI
/// `tag:yaml.org,2002:binary`. Stripping the leading `!` on the
/// shorthand keeps `Tag::new("!!binary") == Tag::new("binary")` —
/// which noyalib's `Tag` already considers equal — both matching.
pub(crate) fn is_binary_tag(tag: &str) -> bool {
    matches!(
        tag,
        "!!binary" | "binary" | "tag:yaml.org,2002:binary" | "!<tag:yaml.org,2002:binary>"
    )
}

fn type_name(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(_) => "bool".to_owned(),
        Value::Number(Number::Integer(_)) => "integer".to_owned(),
        Value::Number(Number::Float(_)) => "float".to_owned(),
        Value::String(_) => "string".to_owned(),
        Value::Sequence(_) => "sequence".to_owned(),
        Value::Mapping(_) => "mapping".to_owned(),
        Value::Tagged(tagged) => format!("tagged value (!{})", tagged.tag().as_str()),
    }
}
