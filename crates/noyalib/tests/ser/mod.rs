// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The serializer suites: emission, quoting, block scalars and formatting.
//!
//! 16 files, compiled into one test binary instead of
//! 16 executables. The files themselves are unchanged.

#![allow(missing_docs)]

mod ser;
mod ser_block_scalar_chomping;
mod ser_block_scalar_indicator;
mod ser_block_scalar_round_trips;
mod ser_colon_hash_plain;
mod ser_compact_list_depth;
mod ser_cst_coverage_extra;
mod ser_entry_points_and_fmt_tags;
mod ser_line_break_characters;
mod ser_no_trailing_whitespace;
mod ser_non_printable_characters;
mod ser_plain_digit_strings;
mod ser_prefer_single_quotes;
mod ser_quoting_edges;
mod ser_tag_indicator_keys;
mod ser_tagged_value_round_trip;
