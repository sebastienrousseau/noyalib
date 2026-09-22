<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<p align="center">
  <img src="https://cloudcdn.pro/noyalib/v1/logos/noyalib.svg" alt="noyalib logo" width="128" />
</p>

<h1 align="center">noyalib</h1>

<p align="center">
  A YAML 1.2 parser and serialiser for Rust with full <code>serde</code> integration, lossless editing, and no <code>unsafe</code> code.
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/noyalib/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/noyalib/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/noyalib"><img src="https://img.shields.io/crates/v/noyalib.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Registry" /></a>
  <a href="https://docs.rs/noyalib"><img src="https://img.shields.io/badge/docs.rs-noyalib-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs" /></a>
  <a href="https://scorecard.dev/viewer/?uri=github.com/sebastienrousseau/noyalib"><img src="https://img.shields.io/ossf-scorecard/github.com/sebastienrousseau/noyalib?style=for-the-badge&label=OpenSSF%20Scorecard&logo=openssf" alt="OpenSSF Scorecard" /></a>
  <a href="LICENSE-APACHE"><img src="https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg?style=for-the-badge" alt="License: Apache-2.0 OR MIT" /></a>
  <a href="https://github.com/sebastienrousseau/noyalib/blob/main/docs/POLICIES.md"><img src="https://img.shields.io/badge/MSRV-1.86.0-93450a.svg?style=for-the-badge&logo=rust" alt="MSRV 1.86.0" /></a>
</p>

---

## Contents

**Getting started**

