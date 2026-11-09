// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Schema/CST composition also compiles when the core's std feature is off.
//!
//! The schema dependency requires a hosted target. These tests check the
//! core's alloc imports independently of that dependency's std requirement.

#![cfg(feature = "validate-schema")]
#![allow(missing_docs)]

#[path = "integrations/coerce_to_schema.rs"]
mod coerce_to_schema;
#[path = "integrations/coerce_to_schema_extra.rs"]
mod coerce_to_schema_extra;
