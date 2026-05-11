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
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
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

use figment::Error as FigmentError;
use figment::providers::Format;

/// Figment [`Format`] for noyalib YAML.
///
/// Build a provider via `Yaml::string(yaml_text)`, `Yaml::file(path)`,
/// or any of the other [`Format`] constructors inherited from the
/// trait. The result implements [`figment::Provider`] and slots into
/// `Figment::merge` / `Figment::join` chains.
#[derive(Debug, Clone, Copy)]
pub struct Yaml;

impl Format for Yaml {
    type Error = FigmentError;

    const NAME: &'static str = "YAML";

    fn from_str<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, Self::Error> {
        // figment's `Format::from_str` constrains `T:
        // DeserializeOwned` only — no `'static`. Bypass noyalib's
        // public `from_str` (which adds `'static` to enable the
        // TypeId-driven tag-preserving path for `Value`) and call
        // the internal non-tag-preserving entry directly.
        // figment's typed targets are profile shapes, not `Value`,
        // so the tag-preserving path would never have applied
        // anyway.
        crate::de::from_str_typed_no_tag_preserve::<T>(s, &crate::ParserConfig::default())
            .map_err(|e| FigmentError::from(e.to_string()))
    }
}
