// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The deserialiser, parser, loader and scanner suites.
//!
//! 11 files, compiled into one test binary instead of
//! 11 executables. The files themselves are unchanged.

#![allow(missing_docs)]

mod de;
mod de_branch_coverage;
mod de_coverage_extra;
mod de_error_field_path;
mod de_streaming_branch_extra;
mod loader_paths;
mod loader_scanner_shapes;
mod parser_coverage_extra;
mod parser_limits_and_error_surface;
mod scanner_panic_regressions;
mod scanner_refusals;
