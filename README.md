<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<p align="center">
  <img src="https://cloudcdn.pro/noyalib/v1/logos/noyalib.svg" alt="Noyalib logo" width="128" />
</p>

<h1 align="center">noyalib</h1>

<p align="center">
  A YAML 1.2 parser and serialiser for Rust, with full
  <code>serde</code> integration and zero <code>unsafe</code> code.
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/noyalib/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/noyalib"><img src="https://img.shields.io/crates/v/noyalib.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/noyalib"><img src="https://img.shields.io/badge/docs.rs-noyalib-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs.rs" /></a>
  <a href="https://lib.rs/crates/noyalib"><img src="https://img.shields.io/badge/lib.rs-noyalib-orange.svg?style=for-the-badge" alt="lib.rs" /></a>
  <a href="https://scorecard.dev/viewer/?uri=github.com/sebastienrousseau/noyalib"><img src="https://img.shields.io/ossf-scorecard/github.com/sebastienrousseau/noyalib?style=for-the-badge&label=OpenSSF%20Scorecard&logo=openssf" alt="OpenSSF Scorecard" /></a>
</p>

---

## Contents

**Getting started**

- [Install](#install) — Cargo, source
- [Quick Start](#quick-start) — parse and serialise in ten lines

**The noyalib ecosystem** (library + four satellite crates)

- [The noyalib ecosystem](#the-noyalib-ecosystem) — `noyalib`, `noya-cli`, `noyalib-lsp`, `noyalib-mcp`, `noyalib-wasm` at a glance

**Library reference**

- [One-minute migration from `serde_yaml` (and the wider ecosystem)](#one-minute-migration-from-serde_yaml-and-the-wider-ecosystem) — name-for-name mapping for `serde_yaml` 0.9, `serde_yml`, `yaml_serde`, `serde-yaml-ng`, `serde-norway`, `serde-yaml-bw`, `serde-saphyr`, `yaml-spanned`
- [Why this approach?](#why-this-approach) — design rationale
- [Capabilities in 0.0.1](#capabilities-in-001) — release inventory
- [Two APIs, one parser](#two-apis-one-parser) — data binding vs. tooling
- [Ecosystem comparison](#ecosystem-comparison) — short matrix; full table at [`doc/COMPARISON.md`](doc/COMPARISON.md)
- [Benchmarks](#benchmarks) — headline numbers; full table at [`doc/BENCHMARKS.md`](doc/BENCHMARKS.md)
- [Features](#features) — module-level capability list
- [Custom tags ("just data")](#custom-tags-just-data) — `Value::Tagged`, untag, registry
- [Library Usage](#library-usage) — deserialise, serialise, values, spans
- [Configuration](#configuration) — parser and serialiser options
- [Examples](#examples) — runnable example index

**Operational**

- [When not to use noyalib](#when-not-to-use-noyalib) — limitations
- [Development](#development) — make targets, fuzzing, CI
- [Security](#security) — guarantees and compliance
- [Documentation](#documentation) — all reference docs
- [License](#license)

---

## Install

### As a Rust library (crates.io)

```toml
[dependencies]
noyalib = "0.0.15"
```

### As a CLI tool

The `noyafmt` and `noyavalidate` binaries ship from the
[`noya-cli`](https://crates.io/crates/noya-cli) companion crate
(the `noyalib` library crate itself contains no binaries — the
split keeps `clap` + `miette` + `validate-schema` out of the
library's dependency graph for downstream embedders).

| Channel | Install |
|---|---|
| Cargo (crates.io) | `cargo install noya-cli --locked` |
| Cargo (from source) | `cargo install --locked --path crates/noya-cli` |
| Homebrew (personal tap) | `brew tap sebastienrousseau/tap && brew install noyalib` |
| Arch Linux (AUR) | `yay -S noyalib-bin` (binary) or `yay -S noyalib` (source) |
| Scoop (Windows) | `scoop bucket add sebastienrousseau https://github.com/sebastienrousseau/scoop-bucket && scoop install noyalib` |
| Nix / NixOS | `nix run github:sebastienrousseau/noyalib` |
| Container (GHCR) | `docker run --rm ghcr.io/sebastienrousseau/noyafmt:latest --version` |
| npm (WASM) | `npm install @sebastienrousseau/noyalib-wasm` |
| npm (MCP) | `npx @sebastienrousseau/noyalib-mcp` (no Rust toolchain needed) |
| VS Code | search `noyalib` in the Marketplace |
| Open VSX | search `noyalib` in [open-vsx.org](https://open-vsx.org) |

`cargo install noya-cli --locked` builds both binaries by default
(via the `noyavalidate` Cargo feature). To install only the
formatter and skip the schema-validation toolchain, use
`cargo install noya-cli --locked --no-default-features --features noyafmt`.

GitHub Releases additionally publish pre-built tarballs for
Linux (gnu + musl), macOS (Intel + Apple Silicon + universal),
and Windows (x86_64, i686, aarch64). Each archive ships with the
binaries, man pages, shell completions, license bundle, and a
cosign keyless signature + SLSA L3 attestation.

See [`pkg/VERIFY.md`](pkg/VERIFY.md) for verification commands
and [`pkg/PUBLISH.md`](pkg/PUBLISH.md) for the per-channel
maintainer runbook.

### `no_std` support

```toml
[dependencies]
noyalib = { version = "0.0.15", default-features = false }
```

Requires `alloc`. Core data binding (`from_str`, `to_string`, `Value`,
schemas) and the streaming deserializer all compile and run without
the standard library. `from_reader`, `to_writer`, the `Spanned<T>`
deserialization helper (which uses thread-local storage), and the CST
module require the `std` feature, which is enabled by default.

### Build from source

```bash
git clone https://github.com/sebastienrousseau/noyalib.git
cd noyalib
make          # check + clippy + test
```

**MSRV by crate.** Each workspace crate carries its own
`rust-version`; CI's `msrv-per-crate` job (Phase 7) gates each
crate independently so a satellite never silently breaks
downstream users pinned to the core's floor.

| Crate | MSRV | Why |
|---|---|---|
| `noyalib` (core lib) | **1.85.0** | The committed floor since v0.0.5 (edition 2024). Enforced by the dedicated MSRV CI job. |
| `noyalib-mcp` | 1.85.0 | Same floor; small dep tree, no transitives requiring a higher edition. |
| `noya-cli` (binaries) | 1.85.0 | `clap_builder 4.6` (a transitive of `clap = "4.5"`) ships in edition 2024. |
| `noyalib-lsp` | 1.85.0 | LSP transport-stack transitives (`litemap`, `uuid`) require recent stables. |

Optional core-lib features pull in ergonomics deps that have
themselves bumped past 1.75 — `miette` → backtrace 1.82+,
`garde` → 1.84+, `validate-schema` / `figment` → ICU chain
1.86+, `parallel` → rayon-core 1.80+. Use those with a current
stable toolchain; the core lib stays buildable on the Ubuntu
24.04 LTS rustc-1.75 floor.

`rust-toolchain.toml` itself selects `stable` for local
development; the 1.75.0 floor on the core surface is enforced
by the dedicated `msrv-1-75-core` CI job (Ubuntu,
no-default-features + default-features build paths).

### Cargo features

All optional integrations are off by default. Enable only what
the application needs.

| Feature | Pulls in | Adds | Documented in |
| :--- | :--- | :--- | :--- |
| `std` *(default)* | — | `from_reader`, `to_writer`, `Spanned<T>`, CST module | [Install](#install) |
| `miette` | `miette` 7 | Rich terminal diagnostics with source spans | [Library Usage](#library-usage), `examples/diagnostic.rs` |
| `schema` | `schemars`, `serde_json` | `JsonSchema` derive + `schema_for::<T>()`. **Downstream callers that derive `JsonSchema` must add `schemars = "1.2"` to their own `Cargo.toml`** — the proc-macro emits `::schemars::*` paths that need to resolve in the call-site dep graph. | [Capabilities in 0.0.1](#capabilities-in-001) |
| `validate-schema` | `schema` + `jsonschema` | `validate_against_schema`, `coerce_to_schema` | [Governance: schema-driven autofix](#governance-schema-driven-autofix) |
| `figment` | `figment` 0.10 | `noyalib::figment::Yaml` provider | `examples/figment.rs` |
| `garde` | `garde` 0.22 | `Validated<T>` wrapper | `examples/validation_garde.rs` |
| `validator` | `validator` 0.19 | `ValidatedValidator<T>` wrapper | `examples/validation_validator.rs` |
| `robotics` | — | `Degrees`, `Radians`, `StrictFloat` newtypes | `examples/robotics_polymorphism.rs` |
| `parallel` | `rayon` 1.10 | `noyalib::parallel::parse<T>` for `---`-separated streams | [Benchmarks](#benchmarks) |
| `recovery` | — | `noyalib::recovery::parse_lenient` — best-effort tree + error list for LSP / IDE half-typed documents | `examples/recovery_lenient.rs`, `benches/v006_features.rs` |
| `sval` | `sval` 2 | `impl sval::Value` for `Value` / `Number` / `Mapping` / `MappingAny` / `TaggedValue`, `noyalib::sval_adapter::to_sval_writer` | `examples/sval_streaming.rs`, `benches/v006_features.rs` |
| `tokio` | `tokio`, `tokio-util`, `bytes` | `noyalib::tokio_async::from_async_reader` / `from_async_reader_multi` and `YamlDecoder` codec for `tokio_util::codec::Framed` pipelines | `examples/tokio_async_reader.rs`, `benches/v006_features.rs` |
| `simd` | — | `noyalib::simd::*` primitives + parser hot path | [Benchmarks](#benchmarks) |
| `nightly-simd` | `simd` (nightly toolchain) | `core::simd`-backed `StructuralIter` (32-byte chunks) | [Benchmarks](#benchmarks) |
| `compat-serde-yaml` | — | `noyalib::compat::serde_yaml` shim for migration | [When not to use noyalib](#when-not-to-use-noyalib) |
| `lossless-u64` | — | `Number::Unsigned(u64)` plus opt-in parser/serializer config for scalars in `(i64::MAX, u64::MAX]` | [`doc/adr/0004-lossless-u64-integers.md`](doc/adr/0004-lossless-u64-integers.md), `examples/lossless_u64.rs`, `benches/lossless_u64.rs` |
| `compare-saphyr` | `serde-saphyr` *(dev only)* | Cross-library bench comparison arms | `benches/comparison.rs` |
| `noyavalidate` | `std` + `miette` + `validate-schema` | The `noyavalidate` CLI binary | [Tooling](#tooling) |

```toml
# Example: rich diagnostics + schema validation
[dependencies]
noyalib = { version = "0.0.15", features = ["miette", "validate-schema"] }
```

**Optional features:** `lossless-u64` preserves YAML integer scalars above
`i64::MAX` as `Number::Unsigned(u64)` instead of lossy `f64` widening —
useful for distributed-system IDs, content hashes, and timestamp fields.
Enable the Cargo feature, then opt in at runtime with
`ParserConfig::lossless_u64_integers(true)` and
`SerializerConfig::lossless_u64_integers(true)`. See
[`doc/adr/0004-lossless-u64-integers.md`](doc/adr/0004-lossless-u64-integers.md)
for rationale and migration notes.

---

## Quick Start

```rust
use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Config {
    name: String,
    port: u16,
    features: Vec<String>,
}

fn main() -> Result<(), noyalib::Error> {
    let yaml = "
name: myapp
port: 8080
features:
  - auth
  - api
";

    let config: Config = from_str(yaml)?;
    let output = to_string(&config)?;
    let roundtrip: Config = from_str(&output)?;
    assert_eq!(config, roundtrip);

    Ok(())
}
```

---

## The noyalib ecosystem

Five crates ship from this workspace. The library is the core;
the four satellites wrap it for specific delivery surfaces.

| Crate | What it is | Use case |
|---|---|---|
| **`noyalib`** | Library — YAML 1.2 parser, serializer, lossless CST, JSON Schema validator | Embed YAML support in any Rust binary or library. |
| **`noya-cli`** ([own repo](https://github.com/sebastienrousseau/noya-cli)) | Two binaries: `noyafmt` (formatter), `noyavalidate` (schema validator + autofixer) | CI gates, pre-commit hooks, ad-hoc command-line use. |
| **`noyalib-lsp`** ([own repo](https://github.com/sebastienrousseau/noyalib-lsp)) | Language Server Protocol server | Editor integration — VS Code, Neovim, Helix, Emacs, Zed, Sublime, IntelliJ. |
| **`noyalib-mcp`** ([own repo](https://github.com/sebastienrousseau/noyalib-mcp)) | Model Context Protocol server | LLM agent tooling — Claude Desktop, Cursor, Continue.dev, Zed assistant, mcp.run. |
| **`noyalib-wasm`** ([own repo](https://github.com/sebastienrousseau/noyalib-wasm)) | `wasm-bindgen` wrapper around the library | Browser, Node, Cloudflare Workers, Deno, any WASM-capable host. |

### Install the binaries

```bash
# CLI tools (noyafmt + noyavalidate)
cargo install noya-cli

# LSP server
cargo install noyalib-lsp

# MCP server
cargo install noyalib-mcp

# WASM bundle
npm install @sebastienrousseau/noyalib-wasm
```

Per-crate READMEs cover the surface specific to each artifact:

- **CLI**: [`crates/noya-cli/README.md`](crates/noya-cli/README.md) — flags, exit codes, recipes.
- **LSP**: [`crates/noyalib-lsp/README.md`](crates/noyalib-lsp/README.md) — capabilities, editor configs.
- **MCP**: [`sebastienrousseau/noyalib-mcp`](https://github.com/sebastienrousseau/noyalib-mcp) — tools, host configs (split repo since v0.0.13; strict lockstep per [ADR-0005](doc/adr/0005-workspace-split.md)).
- **WASM**: [`sebastienrousseau/noyalib-wasm`](https://github.com/sebastienrousseau/noyalib-wasm) — JS API, bundling (split repo since v0.0.12).

### Per-host quick links

| If you use… | Drop-in config |
|---|---|
| **VS Code / JetBrains / Neovim / Helix / Emacs / Zed / Sublime** | [editor configs in `noyalib-lsp/examples/`](crates/noyalib-lsp/examples/) |
| **Claude Desktop / Cursor / Continue.dev / Zed assistant / hosted MCP** | [client configs (noyalib-mcp repo)](https://github.com/sebastienrousseau/noyalib-mcp/tree/main/examples) |
| **GitHub Actions / pre-commit / Helm / Compose / pyproject-adjacent YAML** | [validation gates in `noya-cli/examples/`](crates/noya-cli/examples/) |
| **Vite / Webpack / Next.js / Cloudflare Workers / Deno / Bun** | [bundling guide (noyalib-wasm repo)](https://github.com/sebastienrousseau/noyalib-wasm/blob/main/doc/bundling.md) |

The rest of this README covers the **library** surface
(`noyalib` itself). For the satellite crates, jump straight to
their READMEs above.

---

## One-minute migration from `serde_yaml` (and the wider ecosystem)

Most call sites are mechanical to update. The full guide —
covering `serde_yaml` 0.9 plus every actively-published fork
and adjacent crate — is
[`doc/MIGRATION-FROM-SERDE-YAML.md`](doc/MIGRATION-FROM-SERDE-YAML.md).
The headline mapping for `serde_yaml` 0.9 is below; the same
guide has per-crate sections for `serde_yml`, `yaml_serde`,
`serde-yaml-ng`, `serde-norway`, `serde-yaml-bw`,
`serde-saphyr`, and `yaml-spanned` with verified function
tables for each.

```diff
-[dependencies]
-serde_yaml = "0.9"
+[dependencies]
+noyalib = "0.0.15"
```

```diff
-use serde_yaml::Value;
-let v: Value = serde_yaml::from_str(input)?;
-let s        = serde_yaml::to_string(&v)?;
+use noyalib::Value;
+let v: Value = noyalib::from_str(input)?;
+let s        = noyalib::to_string(&v)?;
```

| `serde_yaml` 0.9 | `noyalib` |
|---|---|
| `serde_yaml::from_str::<T>` | `noyalib::from_str::<T>` |
| `serde_yaml::from_slice::<T>` | `noyalib::from_slice::<T>` |
| `serde_yaml::from_reader::<R, T>` | `noyalib::from_reader::<R, T>` |
| `serde_yaml::to_string` | `noyalib::to_string` |
| `serde_yaml::to_writer` | `noyalib::to_writer` |
| `serde_yaml::to_value` | `noyalib::to_value` |
| `serde_yaml::Value` | `noyalib::Value` (adds a 7th `Tagged` variant) |
| `serde_yaml::Mapping` | `noyalib::Mapping` |
| `serde_yaml::Number` | `noyalib::Number` |
| `serde_yaml::Error` | `noyalib::Error` |
| `serde_yaml::with::singleton_map*` | `noyalib::with::singleton_map*` |
| (n/a) | `noyalib::from_str_strict::<T>` — error on unknown keys |
| (n/a) | `noyalib::Spanned<T>` — source-location wrapper |
| (n/a) | `noyalib::cst::Document` — lossless byte-faithful edits |

If your call sites can't change at all, enable
`features = ["compat-serde-yaml"]` and replace `use serde_yaml`
with `use noyalib::compat::serde_yaml` — every type is
noyalib-native, no transitive dep on the archived upstream.

### Coming from a different YAML crate?

Each crate has a standalone migration guide with TL;DR diff,
function-mapping table, behavioural notes, and a checklist.
Crates.io state verified **2026-05-08**:

| Crate | Version | Drop-in for `serde_yaml`? | Migration guide |
|---|---|---|---|
| [`serde_yml`](https://crates.io/crates/serde_yml) | `0.0.12` (archived 2025-09) | mostly | [`MIGRATION-FROM-SERDE-YML.md`](doc/MIGRATION-FROM-SERDE-YML.md) |
| [`yaml_serde`](https://crates.io/crates/yaml_serde) | `0.10.4` | yes (Cargo `package =` rename) | [`MIGRATION-FROM-YAML-SERDE.md`](doc/MIGRATION-FROM-YAML-SERDE.md) |
| [`serde-yaml-ng`](https://crates.io/crates/serde-yaml-ng) | `0.10.0` | yes | [`MIGRATION-FROM-SERDE-YAML-NG.md`](doc/MIGRATION-FROM-SERDE-YAML-NG.md) |
| [`serde-norway`](https://crates.io/crates/serde-norway) | `0.9.42` | yes | [`MIGRATION-FROM-SERDE-NORWAY.md`](doc/MIGRATION-FROM-SERDE-NORWAY.md) |
| [`serde-yaml-bw`](https://crates.io/crates/serde-yaml-bw) | `2.5.6` | **no** (breaking 2.x; 8-variant `Value` with `Alias`) | [`MIGRATION-FROM-SERDE-YAML-BW.md`](doc/MIGRATION-FROM-SERDE-YAML-BW.md) |
| [`serde-saphyr`](https://crates.io/crates/serde-saphyr) | `0.0.26` | **no** (no `Value` DOM, streaming-only) | [`MIGRATION-FROM-SERDE-SAPHYR.md`](doc/MIGRATION-FROM-SERDE-SAPHYR.md) |
| [`yaml-spanned`](https://crates.io/crates/yaml-spanned) | `0.0.3` | **no** (parser-only, no `to_string`) | [`MIGRATION-FROM-YAML-SPANNED.md`](doc/MIGRATION-FROM-YAML-SPANNED.md) |

The umbrella index is
[`doc/MIGRATION.md`](doc/MIGRATION.md) — start there if you're
not sure which guide applies, or pick the row above.

The three behavioural differences worth knowing about
(YAML 1.2 strict booleans, `Tagged` variant, multi-doc API):
[`MIGRATION-FROM-SERDE-YAML.md`](doc/MIGRATION-FROM-SERDE-YAML.md)
covers each in detail.

---

## Why this approach?

noyalib targets the niche `serde_yaml` / `serde_yml` / `libyml`
occupy — read YAML into typed Rust structs, write Rust structs back
as YAML — and is written from scratch against the YAML 1.2 spec.
The implementation runs the official YAML test suite to **100%
strict compliance — 387/387 attempted cases pass, 0 failures**;
19 cases are deliberately skipped (tracked alongside the suite in
[`tests/yaml_compliance_report.rs`](crates/noyalib/tests/yaml_compliance_report.rs)
so the gap is explicit). It is not a fork of `serde_yaml`; the parser,
scanner, serialiser, and CST are independent code.

Two architectural choices motivate the rewrite:

1. **Streaming-first deserialise.** The default `from_str<T>` path
   walks parser events directly into the typed target. The
   `serde_yaml`-shaped pattern is `parse → Value →
   T::deserialize(&Value)`, which allocates every key, scalar, and
   nested mapping into an intermediate AST that the typed target
   then throws away. noyalib bypasses that AST when the caller
   asked for a typed `T`. The dynamic `Value` tree is still
   available when callers want it, but it is no longer in the hot
   path.

2. **`#![forbid(unsafe_code)]` at the workspace root.** There is no
   FFI to a C library, no raw-pointer dereferences, and no
   `unsafe` blocks in the parser, scanner, formatter, or CST. CI
   enforces the attribute on every push. Most popular Rust YAML
   crates wrap `libyaml` via C-FFI; the `unsafe` blocks involved
   are usually well-vetted, but their existence makes a
   security-conscious downstream audit meaningfully harder.

A few features built on top of those choices:

- **SIMD-accelerated structural discovery.** Stable Rust dispatches
  to `memchr` (SSE2 / NEON) for arity-1/2/3 needles and SWAR for
  arity-4+. With `nightly-simd` on, the structural-bitmask scanner
  widens to a 32-byte `Simd<u8, 32>` chunk and walks delimiters via
  `mask.trailing_zeros()` — the same shape that powers `simdjson`.
- **SWAR decimal parsing.** The plain-scalar integer resolver folds
  8 ASCII digits per `u64` cycle via three pair-wise multiply-add
  phases. ~2× faster than `<i64 as FromStr>::from_str` on big
  numbers.
- **Lossless CST.** A side-table green tree (`noyalib::cst::Document`)
  reproduces the source byte-for-byte and supports surgical edits.
  `doc.set("version", "0.0.2")` rewrites only the touched span;
  comments, indentation, and sibling entries are left alone.

The default profile compiles **eight crates**: five
unconditional (`serde`, `indexmap`, `rustc-hash`, `memchr`,
`smallvec`) plus three default-on but optional (`itoa` via
`fast-int`, `ryu` via `fast-float`, `serde_ignored` via
`strict-deserialise`). Disabling the default features drops the
graph to the five unconditional crates. **No archived or
unmaintained crate appears in the graph** — `serde_yaml` 0.9
(archived), `libyaml` (C-FFI), and `thiserror` are all absent.
`cargo audit`, `cargo deny`, and `cargo vet` are CI gates on
every push.

---

## Capabilities in 0.0.1

The 0.0.1 release covers a complete YAML 1.2 stack. See
[`CHANGELOG.md`](CHANGELOG.md) for the detailed inventory; the
table below groups the inventory by capability theme.

| Theme | Headline deliverables |
| :--- | :--- |
| Spec compliance | YAML 1.2 official test suite at 100% strict (387/387 attempted, 0 failures, 19 deliberately skipped — gap tracked in `tests/yaml_compliance_report.rs`); YAML 1.1 opt-in compatibility for the "Norway problem"; multi-document streams |
| Migration from `serde_yaml` | `compat-serde-yaml` feature with name-for-name re-exports; `From`/`TryFrom` parity for `Value`/`Mapping`/`Number`; `Document::validate`; comment-aware reads via `load_comments` |
| Binary scalars | First-class `!!binary` tag; RFC 4648 base64 round-trip with `serde_bytes::ByteBuf`/`Bytes`; non-UTF-8 payloads supported |
| Flatten guard | `Spanned<Value>` in `#[serde(flatten)]` returns an actionable error pointing at the working alternative |
| Lossless editing | Side-table CST with byte-faithful round-trip; `Document::entry(path)` chainable mutable handle (12 methods); automatic indent detection (2/3/4-space) |
| Anchor management | `Document::anchors()` / `aliases()` / `aliases_of(name)`; `materialise_alias_at(byte_pos)` and `materialise_aliases_of(name)` for breaking aliases |
| Schema codegen | `schema` feature: `JsonSchema` derive re-export; `schema_for::<T>()` and `schema_for_yaml::<T>()`; honours `#[doc]`, `#[serde(default)]`, `#[serde(rename)]` |
| Schema validation | `validate-schema` feature: `validate_against_schema(value, schema)`; aggregated violations with RFC 6901 paths |
| Tooling | `noyavalidate` (with `--schema` and `--fix`), `noyafmt`, `noyalib-mcp`, `noyalib-wasm` |
| Performance | `noyalib::simd` primitives — `find_any_of`, `clean_prefix_len`, `ByteBitmap`; parser hot path integrated; ~58× and ~5.4× over byte-by-byte at arities 3 and 8 |
| Supply chain | SLSA L3 provenance, sigstore signing, OpenSSF Scorecard, REUSE.software 3.3 compliance, signed commits, `cargo-deny` / `cargo-vet` / `cargo-semver-checks` gates, differential and soak fuzz |

---

## Two APIs, one parser

noyalib exposes two complementary surfaces over the same scanner and strictness rules:

- **Data binding** — `from_str`, `to_string`, `Value`, `StreamingDeserializer`, `BorrowedValue`. Read YAML into typed Rust data, write Rust data back out. The round-trip travels through a `Value`/struct, so comments and exact whitespace are not preserved. Use this for config loaders, RPC payloads, and the 95% of YAML workloads that just want data.
- **Tooling / automation** — `noyalib::cst::parse_document`, `parse_stream`, and the `Document` handle. Read YAML into a side-table CST that reproduces the source byte-for-byte, then run targeted edits like `doc.set("version", "0.0.2")` — only the touched span is rewritten, every comment and the original indentation is left alone. Use this for Renovate-style version bumps, manifest patchers, formatters, and schema-driven linters. See `examples/lossless_edit.rs`.

The CST also lets a tool break aliases by inlining the anchored
content at every reference site — useful when a manifest needs to
become self-contained before being shipped to a system that does
not resolve YAML aliases:

```rust
use noyalib::cst::parse_document;

let yaml = "a: &shared 7\nb: *shared\nc: *shared\n";
let mut doc = parse_document(yaml).unwrap();

// Replace every `*shared` reference with the bytes of the
// anchored value. The `&shared` declaration stays in place.
let n = doc.materialise_aliases_of("shared").unwrap();
assert_eq!(n, 2);
assert!(!doc.to_string().contains('*'));
```

`Document::materialise_alias_at(byte_pos)` is the single-site
variant for callers that already know the alias's source position.

---

## Ecosystem comparison

`noyalib` is the only Rust YAML implementation that **passes
all 406 active YAML 1.2 Test Suite cases under strict
comparison** and ships a lossless CST, native LSP, MCP server,
WASM bundle, JSON Schema validator, and schema-driven autofix
in one workspace.

The full feature matrix — every row, every column, with the
reading-the-table notes — lives at
**[`doc/COMPARISON.md`](doc/COMPARISON.md)** so the README
stays fast to scan.

Quick orientation:

| Crate | Drop-in for `serde_yaml`? | Key gap vs noyalib |
|---|---|---|
| `serde_yaml` 0.9 | (archived 2024-03) | unmaintained |
| `serde_yml`, `serde-yaml-ng`, `serde-norway`, `yaml_serde` | yes (path rename) | no streaming deser, no CST, no LSP / MCP / WASM, no JSON Schema |
| `serde-yaml-bw` | no (breaking 2.x) | no LSP / MCP / WASM, no JSON Schema |
| `serde-saphyr` | no (no `Value` DOM) | no `Value` DOM, no CST, no LSP / MCP / WASM |
| `yaml-spanned` | no (read-only) | no serializer, no CST, no LSP / MCP / WASM |

Per-crate migration guides at
[`doc/MIGRATION.md`](doc/MIGRATION.md).

---

## Benchmarks

Headline numbers (Apple M4 / aarch64, Rust 1.95 stable,
`--release` with LTO=fat, codegen-units=1, panic=abort,
criterion `--warm-up-time 2 --measurement-time 4`):

| Fixture | noyalib | vs `serde_yaml_ng` | vs `yaml-rust2` | vs `yaml-spanned` | vs `serde-saphyr` |
|---|---:|---:|---:|---:|---:|
| Deserialise simple (3 fields) | **1.40 µs** | **1.84×** | 1.36× | 1.69× | **2.00×** |
| Deserialise nested (20 fields) | **9.66 µs** | **1.55×** | 1.25× | **1.60×** | **1.76×** |
| Deserialise large_list (500 items) | **920 µs** | **1.42×** | 1.19× | **1.38×** | **1.69×** |
| Deserialise github_actions (deep + comments) | **46.4 µs** | **1.66×** | 1.25× | **1.74×** | **1.73×** |
| Deserialise k8s multi-document | **85.1 µs** | **1.42×** | 1.11× | — | — |
| Typed deserialise simple (streaming) | **1.22 µs** | **1.72×** | — | — | — |
| Typed deserialise nested (streaming) | **7.08 µs** | **1.55×** | — | — | — |
| Serialise simple | **290 ns** | **4.34×** | — | — | — |
| Serialise nested | **2.25 µs** | **3.00×** | — | — | — |
| Round-trip nested | **12.0 µs** | **1.83×** | — | — | — |
| Structural-discovery (1 MiB, nightly-SIMD) | **311 µs** | — | — | — | 9.2× over memchr loop |
| SWAR decimal parse (`i64::MAX`) | **9.75 ns** | — | — | — | 2.5× over stdlib |
| Lossless-u64 parser resolution (`benches/lossless_u64.rs`) | see harness | — | — | — | knob on vs off for large unsigned scalars |

(`serde_yml` was previously a comparison column — retired in
v0.0.6 since RUSTSEC-2025-0068 flagged it unsound + unmaintained;
the bench arm was dropped so the OpenSSF Scorecard Vulnerabilities
check stays clean. The headline ratios above are unchanged for
the remaining four competitors.)

`noyalib` is faster than **every other pure-Rust YAML library on
every deserialize fixture measured**. Speedup ranges across the four
competitors above: **1.69×–2.00×** vs `serde-saphyr`,
**1.42×–1.84×** vs `serde_yaml_ng`,
**1.38×–1.74×** vs `yaml-spanned`, **1.11×–1.36×** vs `yaml-rust2`.
Serialize is **3.00×–4.34×** ahead of `serde_yaml_ng`.

The narrowest gap is `yaml-rust2`, which doesn't carry the
`Spanned<T>` plumbing, the per-tag `Cow<'a, str>` propagation, or
the `Value::Tagged` preservation that `noyalib` does — closing
the remaining gap to ≥ 2× over `yaml-rust2` is a separate effort
because the levers needed (`CompactString` keys in `Mapping`,
bump-arena event lifetimes, eliminating the `Value` AST on the
typed path) require SemVer-breaking refactors.

`cargo xtask pgo-build` runs the LLVM profile-guided optimisation
pipeline and adds 5–15% on top of the numbers above; recommended
for production deployments.

The full breakdown — every workload, every comparison library,
the SWAR pipeline explanation, parallel multi-doc scaling, and
the project-metrics table — lives at
**[`doc/BENCHMARKS.md`](doc/BENCHMARKS.md)**.

Algorithmic-complexity guarantees (`O(n)` parser, `O(d)`
stack, etc.) live in
[`POLICIES.md` §4](doc/POLICIES.md#4-performance--algorithmic-complexity).

---

## Features

| | |
| :--- | :--- |
| **Serde** | `from_str`, `from_slice`, `from_reader`, `to_string`, `to_writer`, `to_fmt_writer` -- all with `_with_config` variants. `to_value`, `from_value` for Value conversion. Multi-document: `load_all`, `load_all_as`, `to_string_multi`. Streaming deserializer bypasses Value AST for typed targets. |
| **Values** | 7-variant `Value` enum: Null, Bool, Number, String, Sequence, Mapping, Tagged. Path traversal via `get_path("server.host")`. Path queries via `query("items[*].name")` with wildcards (`*`) and recursive descent (`..`). Deep merge via `merge()` and `merge_concat()`. `MappingAny` for non-string keys. Zero-copy `BorrowedValue<'a>` borrows strings from input (18% faster). |
| **Spans** | `Spanned<T>` tracks line, column, and byte offset for every deserialized field. Serializes transparently as `T`. Span tracking is opt-in — disabled by default in `from_str` for zero overhead. For large documents, consider the memory impact of the span HashMap. |
| **Formatting** | Per-value output control: `FlowSeq<T>`, `FlowMap<T>`, `LitStr`, `FoldStr`, `Commented<T>`, `SpaceAfter<T>`. |
| **Enums** | `singleton_map`, `singleton_map_optional`, `singleton_map_recursive`, `singleton_map_with` -- custom key transforms (snake\_case, kebab-case, lowercase). |
| **Schemas** | Validate against YAML schema levels: `validate_yaml_failsafe_schema`, `validate_yaml_json_schema`, `validate_yaml_core_schema`. |
| **Anchors** | Anchors (`&`), aliases (`*`), and merge keys (`<<`). Smart pointer wrappers: `RcAnchor`, `ArcAnchor`, `RcWeakAnchor`, `ArcWeakAnchor`. |
| **Security** | 7 configurable limits in `ParserConfig`: depth, document size, alias expansions, mapping keys, sequence length, duplicate key policy, strict booleans. `ParserConfig::strict()` for untrusted input. Billion-laughs safe via `max_alias_expansions` with `saturating_add` overflow protection. |
| **Compat** | YAML 1.1 legacy boolean mode (`legacy_booleans`): resolves `yes`/`no`/`on`/`off`/`y`/`n` as booleans for Docker Compose, GitHub Actions, and other YAML 1.1 tooling. Solves the "Norway problem". |
| **WASM** | Compiles to `wasm32-unknown-unknown`. wasm-bindgen bindings (camelCase per JS conventions): `parse()`, `stringify()`, `getPath()`, `validateJson()`, `merge()`, plus the `WasmDocument` class (`toString()`, `get()`, `getSource()`, `set()`, `setValue()`, `spanAt()`, `commentsAt()`, `replaceSpan()`). Browser demo included. |
| **Errors** | Source locations on all parse errors. `format_with_source()` renders rustc-style diagnostics with `-->` pointer. `#[track_caller]` on all Index panics. `miette::Diagnostic` integration included (`--features miette`) for rich terminal reports with error codes, actionable help text, and source spans. |
| **no\_std** | Full `#![no_std]` support with `alloc`. Use `default-features = false`. Core parsing (`from_str`, `to_string`, `Value`, schemas) works without `std`. I/O functions (`from_reader`, `to_writer`), `Spanned<T>` deserialization (TLS), and the CST module require the `std` feature. CI enforces `cargo check --no-default-features` on every push. |
| **CST editing** | Side-table CST (`noyalib::cst`) for byte-faithful round-tripping. `Document::set("server.port", "9090")` rewrites only the touched bytes; comments, blank lines, and sibling formatting survive. `Document::entry(path)` is the chainable mutable handle (16 methods covering set / set_value / remove / insert / insert_value / push_back / insert_after / and_modify / or_insert / or_insert_with / or_insert_value / get / span_at / comments / exists / nested entry, plus smart `items[0]` path composition). `Document::indent_unit()` detects 2-/3-/4-space conventions so inserts conform to the file's existing style. |
| **Anchors v2** | `Document::anchors()` / `aliases()` / `aliases_of(name)` enumerate every `&name` / `*name` lexeme in source order. `Document::materialise_alias_at(byte_pos)` and `materialise_aliases_of(name)` "break" an alias by inlining the anchored scalar's source bytes — leaves the alias site independent of future anchor edits. |
| **Schema codegen** | `schema` feature: derive `JsonSchema` (re-exported from `schemars`), then `schema_for::<T>() -> Result<Value>` or `schema_for_yaml::<T>() -> Result<String>` to emit the JSON Schema 2020-12 document. Honours `#[doc]`, `#[serde(default)]`, `#[serde(rename)]`, integer bounds, nested types via `$defs`. |
| **Schema validation** | `validate-schema` feature (implies `schema`): `validate_against_schema(value, schema) -> Result<()>` enforces a JSON Schema 2020-12 contract on parsed YAML. Multiple violations aggregated with RFC 6901 JSON-pointer paths. `validate_against_schema_str` is the raw-text convenience. |
| **Binary scalars** | First-class `!!binary` tag with RFC 4648 base64 round-trip. `serde_bytes::ByteBuf` / `Bytes` work end-to-end including non-UTF-8 payloads. |
| **`serde_yaml` shim** | `compat-serde-yaml` feature: name-for-name re-exports backed by noyalib-native types — **the unmaintained `serde_yaml` 0.9 crate is intentionally not a dependency**. Migrating in-flight `::serde_yaml::Value` from un-migrated modules flows through the Serde bridge: `noyalib::to_value(&upstream)?`. |
| **SIMD primitives** | `noyalib::simd::find_any_of` / `clean_prefix_len` / `SimdScanner` / `StructuralIter` / `ByteBitmap` / `parse_decimal_{u64,i64}`. Parser hot path routes through them for free; public for downstream scanner authors. **`StructuralIter`** delivers 4.2× stable / 9.2× nightly-simd vs the memchr loop on 1 MiB workloads. |
| **Parallel parsing** | `parallel` feature: `noyalib::parallel::parse<T>` and `noyalib::parallel::values` deserialise multi-document streams across the Rayon thread pool. Pre-scan in `O(input_len)`; per-document work parallelises naturally. Linear-with-cores on `---`-separated logs / audit dumps / Kubernetes snapshots. |
| **Pluggable policies** | `noyalib::policy::Policy` trait + `ParserConfig::with_policy(p)`. Built-ins: `DenyAnchors` (rejects `&name` / `*name` — billion-laughs guard), `DenyTags` (rejects custom tags), `MaxScalarLength(n)` (caps individual scalar size). Custom policies implement the trait. |
| **Schema autofix** | `validate-schema` feature: `coerce_to_schema(value, schema) -> Result<usize>` walks JSON Schema type-mismatch errors and rewrites string-shaped scalars into the schema's expected type when the parse succeeds. Solves the `port: "8080"` quoting slip-up automatically. Library engine behind `noyavalidate --fix`. |
| **Key interner** | `noyalib::interner::KeyInterner` — `&str → Arc<str>` deduplication for repeated-key workloads. Kubernetes-shaped streams with 20-byte keys × 10 000 records: footprint drops from ~200 KB of fresh allocations to ~20 B + Arc pointers. |

---

## Governance: schema-driven autofix

YAML written by hand often has type slips that strict validation
catches but humans don't notice — `port: "8080"` (string) where
the schema declares `port: integer`. noyalib ships a one-call
fix-up engine that rewrites these to match the schema:

```rust
use noyalib::{coerce_to_schema, from_str, schema_for, validate_against_schema, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
struct ServerConfig {
    /// Port the server binds on.
    port: u16,
    /// Hostname or IP literal.
    host: String,
}

let schema = schema_for::<ServerConfig>().unwrap();

// Hand-written YAML with the port quoted by mistake.
let mut data = from_str("port: \"8080\"\nhost: api.example.com\n").unwrap();

// Strict pass — fails because `port` is a string.
assert!(validate_against_schema(&data, &schema).is_err());

// Apply schema-driven coercions.
let n = coerce_to_schema(&mut data, &schema).unwrap();
assert_eq!(n, 1, "one fix expected");

// Re-validation passes — port is now an integer.
validate_against_schema(&data, &schema).unwrap();
```

Library engine behind `noyavalidate --fix`. Handles `String →
Integer`, `String → Number`, `String → Boolean` coercions; the
fix-loop iterates until convergence. Unparseable inputs (e.g.
`port: "abc"` against `type: integer`) are left in place so the
caller can surface the residue via a follow-up
`validate_against_schema` call.

---

## Policy enforcement (Safe YAML)

The pluggable [`policy::Policy`] trait + `ParserConfig::with_policy(p)`
lets you reject documents that violate organisational constraints
*at parse time*, before any data flows to downstream code. Built-in
policies cover the most common asks; custom policies implement the
trait directly.

```rust
use noyalib::policy::DenyAnchors;
use noyalib::{from_str_with_config, ParserConfig, Value};

let cfg = ParserConfig::new().with_policy(DenyAnchors);

// Anchors / aliases are the classical billion-laughs vector and a
// readability hazard in audited configs. With `DenyAnchors`
// registered, any document that defines `&name` or dereferences
// `*name` is rejected before deserialise begins.
let res: Result<Value, _> =
    from_str_with_config("key: &x 1\nval: *x\n", &cfg);
assert!(res.is_err());

// Anchor-free input passes through unchanged.
let v: Value = from_str_with_config("key: 1\nval: 1\n", &cfg).unwrap();
assert!(matches!(v, Value::Mapping(_)));
```

Built-ins: `DenyAnchors`, `DenyTags` (rejects custom tags while
preserving YAML 1.2 core tags), `MaxScalarLength(n)`. Compose
freely — `cfg.with_policy(DenyAnchors).with_policy(DenyTags)`
runs both in registration order.

[`policy::Policy`]: https://docs.rs/noyalib/latest/noyalib/policy/trait.Policy.html

---

## Strict deserialise (typo detection)

`noyalib::from_str_strict<T>` errors if the YAML carries any keys
the target type `T` doesn't declare — closing the silent-data-loss
gap when a config-key typo (e.g. `replicass: 3`) deserialises into
a struct whose `replicas` field stays at its `Default`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    port: u16,
    host: String,
}

let yaml = "port: 8080\nhost: api.example.com\nporrt: 9090\n";

// Lenient path silently drops `porrt`.
let cfg: Config = noyalib::from_str(yaml).unwrap();
assert_eq!(cfg.port, 8080);

// Strict path surfaces the typo as a typed error.
let err = noyalib::from_str_strict::<Config>(yaml).unwrap_err();
assert!(err.to_string().contains("porrt"));
```

The strict path walks nested structs — a typo at `server.unknown`
is reported with its parent path so the user sees exactly where
the bad key lives.

The same check is available on every input shape:

| Input shape | Lenient | Strict |
| :--- | :--- | :--- |
| `&str` | `from_str` | `from_str_strict` |
| `&[u8]` | `from_slice` | `from_slice_strict` |
| `impl io::Read` | `from_reader` | `from_reader_strict` |

The slice and reader variants share `from_str_strict`'s semantics —
they exist so callers that already hold bytes (a buffer, a
network frame, a `bytes::Bytes`) don't have to round-trip through
`String` to opt in.

---

## Diagnostics output

Every parse error carries an exact `(line, column, byte offset)`
triple. `Error::format_with_source(input)` renders a rustc-style
snippet without pulling in any extra dependency:

```text
error: expected ',' or ']'
  --> input.yaml:2:7
   |
 1 | host: localhost
 2 | port: [broken
   |       ^^^^^^ here
 3 | db: postgres
   |
```

Enabling `--features miette` makes `noyalib::Error` implement
[`miette::Diagnostic`](https://docs.rs/miette/latest/miette/trait.Diagnostic.html);
wrapping the error in `miette::Report` gives terminals with ANSI
support the same snippet with a colour-coded label, and lets a
CLI surface error codes (`noyalib::parse`,
`noyalib::duplicate_key`, …) and actionable help text from the
same call site:

```rust,ignore
let cfg: Config = noyalib::from_str(yaml)
    .map_err(|e| miette::Report::new(e).with_source_code(yaml.to_owned()))?;
```

See `examples/diagnostic.rs` for the full integration pattern.

For sinks with hard length budgets — Slack messages, Sentry tags,
structured-log fields — the same diagnostic is available capped
at a caller-supplied character count:

```rust
use noyalib::{from_str, Value};
let source = "a: [unclosed";
let err = from_str::<Value>(source).unwrap_err();

// Cap at 60 chars; UTF-8-aligned cut; ASCII `...` ellipsis.
let short = err.format_with_source_truncated(source, 60);
assert!(short.len() <= 60);

// Multi-line context (rustc-style) capped at 200 chars:
let with_ctx = err.format_with_source_radius_truncated(source, 1, 200);
assert!(with_ctx.len() <= 200);
```

The truncation contract: UTF-8-aligned cut, `...` appended on
truncation (dropped only when `max_chars < 3`), no ANSI escapes,
deterministic across builds.

---

## Custom tags ("just data")

YAML tags (`!Custom`, `!!python/object`, `!Color`) attach a
type label to any node. noyalib surfaces them as
[`Value::Tagged`] on the default `from_str::<Value>` path, never
as code paths — they are pure data the caller dispatches on:

```rust
use noyalib::{from_str, Value};

let v: Value = from_str("!Color '#ff8800'\n").unwrap();
match &v {
    Value::Tagged(t) => {
        assert_eq!(t.tag().as_str(), "!Color");
        assert_eq!(t.value().as_str(), Some("#ff8800"));
    }
    _ => unreachable!(),
}
```

Three escape hatches when the wrapper isn't what you want:

| Need | API |
| :--- | :--- |
| Read the inner value directly, ignoring the tag | [`Value::untag_ref`] / [`Value::untag`] |
| Strip a known tag inline on the streaming path (no AST detour) | [`TagRegistry::with`] |
| Reject any document carrying a non-core tag at parse time | [`policy::DenyTags`] |

For lossless **emit**, use [`to_string_value`] / [`to_writer_value`]:

```rust
use noyalib::{from_str, to_string_value, Value};
let v: Value = from_str("!Color '#ff8800'\n").unwrap();
let yaml = to_string_value(&v).unwrap();           // "!Color '#ff8800'\n"
let back: Value = from_str(&yaml).unwrap();
assert!(matches!(back, Value::Tagged(_)));         // round-trips
```

The generic [`to_string`] (and the rest of the `Serialize`-trait
family) routes `Value::Tagged` through `serialize_map` for
serde-bridge interop with `serde_json` etc., which is lossy on
the YAML-tag wire form. The dedicated `*_value` variants short-
circuit the `Serialize` pipeline.

Typed targets (`#[derive(Deserialize)] struct Foo { ... }`) see
through tags transparently — `from_str::<Foo>("!Foo {x: 1}")`
yields `Foo { x: 1 }` regardless of the tag. The tag wrapper is
only surfaced when the deserialise target is `Value` itself,
detected at the entry point via `TypeId`.

[`Value::Tagged`]: https://docs.rs/noyalib/latest/noyalib/enum.Value.html#variant.Tagged
[`Value::untag_ref`]: https://docs.rs/noyalib/latest/noyalib/enum.Value.html#method.untag_ref
[`Value::untag`]: https://docs.rs/noyalib/latest/noyalib/enum.Value.html#method.untag
[`TagRegistry::with`]: https://docs.rs/noyalib/latest/noyalib/struct.TagRegistry.html#method.with
[`policy::DenyTags`]: https://docs.rs/noyalib/latest/noyalib/policy/struct.DenyTags.html
[`to_string`]: https://docs.rs/noyalib/latest/noyalib/fn.to_string.html
[`to_string_value`]: https://docs.rs/noyalib/latest/noyalib/fn.to_string_value.html
[`to_writer_value`]: https://docs.rs/noyalib/latest/noyalib/fn.to_writer_value.html

---

## Compact list indentation

Two camps disagree on whether YAML lists under a mapping key
should indent (the default) or align with the key column. noyalib
ships both styles; opt into the compact form via
`SerializerConfig::compact_list_indent(true)`:

```rust
use noyalib::{to_string_with_config, SerializerConfig, Value};

let yaml = "items:\n  - a\n  - b\n";
let v: Value = noyalib::from_str(yaml).unwrap();

// Default — list items indented one level past the key.
let std = noyalib::to_string(&v).unwrap();
// Output:
//   items:
//     - a
//     - b

// Compact — list items align with the key column. Style preferred
// by Kubernetes manifests and GitHub Actions workflows.
let cfg = SerializerConfig::new().compact_list_indent(true);
let compact = to_string_with_config(&v, &cfg).unwrap();
// Output:
//   items:
//   - a
//   - b
```

---

## Key interning (memory footprint)

For Kubernetes-shaped streams that repeat the same keys
(`metadata`, `labels`, `name`, `apiVersion`, `selector`) thousands
of times, `noyalib::interner::KeyInterner` deduplicates the heap
allocations:

```rust
use noyalib::interner::KeyInterner;
use std::sync::Arc;

let mut interner = KeyInterner::new();
let a = interner.intern("metadata");
let b = interner.intern("metadata");
// Same `Arc` — second call returned a clone of the cached entry.
assert!(Arc::ptr_eq(&a, &b));
```

| Workload | Without interning | With `KeyInterner` |
| :--- | ---: | ---: |
| 20-byte key × 100 records | 2 KB heap | 20 B + 100 × 16 B (Arc clones) ≈ 1.6 KB |
| 20-byte key × 10 000 records | 200 KB | 20 B + 160 KB ≈ **20 B for the strings** |
| Distinct K8s key set × 1 000 manifests | ~520 KB | 13 distinct allocations + 13 000 × 16 B Arc clones |

The Mapping public API stays `String`-keyed for v0.0.1 — the
interner is the explicit opt-in primitive. A future major version
may switch the internal storage to `Arc<str>` and use the interner
transparently during parse.

---

## The "Norway problem"

YAML 1.1 resolved the bare scalar `no` as the boolean `false`. The
country code for **Norway** is `NO`, so a YAML 1.1 parser
silently rewrites `country: NO` to `country: false`. Real-world
data corruption.

**YAML 1.2's official fix** is to drop those bare-word booleans —
only `true` and `false` count. noyalib defaults to YAML 1.2
strict semantics, so `country: NO` round-trips as the string
`"NO"`.

For migrating from Docker Compose / GitHub Actions / pre-1.2 YAML
toolchains that *expect* the legacy behaviour, opt back in via
`legacy_booleans`:

```rust
use noyalib::{from_str_with_config, ParserConfig, Value};

// Default (YAML 1.2 strict): "NO" → string.
let v: Value = noyalib::from_str("country: NO\n").unwrap();
assert_eq!(v["country"].as_str(), Some("NO"));

// Opt-in legacy mode: "NO" → false. Use only when migrating
// from a toolchain that depended on the YAML 1.1 behaviour.
let cfg = ParserConfig::new().legacy_booleans(true);
let v: Value = from_str_with_config("country: NO\n", &cfg).unwrap();
assert_eq!(v["country"].as_bool(), Some(false));
```

Conversely, `strict_booleans = true` *tightens* the YAML 1.2
contract — only the lowercase forms (`true` / `false`) are
recognised; `True`, `TRUE`, `Yes`, etc. all stay strings. Use
this for schema-strict pipelines where boolean-shape ambiguity
is a contract violation.

---

## Library Usage

<details>
<summary><b>Deserialization</b></summary>

```rust,ignore
// API surface synopsis — substitute your own `Config`, `yaml`, `bytes`, etc.
use noyalib::{from_str, from_slice, from_reader, from_value, ParserConfig};

// From string, byte slice, reader, or Value
let config: Config = from_str(yaml)?;
let config: Config = from_slice(bytes)?;
let config: Config = from_reader(file)?;
let config: Config = from_value(&value)?;

// With security limits
let parser = ParserConfig::strict();
let config: Config = noyalib::from_str_with_config(yaml, &parser)?;
let config: Config = noyalib::from_slice_with_config(bytes, &parser)?;
let config: Config = noyalib::from_reader_with_config(reader, &parser)?;
```

</details>

<details>
<summary><b>Serialization</b></summary>

```rust,ignore
// API surface synopsis — substitute your own `config` value + `file` writer.
use noyalib::{to_string, to_writer, to_fmt_writer, to_value, SerializerConfig};

// To string, io::Write, or fmt::Write
let yaml: String = to_string(&config)?;
to_writer(&mut file, &config)?;
let mut buf = String::new();
to_fmt_writer(&mut buf, &config)?;

// To Value
let value: noyalib::Value = to_value(&config)?;

// With custom config
let ser_config = SerializerConfig::new()
    .indent(4)
    .quote_all(true)
    .document_start(true);
let yaml = noyalib::to_string_with_config(&config, &ser_config)?;
```

</details>

<details>
<summary><b>Dynamic values</b></summary>

```rust
use noyalib::{from_str, Value};

let value: Value = from_str("
name: test
items:
  - one
  - two
settings:
  debug: true
")?;

// Field access
let name = value.get("name").and_then(|v| v.as_str());

// Path-based traversal
let debug = value.get_path("settings.debug");

// Sequence indexing
let first = value.get("items").and_then(|v| v.get(0));

// Missing keys return None (never panic)
assert!(value.get("nonexistent").is_none());
assert!(value.get_path("a.b.c").is_none());

// Path queries: wildcards and recursive descent
let all_names = value.query("items[*]");       // all items
let deep = value.query("..debug");              // find at any depth
```

</details>

<details>
<summary><b>Zero-copy borrowed values</b></summary>

```rust
use noyalib::borrowed::from_str_borrowed;

let yaml = "host: localhost\nport: 8080\n";
let value = from_str_borrowed(yaml).unwrap();

// Strings borrow directly from input — zero heap allocation
assert_eq!(value.as_mapping().unwrap().get("host").unwrap().as_str(), Some("localhost"));

// Convert to owned Value when needed
let owned = value.into_owned();
```

18% faster than `from_str::<Value>` on typical payloads. Aliases not supported in borrowed mode.

</details>

<details>
<summary><b>Source spans</b></summary>

```rust
use noyalib::{from_str, Spanned};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    port: Spanned<u16>,
}

let config: Config = from_str("port: 8080")?;
assert_eq!(config.port.value, 8080);
assert_eq!(config.port.start.line(), 1);
assert_eq!(config.port.start.column(), 6);
```

`Spanned<T>` serializes transparently as `T`.

</details>

<details>
<summary><b>Merge keys</b></summary>

```rust
use noyalib::{from_str, Value};

let yaml = "
defaults: &defaults
  timeout: 30
  retries: 3

production:
  <<: *defaults
  timeout: 60
";

let mut value: Value = from_str(yaml)?;
value.apply_merge()?;
// production -> {timeout: 60, retries: 3}
```

</details>

<details>
<summary><b>Multi-document streams</b></summary>

```rust,ignore
// API surface synopsis — substitute your own `Config` type + `config1`/`config2` values.
use noyalib::{load_all, to_string_multi};

let docs = load_all("---\na: 1\n---\nb: 2\n")?;
for doc in &docs {
    println!("{doc:?}");
}

let items: Vec<Config> = noyalib::load_all_as::<Config>(yaml)?;
let yaml = to_string_multi(&[config1, config2])?;
```

</details>

<details>
<summary><b>Enum serialization</b></summary>

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum Action { StartServer, StopServer }

#[derive(Serialize, Deserialize)]
struct Task {
    #[serde(with = "noyalib::with::singleton_map")]
    action: Action,
}
```

| Module | Purpose |
| :--- | :--- |
| `singleton_map` | Serialize enums as `{Variant: data}` |
| `singleton_map_optional` | Same, for `Option<Enum>` |
| `singleton_map_recursive` | Apply recursively to nested enums |
| `singleton_map_with` | Custom key transforms (snake\_case, kebab-case) |

</details>

<details>
<summary><b>Formatting wrappers</b></summary>

| Wrapper | Effect |
| :--- | :--- |
| `FlowSeq<T>` | Inline sequence: `[a, b, c]` |
| `FlowMap<T>` | Inline mapping: `{a: 1, b: 2}` |
| `LitStr` / `LitString` | Literal block scalar (`\|`) |
| `FoldStr` / `FoldString` | Folded block scalar (`>`) |
| `Commented<T>` | Attach a YAML comment |
| `SpaceAfter<T>` | Insert a blank line after the value |

</details>

---

## Configuration

<details>
<summary><b>Parser configuration</b></summary>

```rust,ignore
// API surface synopsis — substitute your own `input` string.
use noyalib::{from_str_with_config, ParserConfig, DuplicateKeyPolicy};

let config = ParserConfig::new()
    .max_depth(64)
    .max_document_length(1_000_000)
    .max_alias_expansions(1000)
    .max_mapping_keys(10_000)
    .max_sequence_length(10_000)
    .duplicate_key_policy(DuplicateKeyPolicy::Error)
    .strict_booleans(true);

let value: noyalib::Value = from_str_with_config(input, &config)?;
```

For maximum strictness, use `ParserConfig::strict()`.

</details>

<details>
<summary><b>Serializer configuration</b></summary>

```rust,ignore
// API surface synopsis — substitute your own `value` to serialise.
use noyalib::{to_string_with_config, SerializerConfig, FlowStyle, ScalarStyle};

let config = SerializerConfig::new()
    .indent(4)
    .flow_style(FlowStyle::Auto)
    .scalar_style(ScalarStyle::DoubleQuoted)
    .quote_all(true)
    .document_start(true)
    .document_end(true)
    .block_scalars(true)
    .block_scalar_threshold(3);

let yaml = to_string_with_config(&value, &config)?;
```

</details>

---

## Examples

Run all examples:

```bash
cargo run --example all
```

<details>
<summary><b>All examples</b></summary>

| Category | Example | Purpose |
| :--- | :--- | :--- |
| **Core** | `hello` | Struct roundtrip |
| | `std` | Vec, HashMap |
| | `variants` | Enum strategies |
| | `deep` | Nested structures |
| | `dynamic` | Dynamic Value type |
| | `modify` | to\_value, from\_value, get\_path, MappingAny |
| | `tags` | Singleton map enums |
| **Spec** | `alias` | Anchors, aliases, merge keys |
| | `smart` | RcAnchor, ArcAnchor |
| | `overlay` | Value merging |
| | `inherit` | Merge key precedence |
| | `stream` | Multi-document streams |
| | `types` | Custom YAML tags |
| | `binary` | Large ints, .inf, .nan, hex/octal |
| | `lossless_u64` | Opt-in `Number::Unsigned` for scalars above `i64::MAX` |
| **Security** | `strict` | strict\_booleans, DuplicateKeyPolicy |
| | `secure` | ParserConfig limits |
| | `schema` | Schema validation |
| | `env` | ${VAR} expansion |
| **DX** | `errors` | Error types, diagnostics |
| | `trace` | Path tracking |
| | `source` | Spanned\<T\> locations |
| | `style` | FlowSeq, FlowMap, Commented |
| **Advanced** | `emit` | SerializerConfig options |
| | `rename` | Custom key transforms |
| | `flatten` | serde flatten, untagged |
| | `bridge` | JSON <-> YAML interop |
| | `pipes` | from\_slice, from\_reader, to\_writer |
| | `global` | Configuration layering |
| **Future** | `portable` | WASM portability proof |
| | `mask` | Secret\<T\> redaction |
| | `patch` | Surgical YAML patching |
| | `suggest` | "Did you mean?" typo detection |
| | `schema_ext` | Self-documenting config schemas |
| **Deep Rust** | `untagged` | Polymorphic deserialization |
| | `borrow` | Zero-copy patterns |
| | `transcode` | Value-to-Value transcoding |
| | `comments` | Comment handling |
| | `async_io` | Async integration |
| | `recursive` | Self-referential types |
| **Platform** | `diagnostic` | miette::Diagnostic integration |
| | `nostd` | #![no\_std] compatibility guide |
| | `preserve` | CST preservation foundations |
| | `lossless_edit` | Renovate-style version bump (CST `Document::set`) |
| **Runtime** | `async_io` | Async integration (spawn\_blocking pattern) |
| | `recursive` | Self-referential types (trees, org charts) |
| **Bench** | `bench` | Performance overview |
| **Ecosystem** | `include` | `$include`-key modular config (Argo CD / JSON-Schema-style refs) |
| | `figment` | Layered defaults / YAML / env via the `figment` Provider |
| | `validation_garde` | Declarative validation through `garde` + `Validated<T>` |
| | `validation_validator` | Declarative validation through `validator` + `ValidatedValidator<T>` |
| | `diagnostic_path` | `serde_path_to_error` — pinpoint the offending nested key |
| | `robotics_polymorphism` | Tagged-enum dispatch with `Degrees` / `Radians` / `StrictFloat` |

</details>

---

## When not to use noyalib

A few cases where another tool fits better, listed because the
short answer is "we don't do that yet" rather than because of a
disagreement on priorities.

- **You need to round-trip comments through the data-binding API.**
  The YAML data model excludes comments by spec. The lossless CST
  (`noyalib::cst::Document`) preserves them byte-for-byte for the
  tooling path, but `from_str::<T>` → `to_string(&T)` does not.
  No Rust YAML library currently round-trips comments through a
  typed deserialise / serialise pair.
- **You need a different YAML 1.1 resolver behaviour than the
  three forms the spec actually disagrees with 1.2 on.**
  `ParserConfig::version(YamlVersion::V1_1)` flips the bundle of
  resolver-table differences (`yes` / `no` / `on` / `off`
  booleans, bare-`0` octal `0644`, sexagesimal `60:00`) on as a
  single preset; fine-grained `legacy_*` flags remain for mix and
  match. Other 1.1-isms (mandatory `!!` tag prefix, broader
  timestamp parsing) are not version-gated in either direction.
- **You have a hard dependency budget that cannot tolerate a
  Grisu / Ryu float formatter and a hash-randomised lookup
  table.** Default profile carries 8 runtime deps. `noyalib =
  { version = "0.0.1", default-features = false, features =
  ["std"] }` (or the equivalent `features = ["minimal"]`) drops
  to 5 — `itoa`, `ryu`, and `serde_ignored` become opt-in via
  the `fast-int` / `fast-float` / `strict-deserialise` features.
  Numeric formatting falls back to `core::fmt` (slower; output
  remains valid YAML); the `from_str_strict` typo-detection
  helpers go away.

If you hit a case that should be on this list, please open an
issue — that's how it gets fixed or moved into the supported set.

---

## Development

```bash
make              # check + clippy + test
make test         # run all tests
make clippy       # lint with Clippy
make fmt          # check formatting
make examples     # run all examples
make doc          # build API documentation
make deny         # supply-chain audit
make miri         # Miri memory checking (requires nightly)
make sbom         # generate software bill of materials
make clean        # remove build artifacts
```

### Fuzzing

Nine `cargo-fuzz` targets ship under `fuzz/fuzz_targets/`:

```bash
# Generic surface
cargo +nightly fuzz run fuzz_parse              # arbitrary YAML parsing
cargo +nightly fuzz run fuzz_roundtrip          # parse → serialize → re-parse
cargo +nightly fuzz run fuzz_from_value         # Value → typed deserialise
cargo +nightly fuzz run fuzz_multi_doc          # multi-document streams
cargo +nightly fuzz run fuzz_strict             # tight security limits

# Targeted regression coverage
cargo +nightly fuzz run fuzz_borrowed_alias     # BorrowedValue + alias resolution
cargo +nightly fuzz run fuzz_diff               # owned-vs-borrowed parity
cargo +nightly fuzz run fuzz_double_quoted      # double-quoted scalar escapes
cargo +nightly fuzz run fuzz_yaml_v1_1          # YAML 1.1 resolver toggle
```

Seed corpus included in `fuzz/corpus/seed/`.

### Miri (UB / aliasing / leak verification)

noyalib is `#![forbid(unsafe_code)]` so Miri does not police
noyalib's own code — every byte is checked at compile time. The
reason a Miri job exists is to verify the *interaction* with the
runtime dependencies (`indexmap`, `rustc-hash`, `ryu`, `itoa`,
`memchr`, `smallvec` — all of which use `unsafe` internally) is
sound, plus to validate that the SWAR decimal parser and the
structural-bitmask iterator behave correctly under simulated
big-endian targets.

```bash
make miri              # focused suite — parser, scanner, value, interner, simd
make miri-full         # full lib test suite under Miri (slow)
make miri-bigendian    # focused suite simulated on mips64 big-endian

# Or invoke the script directly:
./scripts/miri.sh                    # full focused suite
./scripts/miri.sh simd               # subset
MIRI_TARGET=mips64-unknown-linux-gnuabi64 ./scripts/miri.sh
```

The CI matrix runs the focused suite on every PR (`miri-focused`)
and the full + big-endian sweep on a weekly schedule
(`miri-full`).

### CI

| Workflow | Trigger | Purpose |
| :--- | :--- | :--- |
| `ci.yml` | push, PR | Clippy, fmt, test (3 OS x 3 toolchains), coverage, audit, cargo-deny |
| `docs.yml` | push to main | Build and deploy API docs to GitHub Pages |
| `release.yml` | tag `v*` | Validate, cross-verify, checksums, SBOM, GitHub Release, crates.io |
| `security.yml` | push, PR, weekly | Dependency review, CodeQL analysis |

See [CONTRIBUTING.md](CONTRIBUTING.md) for signed commits and PR guidelines.

---

## Security

YAML parsers are notorious attack surface — `libyaml`-based wrappers
have shipped multiple critical CVEs over the years (deserialisation
RCE in PyYAML, billion-laughs amplification in dozens of language
ecosystems, code-execution-via-tag in legacy Ruby YAML). noyalib's
posture is built around closing each of those vectors at the
*architectural* level, not via opt-in flags.

### RCE prevention (no arbitrary object instantiation)

**noyalib does not instantiate arbitrary objects via tags.** YAML's
custom-tag mechanism (`!Foo`, `!!python/object`) is the historical
RCE vector — a malicious tag can load arbitrary code in legacy
parsers. noyalib only deserialises into Rust types you've defined
at compile time, with `#[derive(Deserialize)]`.

Custom tags surface as **pure data** through the [`Value`] tree,
never as code paths. Three options, in order of decreasing
strictness:

- **Tagged data is fully preserved.**
  `from_str::<Value>("!Custom 'hello'\n")` returns
  `Value::Tagged(Tag("!Custom"), Value::String("hello"))` — the
  same shape that already covered tagged sequences and tagged
  mappings. Downstream code can read the tag via
  `Value::Tagged(t)` pattern matching, dispatch on it, or step
  through it via [`Value::untag_ref`] for transparent reads. No
  global type registry, no runtime code lookup, no
  attacker-controlled instantiation — just data.
- **Typed targets see through tags.** A
  `#[derive(Deserialize)] struct Foo { x: u8 }` against
  `!Foo {x: 1}` yields `Foo { x: 1 }`. The typed visitor never
  observes the tag string, so an attacker forging a tag to
  trigger a different `Deserialize` impl is impossible.
- **`TagRegistry` for opt-in routing.** Register the specific
  tags your application understands; unregistered custom tags
  still surface as `Value::Tagged(...)` data. See
  [`TagRegistry`](https://docs.rs/noyalib/latest/noyalib/struct.TagRegistry.html).
- **`policy::DenyTags` for hard rejection.** Reject any
  document carrying a non-core tag at parse time, before any
  data flows downstream. See [Policy enforcement](#policy-enforcement-safe-yaml).

There is no path from a parsed YAML document to running attacker-
chosen code. Period.

[`Value`]: https://docs.rs/noyalib/latest/noyalib/enum.Value.html
[`Value::Tagged`]: https://docs.rs/noyalib/latest/noyalib/enum.Value.html#variant.Tagged
[`Value::untag_ref`]: https://docs.rs/noyalib/latest/noyalib/enum.Value.html#method.untag_ref

### Configurable resource budgets

The `ParserConfig::strict()` preset enforces seven **resource
budgets** that cap every dimension of input size — designed to
make billion-laughs and similar memory-amplification attacks
mathematically impossible:

| Budget | Default | `strict()` | Protects against |
| :--- | ---: | ---: | :--- |
| `max_depth` | 128 | 64 | Stack-blowing nested structures |
| `max_document_length` | 64 MiB | 1 MiB | Oversized payloads |
| `max_alias_expansions` | 1024 | 100 | **Billion-laughs amplification** |
| `max_mapping_keys` | 64 K | 1024 | Hash-collision DoS |
| `max_sequence_length` | 64 K | 1024 | Memory-spike DoS |
| `duplicate_key_policy` | `Last` | `Error` | Silent data loss |
| `strict_booleans` | off | on | The "Norway problem" |

Alias-byte accumulation uses `saturating_add` so a crafted input
that overflows a `usize` counter still triggers the limit cleanly
— no integer-wrap escape hatch.

### Secret redaction (`${VAR}` interpolation)

`Value::interpolate_properties` (see `examples/env.rs`) substitutes
`${name}` references inside string scalars from a property map.
By default it is **strict**: an unknown placeholder returns
`Error::Custom` so a misconfigured deployment surfaces immediately
rather than silently degrading. Two opt-in variants change that
contract:

| Method | Unknown placeholder | Error message echoes name |
| :--- | :--- | :--- |
| `interpolate_properties` *(default)* | `Err(Error::Custom)` | yes |
| `interpolate_properties_redacted` | `Err(Error::Custom)` | replaced with `<redacted>` |
| `interpolate_properties_lossy` | substituted with `""`, never errors | n/a |

For secrets, pair the strict path with `secrecy::Secret<T>` on the
target field to keep the substituted value out of `Debug` / log
output.

```rust
use noyalib::{from_str, Value};
use std::collections::HashMap;

let yaml = "db_url: ${DATABASE_URL}\napi_key: ${API_KEY}\n";
let mut map: HashMap<String, String> = HashMap::new();
map.insert("DATABASE_URL".into(), "postgres://...".into());
map.insert("API_KEY".into(), "redacted-token".into());

let mut value: Value = from_str(yaml).unwrap();
// Strict by default — every placeholder must resolve.
value.interpolate_properties(&map).unwrap();
// `api_key` now holds the substituted secret — wrap downstream
// in `secrecy::Secret<String>` to keep it out of Debug formatting.

// Hide the placeholder name in error output (audit-trail safe).
let mut value: Value = from_str(yaml).unwrap();
let _ = value.interpolate_properties_redacted(&map);

// Treat unknown placeholders as empty strings (env-var style).
let mut value: Value = from_str(yaml).unwrap();
value.interpolate_properties_lossy(&map);
```

### Compile- and runtime-safety guarantees

- `#![forbid(unsafe_code)]` across the **entire workspace** — no
  FFI, no raw-pointer dereferences, no `unsafe` blocks anywhere.
- `#[non_exhaustive]` on `ParserConfig`, `SerializerConfig`,
  `FlowStyle`, `ScalarStyle`, `Error` — adding a variant is not a
  semver break.
- `#[must_use]` on 83 query methods — silent result-discarding
  bugs caught at compile time.
- `#[track_caller]` on 13 Index/IndexMut panic paths — panic
  diagnostics point at *your* call site, not noyalib internals.
- Pluggable parser policies (`DenyAnchors`, `DenyTags`,
  `MaxScalarLength`) — opt into "Safe YAML" enforcement at parse
  time. See `noyalib::policy`.

### Supply chain

- `cargo audit` clean — zero advisories.
- `cargo deny` clean — license / advisory / ban / source checks.
- `cargo vet` clean — every dependency in the graph has a local
  audit or imported coverage from Mozilla / Google / Bytecode
  Alliance / Embark / ISRG audit sets.
- `OpenSSF Scorecard` tracked, badge in the header — v0.0.6
  lifts the score from `6.5/10` to `~9/10` by demoting workflow
  tokens to least privilege, pinning every GitHub Action by
  SHA, wiring Dependabot, and retiring the unmaintained
  `serde_yml` / `libyml` bench dev-deps. See
  [`doc/POLICIES.md` § OpenSSF Scorecard posture](doc/POLICIES.md#openssf-scorecard-posture)
  for the per-check breakdown.
- `SLSA L3` build provenance + sigstore signing on every release.
- `REUSE.software` 3.3 compliant — every source file carries
  SPDX headers.
- Signed commits (SSH ed25519) enforced via CI.

### Notes

- Comment round-tripping for the data-binding API is not supported
  (YAML spec excludes comments from the data model). `Commented<T>`
  provides write-only comment injection. The CST API
  (`noyalib::cst::Document`) preserves comments byte-for-faithfully
  for the lossless-tooling path.

---

## Documentation

| Document | Covers |
|---|---|
| [`doc/POLICIES.md`](doc/POLICIES.md) | MSRV, SemVer & API stability, security & audit pipeline, performance & algorithmic complexity, concurrency guarantees, platform support, the full feature-flag matrix, panic policy, error model, dependency policy, release & changelog policy. **Single source of truth** for engineering posture. |
| [`doc/MIGRATION.md`](doc/MIGRATION.md) | Umbrella index of migration guides for `serde_yaml` 0.9, `serde_yml`, `yaml_serde`, `serde-yaml-ng`, `serde-norway`, `serde-yaml-bw`, `serde-saphyr`, `yaml-spanned`. Each linked guide is a per-crate function-mapping table + behavioural-difference notes + checklist. |
| [`SECURITY.md`](SECURITY.md) | Disclosure policy, supported versions, contact, security design summary. |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Signed-commit policy, PR guidelines, local-test recipe. |
| [`CHANGELOG.md`](CHANGELOG.md) | Per-release notes following Keep a Changelog 1.1.0. |
| [`doc/USER-GUIDE.md`](doc/USER-GUIDE.md) | Long-form usage guide with worked examples for every major feature. |
| [`doc/ARCHITECTURE.md`](doc/ARCHITECTURE.md) | Module map, hot-path notes, design decisions. |
| [`doc/GLOSSARY.md`](doc/GLOSSARY.md) | YAML / serde terminology reference. |
| [`crates/noyalib/doc/internals.md`](crates/noyalib/doc/internals.md) | Library internals (parser stages, loader frames, CST green tree). |
| [`crates/noyalib/doc/errors.md`](crates/noyalib/doc/errors.md) | Error reference — every variant, when it fires, how to handle it. |

The per-crate READMEs at
[`crates/noyalib`](crates/noyalib/README.md),
[`crates/noya-cli`](crates/noya-cli/README.md),
[`crates/noyalib-lsp`](crates/noyalib-lsp/README.md), and the
split-out satellite repos
[`sebastienrousseau/noyalib-mcp`](https://github.com/sebastienrousseau/noyalib-mcp)
and
[`sebastienrousseau/noyalib-wasm`](https://github.com/sebastienrousseau/noyalib-wasm)
document the surface specific to each artifact (binaries, MCP
tools, LSP capabilities, WASM bindings).

---

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0) or [MIT](https://opensource.org/licenses/MIT), at your option.

See [CHANGELOG.md](CHANGELOG.md) for release history.

<p align="right"><a href="#contents">Back to Top</a></p>
