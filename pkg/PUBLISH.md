<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `pkg/PUBLISH.md` — distribution-channel runbook

The companion to `docs/PLAN.md`'s Phase 5. Covers every external
channel `release-binaries.yml` ships to: prerequisites, account
creation, secret population, first publish, ongoing maintenance.

Each section answers four questions:

1. **What's the channel?** What's published, where it lands.
2. **Bootstrap.** One-time setup before the first release.
3. **First publish.** What lands, what to verify.
4. **Ongoing maintenance.** What the workflow does on every tag,
   what the maintainer needs to watch.

> **Status legend**
>
> | | |
> |---|---|
> | ⏳ Bootstrap pending | The maintainer hasn't created the upstream account / repo yet. The `release-binaries.yml` job for this channel is wired but will fail until the prerequisites land. |
> | 🟢 Live | First publish succeeded, subsequent releases auto-bump. |
> | 🛠 Manual | Channel cannot be auto-bumped; each release needs a hand-filed PR or submission. Tracked here so the cadence is documented. |

---

## 1. crates.io — Rust source crate

**Channel.** The `noyalib` source crate is published to crates.io
on every tag by the existing `.github/workflows/release.yml`
(distinct from `release-binaries.yml`). Signed via cosign keyless
+ SLSA L3 attestation. **Status: 🟢 live since v0.0.1.**

**Bootstrap.** Already complete. The repo secret
`CARGO_REGISTRY_TOKEN` is configured.

**First publish.** v0.0.1 published 2026-05-04; verifiable via
`cargo search noyalib`.

**Ongoing.** No maintainer action required. Each `v*` tag triggers
the publish.

---

## 2. Personal Homebrew tap (`sebastienrousseau/homebrew-tap`)

**Channel.** A user-installable tap exposing the `noyalib`
formula (binary + completions + man pages). Used by every macOS
Homebrew user and Linuxbrew users until the formula lands in
homebrew-core.

Install path:

```bash
brew tap sebastienrousseau/tap
brew install noyalib
```

**Status: ⏳ Bootstrap pending.**

### Bootstrap (one-time, ~1 h)

1. Create the GitHub repo:

   ```bash
   gh repo create sebastienrousseau/homebrew-tap \
       --public \
       --description "Personal Homebrew tap for noyalib and related tooling" \
       --license MIT
   ```

2. Seed the layout the `homebrew-bump` job expects:

   ```bash
   git clone git@github.com:sebastienrousseau/homebrew-tap.git
   cd homebrew-tap
   mkdir -p Formula
   # Render the template once for the current latest tag, then
   # commit. The release pipeline rewrites it on every subsequent
   # release.
   sed -e "s/__VERSION__/0.0.1/g" \
       -e "s/__SHA256__/$(curl -fsSL https://github.com/sebastienrousseau/noyalib/archive/refs/tags/v0.0.1.tar.gz | sha256sum | cut -d' ' -f1)/g" \
       ../noyalib/pkg/homebrew/noyalib.rb.template \
       > Formula/noyalib.rb
   git add Formula/noyalib.rb
   git commit -S -m "seed: noyalib 0.0.1"
   git push origin main
   ```

3. Generate a fine-grained PAT scoped to the tap repo with
   `Contents: read+write` and `Pull requests: read+write`.

4. Add it to the noyalib repo's secrets as `HOMEBREW_TAP_TOKEN`:

   ```bash
   gh secret set HOMEBREW_TAP_TOKEN --repo sebastienrousseau/noyalib
   ```

### First publish

Dispatch `release-binaries.yml` against the seed tag with
`dry_run: false`. The `homebrew-bump` job opens a PR against the
tap repo bumping `url` and `sha256`. Merge it; verify
`brew install sebastienrousseau/tap/noyalib` works on a clean
machine.

### Ongoing

Per release, `homebrew-bump` PRs the bump automatically. The
maintainer's job is to merge.

---

## 3. AUR — `noyalib-bin` (binary) and `noyalib` (source)

**Channel.** Two packages on the Arch User Repository:

- `noyalib-bin` — pre-built x86_64 / aarch64 tarballs from the
  GitHub Release. Fast install path.
- `noyalib` — built from source via `cargo`. Required by Arch
  policy when binaries are unavailable for a user's arch.

**Status: ⏳ Bootstrap pending.**

### Bootstrap (one-time, ~1 h)

1. Register an AUR account at <https://aur.archlinux.org/register>
   (one-time, links to GitHub / matrix identity).

