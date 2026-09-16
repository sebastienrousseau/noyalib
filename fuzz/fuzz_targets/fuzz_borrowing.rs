//! Fuzz target: the borrowing and multi-document entry points.
//!
//! `from_str_borrowing` and `from_str_borrowed` hand back values that
//! point into the caller's buffer rather than owning their bytes, so a
//! malformed input has to be rejected without leaving a borrow pointing
//! at something that was never valid. `from_str_multi` walks a document
//! stream, where the failure modes are per-document rather than
//! per-input.
//!
//! None of these were reached by an existing fuzz target or by the
//! `panic_free` proptests.
//!
//! Panics are bugs; errors are expected.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::Value;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // Borrowing entries: the returned value points into `s`.
    let _ = noyalib::borrowed::from_str_borrowed(s);
    let _ = noyalib::from_str_borrowing::<Value>(s);

    // Strict mode rejects more, and rejects it earlier.
    let _ = noyalib::from_str_strict::<Value>(s);

    // A document stream: failure is per-document, and a later document
    // being malformed must not corrupt an earlier one. `from_str_multi`
    // lives on the serde_yaml compatibility surface; `load_all_as` is
    // the native equivalent.
    if let Ok(docs) = noyalib::compat::serde_yaml::from_str_multi::<Value>(s) {
        for d in &docs {
            let _ = d.is_null();
        }
    }
    let _ = noyalib::load_all_as::<Value>(s);
});