- [Install](#install) — Cargo, source, and companion tools
- [Requirements](#requirements) — toolchain floor, platforms
- [Quick Start](#quick-start) — parse and serialise typed YAML

**The noyalib ecosystem**

- [The noyalib ecosystem](#the-noyalib-ecosystem) — core library, CLI, LSP, MCP, WASM, and compatibility crate

**Library reference**

- [Capabilities at a glance](#capabilities-at-a-glance) — the current surface by theme
- [Ecosystem comparison](#ecosystem-comparison) — short matrix; full table at [`docs/COMPARISON.md`](docs/COMPARISON.md)
- [Benchmarks](#benchmarks) — headline numbers; full table at [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md)
- [Features](#features) — module-level capability list
- [Configuration](#configuration) — core options
- [Examples](#examples) — runnable example index

**Operational**

- [When not to use noyalib](#when-not-to-use-noyalib) — limitations
- [Development](#development) — make targets, fuzzing, CI
- [Security](#security) — guarantees and compliance
- [Documentation](#documentation) — all reference docs
- [Stability guarantees](#stability-guarantees) — SemVer axis, output stability, minimum toolchain discipline
- [License](#license)

---

## Install

### As a Rust library

```toml
[dependencies]
noyalib = "0.0.50"
```

Disable the default `std` feature for `core` + `alloc` environments:

```toml
[dependencies]
noyalib = { version = "0.0.50", default-features = false }
```

Build the library and its test surface from source:

```bash
git clone https://github.com/sebastienrousseau/noyalib.git
cd noyalib
make
```

Command-line and integration surfaces ship as companion projects:

| Surface | Install |
| :--- | :--- |
| CLI | `cargo install noya-cli --locked` |
| LSP | `cargo install noyalib-lsp --locked` |
| MCP | `cargo install noyalib-mcp --locked` |
| WASM | `npm install @sebastienrousseau/noyalib-wasm` |

---

## Requirements

- Rust **1.86.0 or newer** for the complete build and test surface.
- Linux, macOS, and Windows are tested in CI on stable, beta, and nightly.
- `default-features = false` supports `wasm32-unknown-unknown`,
  `thumbv7em-none-eabihf`, `riscv32imac-unknown-none-elf`, and
  `aarch64-unknown-none` through `core` + `alloc`.

| Component | Minimum toolchain | Enforcement |
| :--- | :---: | :--- |
| `noyalib` | Rust 1.86.0 | `msrv-core` and per-crate CI jobs |
| Satellite crates | Rust 1.86.0 | lockstep per-crate MSRV jobs |
| Optional `compare-saphyr` benchmark | Rust 1.88.0 | excluded from the MSRV gate |

The floor may rise only on the breaking version axis, with the reason recorded
in the changelog. See [`docs/MSRV-AND-DEPRECATION.md`](docs/MSRV-AND-DEPRECATION.md).

---

## Quick Start

```rust
use noyalib::{from_str, to_string};

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
struct Config {
    name: String,
    port: u16,
}

fn main() -> Result<(), noyalib::Error> {
    let config: Config = from_str("name: api\nport: 8080\n")?;
    let yaml = to_string(&config)?;
    let roundtrip: Config = from_str(&yaml)?;
    assert_eq!(config, roundtrip);
    Ok(())
}
```

The typed path streams through Serde without first building a `Value` tree.
Use `noyalib::Value` for dynamic data and `noyalib::cst::Document` for edits
that must retain comments and formatting.

---

## The noyalib ecosystem

The family releases in strict `0.0.x` lockstep. Each repository owns one
delivery surface while the core library remains dependency-focused.

| Component | Purpose | Use case |
| :--- | :--- | :--- |
| [`noyalib`](https://github.com/sebastienrousseau/noyalib) | YAML parser, serialiser, CST, and schema engine | Embed YAML support in Rust software |
| [`noya-cli`](https://github.com/sebastienrousseau/noya-cli) | `noyafmt` and `noyavalidate` binaries | Formatting and validation in shells and CI |
| [`noyalib-lsp`](https://github.com/sebastienrousseau/noyalib-lsp) | Language Server Protocol implementation | Editor diagnostics, formatting, and hover |
| [`noyalib-mcp`](https://github.com/sebastienrousseau/noyalib-mcp) | Model Context Protocol server | Lossless YAML tools for AI agents |
| [`noyalib-wasm`](https://github.com/sebastienrousseau/noyalib-wasm) | `wasm-bindgen` wrapper | Browsers, Node.js, Deno, and Workers |
| [`noyalib-serde-yaml`](https://github.com/sebastienrousseau/noyalib-serde-yaml) | Compatibility package | Migrate code that imports `serde_yaml` |

---

## Capabilities at a glance

| Area | Capability | Status |
| :--- | :--- | :--- |
| Data binding | Serde deserialisation and serialisation | Stable |
| Dynamic data | Tagged `Value`, mappings, paths, and queries | Stable |
| Lossless tooling | CST edits that retain comments and layout | Stable |
| YAML conformance | YAML 1.2 with an opt-in YAML 1.1 resolver | CI-gated |
| Schemas | JSON Schema generation, validation, and coercion | Feature-gated |
| Diagnostics | Source spans, `miette`, and `ariadne` integration | Feature-gated |
| Async and parallel | Tokio readers and Rayon multi-document parsing | Feature-gated |
| Embedded targets | `no_std` with `alloc` | CI-gated |

---

## Ecosystem comparison

This summary identifies API shape, not a universal winner. Workload-specific
trade-offs and the evidence behind each cell are documented separately.

| Project | Serde data binding | Lossless CST | YAML 1.2 suite |
| :--- | :---: | :---: | :---: |
| **noyalib** | Yes | Yes | 406 active cases gated |
| `serde_yaml` | Yes | No | Archived |
| `serde-saphyr` | Yes, without a `Value` DOM | No | Partial |
| `yaml-spanned` | Read-only | No | Partial |

See [`docs/COMPARISON.md`](docs/COMPARISON.md) for the evidence and complete matrix.

---

## Benchmarks

Criterion measurements below use an Apple M4, aarch64, Rust 1.95 stable, and a
release profile with fat LTO and one codegen unit. Treat them as comparative
signals, not guarantees for other machines.

| Scenario | Result | Environment |
| :--- | ---: | :--- |
| Typed deserialise, simple document | 1.22 µs | Apple M4, Rust 1.95 |
| Deserialise, 500-item list | 920 µs | Apple M4, Rust 1.95 |
| Serialise, simple document | 290 ns | Apple M4, Rust 1.95 |
| Round-trip, nested document | 12.0 µs | Apple M4, Rust 1.95 |

See [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) for methodology and full results.

---

## Features

- Serde conversion through `from_str`, `from_slice`, `from_reader`,
  `to_string`, `to_writer`, `to_value`, and `from_value`.
- `Value`, zero-copy `BorrowedValue`, path queries, deep merge, and custom tags.
- Byte-faithful CST editing with typed mutation and structural rollback checks.
- Parser budgets for depth, document size, nodes, aliases, mappings, sequences,
  duplicate keys, and boolean resolution.
- Optional schema, validation, diagnostics, Figment, Tokio, Rayon, `sval`, and
  compatibility integrations.
- Stable SIMD-assisted scanning with an opt-in nightly SIMD implementation.

The complete feature-flag contract lives in
[`docs/POLICIES.md`](docs/POLICIES.md).

---

## Configuration

`ParserConfig` and `SerializerConfig` expose explicit policy without changing
the default typed API:

```rust
use noyalib::{from_str_with_config, DuplicateKeyPolicy, ParserConfig, Value};

let config = ParserConfig::new()
    .max_depth(64)
    .max_document_length(1_000_000)
    .max_alias_expansions(100)
    .duplicate_key_policy(DuplicateKeyPolicy::Error)
    .strict_booleans(true);

let value: Value = from_str_with_config("service: api\n", &config)?;
# Ok::<(), noyalib::Error>(())
```

Use `ParserConfig::strict()` as the starting point for untrusted input. The
configuration reference is in the [user guide](docs/USER-GUIDE.md).

---

## Examples

Runnable examples live in [`crates/noyalib/examples/`](crates/noyalib/examples/).

- [`hello.rs`](crates/noyalib/examples/hello.rs): typed parse and serialise.
- [`lossless_edit.rs`](crates/noyalib/examples/lossless_edit.rs): CST mutation.
- [`harden_untrusted.rs`](crates/noyalib/examples/harden_untrusted.rs): parser budgets.
- [`schema_validation.rs`](crates/noyalib/examples/schema_validation.rs): JSON Schema validation.
- [`tokio_async_reader.rs`](crates/noyalib/examples/tokio_async_reader.rs): asynchronous input.

Run every declared example with `make examples`.

---

## When not to use noyalib

- Use the CST API instead of typed data binding when comments and exact source
  layout must survive a round trip.
- Choose a smaller parser if a five-dependency minimal profile still exceeds the
  application's dependency budget.
- Choose another implementation if the application requires YAML 1.1 behaviour
  beyond noyalib's version preset and compatibility flags.

The [detailed README reference](docs/README-REFERENCE.md) records the longer
rationale and compatibility notes retained from the pre-template README.

---

## Development

```bash
make             # check, clippy, and test
make fmt         # verify rustfmt output
make deny        # dependency policy and advisory checks
make doc         # build API documentation
make examples    # execute every declared example
make bench-smoke # keep benchmark targets compiling
```

CI additionally enforces the OS and toolchain matrix, MSRV and bare-metal
builds, coverage floors, strict rustdoc, README examples, feature combinations,
SemVer compatibility, Miri, fuzz regression corpora, REUSE, and documentation
links. See [`DEVELOPMENT.md`](DEVELOPMENT.md) for local reproduction commands.

---

## Security

Report vulnerabilities privately according to [`SECURITY.md`](SECURITY.md).

The workspace forbids `unsafe` code. `ParserConfig::strict()` bounds resource
use, typed deserialisation cannot instantiate attacker-selected objects, and
custom tags remain data. Every push runs dependency review, advisory checks,
`cargo-deny`, `cargo-vet`, CodeQL, fuzz corpus replay, and supply-chain policy
checks. Releases carry checksums, a CycloneDX SBOM, Sigstore signatures, and
SLSA provenance.

---

## Documentation

- [User Manual](https://sebastienrousseau.github.io/noyalib/manual/)
- [API reference](https://docs.rs/noyalib)
- [Developer documentation](DEVELOPMENT.md)
- [Ecosystem map](docs/ECOSYSTEM.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Engineering policies](docs/POLICIES.md)
- [Migration guides](docs/MIGRATION.md)
- [Compliance grade](docs/COMPLIANCE-GRADE.md)
- [Detailed README reference](docs/README-REFERENCE.md)

---

## Stability guarantees

- During the `0.0.x` lifecycle, the patch component is the breaking-change axis.
- A change to accepted input, emitted output, scalar resolution, error behaviour,
  or the resulting `Value` shape is breaking even when Rust signatures do not
  change.
- `cargo-semver-checks` compares the public API with the previous release.
- Deprecations remain for at least two releases and name their replacement.
- The MSRV may rise only on the breaking axis and must be explained in the
  changelog.

---

## License

Licensed under either [Apache License 2.0](LICENSE-APACHE) or
[MIT](LICENSE-MIT), at your option.
