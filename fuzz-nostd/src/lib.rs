// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Development-only oracle classifier tests.

#![forbid(unsafe_code)]

#[cfg(feature = "std")]
compile_error!("the alloc-only fuzz harness must not enable core std");

#[cfg(test)]
#[path = "../../fuzz/bom_classification.rs"]
mod bom_classification;
