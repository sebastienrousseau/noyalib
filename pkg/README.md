<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `pkg/` — distribution packaging

Per-target packaging artefacts.

> **Status.** Only `docker/` is present in this repository today. The
> other rows below are the planned channels from Phase 4 of
> [`PLAN.md`](../PLAN.md); their directories and the
> `release-binaries.yml` workflow that would read them do not exist yet,
> so those names are deliberately not links. The container image is built
> and pushed by the `ghcr` job in
> [`release.yml`](../.github/workflows/release.yml).

| Directory | Channel | What ships | Where |
|---|---|---|---|
| `debian/` | Debian / Ubuntu source package | `noyalib` + `noyalib-dbgsym` `.deb`s | `dpkg-buildpackage` upstream; `cargo-deb` in CI for the GitHub Release |
| `rpm/` | Fedora / RHEL / openSUSE | `noyalib` + `noyalib-debuginfo` `.rpm`s | `rpmbuild`; `cargo-generate-rpm` in CI |
| `arch/` | Arch / AUR | `noyalib-bin` (binary), `noyalib` (source) PKGBUILDs | `aur-bump` job pushes to AUR |
| `homebrew/` | macOS / Linuxbrew | Formula | `homebrew-bump` job PRs to `sebastienrousseau/homebrew-tap` |
| `nix/` | NixOS / `nix run` | `flake.nix` + nixpkgs `package.nix` | Direct `nix run github:sebastienrousseau/noyalib` until the nixpkgs PR lands |
| `windows/wix/` | Windows MSI | `noyalib.wxs` | `cargo wix` on the windows-msvc release legs |
| `windows/scoop/` | Windows Scoop | `noyalib.json` | `scoop-bump` job pushes to `sebastienrousseau/scoop-bucket` |
| `snap/` | Snap Store | `snapcraft.yaml` | Future v0.1.x — scaffold only for now |
| `flatpak/` | Flathub | `io.noyalib.noyafmt.yaml` | Future v0.1.x — scaffold only for now |
| [`docker/`](docker/) | GHCR container images | `Dockerfile` (distroless), `Dockerfile.full`, `Dockerfile.mcp` | `container-publish` job, multi-arch via buildx |
| `vscode/` | VS Code Marketplace + Open VSX | `.vsix` extension that bundles `noyalib-lsp` | `vscode-extension` job |
| `npm-mcp-wrapper/` | npm | `noyalib-mcp` package — `npx`-runnable wrapper that bootstraps the binary from a GitHub Release on first run | `npm-publish` job |
| [`PUBLISH.md`](PUBLISH.md) | — | Per-channel runbook: bootstrap, secrets, first publish, ongoing maintenance for every distribution channel above | — |
| [`VERIFY.md`](VERIFY.md) | — | cosign + SLSA verification cookbook for every artefact above | — |

Every template carries `__VERSION__` / `__SHA256__` /
`__COMMIT__` placeholders that are rewritten by the per-channel
job in `release-binaries.yml`. The bytes inside the artefacts
themselves are bit-reproducible from source: same git tag →
same SHA256 across runs.

## Why split from `release.yml`?

The existing `release.yml` ships the crate to crates.io with
SLSA L3 attestation and sigstore signing. Splitting the binary-
distribution side into a second workflow keeps the crates.io
publish path minimal and well-tested while the multi-target
build matrix (14 platforms, deb / rpm / homebrew / arch /
container / vsix / npm) is iterated on. Once both workflows are
stable they can be merged or kept split per maintainer
preference.

## Verification

Every artefact is signed with cosign keyless and carries a
SLSA L3 build provenance attestation. See
[`VERIFY.md`](VERIFY.md) for the exact `cosign verify-blob` and
`gh attestation verify` invocations.
