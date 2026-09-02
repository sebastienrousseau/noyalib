<!--
SPDX-FileCopyrightText: 2026 Noyalib
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# MSRV and deprecation policy

Written down because it was being applied from memory, and memory lost
twice in a single release cycle — see [Why this exists](#why-this-exists).

---

## Minimum Supported Rust Version

**Current MSRV: 1.86.0**, declared as `rust-version` in
`crates/noyalib/Cargo.toml` and enforced by the `msrv-core` and
`msrv-per-crate` CI jobs.

### What the MSRV covers

The **library, on its default features and on `--all-features`**. It does
not cover:

- development tooling (`cargo-vet`, `cargo-audit`, coverage);
- **dev-dependencies**, which may require a newer toolchain than the
  library does — see below, because this is the subtle one;
- the `nightly-simd` feature, which requires nightly by definition.

### Raising the MSRV

- A raise ships only on the **breaking-change axis** — during the
  `0.0.x` series that is the patch number (the same axis every
  breaking change uses; see the SemVer section of the crate docs),
  and from `0.x` onward the minor number. It is always called out in
  the changelog under its own heading and never happens silently in
  a compatible release. (Earlier wording said "minor version bump,
  never patch-level", which is unsatisfiable while the release
  number only moves in the patch position — the v0.0.16 raise from
  1.85 to 1.86 shipped exactly this way.)
- A raise needs a reason recorded in the changelog entry: which API, and
  why the alternative was rejected. "It was more convenient" is not a
  reason; "the alternative allocates on a hot path" is.
- Prefer writing the older form. `Option::map_or` over `Option::is_none_or`
  costs nothing and keeps the floor where it is.

### Dev-dependencies are in scope for CI, not for the contract

A dev-dependency whose own `rust-version` exceeds ours fails the MSRV job
even though no consumer of the library is affected. When that happens:

1. **Pin the dev-dependency back** to the last version under our floor,
   if one exists that is not yanked.
2. If no such version exists, that is a **decision**, not a mechanical
   fix. Raising the library's MSRV to accommodate a *test-only*
   dependency narrows what consumers can use, and should be made
   explicitly rather than to make a red job green.

---

## Deprecation

### Process

1. Mark with `#[deprecated(since = "x.y.z", note = "...")]`. The note
   must name the replacement, not just say the item is deprecated.
2. Keep it for **at least two minor releases** after the marking release.
3. Remove only in a release that documents the removal in the changelog
   under **Removed**.

### What is not deprecated

Behaviour changes that fix a defect are not deprecations — they are
fixes, and ship as such with the reasoning in the changelog. v0.0.19
changed bare `nan` / `inf` scalars from resolving as floats to staying
strings; that was a correctness fix to spec-conformance, documented, not
a deprecation cycle.

---

## Why this exists

Both of these were hit inside one release cycle, which is what turned an
implicit convention into a written one:

- **`Option::is_none_or`** was used in the v0.0.21 no_std work. It is
  stable from 1.82; the MSRV is 1.86 — so this one was fine — but the
  same pattern in a sibling crate with a 1.80 floor was caught only by
  clippy's `incompatible_msrv`. The lint is the backstop; writing the
  older form is the habit.
- **A dev-dependency raised its own MSRV five releases past a sibling
  crate's floor**, with its predecessor version yanked. There was no
  mechanical fix, only a choice between pinning to an older major and
  raising a user-facing promise to accommodate a test tool. Without a
  written policy that choice gets made by whoever is trying to get CI
  green.

## Enforcement

| Rule | Enforced by |
|---|---|
| Library builds on the declared MSRV | `msrv-core`, `msrv-per-crate` CI jobs |
| MSRV-incompatible API use | clippy `incompatible_msrv` |
| Deprecations carry a replacement | review |
| Removal after two minor releases | review, changelog |

The first two are mechanical and should stay that way. The last two are
judgement, and this document exists so the judgement is consistent.
