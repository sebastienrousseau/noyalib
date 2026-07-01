<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.11 Release Notes

The **CI integrity** cut. A focused housekeeping release that closes
three latent gaps the v0.0.10 audit surfaced — every one of which
was hidden by cache poisoning in the CI gates. Every gate that
reports green in this release actually exercises what it claims to.

No public API change. No MSRV change (still 1.85). One user-visible
behavior change under `--no-default-features`: the `no_std` build
now actually compiles from a clean state (previously broken since
v0.0.9, silently masked by CI cache poisoning).

## Why this release exists

While auditing v0.0.10's coverage before opening the next feature
release, three CI-level failures surfaced that had been silently
red on `main` for the entire v0.0.9 → v0.0.10 window:

1. `cargo check -p noyalib --no-default-features --lib --locked`
   from a clean target dir produced 8 errors. CI's `no_std
   (alloc-only) build` job had been finishing in 1.89s with a
   SUCCESS verdict — because every other CI job runs with `std`
   on, populates the default `target/` dir, and
   `Swatinem/rust-cache` then serves the no_std `cargo check` a
   matching-fingerprint *cache hit* without actually exercising
   the no_std code path. The check was cache-poisoned; the code
   was actually broken.
2. The `Documentation` workflow (strict `-D warnings` rustdoc,
   deploys to GitHub Pages) had failed every push to `main` since
   2026-06-30 because of three broken intra-doc links in
   `de/config.rs`. PR-time CI didn't catch them because no
   PR-gated strict-rustdoc check existed.
3. OSSF Scorecard was flagging the repository for
   `RUSTSEC-2026-0173` (`proc-macro-error2` unmaintained), which
   reaches noyalib only at build time via the opt-in `validator`
   feature and never ships in a release artefact — accepted risk,
   but not documented in a way Scorecard would honour.

This release fixes all three, hardens the gates so they can't
regress the same way, and closes the Code Scanning alert.

## Highlights

* **`no_std` build actually compiles.**
  `crates/noyalib/src/doc_boundary.rs` used `Vec` and `vec![]`
  without importing them from `crate::prelude`. Under `std` the
  prelude auto-imports both; under `no_std` they're only reachable
  via `alloc::vec::Vec` / `alloc::vec` or the project's
  `crate::prelude` re-export. Added `use crate::prelude::{Vec, vec};`.
  Additionally, `use crate::span_context;` at module scope in
  `de.rs` fired `-D unused-imports` under `--no-default-features`
  because both call sites are inside `#[cfg(feature = "std")]`
  blocks; gated the import behind `#[cfg(feature = "std")]` to
  match. `cargo check -p noyalib --no-default-features --lib
  --locked` now finishes in ~1.14s from a clean target dir with
  no errors.

