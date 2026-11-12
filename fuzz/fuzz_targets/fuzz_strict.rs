//! Fuzz target: strict parsing with tight security limits.
//!
//! Validates that the ParserConfig safety nets (depth, alias count,
//! document length, mapping/sequence bounds) hold under adversarial input.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::{ParserConfig, Value};

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    let config = ParserConfig::new()
        .max_depth(16)
        .max_document_length(4096)
        .max_alias_expansions(8)
        .max_mapping_keys(64)
        .max_sequence_length(64);

    // Must never panic, even with adversarial input and tight limits.
    let _ = noyalib::from_str_with_config::<Value>(s, &config);
});
