<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.12 Release Notes

The **workspace-split pilot** cut. First of a four-release
sequence that moves the four satellite crates
(`noyalib-wasm`, `noyalib-mcp`, `noyalib-lsp`, `noya-cli`)
out of the monorepo into their own repositories, coordinated
under [ADR-0005](doc/adr/0005-workspace-split.md)'s
strict-lockstep versioning contract.

**v0.0.12 splits `noyalib-wasm`.** The other three follow at
v0.0.13, v0.0.14, v0.0.15 respectively.

No breaking public API changes for library callers. No MSRV
change (still 1.85). Users pulling
`noyalib`, `noyalib-mcp`, `noyalib-lsp`, or `noya-cli` from
crates.io see no behaviour change beyond a version bump.
Users pulling `noyalib-wasm` from crates.io / npm also see no
behaviour change — the same `=0.0.12` version and same
`@noyalib/noyalib-wasm` npm package continue to publish, just
from
[`sebastienrousseau/noyalib-wasm`](https://github.com/sebastienrousseau/noyalib-wasm)
now instead of this repo's `crates/noyalib-wasm/`
subdirectory.

## Why this release exists

Every non-library user of the workspace was paying the full
cost of every workspace change: bench churn touching the CLI
had to pass wasm-bindgen build gates; a wasm-pack toolchain
bump paged the LSP maintainer. The library core is the
category-defining artefact; the satellites are consumers of
it. Their release cadences, contributor pools, and hardening
priorities are all divergent.

Splitting each satellite to its own repository under strict
lockstep gives us:

- **Independent CI matrices.** `noyalib-wasm` runs on
  `ubuntu-latest × stable/nightly` end-to-end via
  `wasm-bindgen-test` under a headless browser; the parent
  library CI stays on `ubuntu-latest / macos-latest /
  windows-latest × stable/nightly` for native `cargo test`.
  Neither pays the other's build tax.
- **Independent GitHub metrics.** Stars, watchers, issues, and
  Insights signals stay attached to the crate whose users care
  about them. A JavaScript developer opening an issue on
  `noyalib-wasm` no longer surfaces on the noyalib library
  repo's dashboards.
- **Smaller library footprint.** Users who `cargo add noyalib`
  or `cargo install noya-cli` no longer transitively see the
  wasm-bindgen build tree. The library crate stays lean.

Lockstep versioning (`=0.0.X` exact-match pins) keeps the
"one library, coordinated satellites" story: no user ever
sees a stale satellite paired with a fresh library, and no
satellite maintainer can silently drift the API contract
they're built against.

## What changed in the monorepo

- `crates/noyalib-wasm/` deleted (history preserved on the
  satellite repo via `git subtree split`; all 11 pre-split
  commits carry their original authorship).
- Root `Cargo.toml` workspace member list dropped
  `"crates/noyalib-wasm"`. Workspace still ships 4 publishable
  crates (`noyalib`, `noya-cli`, `noyalib-mcp`, `noyalib-lsp`)
  plus the internal `xtask`.
- Version-tag guard in `.github/workflows/release.yml` now
  cross-checks 3 in-workspace satellites (was 4). The
  `noyalib-wasm` version cross-check moves to the satellite
  repo's release workflow.
- Publish loop in `release.yml` drops the `noyalib-wasm`
  publish step. Order: `noyalib` → sleep 60s for sparse-index
  settle → `noya-cli`, `noyalib-mcp`, `noyalib-lsp` in
  parallel.
- `release-binaries.yml` drops the `npm-publish` job for
  `@noyalib/noyalib-wasm`. That npm publish moves to the
  satellite repo's release workflow, still using Trusted
  Publishing + npm provenance.
- Coverage `ignore-filename-regex` in `.github/workflows/ci.yml`
  and `shared-coverage.yml` no longer references the removed
  `crates/noyalib-wasm/src/lib.rs` path.
- README ecosystem table + per-crate READMEs section link to
  the satellite repo instead of an in-tree path. The
  bundling-guide row now links to the satellite's
  `doc/bundling.md`.
- `doc/ARCHITECTURE.md` and `doc/USER-GUIDE.md` reflect the
  split. Coverage-exclusion callout is now historical.
- `scripts/coverage-gap-report.sh` regex trimmed to match.
- `doc/adr/0005-workspace-split.md`:
  - **Permissions gotcha table** added in
    "Shared reusable workflows" — v0.0.13/14/15 satellites
    must union `pull-requests: read` into their `ci.yml`
    caller `permissions:` block or their first CI run will
    startup_failure with 0 jobs.
  - **Post-implementation update — v0.0.12 pilot** section
    records the concrete outcomes (subtree extraction, strict
    lockstep enforced, ruleset applied, permissions trap
    documented).

## What ships from the new satellite repo

[`sebastienrousseau/noyalib-wasm@v0.0.12`](https://github.com/sebastienrousseau/noyalib-wasm/releases/tag/v0.0.12)
publishes:

- The `noyalib-wasm` crate on crates.io (same name, same
  version as the workspace).
- The `@noyalib/noyalib-wasm` npm package on npmjs.org (same
  name, same version).
- SLSA L3 build provenance attestation (via
  `actions/attest-build-provenance`).
- Keyless sigstore signatures (Fulcio + Rekor) on both the
  `.crate` artefact and the npm bundle.
- SBOM attached to the GitHub Release.

The satellite's CI is composed entirely of reusable workflows
from this repo, pinned by SHA. A hardening pass in this repo
propagates to the satellite within 48h via a Dependabot
SHA-bump PR — enforced by
`scripts/shared-workflow-propagation-monitor.sh`.

## Rollback

If v0.0.12 causes user-visible regressions in the split, the
9-step recipe in
[ADR-0005 §Rollback recipe](doc/adr/0005-workspace-split.md#rollback-recipe)
restores the pre-split monorepo shape in **≤5 minutes** of
cargo compilation on a warm cache (measured 1min 42s in the
2026-07-02 pre-flight dry-run). Yanked satellite versions on
crates.io let the monorepo re-take precedence transparently
for downstream users.

## Upgrading

- **Rust library users** (`noyalib`, `noyalib-mcp`,
  `noyalib-lsp`, `noya-cli`): update `Cargo.toml` version to
  `0.0.12`. No API changes.
- **`noyalib-wasm` users on crates.io**: same as above.
  Update from `0.0.11` to `0.0.12`.
- **`@noyalib/noyalib-wasm` npm users**: same. The npm
  publish moved repositories but not identity.
- **Downstream repos vendoring the monorepo**: if you had a
  `path = "crates/noyalib-wasm"` reference, switch to
  crates.io (`noyalib-wasm = "=0.0.12"`) or `git =
  "https://github.com/sebastienrousseau/noyalib-wasm"`.

## Next up

- **v0.0.13** — split `noyalib-mcp` (issue #130).
- **v0.0.14** — split `noyalib-lsp` (issue #131).
- **v0.0.15** — split `noya-cli` (issue #132). After this,
  the monorepo is officially single-crate for the library
  proper.
- Post-split cleanup: monorepo `MIGRATION.md` for downstream
  vendors (#133), retire monorepo-only tooling now covered by
  per-repo shared workflows (#134).
