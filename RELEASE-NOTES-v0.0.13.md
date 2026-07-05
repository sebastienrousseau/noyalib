<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.13 Release Notes

The **`noyalib-mcp` split** cut. Second satellite to leave the
monorepo under [ADR-0005](doc/adr/0005-workspace-split.md).
Follows the v0.0.12 pilot (which split `noyalib-wasm`);
mechanically identical playbook, extended to the
Model-Context-Protocol server's four publish channels.

No breaking API changes to library callers. No MSRV change
(still 1.85). No wire-format change to the MCP tool inventory
— `tools/list` returns the same set of `parse`, `format`,
`get`, `set`, `validate` tools with identical schemas.

Users pulling `noyalib`, `noyalib-lsp`, or `noya-cli` from
crates.io see nothing beyond a version bump. Users pulling
`noyalib-mcp` from crates.io or `@sebastienrousseau/noyalib-mcp`
from npm see the same version-0.0.13 artefact, just published
from
[`sebastienrousseau/noyalib-mcp`](https://github.com/sebastienrousseau/noyalib-mcp)
instead of this repo's `crates/noyalib-mcp/` subdirectory.

## Why this release exists

Every non-MCP user of the workspace was paying the full CI cost
of the MCP protocol-conformance run, the container publish
matrix, and the MCP-Registry integration. And every MCP-server
maintainer had to babysit changes in the library crate's
tests, benches, and no_std feature-matrix even when the MCP
wire format hadn't changed.

Splitting `noyalib-mcp` under strict lockstep gives us:

- **Independent MCP-specific CI matrices.** The
  `mcp-inspect.yml` protocol-conformance gate runs on every
  MCP-repo push; downstream MCP-integrator repos can watch a
  single, focused CI signal.
- **Independent GitHub-registry metrics.** Issues from AI-agent
  framework maintainers land on the MCP repo, keeping the
  library core's inbox focused on YAML-parser concerns.
- **Smaller library footprint.** `cargo install noya-cli` no
  longer transitively drags in MCP JSON-RPC framing code.
- **Independent supply-chain scope.** MCP consumers (typically
  AI-agent frameworks with sensitive supply-chain scanners)
  see only the MCP dep tree, not the full monorepo footprint.

Lockstep versioning (`=0.0.X` exact-match pins) keeps the
"one library, coordinated satellites" story: no user ever sees
a stale MCP paired with a fresh library.

## What changed in the monorepo

- `crates/noyalib-mcp/` deleted (history preserved on the
  satellite via `git subtree split`; all 14 pre-split commits
  carry their original authorship).
- Root `Cargo.toml` workspace member list drops
  `"crates/noyalib-mcp"`. Workspace now ships 3 publishable
  crates (`noyalib`, `noya-cli`, `noyalib-lsp`) plus internal
  `xtask`.
- Version-tag guard in `.github/workflows/release.yml` now
  cross-checks 2 in-workspace satellites (was 3). The
  `noyalib-mcp` cross-check moves to the satellite.
- Publish loop in `release.yml` drops the `noyalib-mcp`
  publish step. Order: `noyalib` → sleep 60s → `noya-cli`,
  `noyalib-lsp` in parallel.
- `release-binaries.yml` drops the `npm-publish-mcp` job (npm
  wrapper) and the `container-publish` matrix's `noyalib-mcp`
  row. Both move to the satellite.
- MCP-specific workflows removed:
  `.github/workflows/mcp-inspect.yml`,
  `.github/workflows/publish-mcp.yml`.
- Registry manifests removed from parent root and moved to
  satellite: `server.json` (MCP Registry entry),
  `glama.json` (Glama directory manifest).
- Distribution wrappers removed and moved to satellite:
  `pkg/npm-mcp-wrapper/`, `pkg/docker/Dockerfile.mcp`.
- Coverage `ignore-filename-regex` in `ci.yml`,
  `shared-coverage.yml`, and `scripts/coverage-gap-report.sh`
  no longer references `crates/noyalib-mcp/`.
- `REUSE.toml` — MCP-crate path entries removed.
- Docs: `README.md` ecosystem + per-crate links, `doc/USER-GUIDE.md`,
  `doc/ARCHITECTURE.md` reflect the split.
- `doc/adr/0005-workspace-split.md` — post-implementation
  update for the v0.0.13 pilot with concrete outcomes.

## What ships from the new satellite repo

[`sebastienrousseau/noyalib-mcp@v0.0.13`](https://github.com/sebastienrousseau/noyalib-mcp/releases/tag/v0.0.13)
publishes across **four channels**, each with its own
attestation:

1. **crates.io** — `noyalib-mcp@0.0.13` (SLSA L3 provenance +
   sigstore-signed `.crate`).
2. **npm** — `@sebastienrousseau/noyalib-mcp@0.0.13`
   (Trusted Publishing + npm `--provenance` attestation).
3. **GHCR** — `ghcr.io/sebastienrousseau/noyalib-mcp:0.0.13`
   (cosign keyless-signed multi-arch container + SLSA L3
   build-provenance attached to the image).
4. **MCP Registry** — `io.github.sebastienrousseau/noyalib-mcp`
   (OIDC-authenticated registration via `mcp-publisher`).

SBOM attached to the GitHub Release.

The satellite's CI is composed entirely of reusable workflows
from this repo, pinned by SHA. A hardening pass in this repo
propagates to the satellite within 48h via a Dependabot
SHA-bump PR — enforced by
`scripts/shared-workflow-propagation-monitor.sh`.

## Rollback

If v0.0.13 causes user-visible regressions in the MCP split,
the 9-step recipe in
[ADR-0005 §Rollback recipe](doc/adr/0005-workspace-split.md#rollback-recipe)
restores the pre-split monorepo shape in ≤5 minutes of cargo
compilation on a warm cache.

## Upgrading

- **Rust library users** (`noyalib`, `noyalib-lsp`,
  `noya-cli`): update `Cargo.toml` version to `0.0.13`. No API
  changes.
- **`noyalib-mcp` users on crates.io**: same as above. Update
  from `0.0.12` to `0.0.13`. Server still speaks the same
  JSON-RPC protocol with the same tool schemas.
- **`@sebastienrousseau/noyalib-mcp` npm users**: same. The
  wrapper moved repositories but not identity or invocation
  (`npx @sebastienrousseau/noyalib-mcp`).
- **`ghcr.io/sebastienrousseau/noyalib-mcp` container users**:
  no path change; the image still lives at
  `ghcr.io/sebastienrousseau/noyalib-mcp`. Just bump the tag.
- **MCP host configs** (Claude Desktop, Cursor, Continue.dev,
  Zed): no change needed — the server name
  (`io.github.sebastienrousseau/noyalib-mcp`) and the exposed
  tool inventory are unchanged.
- **Downstream repos vendoring the monorepo**: if you had a
  `path = "crates/noyalib-mcp"` reference, switch to crates.io
  (`noyalib-mcp = "=0.0.13"`) or `git =
  "https://github.com/sebastienrousseau/noyalib-mcp"`.

## Next up

- **v0.0.14** — split `noyalib-lsp` (issue #131).
- **v0.0.15** — split `noya-cli` (issue #132). After this,
  the monorepo is officially single-crate for the library
  proper.
- Post-split cleanup: monorepo `MIGRATION.md` for downstream
  vendors (#133), retire monorepo-only tooling now covered by
  per-repo shared workflows (#134).
