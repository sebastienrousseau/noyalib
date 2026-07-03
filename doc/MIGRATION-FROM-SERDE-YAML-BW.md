<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migrating from `serde-yaml-bw` to `noyalib`

[`serde-yaml-bw`](https://crates.io/crates/serde-yaml-bw)
(crate name `serde_yaml_bw`) is `bourumir-wyngs`'s hardened
fork of `serde_yaml`. `2.5.6` (2026-05-02) is the current
release. The `bw` reads as "better warnings" / "billion-laughs
warden" — the fork's marquee features are billion-laughs DoS
hardening, anchor / alias preservation, merge-key support,
nested-enum support, and binary scalars.

The 2.x major version line breaks `serde_yaml`'s API in
several places. The README explicitly states:

> serde_yaml_bw is a fork of serde-yaml originally, but it is
> not a drop-in replacement.

`noyalib` is a clean-room reimplementation of YAML 1.2 with the
same `serde` data model, no `unsafe` code, and no `libyaml` C
dependency. This guide is the function-mapping bridge to
noyalib's surface.

> **Crates.io / docs.rs state verified 2026-05-08.** If the
> upstream API has shifted since, file an issue and we'll update.

## TL;DR

```diff
-[dependencies]
-serde_yaml_bw = "2"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_yaml_bw::Value;
-let v: Value = serde_yaml_bw::from_str(input)?;
-let s        = serde_yaml_bw::to_string(&v)?;
+use noyalib::Value;
+let v: Value = noyalib::from_str(input)?;
+let s        = noyalib::to_string(&v)?;
```

The mechanical migration is similar — the surface diverges in
the multi-document API, the options builders, and the `Value`
enum shape (8 variants vs 7).

## Function-by-function mapping

| `serde_yaml_bw` 2.x | `noyalib` |
|---|---|
| `serde_yaml_bw::from_str::<T>` | `noyalib::from_str::<T>` |
| `serde_yaml_bw::from_slice::<T>` | `noyalib::from_slice::<T>` |
| `serde_yaml_bw::from_reader::<R, T>` | `noyalib::from_reader::<R, T>` |
| `serde_yaml_bw::from_value::<T>` | `noyalib::from_value::<T>` |
| `serde_yaml_bw::from_str_value` / `from_str_value_preserve` | `noyalib::from_str::<Value>` (`Tagged` preserved by default) |
| `serde_yaml_bw::from_str_multi` | `noyalib::load_all_as::<T>` |
| `serde_yaml_bw::from_slice_multi` | `noyalib::load_all_as::<T>` (after `std::str::from_utf8`) |
| `serde_yaml_bw::from_reader_multi` | `noyalib::load_all_as::<T>` (after `read_to_string`) |
| `serde_yaml_bw::from_multiple` | `noyalib::load_all_as::<T>` |
| `serde_yaml_bw::to_string` | `noyalib::to_string` |
| `serde_yaml_bw::to_writer` | `noyalib::to_writer` |
| `serde_yaml_bw::to_string_multi` | `noyalib::to_string_multi` |
| `serde_yaml_bw::to_writer_multi` | `noyalib::to_writer_multi` |
| `serde_yaml_bw::to_value` | `noyalib::to_value` |
| `*_with_options(input, options)` | `noyalib::from_str_with_config(input, &cfg)` (and family) |
| `serde_yaml_bw::Value` (8 variants) | `noyalib::Value` (7 variants — see below) |
| `serde_yaml_bw::Value::Alias(name)` | resolved at parse time; no longer a separate variant |
| `serde_yaml_bw::Mapping` | `noyalib::Mapping` |
| `serde_yaml_bw::Number` | `noyalib::Number` (default `Integer` / `Float`; opt-in `Unsigned(u64)` behind `lossless-u64`) |
| `serde_yaml_bw::Error` | `noyalib::Error` |
| `serde_yaml_bw::SerializerBuilder` | `noyalib::SerializerConfig` + `to_writer_with_config` |
| `serde_yaml_bw::SerializerOptions` | `noyalib::SerializerConfig` |
| `serde_yaml_bw::DeserializerOptions` | `noyalib::ParserConfig` |
| (n/a) | `noyalib::from_str_strict::<T>` — error on unknown keys |
| (n/a) | `noyalib::Spanned<T>` — source-location wrapper |
| (n/a) | `noyalib::cst::Document` — lossless byte-faithful edits |

### `SerializerBuilder` ↔ `SerializerConfig`

`serde_yaml_bw`'s builder pattern:

```rust
use serde_yaml_bw::SerializerBuilder;
let mut buf = Vec::new();
let mut ser = SerializerBuilder::default()
    .check_unresolved_anchors(true)
    .build(&mut buf);
value.serialize(&mut ser)?;
```

The noyalib equivalent uses a config struct:

```rust
use noyalib::SerializerConfig;
let cfg = SerializerConfig::new();
// configure cfg as needed
let mut buf = Vec::new();
noyalib::to_writer_with_config(&mut buf, &value, &cfg)?;
```

### Multi-document streams

`serde_yaml_bw` exposes a four-way fan-out
(`from_str_multi` / `from_slice_multi` / `from_reader_multi` /
`from_multiple`) for multi-doc input. noyalib collapses these
into a single `load_all_as` entry point:

```rust
let docs: Vec<Cfg> = noyalib::load_all_as(input)?;
```

For raw `Value` per document:

```rust
let docs: Vec<noyalib::Value> = noyalib::load_all_as(input)?;
```

### `Value` shape — 8 variants → 7 variants

`serde_yaml_bw::Value` has 8 variants:

- `Null` (with optional anchor)
- `Bool(bool)` (with optional anchor)
- `Number(Number)` (with optional anchor)
- `String(String)` (with optional anchor)
- `Sequence(Sequence)` (with optional anchor)
- `Mapping(Mapping)` (with optional anchor)
- `Alias(String)` — anchor reference, unresolved
- `Tagged(Box<TaggedValue>)`

`noyalib::Value` has 7 variants — there's no `Alias`. noyalib
resolves aliases at parse time, per YAML 1.2:

```diff
 match v {
     Value::Null            => …,
     Value::Bool(_)         => …,
     Value::Number(_)       => …,
     Value::String(_)       => …,
     Value::Sequence(_)     => …,
     Value::Mapping(_)      => …,
-    Value::Alias(_name)    => …,
     Value::Tagged(_)       => …,
 }
```

If your code matched `Value::Alias`, replace those arms with
the post-resolution variant — once resolved, the alias becomes
whatever the anchor pointed at (usually `String`, `Mapping`, or
`Sequence`).

If you relied on `serde_yaml_bw`'s anchor metadata on each
scalar variant, noyalib does not surface that on the `Value`
tree directly. The CST layer (`noyalib::cst::Document`) keeps
anchor / alias source spans and exposes them via the
`Document::annotations` API for editor-grade tooling.

## Behavioural differences worth knowing

1. **Alias resolution is eager.** noyalib resolves
   `*anchor_name` to the anchor's value at parse time
   (per YAML 1.2 §7.1). `serde_yaml_bw`'s `Value::Alias`
   variant has no equivalent in noyalib; if you need to
   *detect* an alias use site, work at the CST layer.
2. **Billion-laughs DoS hardening.** Both crates guard against
   exponential alias amplification. noyalib's defaults:
   `max_alias_expansions = 100`, `max_document_length`
   bounded by the input size; configure via
   `ParserConfig::new().max_alias_expansions(...)`.
3. **Merge keys (`<<`)** — both crates support YAML 1.1 merge
   keys. noyalib's `MergeKeyPolicy` enum lets you choose:
   `Merge` (default), `AsOrdinary` (treat `<<` as a literal
   key), or `Error` (reject).
4. **`Value::Tagged` is preserved.**
   `from_str::<Value>("!Custom 'hi'\n")` returns
   `Value::Tagged(Tag("!Custom"), Value::String("hi"))` — same
   as `serde_yaml_bw`.
5. **YAML 1.2 strict booleans by default.** `country: NO`
   stays `Value::String("NO")`. Opt into YAML 1.1 boolean
   recognition via `ParserConfig::new().legacy_booleans(true)`.

## Drop-in compatibility shim

The `noyalib::compat::serde_yaml` shim mirrors `serde_yaml` 0.9
(7-variant `Value`), not `serde_yaml_bw`. If you used
`serde_yaml_bw`'s 2.x-specific surface (the multi-doc family,
`SerializerBuilder`, the `Alias` variant), the shim won't help —
use the noyalib-native APIs directly.

## Migration checklist

- [ ] Replace `serde_yaml_bw = "2"` → `noyalib = "0.0"`.
- [ ] Replace `use serde_yaml_bw::*;` → `use noyalib::*;`.
- [ ] If you matched `Value::Alias`, drop those arms and
      handle the post-resolution variant instead.
- [ ] Replace `*_multi` / `*_multiple` calls with
      `noyalib::load_all_as`.
- [ ] Replace `SerializerBuilder` /
      `*Options` builders with `noyalib::SerializerConfig` /
      `noyalib::ParserConfig`.
- [ ] If you used the `_with_options` overload family, use
      `noyalib::from_str_with_config` (and the matching
      `to_writer_with_config`, `from_reader_with_config`, …).
- [ ] Decide YAML 1.1-vs-1.2 boolean handling.
- [ ] Run your existing test suite. Failures usually trace to
      either the `Alias` arm or the multi-doc API change.

If you hit a snag, file an issue at
<https://github.com/sebastienrousseau/noyalib/issues>.
