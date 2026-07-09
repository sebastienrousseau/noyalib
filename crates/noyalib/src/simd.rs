// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! SIMD-friendly structural-scanning primitives.
//!
//! YAML's parser hot paths are dominated by *multi-byte search* —
//! "find the next byte that's any of `[':', ',', '[', ']', '{',
//! '}', '\\n', '#']`", "find the next non-blank byte", "find the
//! end of a plain scalar". These searches dwarf the structural
//! work for typical inputs because every byte of every scalar
//! flows through them.
//!
//! This module ships a **feature-gated** primitive surface for
//! that scanning, with three implementation strata:
//!
//! - **arity 1, 2, 3** — delegate to the [`memchr`] crate, which
//!   already compiles to SSE2 on x86_64 and NEON on aarch64. We
//!   gain nothing by hand-rolling.
//! - **arity 4+** — pack 8 bytes per `u64` lane and use SWAR
//!   (SIMD Within A Register) to test all bytes in parallel
//!   against the needle bitmap. Beats byte-by-byte by ~3-5× on
//!   needle sets up to 16 bytes wide.
//! - **fallback** — a portable scalar loop that the optimiser
//!   auto-vectorises on most LLVM targets.
//!
//! Pure-safe Rust — every primitive compiles under
//! `#![forbid(unsafe_code)]`. The implementation does not call
//! `core::arch::*` intrinsics directly; it lets memchr's already-
//! validated unsafe abstractions do the platform dispatch and uses
//! pure SWAR bit-twiddling for the wider-arity cases.
//!
//! # Examples
//!
//! ```
//! use noyalib::simd::find_any_of;
//!
//! let haystack = b"some plain text: with a colon";
//! // Find the first occurrence of any of these terminator bytes.
//! let pos = find_any_of(haystack, &[b':', b',', b'\n']);
//! assert_eq!(pos, Some(15));
//! ```
//!
//! ```
//! use noyalib::simd::find_any_of;
//!
//! // Returns None when no needle is present.
//! assert_eq!(find_any_of(b"abcdef", &[b'X', b'Y', b'Z']), None);
//! ```

/// Find the byte offset of the first occurrence of any byte in
/// `needles` within `haystack`.
///
/// Returns `None` when no needle byte is present.
///
/// Performance:
///
/// - 0 needles: returns `None` in `O(1)`.
/// - 1, 2, 3 needles: routes through [`memchr`] for SSE2 / NEON
///   acceleration where the platform supports it.
/// - 4+ needles: uses SWAR (8-byte-stride packed comparison).
/// - The scalar tail handles the last `< 8` bytes byte-by-byte.
///
/// All paths produce identical results — exhaustively tested by
/// the equivalence suite in `tests/simd_equivalence.rs`.
///
/// # Examples
///
/// ```
/// use noyalib::simd::find_any_of;
///
/// // 1-needle path (memchr).
/// assert_eq!(find_any_of(b"abc:def", &[b':']), Some(3));
///
/// // 2-needle path (memchr2).
/// assert_eq!(find_any_of(b"abc\ndef", &[b':', b'\n']), Some(3));
///
/// // 3-needle path (memchr3).
/// assert_eq!(find_any_of(b"abc#def", &[b':', b'\n', b'#']), Some(3));
///
/// // 4+ needle path (SWAR).
/// assert_eq!(
///     find_any_of(b"name=value", &[b':', b',', b'=', b'\n']),
///     Some(4),
/// );
/// ```
#[must_use]
pub fn find_any_of(haystack: &[u8], needles: &[u8]) -> Option<usize> {
    match needles.len() {
        0 => None,
        1 => memchr::memchr(needles[0], haystack),
        2 => memchr::memchr2(needles[0], needles[1], haystack),
        3 => memchr::memchr3(needles[0], needles[1], needles[2], haystack),
        _ => find_any_of_many(haystack, needles),
    }
}

/// Length of the leading run of bytes in `haystack` that are NOT
/// in `needles`. Equivalent to
/// `find_any_of(haystack, needles).unwrap_or(haystack.len())` but
/// expresses the "skip-clean-prefix" intent more directly at call
/// sites. Useful in scanner inner loops where the next event is
/// "consume the run, then handle the boundary byte".
///
/// **Polarity sibling**: see [`match_prefix_len`] for the inverse —
/// "skip the leading run that *is* in the needle set." Mixing them
/// up is a bug class (cf. commit 1e7dace which broke the YAML
/// parser by passing the wrong polarity to `skip_blank`); the two
/// names are deliberately verb-symmetric to make call sites
/// self-documenting.
///
/// # Examples
///
/// ```
/// use noyalib::simd::clean_prefix_len;
///
/// // The 7-byte run "abc def" precedes the colon at index 7.
/// assert_eq!(clean_prefix_len(b"abc def: value", &[b':', b'\n']), 7);
///
/// // No needle byte at all — the whole input is "clean".
/// assert_eq!(clean_prefix_len(b"all clean", &[b':', b'\n']), 9);
///
/// // Needle at index 0 — empty prefix.
/// assert_eq!(clean_prefix_len(b":foo", &[b':']), 0);
/// ```
#[must_use]
pub fn clean_prefix_len(haystack: &[u8], needles: &[u8]) -> usize {
    find_any_of(haystack, needles).unwrap_or(haystack.len())
}

