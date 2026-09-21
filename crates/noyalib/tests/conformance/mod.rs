// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! YAML spec conformance, tags, comments and document shape.
//!
//! 22 files, compiled into one test binary.

#![allow(missing_docs)]

mod binary_tag_for_string;
mod block_scalar_explicit_indent_leading_spaces;
mod bom_in_stream;
mod comment_mutation;
mod comments;
mod edge_audit;
mod edge_cases;
mod empty_document_targets;
mod flow_block_scalar_indicator;
mod implicit_null_at_eof;
mod legacy_sexagesimal;
mod multi_doc;
mod nested_value_tag_preservation;
mod official_suite;
mod roundtrip_edge_cases;
mod spec_torture;
mod tag_control_characters;
mod tag_registry;
mod ultra_complex;
mod verbatim_tag_unclosed;
mod yaml_compliance_report;
mod yaml_version;
