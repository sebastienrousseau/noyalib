<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.17 Release Notes

A **lockstep-only** release. The `noyalib` core crate has **no code or
behaviour change** since v0.0.16 — `main` was byte-identical to the
v0.0.16 tag. It is republished at 0.0.17 only so the satellite crates,
which carry real fixes, can pin `=0.0.17` under the ADR-0005 strict-
lockstep contract.

Lockstep versioning: `noyalib` bumps `0.0.16` → `0.0.17`. Satellites
publish `=0.0.17` from their own repos.

## Why this release exists

The satellites accumulated fixes on `main` after the v0.0.16 tag that
could not reach users without a new lockstep version:

- **`noyalib-lsp` — `textDocument/formatting` was still a silent no-op.**
  `full_document_edits` used a byte-faithful CST round-trip
  (`parse_document().to_string()`), which equals its input for every
  parseable document, so the server always returned an empty edit list.
  (The v0.0.16 changelog claimed this was fixed; the code still
  round-tripped.) Fixed by calling `noyalib::cst::format`, with
  regression tests that assert a non-canonical document produces a real
  normalizing edit.
- **`noyalib-lsp` / `noya-cli` — crossbeam-epoch RUSTSEC-2026-0204.** The
  invalid-pointer-dereference advisory was present in their v0.0.16
  lockfiles via a transitive dependency; bumped to the patched 0.9.20.

## Repository hardening (all crates, CI/docs only)

No crate-content change, but every repository was brought to the same
CI/security bar:

- **Coverage, MSRV, CodeQL, and OpenSSF Scorecard gates** across all four
  satellites (the core already had them).
- **`noyalib-wasm`** gained a CI `wasm-test` job (`wasm-pack test
  --node`) gating its `#[wasm_bindgen]` surface; its test suite grew
  from 5 to 18 covering every export.
- **Upstream cargo-vet audit imports** added to the satellites, so most
  dependency bumps no longer churn the vet gate.
- **Branch protection tightened** so commit signing is unskippable
  (no bypass on the signature rule) while signed release pushes still
  work.

## What did not change

- The `noyalib` core public API, behaviour, and MSRV (1.86.0) — identical
  to v0.0.16.
- `#![forbid(unsafe_code)]` — intact across the workspace.

## Upgrading

```toml
[dependencies]
noyalib = "0.0.17"
```

No migration is required from v0.0.16 for the core crate. If you use
`noyalib-lsp`'s formatting or run `noyalib-lsp` / `noya-cli`, upgrade to
0.0.17 for the formatting fix and the security patch respectively.