2. Add the maintainer's SSH public key under "My Account" → "SSH
   Public Key".

3. Push the initial PKGBUILDs:

   ```bash
   # noyalib-bin
   git clone ssh://aur@aur.archlinux.org/noyalib-bin.git
   cd noyalib-bin
   sed -e "s/__VERSION__/0.0.1/g" \
       -e "s/__SHA256_X86_64__/$(curl -fsSL https://github.com/sebastienrousseau/noyalib/releases/download/v0.0.1/noyalib-0.0.1-x86_64-unknown-linux-gnu.tar.gz.sha256 | cut -d' ' -f1)/g" \
       -e "s/__SHA256_AARCH64__/$(curl -fsSL https://github.com/sebastienrousseau/noyalib/releases/download/v0.0.1/noyalib-0.0.1-aarch64-unknown-linux-gnu.tar.gz.sha256 | cut -d' ' -f1)/g" \
       ../noyalib/pkg/arch/PKGBUILD.template > PKGBUILD
   makepkg --printsrcinfo > .SRCINFO
   git add PKGBUILD .SRCINFO
   git commit -S -m "Initial commit (0.0.1)"
   git push

   # noyalib (source)
   git clone ssh://aur@aur.archlinux.org/noyalib.git
   cd noyalib
   # render PKGBUILD-source.template, commit similarly
   ```

4. Add the AUR SSH private key (the matching half of the key in
   step 2) to the noyalib repo's secrets as
   `AUR_SSH_PRIVATE_KEY`:

   ```bash
   gh secret set AUR_SSH_PRIVATE_KEY --repo sebastienrousseau/noyalib < /path/to/aur_id_ed25519
   ```

### First publish

Same flow as Homebrew: dispatch `release-binaries.yml` with
`dry_run: false`. The `aur-bump` job pushes the rendered PKGBUILD
to AUR. Verify on a fresh Arch box:

```bash
yay -S noyalib-bin
noyafmt --version
```

### Ongoing

`aur-bump` runs on every release. AUR comments occasionally
arrive with packaging questions — answer within 48 h to stay in
good standing.

---

## 4. Personal Scoop bucket (`sebastienrousseau/scoop-bucket`)

**Channel.** A user-installable Scoop bucket exposing the
`noyalib` manifest for Windows users.

Install path:

```pwsh
scoop bucket add sebastienrousseau https://github.com/sebastienrousseau/scoop-bucket
scoop install sebastienrousseau/noyalib
```

**Status: ⏳ Bootstrap pending.**

### Bootstrap (one-time, ~1 h)

1. Create the bucket repo:

   ```bash
   gh repo create sebastienrousseau/scoop-bucket \
       --public \
       --description "Personal Scoop bucket for noyalib and related Windows tooling" \
       --license MIT
   ```

2. Seed `bucket/noyalib.json` from `pkg/windows/scoop/noyalib.json`
   with the current SHA256s rendered in.

3. Generate a fine-grained PAT (same scopes as the Homebrew tap
   one) and store as `SCOOP_BUCKET_TOKEN`.

### First publish

`scoop-bump` job runs on every release; the maintainer just
verifies the auto-PR / auto-commit lands.

### Ongoing

Mostly hands-off. Scoop's `checkver` mechanism cross-checks
against GitHub Releases, so the bucket self-heals if a release
job partially completes.

---

## 5. VS Code Marketplace + Open VSX

**Channel.** The `noyalib` extension lands in two marketplaces:

- **VS Code Marketplace** (`vscode:extension/sebastienrousseau.noyalib`)
- **Open VSX** (used by VSCodium, Theia, Gitpod)

Both publish from the same `.vsix` produced in the
`vscode-extension` job.

**Status: ⏳ Bootstrap pending.**

### Bootstrap (one-time, ~2 h)

1. **VS Code Marketplace.**
   - Create a publisher at <https://marketplace.visualstudio.com/manage>.
     Publisher id `sebastienrousseau` to match `pkg/vscode/package.json`.
   - Generate a Personal Access Token in Azure DevOps with the
     `Marketplace (manage)` scope.
   - `gh secret set VSCODE_MARKETPLACE_PAT --repo sebastienrousseau/noyalib`

2. **Open VSX.**
   - Sign in at <https://open-vsx.org/> with the same GitHub identity.
   - Create a namespace `sebastienrousseau`.
   - Generate an access token under "User Settings" → "Tokens".
   - `gh secret set OPEN_VSX_PAT --repo sebastienrousseau/noyalib`

