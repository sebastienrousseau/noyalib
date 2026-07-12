<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `noyalib` architecture

The map a contributor (or a distro packager) needs to find their
way around the codebase. Companion to `USER-GUIDE.md` (which
covers the public API) and the per-module rustdoc on
[docs.rs](https://docs.rs/noyalib).

## Workspace layout

```
crates/
├── noyalib/              # the library — public API
│   ├── src/
│   │   ├── lib.rs        # public re-exports + module index
│   │   ├── parser/       # event-stream parser
│   │   ├── cst/          # lossless concrete-syntax tree
│   │   ├── streaming.rs  # zero-AST typed deserialise (hot path)
│   │   ├── de.rs         # AST-shaped deserialise (Value path)
│   │   ├── ser.rs        # serialiser
│   │   ├── value.rs      # 7-variant Value enum
│   │   ├── borrowed.rs   # zero-copy AST
│   │   ├── simd.rs       # find_any_of, SWAR decimals, structural-bitmask
│   │   └── …
│   ├── tests/            # integration tests + YAML 1.2 official suite
│   ├── examples/         # 76 runnable usage examples
│   └── benches/          # Criterion benches
```

The only workspace member is `crates/noyalib`
(`members = ["crates/noyalib"]`). Crates that used to live
under `crates/` have been split into their own repositories per
ADR-0005, and the `xtask` crate was removed:

- `noya-cli` (noyafmt + noyavalidate binaries) → [`sebastienrousseau/noya-cli`](https://github.com/sebastienrousseau/noya-cli) (split at v0.0.13)
- `noyalib-lsp` (Language Server Protocol implementation) → [`sebastienrousseau/noyalib-lsp`](https://github.com/sebastienrousseau/noyalib-lsp) (split at v0.0.13)
- `noyalib-mcp` → [`sebastienrousseau/noyalib-mcp`](https://github.com/sebastienrousseau/noyalib-mcp) (split at v0.0.13)
- `noyalib-wasm` → [`sebastienrousseau/noyalib-wasm`](https://github.com/sebastienrousseau/noyalib-wasm) (split at v0.0.12)
- `xtask` (internal release tooling) → removed; PGO and release
  tasks now run from `scripts/`

The split satellite crates consume `noyalib` from crates.io via
strict-lockstep `=X.Y.Z` version pins.

The lib crate is `#![forbid(unsafe_code)]` workspace-wide. The
satellite crates inherit the same forbid; only the third-party
deps internally use `unsafe` (and Miri verifies the interaction
is sound — see `scripts/miri.sh`).

## End-to-end pipelines

There are **two** pipelines through the library, picked
automatically based on the caller's type:

```
                   ┌──────────────────────────────────────────────┐
                   │                                              │
   from_str::<T>   │   ┌───────┐   ┌────────┐   ┌──────────────┐  │
   (typed target) ─┼─► scanner├─► events ├─► StreamingDeser ├──┼─► T
                   │   └───────┘   └────────┘   └──────────────┘  │
                   │                                              │
   from_str::<Value>                                              │
   (dynamic) ─────►   scanner ──► events ──► loader ─────────────┼─► Value
                                                                 │
   parse_document                                                │
   (CST tooling) ─►   scanner ──► events ──► green-tree builder ─┼─► Document
                                                                 │
                                                                 ▼
                                                  to_string / to_writer / ser
```

The shared bottom of the funnel is **the scanner** (one
implementation, byte-faithful). The shared middle is **the event
stream** (a flat sequence of `Event::Scalar`, `SequenceStart`,
`MappingStart`, etc.). What's done with the events differs by
caller intent.

## Parser internals

### Phase 1 — scanner

`crates/noyalib/src/parser/scanner.rs`. Walks the input one
byte at a time, emits low-level tokens (indent / dedent /
plain-scalar boundary / flow-open / flow-close / quote / anchor /
alias / tag).

The hot path inside the scanner uses `noyalib::simd`:

- **`find_any_of`** — given a needle set `{':', '\n', '#', …}`,
  find the first byte in a window. Routes through:
  - `memchr::memchr` / `memchr2` / `memchr3` (SSE2 / NEON)
    for arity-1 / 2 / 3 needle sets — the most common shape.
  - SWAR (8-byte-stride `u64` packing) for arity 4–8.
  - Scalar fall-back for everything else.
- **`StructuralIter`** (under `--features simd`) — walks 32-byte
  chunks and produces a bitmask of *every* delimiter position
  in one pass via `mask.trailing_zeros()`. Same shape as
  simdjson's structural-character pass. Yields ~4× over the
  memchr loop on 1 MiB workloads (~9× on nightly under
  `core::simd`'s 32-byte vector).
- **`parse_decimal_u64` / `parse_decimal_i64`** — SIMD-Within-
  A-Register decimal parser. Folds 8 ASCII digits per `u64`
  cycle via three pair-wise multiply-add phases; ~2× faster
  than `<i64 as FromStr>::from_str` on large numbers.

All `unsafe`-free — the SIMD primitives are pure-Rust SWAR or
nightly `core::simd`; the SSE2 / NEON paths come from the
`memchr` crate, which uses well-vetted unsafe internally.

### Phase 2 — events

`crates/noyalib/src/parser/events.rs`. Reads the scanner's
tokens and emits semantic events. This is the YAML 1.2 spec's
production rule layer — handle `<<:` merge keys, anchor capture
(`&name`), alias resolution (`*name`), tag handling (`!foo`,
`!!str`).

Anchor expansion is bounded by `max_alias_expansions` in
`ParserConfig` — the billion-laughs vector is mathematically
impossible above the configured budget. The accumulator uses
`saturating_add` so a crafted overflow input still trips the
limit cleanly (no integer-wrap escape).

### Phase 3a — typed streaming (the hot path)

`crates/noyalib/src/streaming.rs`. The default `from_str::<T>`
path. Walks parser events directly into the typed target's
`serde::de::Visitor` interface — **no intermediate `Value` AST
is ever materialised**.

This is the architectural difference from `serde_yaml`-shaped
libraries, which always go event → AST → typed target. For the
common case (deserialise into a struct), eliminating the AST
saves ~one allocation per scalar plus the rebuild work.

The streaming path bakes in YAML 1.2 semantics:

- `<<: *alias` merges natively into the surrounding mapping.
- `!!binary` propagates as a typed tag.
- Custom tags surface via `Value::Tagged` (the streaming path
  routes through the AST loader for these — they're rare
  enough that the overhead is acceptable).

### Phase 3b — AST loader (for `Value` targets)

`crates/noyalib/src/parser/loader.rs`. Builds `Value` (the
7-variant enum: `Null`, `Bool`, `Number`, `String`, `Sequence`,
`Mapping`, `Tagged`). Used by:

- Callers that explicitly ask for `Value` (`from_str::<Value>`).
- Callers that need parser policies (which run against the
  built `Value`, not the streaming pipeline).
- Callers that opt out of streaming via
  `ParserConfig::ignore_binary_tag_for_string(true)`.

`Value` keys are interned via `noyalib::interner::KeyInterner`
when the caller provides one — for Kubernetes-shaped streams
(20-byte keys × 10 000 records) this drops memory footprint
from ~200 KB of fresh allocations to ~20 B of strings + ~160 KB
of `Arc` pointers.

### Phase 3c — CST builder (for tooling)

`crates/noyalib/src/cst/`. The third path through events.

The CST is a side-table green tree. Each node carries:

- A **kind tag** (`MappingNode`, `SequenceNode`, `Scalar`, …).
- A **byte span** `(start, end)` into the original input.
- A list of child node indices.

Crucially, **the original input bytes are kept alongside the
tree**. `Document::to_string()` is byte-identical to the input
that produced it, because `to_string()` simply prints the
original bytes back, sliced by the node boundaries.

Edits work by:

1. Resolving the edit path (`server.port`) to the byte span of
   the value to replace.
2. Replacing the bytes in that span only.
3. Rebuilding any green-tree nodes whose boundaries shift.

Step 3 is bounded by the size of the edit — neighbouring
mappings / sequences / comments are completely untouched. This
is the algorithmic distinction from "round-trip via AST" — that
approach has to re-emit the entire document on every edit.

Foundation of the `noyafmt` (formatter) and `noyavalidate
--fix` (schema-driven autofix) tools.

## The Value tree

`crates/noyalib/src/value.rs`. The 7-variant enum:

```rust
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Sequence(Vec<Value>),
    Mapping(Mapping),
    Tagged(Box<TaggedValue>),
}
```

`Mapping` wraps `IndexMap<String, Value>` so iteration order
matches insertion order (and source order, for parsed
documents).

`TaggedValue` carries a `Tag` + an inner `Value`. Tags are
interned strings; comparisons are byte-equal on the canonical
form (`!!str` and `tag:yaml.org,2002:str` both compare equal).

Path queries (`Value::query("items[*].name")`) implement a
small subset of the JSONPath grammar — wildcards (`*`),
recursive descent (`..`), index ranges (`[0:3]`). See
[`crates/noyalib/src/path.rs`](../crates/noyalib/src/path.rs)
for the full grammar.

## Spans

`crates/noyalib/src/spanned.rs`. `Spanned<T>` is a transparent
wrapper:

```rust
pub struct Spanned<T> {
    pub value: T,
    pub start: Location,
    pub end:   Location,
}
```

`Location` carries `(line, column, byte_offset)` — 1-indexed
lines and columns, 0-indexed bytes (matching rustc's idioms).

Span tracking is thread-local; the AST loader fills a span map
during parsing, then `Spanned<T>` deserialise reads from it
during the typed-deserialise pass. Cleared between calls.

The thread-local is the reason `Spanned<T>` requires the `std`
feature — `no_std` builds use the regular `T` directly.

## Serialiser

`crates/noyalib/src/ser.rs`. Single-pass `serde::Serializer`
that emits canonical YAML 1.2 output:

- Plain scalars when safe (no leading `-` / `?` / `:` / `,` /
  flow-open characters, no special-form like `null` / `true` /
  numbers that could be misread).
- Double-quoted otherwise, with the YAML escape table.
- Block sequences and mappings (the default); flow form opt-in
  via `FlowSeq<T>` / `FlowMap<T>` newtypes from
  `noyalib::fmt`.
- Block scalars (`|`, `>`) when string content would otherwise
  span multiple lines, gated by `block_scalar_threshold`.

Emission is configurable via `SerializerConfig`:

```rust
let cfg = SerializerConfig::new()
    .indent(4)
    .quote_all(true)
    .compact_list_indent(true)        // K8s-style lists
    .document_start(true);
```

## Streaming deserialiser hot path

The streaming path is roughly:

```rust
loop {
    match scanner.next_token()? {
        Token::MappingStart       => visitor.visit_map(...),
        Token::Scalar(text, kind) => visitor.visit_str(text)
                                     .or_else(... typed coercion ...),
        Token::SequenceStart      => visitor.visit_seq(...),
        // ...
    }
}
```

Coercion (e.g. plain scalar `8080` → `u16` field) happens at
the visit boundary. The plain-scalar resolver in
[`crates/noyalib/src/streaming.rs`](../crates/noyalib/src/streaming.rs)
is the YAML 1.2 schema-resolution table:

| Pattern | Resolves to |
|---|---|
| `null`, `~`, empty | `Null` |
| `true` / `false` (lowercase only when `strict_booleans`) | `Bool` |
| `[+-]?[0-9]+` | `Integer` |
| `[+-]?[0-9]*\.[0-9]+` | `Float` |
| `0x[0-9a-fA-F]+` | `Integer` (hex) |
| `0o[0-7]+` | `Integer` (octal) |
| anything else | `String` |

`legacy_octal_numbers(true)` opts into YAML 1.1's bare-`0`
prefix; off by default because the modern `0o` form is
unambiguous.

## Optional surfaces (v0.0.6)

Three v0.0.6 modules sit alongside the core pipeline; each is
gated behind its own Cargo feature so the default build pays
zero compile / binary-size cost for them.

| Module | Feature | Pipeline shape | Source |
|---|---|---|---|
| `recovery` | `recovery` | Wraps `from_str_with_config` in a three-pass retry loop (strict → `DuplicateKeyPolicy::Last` → line-truncation). Multi-document input split via the same `---` scanner the `parallel` module uses. Returns `ParseResult { value, errors, is_complete }`. Zero extra deps. | `crates/noyalib/src/recovery.rs` |
| `sval_adapter` | `sval` | `impl sval::Value for Value` (and `Number` / `Mapping` / `MappingAny` / `TaggedValue`) — streams a noyalib value graph through any `sval::Stream` consumer. Bypasses the serde monomorphisation chain entirely. | `crates/noyalib/src/sval_adapter.rs` |
| `tokio_async` | `tokio` | `from_async_reader` drains a `tokio::io::AsyncRead` to a `Vec<u8>` then runs the standard sync parser. `YamlDecoder` is a `tokio_util::codec::Decoder` that emits one document per `decode` call as soon as a column-0 `---` boundary lands in the buffer. | `crates/noyalib/src/tokio_async.rs` |

All three are pure-safe Rust (preserving the workspace
`unsafe_code = "forbid"` invariant) and exercised by unit
tests inline, integration tests under
[`crates/noyalib/tests/`](../crates/noyalib/tests/), and bench
arms in
[`crates/noyalib/benches/v006_features.rs`](../crates/noyalib/benches/v006_features.rs).

## Tests

| Suite | Where | What |
|---|---|---|
| Unit | inline `#[cfg(test)]` blocks | Per-module logic |
| Integration | `crates/noyalib/tests/` | Cross-module surfaces (139 files) |
| Doc-tests | every `///` block with a code-fence | Public-API examples |
| Property | `crates/noyalib/tests/proptest_*.rs` | Roundtrip invariants via `proptest` |
| Official YAML 1.2 suite | `crates/noyalib/tests/yaml-test-suite/**` | 406/406 strict-pass, 0 failures, 0 deliberate skips |
| CLI smoke | `crates/noya-cli/tests/` | End-to-end `noyafmt` / `noyavalidate` runs (in the split `noya-cli` repo) |

Total: ~4100 tests. Runtime: ~110 s for `cargo test --workspace
--all-features --no-fail-fast` on an M-series Mac.

## Coverage

Workspace coverage is measured by `cargo +nightly llvm-cov` and
gated in CI at 96 % functions / 94 % lines / 93 % regions.
Excluded from instrumentation:

- subprocess-driven protocol smoke tests (the same logic is
  covered by per-module unit tests). These live in the split
  `noyalib-mcp` / `noyalib-lsp` repos per ADR-0005, so they are
  no longer part of this workspace's instrumentation set.

(Prior to v0.0.12, `crates/noyalib-wasm/src/lib.rs` was also
excluded — its JsValue marshalling requires the wasm-bindgen
runtime, so it's covered by `wasm_bindgen_test` invocations in
the satellite repo at
[`sebastienrousseau/noyalib-wasm`](https://github.com/sebastienrousseau/noyalib-wasm)
per ADR-0005.)

Phase 7 of `PLAN.md` ratchets these gates to 98 / 98 / 98.

## Miri

`scripts/miri.sh` runs Miri on the high-leverage modules
(`parser::`, `scanner::`, `value::`, `interner::`, `simd::`)
and verifies the *interaction* between noyalib (which is
`#![forbid(unsafe_code)]`) and the runtime deps (`indexmap`,
`rustc-hash`, `ryu`, `itoa`, `memchr`, `smallvec` — all of
which use `unsafe` internally) is sound.

The full lib test suite runs under Miri on a weekly schedule
(`miri-full` job in `ci.yml`); the focused subset runs per-PR.

`-Zmiri-symbolic-alignment-check` is intentionally **not**
enabled because memchr's x86_64 SSE2 path triggers a known false
positive in `_mm_load_si128` on dynamically-aligned pointers.
The other defaults — Stacked Borrows, leak detection,
uninit-memory checks, OOB checks — are on.

## Cross-platform

| Target | Tested | Notes |
|---|---|---|
| `x86_64-unknown-linux-gnu` | ✓ CI matrix | Primary dev target |
| `aarch64-apple-darwin` | ✓ CI matrix | NEON SIMD path |
| `x86_64-apple-darwin` | ✓ CI matrix | macOS Intel |
| `x86_64-pc-windows-msvc` | ✓ CI matrix | Windows |
| `aarch64-unknown-linux-gnu` | ✓ CI matrix | ARM Linux |
| `wasm32-unknown-unknown` | ✓ via `noyalib-wasm` | 338 KB binary after LTO |
| Big-endian (`mips64`) | ✓ via Miri big-endian | SWAR + structural-bitmask paths verified |
| Other targets in the release matrix | Built at release time | See `release-binaries.yml` |

The SWAR pipelines and the `core::simd` structural-bitmask path
are byte-order agnostic. `u64::from_be_bytes` + `wrapping_mul`
arithmetic produce the same results on big-endian targets;
verified via `MIRI_TARGET=mips64-unknown-linux-gnuabi64` runs in
the weekly Miri-full job.

## Where to read next

- **Public API**: [`USER-GUIDE.md`](USER-GUIDE.md)
- **Migration**:
  [`MIGRATION-FROM-SERDE-YAML.md`](MIGRATION-FROM-SERDE-YAML.md)
- **Distribution / packaging**:
  [`../pkg/PUBLISH.md`](../pkg/PUBLISH.md)
- **Verifying release artefacts**:
  [`../pkg/VERIFY.md`](../pkg/VERIFY.md)
- **Design notes** (historical):
  [`design/`](design/)
- **Releases / phase work plan**:
  [`../PLAN.md`](../PLAN.md)
