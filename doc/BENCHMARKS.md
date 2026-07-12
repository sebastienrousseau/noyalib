<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Benchmarks

All numbers below were measured on **Apple M4 (aarch64-apple-darwin),
Rust 1.95 stable**, criterion `--warm-up-time 2 --measurement-time 4`.
All libraries compiled with `--release` (LTO=fat, codegen-units=1,
panic=abort). Run locally via `cargo bench --bench comparison`.

> **PGO** — `scripts/pgo.sh` runs the full LLVM
> profile-guided optimisation pipeline (instrument → train against
> the YAML test suite + `benches/fixtures/` → merge profdata →
> optimised rebuild). Adds 5-15% on top of the numbers below;
> recommended for production deployments.

For algorithmic-complexity guarantees (`O(n)` parser,
`O(d)` stack depth, `O(1)` anchor lookup, etc.), see
[`POLICIES.md` §4 — Performance & algorithmic complexity](POLICIES.md#4-performance--algorithmic-complexity).

## Headline (5-way Rust YAML head-to-head, every fixture)

Compares noyalib against the five most-used pure-Rust YAML
libraries: `serde_yaml_ng` (the de-facto ecosystem successor to
the unmaintained `serde_yaml`), `yaml-rust2` (the heaviest-tuned
non-serde parser), `serde_yml` (the maintainer's prior crate),
`yaml-spanned` (the closest other span-tracking parser), and
`serde-saphyr` (a high-performance newcomer).

### Deserialize (parse a YAML document into a `Value` AST)

| Fixture | noyalib | vs `serde_yaml_ng` | vs `yaml-rust2` | vs `serde_yml` | vs `yaml-spanned` | vs `serde-saphyr` |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| simple (3 fields) | **1.40 µs** | **1.84×** | 1.36× | **1.96×** | 1.69× | **2.00×** |
| nested (20 fields) | **9.66 µs** | **1.55×** | 1.25× | **1.63×** | **1.60×** | **1.76×** |
| large_list (500 items) | **920 µs** | **1.42×** | 1.19× | **1.48×** | **1.38×** | **1.69×** |
| github_actions (deep + comments) | **46.4 µs** | **1.66×** | 1.25× | **1.72×** | **1.74×** | **1.73×** |
| k8s multi-document | **85.1 µs** | **1.42×** | 1.11× | — | — | — |

(`serde_yml`, `yaml-spanned`, `serde-saphyr` ship single-document
APIs only — no apples-to-apples cell on the multi-document
fixture.)

### Serialize (emit a typed value to YAML)

| Fixture | noyalib | vs `serde_yaml_ng` |
| :--- | ---: | ---: |
| simple (3 fields) | **290 ns** | **4.34×** |
| nested (20 fields) | **2.25 µs** | **3.00×** |

### Round-trip (deserialize + serialize)

| Fixture | noyalib | vs `serde_yaml_ng` |
| :--- | ---: | ---: |
| nested (20 fields) | **12.0 µs** | **1.83×** |

### Headline numbers

`noyalib` is **faster than every Rust YAML library on every
fixture measured**. Speedup ranges:

| Competitor | Speedup range (deserialize) |
| :--- | :---: |
| `serde-saphyr` | **1.69×–2.00×** |
| `serde_yml` | **1.48×–1.96×** |
| `serde_yaml_ng` | **1.42×–1.84×** |
| `yaml-spanned` | **1.38×–1.74×** |
| `yaml-rust2` | **1.11×–1.36×** |

Serialize: **3.00×–4.34×** over `serde_yaml_ng`. Round-trip:
**1.83×** over `serde_yaml_ng`. The gap to `yaml-rust2` is the
narrowest because that library doesn't carry the `Spanned<T>`
plumbing, the per-tag `Cow<'a, str>` propagation, or the
`Value::Tagged` preservation that noyalib does — paying for
those costs is part of the engineering cost. Closing the
remaining gap to ≥ 2× over `yaml-rust2` runs through
SemVer-breaking refactors (`CompactString` keys in `Mapping`,
bump-arena event lifetimes, eliminating the `Value` AST on the
typed path).

## Typed deserialization (streaming, no Value AST)

`from_str::<T>` walks parser events directly into the typed
target, bypassing the `Value` AST when the caller asked for a
typed `T`. The streaming path bakes in YAML 1.2 semantics
(`<<: *alias` merges natively, `!!binary` is propagated as a
typed tag).

| Library | Simple struct | Nested struct |
| :--- | ---: | ---: |
| **noyalib** | **1.22 µs** | **7.08 µs** |
| serde_yaml_ng | 2.10 µs (**1.72×**) | 11.0 µs (**1.55×**) |

## Why `serialize` is so far ahead

The serializer was built around `itoa` / `ryu` direct-write
buffers (the `fast-int` / `fast-float` features) plus
SIMD-driven quote-need detection that lets plain ASCII output
emit borrowed bytes without escaping. `serde_yaml_ng` routes
through `fmt::Write` and revalidates UTF-8 on output — that's
the source of the 3-4× gap.

## Why `deserialize` against `yaml-rust2` is closer

`yaml-rust2` is a heavily-tuned parser that doesn't carry the
`Spanned<T>` plumbing, the per-tag `Cow<'a, str>` propagation, or
the `Value::Tagged` preservation that `noyalib` does. The 1.12-1.35×
gap reflects the engineering cost of those guarantees;
`yaml-rust2` is a good baseline because it's the closest pure-Rust
parser in feature shape. The path to ≥ 2× over `yaml-rust2` on
deserialize runs through SemVer-breaking refactors
(`CompactString` keys in `Mapping`, bump-arena event lifetimes,
eliminating the `Value` AST on the typed path).

## Roundtrip (deserialize + serialize)

| Library | Nested (20 fields) |
| :--- | ---: |
| **noyalib** | **12.7 us** |
| serde\_yaml\_ng | 25.5 us (2.0x) |

## SIMD structural-discovery throughput

How fast each library can find every YAML delimiter in a
1 MiB real-shaped document. The structural-bitmask path
replaces the classical "find one delimiter at a time" pattern
with a 32-byte chunk that drains every delimiter via
`mask.trailing_zeros()` before reloading.
(`benches/structural_bitmask.rs`)

| Path | 4 KiB | 64 KiB | 1 MiB | vs memchr loop |
| :--- | ---: | ---: | ---: | ---: |
| scalar (byte-by-byte baseline) | 13.0 us | 206 us | 3.33 ms | 0.86x |
| memchr + `find_any_of` loop | 11.3 us | 179 us | 2.89 ms | 1.0x |
| **`StructuralIter` (stable)** | **2.7 us** | **42.3 us** | **681 us** | **4.2x** |
| **`StructuralIter` (nightly-simd)** | **1.20 us** | **19.7 us** | **311 us** | **9.2x** |

`serde_yaml_ng` and `serde-saphyr` use byte-by-byte structural
discovery — they sit alongside the *scalar baseline* row and
lose to the 32-byte-bitmask path by an order of magnitude on
the 1 MiB workload.

## SWAR decimal-integer parsing

Plain-scalar integer resolution via the SIMD-Within-A-Register
pipeline that folds 8 ASCII digits per `u64` cycle.
(`benches/numeric_parse.rs`)

| Width | stdlib `from_str` | **SWAR** | speedup |
| :--- | ---: | ---: | ---: |
| 8 digits | 8.12 ns | **3.74 ns** | **2.17x** |
| 19 digits | 22.0 ns | **9.25 ns** | **2.38x** |
| `i64::MAX` | 24.6 ns | **9.75 ns** | **2.52x** |
| Bulk parse 1000 ints | 7.93 us | **5.38 us** | **1.47x** |

### How the SWAR pipeline works

The standard library's `i64::from_str` walks one digit at a
time, branching on each character to validate it's `0..=9`
and folding into the accumulator with `acc * 10 + digit`.
SWAR fuses this loop:

1. Load 8 bytes into a `u64` register (one aligned `mov`).
2. Subtract `b'0'` lane-wise (`sub_packed`).
3. Validate every byte is `< 10` via a single masked compare.
4. Combine the eight digits into a single integer with two
   multiply-and-add rounds (the `0x010A0064...` constant
   pipeline).

The result is an 8-digit chunk consumed in 6 instructions
instead of ~24, and `i64::MAX` (19 digits) parses in 9.75 ns
versus 24.6 ns — a 2.5x speedup. The full pipeline is
documented inline in
[`crates/noyalib/src/simd.rs`](../crates/noyalib/src/simd.rs).

Every SWAR pipeline has a portable byte-by-byte fallback that
Miri verifies under big-endian (`mips64`) so the byte-order
assumption stays explicit.

## Parallel multi-document throughput

Linear scaling across CPU cores for `---`-separated streams
(telemetry logs, audit exports, Kubernetes-resource snapshots).
Pre-scan runs in `O(input_len)` on the main thread; the per-
document parse work distributes across the Rayon thread pool.
(`benches/streaming_vs_value.rs`, `benches/large_doc_soak.rs`)

```rust
// Single-threaded baseline:
let docs: Vec<MyType> = noyalib::load_all_as(yaml)?;

// Parallel (off by default — pulls Rayon under `parallel`
// feature). Drop-in replacement, scales near-linearly with cores
// on multi-document inputs:
let docs: Vec<MyType> = noyalib::parallel::parse(yaml)?;
```

Other Rust YAML libraries the comparison table covers run
single-threaded.

## Architecture validation

| Capability | Measured Impact |
| :--- | :--- |
| Streaming deserializer (bypasses Value AST) | **30% faster** (14.0 vs 19.4 us) |
| `BorrowedValue<'a>` (zero-copy AST) | **18% faster** (16.0 vs 19.4 us) |
| Zero-copy scanner (`Cow::Borrowed`) | **12% fewer** allocations (6.3 vs 7.1 us) |
| Span-free path (`from_str` default) | **34% less** overhead (5.6 vs 8.5 us) |
| FxHasher for Mapping keys | Faster key insertion and lookup |
| SIMD scanning (`memchr`) | Faster delimiter search on large inputs |
| Path queries | `value.query("items[*].name")` with `*` and `..` |
| DoS rejection (billion laughs) | **<3 us** with `ParserConfig::strict()` |
| DoS rejection (deep nesting) | **<4 us** |

Reproduce: `cargo bench --bench comparison` and
`cargo bench --bench architecture`.

## v0.0.6 feature benchmarks

Three additional bench arms cover the optional surfaces added
in v0.0.6 (`recovery`, `sval`, `tokio`). Run together with:

```bash
cargo bench --bench v006_features --features recovery,sval,tokio
```

| Group | Compares | Interpretation |
| :--- | :--- | :--- |
| `recovery_strict_vs_lenient` | `from_str` vs `recovery::parse_lenient` on valid input | The lenient wrapper should not regress strict throughput by more than rounding noise (recovery retries only execute on `Err`). |
| `sval_vs_serde_value` | `from_str` → `Value` (serde) vs streaming a pre-built `Value` through a no-op `sval::Stream` | Isolates the per-event cost of the sval surface — useful when deciding whether the serde-free route is worth the extra dep. |
| `tokio_async_drain` | `from_slice` (sync) vs `tokio_async::from_async_reader` against an in-memory `BufReader` | Quantifies the async fixed-overhead cost when I/O latency is zero; real network latency dwarfs this. |

## Project metrics

| Metric | Value |
| :--- | :--- |
| **Source** | 26,000+ lines across the workspace |
| **Test suite** | 3,686 tests + 431 doctests + CLI smoke + 13 stress/load |
| **YAML Test Suite** | 100% strict compliance: 406/406 attempted cases pass, 0 failures, 0 deliberate skips |
| **Examples** | 76 runnable examples |
| **Coverage** | 96%+ function coverage / 94%+ line coverage / 93%+ region coverage (CI-gated) |
| **Dependencies** | 5 unconditional + 3 default-on optional (`itoa`, `ryu`, `serde_ignored`) + 12 opt-in optional (`miette`, `garde`, `validator`, `schemars`, `serde_json`, `jsonschema`, `figment`, `rayon`, `serde-saphyr`, plus the three default-on opt-outs) |
| **WASM binary** | 338 KB (release, LTO) |
| **MSRV** | Rust 1.85.0 (core, since v0.0.5 — edition 2024); newer for optional features (see [POLICIES.md](POLICIES.md#1-msrv-minimum-supported-rust-version)) |
