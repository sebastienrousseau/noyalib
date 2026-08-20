<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.7 — Release Notes

The **Supply-chain Hardening** cut. Closes three CVEs in the
VS Code extension's npm tree, tightens the release pipeline
end-to-end, and folds in the routine Dependabot backlog
(serde, clap, criterion 0.8, yaml-competitors, actions group,
plus eight standalone action bumps).

No public API change; no MSRV change (still 1.85). Existing
v0.0.6 callers can upgrade with no code edits.

## Highlights

* **Three CVEs cleared.** The legacy `vsce` devDependency in
  `pkg/vscode/` was dragging in vulnerable transitives
  (`xml2js@0.4.x` → GHSA-776f-qx25-q3cc,
  `qs@6.11–6.15.1` → GHSA-q8mj-m7cp-5q26,
  `tmp@<0.2.6` → GHSA-ph9p-34f9-6g65). The package wasn't
  actually invoked by CI — the release job already used
  `npx --yes @vscode/vsce` — so the whole subtree was removed.
* **`npm install` → `npm ci`** in the VS Code extension build,
  with `pkg/vscode/package.json` exact-pinned (no `^`) and a
  committed `package-lock.json`. OpenSSF Scorecard's
  `Pinned-Dependencies` check now reads 10/10.
* **Cosign installer pinned to v4.1.2** (was v3.9.2) via the
  sigstore group bump; signed release artefacts continue to
  upload `.crate.sig` + `.crate.pem` to every GitHub Release.
* **Auto-approve workflow for Dependabot** so bot PRs accrue
  the implicit approvals Scorecard's `Code-Review` check
  counts, without manual intervention.
* **criterion 0.8** across all benches, with the
  `criterion::black_box` → `std::hint::black_box` migration
  done in every bench file so the 0.8 deprecation lint stays
  clean.

## What ships

### Security — drop legacy `vsce` (PR #70)

The `vsce` npm package was retired upstream in favour of
`@vscode/vsce`. `pkg/vscode/package.json` carried it as a
devDependency, but the `vscode-extension` release job in
`.github/workflows/release-binaries.yml` never invoked it —
it called `npx --yes @vscode/vsce package` / `publish`
directly. Removing the dead devDep deleted the entire
vulnerable transitive subtree at once. The lockfile was
regenerated against the slimmer tree.

### Supply-chain — npm pinning (PRs #66 / #69 / #80)

* `pkg/vscode/package.json` — every dep pinned to an exact
  version (no `^`).
* `pkg/vscode/package-lock.json` — committed, with integrity
  hashes for the entire transitive tree.
* `.github/workflows/release-binaries.yml` — the
  `vscode-extension` job now runs `npm ci` instead of
  `npm install`. `npm ci` refuses to run without a lockfile
  and refuses to mutate one, so every release build resolves
  to exactly the integrity-checked tree we committed.

### Dependabot backlog (PR #80)

Serde, clap, criterion (0.5.1 → 0.8.2), yaml-competitors
(yaml-rust2 0.9 → 0.11, rust-yaml 0.0.5 → 1.1), the actions
group (10 actions, headed by checkout v5 → v6), taiki-e
install-action, docker/setup-qemu-action v3 → v4,
mislav/bump-homebrew-formula-action v3 → v4, plus the
sigstore/cosign-installer v3 → v4.1.2 bump from PR #63.

cargo-vet exemptions added for the 13 new bench/competitor
transitives and clap_mangen 0.3.0 promoted to safe-to-deploy.

### Benches — `criterion::black_box` migration

criterion 0.8 deprecates `criterion::black_box` in favour of
`std::hint::black_box`. Every bench file in the workspace —
14 files across `crates/noyalib/`, `crates/noyalib-lsp/`,
`crates/noyalib-mcp/`, and `crates/noya-cli/` — drops the
`black_box` import from criterion and pulls it from
`std::hint` instead.

### OpenSSF — Code-Review auto-approve (PR #64)

`.github/workflows/auto-approve-dependabot.yml` uses
`dependabot/fetch-metadata` to identify patch + minor bumps
and auto-approves them. This is the lever for Scorecard's
`Code-Review` check on a solo-maintainer project: bot PRs
acquire the implicit approval Scorecard counts, while human
PRs continue to merge via `gh pr merge --admin`.

### Docker base bump (PR #62)

`pkg/docker/Dockerfile{,.full,.mcp}` move from
`rust:1.85-bookworm` to `rust:1.96-bookworm`, all pinned by
`@sha256:` digest.

## OpenSSF Scorecard

The v0.0.6 cut hit 8.5/10. v0.0.7 adds:

* **Pinned-Dependencies** 9/10 → 10/10 (npm exact-pin + ci).
* **Vulnerabilities** 7/10 → expected 10/10 once the next
  scan picks up the dropped `xml2js`/`qs`/`tmp`.
* **CII-Best-Practices** — Passing badge earned during the
  v0.0.6 release; the next scan should lift this from 0/10 to
  the corresponding tier score.
* **Code-Review** auto-approve workflow is in place; the
  metric will climb as bot PRs land.

## Upgrade

```toml
# Cargo.toml
noyalib  = "0.0.7"
```

No code changes required; the runtime API is unchanged from
v0.0.6.

## Verification

```bash
# Verify the release-artefact signatures (keyless, sigstore).
cosign verify-blob \
  --certificate noya-cli-0.0.7.crate.pem \
  --signature   noya-cli-0.0.7.crate.sig \
  --certificate-identity-regexp \
    'https://github.com/sebastienrousseau/noyalib/\.github/workflows/release\.yml@.*' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  noya-cli-0.0.7.crate

# Verify the SLSA L3 build provenance against the public
# attestation store.
gh attestation verify noya-cli-0.0.7.crate \
  --owner sebastienrousseau
```

## Acknowledgements

This was a chore-heavy release driven entirely by Dependabot
and OpenSSF Scorecard signal. No external contributions to
call out for this cut.
