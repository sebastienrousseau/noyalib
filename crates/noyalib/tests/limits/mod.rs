// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Parser budgets, policies and hostile input.
//!
//! 11 files, compiled into one test binary.

#![allow(missing_docs)]

mod alias_depth;
mod alias_expansion;
mod budget_breach;
mod dos_hardening;
mod max_nodes_budget;
mod multi_document_rejection;
mod policy;
mod require_indent;
mod skipped_values;
mod streaming_budget_parity;
mod stress_load;
