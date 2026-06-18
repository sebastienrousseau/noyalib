<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib production-readiness plan

This document is the long-form roadmap for shipping noyalib as a
distro-grade Rust library and CLI tool: ≥ 98 % unit-test, doc,
example, and benchmark coverage on every package; a `ripgrep`-shaped
repo layout; and a multi-platform release pipeline that produces
signed artefacts ready for crates.io, npm, the VS Code Marketplace,
Linux distros (deb / rpm / Arch / Nix), Homebrew, AUR, Scoop, and a
container registry.

The plan is written so any maintainer can pick up where the last
commit left off. Each phase is sized for a single self-contained
PR; the order respects dependencies (gates first, restructure
early, distro outreach last).

## Working invariants

These rules apply to every change inside this plan. They are
binding even when not restated in a section.

- **CI must always be green.** Every push verifies the workflow
  status; any red job is fixed in the same session before
  declaring done. Never bypass with `--no-verify`, `[skip ci]`,
  or `if: false`.
- **Conventional Commits** with signed (`-S`) commits.
  `Assisted-by:` trailer is appended automatically by the
  `commit-msg` hook; the branding signature is appended manually
  per `~/.claude/CLAUDE.md`.
- **`#![forbid(unsafe_code)]`** at every crate root in the
  workspace. No exceptions.
- **No fabrication.** API names, flag names, paths, version
  numbers, and CI matrix entries are checked against the source
  before committing.
- **Single zero-logic commit for the restructure (Phase 1).** The
  Big Move changes only file paths and `Cargo.toml` `members =`
  entries. No comment edits, no formatting changes, no
  refactoring inside the moved files. This preserves
  `git blame --follow` accuracy for every line.

## Locked decisions

| Decision | Choice | Rationale |
|---|---|---|
| **Naming (E1)** | `crates/noya-cli/` package, binaries stay `noyafmt` + `noyavalidate` | Two distinct tools (formatter vs. CI-gate validator) following Unix philosophy; binary names already shipped in v0.0.1 examples and integrations, so no breaking change for downstream consumers. |
| **Asset generation (B)** | Hybrid `build.rs` + `xtask` | `NOYA_GEN_ASSETS=1 cargo build --release` produces man pages and completions for distro packagers; `cargo xtask` paths reuse the same generation code so docs and binaries never drift. |
| **Restructure cadence (Phase 1)** | One zero-logic commit, file-moves only | Preserves `git blame --follow`; surface area is large, code semantics are unchanged. |
| **Universal macOS bottle (R1)** | Yes — `lipo` job after the two macos legs | Homebrew bottle complexity drops; Intel-Rosetta users get the same artefact. |
| **Debug-symbol packaging (R2)** | Yes — `llvm-objcopy --only-keep-debug` per Linux gnu leg, ship `noyalib-<v>-<target>-debuginfo.tar.gz` and `-debuginfo` sub-packages | Enterprise Linux deployments need separate symbol packages for telemetry / crash unwinders. |
| **Vendoring (C)** | Yes — `make vendor` target, `vendor-build` CI job, documented in `pkg/VERIFY.md` | Air-gapped, FIPS-bound, and RHEL build chains require offline-buildable source trees. |
| **CodeQL (D)** | Yes — new job in `security.yml` | CWE coverage complements `cargo-deny` (license/advisory) and `cargo-vet` (audit chain). |

## Status snapshot (`cf2d877`)

| Axis | Current | Target | Gap |
|---|---|---|---|
| Workspace `unsafe_code` | `forbid` | `forbid` | — |
| YAML 1.2 conformance | 406 / 406 | 406 / 406 | — |
| Lib unit tests | 146 pass | ≥ 98 % line every package | needs gap fill |
| Integration tests | 213+ pass | — | — |
| Doc-tests | 337 pass | — | — |
| `cargo audit` / `deny` / `vet` | clean | clean | — |
| Workspace coverage gate | 95 % fn / 92 % region / 93 % line | 98 % everywhere | +3–6 pp |
| `noyalib-wasm` coverage | excluded | dedicated 90 % gate | needs first run |
| `noyalib-mcp` coverage | partial exclusion | dedicated 90 % gate | needs first run |
| `noyalib-lsp` coverage | partial exclusion | dedicated 90 % gate | needs first run |
| Rustdoc coverage | `missing_docs = warn` | `deny`, ≥ 98 % items | needs audit |
| Examples surface coverage | 56 examples | every public module | 5 modules uncovered |
| Benchmark coverage | 10 benches | every hot path | 4 paths uncovered |
| Release pipeline | crates.io + sigstore | crates.io + npm + VS Code + 14 binary targets + .deb + .rpm + .msi + Homebrew + AUR + container | major rewrite |
| Repo layout | flat | `crates/` + `pkg/` + `doc/` + `ci/` + `complete/` | restructure |

## Phase 0 — Pre-flight

| Gate | Status |
|---|---|
| CI fully green on `feat/v0.0.1` | ✓ at `cf2d877` |
| All 56 examples runnable | ✓ |
| Coverage gate passing | ✓ (95/93/92) |
| Miri focused suite | ✓ |
| `cargo-vet`, `cargo-deny`, REUSE | ✓ |
| Branch protection on `main` | verify before Phase 1 — required reviews + status checks |

