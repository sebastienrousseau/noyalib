<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<p align="center">
  <img src="https://cloudcdn.pro/noyalib/v1/logos/noyalib.svg" alt="Noyalib logo" width="128" />
</p>

<h1 align="center">noyalib</h1>

<p align="center">
  <strong>A pure-Rust YAML 1.2 library with full <code>serde</code>
  integration and zero <code>unsafe</code> code — the engine
  half of the noyalib workspace.</strong>
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

- [Install](#install) — Cargo, MSRV
- [Quick Start](#quick-start) — parse and serialise in ten lines
- [Why this approach?](#why-this-approach) — design rationale
- [Surface](#surface) — what this crate exposes
- [YAML 1.2 conformance](#yaml-12-conformance) — official-suite results
- [Cargo features](#cargo-features) — feature-flag matrix
- [Library Usage](#library-usage) — common patterns
- [Examples](#examples) — runnable example index
- [Benchmarks](#benchmarks) — performance evidence
- [When not to use noyalib](#when-not-to-use-noyalib) — limitations
- [Migrating from `serde_yaml`](#migrating-from-serde_yaml) — name-for-name mapping
- [Documentation](#documentation) — extended reading
- [License](#license)

---

## Install

```toml
[dependencies]
noyalib = "0.0.15"
```

`no_std` (alloc-only) builds:

```toml
[dependencies]
noyalib = { version = "0.0.15", default-features = false }
```

Core data binding (`from_str`, `to_string`, `Value`, schemas) and
the streaming deserialiser run without the standard library.
`from_reader`, `to_writer`, the `Spanned<T>` deserialise helper
(uses thread-local storage), and the CST module require the
`std` feature, which is enabled by default.

**Lean / FIPS / embedded profile** — for users with strict
dependency budgets, `default-features = false, features =
["std"]` (or the equivalent `features = ["minimal"]` alias) drops
`itoa`, `ryu`, and `serde_ignored`. Numeric formatting falls back
to `core::fmt` (slower, output remains valid YAML); the
`from_str_strict` / `from_slice_strict` / `from_reader_strict`
typo-detection helpers are absent. Re-enable individually with
`features = ["fast-int", "fast-float", "strict-deserialise"]`.

**MSRV: Rust 1.75.0**, enforced by the `msrv-1-75-core` CI job.

---

## Quick Start

```rust
use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Config {
    host: String,
    port: u16,
    features: Vec<String>,
}

let yaml = "host: api.example.com\nport: 8080\nfeatures: [auth, api]\n";
let cfg: Config = from_str(yaml).unwrap();
let out: String = to_string(&cfg).unwrap();
let round: Config = from_str(&out).unwrap();
assert_eq!(cfg, round);
```

---

## Why this approach?

noyalib targets the niche `serde_yaml` / `serde_yml` / `libyml`
occupy — read YAML into typed Rust structs, write Rust structs
back as YAML — and is written from scratch against the YAML 1.2
spec. The implementation runs the official YAML test suite to
**100% strict compliance — 387/387 attempted cases pass, 0
failures**; 19 cases are deliberately skipped (tracked in
`tests/yaml_compliance_report.rs`). It is not a fork of `serde_yaml`;
the parser, scanner, serialiser, and CST are independent code.

Two architectural choices motivate the rewrite:

1. **Streaming-first deserialise.** The default `from_str<T>`
   path walks parser events directly into the typed target. The
   `serde_yaml`-shaped pattern is `parse → Value →
   T::deserialize(&Value)`, which allocates every key, scalar,
   and nested mapping into an intermediate AST that the typed
   target then throws away. noyalib bypasses that AST when the
   caller asked for a typed `T`. The dynamic `Value` tree is
   still available when callers want it, but it is no longer in
   the hot path.

2. **`#![forbid(unsafe_code)]` at the workspace root.** No FFI
   to a C library, no raw-pointer dereferences, and no `unsafe`
   blocks in the parser, scanner, formatter, or CST. CI enforces
   the attribute on every push. Most popular Rust YAML crates
   wrap `libyaml` via C-FFI; the `unsafe` blocks involved are
   usually well-vetted, but their existence makes a
   security-conscious downstream audit meaningfully harder.

A few features built on top of those choices: SIMD-accelerated
structural discovery (`memchr` SSE2/NEON for arity 1–3, SWAR
for 4–8, optional `core::simd` 32-byte structural-bitmask),
SWAR decimal parsing (~2× faster than `<i64 as FromStr>`), and
a side-table CST that reproduces the source byte-for-byte and
supports surgical edits.

The dependency tree is **eight required crates**: `serde`,
`indexmap`, `rustc-hash`, `itoa`, `ryu`, `memchr`, `smallvec`,
`serde_ignored`. **No archived or unmaintained crates appear in
the graph** — `serde_yaml` 0.9 (archived), `libyaml` (C-FFI),
and `thiserror` are all absent. `cargo audit`, `cargo deny`, and
`cargo vet` are CI gates on every push.

---

## Surface

| Module | Use |
|---|---|
| [`from_str`](https://docs.rs/noyalib/latest/noyalib/fn.from_str.html), [`from_slice`](https://docs.rs/noyalib/latest/noyalib/fn.from_slice.html), [`from_reader`](https://docs.rs/noyalib/latest/noyalib/fn.from_reader.html), [`from_value`](https://docs.rs/noyalib/latest/noyalib/fn.from_value.html) | Read YAML into `T: Deserialize`. Each has a `_with_config` variant. |
| [`to_string`](https://docs.rs/noyalib/latest/noyalib/fn.to_string.html), [`to_writer`](https://docs.rs/noyalib/latest/noyalib/fn.to_writer.html), [`to_value`](https://docs.rs/noyalib/latest/noyalib/fn.to_value.html) | Write `T: Serialize` back as YAML. |
| [`from_str_strict`](https://docs.rs/noyalib/latest/noyalib/fn.from_str_strict.html) (+ `_slice`, `_reader`) | Strict deserialise — error on any key the target type does not declare. Closes the silent-data-loss gap on config-key typos. |
| [`Value`](https://docs.rs/noyalib/latest/noyalib/enum.Value.html) | Dynamic tree (7 variants: `Null`, `Bool`, `Number`, `String`, `Sequence`, `Mapping`, `Tagged`). Path queries via `query("items[*].name")`. |
| [`Spanned<T>`](https://docs.rs/noyalib/latest/noyalib/struct.Spanned.html) | Wraps any `T` with `(line, column, byte offset)`. Survives `#[serde(flatten)]`. |
| [`cst::Document`](https://docs.rs/noyalib/latest/noyalib/cst/struct.Document.html) | Lossless CST. `doc.set("server.port", "9090")` rewrites only the touched span; comments + indentation preserved. |
| [`policy::{DenyAnchors, DenyTags, MaxScalarLength}`](https://docs.rs/noyalib/latest/noyalib/policy/index.html) | Pluggable parser policies. Reject documents at parse time. |
| [`schema_for`](https://docs.rs/noyalib/latest/noyalib/fn.schema_for.html), [`validate_against_schema`](https://docs.rs/noyalib/latest/noyalib/fn.validate_against_schema.html), [`coerce_to_schema`](https://docs.rs/noyalib/latest/noyalib/fn.coerce_to_schema.html) | JSON Schema 2020-12 codegen, validation, and schema-driven autofix. |
| [`parallel::parse`](https://docs.rs/noyalib/latest/noyalib/parallel/fn.parse.html) | Multi-doc parse across the Rayon thread pool. Linear with cores. |
| [`borrowed::from_str_borrowed`](https://docs.rs/noyalib/latest/noyalib/borrowed/fn.from_str_borrowed.html) | Zero-copy AST. Scalars borrow from input bytes (~18 % faster). |
| [`compat::serde_yaml`](https://docs.rs/noyalib/latest/noyalib/compat/serde_yaml/index.html) | Drop-in shim — `use noyalib::compat::serde_yaml as serde_yaml`. |

---

## YAML 1.2 conformance

Validated against the
[official YAML test suite](https://github.com/yaml/yaml-test-suite)
at **100% strict compliance — 387 / 387 attempted cases pass, 0
failures, 19 deliberately skipped**. The skip list is tracked in
`tests/yaml_compliance_report.rs` so the gap is explicit and
audit-friendly. The conformance report rebuilds on every CI run
via `cargo test --test yaml_compliance_report`.

The same suite also exercises:

- Anchors, aliases, and `<<:` merge keys.
- Flow vs block style mixing.
- Quoted, plain, literal (`|`), and folded (`>`) scalar styles.
- Multi-document streams (`---`).
- Every YAML 1.2 core schema tag (`!!int`, `!!float`, `!!bool`,
  `!!null`, `!!str`).

Opt-in YAML 1.1 boolean parsing is available for backwards
compatibility:

```rust
use noyalib::{from_str_with_config, ParserConfig};
let cfg = ParserConfig::new().legacy_booleans(true);
// "yes" / "no" / "on" / "off" parse as bool under legacy mode.
```

---

## Cargo features

All optional integrations are off by default. Enable only what
the application needs.

| Feature | Pulls in | Adds |
|---|---|---|
| `std` *(default)* | — | `from_reader`, `to_writer`, `Spanned<T>`, CST module |
| `miette` | `miette` 7 | Rich terminal diagnostics with source spans |
| `schema` | `schemars`, `serde_json` | `JsonSchema` derive + `schema_for::<T>()` |
| `validate-schema` | `schema` + `jsonschema` | `validate_against_schema`, `coerce_to_schema` |
| `figment` | `figment` 0.10 | `noyalib::figment::Yaml` provider |
| `garde` | `garde` 0.22 | `Validated<T>` wrapper |
| `validator` | `validator` 0.19 | `ValidatedValidator<T>` wrapper |
| `robotics` | — | `Degrees` / `Radians` / `StrictFloat` newtypes |
| `parallel` | `rayon` 1.10 | `noyalib::parallel::parse<T>` |
| `recovery` | — | `noyalib::recovery::parse_lenient` — best-effort parsing for LSP / IDE half-typed documents |
| `sval` | `sval` 2 | `impl sval::Value` for `Value` / `Number` / `Mapping` / `MappingAny` / `TaggedValue`, `noyalib::sval_adapter::to_sval_writer` |
| `tokio` | `tokio`, `tokio-util`, `bytes` | `noyalib::tokio_async::from_async_reader` / `from_async_reader_multi` and `YamlDecoder` codec for `tokio_util::codec::Framed` |
| `simd` | — | `noyalib::simd::*` primitives + parser hot path |
| `nightly-simd` | `simd` (nightly) | `core::simd`-backed 32-byte structural-bitmask scanner |
| `compat-serde-yaml` | — | `noyalib::compat::serde_yaml` shim for migration |

---

## Library Usage

### Read into typed structs

```rust
use noyalib::from_str;
use serde::Deserialize;

#[derive(Deserialize)]
struct Server { host: String, port: u16 }

let s: Server = from_str("host: api\nport: 8080\n")?;
# Ok::<_, noyalib::Error>(())
```

### Strict deserialise (reject unknown keys)

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Cfg { port: u16, host: String }

let yaml = "port: 8080\nhost: api\nporrt: 9090\n";   // typo

// Lenient: silently drops the typo.
let _ : Cfg = noyalib::from_str(yaml)?;

// Strict: typed error pointing at `porrt`.
let err = noyalib::from_str_strict::<Cfg>(yaml).unwrap_err();
assert!(err.to_string().contains("porrt"));
# Ok::<_, noyalib::Error>(())
```

### Lossless CST edit

```rust
use noyalib::cst::parse_document;

let mut doc = parse_document(
    "server:\n  port: 8080  # bind\n"
)?;
doc.set("server.port", "9090")?;
assert!(doc.to_string().contains("# bind"));   // comment preserved
# Ok::<_, noyalib::Error>(())
```

### Source spans

```rust
use noyalib::{from_str, Spanned};
use serde::Deserialize;

#[derive(Deserialize)]
struct Cfg { port: Spanned<u16> }

let cfg: Cfg = from_str("port: 8080\n")?;
assert_eq!(cfg.port.value, 8080);
assert_eq!(cfg.port.start.line(), 1);
# Ok::<_, noyalib::Error>(())
```

---

## Examples

60+ runnable examples under
[`crates/noyalib/examples/`](examples/):

```bash
cargo run --example all                  # runs every default-feature example
cargo run --example hello                # struct round-trip
cargo run --example dynamic              # dynamic Value tree
cargo run --example lossless_edit        # CST edits, comments preserved
cargo run --example flatten              # serde flatten + untagged
cargo run --example diagnostic   --features miette
cargo run --example schema_validation --features validate-schema
cargo run --example figment      --features figment
cargo run --example validation_garde --features garde
cargo run --example robotics_polymorphism --features robotics
cargo run --example recovery_lenient --features recovery
cargo run --example sval_streaming   --features sval
cargo run --example tokio_async_reader --features tokio
```

Categories (full list in
[`examples/`](examples/)): Core (`hello`, `std`, `variants`,
`deep`, `dynamic`, `modify`, `tags`); Spec (`alias`, `smart`,
`overlay`, `inherit`, `stream`, `types`, `binary`); Logic &
Security (`strict`, `secure`, `schema`, `env`); DX (`errors`,
`trace`, `source`, `style`); Advanced (`emit`, `rename`,
`flatten`, `bridge`, `pipes`, `global`); Future-Proof
(`portable`, `mask`, `patch`, `suggest`, `schema_ext`); Deep
Rust (`untagged`, `borrow`, `transcode`, `comments`); Platform
(`diagnostic`, `nostd`, `preserve`); Ecosystem
(`include`, `figment`, `validation_garde`,
`validation_validator`, `diagnostic_path`,
`robotics_polymorphism`).

---

## Benchmarks

```bash
cargo bench --bench benchmarks            # core throughput
cargo bench --bench comparison            # vs serde_yaml_ng, yaml-rust2, saphyr
cargo bench --bench architecture          # streaming vs AST, zero-copy, span-free
cargo bench --bench simd            --features simd
cargo bench --bench numeric_parse         # SWAR decimal pipeline
cargo bench --bench structural_bitmask
cargo bench --bench streaming_vs_value
cargo bench --bench incremental_repair    # CST edit cost
cargo bench --bench validation_overhead
cargo bench --bench large_doc_soak
```

Reproducible on Apple M-series + ubuntu-latest. Published
throughput tables sit in the
[Benchmarks section of the workspace README](https://github.com/sebastienrousseau/noyalib#benchmarks):
`noyalib` is **faster than every other pure-Rust YAML library
on every deserialize fixture measured**. Speedup ranges:
**1.69×–2.00×** vs `serde-saphyr`, **1.48×–1.96×** vs `serde_yml`,
**1.42×–1.84×** vs `serde_yaml_ng`, **1.38×–1.74×** vs
`yaml-spanned`, **1.11×–1.36×** vs `yaml-rust2`. Serialize is
**3.00×–4.34×** ahead of `serde_yaml_ng`. The structural-bitmask
scanner runs 4.2× (stable) / 9.2× (nightly-simd) over the memchr
loop on 1 MiB documents; the SWAR decimal parser runs 2.17–2.52×
faster than the stdlib `from_str`.

---

## When not to use noyalib

- **You need to round-trip comments through the data-binding
  API.** The YAML data model excludes comments by spec. The
  lossless CST (`noyalib::cst::Document`) preserves them
  byte-for-byte for the tooling path, but `from_str::<T>` →
  `to_string(&T)` does not. No Rust YAML library currently
  round-trips comments through a typed deserialise / serialise
  pair.
- **You need a different YAML 1.1 resolution rule than the three
  the spec actually disagrees with 1.2 on.** noyalib defaults to
  YAML 1.2 strict semantics; opt in via
  `ParserConfig::version(YamlVersion::V1_1)` to flip the three
  resolver-table differences (`yes` / `no` / `on` / `off`
  booleans, bare-`0` octal `0644`, sexagesimal `60:00`) on as a
  bundle. Other 1.1-isms (mandatory `!!` tag prefix, broader
  timestamp parsing) are not currently version-gated; both 1.1
  and 1.2 mode accept the relaxed forms.

---

## Migrating from another YAML crate

The headline diff for `serde_yaml` 0.9 plus a name-for-name
mapping table sits in the
[workspace README](https://github.com/sebastienrousseau/noyalib#one-minute-migration-from-serde_yaml-and-the-wider-ecosystem).
The deeper guide also covers `serde_yml`, `yaml_serde`,
`serde-yaml-ng`, `serde-norway`, `serde-yaml-bw`,
`serde-saphyr`, and `yaml-spanned` — per-crate function tables,
behavioural-difference notes, the drop-in shim, and a
migration checklist:
[`doc/MIGRATION-FROM-SERDE-YAML.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/MIGRATION-FROM-SERDE-YAML.md).

---

## Documentation

- **API reference**: <https://docs.rs/noyalib>
- **Engineering policies** (MSRV, SemVer, security, performance, concurrency, platform support, feature flags):
  [`doc/POLICIES.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/POLICIES.md)
- **Migration guides** (`serde_yaml` and 7 other YAML crates):
  [`doc/MIGRATION.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/MIGRATION.md)
- **Security policy**:
  [`SECURITY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/SECURITY.md)
- **Internals (module map, hot paths)**:
  [`doc/internals.md`](https://github.com/sebastienrousseau/noyalib/blob/main/crates/noyalib/doc/internals.md)
- **Error reference**:
  [`doc/errors.md`](https://github.com/sebastienrousseau/noyalib/blob/main/crates/noyalib/doc/errors.md)
- **User guide**:
  [`doc/USER-GUIDE.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/USER-GUIDE.md)
- **Architecture overview**:
  [`doc/ARCHITECTURE.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/ARCHITECTURE.md)
- **Workspace README**:
  <https://github.com/sebastienrousseau/noyalib#readme>

---

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT](https://opensource.org/licenses/MIT), at your option.
