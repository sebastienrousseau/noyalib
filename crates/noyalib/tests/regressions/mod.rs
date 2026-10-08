// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Issue repros, release-phase sweeps and competitive checks.
//!
//! 23 files, compiled into one test binary.

#![allow(missing_docs)]

mod comment_printable;
mod competitive_features;
mod competitive_features_full;
mod competitor_bugs;
mod cst_unicode_space;
mod duplicate_key_identity;
mod escaped_break_empty_lines;
mod feature_matrix;
mod fmt;
mod folded_spaced_lines;
#[cfg(feature = "strict-deserialise")]
mod issue_239;
mod issue_46;
mod key_collision_streaming;
mod leading_comment_repro;

mod phase1_features;
mod phase2;
mod phase3;
mod phase4;
mod phase5;
mod radix_int_spelling;
mod read_iterator;
mod recovery;
mod reference_docs_are_complete;
mod remove_flow_data_loss;
mod review_fixes;
mod set_fragment_containment;
mod shaped_document_paths;
mod type_mismatch_and_config_surface;
mod typed_key_text;
mod verbatim_tags;