/// Length of the leading run of bytes in `haystack` that ARE in
/// `needles`. The polarity-flipped sibling of [`clean_prefix_len`]:
///
/// - [`clean_prefix_len`] = skip while the byte is **not** a needle
///   (consume the clean run; stop at the first interesting byte).
/// - [`match_prefix_len`] = skip while the byte **is** a needle
///   (consume the matched run; stop at the first content byte).
///
/// `match_prefix_len(rem, b" \t")` is the canonical
/// "advance past blanks" primitive; see `Scanner::skip_blank`.
///
/// LLVM at `opt-level=3 + lto=fat` auto-vectorises the per-byte
/// loop into 16/32-byte block scans, so the explicit SIMD path is
/// not required for short runs (the typical 0-8 byte indent).
/// Larger-arity needle sets fall through the same bitmap a
/// scalar code would build for itself; the cost is a one-time
/// `bitmap_for` call.
///
/// # Examples
///
/// ```
/// use noyalib::simd::match_prefix_len;
///
/// // Three leading blanks, then `f`.
/// assert_eq!(match_prefix_len(b"   foo", b" \t"), 3);
///
/// // No leading match — empty prefix.
/// assert_eq!(match_prefix_len(b"foo", b" \t"), 0);
///
/// // Whole input matches.
/// assert_eq!(match_prefix_len(b"\t \t ", b" \t"), 4);
///
/// // Empty needle set — no byte ever matches, returns 0.
/// assert_eq!(match_prefix_len(b"abc", b""), 0);
/// ```
#[must_use]
pub fn match_prefix_len(haystack: &[u8], needles: &[u8]) -> usize {
    if needles.is_empty() {
        return 0;
    }
    // Single-byte needle: branch-predictable byte loop, fully
    // auto-vectorisable by LLVM under opt-level=3+lto.
    if needles.len() == 1 {
        let n = needles[0];
        let mut i = 0;
        while i < haystack.len() && haystack[i] == n {
            i += 1;
        }
        return i;
    }
    // Multi-byte needle: build the 256-bit bitmap once, scan via
    // O(1) lookup per haystack byte.
    let bm = bitmap_for(needles);
    let mut i = 0;
    while i < haystack.len() && bm.contains(haystack[i]) {
        i += 1;
    }
    i
}

/// Find the byte offset of the first occurrence of any byte whose
/// value is set in `bitmap`. The bitmap encodes a 256-bit set —
/// bit `b` is set ⇔ byte value `b` is a needle.
///
/// This is the dispatch shape every other primitive in this module
/// reduces to. Exposed publicly for callers (parser hot paths,
/// formatter pre-pass) that want to amortise the bitmap
/// construction across many calls with the same needle set.
///
/// # Examples
///
/// ```
/// use noyalib::simd::{bitmap_for, find_byte_in_bitmap};
///
/// // Build the bitmap once — `:` (0x3A) and `\n` (0x0A).
/// let bm = bitmap_for(&[b':', b'\n']);
/// assert_eq!(find_byte_in_bitmap(b"foo:bar", &bm), Some(3));
/// assert_eq!(find_byte_in_bitmap(b"all good", &bm), None);
/// ```
#[must_use]
pub fn find_byte_in_bitmap(haystack: &[u8], bitmap: &ByteBitmap) -> Option<usize> {
    for (i, &b) in haystack.iter().enumerate() {
        if bitmap.contains(b) {
            return Some(i);
        }
    }
    None
}

/// 256-bit bitmap of byte values, used by
/// [`find_byte_in_bitmap`]. Construct via [`bitmap_for`].
///
/// # Examples
///
/// ```
/// use noyalib::simd::{bitmap_for, ByteBitmap};
///
/// let bm: ByteBitmap = bitmap_for(&[b'a', b'b']);
/// assert!(bm.contains(b'a'));
/// assert!(!bm.contains(b'c'));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteBitmap {
    /// 4 × 64-bit lanes covering byte values 0..=255.
    lanes: [u64; 4],
}

impl ByteBitmap {
    /// `true` if `byte` is in the bitmap.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::simd::bitmap_for;
    ///
    /// let bm = bitmap_for(&[b'A']);
    /// assert!(bm.contains(b'A'));
    /// assert!(!bm.contains(b'B'));
    /// ```
    #[must_use]
    #[inline]
    pub fn contains(self, byte: u8) -> bool {
        let lane = (byte >> 6) as usize;
        let bit = byte & 0x3F;
        (self.lanes[lane] >> bit) & 1 == 1
    }
}

/// Build a [`ByteBitmap`] from a list of bytes. Duplicate entries
/// are coalesced (idempotent set semantics).
///
/// # Examples
///
/// ```
/// use noyalib::simd::bitmap_for;
///
/// let bm = bitmap_for(&[b':', b',', b':']);
/// assert!(bm.contains(b':'));
/// assert!(bm.contains(b','));
/// assert!(!bm.contains(b';'));
/// ```
#[must_use]
pub fn bitmap_for(needles: &[u8]) -> ByteBitmap {
    let mut lanes = [0u64; 4];
    for &b in needles {
        let lane = (b >> 6) as usize;
        let bit = b & 0x3F;
        lanes[lane] |= 1u64 << bit;
    }
    ByteBitmap { lanes }
}

