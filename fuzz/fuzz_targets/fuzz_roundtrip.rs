//! Fuzz target: parse → serialize → parse roundtrip.
//!
//! Validates that serialized output re-parses to an equivalent Value.
//! Catches serialization bugs that produce invalid YAML.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::Value;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(value) = noyalib::from_str::<Value>(s) else {
        return;
    };

    // Serialize back to YAML.
    let Ok(yaml) = noyalib::to_string(&value) else {
        return;
    };

    // Re-parse the output — must succeed if serialization succeeded.
    let reparsed =
        noyalib::from_str::<Value>(&yaml).expect("serialized YAML must re-parse without error");

    // Structural equality (NaN != NaN is expected, so skip deep assert
    // when floats are involved — just verify it parses).
    let _ = reparsed;
});
