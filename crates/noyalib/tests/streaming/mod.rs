// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The streaming reader suites: events, typed reads and refusals.
//!
//! 10 files, compiled into one test binary instead of
//! 10 executables. The files themselves are unchanged.

#![allow(missing_docs)]

mod streaming_alias_lookahead;
mod streaming_binary;
mod streaming_coverage_extra;
mod streaming_de_final_push;
mod streaming_failsafe_tags;
mod streaming_merge_and_enum_paths;
mod streaming_public_api;
mod streaming_round3_push;
mod streaming_tagged_and_refusals;
mod streaming_type_mismatches;
