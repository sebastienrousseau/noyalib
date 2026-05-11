<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.4 — Release Notes

Opens the v0.0.4 cycle with the `!include` directive (issue #10)
— composable YAML documents via a user-supplied resolver, with
first-party filesystem sandboxing and cycle protection.

## Highlights

- **`!include` directive.** After parse, every
  `Value::Tagged(!include, scalar_spec)` node is replaced with
  the resolver's output. Resolvers are arbitrary
  `Fn(IncludeRequest<'_>) -> Result<InputSource>` closures — pull
  from a virtual filesystem, an HTTP cache, a key/value store, or
  the bundled `SafeFileResolver`.
- **`SafeFileResolver`** (behind `include_fs`). Filesystem-backed
  resolver rooted at a directory. Path-traversal attempts
  (`../../etc/passwd`) are caught by canonicalisation +
  root-prefix check; symlinks are governed by `SymlinkPolicy`.
- **Fragment anchors.** `!include file.yaml#key` narrows to the
  named top-level mapping key inside the included document.
- **Cycle detection.** Per-walk visited set rejects A→B→A
  regardless of depth.
- **Depth ceiling.** `ParserConfig::max_include_depth` defaults
  to 24 (8 in `ParserConfig::strict()`).

## What ships

### Two new Cargo features

| Feature | Scope | Implies |
| :--- | :--- | :--- |
| `include` | Resolver types: `IncludeResolver`, `IncludeRequest`, `InputSource`. Works in no_std-style builds. | — |
| `include_fs` | Adds `SafeFileResolver` + `SymlinkPolicy`. | `include` + `std` |

### New public API

| API | Where | Feature gate |
| :--- | :--- | :--- |
| `ParserConfig::include_resolver(resolver)` | `de.rs` | `include` |
| `ParserConfig::max_include_depth(usize)` | `de.rs` | `include` |
| `include::IncludeResolver` | `include.rs` | `include` |
| `include::IncludeRequest<'a>` | `include.rs` | `include` |
| `include::InputSource` | `include.rs` | `include` |
| `include::SafeFileResolver` | `include.rs` | `include_fs` |
| `include::SymlinkPolicy` | `include.rs` | `include_fs` |
| `include::split_fragment` | `include.rs` | `include` |

`IncludeResolver` is a newtype around
`Arc<dyn Fn(IncludeRequest<'_>) -> Result<InputSource> + Send + Sync>`
with a hand-rolled `Debug` impl. The `dyn Fn` doesn't auto-impl
`Debug` and `thiserror` / derive-`Debug` aren't applicable — the
impl is hand-rolled (matches the policy in `error.rs`). `Arc`
(not `Box`) so configs stay cheap to clone.

## What changed (besides the new APIs)

- **Streaming fast-path** is automatically disabled when an
  include resolver is installed so the post-parse walk runs
  uniformly across every typed target.
- **`ParserConfig::strict()`** now tightens `max_include_depth`
  to 8 (the default for `ParserConfig::new()` is 24),
  proportional to its other depth caps (`max_depth` 128 → 64,
  `max_alias_expansions` 1024 → 100).

## Migration from v0.0.3

No breaking changes — additive only. Drop-in upgrade:

```toml
[dependencies]
noyalib = { version = "0.0.4", features = ["include_fs"] }
```

`!include` substitution is opt-in via `ParserConfig::include_resolver`
— callers who don't install a resolver get the same behaviour
as v0.0.3 (tagged values flow through unchanged).

## Headline numbers

- **YAML 1.2 spec compliance: 100% strict** — 406/406 official
  YAML Test Suite cases pass.
- **Zero `unsafe`** workspace-wide.
- **4 000+ workspace tests + 495+ doctests + 24 new
  integration tests** for the v0.0.4 APIs.
- **96.31% function / 94.28% line / 93.38% region** coverage —
  above the 96/94/93 CI gates.
- **Five publishable crates** in lockstep at v0.0.4 —
  `noyalib`, `noya-cli`, `noyalib-mcp`, `noyalib-lsp`,
  `noyalib-wasm`.

## Compatibility

- **Public API:** additive only.
- **MSRV:** Rust **1.75.0** stable (unchanged).
- **Wire format:** unchanged.

## Verification

```bash
cargo install noya-cli --version 0.0.4
noyafmt --version
noyavalidate --version

# Cosign-verify any release artefact
cosign verify-blob \
  --certificate "noyalib-0.0.4.crate.pem" \
  --signature   "noyalib-0.0.4.crate.sig" \
  --certificate-identity-regexp \
    "^https://github.com/sebastienrousseau/noyalib/" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  noyalib-0.0.4.crate
```

Full verification recipes in [`pkg/VERIFY.md`](pkg/VERIFY.md).

## Acknowledgements

The `!include` directive design draws on the same security model
as `serde_yaml_ng`'s tag-handler dispatch (sandbox-by-default,
explicit symlink policy) — adapted to noyalib's
`Value::Tagged` substitution model so resolution stays purely
post-parse.