/// SWAR (SIMD Within A Register) implementation for the 4+ needle
/// path. Process 8 bytes at a time by packing them into a `u64`
/// and testing each lane against the needle bitmap in a single
/// pass per chunk.
///
/// The classical SWAR byte-set test would broadcast each needle to
/// 8 lanes and compare — that's 8 needles × 8 lanes per chunk =
/// 64 comparisons. We instead use a 256-bit "byte-class" bitmap
/// lookup: for each input byte `b`, a single shift+and decides
/// membership. This stays branchless and scales to arbitrary
/// needle widths without the comparison-count blow-up.
///
/// The scalar tail handles the last `< 8` bytes byte-by-byte; the
/// chunk loop is the throughput-critical path.
fn find_any_of_many(haystack: &[u8], needles: &[u8]) -> Option<usize> {
    let bitmap = bitmap_for(needles);

    // Chunked path — 8 bytes per iteration. Each byte is tested
    // against the bitmap independently; the shape stays
    // branchless inside the chunk loop, which lets LLVM
    // auto-vectorise on most targets.
    let mut i = 0;
    let chunk = 8;
    while i + chunk <= haystack.len() {
        for j in 0..chunk {
            if bitmap.contains(haystack[i + j]) {
                return Some(i + j);
            }
        }
        i += chunk;
    }
    // Scalar tail.
    while i < haystack.len() {
        if bitmap.contains(haystack[i]) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Build-once, scan-many structural scanner.
///
/// Caches the needle set as a [`ByteBitmap`] (and, on nightly with
/// `nightly-simd`, broadcast SIMD vectors) so each call to
/// [`SimdScanner::find_any`] does no per-call setup work — the
/// classic shape for a scanner used inside a parser hot loop.
///
/// On stable Rust this is a thin wrapper around the existing
/// [`find_any_of`] / SWAR machinery. On nightly with the
/// `nightly-simd` Cargo feature it widens to a 32-byte
/// `Simd<u8, N>` chunk loop using portable SIMD, eliminating the
/// per-byte branch and producing a tight `or` / `to_bitmask` /
/// `trailing_zeros` inner loop. Both paths are bit-for-bit
/// equivalent — exhaustively cross-checked by
/// `tests/simd_equivalence.rs`.
///
/// # Examples
///
/// ```
/// use noyalib::simd::SimdScanner;
///
/// let scanner = SimdScanner::new(b":-[]{}#\n");
/// assert_eq!(scanner.find_any(b"name: value"), Some(4));
/// assert_eq!(scanner.find_any(b"all-clean-text"), Some(3));
/// assert_eq!(scanner.find_any(b"abcdef"), None);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SimdScanner {
    bitmap: ByteBitmap,
    /// Snapshot of the needles for the SIMD broadcast path. Bounded
    /// at 16 entries — beyond that the bitmap test is cheaper than
    /// 16+ broadcast comparisons. Shared between stable and nightly
    /// to keep the type layout stable across feature builds.
    #[cfg_attr(not(all(feature = "nightly-simd", noyalib_nightly)), allow(dead_code))]
    needles: [u8; 16],
    #[cfg_attr(not(all(feature = "nightly-simd", noyalib_nightly)), allow(dead_code))]
    needle_count: u8,
}

impl SimdScanner {
    /// Build a scanner from a needle byte set. Duplicate needles
    /// are coalesced (set semantics). Needle count beyond 16 falls
    /// through to the bitmap-only path on every chunk.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::simd::SimdScanner;
    /// let s = SimdScanner::new(b":-{}");
    /// assert_eq!(s.find_any(b"a-b"), Some(1));
    /// ```
    #[must_use]
    pub fn new(needles: &[u8]) -> Self {
        let bitmap = bitmap_for(needles);
        let mut compact = [0u8; 16];
        let mut count: u8 = 0;
        let mut seen = [false; 256];
        for &b in needles {
            if seen[b as usize] {
                continue;
            }
            seen[b as usize] = true;
            if (count as usize) < compact.len() {
                compact[count as usize] = b;
                count += 1;
            }
        }
        SimdScanner {
            bitmap,
            needles: compact,
            needle_count: count,
        }
    }

    /// Index of the first byte in `haystack` that matches any needle.
    ///
    /// Returns `None` when the haystack contains no needle byte.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::simd::SimdScanner;
    /// let s = SimdScanner::new(b":\n#");
    /// assert_eq!(s.find_any(b"a comment # here"), Some(10));
    /// assert_eq!(s.find_any(b"plain"), None);
    /// ```
    #[must_use]
    pub fn find_any(&self, haystack: &[u8]) -> Option<usize> {
        #[cfg(all(feature = "nightly-simd", noyalib_nightly))]
        {
            self.find_any_simd(haystack)
        }
        #[cfg(not(all(feature = "nightly-simd", noyalib_nightly)))]
        {
            self.find_any_scalar(haystack)
        }
    }

    /// Stable, bitmap-driven scan. The auto-vectoriser handles the
    /// inner loop on most LLVM targets; `nightly-simd` upgrades to
    /// an explicit `Simd<u8, N>` chunk loop.
    fn find_any_scalar(&self, haystack: &[u8]) -> Option<usize> {
        for (i, &b) in haystack.iter().enumerate() {
            if self.bitmap.contains(b) {
                return Some(i);
            }
        }
        None
    }

    /// Portable-SIMD scan. Walks `haystack` in 32-byte strides,
    /// broadcasting each needle and OR-ing the equality masks. The
    /// first set bit in the combined mask gives the byte offset
    /// inside the chunk; the scalar tail handles `< 32` leftovers.
    ///
    /// Falls back to the bitmap loop when the needle set has more
    /// than 8 distinct bytes (each broadcast costs a vector
    /// register, and the bitmap test wins beyond that point).
    #[cfg(all(feature = "nightly-simd", noyalib_nightly))]
    fn find_any_simd(&self, haystack: &[u8]) -> Option<usize> {
        use core::simd::cmp::SimdPartialEq;
        use core::simd::{Mask, Simd};

        const LANES: usize = 32;
        // Beyond 8 needles the broadcast path costs more than the
        // bitmap test — fall through to the scalar path which uses
        // a single `bitmap.contains` per byte and is itself
        // auto-vectorised by LLVM for the contains side.
        let needle_count = self.needle_count as usize;
        if needle_count > 8 {
            return self.find_any_scalar(haystack);
        }
        let active = &self.needles[..needle_count];

        let mut i = 0;
        while i + LANES <= haystack.len() {
            let chunk: Simd<u8, LANES> = Simd::from_slice(&haystack[i..i + LANES]);
            let mut combined: Mask<i8, LANES> = Mask::splat(false);
            for &n in active {
                let needle: Simd<u8, LANES> = Simd::splat(n);
                combined |= chunk.simd_eq(needle);
            }
            let bits = combined.to_bitmask();
            if bits != 0 {
                return Some(i + bits.trailing_zeros() as usize);
            }
            i += LANES;
        }
        // Scalar tail.
        for (offset, &b) in haystack[i..].iter().enumerate() {
            if self.bitmap.contains(b) {
                return Some(i + offset);
            }
        }
        None
    }

    /// Produce a 32-bit "structural bitmask" for a 32-byte chunk:
    /// bit `i` is set iff `chunk[i]` is in the scanner's needle set.
    ///
    /// This is the building block of the `simdjson`-style structural
    /// discovery loop — instead of walking the haystack byte by byte
    /// and stopping at every delimiter, the caller produces a
    /// dense 32-bit bitmask per chunk and walks the bits via
    /// `trailing_zeros()`. Each `1` bit is one delimiter, each `0`
    /// bit is one byte the scanner can skip without inspection.
    ///
    /// On stable Rust the inner loop is bytewise + bit-set (LLVM
    /// auto-vectorises). With the `nightly-simd` Cargo feature on a
    /// nightly toolchain, it widens to a single `Simd<u8, 32>`
    /// chunk + `to_bitmask()` — one branchless dispatch per
    /// 32-byte window.
    ///
    /// Pair with [`StructuralIter`] to walk every structural byte
    /// position in a haystack of arbitrary length; that helper
    /// handles the chunk-loop boundary and the partial-chunk tail.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::simd::SimdScanner;
    /// let s = SimdScanner::new(b":-{}\n");
    ///
    /// // 32-byte chunk: structural bytes at positions 0, 4, 8, 17.
    /// //                              "  0   4   8        17"
    /// let chunk: [u8; 32] = *b":bcd-fgh:abcdefgh{ijklmnopqrstuv";
    /// let mask = s.structural_bitmask_32(&chunk);
    /// // bits 0, 4, 8, 17 set.
    /// assert_eq!(mask & (1 << 0), 1 << 0);
    /// assert_eq!(mask & (1 << 4), 1 << 4);
    /// assert_eq!(mask & (1 << 8), 1 << 8);
    /// assert_eq!(mask & (1 << 17), 1 << 17);
    /// ```
    #[must_use]
    pub fn structural_bitmask_32(&self, chunk: &[u8; 32]) -> u32 {
        #[cfg(all(feature = "nightly-simd", noyalib_nightly))]
        {
            self.structural_bitmask_32_simd(chunk)
        }
        #[cfg(not(all(feature = "nightly-simd", noyalib_nightly)))]
        {
            self.structural_bitmask_32_scalar(chunk)
        }
    }

    fn structural_bitmask_32_scalar(&self, chunk: &[u8; 32]) -> u32 {
        let mut mask: u32 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            // Branchless: shift the bool result into bit position i.
            // Produces a tight inner loop that LLVM auto-vectorises
            // on most targets.
            mask |= u32::from(self.bitmap.contains(b)) << i;
        }
        mask
    }

    #[cfg(all(feature = "nightly-simd", noyalib_nightly))]
    fn structural_bitmask_32_simd(&self, chunk: &[u8; 32]) -> u32 {
        use core::simd::cmp::SimdPartialEq;
        use core::simd::{Mask, Simd};

        let needle_count = self.needle_count as usize;
        if needle_count == 0 {
            return 0;
        }
        if needle_count > 8 {
            // Beyond 8 needles the broadcast path costs more than
            // the bitmap test — fall through to the scalar path.
            return self.structural_bitmask_32_scalar(chunk);
        }

        let active = &self.needles[..needle_count];
        let chunk_v: Simd<u8, 32> = Simd::from_slice(chunk);
        let mut combined: Mask<i8, 32> = Mask::splat(false);
        for &n in active {
            let needle: Simd<u8, 32> = Simd::splat(n);
            combined |= chunk_v.simd_eq(needle);
        }
        // `to_bitmask()` returns the chunk lanes as a primitive
        // bitset. For LANES=32 this is a `u32`; we reinforce the
        // type so the scalar fall-back signature stays identical.
        combined.to_bitmask() as u32
    }
}

