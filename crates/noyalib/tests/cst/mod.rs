// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The concrete-syntax-tree suites: parsing, editing, styling and round-tripping.
//!
//! 55 files, compiled into one test binary instead of
//! 55 executables. The files themselves are unchanged.

#![allow(missing_docs)]

mod cst_anchors;
mod cst_block_growth;
mod cst_block_literal_trailing_comment;
mod cst_comment_editing_paths;
mod cst_comments_on_a_block_valued_key;
mod cst_comments_on_an_entry_with_no_value;
mod cst_crlf_splices;
mod cst_decorated_and_crlf_edits;
mod cst_document_coverage;
mod cst_document_final_push;
mod cst_edit_errors;
mod cst_emit;
mod cst_entry_or_insert;
mod cst_error_paths;
mod cst_flow_inserts;
mod cst_format;
mod cst_fragment_rollback;
mod cst_incremental;
mod cst_indent_detection;
mod cst_inline_comment;
mod cst_insert_anchor_trailing_comment;
mod cst_insert_containment;
mod cst_insert_keep_chomped_scalar;
mod cst_key_span;
mod cst_leading_comment;
mod cst_line_break_characters;
mod cst_move_item;
mod cst_mutation;
mod cst_mutation_coverage;
mod cst_mutator_error_paths;
mod cst_non_printable_characters;
mod cst_parse_config;
mod cst_quoted_path_segments;
mod cst_remove_flow_and_sole;
mod cst_remove_fuzz_regressions;
mod cst_remove_merge_key;
mod cst_remove_multiline;
mod cst_remove_seq_item_first_key;
mod cst_remove_wrapped_flow;
mod cst_rename_key;
mod cst_round_trip;
mod cst_schema_tag_audit;
mod cst_seq_edit;
mod cst_set_path;
mod cst_set_path_and_styling;
mod cst_set_value_collection;
mod cst_set_value_flow_context;
mod cst_set_value_noop;
mod cst_smart_styling;
mod cst_stream;
mod cst_structure;
mod cst_style_heuristics;
mod cst_swap_items;
mod cst_tab_before_comment;
mod cst_tag_span;
