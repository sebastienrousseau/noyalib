// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Issue repros, release-phase sweeps and competitive checks.
//!
//! 22 files, compiled into one test binary.

#![allow(missing_docs)]

mod competitive_features;
mod competitive_features_full;
mod competitor_bugs;
mod feature_matrix;
mod fmt;
mod issue_239;
mod issue_46;
mod key_collision_streaming;
mod leading_comment_repro;
mod phase1_features;
mod phase2;
mod phase3;
mod phase4;
mod phase5;
mod read_iterator;
mod recovery;
mod reference_docs_are_complete;
mod remove_flow_data_loss;
mod review_fixes;
mod set_fragment_containment;
mod shaped_document_paths;
mod type_mismatch_and_config_surface;