3. Bundle `noyalib-lsp` into the extension so users without a
   Rust toolchain still get LSP features. The `vscode-extension`
   job copies the host-runner-built binary into `pkg/vscode/bin/`
   before `vsce package`.

### First publish

Dispatch `release-binaries.yml` with `dry_run: false`. The
`vscode-extension` job:

- Builds the `.vsix`
- Publishes to VS Code Marketplace via `vsce publish`
- Publishes to Open VSX via `ovsx publish`
- Uploads the `.vsix` to the GitHub Release

Verify by installing on both marketplaces:

```bash
code --install-extension sebastienrousseau.noyalib
codium --install-extension sebastienrousseau.noyalib    # Open VSX
```

### Ongoing

Each release reships the extension. Watch the Marketplace's
"User Reviews" and "Issues" tabs for editor-specific bug
reports.

---

## 6. npm — `@noyalib/noyalib-wasm` and `noyalib-mcp`

**Channel.** Two npm packages:

- `@noyalib/noyalib-wasm` — WASM-pack output of
  `crates/noyalib-wasm`. Browser / Node bundlers consume this.
- `noyalib-mcp` — the npx-installable wrapper from
  `pkg/npm-mcp-wrapper/`. Lets AI agents invoke
  `npx noyalib-mcp` without a Rust toolchain.

Both publish with `npm publish --provenance` so each release
carries an npm-native SLSA attestation linked to the GitHub
Actions run that produced it. Authentication uses npm Trusted
Publishing (OIDC) — no long-lived `NPM_TOKEN` is required;
GitHub Actions mints a per-run identity token that npm validates
against the package's trusted-publisher policy.

**Status: ⏳ Bootstrap pending.**

### Bootstrap (one-time, ~30 min)

1. Sign in to npm with the maintainer GitHub identity.
2. Create the org / scope `@noyalib` (free for an open-source
   project).
3. **First publish only:** the chicken-and-egg around Trusted
   Publishing means the package must exist on npm before a
   trusted publisher can be configured. Generate a *short-lived*
   granular access token at
   <https://www.npmjs.com/settings/<user>/tokens> with
   `Read and Publish` scope and the package allowlist `@noyalib/*`
   plus `noyalib-mcp`. Use it for the very first publish only,
   then **revoke immediately**.
4. **After the first publish — configure Trusted Publishing:**
   - <https://www.npmjs.com/package/@noyalib/noyalib-wasm/access>
     → *Trusted Publishers → Add*:
       - Repository: `sebastienrousseau/noyalib`
       - Workflow filename: `release-binaries.yml`
       - Environment: *(leave blank)*
   - Repeat the same for
     <https://www.npmjs.com/package/noyalib-mcp/access>.
5. **Retire the bootstrap secret** once Trusted Publishing is
   wired:

   ```bash
   gh secret delete NPM_TOKEN --repo sebastienrousseau/noyalib
   ```

   The `release-binaries.yml` workflow no longer reads
   `NPM_TOKEN`; both publish jobs declare `id-token: write` and
   rely on the OIDC handshake exclusively.

### First publish

Dispatched along with the rest. Verify:

```bash
npm view @noyalib/noyalib-wasm version
npm view noyalib-mcp     version
npx noyalib-mcp          # spawns the MCP server, downloads binary
```

### Ongoing

Watch the npm "weekly downloads" graph as a leading indicator of
AI-tooling adoption (the wrapper is the primary install path for
agent integrations).

---

## 7. GHCR container images

**Channel.** Three container images pushed to GitHub Container
Registry on every release:

- `ghcr.io/sebastienrousseau/noyafmt:<v>` — distroless
- `ghcr.io/sebastienrousseau/noyalib:<v>` — debian-slim, full feature
- `ghcr.io/sebastienrousseau/noyalib-mcp:<v>` — distroless MCP

Multi-arch via buildx (`linux/amd64`, `linux/arm64`).

**Status: ⏳ Bootstrap pending — but only because no release has
been dispatched yet; no external account creation needed.**

### Bootstrap

The repo's default `GITHUB_TOKEN` has `packages: write`
permission for GHCR pushes. No additional secrets required.

### First publish

Dispatch the workflow; verify with:

