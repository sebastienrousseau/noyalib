<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migrating to `noyalib`

This is the umbrella index for migrating to `noyalib` from
every actively-published Rust YAML crate. Each link goes to a
standalone, self-contained guide with TL;DR diff, function
table, behavioural notes, and migration checklist.

> **Crates.io / docs.rs state verified 2026-05-08.** If the
> upstream API has shifted since, file an issue at
> <https://github.com/sebastienrousseau/noyalib/issues> and we'll
> update.

## Pick your starting crate

| Coming from | Drop-in for `serde_yaml`? | Migration guide |
|---|---|---|
| [`serde_yaml`](https://crates.io/crates/serde_yaml) `0.9.34+deprecated` | (the original — archived 2024-03) | [`MIGRATION-FROM-SERDE-YAML.md`](MIGRATION-FROM-SERDE-YAML.md) |
| [`serde_yml`](https://crates.io/crates/serde_yml) `0.0.12` | mostly (archived 2025-09) | [`MIGRATION-FROM-SERDE-YML.md`](MIGRATION-FROM-SERDE-YML.md) |
| [`yaml_serde`](https://crates.io/crates/yaml_serde) `0.10.4` | yes (Cargo `package =` rename) | [`MIGRATION-FROM-YAML-SERDE.md`](MIGRATION-FROM-YAML-SERDE.md) |
| [`serde-yaml-ng`](https://crates.io/crates/serde-yaml-ng) `0.10.0` | yes | [`MIGRATION-FROM-SERDE-YAML-NG.md`](MIGRATION-FROM-SERDE-YAML-NG.md) |
| [`serde-norway`](https://crates.io/crates/serde-norway) `0.9.42` | yes | [`MIGRATION-FROM-SERDE-NORWAY.md`](MIGRATION-FROM-SERDE-NORWAY.md) |
| [`serde-yaml-bw`](https://crates.io/crates/serde-yaml-bw) `2.5.6` | **no** (breaking 2.x; 8-variant `Value` with `Alias`) | [`MIGRATION-FROM-SERDE-YAML-BW.md`](MIGRATION-FROM-SERDE-YAML-BW.md) |
| [`serde-saphyr`](https://crates.io/crates/serde-saphyr) `0.0.26` | **no** (no `Value` DOM, streaming-only) | [`MIGRATION-FROM-SERDE-SAPHYR.md`](MIGRATION-FROM-SERDE-SAPHYR.md) |
| [`yaml-spanned`](https://crates.io/crates/yaml-spanned) `0.0.3` | **no** (read-only, no `to_string`) | [`MIGRATION-FROM-YAML-SPANNED.md`](MIGRATION-FROM-YAML-SPANNED.md) |

## What to expect from `noyalib` regardless of source crate

Across every guide, three behavioural notes recur. They each
default to safer / stricter behaviour than `serde_yaml` 0.9 and
its forks; opt-outs are documented per guide.

1. **`Value::Tagged` is preserved through the `Value` data path.**
   `from_str::<Value>("!Custom 'hi'\n")` returns
   `Value::Tagged(t)` (`t.tag() == "!Custom"`, `t.value() == Value::String("hi")`). Typed
   targets (`#[derive(Deserialize)] struct Foo { … }`) still
   see through the tag transparently — the preservation only
   affects the dynamic `Value` path.
2. **YAML 1.2 strict booleans by default.** `country: NO` parses
   as `Value::String("NO")`, not `Value::Bool(false)`. Opt into
   YAML 1.1 boolean recognition via
   `ParserConfig::new().legacy_booleans(true)` if you need the
   old behaviour for round-trip compatibility with legacy
   files.
3. **Multi-doc streams use `load_all_as`.**
   `noyalib::load_all_as::<T>(input)?` returns a `Vec<T>`
   directly. The `Deserializer::from_str(...).map(...)` pattern
   from `serde_yaml` and its forks collapses to this one entry
   point.

## What `noyalib` adds (no equivalent in any of the eight)

- `noyalib::from_str_strict::<T>` — error on unknown keys.
- `noyalib::Spanned<T>` — source-location wrapper for typed
  fields. (`yaml-spanned` covers a similar use case for the
  dynamic path; noyalib covers both dynamic and typed.)
- `noyalib::cst::Document` — lossless byte-faithful edits with
  comment / whitespace preservation.
- `noyalib::ParserConfig` / `noyalib::SerializerConfig` —
  unified, fluent-builder configuration covering every
  per-call toggle.
- `noyalib::validate_against_schema` — JSON Schema 2020-12
  validation against the parsed document.
- `noyalib::compat::serde_yaml` — name-for-name re-export shim
  for the most common in-flight `use serde_yaml::*;` pattern,
  with no transitive `serde_yaml` 0.9 dependency.
- A set of binaries: `noyafmt` (formatter), `noyavalidate`
  (schema validator + auto-fixer), `noyalib-mcp` (Model
  Context Protocol server for AI tooling), `noyalib-lsp`
  (Language Server Protocol implementation).

## What `noyalib` does **not** do (yet)

- No equivalent to `serde-saphyr`'s strict
  panic-free-by-construction proof — noyalib's defence
  is `#![forbid(unsafe_code)]` plus the resource-limit gates,
  not formal verification.
- No streaming `Deserializer` you can `next()` event-by-event
  from user code — `noyalib::Deserializer` is a serde-trait
  driver, not an event iterator.
- No equivalent to `serde-yaml-bw`'s `Value::Alias` variant —
  noyalib resolves aliases at parse time per YAML 1.2 §7.1.

If your migration runs into a shape we don't cover, file an
issue at <https://github.com/sebastienrousseau/noyalib/issues>
with the upstream crate version and the input that misbehaves.
