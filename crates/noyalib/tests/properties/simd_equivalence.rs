// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase 4 — exhaustive equivalence between SIMD-routed
//! `find_any_of` and the scalar baseline.
//!
//! Strategy: for every needle-set arity from 0 to 16, fill a
//! haystack with non-needle bytes, sweep a needle through every
//! position, and assert both implementations agree. Plus a
//! stress run on a 64 KiB haystack with sparse and dense needle
//! placement to exercise the SWAR chunk loop's straddling
//! behaviour at large scale.

#![cfg(feature = "simd")]
#![allow(missing_docs)]

use noyalib::simd::{bitmap_for, clean_prefix_len, find_any_of, find_byte_in_bitmap};

fn scalar_find(haystack: &[u8], needles: &[u8]) -> Option<usize> {
    for (i, &b) in haystack.iter().enumerate() {
        if needles.contains(&b) {
            return Some(i);
        }
    }
    None
}

#[test]
fn equivalence_arity_0_through_16() {
    // Cover every dispatch path: arity 0 (returns None), arity
    // 1/2/3 (memchr), arity 4+ (SWAR).
    for arity in 0..=16 {
        let needles: Vec<u8> = (0..arity).map(|i| b'!' + i as u8).collect();
        for length in [
            0usize, 1, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 256, 1024,
        ] {
            let mut buf = vec![0u8; length];
            // Fill with bytes that are guaranteed not to be in
            // the needle set.
            for (i, slot) in buf.iter_mut().enumerate() {
                let mut v = (i as u8).wrapping_add(0x80);
                while needles.contains(&v) {
                    v = v.wrapping_add(1);
                }
                *slot = v;
            }
            assert_eq!(
                find_any_of(&buf, &needles),
                scalar_find(&buf, &needles),
                "arity={arity} length={length} needles={needles:?}",
            );

            if length == 0 || arity == 0 {
                continue;
            }
            // Sprinkle each needle at each position and verify
            // both implementations agree on the *first* match.
            for pos in 0..length {
                let saved = buf[pos];
                for &n in &needles {
                    buf[pos] = n;
                    assert_eq!(
                        find_any_of(&buf, &needles),
                        scalar_find(&buf, &needles),
                        "arity={arity} length={length} pos={pos} needle={n:#x}",
                    );
                }
                buf[pos] = saved;
            }
        }
    }
}

#[test]
fn equivalence_64kib_dense_needles() {
    // Stress the chunk loop with a large haystack and a
    // structurally-dense needle set (every other byte position
    // sees a hit).
    let needles: &[u8] = b":,#=[]{}\n\t '\"\\";
    let mut buf = vec![0u8; 64 * 1024];
    // Fill with high-bit bytes — guaranteed non-needle.
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot = 0x80 | ((i & 0x7F) as u8);
    }
    // Place a needle every 7 bytes.
    for i in (3..buf.len()).step_by(7) {
        buf[i] = needles[i % needles.len()];
    }
    let pos_simd = find_any_of(&buf, needles);
    let pos_scalar = scalar_find(&buf, needles);
    assert_eq!(pos_simd, pos_scalar, "first match must agree");
    assert_eq!(pos_simd, Some(3));
}

#[test]
fn equivalence_64kib_sparse_needle() {
    let needles: &[u8] = b":#=,";
    let mut buf = vec![b'.'; 64 * 1024];
    // Single needle at the very end.
    let last = buf.len() - 1;
    buf[last] = b':';
    assert_eq!(find_any_of(&buf, needles), scalar_find(&buf, needles),);
    assert_eq!(find_any_of(&buf, needles), Some(64 * 1024 - 1));
}

#[test]
fn clean_prefix_len_equivalence() {
    let inputs: &[(&[u8], &[u8])] = &[
        (b"plain text: with colon", b":,\n#"),
        (b"no needles here", b":,\n#"),
        (b":start", b":,\n#"),
        (b"", b":,\n#"),
        (b"long           text       with    spaces", b":\n#"),
    ];
    for (haystack, needles) in inputs {
        let expected = scalar_find(haystack, needles).unwrap_or(haystack.len());
        assert_eq!(clean_prefix_len(haystack, needles), expected);
    }
}

#[test]
fn bitmap_path_equivalence() {
    // The bitmap entry-point must agree with find_any_of for the
    // same needle set.
    let needle_sets: &[&[u8]] = &[b":,\n", b":,\n#", b"[]{}:,#", b" \t:,\n#="];
    for &needles in needle_sets {
        let bm = bitmap_for(needles);
        let inputs: &[&[u8]] = &[
            b"",
            b".",
            b"plain text",
            b"text: with: many: colons",
            b"a,b,c,d,e",
        ];
        for haystack in inputs {
            assert_eq!(
                find_byte_in_bitmap(haystack, &bm),
                find_any_of(haystack, needles),
                "needles={:?} haystack={:?}",
                needles,
                core::str::from_utf8(haystack).unwrap_or("<binary>"),
            );
        }
    }
}

#[test]
fn full_byte_range_membership() {
    // Bitmap covering every possible byte value classifies every
    // byte as a member.
    let all: Vec<u8> = (0u8..=255).collect();
    let bm = bitmap_for(&all);
    for b in 0u8..=255 {
        assert!(bm.contains(b));
    }
    // First-match position is always 0 for non-empty input.
    assert_eq!(find_byte_in_bitmap(&[42, 17, 9], &bm), Some(0));
}

#[test]
fn duplicate_needles_idempotent() {
    let with_dups = b":::,,:";
    let without = b":,";
    let inputs: &[&[u8]] = &[b"abc:def", b"plain", b",,first"];
    for h in inputs {
        assert_eq!(find_any_of(h, with_dups), find_any_of(h, without));
    }
}