```bash
docker pull ghcr.io/sebastienrousseau/noyafmt:0.0.1
docker run --rm ghcr.io/sebastienrousseau/noyafmt:0.0.1 --version
```

### Ongoing

GHCR retention defaults are aggressive for unattested images.
Our cosign + SLSA attestations land them in the "verified" tier;
no manual janitorial needed.

---

## 8. Homebrew core (upstream)

**Channel.** `Homebrew/homebrew-core` — every macOS Homebrew
user gets `brew install noyalib` without first tapping our
personal repo.

**Status: 🛠 Manual; eligibility-gated.**

### Eligibility (per `homebrew-core/CONTRIBUTING.md`)

- Repo ≥ 75 GitHub stars.
- Project ≥ 30 days old.
- Stable release (≥ 1.0.0 ideal, but solid pre-1.0 with
  documented stability story is accepted at maintainer
  discretion).

### Submission

Once eligible, file a hand-rolled PR against the homebrew-core
repo:

```bash
brew create --tap homebrew/core https://github.com/sebastienrousseau/noyalib/archive/refs/tags/v<latest>.tar.gz
# Edit Formula/noyalib.rb until `brew style` and `brew test` pass.
# Submit a PR with the homebrew-core PR template filled in.
```

After merge, the personal tap can stay as a fallback for
pre-release versions, but `brew install noyalib` defaults to the
core copy.

### Ongoing

Major-version bumps in homebrew-core require a manual PR. Patch
bumps can use `brew bump-formula-pr noyalib --version <new>`.

---

## 9. Debian (upstream)

**Channel.** Debian's archive — `apt install noyalib` on every
Debian / Ubuntu / derivative install, no extra repo needed.

**Status: 🛠 Manual; months-long process.**

### Submission

1. **ITP bug.** File an Intent-To-Package bug against the `wnpp`
   pseudo-package:

   ```text
   Subject: ITP: noyalib -- Pure-Rust YAML 1.2 parser, formatter, and validator
   Package: wnpp
   Severity: wishlist
   Owner: Sebastien Rousseau <sebastian.rousseau@gmail.com>

     Package name    : noyalib
     Version         : 0.0.1
     Upstream Author : Sebastien Rousseau
     URL             : https://github.com/sebastienrousseau/noyalib
     License         : MIT OR Apache-2.0
     Programming Lang: Rust
     Description     : (paste the package's description here)
   ```

2. **Find a Debian Developer sponsor.** Mail the
   `pkg-rust-maintainers@alioth-lists.debian.net` list with the
   ITP number and a working `dpkg-buildpackage` build (use
   `pkg/debian/` as the seed).

3. **Upload to mentors.debian.net.** Once a DD agrees to sponsor,
   they push the package to the NEW queue.

4. **NEW queue review.** Debian ftpmasters review for licence
   compliance; usually 1–4 weeks.

### Ongoing

After acceptance, future releases flow via:

```bash
git -C /path/to/noyalib-debian dch -i --distribution unstable
# Edit debian/changelog, debian/control if needed
dpkg-buildpackage -us -uc
dput mentors noyalib_<version>-1_source.changes
```

---

## 10. Fedora / RHEL (upstream)

**Channel.** Fedora's repos via dist-git → `dnf install noyalib`
on Fedora / EPEL.

**Status: 🛠 Manual; months-long process.**

### Submission

