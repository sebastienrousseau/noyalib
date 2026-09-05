// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! What the alloc-only serialiser writes, the alloc-only parser reads
//! back: emitted text re-parses, and to the same value when no NaN is
//! involved.
#![no_main]
#![deny(unused_imports)]
#[cfg(feature = "std")]
compile_error!("fuzz-nostd must not enable noyalib/std; check feature unification");

extern crate alloc;
use alloc::string::String;
use core::str;
use libfuzzer_sys::fuzz_target;
use noyalib::Value;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = str::from_utf8(data) else { return };
    let Ok(v) = noyalib::from_str::<Value>(s) else { return };
    let out: String = noyalib::to_string(&v).expect("alloc-only serialiser");
    let back: Value = noyalib::from_str(&out).expect("alloc-only re-parse");
    if !out.to_ascii_lowercase().contains("nan") {
        assert_eq!(back, v, "alloc-only round-trip drift:\n{out}");
    }
});
