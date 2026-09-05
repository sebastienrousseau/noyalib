<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# The noyalib Manual

noyalib is a YAML 1.2 library for Rust: pure safe code
(`#![forbid(unsafe_code)]`), full serde integration, a lossless CST
for byte-faithful editing, JSON Schema validation, and 406/406 on
the official YAML test suite. Six crates ship in lockstep at the
identical `=0.0.X` — the library plus the `noya-cli` binaries, the
LSP and MCP servers, the WASM bundle, and the `noyalib-serde-yaml`
drop-in replacement for archived `serde_yaml`.

This manual is the rendered form of the Markdown that lives in the
repository's [`docs/`](https://github.com/sebastienrousseau/noyalib/tree/main/docs)
directory — every page here is a file there, and the repository copy
is canonical.

## Where to start

| You want to | Read |
| :--- | :--- |
| Parse or write YAML from Rust | the [User guide](USER-GUIDE.md) |
| Leave `serde_yaml` behind | the [migration guides](MIGRATION.md) — for the one-line drop-in, see [noyalib-serde-yaml](https://github.com/sebastienrousseau/noyalib-serde-yaml) |
| Edit YAML without destroying formatting | the User guide's CST sections, then [Architecture](ARCHITECTURE.md) |
| Judge the library before adopting it | [Choosing a YAML library](COMPARISON.md) and [Testing and verification](TESTING.md) |
| Contribute | [`DEVELOPMENT.md`](https://github.com/sebastienrousseau/noyalib/blob/main/DEVELOPMENT.md) and [`CONTRIBUTING.md`](https://github.com/sebastienrousseau/noyalib/blob/main/CONTRIBUTING.md) at the repository root |

## The rest of the documentation

- **API reference** — [docs.rs/noyalib](https://docs.rs/noyalib)
  (also rendered [alongside this manual](../noyalib/index.html))
- **Getting started** — [`GETTING_STARTED.md`](https://github.com/sebastienrousseau/noyalib/blob/main/GETTING_STARTED.md)
- **Per-crate guides** — each satellite repository's README and
  `docs/` directory
