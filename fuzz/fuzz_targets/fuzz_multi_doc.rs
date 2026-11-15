//! Fuzz target: multi-document parsing.
//!
//! Exercises document boundary detection, `---` / `...` markers,
//! and per-document anchor scope isolation.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // Multi-document load — should never panic.
    if let Ok(iter) = noyalib::load_all(s) {
        for doc in iter {
            let _ = doc;
        }
    }

    // Multi-document serialize roundtrip.
    if let Ok(iter) = noyalib::load_all(s) {
        let docs: Vec<_> = iter.filter_map(|d| d.ok()).collect();
        let _ = noyalib::to_string_multi(&docs);
    }
});