**Effort: 5 min.** Branch protection check only.

## Phase 1 — Repo restructure (one zero-logic commit)

The structural change. No semantic edits — only path moves and
`Cargo.toml` members/path adjustments. Verified locally before
push; CI is the post-push sanity check.

### 1.1 Target tree (final shape, including all later-phase additions)

```
noyalib/
├── Cargo.toml                # workspace manifest only
├── Cargo.lock
├── README.md, CHANGELOG.md, PLAN.md, CONTRIBUTING.md, SECURITY.md, FAQ.md (new)
├── LICENSE-MIT, LICENSE-APACHE, COPYING (new), NOTICE (generated)
├── REUSE.toml
├── ci/                       # CI helper scripts (new)
│   ├── docker/
│   ├── build-deb
│   ├── install-cross
│   ├── ubuntu-install-packages
│   └── macos-install-packages
├── complete/                 # source shell completions (new)
├── crates/
│   ├── noyalib/              # library — moved from root
│   ├── noya-cli/             # noyafmt + noyavalidate binaries (extracted from src/bin)
│   ├── noyalib-lsp/          # already exists, moved under crates/
│   ├── noyalib-mcp/          # already exists, moved under crates/
│   ├── noyalib-wasm/         # already exists, moved under crates/
│   └── xtask/                # cargo xtask runner (new)
├── doc/                      # documentation (new)
│   ├── noyafmt.1, noyafmt.1.adoc
│   ├── noyavalidate.1, noyavalidate.1.adoc
│   ├── USER-GUIDE.md, MIGRATION-FROM-SERDE-YAML.md, ARCHITECTURE.md
│   └── design/               # docs/design/ moves here
├── pkg/                      # distribution metadata (new)
│   ├── README.md, VERIFY.md
│   ├── debian/, rpm/, arch/, nix/, windows/, snap/, flatpak/, docker/
│   ├── homebrew/noyalib.rb.template
│   ├── vscode/               # VS Code extension source
│   ├── npm-mcp-wrapper/      # npx wrapper for noyalib-mcp
│   └── completions/          # generated completions (gitignored except .gitkeep)
├── benches/                  # workspace-level Criterion benches
├── examples/                 # workspace-level examples
├── fuzz/                     # cargo-fuzz harnesses (kept at root)
├── scripts/
│   ├── miri.sh
│   ├── coverage-gap-report.sh
│   ├── release-checklist.sh
│   └── msrv-per-crate.sh     # Phase 7.5 dynamic MSRV check
├── supply-chain/             # cargo-vet, unchanged
└── .github/workflows/
    ├── ci.yml, release.yml, docs.yml, security.yml, scorecard.yml, codspeed.yml
    ├── release-binaries.yml  # multi-platform binary release (new)
    ├── nightly.yml           # daily snapshot builds (new)
    └── promotional.yml       # Homebrew/AUR/Scoop bumps after release (new)
```

### 1.2 Migration sequence (to be executed inside the zero-logic commit)

```bash
# 1. Move library to crates/noyalib/
mkdir -p crates/noyalib
git mv src tests examples benches crates/noyalib/

# 2. Move satellite crates
git mv noyalib-wasm noyalib-mcp noyalib-lsp crates/

# 3. Extract CLI binaries (src/bin moved with the library above; now relocate)
mkdir -p crates/noya-cli/src/bin
git mv crates/noyalib/src/bin/*.rs crates/noya-cli/src/bin/
rmdir crates/noyalib/src/bin

# 4. Move design docs
git mv docs/design doc/design

# 5. Edit workspace Cargo.toml + crate manifests (members, paths, [[example]] paths,
#    workspace inheritance via [workspace.package]/[workspace.dependencies])

# 6. Edit REUSE.toml (every noyalib-{wasm,mcp,lsp}/Cargo.toml → crates/noyalib-{...}/)

# 7. Edit every `working-directory:` in .github/workflows/

# 8. Verify
cargo build --workspace --all-features
cargo test  --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
reuse lint
cargo vet
cargo deny check
```

### 1.3 Workspace `Cargo.toml` shape

```toml
[workspace]
resolver = "2"
members = [
    "crates/noyalib",
    "crates/noya-cli",
    "crates/noyalib-lsp",
    "crates/noyalib-mcp",
    "crates/noyalib-wasm",
    "crates/xtask",
]
exclude = ["fuzz", "examples/wasm"]

[workspace.package]
edition      = "2021"
rust-version = "1.75.0"
authors      = ["Sebastien Rousseau <sebastian.rousseau@gmail.com>"]
license      = "MIT OR Apache-2.0"
repository   = "https://github.com/sebastienrousseau/noyalib"
homepage     = "https://github.com/sebastienrousseau/noyalib"

[workspace.dependencies]
# (every shared dep declared once, used via `<dep>.workspace = true` per crate)

[workspace.lints.rust]
unsafe_code     = "forbid"
unreachable_pub = "forbid"
missing_docs    = "deny"   # ratchet-up — see Phase 1.5

[profile.release]
codegen-units    = 1
lto              = true
opt-level        = "s"
panic            = "abort"
strip            = "symbols"

[profile.dist]
inherits = "release"
debug    = "limited"
strip    = false
```

