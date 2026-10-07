//! Fuzz target: `cst::format` keeps the meaning of what it formats.
//!
//! # Invariants
//!
//! 1. No panic, whatever the input.
//! 2. When the input loads, a formatted output loads to the same
//!    values: `format` either preserves the meaning or refuses.
//! 3. Formatting an output again keeps the meaning.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::Value;

fn load(s: &str) -> Option<Vec<Value>> {
    noyalib::load_all_as::<Value>(s).ok()
}

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(formatted) = noyalib::cst::format(input) else {
        return;
    };
    let Some(before) = load(input) else {
        return;
    };
    let after = load(&formatted)
        .unwrap_or_else(|| panic!("format made a valid document invalid: {formatted:?}"));
    assert_eq!(after, before, "format changed the meaning: {formatted:?}");
    if let Ok(again) = noyalib::cst::format(&formatted) {
        assert_eq!(load(&again), Some(before), "a second format changed the meaning");
    }
});