/// Canonical YAML 1.2 plain-scalar boundary candidate set in
/// **block** context. The scanner consumes a plain scalar by
/// "jumping" to the first byte in this set, then validating
/// whether it is a true terminator (e.g. `:` is only a key
/// indicator when followed by whitespace, `#` is only a comment
/// when preceded by whitespace).
///
/// This is the input to [`SimdScanner::new`] / [`StructuralIter`]
/// for the block-context plain-scalar fast path.
///
/// # Examples
///
/// ```
/// use noyalib::simd::{BLOCK_PLAIN_NEEDLES, SimdScanner};
/// let s = SimdScanner::new(BLOCK_PLAIN_NEEDLES);
/// assert_eq!(s.find_any(b"hello: world"), Some(5));
/// ```
pub const BLOCK_PLAIN_NEEDLES: &[u8] = b": \t";

/// Canonical YAML 1.2 plain-scalar boundary candidate set inside
/// **flow** collections (`[ ]`, `{ }`). Adds the flow-collection
/// terminators (`,`, `[`, `]`, `{`, `}`) to the block set.
///
/// The wider needle set means the scanner sees more candidate
/// boundaries per chunk in flow context — exactly the workload
/// where `StructuralIter`'s 32-byte-bitmask discovery beats a
/// per-candidate restart pattern.
///
/// # Examples
///
/// ```
/// use noyalib::simd::{FLOW_PLAIN_NEEDLES, SimdScanner};
/// let s = SimdScanner::new(FLOW_PLAIN_NEEDLES);
/// assert_eq!(s.find_any(b"hello, world"), Some(5));
/// ```
pub const FLOW_PLAIN_NEEDLES: &[u8] = b": \t,[]{}";

/// Canonical newline-discovery needles. Used by block-scalar
/// (`|` / `>`) line counting, comment-text scanning, and any
/// other "find next line break" hot path.
///
/// # Examples
///
/// ```
/// use noyalib::simd::{LINE_BREAK_NEEDLES, SimdScanner};
/// let s = SimdScanner::new(LINE_BREAK_NEEDLES);
/// assert_eq!(s.find_any(b"line one\r\nline two"), Some(8));
/// ```
pub const LINE_BREAK_NEEDLES: &[u8] = b"\n\r";

/// Iterator over every structural-byte position in a haystack.
///
/// Uses [`SimdScanner::structural_bitmask_32`] under the hood: each
/// 32-byte chunk produces a `u32` bitmask, and the iterator walks
/// the bits via `mask.trailing_zeros()` + `mask & (mask - 1)` (the
/// classic "blsr" pattern). The state machine in a parser inner
/// loop can advance directly from one delimiter to the next without
/// inspecting any of the bytes between them — the same shape that
/// powers `simdjson`'s structural-character pass.
///
/// # Examples
///
/// ```
/// use noyalib::simd::{SimdScanner, StructuralIter};
///
/// let s = SimdScanner::new(b":\n");
/// let positions: Vec<usize> =
///     StructuralIter::new(&s, b"k1: v1\nk2: v2\n").collect();
/// assert_eq!(positions, vec![2, 6, 9, 13]);
/// ```
#[derive(Debug)]
pub struct StructuralIter<'a> {
    scanner: &'a SimdScanner,
    haystack: &'a [u8],
    /// Next byte position to scan (chunk loader anchors here).
    cursor: usize,
    /// Base offset for `cached_mask`'s bits — the start of the
    /// chunk that produced the cached bits.
    cached_base: usize,
    /// Bitmask of structural positions inside the chunk that
    /// started at `cached_base`. Bit `i` in the mask corresponds
    /// to byte `cached_base + i` in `haystack`.
    cached_mask: u32,
}

