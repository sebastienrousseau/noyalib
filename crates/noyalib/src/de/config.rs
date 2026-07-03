//! Parser configuration types.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;

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
    /// When `true`, and the `lossless-u64` Cargo feature is enabled,
    /// YAML integer scalars in `(i64::MAX, u64::MAX]` resolve as
    /// unsigned integers instead of falling through to `f64`.
    ///
    /// Default `false` to preserve the historical public
    /// `Integer(i64)` / `Float(f64)` model and serde-yaml compatibility.
    #[cfg(feature = "lossless-u64")]
    #[cfg_attr(docsrs, doc(cfg(feature = "lossless-u64")))]
    pub lossless_u64_integers: bool,
    /// Indentation-validation mode. See [`RequireIndent`].
    /// Default: [`RequireIndent::Unchecked`] — accept any
    /// well-formed YAML indent.
    pub require_indent: RequireIndent,
    /// Pluggable "Safe YAML" policies, run during parsing.
    ///
    /// Each [`Policy`](crate::policy::Policy) inspects parser
    /// events and the post-parse [`Value`](crate::Value) tree; any policy
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
    /// Each scalar in the resulting [`Value`](crate::Value) tree is walked and
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
    /// the parse with [`Error::Custom`](crate::Error::Custom).
    /// When `false` (default), unknown placeholders are replaced
    /// with the empty string — the lossy semantics matching
    /// [`Value::interpolate_properties_lossy`](crate::Value::interpolate_properties_lossy).
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub strict_properties: bool,
    /// `!include` directive resolver. When set, the post-parse
    /// walk substitutes every `Value::Tagged(!include, spec)`
    /// node with the result of `resolver(IncludeRequest)`. See
    /// [`crate::include::IncludeResolver`] for the closure
    /// signature and [`crate::include::SafeFileResolver`] for
    /// the bundled filesystem implementation.
    ///
    /// `None` (default) disables include expansion; tagged
    /// `!include` nodes flow through unchanged.
    #[cfg(feature = "include")]
    #[cfg_attr(docsrs, doc(cfg(feature = "include")))]
    pub include_resolver: Option<crate::include::IncludeResolver>,
    /// Maximum `!include` recursion depth. Default 24. Each
    /// nested `!include` increments the depth counter; once the
    /// limit is reached, the parser aborts with
    /// `Error::RecursionLimitExceeded`. Pairs with a per-walk
    /// visited-set to catch cycles independent of depth.
    #[cfg(feature = "include")]
    #[cfg_attr(docsrs, doc(cfg(feature = "include")))]
    pub max_include_depth: usize,
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
            #[cfg(feature = "lossless-u64")]
            lossless_u64_integers: false,
            require_indent: RequireIndent::Unchecked,
            policies: Vec::new(),
            #[cfg(feature = "std")]
            properties: None,
            #[cfg(feature = "std")]
            strict_properties: false,
            #[cfg(feature = "include")]
            include_resolver: None,
            #[cfg(feature = "include")]
            max_include_depth: 24,
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
            #[cfg(feature = "lossless-u64")]
            lossless_u64_integers: false,
            require_indent: RequireIndent::Even,
            policies: Vec::new(),
            #[cfg(feature = "std")]
            properties: None,
            #[cfg(feature = "std")]
            strict_properties: true,
            #[cfg(feature = "include")]
            include_resolver: None,
            // Strict mode tightens the include recursion ceiling
            // proportionally to its other depth caps (max_depth
            // 128 → 64, max_alias_expansions 1024 → 100).
            #[cfg(feature = "include")]
            max_include_depth: 8,
        }
    }

    /// Install a `${KEY}` substitution table consulted after
    /// parsing.
    ///
    /// Each scalar in the resulting [`Value`](crate::Value) tree is walked and
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

    /// Install an `!include` directive resolver.
    ///
    /// Each `Value::Tagged(!include, scalar_spec)` node in the
    /// parsed tree is replaced with the resolver's output. The
    /// resolver is consulted with an `IncludeRequest` carrying
    /// the verbatim spec text, a stable source-id, and the
    /// current recursion depth.
    ///
    /// Pair with [`Self::max_include_depth`] to bound the
    /// recursion ceiling. Cycle detection (A includes B includes
    /// A) runs independently using a per-walk visited set.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::include::{IncludeRequest, IncludeResolver, InputSource};
    /// use noyalib::{ParserConfig, Result};
    ///
    /// let resolver = IncludeResolver::new(|req: IncludeRequest<'_>| -> Result<InputSource> {
    ///     // For an in-memory test, fabricate a YAML payload
    ///     // keyed on the spec.
    ///     Ok(InputSource::new(req.spec, format!("name: {}\n", req.spec)))
    /// });
    /// let cfg = ParserConfig::new().include_resolver(resolver);
    /// # let _ = cfg;
    /// ```
    #[cfg(feature = "include")]
    #[cfg_attr(docsrs, doc(cfg(feature = "include")))]
    #[must_use]
    pub fn include_resolver(mut self, resolver: crate::include::IncludeResolver) -> Self {
        self.include_resolver = Some(resolver);
        self
    }

    /// Maximum `!include` recursion depth.
    ///
    /// Default 24 (8 in [`Self::strict()`]). Each nested
    /// `!include` increments the depth; once the limit is
    /// reached, the parser aborts with
    /// `Error::RecursionLimitExceeded`. The cap is independent
    /// of [`Self::max_depth`] (which bounds *YAML structural*
    /// nesting) and of the per-walk cycle-detection set (which
    /// catches A→B→A regardless of depth).
    #[cfg(feature = "include")]
    #[cfg_attr(docsrs, doc(cfg(feature = "include")))]
    #[must_use]
    pub fn max_include_depth(mut self, depth: usize) -> Self {
        self.max_include_depth = depth;
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

    /// Enable or disable lossless unsigned integer resolution.
    ///
    /// With the `lossless-u64` feature enabled, setting this to
    /// `true` lets YAML integer scalars in `(i64::MAX, u64::MAX]`
    /// resolve as `Number::Unsigned` instead of falling through to
    /// `Number::Float`.
    #[cfg(feature = "lossless-u64")]
    #[cfg_attr(docsrs, doc(cfg(feature = "lossless-u64")))]
    #[must_use]
    pub fn lossless_u64_integers(mut self, on: bool) -> Self {
        self.lossless_u64_integers = on;
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
