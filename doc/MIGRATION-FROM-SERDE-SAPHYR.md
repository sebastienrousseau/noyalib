<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migrating from `serde-saphyr` to `noyalib`

[`serde-saphyr`](https://crates.io/crates/serde-saphyr) (module
`serde_saphyr`) is `bourumir-wyngs`'s clean-room serde adapter
on top of the `saphyr` parser. `0.0.26` (2026-05-04) is the
current release. The README explicitly states:

> serde-saphyr is not a fork of serde-yaml and shares no code
> with it (apart from some reused tests).

The marquee design choice: **no `Value` DOM**. `serde-saphyr`
streams events directly into the typed deserialise target. If
you need an untyped sink, the README suggests `serde_json::Value`.

`noyalib` is a clean-room reimplementation of YAML 1.2 with the
same `serde` data model, no `unsafe` code, and no `libyaml` C
dependency. noyalib's default `from_str<T>` path is also
streaming-first; the `Value` DOM is *added*, not removed.

> **Crates.io / docs.rs state verified 2026-05-08.** If the
> upstream API has shifted since, file an issue and we'll update.

## TL;DR

```diff
-[dependencies]
-serde-saphyr = "0.0"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_saphyr::from_str;
-let cfg: Config = from_str(input)?;
+use noyalib::from_str;
+let cfg: Config = from_str(input)?;
```

The typed-target migration is a name swap. The major adds are
the `Value` DOM, the `Spanned<T>` wrapper, the lossless
`cst::Document` editor, and JSON Schema validation.

## Function-by-function mapping

| `serde_saphyr` 0.0 | `noyalib` |
|---|---|
| `serde_saphyr::from_str::<T>` | `noyalib::from_str::<T>` |
| `serde_saphyr::from_slice::<T>` | `noyalib::from_slice::<T>` |
| `serde_saphyr::from_reader::<R, T>` | `noyalib::from_reader::<R, T>` |
| `serde_saphyr::from_multiple::<T>` | `noyalib::load_all_as::<T>` |
| `serde_saphyr::from_str_with_options(input, options)` | `noyalib::from_str_with_config(input, &cfg)` |
| `serde_saphyr::from_slice_with_options` | `noyalib::from_slice_with_config` |
| `serde_saphyr::from_reader_with_options` | `noyalib::from_reader_with_config` |
| `serde_saphyr::from_multiple_with_options` | `noyalib::load_all_with_config` (untyped `DocumentIterator`; deserialize each item, or `load_all_as::<T>` without a config) |
| `serde_saphyr::to_string` | `noyalib::to_string` |
| `serde_saphyr::to_io_writer` | `noyalib::to_writer` |
| `serde_saphyr::to_fmt_writer` | `noyalib::to_fmt_writer` |
| `serde_saphyr::to_string_multiple` | `noyalib::to_string_multi` |
| `serde_saphyr::Options` (parse-side builder) | `noyalib::ParserConfig` |
| `serde_saphyr::SerializerOptions` | `noyalib::SerializerConfig` |
| `serde_saphyr::Deserializer` (streaming type) | `noyalib::Deserializer` |
| `options!` macro | `ParserConfig::new()`-with-builder-methods |
| `ser_options!` macro | `SerializerConfig::new()`-with-builder-methods |
| (no `Value` type) | `noyalib::Value` (7 variants, including `Tagged`) |
| (no `Mapping` type) | `noyalib::Mapping` |
| (no `Number` type) | `noyalib::Number` |
| (no `Spanned<T>` wrapper) | `noyalib::Spanned<T>` — source-location wrapper |
| (no CST) | `noyalib::cst::Document` — lossless byte-faithful edits |
| (no schema validation) | `noyalib::validate_against_schema` (JSON Schema 2020-12) |

### Options builders

`serde_saphyr` uses an explicit `Options` struct (or the
`options!` macro shorthand) per call. noyalib uses
`ParserConfig` / `SerializerConfig` builders:

