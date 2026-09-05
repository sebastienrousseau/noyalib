// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The parser on the `core` + `alloc` build must never panic on any
//! input; every failure is an `Err` whose Display renders.
#![no_main]
#![deny(unused_imports)]
#[cfg(feature = "std")]
compile_error!("fuzz-nostd must not enable noyalib/std; check feature unification");

extern crate alloc;
use alloc::string::ToString;
use core::str;
use libfuzzer_sys::fuzz_target;
use noyalib::Value;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = str::from_utf8(data) else { return };
    if let Err(e) = noyalib::from_str::<Value>(s) {
        let _ = e.to_string();
    }
});
