<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `no_std` support

Disable default features to use the core library with `core` and `alloc`:

```toml
[dependencies]
noyalib = { version = "0.0.59", default-features = false }
serde = { version = "1", default-features = false, features = ["alloc", "derive"] }
```

The application supplies an allocator. The target must support pointer-width
atomics: shared configuration, anchors and interned strings use `alloc::sync::Arc`.
Targets without those atomics, including `thumbv6m-none-eabi` and
`riscv32imc-unknown-none-elf`, are currently unsupported.

| Surface | Available with `no_std` and `alloc` |
| :--- | :--- |
| YAML parsing | Typed, dynamic `Value`, borrowed values, and document streams from strings and slices |
| Serialization | Strings, `Value`, and `core::fmt::Write` sinks |
| YAML semantics | Tags, anchors, aliases, merge keys, YAML 1.1 options, parser policies and budgets |
| Source information | Comments, structured diagnostics, deserialization error field paths, and `Spanned<T>` locations |
| Lossless tooling | CST parsing, formatting, typed growth, structural edits and rollback guards |
| Configuration | Property interpolation and recursive `Rc` anchor wrappers |
| Portable options | `fast-int`, `fast-float`, `strict-deserialise`, `recovery`, `lossless-float`, `lossless-u64`, `include`, `compat-serde-yaml`, `sval`, `simd`, `wasm-opt`, and stable fallback for `nightly-simd` |

The `include` feature accepts caller-supplied resolvers; it does not require a
filesystem. Private `core` helpers supply floating-point fraction and fused
multiply-add operations without a runtime math dependency or an additional feature.
Hosted builds retain their inherent std methods. `libm` is a development-only
reference for numerical tests. Disabling formatting accelerators retains integer
and float serialization.

The string and slice strict helpers work without std; the reader helper needs
std. Enabling `recovery` no longer enables std implicitly. Hosted callers that
also need IO APIs should explicitly enable std.

Property maps keep their std `HashMap` type in hosted builds. Alloc-only maps
use `hashbrown::HashMap<String, T, rustc_hash::FxBuildHasher>`. The types can
be inferred through `ParserConfig::properties`; no new map wrapper is required.

Some APIs require `std`: IO readers and writers, recursive `Arc<Mutex<_>>`
anchor wrappers, and automatic shared-identity serialization. Explicit anchor
registries remain portable. Filesystem includes, Tokio, Rayon, schema and validation integrations,
and diagnostic renderers also depend on `std`, directly or through their
dependencies. Enabling those integrations is outside the alloc-only feature set.

The `no_std` integration test target reuses hosted tests for YAML semantics,
serialization, borrowing, spans, error paths, alias budgets, and optional
portable features. The same matrix also runs the complete CST test target and
the `alloc_apis` public API regressions. CI runs core, accelerated, and combined portable
configurations in separate target directories. Bare-metal checks compile the
combined portable features on `thumbv7em-none-eabihf`,
`riscv32imac-unknown-none-elf`, and `aarch64-unknown-none`; WebAssembly is
checked separately. Native test harnesses use `std`, so they complement the
bare-metal checks rather than establish a std-free dependency graph themselves.

Run the baseline behavior checks locally:

```sh
CARGO_TARGET_DIR=target-alloc-core cargo test --locked --no-default-features --test no_std --test issue_494
```

Check float behavior and the active dependency graph from isolated default,
minimal std, and alloc-only consumers:

```sh
python3 scripts/check-float-portability.py
```

Run every library unit test with the std feature disabled:

```sh
cargo test --locked --no-default-features --lib
```

The unit-test harness runs on a host and imports std for test helpers such as
thread-local fault injection. The library keeps its alloc-only configuration;
the bare-metal builds above check it without a host test harness.