```diff
-use serde_saphyr::{from_str_with_options, options};
-let cfg: Config = from_str_with_options(input, options! {
-    max_depth: 64,
-})?;
+use noyalib::{from_str_with_config, ParserConfig};
+let parser = ParserConfig::new().max_depth(64);
+let cfg: Config = from_str_with_config(input, &parser)?;
```

Same shape, different surface — the noyalib builder is a fluent
struct so you can clone / share configs across call sites.

### Adding a `Value` DOM

If you used `serde_saphyr` and wanted untyped inspection, you
were probably routing through `serde_json::Value`. noyalib has
a YAML-native `Value`:

```diff
-let v: serde_json::Value = serde_saphyr::from_str(input)?;
+let v: noyalib::Value = noyalib::from_str(input)?;
```

`noyalib::Value` keeps YAML's full data model (numeric types
distinct from JSON's, `Tagged` for custom tags, ordered
mappings via `IndexMap`).

## Behavioural differences worth knowing

1. **noyalib has a `Value` DOM, but doesn't require it.** The
   default `from_str::<T>` path streams directly into typed
   targets — same shape as `serde_saphyr`, no AST allocated.
   The `Value` tree is only built when the caller asks for it
   via `from_str::<Value>`.
2. **Panic-free parser.** Both crates target the same
   "defensive parser" niche. `serde-saphyr`'s "panic-free"
   guarantee corresponds to noyalib's
   `#![forbid(unsafe_code)]` plus the resource-limit gates
   (`max_depth`, `max_alias_expansions`, `max_document_length`).
3. **`Value::Tagged` exists in noyalib.** Custom-tag scalars
   like `!Custom 'hi'` surface as
   `Value::Tagged(t)` (`t.tag() == "!Custom"`, `t.value() == Value::String("hi")`). If
   you hit a tagged scalar in `serde_saphyr`'s typed path the
   tag was simply preserved in the YAML 1.2 sense; noyalib
   typed deserialise sees through the tag transparently to the
   inner value, matching the `serde_saphyr` typed contract.
4. **YAML 1.2 strict by default.** `country: NO` stays a string.
   `serde-saphyr` is YAML 1.2 by design; the migration is
   neutral on this dimension.
5. **Tag-driven instantiation rejected.** `serde-saphyr`
   rejects YAML tags that try to pick a typed-deserialise
   target (the so-called "billion-laughs of types" attack);
   noyalib's typed path also ignores tags during target
   resolution, only using them for the `Value` data path.

## Drop-in compatibility shim

`noyalib::compat::serde_yaml` mirrors `serde_yaml` 0.9, not
`serde-saphyr`. There's no `serde-saphyr`-shaped shim because
the surface is small enough that the function-by-function
table above is the migration. If you want to keep
`use serde_saphyr::*;` imports, define a thin local module
that re-exports the noyalib equivalents:

```rust
// in src/yaml_compat.rs
pub use noyalib::{
    from_reader, from_slice, from_str,
    to_fmt_writer, to_string,
    to_writer as to_io_writer,  // rename
    ParserConfig as Options,
    SerializerConfig as SerializerOptions,
};
```

## Migration checklist

- [ ] Replace `serde-saphyr = "0.0"` → `noyalib = "0.0"`.
- [ ] Replace `use serde_saphyr::*;` → `use noyalib::*;`.
- [ ] Replace `Options { max_depth: …, … }` /
      `options! { … }` → `ParserConfig::new().max_depth(…)…`.
- [ ] Replace `*_with_options` calls with
      `*_with_config(input, &cfg)`.
- [ ] Replace `to_io_writer` with `to_writer`,
      `to_string_multiple` with `to_string_multi`.
- [ ] If you routed dynamic inspection through `serde_json::Value`,
      switch to `noyalib::Value` for the YAML data model
      (numeric types distinct from JSON, `Tagged` for custom
      tags, ordered mappings).
- [ ] Run your existing test suite.

If you hit a snag, file an issue at
<https://github.com/sebastienrousseau/noyalib/issues>.