### 1.4 Naming locked-in

- Library crate: `noyalib` (unchanged, public API surface preserved).
- CLI crate: `noya-cli` (new package name; binaries `noyafmt` and `noyavalidate` keep their names so `apt install noyalib && noyafmt …` continues to work).
- Satellites: `noyalib-lsp`, `noyalib-mcp`, `noyalib-wasm`.

### 1.5 Workspace-wide `missing_docs = "deny"`

All crates inherit via:

```toml
[lints]
workspace = true
```

Audit pass: ~30 public items currently lack doc comments; each gets a one-line `///`. CI's `cargo doc` step uses `RUSTDOCFLAGS="-D missing_docs"` so undocumented items fail the build.

**Phase 1 deliverables:** 1 commit, ~250 file moves, ~12 manifest edits, no behavioural change. **Effort: 1 day code + 1 review.**

**Risks & mitigations:**
- *`git blame --follow` confusion.* Single move-only commit preserves rename detection. `BLAME-MOVES.md` records the SHA for archaeologists.
- *External users bookmarking `src/...` URLs.* README links updated; redirects added in any old-path docs.

## Phase 2 — Tooling (build.rs + xtask hybrid)

**Goal:** Every distro packaging artefact (man pages, shell completions) produced reproducibly from source via either `cargo build` (for distros) or `cargo xtask` (for developers iterating).

### 2.1 `build.rs` in `crates/noya-cli/`

```rust
//! Generates man pages and shell completions during `cargo build` so
//! distro packagers (rpmbuild / dpkg-buildpackage) get the artefacts
//! without needing a separate `cargo xtask` invocation.
//!
//! Off by default — enabled by `NOYA_GEN_ASSETS=1`. Generation is
//! deterministic (no clock reads, no env reads beyond the trigger
//! var), so output is bit-reproducible.

fn main() {
    println!("cargo:rerun-if-env-changed=NOYA_GEN_ASSETS");
    if std::env::var("NOYA_GEN_ASSETS").as_deref() != Ok("1") {
        return;
    }
    let out_dir: PathBuf = std::env::var_os("OUT_DIR").unwrap().into();
    noya_cli::codegen::generate_completions(&out_dir);
    noya_cli::codegen::generate_manpages(&out_dir);
}
```

Distro packagers run:

```bash
NOYA_GEN_ASSETS=1 cargo build --release --locked
# Artefacts at target/release/build/noya-cli-<hash>/out/{noyafmt.1, noyafmt.bash, …}
```

A short shim `ci/extract-assets.sh` finds and copies them to a stable location.

### 2.2 `crates/xtask/` for developer ergonomic

```bash
cargo xtask completions   # writes complete/*.{bash,fish,zsh,ps1}
cargo xtask manpages      # writes doc/*.1
cargo xtask sbom          # writes SBOM.txt
cargo xtask notice        # writes NOTICE
```

Both entry points call into the same `noya_cli::codegen::*` functions, so output is byte-identical regardless of which path produced it. CI proves this with:

```yaml
- run: NOYA_GEN_ASSETS=1 cargo build --release --bin noyafmt --bin noyavalidate
- run: cargo xtask completions
- run: diff -r target/release/build/noya-cli-*/out/ complete/
```

### 2.3 Asciidoc man pages

`doc/noyafmt.1.adoc` (hand-written prose) + the clap-derived flag dump (auto-generated) → `doc/noyafmt.1` (committed). Generated via `asciidoctor` invoked from `cargo xtask manpages`.

**Phase 2 deliverables:** 1 PR. ~6 new files. **Effort: 1.5 days.**

## Phase 3 — Distro packaging metadata

`pkg/` populated with templates. No CI workflow changes (those land in Phase 4). All templates are inert text with `__VERSION__` / `__SHA256__` placeholders.

### 3.1 — 3.7 Standard Linux + macOS + Windows targets

| Subdir | Files | Purpose |
|---|---|---|
| `pkg/debian/` | `control`, `copyright`, `changelog`, `compat`, `rules`, `README.md` | Source-package metadata for upstream Debian |
| `pkg/rpm/` | `noyalib.spec` | Fedora / RHEL .spec |
| `pkg/arch/` | `PKGBUILD.template`, `PKGBUILD-source.template` | AUR `noyalib-bin` and `noyalib` |
| `pkg/homebrew/` | `noyalib.rb.template` | Personal tap (`sebastienrousseau/homebrew-tap`) → eventually homebrew-core |
| `pkg/nix/` | `flake.nix`, `package.nix` | `nix run github:sebastienrousseau/noyalib` works for any Nix user |
| `pkg/windows/wix/` | `noyalib.wxs` | MSI installer source |
| `pkg/windows/scoop/` | `noyalib.json` | Scoop bucket manifest |
| `pkg/snap/` | `snapcraft.yaml` | Snap classic confinement |
| `pkg/flatpak/` | `io.noyalib.noyafmt.yaml` | Flathub manifest |
| `pkg/docker/` | `Dockerfile`, `Dockerfile.full`, `Dockerfile.mcp` | Distroless container, full-feature image, MCP server image |

