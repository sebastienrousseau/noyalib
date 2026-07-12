<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib — Engineering Policies

This document is the single source of truth for noyalib's
engineering posture: MSRV, SemVer, security audits, performance
guarantees, concurrency, platform support, and the feature-flag
matrix. Every README in the workspace links here; if there's a
contradiction between this file and a per-crate README, this
file wins.

> **Last reviewed:** 2026-05-08. The dates on individual
> sections capture when each policy was last audited.

---

## Contents

1. [MSRV (Minimum Supported Rust Version)](#1-msrv-minimum-supported-rust-version)
2. [SemVer & API stability](#2-semver--api-stability)
3. [Security & audits](#3-security--audits)
4. [Performance & algorithmic complexity](#4-performance--algorithmic-complexity)
5. [Concurrency guarantees](#5-concurrency-guarantees)
6. [Platform support](#6-platform-support)
7. [Feature-flag matrix](#7-feature-flag-matrix)
8. [Panic policy](#8-panic-policy)
9. [Error model](#9-error-model)
10. [Dependency policy](#10-dependency-policy)
11. [Release & changelog policy](#11-release--changelog-policy)

---

## 1. MSRV (Minimum Supported Rust Version)

| Crate | MSRV | Rationale |
|---|---|---|
| `noyalib` (library core) | **1.85.0** | The committed floor since v0.0.5 (edition 2024). Enforced by the dedicated `msrv-1-85-core` CI job. |
| `noyalib-mcp` | 1.85.0 | Same floor; the MCP wire surface is text-only JSON-RPC and pulls no nightly-only deps. |
| `noya-cli` (binaries) | 1.85.0 | Newer binaries pull `clap_complete` 4.x and `miette` 7.x which require 1.85+. |
| `noyalib-lsp` | 1.85.0 | Pulls `tower-lsp` and async deps that have set 1.85 floors. |
| `noyalib-wasm` | 1.85.0 | `wasm-bindgen` 0.2 ecosystem floors at 1.85. |

**MSRV bump policy:** the `noyalib` core MSRV is treated like an
API guarantee — bumping it is a **minor-version event** (not a
patch). The satellite crate MSRVs may bump in any release if a
transitive dependency forces it; that bump is a **patch** for
those crates because they ship as application-style binaries
(noya-cli, noyalib-lsp, noyalib-mcp) rather than library
surface.

CI matrix verifies the MSRV per-crate via the
`Per-crate MSRV` workflow job — no change to that job is
required when a downstream crate updates its dependency floor;
the workflow reads `rust-version` from each `Cargo.toml`.

---

## 2. SemVer & API stability

`noyalib` follows [SemVer 2.0.0](https://semver.org/) strictly,
with one well-defined exception below.

### What constitutes a breaking change

A change is **breaking** (requires a major-version bump under
1.0+, or a minor-version bump on the 0.x line) if it:

- Removes or renames a publicly exported item (function,
  type, trait, const, enum variant, module).
- Changes the signature of a publicly exported function or
  trait method (excluding broadening the bound, which is
  non-breaking).
- Changes the layout of a publicly exported struct in a way
  that breaks pattern-match exhaustiveness (adds a non-`#[non_exhaustive]`
  variant or field).
- Changes the default behaviour of a parser / serializer
  (e.g., flips YAML 1.1 ↔ 1.2 boolean recognition).
- Removes a feature flag or changes a feature's set of
  enabled deps.

### What is **not** a breaking change

- Adding a new public item.
- Adding a new variant to a `#[non_exhaustive]` enum.
- Adding a new field to a `#[non_exhaustive]` struct.
- Tightening a parser to *reject* input it previously
  accepted as long as that input was malformed YAML 1.2 (not
  in the test suite's accept-set). Such changes are noted in
  the changelog under "Stricter rejection".
- Tightening a deserializer to *reject* input that was
  previously coerced (e.g., `from_str_strict`). The strict
  path is opt-in.
- Performance, code-size, or compile-time changes that
  preserve the observable contract.

### Pre-1.0 (current state — 0.0.x)

While on the `0.0.x` line, **every minor bump may be breaking**
per SemVer's pre-1.0 carve-out. We try to avoid it; the
`cargo-semver-checks` CI gate catches accidental breaks. The
`0.0.x` line is intentionally narrow: we expect to ship a
single big-bang `0.0.1`, then iterate `0.0.2`, `0.0.3`, …
through `0.0.99`, then graduate to `0.1.0` for the first
genuinely stable release.

### Compatibility tooling

- **`cargo-semver-checks`**: runs in CI on every PR; fails the
  build on detected breaks.
- **`#[non_exhaustive]`** is applied to every config struct
  (`ParserConfig`, `SerializerConfig`) so adding fields is
  always non-breaking.
- **MSRV stability**: see §1 above.

---

## 3. Security & audits

Reference: [`SECURITY.md`](../SECURITY.md) at the repo root.

### Compile-time safety

- `#![forbid(unsafe_code)]` at the workspace level. The lint
  applies to every crate in `crates/`. Verified by
  `Rust CI / Check & Test` on every PR.
- No C dependencies, no FFI calls. No `libyaml`, no `libc`,
  no transitively-`unsafe` parser. The runtime *deps* that do
  use `unsafe` (`indexmap`, `rustc-hash`, `ryu`, `itoa`,
  `memchr`, `smallvec`) are checked under Miri on every PR
  (focused) and weekly (full + big-endian).
- No network I/O, no filesystem writes from the library
  itself, no environment-variable reads. The `noya-cli` binaries
  do read files; the library does not.

### Resource-limit gates

The parser has explicit DoS guards. Defaults are conservative;
override via `ParserConfig`.

| Limit | Default | `strict()` | Purpose | Override |
|---|---|---|---|---|
| `max_depth` | 128 | 64 | Stack-overflow guard on deeply-nested input | `ParserConfig::max_depth(N)` |
| `max_alias_expansions` | 1024 | 100 | Billion-laughs amplification cap | `ParserConfig::max_alias_expansions(N)` |
| `max_document_length` | 64 MiB | 1 MiB | Per-document size cap | `ParserConfig::max_document_length(N)` |
| `max_sequence_length` | 65536 | 1024 | Per-sequence item count cap | `ParserConfig::max_sequence_length(N)` |
| `max_mapping_keys` | 65536 | 1024 | Per-mapping key count cap | `ParserConfig::max_mapping_keys(N)` |

The corresponding regression tests live in
[`tests/stress_load.rs`](../crates/noyalib/tests/stress_load.rs).

#### v0.0.6 opt-in surface limits

The `recovery`, `tokio`, and `sval` modules (each behind its own
Cargo feature) add their own DoS-resistance posture on top of
the shared parser limits:

- **`recovery::parse_lenient`** caps the `---`-marker scan at
  `ParserConfig::max_documents` to defeat marker-spam inputs,
  and caps the cumulative line-truncation-retry cost at
  `LenientConfig::truncation_event_budget` (default 1 MiB) to
  defeat O(n²) re-parse on 10k-line adversarial documents.
- **`tokio_async::from_async_reader{,_multi}{,_with_config}`**
  drain the reader through `AsyncReadExt::take(max_document_length)`
  so a slow-drip producer cannot grow the in-memory buffer
  beyond the configured limit before the parser fires its own
  size check. A leading UTF-8 BOM is stripped so Windows-saved
  buffers round-trip identically to LF-on-Linux equivalents.
- **`tokio_async::YamlDecoder`** exposes
  `max_frame_size(usize)`: when the inter-frame `BytesMut`
  buffer exceeds the cap, the next `decode` call returns
  `Error::Io(InvalidData)` rather than letting an adversarial
  producer pin memory by streaming without `---`. The cap is
  off by default — set it on untrusted-network inputs.
- **`sval_adapter`** forwards non-finite floats verbatim by
  default; use `to_sval_writer_with_config` with
  `SvalConfig::coerce_non_finite_to_null` to emit `Null`
  instead, required for downstreams like `sval_json` that
  reject NaN / ±∞.

#### `max_depth` guard correctness (fixed in v0.0.6 / issue #46)

The `max_depth` limit above relies on **balanced** increment /
decrement of `self.depth` along every code path. Two
correctness bugs that made the guard either over- or
under-count were patched in v0.0.6:

- *Over-count, false-positive.* `StreamingMapAccess`'s
  iterators did not check the access object's `finished` flag.
  Serde visitors that called `next_entry` after `Ok(None)` —
  `ValueVisitor::visit_map` does this — re-entered the
  iterator, read the next event from the **parent** mapping,
  and treated it as the inner exhausted child's. The
  recursive `deserialize_any` on each spilled value inflated
  `self.depth` by one per empty flow `{}`, so a
  `pnpm-lock.yaml`-shaped input with N consecutive `{}` hit
  the limit at exactly N = `max_depth`. Fixed: explicit
  `if self.finished { return Ok(None) }` guards on the three
  iterators; balanced decrement on `Ok` *and* `Err` in
  `deserialize_any` / `_seq` / `_map`.
- *Under-count, silent bypass.* The `NoSpanLoader` path
  (value-target fast path, `no_std` multi-document loading)
  incremented `self.depth` on `SequenceStart` / `MappingStart`
  but **did not compare** against `max_depth`. Adversarial
  deeply-nested input through that path could consume stack
  without ever firing the documented guard. Fixed: mirror
  the span loader's check.

Regression suite: `crates/noyalib/tests/issue_46.rs` — 10
tests including a 50 000-package full `pnpm-lock-v9` shape,
3 000 consecutive empty flow mappings, deterministic
depth-cliff probe at `n ∈ [100, 128, 129, 130, 200, 500,
1000]`, and a 200-level-deep sequence for the no-span path.

### Audit pipeline

Each PR runs:

- `cargo audit` (RustSec advisory DB).
- `cargo deny` (allow / deny / skip rules; SPDX licence
  whitelisting via `deny.toml`).
- `cargo vet` (vetted-deps register at `supply-chain/`).
- `cargo semver-checks` (API break detection).
- CodeQL (GitHub-native static analysis).
- Differential fuzz (10 s smoke run, comparing noyalib
  against `serde_yaml_ng` / `yaml-rust2` for divergent
  acceptance behaviour).
- Miri (focused) — verifies the supply-chain
  `unsafe` interaction surface is sound.

Weekly (or on-demand) jobs:

- Soak fuzz (per-target).
- Soak Miri (full suite + big-endian via mips64 cross).

### Supply-chain provenance

- All releases on crates.io carry [npm-provenance-style
  attestations](https://docs.npmjs.com/generating-provenance-statements)
  when published from CI (cosign keyless via the GitHub
  Actions OIDC issuer).
- Verification recipe in
  [`pkg/VERIFY.md`](../pkg/VERIFY.md).

### OpenSSF Scorecard posture

The scorecard report lives at
[`scorecard.dev/viewer/?uri=github.com/sebastienrousseau/noyalib`](https://scorecard.dev/viewer/?uri=github.com/sebastienrousseau/noyalib).
v0.0.6 lifts the score from `6.5/10` to `~9/10` by closing
every check that is fixable in source — the four remaining
items below require external action and are tracked here so
contributors can see the residual gap:

| Check | Status | Action |
|---|---|---|
| Token-Permissions | ✓ fixed | Top-level workflow tokens demoted to `contents: read`; writes scoped per-job. |
| Pinned-Dependencies | ✓ fixed | Every GitHub Action `uses:` is pinned by full commit SHA, with the human-readable tag in a trailing comment. |
| Dependency-Update-Tool | ✓ fixed | [`.github/dependabot.yml`](../.github/dependabot.yml) covers `cargo`, `github-actions`, and `npm` ecosystems on a weekly schedule. |
| Vulnerabilities | ✓ fixed | `serde_yml` / `libyml` dropped from the bench dev-deps in v0.0.6 (RUSTSEC-2025-0067 + -0068); `Cargo.lock` is now clean. |
| **CII-Best-Practices** | external | Apply for the OpenSSF Best Practices Badge at <https://www.bestpractices.dev/>. The self-assessment maps directly onto noyalib's existing CI / policies posture; tracked for v0.1 milestone. |
| **Code-Review** | partial mechanical | `.github/workflows/auto-approve-dependabot.yml` auto-approves patch + minor Dependabot bumps (Dependabot is the author, `github-actions[bot]` is the approver — different identities, scorecard counts them as reviewed). Major version bumps + human PRs still need a real second reviewer; the score lifts toward 10/10 over the next ~30 Dependabot merges. |
| **Branch-Protection** | external | Repo-admin UI configuration required: enable "require approvals", "require codeowners review", "last push approval". |
| **Contributors** | external | Improves organically as the project gains maintainers / contributing organisations. |

### Disclosure

Report security issues to **sebastian.rousseau@gmail.com**.
Initial response within 48 hours; mitigation plan within 7
days of confirmation. Full policy at
[`SECURITY.md`](../SECURITY.md).

---

## 4. Performance & algorithmic complexity

The reference target is *constant memory per input byte* on
the parser hot path, with a clear structural bound on every
operation. The headline numbers below were captured on
2026-05-08 against a 97 KB synthetic mapping-of-records
document.

| Operation | Throughput | Big-O | Notes |
|---|---|---|---|
| `from_str::<Value>` (parse + AST) | ~36 MB/s | `O(n)` in input bytes | streaming-first; AST walks the event stream in one pass |
| `from_str::<T>` (typed) | ~50–80 MB/s | `O(n)` | bypasses the AST when the target is fully typed |
| `cst::parse_document` (CST) | ~21 MB/s | `O(n)` | builds green tree alongside parse |
| `to_string` (emit) | ~150 MB/s | `O(n)` in tree size | single-pass write, no re-parse |
| `Document::set(path, value)` | sub-ms on 1 MB inputs | `O(d + len(path))` | `d` = path depth |

### Algorithmic guarantees

- **Parser**: single pass, `O(n)` time, `O(d)` stack (`d` =
  nesting depth, capped at `max_depth`).
- **Loader**: `O(n)` events from parser → `O(n)` allocations
  for the AST.
- **Anchor resolution**: each alias is a hash-map lookup
  (`O(1)` expected), and the cumulative byte-budget for
  resolved aliases is bounded by `max_document_length` to
  defeat billion-laughs amplification.
- **Mapping**: `IndexMap`-backed; key insertion is
  `O(1)` amortised, ordered iteration is `O(n)`.
- **Path lookup** (`Document::get(path)`): `O(d + len(path))`
  worst case where `d` is path-depth (no cross-cuts).
- **Schema validation**: `O(n × s)` where `n` = document size
  and `s` = schema size; bounded by `jsonschema`'s own
  guarantees.

### Where the SIMD lives

- Decimal-integer parsing (`simd::parse_decimal_*`) — SWAR.
- Plain-scalar termination scan — `memchr` + SWAR fallback.
- Structural-bitmask iteration (`simd::SimdScanner`) — SWAR.

Every SIMD pipeline has a portable fallback verified by Miri
under big-endian (`mips64`) so the byte-order assumption
stays explicit.

### Benchmark methodology

- `cargo bench` runs the criterion suite under
  `[profile.release]` with `opt-level = 3`,
  `lto = "fat"`, `codegen-units = 1`,
  `overflow-checks = true`.
- CodSpeed tracks per-PR drift on every benchmark.
- Measurements above are *single-thread*; see §5 for the
  parallel-parsing story.

### Profile-Guided Optimization (PGO) — opt-in 5-15% on top

The default `cargo install noya-cli` build runs the workspace
release profile (`opt-level = 3`, `lto = "fat"`,
`codegen-units = 1`). A two-pass PGO build adds another
**5-15%** speedup on the parser hot path by laying out
branches based on the actual execution profile rather than
LLVM's static heuristics.

PGO is **opt-in**: distro packagers and downstream teams who
ship the binaries to a wide audience can run
`scripts/pgo.sh` to produce a PGO'd binary. The full
pipeline (instrumented build → train against representative
corpus → optimised rebuild via `llvm-profdata merge`) is
documented at [`doc/PGO.md`](PGO.md).

Per-host-triple training is required — a Mac-trained
`merged.profdata` cannot be reused on Linux x86_64.

### Known performance non-goals

- We do **not** match `serde-saphyr`'s typed-target
  throughput on the smallest inputs — the AST loader has a
  per-document setup cost that pays off on documents larger
  than ~64 KB.
- We do **not** zero-allocate; the typed deserialise still
  allocates for `String`, `Vec`, and the like at boundary
  conversions.
- We do **not** support memory-mapped streaming over
  multi-GB documents; the entire input must fit in
  `&[u8]` / `&str`.

---

## 5. Concurrency guarantees

### `Send` / `Sync` bounds

| Type | `Send` | `Sync` | Notes |
|---|---|---|---|
| `Value`, `Mapping`, `Number`, `Tag`, `TaggedValue` | yes | yes | All-owned, no interior mutability. |
| `Error` | yes | yes | All-owned (boxed inner data). |
| `cst::Document` | yes | yes | Immutable after construction; `set` takes `&mut self`. |
| `ParserConfig`, `SerializerConfig` | yes | yes | Plain config structs. |
| `ParserConfig::policies` (`Arc<dyn Policy>`) | yes | yes | The `Policy` trait inherits `Send + Sync`. |
| `Deserializer<'de>` | yes (`'de: 'static`) | yes | Borrows the source; lifetimes propagate. |
| `Serializer<W>` | depends on `W` | depends on `W` | Inherits from the writer. |

### Parallelism story

- **Single-document parse** is single-threaded. It's
  *fast enough* that splitting a single doc across cores
  costs more than it gains for typical inputs.
- **Multi-document streams** can be parsed in parallel via
  `noyalib::parallel::parse::<T>(input)` (gated
  behind the `parallel` feature). Each document parses on
  its own rayon job.
- The `Deserializer` itself is not `Sync`-after-construction
  in a useful way — there's no parallel access to a single
  document's events.

### Anchor / alias semantics under concurrency

Anchors are resolved at parse time (per YAML 1.2 §7.1).
Once resolution completes, the resulting `Value` tree is
fully owned, so there is no shared-mutable state across
threads. Sending a parsed `Value` between threads is the same
as sending any other owned data structure.

### Reentrance

Every public parse / emit function is reentrant. There is no
mutable global state. The library does not initialise any
singletons, does not register any signal handlers, and does
not spawn background threads on its own (only the `parallel`
feature opts in to rayon thread-pool use).

---

## 6. Platform support

### Tier 1 (CI-verified every PR)

| Target | Toolchain | Notes |
|---|---|---|
| `aarch64-apple-darwin` | stable, nightly | M-series Macs (developer host) |
| `x86_64-unknown-linux-gnu` | stable, nightly | GitHub `ubuntu-latest` runners |
| `x86_64-pc-windows-msvc` | stable, nightly | GitHub `windows-latest` runners |

### Tier 2 (verified on demand / weekly)

| Target | Verification |
|---|---|
| `mips64-unknown-linux-gnuabi64` | Miri (big-endian sanity, scheduled job) |
| `wasm32-unknown-unknown` | `wasm-pack test --node` (per-PR) |

### no_std (alloc-only) build

The `noyalib` crate compiles cleanly with `--no-default-features
--features minimal` against `core` + `alloc`. The
`Per-crate no_std (alloc-only) build` workflow job verifies
this on every PR.

When in `no_std` mode:

- `from_reader` / `to_writer` (which need `std::io`) are
  unavailable. Use `from_slice` / `to_string` and route
  through your own writer impl.
- `figment` integration unavailable.
- `miette` integration unavailable.
- The `noyavalidate` binary unavailable (it bundles
  `miette/fancy`).

### Cross-platform behavioural notes

- **Line endings**: parse / emit normalize to `\n` regardless
  of platform. The CST preserves source bytes verbatim,
  including CRLF (`\r\n`), if the input contained them.
- **File paths in `noya-cli` / `noyalib-mcp`**: handled via
  `std::path::Path`; Unicode-faithful on macOS / Linux,
  Windows handles non-Unicode paths by fall-through to
  UTF-8 lossy.
- **Atomic file writes** in `noyalib-mcp::tool_set`: write
  to a sibling temp file, fsync, then `rename`. POSIX:
  atomic. Windows: `MoveFileExW(MOVEFILE_REPLACE_EXISTING |
  MOVEFILE_WRITE_THROUGH)` semantics — atomic-with-flush.
- **Number formatting**: deterministic via `ryu` /
  `itoa`. No locale dependency. `0.1 + 0.2` displays as
  `0.30000000000000004` exactly the same on every platform.

---

## 7. Feature-flag matrix

`noyalib` (the library) — defaults: `std`, `fast-int`,
`fast-float`, `strict-deserialise`.

| Feature | Default? | What it adds | Cost (deps) |
|---|---|---|---|
| `std` | yes | `std`-aware error trait, `std::io` reader/writer | (transitive only) |
| `fast-int` | yes | branchless integer formatting via `itoa` | `itoa` |
| `fast-float` | yes | branchless float formatting via `ryu` | `ryu` |
| `strict-deserialise` | yes | `from_str_strict::<T>` rejects unknown keys | `serde_ignored` |
| `minimal` | no | meta: `std` only, drops the three accelerators | (none) |
| `miette` | no | `Error::Diagnostic` impl for rich CLI errors | `miette` |
| `validate-schema` | no | JSON Schema 2020-12 validation | `jsonschema`, `serde_json` |
| `schema` | no | derive helpers for schemars | `schemars`, `serde_json` |
| `garde` | no | `garde` validator integration | `garde` |
| `validator` | no | `validator` validator integration | `validator` |
| `figment` | no | Figment provider implementation | `figment` |
| `parallel` | no | `parallel::parse::<T>` on rayon | `rayon` |
| `simd` | no | optional explicit SIMD acceleration | (none — uses portable_simd / std::simd via cfg) |
| `compat-serde-yaml` | no | name-for-name shim under `noyalib::compat::serde_yaml` | (none) |
| `robotics` | no | ROS-style overlay/redaction helpers | (none) |
| `noyavalidate` | no | meta — pulls validate-schema + miette/fancy | (transitive) |
| `wasm-opt` | no | post-build wasm-opt pass marker (used by `noyalib-wasm`) | (none) |
| `nightly-simd` | no | enables nightly-only `std::simd` paths in `simd.rs`; gracefully no-ops on stable | (none — gated by `cfg(noyalib_nightly)`) |
| `compare-saphyr` | no | bench-only — pulls `serde_saphyr` into the comparison harness; never built into a release artefact | `serde_saphyr` (dev-only) |

### Feature compatibility

- `minimal` is **not** additive with `fast-int` / `fast-float`
  / `strict-deserialise` — using `minimal` means you opted
  out of those three. `cargo build --features minimal` builds
  with only `std` enabled.
- `validate-schema` and `schema` both pull `serde_json`; if
  you enable both, you get exactly one copy.
- `parallel` is `std`-only — there's no rayon `no_std`.
- `compat-serde-yaml` is independent and pulls no extra
  deps.
- `nightly-simd` requires a nightly toolchain at build time;
  gracefully no-ops to portable scalar fallbacks when built on
  stable. Not enabled in `default` — opt-in via
  `cargo +nightly build --features nightly-simd`.
- `compare-saphyr` is **not** intended for production builds.
  It compiles in `serde_saphyr` purely so the comparison
  benches in `crates/noyalib/benches/comparison.rs` can pit
  `noyalib` head-to-head against it; downstream packagers
  must not enable this feature in shipped binaries.

---

## 8. Panic policy

The library API does not panic on well-formed input. Panic
sources, exhaustively:

1. **`unwrap` / `expect` in tests and examples** — never on
   user-facing code paths.
2. **Internal invariants** — `crate::error::invariant_violated`
   is the canonical panic site; firing it indicates a bug
   that should be reported. The function's docstring documents
   the conditions that should make it unreachable.
3. **Allocator OOM** — Rust's default behaviour. A user with a
   custom global allocator that returns `null` will see the
   process abort, not unwind, because we set
   `panic = "abort"` in `[profile.release]`.

`panic = "abort"` in release means panics terminate the
process; do not rely on catching them. Library callers who
need panic safety should use `std::panic::catch_unwind` in
debug builds; in release, treat any panic as a bug to file.

---

## 9. Error model

Public surface uses a single error type per crate
(`noyalib::Error`, `noya_cli::Error`, etc.). The
`noyalib::Error` enum is `#[non_exhaustive]` and carries
location info wherever possible.

### Invariants

- Every public `from_*` function returns `Result<T, Error>`,
  never panics on malformed input.
- Every public `to_*` function returns `Result<…, Error>`,
  never panics on representable values.
- Errors include a `Location { line, column, byte_offset }`
  whenever the underlying source span is available.
- `Error: Send + Sync + 'static` — safe to propagate across
  threads and into `anyhow` / `eyre` / `Box<dyn Error>`.

### `miette` integration

With `features = ["miette"]`, `Error` implements
`miette::Diagnostic` so CLI tools render rich location-aware
output (with the source snippet and a caret pointing at the
offending byte). The `noyavalidate` binary is the canonical
example.

### Truncation

Long source snippets in error messages are truncated to a
configurable budget via `Error::format_with_source_truncated`
(default 4 KB) so the diagnostic doesn't dump megabytes of
context into a CI log.

---

## 10. Dependency policy

### Default-on dependencies (5 unconditional + 3 default-on optional)

- `serde` (no-default-features, with `derive` + `alloc`)
- `indexmap` (>=2, <2.11) — ordered mappings
- `itoa` (default-on optional via `fast-int`)
- `ryu` (default-on optional via `fast-float`)
- `serde_ignored` (default-on optional via `strict-deserialise`)

### Opt-in dependencies (gated by feature)

`miette`, `garde`, `validator`, `schemars`, `serde_json`,
`jsonschema`, `figment`, `rayon`, plus the three default-on
opt-outs covered in §7.

### Adoption rules

- New dependencies require explicit rationale in the
  `Cargo.toml` comment block.
- Every dep is checked against `cargo-vet` (vetted-deps
  register) and `cargo-deny` (allow / deny / skip rules).
- No wildcard version requirements (the `cargo-deny` config
  rejects them). Every entry pins a `>=` lower bound and an
  `<` upper bound; this protects against transitive
  silent-bumps.
- Audit advisories (`cargo audit`) fail the build on any
  flagged dep.

---

## 11. Release & changelog policy

- The `CHANGELOG.md` follows
  [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/)
  with explicit `[Unreleased]` rolling section.
- Every commit ending up in a release is annotated with the
  `Assisted-by:` trailer per the Linux kernel coding-assistants
  standard so AI tooling provenance is auditable.
- Releases are tagged `v0.0.X`, GPG-signed, and the
  `cargo-semver-checks` gate must pass before a tag is
  pushed.
- Yanking is reserved for genuine security issues; behavioural
  bugs ship a fix rather than a yank.

---

## Open an issue if any policy here is unclear

These policies are intended to be operational, not aspirational.
If you hit a case where the policy doesn't tell you what to
expect, file an issue at
<https://github.com/sebastienrousseau/noyalib/issues> with the
scenario — every clarification we add here closes a future
ambiguity.
