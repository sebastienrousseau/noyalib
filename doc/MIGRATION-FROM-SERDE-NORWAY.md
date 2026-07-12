<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migrating from `serde-norway` to `noyalib`

[`serde-norway`](https://crates.io/crates/serde-norway) (crate
name `serde_norway`) is `cafkafk`'s hard-fork of `serde_yaml`.
`0.9.42` (2024-12-21) is the most recent release. The fork name
nods at the YAML 1.1 "Norway problem" (`country: NO` parsing as
`false`). The API is `serde_yaml`-shape-identical — the rebrand
is the value-add, not API divergence.

`noyalib` is a clean-room reimplementation of YAML 1.2 with the
same `serde` data model, no `unsafe` code, and no `libyaml` C
dependency.

> **Crates.io / docs.rs state verified 2026-05-08.** If the
> upstream API has shifted since, file an issue and we'll update.

## TL;DR

```diff
-[dependencies]
-serde_norway = "0.9"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_norway::Value;
-let v: Value = serde_norway::from_str(input)?;
-let s        = serde_norway::to_string(&v)?;
+use noyalib::Value;
+let v: Value = noyalib::from_str(input)?;
+let s        = noyalib::to_string(&v)?;
```

## Function-by-function mapping

`serde_norway` mirrors `serde_yaml` 0.9 exactly.

| `serde_norway` 0.9 | `noyalib` |
|---|---|
| `serde_norway::from_str::<T>` | `noyalib::from_str::<T>` |
| `serde_norway::from_slice::<T>` | `noyalib::from_slice::<T>` |
| `serde_norway::from_reader::<R, T>` | `noyalib::from_reader::<R, T>` |
| `serde_norway::from_value::<T>` | `noyalib::from_value::<T>` |
| `serde_norway::to_string` | `noyalib::to_string` |
| `serde_norway::to_writer` | `noyalib::to_writer` |
| `serde_norway::to_value` | `noyalib::to_value` |
| `serde_norway::Value` | `noyalib::Value` (same 7 variants) |
| `serde_norway::Mapping` | `noyalib::Mapping` |
| `serde_norway::Number` | `noyalib::Number` |
| `serde_norway::Error` | `noyalib::Error` |
| `serde_norway::Deserializer` | `noyalib::Deserializer` |
| `serde_norway::Serializer` | `noyalib::Serializer` |
| `serde_norway::with::singleton_map*` | `noyalib::with::singleton_map*` |
| (n/a) | `noyalib::from_str_strict::<T>` — error on unknown keys |
| (n/a) | `noyalib::Spanned<T>` — source-location wrapper |
| (n/a) | `noyalib::cst::Document` — lossless byte-faithful edits |
| (n/a) | `noyalib::ParserConfig` — explicit resource limits |

`serde_norway::Value` is the same 7-variant enum: `Null`,
`Bool`, `Number`, `String`, `Sequence`, `Mapping`,
`Tagged(Box<TaggedValue>)`. Match arms unchanged.

## Behavioural differences worth knowing

If you adopted `serde_norway` specifically because of the Norway
problem, the migration to `noyalib` keeps that fix in
place — `noyalib` defaults to YAML 1.2 strict booleans (only
`true` / `false` parse as `Value::Bool`). `country: NO` stays
`Value::String("NO")` under both crates.

The other two behavioural notes that apply to every `serde_yaml`
fork apply here too. Full discussion in
[`MIGRATION-FROM-SERDE-YAML.md`](MIGRATION-FROM-SERDE-YAML.md):

1. **`Value::Tagged` is preserved through the `Value` data path.**
   `from_str::<Value>("!Custom 'hi'\n")` returns
   `Value::Tagged(t)` (`t.tag() == "!Custom"`, `t.value() == Value::String("hi")`). Call
   `.untag()` to get the inner if you used the unwrapped form.
2. **YAML 1.2 strict booleans by default.** Same default as
   `serde_norway` — no behavioural change.
3. **Multi-doc streams use `load_all_as`.** `noyalib::load_all_as::<T>(input)`
   returns `Vec<T>` directly.

## Drop-in compatibility shim

```toml
[dependencies]
noyalib = { version = "0.0", features = ["compat-serde-yaml"] }
```

```diff
-use serde_norway::{from_str, to_string, Value, Mapping, Number};
+use noyalib::compat::serde_yaml::{from_str, to_string, Value, Mapping, Number};
```

## Migration checklist

- [ ] Replace `serde_norway = "0.9"` → `noyalib = "0.0"` (or
      `noyalib = { version = "0.0", features = ["compat-serde-yaml"] }`
      for the shim path).
- [ ] Replace `use serde_norway::*;` → `use noyalib::*;` (or
      `use noyalib::compat::serde_yaml::*;`).
- [ ] If you exhaustively match `Value`, the 7-arm shape is
      unchanged.
- [ ] No boolean-handling decision needed — both crates default
      to YAML 1.2 strict.
- [ ] Run your existing test suite.

If you hit a snag, file an issue at
<https://github.com/sebastienrousseau/noyalib/issues>.
