// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The coverage suites, compiled into one test binary.
//!
//! These 37 files were written one per gap-closing round. As separate
//! `tests/*.rs` files, Cargo built and linked 37 executables for them;
//! as modules of one binary it links once. The files themselves are
//! unchanged — including the four that carry a crate-level
//! `#![cfg(feature = "...")]`, which gates the module just as it gated
//! the binary.

#![allow(missing_docs)]

mod coverage_100;
mod coverage_b1_de;
mod coverage_b1_format_ser;
mod coverage_b1_parser;
mod coverage_b1_streaming;
mod coverage_b1_value_with;
mod coverage_boost;
mod coverage_borrowed;
mod coverage_borrowed_full;
mod coverage_cst_green;
mod coverage_de;
mod coverage_de_include;
mod coverage_error;
mod coverage_final;
mod coverage_final_sweep;
mod coverage_final_sweep2;
mod coverage_final_sweep3;
mod coverage_final_sweep4;
mod coverage_fmt;
mod coverage_full;
mod coverage_gaps;
mod coverage_loader;
mod coverage_loader_full;
mod coverage_misc;
mod coverage_number_unsigned;
mod coverage_recovery;
mod coverage_regression;
mod coverage_remaining;
mod coverage_scanner;
mod coverage_schema_validate;
mod coverage_ser;
mod coverage_singleton_map;
mod coverage_spanned_anchors;
mod coverage_unsigned_and_format;
mod coverage_untested_public_api;
mod coverage_value;
mod coverage_value_serde;
