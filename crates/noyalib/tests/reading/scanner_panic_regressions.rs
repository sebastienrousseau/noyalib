// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Scanner panic regressions discovered by libfuzzer.
//!
//! Every input here is a literal corpus seed that previously caused
//! a panic in the scanner. The contract is unconditional: `from_str`
//! may return any `Result`, but **must not panic** on any byte
//! sequence — adversarial or otherwise. A panic in the parser is a
//! denial-of-service vector for any service that ingests untrusted
//! YAML.

#![allow(missing_docs)]

use noyalib::Value;

/// `:\n*\n:\n*\n Q Q` — fuzz_double_quoted artefact
/// `crash-dc2280b1d04...`. The simple-key tracker pointed past the
/// last emitted token's span end, so
/// `tokens.last().span.end < sk.index` and `&input[sk.index..end]`
/// panicked with "slice index starts at 2 but ends at 0". Fixed by
/// clamping `key_end` to `max(sk.index)` before computing the
/// trimmed-tail slice in `fetch_value`.
#[test]
fn fetch_value_with_alias_after_empty_implicit_key_does_not_panic() {
    let input: &[u8] = b":\n*\n:\n*\n Q Q";
    let s = std::str::from_utf8(input).unwrap();
    // Any Result is acceptable; only a panic would be a regression.
    let _ = noyalib::from_str::<Value>(s);
}

/// Second fuzz_yaml_v1_1 artefact tickling the same panic site
/// (`crash-4a2f242c91...`) — single-quoted scalar continuation
/// adjacent to flow markers. Different concrete bytes, same root
/// cause; kept as an independent regression so future refactors
/// can't accidentally re-introduce the panic on either shape.
#[test]
fn fetch_value_with_single_quoted_continuation_does_not_panic() {
    let input: &[u8] = b":\n\n')\t\t)\t)\t\x0b)\t)\t\x0b)\t\x0b!\t{\n:\n]";
    let s = std::str::from_utf8(input).unwrap();
    let _ = noyalib::from_str::<Value>(s);
}