### 3.10 VS Code extension (`pkg/vscode/`)

```
pkg/vscode/
├── package.json
├── README.md
├── CHANGELOG.md
├── icon.png
├── language-configuration.json
├── syntaxes/yaml.tmLanguage.json
└── src/extension.ts          # spawns noyalib-lsp over stdio
```

Built and packaged by a new `vscode-extension` job in `release-binaries.yml`. Published to:

- VS Code Marketplace (`vsce publish`, secret `VSCODE_MARKETPLACE_PAT`).
- Open VSX (`ovsx publish`, secret `OPEN_VSX_PAT`).
- Uploaded as `noyalib-<version>.vsix` to the GitHub Release.

The extension auto-detects a bundled `noyalib-lsp` binary; falls back to `which noyalib-lsp` for system installs.

### 3.11 npm via `wasm-pack` (`crates/noyalib-wasm/`)

New `npm-publish` job in `release-binaries.yml`:

```yaml
- run: |
    cd crates/noyalib-wasm
    wasm-pack build --release --target bundler --scope noyalib
- run: |
    cd crates/noyalib-wasm/pkg
    npm publish --provenance --access public
```

Package name: `@noyalib/noyalib-wasm`. Secret: `NPM_TOKEN`. The
`--provenance` flag produces an npm-native SLSA attestation.

### 3.12 MCP server distribution

Two channels for `noyalib-mcp`:

1. **Docker image** (`pkg/docker/Dockerfile.mcp`) → `ghcr.io/sebastienrousseau/noyalib-mcp:<v>` via the existing `container-publish` job.
2. **`npx` wrapper** (`pkg/npm-mcp-wrapper/`): a small Node bootstrap that downloads the platform-appropriate `noyalib-mcp` binary from GitHub Releases on first run, caches in `~/.cache/noyalib-mcp/`. Published as `noyalib-mcp` on npm. Lets AI agents and users invoke `npx noyalib-mcp` without a Rust toolchain — removes the #1 barrier to MCP-tool adoption.

A Docker Desktop Extension is feasible but disproportionately costly to maintain — defer to v0.1.x.

**Phase 3 deliverables:** 1 PR. ~30 new template files. No CI workflow changes. **Effort: 3 days.**

## Phase 4 — Release pipeline (`release-binaries.yml`)

A single workflow that on every `v*` tag produces signed binaries for 14 targets, `.deb` + `.rpm` packages for the Linux gnu legs, MSIs for Windows, container images for GHCR, the VS Code `.vsix`, the npm `@noyalib/noyalib-wasm` package, and updates Homebrew + AUR + Scoop manifests automatically.

### 4.1 Triggers (two-phase rollout)

```yaml
# Phase 4.1.a — initial introduction
on:
  workflow_dispatch:
    inputs:
      tag:      { description: 'Tag to package', required: true }
      dry_run:  { type: boolean, default: true }

# Phase 4.1.b — after a clean dry-run end-to-end
on:
  push:
    tags: ['v[0-9]+.[0-9]+.[0-9]+', 'v[0-9]+.[0-9]+.[0-9]+-*']
  workflow_dispatch:
    inputs:
      dry_run: { type: boolean, default: false }
```

### 4.2 Job DAG

```
                   ┌─ create-release ─┐
                   │                  │
                   v                  v
            ┌─ build-release  ─┐  build-source-tarball
            │  (matrix × 14)    │
            ├──► .deb (gnu legs)┤
            ├──► .rpm (gnu legs)┤
            ├──► .msi (win legs)┤
            │                   │
            v                   v
       lipo-universal     verify-reproducible
            │                   │
            └──────► github-release ─────────┐
                       │                     │
       ┌───────┬───────┼───────┬─────────┐   │
       v       v       v       v         v   │
    homebrew  aur   scoop   vscode-      npm-publish
     bump    bump    bump   extension       │
                              │             │
                              v             v
                      ovsx publish   container-publish
```

### 4.3 Build matrix (14 targets)

| target | runner | toolchain | `cross`? | .deb | .rpm |
|---|---|---|---|---|---|
| `x86_64-unknown-linux-gnu` | ubuntu-latest | stable | no | ✓ | ✓ |
| `x86_64-unknown-linux-musl` | ubuntu-latest | stable | yes | — | — |
| `aarch64-unknown-linux-gnu` | ubuntu-24.04-arm | stable | no | ✓ | ✓ |
| `aarch64-unknown-linux-musl` | ubuntu-24.04-arm | stable | no | — | — |
| `armv7-unknown-linux-gnueabihf` | ubuntu-latest | stable | yes | ✓ | — |
| `arm-unknown-linux-gnueabihf` | ubuntu-latest | stable | yes | ✓ | — |
| `powerpc64le-unknown-linux-gnu` | ubuntu-latest | stable | yes | ✓ | — |
| `s390x-unknown-linux-gnu` | ubuntu-latest | stable | yes | ✓ | — |
| `riscv64gc-unknown-linux-gnu` | ubuntu-latest | stable | yes | ✓ | — |
| `x86_64-apple-darwin` | macos-13 | stable | no | — | — |
| `aarch64-apple-darwin` | macos-14 | stable | no | — | — |
| `x86_64-pc-windows-msvc` | windows-latest | stable | no | — | — |
| `i686-pc-windows-msvc` | windows-latest | stable | no | — | — |
| `aarch64-pc-windows-msvc` | windows-latest | stable | no | — | — |

