<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migrating from `serde-yaml-ng` to `noyalib`

[`serde-yaml-ng`](https://crates.io/crates/serde-yaml-ng)
(crate name `serde_yaml_ng`) is `acatton`'s independent
continuation of `serde_yaml`. The README markets it as "as
compatible as possible" with the original. `0.10.0`
(2024-05-26) is the most recent release — actively maintained
but the cadence is slow.

`noyalib` is a clean-room reimplementation of YAML 1.2 with the
same `serde` data model, no `unsafe` code, and no `libyaml` C
dependency.

> **Crates.io / docs.rs state verified 2026-05-08.** If the
> upstream API has shifted since, file an issue and we'll update.

## TL;DR

```diff
-[dependencies]
-serde_yaml_ng = "0.10"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_yaml_ng::Value;
-let v: Value = serde_yaml_ng::from_str(input)?;
-let s        = serde_yaml_ng::to_string(&v)?;
+use noyalib::Value;
+let v: Value = noyalib::from_str(input)?;
+let s        = noyalib::to_string(&v)?;
```

## Function-by-function mapping

`serde_yaml_ng` mirrors `serde_yaml` 0.9 exactly.

| `serde_yaml_ng` 0.10 | `noyalib` |
|---|---|
| `serde_yaml_ng::from_str::<T>` | `noyalib::from_str::<T>` |
| `serde_yaml_ng::from_slice::<T>` | `noyalib::from_slice::<T>` |
| `serde_yaml_ng::from_reader::<R, T>` | `noyalib::from_reader::<R, T>` |
| `serde_yaml_ng::from_value::<T>` | `noyalib::from_value::<T>` |
| `serde_yaml_ng::to_string` | `noyalib::to_string` |
| `serde_yaml_ng::to_writer` | `noyalib::to_writer` |
| `serde_yaml_ng::to_value` | `noyalib::to_value` |
| `serde_yaml_ng::Value` | `noyalib::Value` (same 7 variants) |
| `serde_yaml_ng::Mapping` | `noyalib::Mapping` |
| `serde_yaml_ng::Number` | `noyalib::Number` (default `Integer` / `Float`; opt-in `Unsigned(u64)` behind `lossless-u64`) |
| `serde_yaml_ng::Error` | `noyalib::Error` |
| `serde_yaml_ng::Deserializer` | `noyalib::Deserializer` |
| `serde_yaml_ng::Serializer` | `noyalib::Serializer` |
| `serde_yaml_ng::with::singleton_map*` | `noyalib::with::singleton_map*` |
| (n/a) | `noyalib::from_str_strict::<T>` — error on unknown keys |
| (n/a) | `noyalib::Spanned<T>` — source-location wrapper |
| (n/a) | `noyalib::cst::Document` — lossless byte-faithful edits |
| (n/a) | `noyalib::ParserConfig` — explicit resource limits |

`serde_yaml_ng::Value` is the same 7-variant enum: `Null`,
`Bool`, `Number`, `String`, `Sequence`, `Mapping`,
`Tagged(Box<TaggedValue>)`. Match arms unchanged.

## Behavioural differences worth knowing

The three notes that apply to the `serde_yaml` migration apply
here. Full discussion in
[`MIGRATION-FROM-SERDE-YAML.md`](MIGRATION-FROM-SERDE-YAML.md);
headlines:

1. **`Value::Tagged` is preserved.** `from_str::<Value>("!Custom 'hi'\n")`
   returns `Value::Tagged(t)` (`t.tag() == "!Custom"`, `t.value() == Value::String("hi")`).
   `serde_yaml_ng` matched `serde_yaml`'s pre-Tagged behaviour;
   noyalib preserves the wrapper. Call `.untag()` to get the
   inner.
2. **YAML 1.2 strict booleans by default.** `country: NO` stays
   `Value::String("NO")`. Opt into YAML 1.1 boolean recognition
   via `ParserConfig::new().legacy_booleans(true)`.
3. **Multi-doc streams** — use `noyalib::load_all_as::<T>(input)`
   for `Vec<T>` of every document in the stream.

## Drop-in compatibility shim

```toml
[dependencies]
noyalib = { version = "0.0", features = ["compat-serde-yaml"] }
```

```diff
-use serde_yaml_ng::{from_str, to_string, Value, Mapping, Number};
+use noyalib::compat::serde_yaml::{from_str, to_string, Value, Mapping, Number};
```

The shim is a noyalib-native re-export under the
`compat::serde_yaml` path; no transitive `serde_yaml_ng`
dependency.

## Migration checklist

- [ ] Replace `serde_yaml_ng = "0.10"` → `noyalib = "0.0"` (or
      `noyalib = { version = "0.0", features = ["compat-serde-yaml"] }`
      for the shim path).
- [ ] Replace `use serde_yaml_ng::*;` → `use noyalib::*;` (or
      `use noyalib::compat::serde_yaml::*;`).
- [ ] If you exhaustively match `Value`, the 7-arm shape is
      unchanged.
- [ ] Decide YAML 1.1-vs-1.2 boolean handling.
- [ ] Run your existing test suite.

If you hit a snag, file an issue at
<https://github.com/sebastienrousseau/noyalib/issues>.