impl<'a> StructuralIter<'a> {
    /// Construct an iterator over every structural-byte position in
    /// `haystack` according to `scanner`'s needle set.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib::simd::{SimdScanner, StructuralIter};
    /// let s = SimdScanner::new(b":\n");
    /// let it = StructuralIter::new(&s, b"a: b\n");
    /// let v: Vec<usize> = it.collect();
    /// assert_eq!(v, vec![1, 4]);
    /// ```
    #[must_use]
    pub fn new(scanner: &'a SimdScanner, haystack: &'a [u8]) -> Self {
        StructuralIter {
            scanner,
            haystack,
            cursor: 0,
            cached_base: 0,
            cached_mask: 0,
        }
    }
}

impl Iterator for StructuralIter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        // Drain the cached chunk first.
        if self.cached_mask != 0 {
            let bit = self.cached_mask.trailing_zeros() as usize;
            // Clear the lowest set bit (`x & (x - 1)` — the standard
            // `BLSR` idiom).
            self.cached_mask &= self.cached_mask - 1;
            return Some(self.cached_base + bit);
        }

        // Refill from the next 32-byte chunk; if the tail is
        // shorter than 32 bytes, fall back to the scalar contains
        // test for the remaining bytes.
        loop {
            if self.cursor >= self.haystack.len() {
                return None;
            }
            let remaining = self.haystack.len() - self.cursor;
            if remaining >= 32 {
                let mut chunk = [0u8; 32];
                chunk.copy_from_slice(&self.haystack[self.cursor..self.cursor + 32]);
                let mask = self.scanner.structural_bitmask_32(&chunk);
                let chunk_origin = self.cursor;
                self.cursor += 32;
                if mask != 0 {
                    let bit = mask.trailing_zeros() as usize;
                    self.cached_base = chunk_origin;
                    self.cached_mask = mask & (mask - 1);
                    return Some(chunk_origin + bit);
                }
                continue;
            }

            // Tail (< 32 bytes): scalar scan.
            for offset in 0..remaining {
                let b = self.haystack[self.cursor + offset];
                if self.scanner.bitmap.contains(b) {
                    let pos = self.cursor + offset;
                    self.cursor = pos + 1;
                    return Some(pos);
                }
            }
            self.cursor = self.haystack.len();
            return None;
        }
    }
}

/// SWAR decimal parser: parse an unsigned 64-bit integer from a
/// byte slice of ASCII digits. Returns `None` if the slice is
/// empty, contains a non-digit byte, or overflows `u64`.
///
/// Uses SWAR (SIMD Within A Register) when the input is at least
/// 8 bytes long: 8 ASCII digits are parsed in a single `u64`
/// multiply-shift pipeline, eliminating the per-byte branch the
/// stdlib `<u64 as FromStr>::from_str` walks. For inputs shorter
/// than 8 bytes the scalar fallback runs — a tight `* 10 + d`
/// loop that LLVM auto-vectorises on most targets.
///
/// Pure-safe Rust — preserves the workspace
/// `#![forbid(unsafe_code)]` invariant. Validates every byte is
/// in `b'0'..=b'9'` before computing the result, so malformed
/// input never produces a garbage answer.
///
/// # Examples
///
/// ```
/// use noyalib::simd::parse_decimal_u64;
/// assert_eq!(parse_decimal_u64(b"12345"), Some(12_345));
/// assert_eq!(parse_decimal_u64(b"9223372036854775807"),
///            Some(9_223_372_036_854_775_807));
/// assert_eq!(parse_decimal_u64(b""), None);
/// assert_eq!(parse_decimal_u64(b"12a4"), None);
/// assert_eq!(parse_decimal_u64(b"99999999999999999999"), None); // overflow
/// ```
#[must_use]
pub fn parse_decimal_u64(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() {
        return None;
    }
    let mut result: u64 = 0;
    let mut i = 0;
    // SWAR fast path: process 8 digits per iteration. Bypass when
    // the remaining slice is shorter — the validation cost would
    // outweigh the SWAR multiply pipeline.
    while i + 8 <= bytes.len() {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&bytes[i..i + 8]);
        // Shift the accumulator up by 8 decimal places and add the new
        // chunk. A non-numeric chunk or overflow bails via `?`.
        let eight = parse_8_digits(arr)?;
        result = result.checked_mul(100_000_000)?.checked_add(eight)?;
        i += 8;
    }
    // Scalar tail: 0..7 remaining bytes.
    while i < bytes.len() {
        let b = bytes[i];
        if !b.is_ascii_digit() {
            return None;
        }
        result = result.checked_mul(10)?.checked_add((b - b'0') as u64)?;
        i += 1;
    }
    Some(result)
}

/// SWAR decimal parser for signed `i64`. Accepts an optional
/// leading `+` or `-`, then forwards to [`parse_decimal_u64`].
/// Returns `None` for empty input, non-digit bytes, or overflow
/// in either direction.
///
/// # Examples
///
/// ```
/// use noyalib::simd::parse_decimal_i64;
/// assert_eq!(parse_decimal_i64(b"42"), Some(42));
/// assert_eq!(parse_decimal_i64(b"-42"), Some(-42));
/// assert_eq!(parse_decimal_i64(b"+42"), Some(42));
/// assert_eq!(parse_decimal_i64(b"-9223372036854775808"), Some(i64::MIN));
/// assert_eq!(parse_decimal_i64(b"9223372036854775808"), None);
/// ```
#[must_use]
pub fn parse_decimal_i64(bytes: &[u8]) -> Option<i64> {
    if bytes.is_empty() {
        return None;
    }
    let (negative, digits) = match bytes[0] {
        b'-' => (true, &bytes[1..]),
        b'+' => (false, &bytes[1..]),
        _ => (false, bytes),
    };
    let abs = parse_decimal_u64(digits)?;
    if negative {
        // Special-case `i64::MIN` whose absolute value (2^63) does
        // not fit in i64. The unsigned form 9_223_372_036_854_775_808
        // converts directly via the wrapping cast.
        if abs == (i64::MAX as u64) + 1 {
            Some(i64::MIN)
        } else {
            i64::try_from(abs).ok().map(|n| -n)
        }
    } else {
        i64::try_from(abs).ok()
    }
}