`fail-fast: false` so one target failure doesn't abort the rest.

### 4.4 Per-target steps

1. Checkout at the resolved tag.
2. Install toolchain (`dtolnay/rust-toolchain@master` + `rust-src` for musl sysroot).
3. Cache (`Swatinem/rust-cache@v2`, key includes target triple).
4. Install `cross` if needed.
5. **Build**: `cargo build --release --locked --target $TARGET --bin noyafmt --bin noyavalidate`.
6. **Strip + debug-symbol split** (Linux gnu only):
   ```bash
   for bin in noyafmt noyavalidate; do
     BIN="target/${TARGET}/release/${bin}"
     llvm-objcopy --only-keep-debug "${BIN}" "${BIN}.debug"
     llvm-objcopy --strip-debug --strip-unneeded "${BIN}"
     llvm-objcopy --add-gnu-debuglink="${BIN}.debug" "${BIN}"
   done
   ```
7. **Verify** binary type via `file`.
8. **Stage tarball** (or zip on Windows) including `README.md`, `CHANGELOG.md`, `LICENSE-*`, `COPYING`, `doc/*.1`, `complete/*`, `NOTICE`.
9. **Generate SHA256 + SHA512** sidecars.
10. **Build .deb** via `cargo deb --target $TARGET --no-build`.
11. **Build .rpm** via `cargo generate-rpm` (x86_64-gnu + aarch64-gnu).
12. **Build .msi** via `cargo wix` (windows legs).
13. **Sign** every artefact with cosign keyless.
14. **Attest SLSA L3** via `actions/attest-build-provenance@v2`.
15. **Upload** to the GitHub Release.

### 4.5 Reproducible builds

Every build leg exports:

```bash
export SOURCE_DATE_EPOCH=$(git log -1 --format=%ct "$GITHUB_REF")
export RUSTFLAGS="--remap-path-prefix=$HOME=~ --remap-path-prefix=$(pwd)=/build"
```

A `verify-reproducible` job rebuilds one target on a clean runner and `diff`s the SHA256 against the published artefact. Promotional jobs (Homebrew, AUR, Scoop, VS Code, npm) gate on this job's success.

### 4.6 macOS Universal Binary (`lipo-universal`)

After both macos legs succeed:

```bash
for bin in noyafmt noyavalidate; do
  lipo -create \
    "noyalib-${VER}-x86_64-apple-darwin/${bin}" \
    "noyalib-${VER}-aarch64-apple-darwin/${bin}" \
    -output "${bin}"
  lipo -info "${bin}"   # verifies fat-binary structure
done
tar -czf "noyalib-${VER}-universal-apple-darwin.tar.gz" universal/
```

Homebrew can then `bottle :all_macos` against the universal tarball.

### 4.7 Promotional jobs (post `github-release`)

| Job | Action | Secret |
|---|---|---|
| `homebrew-bump` | `mislav/bump-homebrew-formula-action@v3` against `sebastienrousseau/homebrew-tap` | `HOMEBREW_TAP_TOKEN` |
| `aur-bump` | `KSXGitHub/github-actions-deploy-aur@v3` for `noyalib-bin` | `AUR_SSH_PRIVATE_KEY` |
| `scoop-bump` | `ScoopInstaller/GithubActions@v2` against `sebastienrousseau/scoop-bucket` | `SCOOP_BUCKET_TOKEN` |
| `vscode-extension` | `vsce publish` + `ovsx publish` | `VSCODE_MARKETPLACE_PAT`, `OPEN_VSX_PAT` |
| `npm-publish` | `npm publish --provenance` for `@noyalib/noyalib-wasm` | `NPM_TOKEN` |
| `container-publish` | `docker/build-push-action@v6` to GHCR (multi-arch amd64+arm64) | `GITHUB_TOKEN` (or optional `DOCKER_PASSWORD` for non-GHCR registries) |

### 4.8 Required repo secrets summary

| Secret | Purpose |
|---|---|
| `CARGO_REGISTRY_TOKEN` | crates.io publish (existing) |
| `HOMEBREW_TAP_TOKEN` | PR against personal tap |
| `AUR_SSH_PRIVATE_KEY` | push to AUR |
| `SCOOP_BUCKET_TOKEN` | push to Scoop bucket |
| `VSCODE_MARKETPLACE_PAT` | `vsce publish` |
| `OPEN_VSX_PAT` | `ovsx publish` |
| `NPM_TOKEN` | npm publish (with `--provenance`) |
| `DOCKER_PASSWORD` | optional; only if pushing to non-GHCR registries |
| `GITHUB_TOKEN` | provided automatically (GHCR push, release upload) |

