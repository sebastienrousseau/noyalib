<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Packaging noyalib for a distribution

Written for downstream maintainers (Debian, Fedora, Arch, Nix,
Homebrew). Everything here is CI-enforced upstream, so it stays true
between releases.

## Licensing

Dual `MIT OR Apache-2.0`, REUSE 3.3 compliant: every file carries
machine-readable licensing (inline SPDX headers or `REUSE.toml`
annotations), lintable with `reuse lint`. The vendored YAML test
suite under `crates/noyalib/tests/yaml-test-suite/` is MIT,
copyright yaml-test-suite contributors.

## Toolchain floor

MSRV is declared as `rust-version` in each crate's `Cargo.toml` and
CI-enforced (currently **1.86.0**, identical across the family).
Raises happen only on the breaking-change axis with the reason
recorded in the CHANGELOG — policy and history in
[`MSRV-AND-DEPRECATION.md`](MSRV-AND-DEPRECATION.md). Dev-dependencies
may need a newer toolchain than the library; the contract covers the
library on default and all-features.

## The version pin model

Six crates release in lockstep at the identical `=0.0.X`: `noyalib`
(the library), `noya-cli` (the `noyafmt`/`noyavalidate` binaries),
`noyalib-lsp`, `noyalib-mcp`, `noyalib-wasm`, and
`noyalib-serde-yaml` (a package-rename drop-in for archived
`serde_yaml`). Satellites pin the core **exactly**; packaging
`noya-cli 0.0.X` requires `noyalib 0.0.X`. The pin is the
compatibility contract — do not relax it.

## Building and testing offline

```sh
make vendor          # cargo vendor the full dependency tree
cargo build --locked --offline
cargo test --locked --offline
```

Lockfiles are committed and CI builds `--locked`; a resolver run is
never required. The official YAML test suite is vendored, so the
full compliance suite (406 cases) runs hermetically.

## Binaries, manpages, completions (noya-cli)

`make install` in the noya-cli repository honors `PREFIX` and
`DESTDIR` and installs binaries, roff manpages, and bash/zsh/fish
completions to FHS paths. The committed manpages and completions are
regenerated from the clap definitions (`make assets`) and CI fails
if they drift; `[package.metadata.deb]` and
`[package.metadata.generate-rpm]` in its `Cargo.toml` describe the
same layout for `cargo deb` / `cargo generate-rpm`. Release binaries
are built with `cargo auditable`, so `cargo audit bin` can read the
embedded dependency list.

## Verifying upstream artefacts

Tags and commits are signed (`KEYS.asc` at the repo root); release
artefacts carry sigstore bundles, SHA256/SHA512 checksum files, a
CycloneDX SBOM, and SLSA build-provenance attestations
(`gh attestation verify --owner sebastienrousseau <artefact>`). The
full cookbook is [`pkg/VERIFY.md`](../pkg/VERIFY.md).

## Questions

Open an issue with the `packaging` label, or see
[`SUPPORT.md`](../SUPPORT.md). Security contacts are in
[`SECURITY.md`](../SECURITY.md) — never a public issue for a
vulnerability.
