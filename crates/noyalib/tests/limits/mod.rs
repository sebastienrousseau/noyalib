// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Parser budgets, policies and hostile input.
//!
//! 9 files, compiled into one test binary.

#![allow(missing_docs)]

mod alias_expansion;
mod budget_breach;
mod dos_hardening;
mod max_nodes_budget;
mod multi_document_rejection;
mod policy;
mod require_indent;
mod streaming_budget_parity;
mod stress_load;
