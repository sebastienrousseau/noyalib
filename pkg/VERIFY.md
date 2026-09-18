<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Verifying noyalib release artefacts

Every `noyalib` release artefact is signed keylessly with
[sigstore](https://www.sigstore.dev/) and carries a SLSA L3 build
provenance attestation linking it to the exact CI workflow run
that produced it. This document is the reference for downstream
packagers (Debian, Fedora, Arch, Homebrew, container builders)
who need to verify those bindings before trusting the artefact.

> **Tooling.** All commands assume modern `cosign` (≥ 2.x) and
> `gh` (≥ 2.40). Both ship in current Linux distros, in Homebrew,
> and as pre-built binaries from the upstream releases.

## Verifying a release tarball

Each tarball ships alongside `<artefact>.sig` (signature) and
`<artefact>.pem` (Fulcio certificate). Verify the binding to this
repo and workflow with:

```bash
COSIGN_EXPERIMENTAL=1 cosign verify-blob \
  --certificate-identity-regexp 'https://github.com/sebastienrousseau/noyalib/' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  --certificate <artefact>.pem \
  --signature   <artefact>.sig \
  <artefact>
```

Expected output: `Verified OK`. If the certificate identity or
issuer doesn't match, the signature is from a different workflow
or a different repo — do not trust the artefact.

## Verifying SLSA L3 build provenance

The provenance attestation records:

- The exact source commit the artefact was built from.
- The CI workflow file path (`.github/workflows/release.yml` or
  `release-binaries.yml`).
- The triggering ref (must match `refs/tags/v<version>`).
- The builder identity (GitHub-hosted runner).

```bash
gh attestation verify \
  --owner sebastienrousseau \
  <artefact>
```

Expected output includes a green-tick line and the predicate type
`https://slsa.dev/provenance/v1`.

## Verifying SBOM authenticity

`SBOM.txt` is generated from `cargo tree --edges normal` at build
time and signed via the same keyless flow:

```bash
cosign verify-blob \
  --certificate-identity-regexp 'https://github.com/sebastienrousseau/noyalib/' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  --certificate SBOM.txt.pem \
  --signature   SBOM.txt.sig \
  SBOM.txt
```

## Verifying a crates.io release

The crate published to `crates.io` carries the same signatures as
the GitHub-Release tarball; pull the `.crate` from the GitHub
Release alongside its `.sig` / `.pem` and run the
`cosign verify-blob` command above. The `.crate` byte-identity is
preserved by `cargo` between local pack and registry upload.

## Verifying a container image

Container images pushed to GHCR are signed via `cosign` keyless:

```bash
cosign verify ghcr.io/sebastienrousseau/noyafmt:<tag> \
  --certificate-identity-regexp 'https://github.com/sebastienrousseau/noyalib/' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com'
```

## Sigstore root-of-trust pinning

`cosign verify-blob` consults the public Rekor + Fulcio
endpoints by default. For most callers that's correct — sigstore
is the public infrastructure noyalib pins to. When a packager
wants to be **fully reproducible** (no live network query at
verification time) the public keys can be pinned locally:

```bash
# Fetch and pin the current Fulcio + Rekor public keys.
cosign initialize --root https://tuf-repo-cdn.sigstore.dev

# Verify against the pinned trust root rather than the live
# endpoint. Useful for hermetic / FIPS-bound build environments.
cosign verify-blob \
  --insecure-ignore-tlog=false \
  --rekor-url=offline \
  --certificate-identity-regexp 'https://github.com/sebastienrousseau/noyalib/' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  --certificate <artefact>.pem \
  --signature   <artefact>.sig \
  <artefact>
```

The keys rotate infrequently (Fulcio root: ~once per 6 months;
Rekor: stable since 2022). Re-run `cosign initialize` whenever
sigstore announces a rotation; otherwise the pinned trust root
remains valid.

## Building from a vendored tarball (offline / FIPS / RHEL)

Some downstream environments cannot reach crates.io at build
time — air-gapped enterprise networks, Fedora's `mock` builders,
RHEL's `koji` + `osbuild`. noyalib publishes a
`noyalib-<version>-vendor.tar.xz` artefact alongside every
release that contains the entire dependency tree pre-fetched
into a `vendor/` directory.

```bash
# 1. Extract the vendored tree alongside the source.
tar -xJf noyalib-0.0.1-vendor.tar.xz   # produces vendor/

# 2. Configure cargo to use the vendored sources instead of
#    crates.io.
mkdir -p .cargo
cat > .cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "vendored"
[source.vendored]
directory = "vendor"
EOF

# 3. Build offline. cargo will refuse network access; if the
#    vendored tree is incomplete the build fails at this step
#    with a clear error.
cargo build --release --offline --locked
```

The vendor tarball is produced by `make vendor` and is signed alongside every other release
artefact via cosign keyless. The same `cosign verify-blob`
invocation as for the source crate applies.

## Reporting a verification failure

If any verification step fails for a published release, treat it
as a security incident and contact the maintainer per
`SECURITY.md`. Do not redistribute the artefact.