* **Three broken intra-doc links fixed.** `de/config.rs` referenced
  `[Value]` at lines 179, 191, and 348 and `[Error::Custom]` at
  line 213 without qualifying them; the module only imports
  `crate::prelude::*` and neither item is in the prelude, so
  rustdoc could not resolve the links. Qualified all four to
  `[Value](crate::Value)` and `[Error::Custom](crate::Error::Custom)`.
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
  --all-features` now runs clean.

* **CI cache-poisoning guard applied across every specialised
  cargo job.** Every job that runs cargo with a non-default
  feature or toolchain configuration now uses an isolated
  `CARGO_TARGET_DIR` plus a scoped `Swatinem/rust-cache`
  namespace, so its fingerprint can never be served by another
  configuration's cache:

  * `ci.yml`: `no_std` → `target-no-std`,
    `MSRV (1.85.0) core build` → three sub-dirs
    (`target-msrv-nostd`, `target-msrv-default`, `target-msrv-clippy`),
    `Miri focused` → `target-miri`, `Miri full` → `target-miri-full`,
    `Coverage gate` → `target-coverage`, `Differential fuzz` →
    `target-fuzz`, `rustdoc (strict)` → `target-docs-strict`.
  * `security.yml`: `Soak fuzz` → `target-soak-fuzz-<matrix.target>`,
    `Soak Miri` → `target-soak-miri`.
  * `docs.yml`: `Build Documentation` → `target-docs-pages`
    (Pages upload path bumped from `target/doc` to
    `target-docs-pages/doc` to match).
  * `release.yml`: `Validate` → `target-release-validate`,
    `Cross-verify` → `target-release-verify-<matrix.os>`.

  Deliberately not isolated: the CI Test matrix and
  `vendor + offline` (both run `--all-features` — the maximal
  config, so cache poisoning would trigger a real rebuild
  everywhere), and `release-binaries.yml` (per-target
  cross-compilation which cargo already isolates via
  `target/<triple>/`).

* **New `docs-strict` PR-gated job.** Mirrors the exact
  `RUSTDOCFLAGS` (`-D warnings` +
  `rustdoc::broken_intra_doc_links` +
  `rustdoc::private_intra_doc_links` +
  `rustdoc::invalid_codeblock_attributes` +
  `rustdoc::invalid_html_tags` +
  `rustdoc::bare_urls`) that `docs.yml` uses on `main`, but on
  every PR. Broken doc links now fail a PR instead of the
  Pages deployment after merge.

* **Code Scanning alert #36 closed.** OSSF Scorecard's
  Vulnerabilities check was flagging `RUSTSEC-2026-0173`
  (`proc-macro-error2` unmaintained). A source-controlled
  `osv-scanner.toml` at the repo root now documents the ignore
  with matching rationale, mirroring the pre-existing
  `deny.toml` `[advisories.ignore]` entry so a single grep for
  the RUSTSEC ID surfaces every place the acceptance is
  documented. The alert has been dismissed with `won't fix`
  reason.

  Rationale for accepting the risk: `proc-macro-error2` reaches
  noyalib only at build time via `validator_derive → validator`,
  which is behind the OPT-IN `validator` cargo feature (not in
  the default set); it never ships in a release artefact. No
  maintained drop-in exists yet as of 2026-07-01 (`validator`
  0.20, released 2025-01-20, still depends on
  `proc-macro-error2`). Revisit when `validator` cuts a release
  that drops the unmaintained dep.

## Public API / behaviour

No API change. Downstream users on the default feature set see
identical behaviour to v0.0.10; downstream users on
`--no-default-features` see the previously-broken `no_std` build
succeed for the first time since v0.0.9.

## Upgrade

`cargo update` picks up the version bump automatically; no code
change required.

## Verification

Locally on the `feat/v0.0.11` branch before merge:

* `cargo build --workspace --all-features` — clean
* `cargo test --workspace --all-features` — all tests pass
* `cargo test --doc -p noyalib --all-features` — 528 doctests pass
* `cargo check -p noyalib --no-default-features --lib --locked`
  (from a fresh target dir) — clean, 1.14s
* `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
  --all-features` — clean across every workspace crate
* `cargo +1.85.0 check -p noyalib --lib --locked` — MSRV unchanged
* `cargo bench -p noyalib --no-run --all-features` — all 8 bench
  binaries compile
* `cargo build --examples -p noyalib --all-features` — all 50+
  examples build
* `cargo audit` — zero advisories
* `cargo deny check` — advisories / bans / licenses / sources all ok
* `cargo vet --locked` — 269 fully audited, 14 exempted

CI 100% green on PR #124: 44 SUCCESS / 0 FAILURE / 12 SKIPPED
(the SKIPs are schedule-only or event-gated jobs).

## Credits

Housekeeping release with no external contributor changes. Three
open contributor PRs (@zoosky's leading-BOM fix landed as v0.0.10;
@EdJoPaTo's `serde_core` migration and @canardleteer's
`lossless-u64` variant staged for their own dedicated later
releases).