1. Become a Fedora Account holder
   (<https://accounts.fedoraproject.org>).
2. Find a sponsor in the `packager-sponsors` group via the
   `#fedora-devel` Matrix channel.
3. File a package review request:
   <https://bugzilla.redhat.com/enter_bug.cgi?product=Fedora&component=Package%20Review>.
   Attach `pkg/rpm/noyalib.spec` rendered for the current tag.
4. Address review comments; once a `+` flag from a reviewer
   lands, request SCM admin to create the dist-git repo.
5. Push the rendered `.spec` to dist-git's `rawhide` branch and
   build via `koji`.

### Ongoing

Standard Fedora workflow — `fedpkg new-sources` for tarball
updates, `fedpkg build` for koji builds.

---

## 11. nixpkgs (upstream)

**Channel.** PR against `NixOS/nixpkgs` adding
`pkgs/development/tools/noyalib/default.nix`. Once merged, every
NixOS user gets `nix-env -iA nixpkgs.noyalib`.

**Status: 🛠 Manual; ~1–4 weeks for first PR.**

### Submission

```bash
git clone git@github.com:sebastienrousseau/nixpkgs.git
cd nixpkgs
git remote add upstream https://github.com/NixOS/nixpkgs.git
git fetch upstream
git checkout -b noyalib-init upstream/master

mkdir -p pkgs/development/tools/noyalib
cp ../noyalib/pkg/nix/package.nix pkgs/development/tools/noyalib/default.nix
# Adapt to nixpkgs idioms — the standalone derivation needs minor
# tweaks (e.g. `lib` is implicit in nixpkgs scope).

# Add an entry under `pkgs/top-level/all-packages.nix`:
#   noyalib = callPackage ../development/tools/noyalib { };

git add pkgs/
git commit -m "noyalib: init at 0.0.1"
git push origin noyalib-init
gh pr create \
    --repo NixOS/nixpkgs \
    --title "noyalib: init at 0.0.1" \
    --body "<paste from .github/PR_TEMPLATE/init.md>"
```

### Ongoing

Major-version bumps need a hand-rolled PR. Patch bumps can use
`nix-update noyalib`.

---

## 12. Snapcraft

**Channel.** The Snap Store; classic-confined snap installable
via `snap install noyalib`.

**Status: ⏳ Bootstrap pending — earmarked for v0.1.x.**

### Bootstrap

```bash
snap install snapcraft --classic
snapcraft register noyalib
# Enter project metadata when prompted.
```

### First publish

```bash
cd /path/to/noyalib
snapcraft pack
snapcraft upload --release=stable noyalib_<v>_amd64.snap
```

### Ongoing

`pkg/snap/snapcraft.yaml` drives the build; per-release upload
can be automated via the `snapcraft-action` GitHub Action once
the credentials are stored.

---

## 13. Flathub

**Channel.** Flathub — `flatpak install flathub io.noyalib.noyafmt`.

**Status: 🛠 Manual; earmarked for v0.1.x.**

### Submission

1. Fork `flathub/flathub` on GitHub.
2. Create a new branch named `io.noyalib.noyafmt`.
3. Add `pkg/flatpak/io.noyalib.noyafmt.yaml` to the branch root.
4. Add a `cargo-sources.json` produced via
   `flatpak-cargo-generator.py Cargo.lock`.
5. PR against `flathub/flathub`. Reviewers test the manifest in
   their CI; merge typically lands in 1–2 weeks.

### Ongoing

Each release-bump opens a PR via the bot
`flathub/flatpak-external-data-checker` if configured;
otherwise hand-roll it.

---

## 14. openSUSE OBS

**Channel.** openSUSE Build Service mirrors the project to every
openSUSE distro and Tumbleweed.

**Status: ⏳ Bootstrap pending — earmarked for v0.1.x.**

### Submission

1. Sign up at <https://build.opensuse.org/>.
2. `osc co home:<user>` to create your home project.
3. Place `pkg/rpm/noyalib.spec` and the source tarball; `osc
   commit`.
4. Once green, submit-request to `devel:languages:rust`.

---

## Cadence summary

| Channel | Auto-bumped per release | Manual review needed |
|---|---|---|
| crates.io | yes | no |
| GHCR (container) | yes | no |
| Homebrew tap | yes (PR auto-opened) | maintainer merges PR |
| AUR | yes | watch comments |
| Scoop bucket | yes | none |
| VS Code Marketplace + Open VSX | yes | none |
| npm | yes | none |
| Homebrew core | no | per release |
| Debian / Fedora | no | per release (eventually flow through DD/Fedora packagers) |
| nixpkgs | no | per release |
| Snapcraft | yes (post-bootstrap) | none |
| Flathub | no | per release |

## Releasing — the maintainer's checklist

The end-to-end flow once every channel above is bootstrapped:

```bash
# 1. Cut the release commit (Cargo.toml version bump,
#    CHANGELOG entry, …)
cargo workspaces version patch

# 2. Tag + push
git tag -s v0.1.0 -m "Release v0.1.0"
git push --tags

# 3. release.yml fires automatically → publishes to crates.io.
# 4. Dispatch release-binaries.yml manually (Phase 4.1.b will
#    eventually auto-fire on tag push):
gh workflow run release-binaries.yml \
    --ref main \
    -f tag=v0.1.0 \
    -f dry_run=false

# 5. Watch CI: every channel job lands a notification.
gh run watch

# 6. Verify via the cookbook in pkg/VERIFY.md.
```

Pause and triage any red job before merging the next change to
`main` — per the project's "CI must always be green" invariant.
