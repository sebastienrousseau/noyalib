//! Fuzz target: cross-check `from_str::<Value>` against
//! `cst::parse_document` for divergence.
//!
//! The `Value` target on `from_str` now runs the span-free
//! `NoSpanLoader` fast path (the streaming path was silently
//! collapsing distinct-typed key collisions and skipping three
//! DoS budgets — see the v0.0.14 loader-parity fix). This fuzz
//! target's whole job is to make sure that fast path stays in
//! lock-step with the span-full CST loader on every input the
//! fuzzer can reach: if one accepts and the other rejects, the
//! divergence is a bug in the parity.
//!
//! Divergence rules:
//!
//! * both accept → OK.
//! * both reject → OK.
//! * one accepts and the other rejects → **PANIC** (regression).
//!
//! Panics on non-divergent runs are always bugs (both paths are
//! supposed to be panic-free).

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::Value;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    let value_res = noyalib::from_str::<Value>(s);
    let cst_res = noyalib::cst::parse_document(s);

    match (value_res.is_ok(), cst_res.is_ok()) {
        (true, true) | (false, false) => {}
        (true, false) => {
            // The CST loader is strict about the parity guards
            // (KeyCollision, DoS budgets) — if it errors, the
            // fast Value path must too.
            panic!(
                "divergence: from_str::<Value> succeeded but cst::parse_document rejected input"
            );
        }
        (false, true) => {
            // Reverse divergence: fast path errored where cst
            // accepted. Also a bug — parity was two-way.
            panic!(
                "divergence: cst::parse_document succeeded but from_str::<Value> rejected input"
            );
        }
    }
});
