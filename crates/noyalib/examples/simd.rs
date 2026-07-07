// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! SIMD-friendly structural scanning primitives.
//!
//! Requires `--features simd`. Noyalib's scanner uses the same
//! `simd::find_any_of` primitive to jump over plain scalar text
//! byte-at-a-time. Downstream callers building custom scanners
//! (grammar-driven config parsers, log formatters, YAML dialects
//! layered on top of noyalib) can reuse the same primitive.
//!
//! Run:
//! ```text
//! cargo run --example simd --features simd --release
//! ```

use noyalib::simd::{ByteBitmap, SimdScanner, bitmap_for, clean_prefix_len, find_any_of};

fn main() {
    // ── 1. Single-byte search ────────────────────────────────
    // The 1-needle path routes through memchr → SSE2/NEON.
    let line = b"key: value  # trailing comment";
    assert_eq!(find_any_of(line, b":"), Some(3));
    println!("find_any_of(':') → {:?}", find_any_of(line, b":"));

    // ── 2. Multi-byte search ─────────────────────────────────
    // 4+ needles route through the SWAR path (8 bytes per lane).
    // Classic YAML plain-scalar-terminator set.
    let plain_end = find_any_of(line, b":,#\n");
    assert_eq!(plain_end, Some(3));
    println!("plain-terminator set → {plain_end:?}");

    // ── 3. Skip-clean-prefix ─────────────────────────────────
    // How far can we advance before we hit a terminator? This is
    // the exact question a scanner asks when scoping a plain
    // scalar. Same result as find_any_of but expresses the intent
    // directly.
    let advance = clean_prefix_len(b"abc def:more", b":,#\n");
    assert_eq!(advance, 7); // "abc def"
    println!("clean_prefix_len before terminator → {advance}");

    // ── 4. Pre-built bitmap ──────────────────────────────────
    // For hot loops that reuse the same needle set, build the
    // bitmap once and keep it. `ByteBitmap` is `Copy` — cheap to
    // pass by value.
    let terminators: ByteBitmap = bitmap_for(b":,#\n");
    assert!(terminators.contains(b':'));
    assert!(terminators.contains(b'\n'));
    assert!(!terminators.contains(b'a'));
    println!("bitmap_for(\":,#\\n\") built once, reused per line");

    // ── 5. Structural scan on a whole document ───────────────
    // `SimdScanner` is the stateful side of the API — construct
    // once with your needle set, then run scan queries against
    // multiple haystacks.
    let scanner = SimdScanner::new(b":,{}[]");
    let doc = b"{key: [a, b], nested: {inner: value}}";
    let mut cursor = 0;
    let mut hits = 0;
    while let Some(offset) = scanner.find_any(&doc[cursor..]) {
        hits += 1;
        cursor += offset + 1;
        if hits > 32 {
            break; // guard rail
        }
    }
    println!(
        "SimdScanner found {hits} structural bytes in {} bytes",
        doc.len()
    );

    // ── 6. Miss case ─────────────────────────────────────────
    // Returns None when no needle byte is present.
    assert_eq!(find_any_of(b"abcdefgh", b"XYZ"), None);
    println!("find_any_of on a needle-free haystack → None");
}
