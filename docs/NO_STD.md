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
| Portable options | `fast-int`, `fast-float`, `lossless-float`, `lossless-u64`, `include`, `compat-serde-yaml`, `sval`, `simd`, `wasm-opt`, and stable fallback for `nightly-simd` |

The `include` feature accepts caller-supplied resolvers; it does not require a
filesystem. `libm` stays a mandatory dependency so floating-point fraction and
fused multiply-add operations work without an additional math feature. Disabling
formatting accelerators retains integer and float serialization.

Some APIs require `std`: IO readers and writers, CST editing, property
interpolation, recursive anchor wrappers, shared-identity serialization,
and strict unknown-field helpers. The `strict-deserialise` feature compiles
in an alloc-only build but those helpers require `std` as well. Filesystem
includes, recovery, Tokio, Rayon, schema integrations, validation integrations,
and diagnostic renderers also depend on `std`, directly or through their
dependencies. Enabling those integrations is outside the alloc-only feature set.

The `no_std` integration test target reuses hosted tests for YAML semantics,
serialization, borrowing, spans, error paths, alias budgets, and optional
portable features. CI runs it with core, accelerated, and combined portable
configurations in separate target directories. Bare-metal checks compile the
combined portable features on `thumbv7em-none-eabihf`,
`riscv32imac-unknown-none-elf`, and `aarch64-unknown-none`; WebAssembly is
checked separately. Native test harnesses use `std`, so they complement the
bare-metal checks rather than establish a std-free dependency graph themselves.

Run the baseline behavior checks locally:

```sh
CARGO_TARGET_DIR=target-alloc-core cargo test --locked --no-default-features --test no_std --test issue_494
```
