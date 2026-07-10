// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Internal base64 codec for `!!binary` scalars (YAML 1.2.2 §10.4).
//!
//! YAML's binary tag carries an RFC 4648 base64-encoded payload. The
//! standard alphabet, padding required, *whitespace tolerated* — a
//! `!!binary` scalar in real YAML often spans many lines indented
//! under a mapping key, so the decoder has to skip those interior
//! spaces / tabs / newlines before it builds the byte stream.
//!
//! We hand-roll a minimal codec here rather than pulling the
//! `base64` crate as a runtime dep — the surface we need is small,
//! the spec is fixed, and avoiding a new transitive dep keeps the
//! supply-chain ledger as tight as possible.

use crate::prelude::*;

/// Decode an RFC 4648 base64 string (standard alphabet, padding
/// required) into the corresponding byte sequence. ASCII whitespace
/// (` `, `\t`, `\r`, `\n`) is silently tolerated and discarded so
/// multi-line `!!binary` scalars round-trip without pre-processing
/// from the caller.
///
/// Returns `Err` with a short, deterministic message on:
/// invalid alphabet character, invalid length (after stripping
/// whitespace), invalid padding placement.
pub(crate) fn decode(input: &str) -> Result<Vec<u8>, &'static str> {
    // Build the decode lookup table at compile time: the standard
    // alphabet `A-Z a-z 0-9 + /`, with `=` flagged as the padding
    // sentinel.
    static LUT: [i8; 256] = build_lut();

    // Pre-walk: strip whitespace into a temporary so the rest of
    // the function works on dense bytes.
    let mut buf: Vec<u8> = Vec::with_capacity(input.len());
    for &b in input.as_bytes() {
        if matches!(b, b' ' | b'\t' | b'\r' | b'\n') {
            continue;
        }
        buf.push(b);
    }

    if buf.len() % 4 != 0 {
        return Err("base64 length not a multiple of 4 (after stripping whitespace)");
    }
    if buf.is_empty() {
        return Ok(Vec::new());
    }

    let mut out: Vec<u8> = Vec::with_capacity(buf.len() / 4 * 3);

    // Process groups of four 6-bit symbols → three 8-bit bytes.
    let mut i = 0;
    while i < buf.len() {
        let q0 = LUT[buf[i] as usize];
        let q1 = LUT[buf[i + 1] as usize];
        let q2 = LUT[buf[i + 2] as usize];
        let q3 = LUT[buf[i + 3] as usize];

        // -1 = invalid, -2 = '='. Negative-but-not-pad anywhere is
        // a hard error.
        if q0 < 0 || q1 < 0 {
            return Err("invalid base64 character (alphabet)");
        }
        let b0 = ((q0 as u32) << 18) | ((q1 as u32) << 12);

        match (q2, q3) {
            // Full quartet — three output bytes.
            (q2, q3) if q2 >= 0 && q3 >= 0 => {
                let v = b0 | ((q2 as u32) << 6) | (q3 as u32);
                out.push(((v >> 16) & 0xff) as u8);
                out.push(((v >> 8) & 0xff) as u8);
                out.push((v & 0xff) as u8);
            }
            // Final quartet with one byte of payload (`Xx==`).
            (-2, -2) if i + 4 == buf.len() => {
                out.push(((b0 >> 16) & 0xff) as u8);
            }
            // Final quartet with two bytes of payload (`XYZ=`).
            (q2, -2) if q2 >= 0 && i + 4 == buf.len() => {
                let v = b0 | ((q2 as u32) << 6);
                out.push(((v >> 16) & 0xff) as u8);
                out.push(((v >> 8) & 0xff) as u8);
            }
            _ => return Err("invalid base64 padding"),
        }

        i += 4;
    }

    Ok(out)
}

/// Encode a byte slice as standard-alphabet RFC 4648 base64 with
/// padding. Output is one continuous line — multi-line wrapping for
/// pretty-printed `!!binary` scalars is the serializer's job, not
/// the codec's.
pub(crate) fn encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let chunks = input.chunks_exact(3);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
        out.push(ALPHABET[(n & 0x3f) as usize] as char);
    }

    match remainder {
        [] => {}
        [a] => {
            let n = (*a as u32) << 16;
            out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
            out.push('=');
            out.push('=');
        }
        [a, b] => {
            let n = ((*a as u32) << 16) | ((*b as u32) << 8);
            out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
            out.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
            out.push('=');
        }
        _ => unreachable!("chunks_exact(3) leaves 0..=2 bytes"),
    }

    out
}

