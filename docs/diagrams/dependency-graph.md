# Workspace dependency graph

The shape of crate-to-crate dependencies inside the noyalib
workspace. The arrows read "depends on" — `noya-cli → noyalib`
means `noya-cli` pulls `noyalib` from the workspace.

```mermaid
graph TD
    noyalib["noyalib<br/>core library<br/>MSRV 1.86"]

    noya_cli["noya-cli<br/>noyafmt + noyavalidate<br/>MSRV 1.86"]
    noyalib_lsp["noyalib-lsp<br/>LSP server<br/>MSRV 1.86"]
    noyalib_mcp["noyalib-mcp<br/>MCP server<br/>MSRV 1.86"]
    noyalib_wasm["noyalib-wasm<br/>WASM bindings<br/>MSRV 1.86"]
    xtask["xtask<br/>internal tooling"]

    noya_cli --> noyalib
    noyalib_lsp --> noyalib
    noyalib_mcp --> noyalib
    noyalib_wasm --> noyalib
    xtask --> noyalib
    xtask --> noya_cli

    classDef core fill:#1f6feb,stroke:#0d419d,color:#fff
    classDef satellite fill:#3fb950,stroke:#1f6f3a,color:#fff
    classDef tooling fill:#bd6107,stroke:#7d4101,color:#fff
    class noyalib core
    class noya_cli,noyalib_lsp,noyalib_mcp,noyalib_wasm satellite
    class xtask tooling
```

## Reading the graph

**`noyalib`** is the only crate with no internal dependencies — it
is the root of the workspace. Every other crate sits downstream
of it. This is enforced architecturally: `noyalib` cannot import
from `noya-cli`, `noyalib-lsp`, etc., even via tests.

**Satellite crates** (`noya-cli`, `noyalib-lsp`, `noyalib-mcp`,
`noyalib-wasm`) depend only on `noyalib`. They never depend on each
other, even when their feature surface overlaps. The MCP server
and the LSP server, for instance, both implement `format` and
`parse` operations, but each does so by calling `noyalib::cst::format`
directly — neither imports from the other. This keeps the per-crate
dependency footprint minimal and the integration tests independent.

**`xtask`** is the build-tooling crate (`cargo xtask completions`,
`cargo xtask manpages`, etc.). It pulls in `noya-cli` to call the
shared clap-derive command builders, and `noyalib` transitively.
It is `publish = false` and never ships to crates.io.

## MSRV (single lockstep floor)

| Crate | MSRV | Reason |
|---|---|---|
| `noyalib` | **1.86.0** | Single lockstep floor since v0.0.16; the lowest toolchain the project builds *and tests* on (`criterion 0.8` dev-dep requires 1.86) |
| `noyalib-mcp` | 1.86.0 | Lockstep with the core floor |
| `noyalib-wasm` | 1.86.0 | Lockstep with the core floor; wasm-bindgen 0.2 floors at 1.86 |
| `noya-cli` | 1.86.0 | Lockstep with the core floor; `clap_builder 4.6` is edition-2024 |
| `noyalib-lsp` | 1.86.0 | Lockstep with the core floor; LSP transport stack (`litemap`, `uuid`) is edition-2024 |
| `xtask` | 1.86.0 | Inherits the lockstep floor |

CI's `Per-crate MSRV` job still enforces the floor per crate — it reads
each `crates/*/Cargo.toml`'s `rust-version` and compiles against exactly
that — so a satellite adopting a higher floor cannot silently drag the
core up with it. The mechanism that kept the floors independent is
intact; what changed in v0.0.16 is only the numbers it reads.

As of v0.0.16 every crate declares 1.86.0. The binding constraint is
`criterion 0.8`, a **dev-dependency**: with `rust-version` temporarily
set to 1.85, `cargo +1.85.0 check --lib` compiles cleanly but
`cargo +1.85.0 check --all-targets` fails with
`criterion@0.8.2 requires rustc 1.86`. So no test, bench or coverage run
can execute below 1.86, and the project publishes the floor it verifies
rather than the lower one the library alone would reach.

No *runtime* dependency of the core crate requires 1.86 — the highest
runtime floor in the tree is 1.85. The earlier claim that
`validate-schema`'s ICU chain forced 1.86 was an artefact of a lockfile
refresh that did not ship; see `CHANGELOG.md` for v0.0.16.

## External dependency surface

Each crate's external (non-workspace) dependency count, default
profile:

| Crate | Runtime deps | Dev deps |
|---|---|---|
| `noyalib` | 8 (5 lean) | 8 |
| `noya-cli` | 2 (clap, miette) | 1 (criterion) |
| `noyalib-lsp` | 2 (serde, serde_json) | 1 (criterion) |
| `noyalib-mcp` | 2 (serde, serde_json) | 1 (criterion) |
| `noyalib-wasm` | 4 (wasm-bindgen et al.) | 1 (wasm-bindgen-test) |
| `xtask` | 3 (clap, clap_complete, clap_mangen) | 0 |

The `noyalib` lean profile (`--no-default-features --features
["std"]` or the `minimal` alias) drops to 5 — `itoa`, `ryu`, and
`serde_ignored` become opt-in via `fast-int`, `fast-float`, and
`strict-deserialise`. See [ADR 0001](../adr/0001-cst-rowan-shape.md)
for the architectural rationale and the per-crate README for the
opt-in matrix.

## Generating this graph

To regenerate the dot graph from the live workspace:

```sh
cargo depgraph --workspace-only --dedup-transitive-deps | dot -Tsvg > /tmp/deps.svg
```

`cargo-depgraph` is in `[dev-dependencies]` for the workspace; its
output is the source of truth for the Mermaid above. When the graph
shape changes (new crate added, dep direction reversed) update both
this file and [`docs/ARCHITECTURE.md`](../ARCHITECTURE.md).
