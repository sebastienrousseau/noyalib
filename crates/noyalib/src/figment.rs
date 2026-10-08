// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! [`figment`] provider for noyalib YAML.
//!
//! [`figment`] is the popular layered-configuration crate: it
//! merges multiple config sources (env vars, TOML / JSON / YAML
//! files, CLI flags, in-memory overrides) into a single typed
//! struct via `Figment::new().merge(...).join(...).extract()`. The
//! [`Yaml`](crate::figment::Yaml) provider in this module plugs noyalib into that
//! chain the same way `figment::providers::Toml` /
//! `figment::providers::Json` do — without depending on the
//! unmaintained `serde_yaml` 0.9 crate.
//!
//! Gated behind the `figment` Cargo feature.
//!
//! # Examples
//!
//! ```rust
//! use figment::providers::Format;
//! use figment::Figment;
//! use noyalib::figment::Yaml;
//!
//! #[derive(serde::Deserialize)]
//! struct Config {
//!     name: String,
//!     port: u16,
//! }
//!
//! let yaml = "name: noyalib\nport: 8080\n";
//! let cfg: Config = Figment::new().merge(Yaml::string(yaml)).extract().unwrap();
//! assert_eq!(cfg.name, "noyalib");
//! assert_eq!(cfg.port, 8080);
//! ```
//!
//! Layered example — start with defaults, override with a YAML file,
//! finalise with environment variables:
//!
//! ```rust,ignore
//! use figment::Figment;
//! use figment::providers::{Env, Serialized};
//! use noyalib::figment::Yaml;
//!
//! let cfg = Figment::new()
//!     .merge(Serialized::defaults(MyDefaults::default()))
//!     .merge(Yaml::file("config.yaml"))
//!     .merge(Env::prefixed("MYAPP_"))
//!     .extract::<MyConfig>()
//!     .unwrap();
//! ```

use std::path::{Path, PathBuf};

use figment::providers::Format;
use figment::value::{Dict, Map};
use figment::{Error as FigmentError, Metadata, Profile, Provider};

use crate::ParserConfig;

/// Figment [`Format`] for noyalib YAML.
///
/// Build a provider via `Yaml::string(yaml_text)`, `Yaml::file(path)`,
/// or any of the other [`Format`] constructors inherited from the
/// trait. The result implements [`figment::Provider`] and slots into
/// `Figment::merge` / `Figment::join` chains.
///
/// These providers parse under the default [`ParserConfig`]: a source
/// over `max_document_length` is refused, and a file is read no
/// further than one byte past that limit. Use
/// [`Yaml::string_with_config`] or [`Yaml::file_with_config`] to parse
/// under other limits or policies.
#[derive(Debug, Clone, Copy)]
pub struct Yaml;

impl Format for Yaml {
    type Error = FigmentError;

    const NAME: &'static str = "YAML";

    fn from_str<T: serde_core::de::DeserializeOwned>(s: &str) -> Result<T, Self::Error> {
        // figment's `Format::from_str` constrains `T:
        // DeserializeOwned` only — no `'static`. Bypass noyalib's
        // public `from_str` (which adds `'static` to enable the
        // TypeId-driven tag-preserving path for `Value`) and call
        // the internal non-tag-preserving entry directly.
        // figment's typed targets are profile shapes, not `Value`,
        // so the tag-preserving path would never have applied
        // anyway.
        parse(s, &ParserConfig::default()).map_err(to_figment)
    }

    fn from_path<T: serde_core::de::DeserializeOwned>(path: &Path) -> Result<T, Self::Error> {
        // figment's default reads the whole file first; stop one byte
        // past the length limit instead.
        let config = ParserConfig::default();
        read_bounded(path, &config)
            .and_then(|text| parse(&text, &config))
            .map_err(to_figment)
    }
}

impl Yaml {
    /// A provider that parses `yaml` under `config`, for limits or
    /// policies other than the defaults. Emits to the default profile
    /// unless [`YamlWithConfig::nested`] or [`YamlWithConfig::profile`]
    /// says otherwise, like [`Format::string`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use figment::Figment;
    /// use noyalib::ParserConfig;
    /// use noyalib::figment::Yaml;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct Config {
    ///     port: u16,
    /// }
    ///
    /// let provider = Yaml::string_with_config("port: 8080\n", ParserConfig::strict());
    /// let cfg: Config = Figment::new().merge(provider).extract().unwrap();
    /// assert_eq!(cfg.port, 8080);
    /// ```
    #[must_use]
    pub fn string_with_config(yaml: &str, config: ParserConfig) -> YamlWithConfig {
        YamlWithConfig::new(Source::String(yaml.to_owned()), config)
    }

    /// A provider that reads and parses the file at `path` under
    /// `config`. The path is used as given (no search of parent
    /// directories, unlike [`Format::file`]); a missing file is an
    /// error. Reading stops one byte past `max_document_length`.
    #[must_use]
    pub fn file_with_config<P: AsRef<Path>>(path: P, config: ParserConfig) -> YamlWithConfig {
        YamlWithConfig::new(Source::File(path.as_ref().to_path_buf()), config)
    }
}

/// A figment [`Provider`] for YAML parsed under a caller-supplied
/// [`ParserConfig`]. Built by [`Yaml::string_with_config`] or
/// [`Yaml::file_with_config`].
#[derive(Debug, Clone)]
pub struct YamlWithConfig {
    source: Source,
    config: ParserConfig,
    profile: Option<Profile>,
}

#[derive(Debug, Clone)]
enum Source {
    String(String),
    File(PathBuf),
}

impl YamlWithConfig {
    fn new(source: Source, config: ParserConfig) -> Self {
        Self {
            source,
            config,
            profile: Some(Profile::Default),
        }
    }

    /// Treat the document's top-level keys as profile names, like
    /// figment's `Data::nested`.
    #[must_use]
    pub fn nested(mut self) -> Self {
        self.profile = None;
        self
    }

    /// Emit the document's values to `profile` (when not nested).
    #[must_use]
    pub fn profile<P: Into<Profile>>(mut self, profile: P) -> Self {
        self.profile = Some(profile.into());
        self
    }
}

impl Provider for YamlWithConfig {
    fn metadata(&self) -> Metadata {
        match &self.source {
            Source::String(_) => Metadata::named("YAML source string"),
            Source::File(path) => Metadata::from("YAML file", path.as_path()),
        }
    }

    fn data(&self) -> Result<Map<Profile, Dict>, FigmentError> {
        let file_text;
        let text = match &self.source {
            Source::String(s) => s.as_str(),
            Source::File(path) => {
                file_text = read_bounded(path, &self.config).map_err(to_figment)?;
                file_text.as_str()
            }
        };
        match &self.profile {
            Some(profile) => parse::<Dict>(text, &self.config).map(|dict| profile.collect(dict)),
            None => parse(text, &self.config),
        }
        .map_err(to_figment)
    }
}

/// Parse `s` under `config`.
fn parse<T: serde_core::de::DeserializeOwned>(s: &str, config: &ParserConfig) -> crate::Result<T> {
    crate::de::from_str_typed_no_tag_preserve::<T>(s, config)
}

/// Read `path`, stopping one byte past `config.max_document_length`.
fn read_bounded(path: &Path, config: &ParserConfig) -> crate::Result<String> {
    let file = std::fs::File::open(path).map_err(crate::Error::Io)?;
    crate::de::read_to_string_bounded(file, config)
}

/// figment's error type carries the message as text.
fn to_figment(error: crate::Error) -> FigmentError {
    FigmentError::from(error.to_string())
}
