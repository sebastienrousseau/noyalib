// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Exercise the portable public API with the library's std feature disabled.
//!
//! The test harness uses std; bare-metal builds separately check that library
//! dependencies stay alloc-only. Reuse the hosted tests to keep expectations
//! identical across the two library configurations.

#![allow(missing_docs)]
#![cfg(not(feature = "std"))]

#[path = "limits/alias_depth.rs"]
mod alias_depth;
#[path = "limits/alias_expansion.rs"]
mod alias_expansion;
#[path = "anchors/redefinition.rs"]
mod anchor_redefinition;
#[path = "values/binary_serde_bytes.rs"]
mod binary_serde_bytes;
#[path = "regressions/competitive_features.rs"]
mod competitive_features;
#[path = "regressions/competitive_features_full.rs"]
mod competitive_features_full;
#[path = "values/complex_serde_interop.rs"]
mod complex_serde_interop;
#[path = "reading/de_error_field_location.rs"]
mod de_error_field_location;
#[path = "reading/de_error_field_path.rs"]
mod de_error_field_path;
#[path = "limits/dos_hardening.rs"]
mod dos_hardening;
#[path = "values/flatten_value.rs"]
mod flatten_value;
#[path = "limits/flow_key_budget.rs"]
mod flow_key_budget;
#[path = "integrations/include_directive.rs"]
mod include_directive;
#[path = "conformance/legacy_sexagesimal.rs"]
mod legacy_sexagesimal;
#[path = "values/mapping_any.rs"]
mod mapping_any;
#[path = "limits/max_nodes_budget.rs"]
mod max_nodes_budget;
#[path = "values/number_float_display.rs"]
mod number_float_display;
#[path = "conformance/official_suite.rs"]
mod official_suite;
#[path = "values/plain_float_specials.rs"]
mod plain_float_specials;
#[path = "values/query_recursive.rs"]
mod query_recursive;
#[path = "limits/require_indent.rs"]
mod require_indent;
#[path = "values/scalar_resolution_toggles.rs"]
mod scalar_resolution_toggles;
#[path = "reading/scanner_panic_regressions.rs"]
mod scanner_panic_regressions;
#[path = "reading/scanner_refusals.rs"]
mod scanner_refusals;
#[path = "ser/ser_block_scalar_chomping.rs"]
mod ser_block_scalar_chomping;
#[path = "ser/ser_block_scalar_indicator.rs"]
mod ser_block_scalar_indicator;
#[path = "ser/ser_block_scalar_round_trips.rs"]
mod ser_block_scalar_round_trips;
#[path = "ser/ser_colon_hash_plain.rs"]
mod ser_colon_hash_plain;
#[path = "ser/ser_compact_list_depth.rs"]
mod ser_compact_list_depth;
#[path = "ser/ser_directive_tags.rs"]
mod ser_directive_tags;
#[path = "ser/ser_emitter_edge_cases.rs"]
mod ser_emitter_edge_cases;
#[path = "ser/ser_flow_quoting.rs"]
mod ser_flow_quoting;
#[path = "ser/ser_line_break_characters.rs"]
mod ser_line_break_characters;
#[path = "ser/ser_no_trailing_whitespace.rs"]
mod ser_no_trailing_whitespace;
#[path = "ser/ser_non_printable_characters.rs"]
mod ser_non_printable_characters;
#[path = "ser/ser_plain_digit_strings.rs"]
mod ser_plain_digit_strings;
#[path = "ser/ser_prefer_single_quotes.rs"]
mod ser_prefer_single_quotes;
#[path = "ser/ser_quoting_edges.rs"]
mod ser_quoting_edges;
#[path = "ser/ser_tag_indicator_keys.rs"]
mod ser_tag_indicator_keys;
#[path = "ser/ser_tagged_value_round_trip.rs"]
mod ser_tagged_value_round_trip;
#[path = "values/serde_ecosystem.rs"]
mod serde_ecosystem;
#[path = "integrations/serde_yaml_contract.rs"]
mod serde_yaml_contract;
#[path = "properties/simd_equivalence.rs"]
mod simd_equivalence;
#[path = "limits/skipped_values.rs"]
mod skipped_values;
#[path = "values/spanned.rs"]
mod spanned;
#[path = "spec/mod.rs"]
mod spec;
#[path = "streaming/streaming_alias_lookahead.rs"]
mod streaming_alias_lookahead;
#[path = "streaming/streaming_binary.rs"]
mod streaming_binary;
#[path = "streaming/streaming_public_api.rs"]
mod streaming_public_api;
#[path = "streaming/streaming_type_mismatches.rs"]
mod streaming_type_mismatches;
#[path = "values/string_target_plain_scalar.rs"]
mod string_target_plain_scalar;
#[path = "conformance/verbatim_tag_unclosed.rs"]
mod verbatim_tag_unclosed;
#[path = "conformance/yaml_version.rs"]
mod yaml_version;
#[path = "values/zero_copy_str_deser.rs"]
mod zero_copy_str_deser;

#[path = "coverage/coverage_recovery.rs"]
mod coverage_recovery;
#[path = "regressions/recovery.rs"]
mod recovery;
