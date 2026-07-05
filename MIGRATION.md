<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migration Guide — noyalib workspace split

This guide covers what changed for consumers when the noyalib
workspace was split into satellite repositories under
[ADR-0005](doc/adr/0005-workspace-split.md). It's aimed at:

- **Rust developers** who `cargo add noyalib*` or ship binaries
  that depend on the workspace.
- **JavaScript / TypeScript developers** using the WASM or MCP
  npm packages.
- **DevOps** running noyalib containers from GHCR.
- **Downstream repo maintainers** who vendored the monorepo via
  `git submodule` or `path = "…"`.

If you're only consuming published packages (crates.io, npm,
GHCR), the short version is: **no code changes required, just
update version pins.**

---

## Split timeline

| Release | Split | New repo | Shipped |
|---|---|---|---|
| v0.0.12 | `noyalib-wasm` | [`sebastienrousseau/noyalib-wasm`](https://github.com/sebastienrousseau/noyalib-wasm) | 2026-07-02 |
| v0.0.13 | `noyalib-mcp` | [`sebastienrousseau/noyalib-mcp`](https://github.com/sebastienrousseau/noyalib-mcp) | 2026-07-05 |
| v0.0.13 | `noyalib-lsp` | [`sebastienrousseau/noyalib-lsp`](https://github.com/sebastienrousseau/noyalib-lsp) | 2026-07-05 |
| v0.0.13 | `noya-cli` | [`sebastienrousseau/noya-cli`](https://github.com/sebastienrousseau/noya-cli) | 2026-07-05 |

Strict-lockstep versioning: every satellite pins
`noyalib = "=X.Y.Z"` matching the release tag. Satellites
release simultaneously with the parent library.

---

## For Rust library consumers

### Consuming `noyalib` (the library core)

**No changes.** `noyalib` still lives in this repo and publishes
to crates.io as before:

```toml
[dependencies]
noyalib = "0.0.13"
```

### Consuming a satellite crate

Each satellite still publishes to crates.io under the same crate
name. Only the source repository changed. Update your Cargo.toml
version pin:

```toml
[dependencies]
noyalib-wasm = "0.0.13"   # crates.io — repo doesn't matter to Cargo
noyalib-mcp  = "0.0.13"
```

The [ADR-0005 strict-lockstep contract](doc/adr/0005-workspace-split.md#versioning-contract)
guarantees these versions move in lockstep with `noyalib`.

### Filing issues + reading source

Point issue reports at the satellite repo, not this one:

- `noyalib-wasm` issues → https://github.com/sebastienrousseau/noyalib-wasm/issues
- `noyalib-mcp` issues → https://github.com/sebastienrousseau/noyalib-mcp/issues

Repository-scoped issue tracking keeps AI-agent framework
maintainers' bug reports on the MCP repo, browser-integration
questions on the WASM repo, and library-parser concerns here.

---

## For JavaScript / TypeScript consumers

### `@sebastienrousseau/noyalib-wasm` (WASM bindings)

Available on [npmjs.org](https://www.npmjs.com/package/@sebastienrousseau/noyalib-wasm).
The npm scope changed from the pre-split plan of `@noyalib`
(scope not registered) to `@sebastienrousseau` (auto-scoped to
publisher). Package identity is stable from v0.0.12 onward.

```bash
npm install @sebastienrousseau/noyalib-wasm
# or
pnpm add @sebastienrousseau/noyalib-wasm
```

**JavaScript API is unchanged.** Import paths, function
signatures, and TypeScript typings are byte-for-byte identical
to what was shipped from the monorepo. See the
[satellite's Quick Start](https://github.com/sebastienrousseau/noyalib-wasm#quick-start).

### `@sebastienrousseau/noyalib-mcp` (MCP server wrapper)

Available on [npmjs.org](https://www.npmjs.com/package/@sebastienrousseau/noyalib-mcp).

```bash
npx @sebastienrousseau/noyalib-mcp
```

**MCP tool inventory is unchanged.** The server still exposes
`parse`, `format`, `get`, `set`, `validate` with the same
schemas and JSON-RPC wire format. Your Claude Desktop / Cursor
/ Continue.dev / Zed configs don't need updating.

---

## For DevOps consumers

### Container images (GHCR)

- `ghcr.io/sebastienrousseau/noyafmt` — still ships from this
  monorepo.
- `ghcr.io/sebastienrousseau/noyalib` — still ships from this
  monorepo.
- `ghcr.io/sebastienrousseau/noyalib-mcp` — **now ships from
  the satellite repo** (image path unchanged; only the source
  repo changed).

All images are cosign keyless-signed. Verify against the split
repo's OIDC identity:

```bash
cosign verify \
    --certificate-identity-regexp 'https://github.com/sebastienrousseau/noyalib-mcp/' \
    --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
    ghcr.io/sebastienrousseau/noyalib-mcp:0.0.13
```

### MCP Registry entry

`io.github.sebastienrousseau/noyalib-mcp` — identity unchanged,
publisher moved to the split repo. AI-agent hosts (Claude
Desktop, mcp.run, Glama) resolve the entry to the same
`ghcr.io/sebastienrousseau/noyalib-mcp` image.

---

## For downstream monorepo vendors

If you had a `git submodule` or `path = "…"` pointing at
`crates/noyalib-wasm/` or `crates/noyalib-mcp/` in the monorepo,
those paths **no longer exist** in the split releases (v0.0.12
onward). Migrate to one of:

### Option A — pull from crates.io (recommended)

```toml
[dependencies]
noyalib-wasm = "=0.0.13"    # exact-match pins recommended
noyalib-mcp  = "=0.0.13"
```

Same version identity, same crate identity — only the source
repo changed.

### Option B — pull from the satellite git repo

```toml
[dependencies]
noyalib-wasm = { git = "https://github.com/sebastienrousseau/noyalib-wasm", tag = "v0.0.13" }
noyalib-mcp  = { git = "https://github.com/sebastienrousseau/noyalib-mcp",  tag = "v0.0.13" }
```

### Option C — subtree-vendor the satellite

```bash
git subtree add --prefix=vendor/noyalib-wasm \
    https://github.com/sebastienrousseau/noyalib-wasm v0.0.13 --squash
```

If you were also depending on `pkg/npm-mcp-wrapper/` or
`pkg/docker/Dockerfile.mcp`, those moved to the satellite too
— see the satellite's `pkg/` directory.

---

## What did NOT change

- `noyalib` library crate: same identity, same public API, same
  MSRV (1.85). Any patch fix here still lands here.
- `noyalib` docs (docs.rs, GitHub Pages): unchanged.
- Sigstore verification recipes: unchanged (identity migrated
  to the split repos where applicable — see cosign snippets
  above).

---

## Rollback

If any split turns out to be problematic during the 14-day soak
review, the [ADR-0005 rollback recipe](doc/adr/0005-workspace-split.md#rollback-recipe)
restores the pre-split monorepo shape in ≤ 5 minutes of cargo
compilation on a warm cache. Rollback yanks the satellite
crate versions on crates.io so the monorepo re-takes precedence
transparently for downstream users.

---

## Questions?

- **Rust library / core parser questions**: file at
  https://github.com/sebastienrousseau/noyalib/issues
- **WASM / browser / bundler questions**: file at
  https://github.com/sebastienrousseau/noyalib-wasm/issues
- **MCP / AI agent tooling questions**: file at
  https://github.com/sebastienrousseau/noyalib-mcp/issues
- **Security issues**: email sebastian.rousseau@gmail.com — do
  not open a public issue.
