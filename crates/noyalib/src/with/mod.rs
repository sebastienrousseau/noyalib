//! Helper modules for customizing serialization and deserialization.
//!
//! This module provides utilities for controlling how certain types are
//! serialized and deserialized, particularly enums.
//!
//! # Singleton Map
//!
//! The `singleton_map` family of modules provides helpers for serializing
//! enums as single-entry maps, which is a common YAML pattern.
//!
//! ## Example
//!
//! ```rust
//! use noyalib::with::singleton_map;
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! enum Action {
//!     Start { delay: u32 },
//!     Stop,
//! }
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
//! struct Task {
//!     name: String,
//!     #[serde(with = "singleton_map")]
//!     action: Action,
//! }
//!
//! let task = Task {
//!     name: "my-task".to_string(),
//!     action: Action::Start { delay: 5 },
//! };
//!
//! let yaml = noyalib::to_string(&task).unwrap();
//! // Output:
//! // name: my-task
//! // action:
//! //   Start:
//! //     delay: 5
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

pub mod singleton_map;
pub mod singleton_map_optional;
pub mod singleton_map_recursive;
pub mod singleton_map_with;

/// Alias for `singleton_map_recursive`.
///
/// This provides compatibility with code that uses the `nested_singleton_map`
/// name.
pub use singleton_map_recursive as nested_singleton_map;