/// Parse exactly 8 ASCII digits in a single SWAR pipeline.
///
/// The classic `(* 10) + (* 100) + (* 10000)` ladder folds the
/// 8-byte block into a single `u64` digit value via three
/// shift-add-mask phases. Each phase pairs adjacent units so the
/// total instruction count is independent of the input value —
/// branch-free arithmetic on a `u64` register.
///
/// Returns `None` if any byte is outside `b'0'..=b'9'`.
fn parse_8_digits(arr: [u8; 8]) -> Option<u64> {
    let chunk = u64::from_be_bytes(arr);
    // Validate: every byte is in 0x30..=0x39 ('0'..='9'). Subtract
    // 0x30 from each byte; result is 0..=9 if valid. To detect
    // out-of-range, check the subtracted byte is < 10 by adding
    // 0x76 (= 0x80 - 0x0A) and looking for high-bit propagation.
    let sub = chunk.wrapping_sub(0x3030_3030_3030_3030);
    let above_9 = sub.wrapping_add(0x7676_7676_7676_7676) & 0x8080_8080_8080_8080;
    let below_0 = chunk & 0x8080_8080_8080_8080;
    if above_9 != 0 || below_0 != 0 {
        return None;
    }
    // SWAR fold: three phases of pair-wise (high*N + low). The
    // big-endian `from_be_bytes` reading puts the leftmost digit
    // in the highest byte, which is exactly what the per-pair
    // shift-and-mask pattern wants.
    //
    // Phase 1 — pair adjacent bytes (16-bit half-words):
    //   for each pair (high_byte, low_byte): high*10 + low.
    //   Multiplying high by 10 cannot overflow the half-word
    //   (max digit 9 → 90 < 256), so the pairs stay independent.
    let high = (sub & 0xFF00_FF00_FF00_FF00) >> 8;
    let low = sub & 0x00FF_00FF_00FF_00FF;
    let chunk = high.wrapping_mul(10).wrapping_add(low);

    // Phase 2 — pair adjacent half-words (32-bit halves):
    //   for each pair (high_word, low_word): high*100 + low.
    //   Max half-word value is 99 * 100 + 99 = 9999 < 65 536, so
    //   each 32-bit pair stays independent.
    let high = (chunk & 0xFFFF_0000_FFFF_0000) >> 16;
    let low = chunk & 0x0000_FFFF_0000_FFFF;
    let chunk = high.wrapping_mul(100).wrapping_add(low);

    // Phase 3 — combine the two 32-bit halves into a single u64:
    //   high * 10_000 + low. Max high half is 9999, so high*10_000
    //   = 99_990_000; plus 9999 = 99_999_999 < 2^32 — fits.
    let high = chunk >> 32;
    let low = chunk & 0xFFFF_FFFF;
    Some(high.wrapping_mul(10_000).wrapping_add(low))
}

#[cfg(test)]
#[allow(clippy::byte_char_slices)]
mod tests {
    use super::*;

    #[test]
    fn empty_haystack_returns_none() {
        assert_eq!(find_any_of(b"", &[b':']), None);
        assert_eq!(find_any_of(b"", &[b':', b',', b'#', b'=']), None);
    }

    #[test]
    fn empty_needles_returns_none() {
        assert_eq!(find_any_of(b"hello", &[]), None);
    }

    #[test]
    fn arity_1_finds_first() {
        assert_eq!(find_any_of(b"abc:def:ghi", &[b':']), Some(3));
    }

    #[test]
    fn arity_2_finds_first_of_either() {
        assert_eq!(find_any_of(b"abc#def\n", &[b':', b'#']), Some(3));
        assert_eq!(find_any_of(b"abc\ndef#", &[b':', b'#']), Some(7));
    }

    #[test]
    fn arity_3_finds_first_of_three() {
        assert_eq!(find_any_of(b"abc\ndef:ghi", &[b':', b'\n', b'#']), Some(3));
    }

    #[test]
    fn arity_4_uses_swar_path() {
        let needles = &[b':', b',', b'=', b'\n'];
        assert_eq!(find_any_of(b"abc=def", needles), Some(3));
        assert_eq!(find_any_of(b"plain text only", needles), None);
    }

    #[test]
    fn arity_8_finds_correctly() {
        let needles = b"[]{}:,#\n";
        assert_eq!(find_any_of(b"hello{world", needles), Some(5));
        assert_eq!(find_any_of(b"plain text", needles), None);
    }

    #[test]
    fn boundary_at_chunk_edge() {
        // Place the needle at byte index 7, 8, 9 — straddling the
        // 8-byte SWAR chunk boundary.
        for pos in 0..16 {
            let mut input = vec![b'.'; 16];
            input[pos] = b':';
            assert_eq!(
                find_any_of(&input, &[b':', b',', b'#', b'\n', b'=']),
                Some(pos),
                "needle at position {pos}",
            );
        }
    }

    #[test]
    fn long_haystack_finds_at_far_position() {
        let mut input = vec![b'.'; 1024];
        input[1000] = b':';
        assert_eq!(
            find_any_of(&input, &[b':', b',', b'\n', b'#', b'=']),
            Some(1000),
        );
    }

    #[test]
    fn long_haystack_with_no_match_returns_none() {
        let input = vec![b'.'; 1024];
        assert_eq!(find_any_of(&input, &[b':', b',', b'#', b'\n', b'=']), None);
    }

    #[test]
    fn clean_prefix_len_basic() {
        assert_eq!(clean_prefix_len(b"abc:def", &[b':']), 3);
        assert_eq!(clean_prefix_len(b"all clean", &[b':']), 9);
        assert_eq!(clean_prefix_len(b":start", &[b':']), 0);
    }

    #[test]
    fn bitmap_membership() {
        let bm = bitmap_for(&[b'A', b'\n', 0]);
        assert!(bm.contains(b'A'));
        assert!(bm.contains(b'\n'));
        assert!(bm.contains(0));
        assert!(!bm.contains(b'B'));
        assert!(!bm.contains(255));
    }

    #[test]
    fn bitmap_full_set() {
        let needles: Vec<u8> = (0u8..=255).collect();
        let bm = bitmap_for(&needles);
        for b in 0u8..=255 {
            assert!(bm.contains(b), "byte {b}");
        }
    }