Documented in `pkg/README.md` for new maintainers.

**Phase 4 deliverables:** 1 PR adding `release-binaries.yml` (~700 lines), simplifying `release.yml` to crates.io-only, `pkg/Dockerfile`+`pkg/Dockerfile.full`+`pkg/Dockerfile.mcp`, secrets documentation. **Effort: 5 days, including dry-run iteration.**

## Phase 5 — Distro publishing

| Channel | Submit | Effort | Auto-bumped after submission? |
|---|---|---|---|
| Personal Homebrew tap | New repo `sebastienrousseau/homebrew-tap`, seed `Formula/noyalib.rb` | 1 h | yes (`homebrew-bump`) |
| AUR (`noyalib-bin`) | AUR account + SSH key, push initial PKGBUILD | 1 h | yes (`aur-bump`) |
| Personal Scoop bucket | New repo + initial `noyalib.json` | 1 h | yes (`scoop-bump`) |
| VS Code Marketplace + Open VSX | One-time publisher account creation | 2 h | yes (`vscode-extension`) |
| npm scope `@noyalib` | One-time org / scope creation | 30 min | yes (`npm-publish`) |
| Homebrew core | PR against `Homebrew/homebrew-core` once ≥ 75 stars + ≥ 30 days + stable API | hand-filed | no — manual |
| Debian | ITP bug + DD sponsor + NEW queue (months) | weeks | no — manual |
| Fedora / RHEL | Packager account + sponsor + review request (months) | weeks | no — manual |
| openSUSE OBS | Account + initial submission | 2 h | semi-auto |
| nixpkgs | PR against `NixOS/nixpkgs` | 2 h | no — manual |
| Snapcraft | `snapcraft register` + `snapcraft upload` | 1 h | semi-auto via store |
| Flathub | PR against `flathub/io.noyalib.noyafmt` | 2 h | no — manual |

**Phase 5 deliverables:** ad-hoc, multi-week. Each distro tracked separately as it lands. **Effort: months of intermittent maintainer work.**

> **Runbook:** `pkg/PUBLISH.md` is the per-channel runbook
> covering bootstrap (account creation, secret population),
> first publish, and ongoing maintenance for every channel in
> the table above. The maintainer follows it sequentially as
> distros come online.

## Phase 6 — Documentation

| File | Content | Effort |
|---|---|---|
| `doc/USER-GUIDE.md` | Long-form tutorial complementing README. | 1 d |
| `doc/MIGRATION-FROM-SERDE-YAML.md` | Name-by-name mapping. | 0.5 d |
| `doc/ARCHITECTURE.md` | Parser → AST → CST → output. | 1 d |
| `doc/noyafmt.1.adoc`, `doc/noyavalidate.1.adoc` | Asciidoc man pages. | 1 d |
| `pkg/VERIFY.md` | cosign + SLSA + vendoring cookbook. | 0.5 d |
| `SECURITY.md` extension | Verification commands. | 0.25 d |
| `CONTRIBUTING.md` extension | "How to add a new packaging target". | 0.5 d |
| `README.md` install table | All distros. | 0.25 d |

**Effort: 1 week, intermittent.**

## Phase 7 — Hardening

### 7.1 Reproducible builds

`SOURCE_DATE_EPOCH` + `--remap-path-prefix` per release leg. `verify-reproducible` job in the release pipeline gates promotional jobs on byte-identity.

### 7.2 MUSL static linking

`file` check confirms `statically linked` on every musl leg. CI fails the release leg if a musl binary links dynamically.

### 7.3 Dependency vendoring (`make vendor`)

```makefile
vendor:
	cargo vendor --versioned-dirs vendor
	@echo "Vendored. Configure cargo to use it via .cargo/config.toml [source.crates-io] replace-with = 'vendored'."
```

CI job `vendor-build` runs `cargo vendor` then `cargo build --offline --workspace`. Docs in `pkg/VERIFY.md`. A `vendor-tarball` release job ships `noyalib-<version>-vendor.tar.xz` with each release for cached offline builds.

### 7.4 CodeQL static analysis

New job in `.github/workflows/security.yml`:

```yaml
codeql-analyze:
  permissions: { actions: read, contents: read, security-events: write }
  strategy: { matrix: { language: [rust] } }
  steps:
    - uses: actions/checkout@v5
    - uses: github/codeql-action/init@v3
      with: { languages: ${{ matrix.language }} }
    - run: cargo build --workspace --all-features
    - uses: github/codeql-action/analyze@v3
```

Findings land in the repo's Security tab. Triage cadence: weekly during the launch program, monthly thereafter.

### 7.5 Dynamic per-crate MSRV check

`scripts/msrv-per-crate.sh`:

```bash
#!/usr/bin/env bash
# Run `cargo +<msrv> check` per workspace crate independently, so a
# satellite crate with a higher MSRV than the workspace floor is
# caught before it merges.
set -euo pipefail
for crate in crates/*/; do
  manifest="${crate}Cargo.toml"
  [[ -f "$manifest" ]] || continue
  msrv=$(grep -E '^rust-version' "$manifest" | head -1 | sed 's/.*"\(.*\)".*/\1/')
  [[ -n "$msrv" ]] || continue
  echo "→ Checking ${crate} against rust ${msrv}"
  rustup toolchain install "$msrv" --profile minimal --quiet
  cargo +"$msrv" check --manifest-path "$manifest" --locked
done
```

