<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.3 — Release Notes

A surgical patch release that widens the `rustc-hash` dependency
cap so downstream crates pulling `rustc-hash 2.1+` (notably
`scraper`, `selectors`, and anything depending on them via
`html-generator`) can co-resolve cleanly with noyalib.

## What changed

- **`rustc-hash = ">=2, <2.1"` → `rustc-hash = ">=2, <3"`** in
  `crates/noyalib/Cargo.toml`. The original cap was defensive
  (the version we'd tested against at v0.0.2 release time),
  not load-bearing — noyalib's usage is the stable
  `FxHashMap` / `FxHashSet` / `FxBuildHasher` surface,
  unchanged across the 2.x line.

That's the entire behaviour change. No public API moved. No
private semantics moved. Nothing else.

## Why now

`html-generator`'s dep chain pulls
`scraper 0.26 → selectors 0.36 → rustc-hash ^2.1.1`. Under
v0.0.2's `<2.1` cap, a workspace pulling both `noyalib` and
`html-generator` failed to resolve. Widening to `<3` lifts the
block without any code or semantic change.

## MSRV preservation

`Cargo.lock` is pinned to `rustc-hash 2.0.0` because the 2.1+
manifest declares `rust-version = "1.77"`, above noyalib's
documented 1.75 MSRV floor. The same MSRV-preserving lockfile
pattern v0.0.2 used for `indexmap 2.10 / hashbrown 0.15`:

- Downstream consumers on Rust ≥ 1.77 are free to
  `cargo update -p rustc-hash` to take 2.1+.
- Consumers on Rust 1.75 inherit our lockfile pin via
  `cargo build --locked`.

## Lockstep version bumps

All five publishable crates bump to 0.0.3 in lockstep —
`noyalib`, `noya-cli`, `noyalib-mcp`, `noyalib-lsp`,
`noyalib-wasm`. The release workflow's lockstep guard requires
this. Path-dep `version` pins in the satellite manifests follow.

## Compatibility

- **Public API:** unchanged from v0.0.2.
- **Wire format:** unchanged.
- **MSRV:** Rust 1.75.0 stable (unchanged).
- **Drop-in upgrade:** bump the version pin in your `Cargo.toml`
  and `cargo update -p noyalib`. No code changes required.

## Headline numbers

Unchanged from v0.0.2: 100% strict YAML Test Suite (406/406),
zero `unsafe`, 4 000+ workspace tests + 495+ doctests.

## Verification

```bash
cargo install noya-cli --version 0.0.3
noyafmt --version    # noyafmt 0.0.3
noyavalidate --version

# Cosign-verify any release artefact
cosign verify-blob \
  --certificate "noyalib-0.0.3.crate.pem" \
  --signature   "noyalib-0.0.3.crate.sig" \
  --certificate-identity-regexp \
    "^https://github.com/sebastienrousseau/noyalib/" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  noyalib-0.0.3.crate
```
