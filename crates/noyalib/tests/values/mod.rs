// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The Value tree, scalar resolution and the Serde bridge.
//!
//! 19 files, compiled into one test binary.

#![allow(missing_docs)]

mod binary_serde_bytes;
mod borrowed_anchors_and_format;
mod complex_serde_interop;
mod cov_number_ord;
mod flatten_value;
mod flattened_capture;
mod json_surrogate_escape;
mod mapping_any;
mod number_float_display;
mod plain_float_specials;
mod scalar_resolution_toggles;
mod serde;
mod serde_ecosystem;
mod spanned;
mod string_target_plain_scalar;
mod value;
mod value_coverage_extra;
mod value_final_push;
mod zero_copy_str_deser;