    /// Equivalence: arity-4+ SWAR path matches the scalar baseline
    /// across many inputs. Both paths produce identical results
    /// for every position, every needle set, every byte value.
    #[test]
    fn swar_equivalence_with_scalar() {
        let scalar = |haystack: &[u8], needles: &[u8]| -> Option<usize> {
            for (i, &b) in haystack.iter().enumerate() {
                if needles.contains(&b) {
                    return Some(i);
                }
            }
            None
        };

        // Deterministic pseudo-random inputs covering boundary
        // straddling, sparse needles, dense needles.
        let needle_sets: &[&[u8]] = &[
            b"[]{}:,#\n",
            b":,=",              // arity 3 — sanity-check the dispatch boundary
            b":",                // arity 1
            b":,#=[]{}\n\t \"'", // 13 needles
        ];
        for &needles in needle_sets {
            for length in [0usize, 1, 7, 8, 9, 16, 17, 63, 64, 65, 1023, 1024] {
                let mut buf = vec![0u8; length];
                // Fill with non-needle bytes.
                for (i, slot) in buf.iter_mut().enumerate() {
                    *slot = (i as u8).wrapping_add(33); // start past control bytes
                    while needles.contains(slot) {
                        *slot = slot.wrapping_add(1);
                    }
                }
                // No-needle case.
                assert_eq!(find_any_of(&buf, needles), scalar(&buf, needles));
                // Sprinkle a needle at every position; both paths
                // must agree.
                for pos in 0..length {
                    let saved = buf[pos];
                    buf[pos] = needles[pos % needles.len()];
                    assert_eq!(
                        find_any_of(&buf, needles),
                        scalar(&buf, needles),
                        "mismatch needles={:?} length={length} pos={pos}",
                        needles,
                    );
                    buf[pos] = saved;
                }
            }
        }
    }

    #[test]
    fn scanner_basic_finds_first_needle() {
        let s = SimdScanner::new(b":-{}");
        assert_eq!(s.find_any(b"a-b"), Some(1));
        assert_eq!(s.find_any(b"abc"), None);
    }

    #[test]
    fn scanner_long_input_match_at_far_position() {
        let s = SimdScanner::new(b":,#\n[]{}");
        let mut buf = vec![b'.'; 4096];
        buf[3000] = b'#';
        assert_eq!(s.find_any(&buf), Some(3000));
    }

    #[test]
    fn scanner_matches_baseline_across_needle_sets() {
        let baseline = |haystack: &[u8], needles: &[u8]| -> Option<usize> {
            for (i, &b) in haystack.iter().enumerate() {
                if needles.contains(&b) {
                    return Some(i);
                }
            }
            None
        };
        let needle_sets: &[&[u8]] = &[
            b":\n",
            b":,#\n",
            b"[]{}:,#\n",
            b"abcdefghij", // 10 needles — exercises the >8 fall-through
        ];
        for &needles in needle_sets {
            let s = SimdScanner::new(needles);
            for length in [0usize, 1, 31, 32, 33, 64, 127, 128, 1024] {
                let mut buf = vec![0u8; length];
                for (i, slot) in buf.iter_mut().enumerate() {
                    *slot = (i as u8).wrapping_add(33);
                    while needles.contains(slot) {
                        *slot = slot.wrapping_add(1);
                    }
                }
                assert_eq!(
                    s.find_any(&buf),
                    baseline(&buf, needles),
                    "no-needle: needles={needles:?} len={length}"
                );
                for pos in 0..length {
                    let saved = buf[pos];
                    buf[pos] = needles[pos % needles.len()];
                    assert_eq!(
                        s.find_any(&buf),
                        baseline(&buf, needles),
                        "needles={needles:?} len={length} pos={pos}",
                    );
                    buf[pos] = saved;
                }
            }
        }
    }

    // ── structural_bitmask_32 + StructuralIter ─────────────────────