CI job `msrv-per-crate` runs this on every PR. Catches drift if e.g. `noyalib-lsp` adopts a feature that requires Rust 1.80 while the workspace floor is still 1.75 — which would silently break distros pinned to the lower rustc.

### 7.6 Intra-doc link strictness

CI's docs job:

```yaml
- env:
    RUSTDOCFLAGS: >-
      -D warnings
      -D rustdoc::broken_intra_doc_links
      -D rustdoc::private_intra_doc_links
      -D rustdoc::invalid_codeblock_attributes
      -D rustdoc::invalid_html_tags
      -D rustdoc::bare_urls
      --cfg docsrs
      --extern-html-root-url serde=https://docs.rs/serde/latest
      --extern-html-root-url indexmap=https://docs.rs/indexmap/latest
  run: cargo +nightly doc --workspace --all-features --no-deps
```

Every broken `[`Type`]` reference in a doc comment fails CI.

### 7.7 License compliance

`cargo about generate -c about.toml about.hbs > NOTICE` in CI; `NOTICE` shipped in every release tarball, .deb, .rpm, MSI.

### 7.8 Cross-target test matrix

`cargo test` via `cross` on every release-binary target, not just host runners. New `cross-test` job in `ci.yml`, scheduled weekly (cost-aware — full matrix takes ~2 h).

### 7.9 Cosign root-of-trust pinning

`pkg/VERIFY.md` documents the Rekor + Fulcio public-key fingerprints + their expiry rotation cadence so verifiers don't depend on sigstore-go's defaults.

### 7.10 OpenSSF Best Practices badge

Apply at https://www.bestpractices.dev once Phases 1–4 land. Gold target.

**Effort: 1.5 weeks.**

## Verification matrix

After every phase, the following must be true:

| Gate | Phase 0 | 1 | 2 | 3 | 4 | 7 |
|---|---|---|---|---|---|---|
| `cargo build --workspace --all-features` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `cargo test --workspace --all-features` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Coverage gate | 95/93/92 | ratchet to 98 | 98 | 98 | 98 | 98 |
| `cargo doc` (no warnings, no broken intra-doc links) | warn | deny | deny | deny | deny | deny |
| 56+ examples runnable | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `reuse lint` clean | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `cargo audit` / `deny` / `vet` clean | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Miri focused suite green | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `msrv-per-crate.sh` clean | — | ✓ | ✓ | ✓ | ✓ | ✓ |
| CodeQL clean | — | — | — | — | — | ✓ |
| `cargo vendor` + offline build clean | — | — | — | — | — | ✓ |
| Release dry-run produces all 14 + universal tarballs | — | — | — | — | ✓ | ✓ |
| Reproducible build verified | — | — | — | — | ✓ | ✓ |
| `npm publish --dry-run`, `vsce package` clean | — | — | — | — | ✓ | ✓ |

## Estimated timeline

| Phase | Effort | Calendar |
|---|---|---|
| 0 — pre-flight | done | day 1 |
| 1 — restructure (one zero-logic commit) | 1 d code + 1 d review | days 2–3 |
| 2 — tooling (build.rs + xtask hybrid) | 1.5 d | days 4–5 |
| 3 — packaging templates + VS Code + npm + MCP wrappers | 3 d | days 6–8 |
| 4 — release pipeline + universal + debug-info + npm + .vsix | 5 d | days 9–13 |
| 5 — distro publishing | weeks (per distro) | weeks 3+ |
| 6 — documentation | 1 wk (intermittent) | weeks 3+ |
| 7 — hardening (vendor + CodeQL + intra-doc + per-crate MSRV) | 1.5 wk | weeks 3–4 |

**Total focused engineering: ~4 working weeks** before distro outreach kicks off.

## Execution checklist

| # | Phase | Lands | Estimated commits |
|---|---|---|---|
| 1 | 0 | branch protection verification | 0 — config only |
| 2 | 1 | restructure (single zero-logic commit) | 1 |
| 3 | 1.5 | `missing_docs = "deny"` + audit fixes | 1 |
| 4 | 2 | xtask + build.rs hybrid | 1 |
| 5 | 3 | `pkg/` templates (deb/rpm/arch/nix/win/snap/flatpak/docker) | 1 |
| 6 | 3.10 | `pkg/vscode/` extension scaffold | 1 |
| 7 | 3.11–12 | `pkg/npm-mcp-wrapper/` + `package.json.tmpl` | 1 |
| 8 | 4 | `release-binaries.yml` workflow_dispatch + dry-run | 1 |
| 9 | 4.1.b | flip release-binaries.yml to auto-on-tag | 1 |
| 10 | 4.6 | `lipo-universal` job | 1 |
| 11 | 4.7 | promotional jobs (Homebrew/AUR/Scoop/VS Code/npm/container) | 1 |
| 12 | 7.1 | reproducible builds + `verify-reproducible` job | 1 |
| 13 | 7.3 | `cargo vendor` + `make vendor` + `vendor-build` job | 1 |
| 14 | 7.4 | CodeQL job | 1 |
| 15 | 7.5 | `msrv-per-crate.sh` + CI job | 1 |
| 16 | 7.6 | strengthened RUSTDOCFLAGS in docs job | 1 |
| 17 | 7.7 | `cargo about` + `NOTICE` shipping | 1 |
| 18 | 6 | doc/USER-GUIDE.md, doc/MIGRATION-FROM-SERDE-YAML.md, doc/ARCHITECTURE.md | 3 |
| 19 | 6 | man-page asciidoc sources | 1 |
| 20 | 5 | distro PRs (per distro, ad-hoc) | many |

