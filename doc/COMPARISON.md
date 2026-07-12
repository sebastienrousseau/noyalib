<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Ecosystem comparison

How `noyalib` lines up against the other Rust YAML libraries it
is likely to be evaluated alongside. Cells reflect the state of
the named crates as of **2026-05** (verified against the latest
crates.io release of each); corrections welcome via PR.

For per-crate migration guides (function tables, behavioural
notes, drop-in shim notes), see
[`MIGRATION.md`](MIGRATION.md) and the per-crate guides
linked from there.

| | noyalib | serde\_yml | serde\_yaml\_ng | saphyr | yaml-rust2 | rust-yaml |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **YAML Test Suite** | 100% strict (406/406) | — | — | — | — | — |
| **Pure Rust** | Yes | No (C-FFI) | No (C-FFI) | Yes | Yes | Yes |
| **Zero `unsafe`** | Yes | No | No | Yes | Yes | Yes |
| **Serde integration** | Yes | Yes | Yes | Yes | No | Yes |
| **Streaming deser** | Yes | No | No | No | No | No |
| **`#![no_std]`** | Yes | No | No | No | No | No |
| **Zero-copy scalars** | Yes | No | No | No | No | Yes |
| **SIMD scanning** | Yes (memchr + bitmask) | No | No | No | No | No |
| **SWAR numeric parse** | Yes | No | No | No | No | No |
| **Parallel multi-doc** | Yes (`parallel::parse`) | No | No | No | No | No |
| **DoS hardened** | ~12 budgets | Basic | Basic | Yes | No | Yes |
| **Pluggable policies** | Yes (`policy::Policy`) | No | No | No | No | No |
| **Secret interpolation** | Yes (`${VAR}`) | No | No | Yes | No | No |
| **CST manipulation** | Yes (`cst::Document`) | No | No | No | No | No |
| **Native LSP** | Yes (`noyalib-lsp`) | No | No | No | No | No |
| **MCP server** | Yes (`noyalib-mcp`) | No | No | No | No | No |
| **JSON Schema codegen** | Yes (`schema_for`) | No | No | No | No | No |
| **JSON Schema validate** | Yes (`validate_against_schema`) | No | No | No | No | No |
| **Schema-driven autofix** | Yes (`coerce_to_schema`) | No | No | No | No | No |
| **`miette` diagnostics** | Yes | No | No | No | No | No |
| **WASM** | 338 KB | No | No | No | No | No |
| **Source spans** | Yes | No | No | Yes | No | No |
| **YAML 1.1 compat** | Yes | Yes | Yes | No | Yes | No |
| **Serialization** | Yes | Yes | Yes | Yes | No | No |
| **Path queries** | `query("..name")` | No | No | No | No | No |
| **Zero-copy AST** | `BorrowedValue<'a>` | No | No | No | No | Partial |

## Reading the table

- **YAML Test Suite (100% strict)** — `noyalib` is the only
  Rust YAML implementation that passes all 406 active cases
  in the YAML 1.2 official Test Suite under strict
  comparison. The other libraries either don't track this
  metric publicly or apply lenience that lets cases the spec
  rejects pass through.
- **C-FFI rows** — `serde_yml` and `serde_yaml_ng`
  (the active forks of `dtolnay/serde-yaml`) wrap `libyaml`
  via `unsafe-libyaml` in their dep tree. Pure-Rust
  alternatives don't have this transitive C dependency.
- **Streaming deser** — `noyalib::from_str::<T>` walks parser
  events directly into the typed target without building an
  intermediate `Value` AST. The other crates that *do* have
  serde integration build the AST first.
- **DoS hardening (~12 budgets)** — `max_depth`,
  `max_alias_expansions`, `max_document_length`,
  `max_sequence_length`, `max_mapping_keys`, `max_events`,
  `max_nodes`, `max_total_scalar_bytes`, `max_documents`,
  `max_merge_keys`, plus the cumulative-alias-byte budget, the
  alias/anchor ratio guard, and the per-document size
  cap. See [`POLICIES.md`](POLICIES.md#3-security--audits)
  for defaults and override knobs.

## How to verify a row yourself

The cells were sourced from each crate's `Cargo.toml`,
`docs.rs` API surface, and recent release notes. To
re-verify (e.g. after a new release elsewhere):

```bash
# Source-of-truth dump for any crate's exported surface:
cargo doc --no-deps -p <crate> --open

# Dependency tree (catches the libyaml C-FFI hop):
cargo tree -p <crate> --target all
```

If you spot a stale or wrong cell, file a correction at
<https://github.com/sebastienrousseau/noyalib/issues>.