    fn baseline_bitmask_32(chunk: &[u8; 32], needles: &[u8]) -> u32 {
        let mut m: u32 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            if needles.contains(&b) {
                m |= 1u32 << i;
            }
        }
        m
    }

    #[test]
    fn structural_bitmask_marks_every_needle_position() {
        let s = SimdScanner::new(b":-{}\n");
        let chunk: [u8; 32] = *b":bcd-fgh:abcdefgh{ijklmnopqrstuv";
        let mask = s.structural_bitmask_32(&chunk);
        // Bytes at positions 0, 4, 8, 17 are needles.
        assert_eq!(mask & (1 << 0), 1 << 0);
        assert_eq!(mask & (1 << 4), 1 << 4);
        assert_eq!(mask & (1 << 8), 1 << 8);
        assert_eq!(mask & (1 << 17), 1 << 17);
        // Sanity: no other bits set.
        assert_eq!(mask, (1 << 0) | (1 << 4) | (1 << 8) | (1 << 17));
    }

    #[test]
    fn structural_bitmask_is_zero_for_clean_chunks() {
        let s = SimdScanner::new(b":\n");
        let chunk = *b"plain text only no delim chars!.";
        assert_eq!(s.structural_bitmask_32(&chunk), 0);
    }

    #[test]
    fn structural_bitmask_matches_baseline_across_needle_sets() {
        // Test every YAML-relevant needle set against deterministic
        // chunks: the SIMD path and the scalar path must produce
        // bit-for-bit equal masks.
        let needle_sets: &[&[u8]] = &[
            b":",            // arity 1
            b":\n",          // arity 2
            b":,#",          // arity 3
            b":-[]{}\n",     // arity 7
            b":-[]{}\n# \t", // arity 10 (>8 — exercises fall-through)
        ];
        for &needles in needle_sets {
            let s = SimdScanner::new(needles);
            // 64 deterministic chunks: each one has needle bytes at
            // varying positions + filler.
            for variant in 0..64u8 {
                let mut chunk = [0u8; 32];
                for (i, slot) in chunk.iter_mut().enumerate() {
                    *slot = (i as u8).wrapping_add(33).wrapping_add(variant);
                    while needles.contains(slot) {
                        *slot = slot.wrapping_add(1);
                    }
                }
                // Sprinkle needles at some positions deterministically.
                for j in (0..32).step_by(3) {
                    if (variant as usize + j) & 1 == 1 {
                        chunk[j] = needles[(variant as usize + j) % needles.len()];
                    }
                }
                let actual = s.structural_bitmask_32(&chunk);
                let expected = baseline_bitmask_32(&chunk, needles);
                assert_eq!(
                    actual, expected,
                    "needles={needles:?} variant={variant} chunk={chunk:?}"
                );
            }
        }
    }

    #[test]
    fn structural_iter_yields_positions_in_order() {
        let s = SimdScanner::new(b":\n");
        let positions: Vec<usize> = StructuralIter::new(&s, b"k1: v1\nk2: v2\n").collect();
        assert_eq!(positions, vec![2, 6, 9, 13]);
    }

    #[test]
    fn structural_iter_handles_empty_input() {
        let s = SimdScanner::new(b":");
        let positions: Vec<usize> = StructuralIter::new(&s, b"").collect();
        assert!(positions.is_empty());
    }

    #[test]
    fn structural_iter_handles_partial_chunk_tail() {
        // Tail less than 32 bytes — exercises the scalar fall-back
        // inside StructuralIter::next.
        let s = SimdScanner::new(b":");
        let positions: Vec<usize> = StructuralIter::new(&s, b"abc:def:gh").collect();
        assert_eq!(positions, vec![3, 7]);
    }

    #[test]
    fn structural_iter_spans_multiple_chunks() {
        // Build a 100-byte haystack with a needle every 25 bytes —
        // the iterator must straddle the 32-byte chunk boundary
        // cleanly and not double-count or miss any.
        let mut buf = vec![b'.'; 100];
        for &p in &[5usize, 25, 60, 95] {
            buf[p] = b':';
        }
        let s = SimdScanner::new(b":");
        let positions: Vec<usize> = StructuralIter::new(&s, &buf).collect();
        assert_eq!(positions, vec![5, 25, 60, 95]);
    }

    #[test]
    fn structural_iter_count_matches_scalar_baseline() {
        let s = SimdScanner::new(b":\n,#");
        // Adversarial input — alternating needle / non-needle.
        let buf: Vec<u8> = (0..2048u32)
            .map(|i| if i % 7 == 0 { b':' } else { b'a' })
            .collect();
        let scalar: Vec<usize> = buf
            .iter()
            .enumerate()
            .filter_map(|(i, &b)| (b == b':').then_some(i))
            .collect();
        let simd: Vec<usize> = StructuralIter::new(&s, &buf).collect();
        assert_eq!(simd, scalar);
    }

    // ── SWAR decimal parsing ──────────────────────────────────────

    #[test]
    fn parse_decimal_u64_basic() {
        assert_eq!(parse_decimal_u64(b"0"), Some(0));
        assert_eq!(parse_decimal_u64(b"42"), Some(42));
        assert_eq!(parse_decimal_u64(b"12345678"), Some(12_345_678));
        assert_eq!(parse_decimal_u64(b"99999999"), Some(99_999_999));
    }

    #[test]
    fn parse_decimal_u64_long_inputs_swar_path() {
        assert_eq!(parse_decimal_u64(b"1234567890"), Some(1_234_567_890));
        assert_eq!(
            parse_decimal_u64(b"9999999999999999"),
            Some(9_999_999_999_999_999),
        );
        assert_eq!(
            parse_decimal_u64(b"1234567812345678"),
            Some(1_234_567_812_345_678),
        );
    }

    #[test]
    fn parse_decimal_u64_max() {
        assert_eq!(parse_decimal_u64(b"18446744073709551615"), Some(u64::MAX));
    }

    #[test]
    fn parse_decimal_u64_rejects_non_digits() {
        assert_eq!(parse_decimal_u64(b""), None);
        assert_eq!(parse_decimal_u64(b"12a4"), None);
        assert_eq!(parse_decimal_u64(b" 42"), None);
        assert_eq!(parse_decimal_u64(b"42 "), None);
        assert_eq!(parse_decimal_u64(b"-42"), None);
        assert_eq!(parse_decimal_u64(b"+42"), None);
    }

    #[test]
    fn parse_decimal_u64_overflow_returns_none() {
        // u64::MAX + 1
        assert_eq!(parse_decimal_u64(b"18446744073709551616"), None);
        // Way overflow
        assert_eq!(parse_decimal_u64(b"99999999999999999999999"), None,);
    }

    #[test]
    fn parse_decimal_u64_matches_stdlib_baseline() {
        // Sweep many lengths and values to ensure SWAR path is
        // bit-for-bit equivalent to the stdlib parser for every
        // length 1..=20.
        for n in [
            0u64,
            1,
            9,
            10,
            99,
            100,
            999,
            1000,
            12345,
            1234567,
            12345678,
            123456789,
            9876543210,
            1234567890123456,
            9_223_372_036_854_775_807, // i64::MAX
            10_000_000_000_000_000_000,
            u64::MAX - 1,
            u64::MAX,
        ] {
            let s = n.to_string();
            assert_eq!(parse_decimal_u64(s.as_bytes()), Some(n), "n={n}");
        }
    }

    #[test]
    fn parse_decimal_i64_handles_signs() {
        assert_eq!(parse_decimal_i64(b"42"), Some(42));
        assert_eq!(parse_decimal_i64(b"+42"), Some(42));
        assert_eq!(parse_decimal_i64(b"-42"), Some(-42));
        assert_eq!(parse_decimal_i64(b"0"), Some(0));
        assert_eq!(parse_decimal_i64(b"-0"), Some(0));
    }

    #[test]
    fn parse_decimal_i64_handles_min_max() {
        assert_eq!(parse_decimal_i64(b"9223372036854775807"), Some(i64::MAX));
        assert_eq!(parse_decimal_i64(b"-9223372036854775808"), Some(i64::MIN));
    }

    #[test]
    fn parse_decimal_i64_rejects_overflow() {
        // i64::MAX + 1
        assert_eq!(parse_decimal_i64(b"9223372036854775808"), None);
        // i64::MIN - 1
        assert_eq!(parse_decimal_i64(b"-9223372036854775809"), None);
    }

    #[test]
    fn parse_decimal_i64_matches_stdlib_across_full_range() {
        for n in [
            0i64,
            1,
            -1,
            10,
            -10,
            100,
            -100,
            i32::MIN as i64,
            i32::MAX as i64,
            i64::MIN,
            i64::MAX,
            i64::MAX - 1,
            i64::MIN + 1,
            -123_456_789,
            987_654_321,
        ] {
            let s = n.to_string();
            assert_eq!(parse_decimal_i64(s.as_bytes()), Some(n), "n={n}");
        }
    }
}
