//! Fuzz target: the `from_reader*` and `from_slice*` families.
//!
//! These take the same untrusted bytes as [`noyalib::from_str`] but by a
//! different route — `from_slice` skips the UTF-8 precondition that
//! `from_str` gets for free, and `from_reader` adds an IO layer that can
//! return short reads and interior errors. Neither family was reached by
//! any fuzz target or by the `panic_free` proptests, so the paths that
//! handle a malformed byte sequence *before* it becomes a `&str` had no
//! adversarial coverage at all.
//!
//! Panics are bugs; errors are expected.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::Value;

fuzz_target!(|data: &[u8]| {
    // Raw bytes, no UTF-8 guarantee: this is the half `from_str` never
    // sees.
    let _ = noyalib::from_slice::<Value>(data);
    let _ = noyalib::from_slice_strict::<Value>(data);

    // The reader family over the same bytes. `&[u8]` is a `Read`, so a
    // truncated or invalid sequence exercises the incremental path.
    let _ = noyalib::from_reader::<_, Value>(data);
    let _ = noyalib::from_reader_strict::<_, Value>(data);

    // A deserialise target that is not `Value` takes a different route
    // through the deserializer.
    let _ = noyalib::from_slice::<String>(data);
    let _ = noyalib::from_reader::<_, std::collections::BTreeMap<String, String>>(data);
});
