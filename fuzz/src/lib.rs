// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Development-only oracle classifier tests.

#![forbid(unsafe_code)]

#[cfg(test)]
#[path = "../bom_classification.rs"]
mod bom_classification;

#[cfg(test)]
#[path = "../oracle_tab_tests.rs"]
mod oracle_tab_tests;
