<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.9 Release Notes

The **supply-chain refresh** cut. Batches eight open Dependabot PRs
into a single reviewable change, clears two RustSec advisories
(`anyhow` and `memmap2`), migrates `jsonschema` from 0.33 to 0.46 with
the corresponding `ValidationError` API change, and refreshes the
cargo-vet exemptions + `imports.lock` snapshot so the supply-chain
gates run green from a clean state.

No public API change. No MSRV change (still 1.85). No behavior change
for library users on the default feature set — this release is
entirely about the supply-chain surface and the internal `jsonschema`
call sites in `schema_validate.rs` and `cst/coerce.rs`.

## Highlights

* **Two RustSec advisories cleared.**
  * `anyhow` 1.0.102 → 1.0.103 closes `RUSTSEC-2026-0190`
    (`Error::downcast_mut` unsoundness — an interior-mutability edge
    case on the boxed error's downcast path).
  * `memmap2` 0.9.10 → 0.9.11 closes `RUSTSEC-2026-0186` (unchecked
    pointer offset in the mmap segment traversal).
  * Both are dev-graph deps for noyalib; neither ships in a release
    artefact, but downstream users would still see the advisories
    via their own `cargo audit`. The bumps take that noise away.
* **`jsonschema` 0.33 → 0.46.7.**
  `jsonschema::error::ValidationError::kind` and
  `ValidationError::instance_path` are now methods, not public
  fields. Updated all four call sites in `schema_validate.rs` and
  `cst/coerce.rs` from `err.kind` / `err.instance_path` to
  `err.kind()` / `err.instance_path()`. All 16 schema tests
  (`cargo test -p noyalib --features validate-schema schema`) pass
  under the new API.
* **Batched Dependabot backlog.** Eight open Dependabot PRs
  (#111, #112, #114, #115, #116, #119, #120, #121) landed in one
  reviewable bundle so the CHANGELOG tells one story instead of
  eight micro-stories.
  * GitHub Actions: `actions/checkout` 6.0.3 → 7.0.0, `actions/cache`
    5.0.5 → 6.1.0, `actions/attest-build-provenance` 4.1.0 → 4.1.1,
    `github/codeql-action/{init,analyze,upload-sarif}` SHA bump,
    `ossf/scorecard-action` SHA bump, `dtolnay/rust-toolchain` master
    SHA bump, `taiki-e/install-action` 2.81.8 → 2.82.2.
  * Cargo: `bytes` 1.11.1 → 1.12.0, `serde-saphyr` 0.0.27 → 0.0.28.
* **Supply-chain gates re-baselined.**
  * `deny.toml` gains a scoped `Zlib` license allowance for
    `foldhash` (transitive via `hashbrown 0.16 → referencing 0.46`),
    matching the existing MIT-0 / BSD-2-Clause posture. Zlib is
    OSI-approved, FSF-Free and permissive.
  * `supply-chain/config.toml` shrinks from 18 to 14 exemptions
    after `cargo vet regenerate exemptions` — the upstream audit
    imports (mozilla, google, bytecode-alliance, embark, fermyon,
    isrg, zcash) now cover 12 previously-local exemptions.
  * `supply-chain/imports.lock` refreshed via
    `cargo vet regenerate imports` so `--locked` accepts the current
    dep graph against the newest publisher records.
* **Workspace crates bumped to 0.0.9** (`noyalib`, `noya-cli`,
  `noyalib-mcp`, `noyalib-lsp`, `noyalib-wasm`) with intra-workspace
  `version =` pins synced. `xtask` stays at 0.0.1.

## Public API / behaviour

No change. `noyalib::schema_validate::coerce_to_schema`,
`noyalib::cst::coerce_to_schema`, and every other `validate-schema`
entry point behave identically to v0.0.8 — the API migration is
purely internal.

## Upgrade

`cargo update` picks up the bumps automatically. Downstream users
who explicitly pinned `jsonschema` in their own `Cargo.toml` should
bump their pin to `"0.46"` if they want their own graph to line up
with noyalib's; there is no runtime impact if they leave it at 0.33
because their `jsonschema` sits in a different fingerprint slot from
noyalib's own use.

Downstream users who rely on the transitive `bytes` 1.11 or
`serde-saphyr` 0.0.27 pins should retest under the new minors; both
are backwards compatible in practice.

## Verification

Every gate green: build across Ubuntu / macOS / Windows × stable /
nightly, MSRV 1.85 core, `no_std` (alloc-only), `cargo audit`
(zero advisories), `cargo deny check` (advisories / bans / licenses /
sources all ok), `cargo vet --locked` (269 fully audited, 14
exempted), Miri focused, Coverage gate ≥95%, CodeQL, differential
fuzz, cargo-machete, cargo-semver-checks, REUSE.software compliance,
signed-commit verification.

## Credits

Dependabot for the underlying advisory + version data. No external
contributor PRs were folded into this release (the three open
contributor PRs — `serde_core` migration by
[@EdJoPaTo](https://github.com/EdJoPaTo), lossless-u64 by
[@canardleteer](https://github.com/canardleteer), leading-BOM
scanner fix by [@zoosky](https://github.com/zoosky) — are staged
for dedicated later releases).
