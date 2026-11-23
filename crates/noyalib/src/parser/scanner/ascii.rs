//! Word-at-a-time screen for printable ASCII, the fast path of the
//! unquoted-scalar printable check in `scalars.rs`.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

/// `0x01` in every byte of a word.
const ONES: u64 = u64::from_le_bytes([0x01; 8]);
/// The high bit of every byte of a word.
const HIGHS: u64 = u64::from_le_bytes([0x80; 8]);

/// True when every byte is printable ASCII (`0x20..=0x7E`).
///
/// Such text holds no raw control, no DEL and no multi-byte character,
/// so it passes every unquoted-scalar printable check (§5.1) and needs
/// no closer look. Anything else returns `false` and takes the exact
/// byte-level checks, which decide the error.
///
/// Eight bytes per step: a byte's high bit is set in `word` when it is
/// non-ASCII, in `below_space` when it is below `0x20`, and in `del`
/// when it is `0x7F`. A borrow only crosses bytes after a byte that
/// already flags, so the test is exact for "any byte flags".
pub(super) fn is_printable_ascii(bytes: &[u8]) -> bool {
    let (words, tail) = bytes.as_chunks::<8>();
    let mut flags = 0;
    for &chunk in words {
        let word = u64::from_le_bytes(chunk);
        let below_space = word.wrapping_sub(ONES * 0x20) & !word;
        let xor_del = word ^ (ONES * 0x7F);
        let del = xor_del.wrapping_sub(ONES) & !xor_del;
        flags |= word | below_space | del;
    }
    flags & HIGHS == 0 && tail.iter().all(|&b| (0x20..0x7F).contains(&b))
}

#[cfg(test)]
mod tests {
    use super::is_printable_ascii;

    /// The reference the word-at-a-time screen must agree with.
    fn reference(bytes: &[u8]) -> bool {
        bytes.iter().all(|&b| (0x20..0x7F).contains(&b))
    }

    #[test]
    fn agrees_with_the_bytewise_predicate_at_every_position() {
        // Every byte value, alone and at every offset of a printable run
        // long enough to cover two whole words and a tail.
        for len in 0..20 {
            for at in 0..len {
                for b in 0..=u8::MAX {
                    let mut buf = [b'a'; 20];
                    buf[at] = b;
                    let text = &buf[..len];
                    assert_eq!(
                        is_printable_ascii(text),
                        reference(text),
                        "byte {b:#04x} at {at} of {len}"
                    );
                }
            }
        }
    }

    #[test]
    fn accepts_the_printable_bounds_and_rejects_their_neighbours() {
        assert!(is_printable_ascii(b""));
        assert!(is_printable_ascii(b" ~ ~ ~ ~ ~ ~ ~ ~"));
        assert!(!is_printable_ascii(b"key: value\x1f text"));
        assert!(!is_printable_ascii(b"key: value\x7f text"));
        assert!(!is_printable_ascii(b"key: value\t text"));
        assert!(!is_printable_ascii("key: caf\u{e9} text".as_bytes()));
    }

    #[test]
    fn a_borrow_after_a_flagged_byte_does_not_hide_it() {
        // `0x00` borrows into the next byte; the result must still flag.
        assert!(!is_printable_ascii(&[
            0x00, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20
        ]));
        assert!(!is_printable_ascii(&[
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x00
        ]));
    }
}
