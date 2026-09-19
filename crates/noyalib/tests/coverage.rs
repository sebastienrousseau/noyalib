// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Binary root for the coverage suites; see `coverage/mod.rs`.
//!
//! Mirrors the `tests/spec.rs` + `tests/spec/mod.rs` pattern already
//! used in this directory.

#![allow(missing_docs)]

#[path = "coverage/mod.rs"]
mod cases;
