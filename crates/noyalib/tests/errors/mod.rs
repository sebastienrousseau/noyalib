// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Error construction, rendering and diagnostic adapters.
//!
//! 11 files, compiled into one test binary.

#![allow(missing_docs)]

mod ariadne_adapter;
mod dup_key_spans;
mod duplicate_key_location;
mod error_coverage_extra;
mod error_kind;
mod error_render;
mod error_snippet_radius;
mod panic_free;
mod typed_error_location;
mod ux_diagnostics;
mod validated_miette_bridge;
