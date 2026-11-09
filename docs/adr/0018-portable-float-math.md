<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# 0018. Keep portable float math independent of runtime dependencies

- **Status:** accepted
- **Date:** 2026-11-09
- **Authors:** Noyalib contributors

## Context

Issue [#500](https://github.com/sebastienrousseau/noyalib/issues/500) identifies
an unused math dependency in hosted consumers. The std prelude already calls
inherent float methods; only the alloc-only prelude calls `libm::trunc` and
`libm::fma`. Making libm optional through a new feature would require existing
`--no-default-features` consumers to change their commands. That conflicts
with the required bare-metal compatibility.

The operations are private and limited to fractional-part checks and fused
multiply-add for YAML 1.1 sexagesimal floats. Separately rounding multiplication
and addition changes parse results. The stable Rust 1.89 floor and the
zero-unsafe policy also rule out using experimental core intrinsics.

## Decision

Keep std builds on their inherent methods. Implement private binary64 helpers
using `core` operations for alloc-only builds and retain libm as a development
oracle. Preserve all existing feature definitions, public signatures, and
consumer commands.

Fractional parts subtract a truncation formed by clearing the fraction bits.
This preserves subnormals, signed zeros, and nonfinite behavior without float
to integer casts.

The fused operation decomposes each finite operand into an integer mantissa
and a power of two. Its product fits exactly in 106 bits. Align the product
and addend at bit 126 of a `u128`, leaving a carry bit. When magnitudes are close
enough to cancel, both operands fit exactly. For distant terms, retain a sticky
bit recording any discarded nonzero bits. The larger term has trailing zeros,
so adding or subtracting the sticky term preserves which side of a rounding
boundary contains the exact result.

Round the combined integer once to nearest with ties to even. Clamp the output
quantum to the subnormal grid before rounding, avoiding double rounding at
underflow. Handle overflow, exact cancellation, signed zero, infinities and NaNs
explicitly. No floating environment API or unsafe operation is used.

Enforce active dependency graphs and public float behavior from isolated default,
minimal std, and alloc-only consumers. Compare the private helpers with std and
libm on boundary cases, cancellation-biased inputs, and deterministic bit patterns.
Include the helpers in focused Miri, including the existing big-endian profile.

## Consequences

- Hosted consumers can avoid the runtime dependency while existing bare-metal
  builds keep float support without a new feature.
- The project owns a small numerical kernel and its correctness tests. Changes
  must retain fused rounding, subnormal behavior and both reference comparisons.
- Std builds retain their existing execution path. The portable operation uses
  integer arithmetic and does not require a hardware fused instruction.
- NaN payloads remain unspecified, matching Rust's float contract; numerical
  comparisons check the NaN class and exact bits for every other result.

## Alternatives considered

An optional libm feature requires changing existing alloc-only consumers.
Target-specific dependencies cannot distinguish std and no_std consumers on
the same target. A replacement unconditional math crate retains an unnecessary
dependency in hosted builds. Ordinary multiplication followed by addition
changes rounding. Experimental core math or an MSRV bump violates the current
toolchain contract.

## References

- [Rust fused multiply-add contract](https://doc.rust-lang.org/std/primitive.f64.html#method.mul_add)
- [Cargo feature unification](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification)
- [Zero-unsafe policy](0003-zero-unsafe-policy.md)