Total ~20 PRs over ~4 weeks for the engineering work; distro
outreach then runs at distro maintainer pace.

## Road to 1.0 — the long 0.x runway

`1.0.0` is a contract, not a milestone date: it promises SemVer-strict
stability and a commercial-grade support commitment. It is **earned by
field maturity, not scheduled.** The entire `0.x` line — all the way
through `0.9.999` — is the maturation period: ship, gather real-world
usage, break the API freely under `0.x` rules, and harden it against
actual workloads rather than against our own test suite. This is a
deliberately long road. There is no target date, only gates.

### Versioning posture during 0.x

Cargo's SemVer rules let a `0.0.x` patch break, and a `0.x.0` minor
break. We use that latitude on purpose — a wrong early API is far
cheaper to fix now than to carry past `1.0`.

| Band | Theme | API contract |
|---|---|---|
| `0.0.x` | Foundation & hardening (Phases 0–7 above) | Exploratory; may break per patch, noted in `CHANGELOG.md`. |
| `0.1 → 0.8` | Capability maturation & API convergence | Each minor is a coherent capability milestone plus accumulated field feedback. Breaking changes allowed, increasingly rare, always with migration notes. |
| `0.9.x` | API-freeze candidates | Feature-complete for the `1.0` scope; bugfixes and polish only. A public, written **1.0 API review** opens here. |
| `0.9.999` | Pre-1.0 soak / extended RC | The frozen `1.0` API, shipped as a long-lived, widely-deployed release. It runs in the wild until every gate below is demonstrably met. `1.0.0` is then a near-mechanical re-tag of a proven `0.9.999`. |

`0.9.999` is intentional: it announces "this *is* 1.0, pending proof"
and lets real deployments soak the final API for a long time, without
prematurely spending the `1.0` stability promise.

### 1.0 gates (signal-gated, never date-gated)

`1.0.0` ships only when **all** of the following are demonstrably true.
Each is a measurable signal, not a vibe.

**API & correctness**
- ≥ 4 consecutive minors with **zero** breaking changes to the public API.
- `cargo semver-checks` clean across that window; no `#[doc(hidden)]` or
  `__`-prefixed escape hatches inside the `1.0` surface.
- A written, reviewed **1.0 API guarantee** enumerating the stable
  surface and the explicitly-unstable corners.
- Fuzz targets accumulate ≥ N CPU-months with no new reproducers;
  coverage gate held at its ceiling.

**Adoption & field testing**
- Real downstream users beyond CI/mirrors: a meaningful count of distinct
  crates.io reverse-dependencies and/or named production deployments.
- Issues from real workloads triaged to **zero open criticals** — a
  steady state, not a launch spike.
- Confirmed in the wild across the target matrix (OSes, architectures,
  `no_std`), not just in our runners.

**Community & governance**
- **≥ 2 maintainers** with merge rights (bus-factor retired).
- `CODE_OF_CONDUCT.md` + a short `GOVERNANCE.md` (how decisions and
  releases are made); active Discussions.
- The security-disclosure and release processes each exercised
  end-to-end by more than one person.

**Ecosystem & distribution**
- Every distribution channel green **and consumed**: crates.io, docs.rs,
  Docker/GHCR, and the packaging channels (AUR / Scoop / Homebrew / npm /
  VS Code — see #99) actually publishing and used by real installs.
- A stable, documented MSRV policy; the serde_yaml migration guide
  validated by external migrators.

**Operational & supply chain**
- Reproducible builds reproduced independently by a third party.
- OpenSSF Scorecard ≥ 8.0 and the OpenSSF Best Practices badge at
  *passing* (targeting *silver*); zero open advisories.
- Signed releases verified by consumers (the `pkg/VERIFY.md` cookbook
  exercised externally).

**Commercial readiness**
- A written support / stability / LTS policy and a security-response SLA.
- Published performance baselines with regression gating in CI.

### Explicit non-gates

`1.0` is **not** triggered by a calendar date, a feature count, a star
count, or "it's been long enough." A breaking change during `0.x` is a
tool we use deliberately, not a failure to apologise for.

### After 1.0

- Strict SemVer: a breaking change implies `2.0` and a deprecation cycle.
- A `1.x` LTS branch carrying backported security fixes.
- The stable core becomes the foundation a commercial offering can layer
  on — never a reason to destabilise the open library.