const fn build_lut() -> [i8; 256] {
    let mut lut = [-1_i8; 256];
    let mut i = 0;
    while i < 26 {
        lut[b'A' as usize + i] = i as i8;
        lut[b'a' as usize + i] = (i + 26) as i8;
        i += 1;
    }
    let mut i = 0;
    while i < 10 {
        lut[b'0' as usize + i] = (i + 52) as i8;
        i += 1;
    }
    lut[b'+' as usize] = 62;
    lut[b'/' as usize] = 63;
    lut[b'=' as usize] = -2; // padding sentinel
    lut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        assert!(decode("").unwrap().is_empty());
        assert!(encode(&[]).is_empty());
    }

    #[test]
    fn roundtrip_one_byte() {
        for b in 0u8..=255 {
            let s = encode(&[b]);
            assert_eq!(s.len(), 4);
            assert_eq!(decode(&s).unwrap(), vec![b]);
        }
    }

    #[test]
    fn roundtrip_two_bytes() {
        let bytes = [0xab, 0xcd];
        let s = encode(&bytes);
        assert_eq!(s, "q80=");
        assert_eq!(decode(&s).unwrap(), bytes);
    }

    #[test]
    fn roundtrip_three_bytes() {
        let bytes = [0x01, 0x02, 0x03];
        let s = encode(&bytes);
        assert_eq!(s, "AQID");
        assert_eq!(decode(&s).unwrap(), bytes);
    }

    #[test]
    fn roundtrip_hello() {
        let s = encode(b"Hello, World!");
        assert_eq!(s, "SGVsbG8sIFdvcmxkIQ==");
        assert_eq!(decode(&s).unwrap(), b"Hello, World!");
    }

    #[test]
    fn decode_tolerates_whitespace_and_newlines() {
        // Mimics how a YAML !!binary scalar appears under a mapping
        // key after the block scalar reader has folded continuation
        // lines: spaces and newlines interleaved.
        let s = "SGVs\n  bG8s\n  IFdv\n  cmxk\n  IQ==\n";
        assert_eq!(decode(s).unwrap(), b"Hello, World!");
    }

    #[test]
    fn decode_rejects_invalid_alphabet() {
        assert!(decode("**==").is_err());
    }

    #[test]
    fn decode_rejects_bad_padding_position() {
        // Padding may only appear in the final quartet.
        assert!(decode("AB==CDEF").is_err());
    }

    #[test]
    fn decode_rejects_bad_length() {
        // Three characters total — not a complete base64 group.
        assert!(decode("ABC").is_err());
    }

    #[test]
    fn roundtrip_random_byte_pattern() {
        let bytes: Vec<u8> = (0..=255_u8).cycle().take(1023).collect();
        let s = encode(&bytes);
        assert_eq!(decode(&s).unwrap(), bytes);
    }

    #[test]
    fn build_lut_maps_alphabet_and_sentinels() {
        // `build_lut` is a `const fn` used only to initialise the
        // `static LUT` at compile time, so runtime coverage never sees
        // its body. Calling it in this (non-const) test context executes
        // it under instrumentation and validates every arm of the table.
        let lut = build_lut();
        // A-Z -> 0..=25
        assert_eq!(lut[b'A' as usize], 0);
        assert_eq!(lut[b'Z' as usize], 25);
        // a-z -> 26..=51
        assert_eq!(lut[b'a' as usize], 26);
        assert_eq!(lut[b'z' as usize], 51);
        // 0-9 -> 52..=61
        assert_eq!(lut[b'0' as usize], 52);
        assert_eq!(lut[b'9' as usize], 61);
        // Non-alphabet symbols.
        assert_eq!(lut[b'+' as usize], 62);
        assert_eq!(lut[b'/' as usize], 63);
        assert_eq!(lut[b'=' as usize], -2, "padding sentinel");
        // Everything else is rejected (-1).
        assert_eq!(lut[b' ' as usize], -1);
        assert_eq!(lut[0], -1);
        assert_eq!(lut[255], -1);
        // Agreement with the standard base64 alphabet: decoding the i-th
        // alphabet byte yields i.
        let alphabet: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        for (i, &c) in alphabet.iter().enumerate() {
            assert_eq!(lut[c as usize], i as i8, "alphabet byte {c} at index {i}");
        }
    }
}
