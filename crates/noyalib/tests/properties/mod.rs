// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Property, differential and equivalence testing.
//!
//! 12 files, compiled into one test binary.

#![allow(missing_docs)]

mod differential_entry_points;
mod differential_readers;
mod no_span_loader_coverage;
mod no_span_loader_parity;
mod parallel_agreement;
mod properties_interpolation;
mod property_interpolation;
mod proptest;
mod proptest_emitter_paths;
mod set_value_roundtrip_prop;
mod simd_equivalence;
mod span_tree_invariants;
