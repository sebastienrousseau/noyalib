<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migrating from `yaml_serde` to `noyalib`

[`yaml_serde`](https://crates.io/crates/yaml_serde) (also
resolvable as `yaml-serde`) is the actively maintained fork of
`serde_yaml` under the `yaml` org. `0.10.4` (2026-03-11) is the
current release. The fork advertises itself as a true drop-in
for `serde_yaml` and documents a Cargo `package =` rename so
existing `use serde_yaml::*;` imports keep compiling without
touching call sites.

`noyalib` is a clean-room reimplementation of YAML 1.2 with the
same `serde` data model, no `unsafe` code, and no `libyaml` C
dependency.

> **Crates.io / docs.rs state verified 2026-05-08.** If the
> upstream API has shifted since, file an issue and we'll update.

## TL;DR

```diff
-[dependencies]
-yaml_serde = "0.10"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use yaml_serde::Value;
-let v: Value = yaml_serde::from_str(input)?;
-let s        = yaml_serde::to_string(&v)?;
+use noyalib::Value;
+let v: Value = noyalib::from_str(input)?;
+let s        = noyalib::to_string(&v)?;
```

If you adopted `yaml_serde` via the Cargo package-rename trick
(keeping `use serde_yaml::*;` in source), drop the rename:

```diff
-[dependencies]
-serde_yaml = { package = "yaml_serde", version = "0.10" }
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_yaml::{from_str, to_string, Value};
+use noyalib::{from_str, to_string, Value};
```

## Function-by-function mapping

`yaml_serde` mirrors `serde_yaml` 0.9 exactly. Every entry below
holds for both spellings; substitute your import path.

| `yaml_serde` 0.10 | `noyalib` |
|---|---|
| `yaml_serde::from_str::<T>` | `noyalib::from_str::<T>` |
| `yaml_serde::from_slice::<T>` | `noyalib::from_slice::<T>` |
| `yaml_serde::from_reader::<R, T>` | `noyalib::from_reader::<R, T>` |
| `yaml_serde::from_value::<T>` | `noyalib::from_value::<T>` |
| `yaml_serde::to_string` | `noyalib::to_string` |
| `yaml_serde::to_writer` | `noyalib::to_writer` |
| `yaml_serde::to_value` | `noyalib::to_value` |
| `yaml_serde::Value` | `noyalib::Value` (same 7 variants) |
| `yaml_serde::Mapping` | `noyalib::Mapping` |
| `yaml_serde::Number` | `noyalib::Number` |
| `yaml_serde::Error` | `noyalib::Error` |
| `yaml_serde::Deserializer` | `noyalib::Deserializer` |
| `yaml_serde::Serializer` | `noyalib::Serializer` |
| `yaml_serde::with::singleton_map` | `noyalib::with::singleton_map` |
| `yaml_serde::with::singleton_map_recursive` | `noyalib::with::singleton_map_recursive` |
| `yaml_serde::with::singleton_map_optional` | `noyalib::with::singleton_map_optional` |
| `yaml_serde::with::singleton_map_with` | `noyalib::with::singleton_map_with` |
| (n/a) | `noyalib::from_str_strict::<T>` — error on unknown keys |
| (n/a) | `noyalib::Spanned<T>` — source-location wrapper |
| (n/a) | `noyalib::cst::Document` — lossless byte-faithful edits |
| (n/a) | `noyalib::ParserConfig` — explicit resource limits |

`yaml_serde::Value` is the same 7-variant enum as
`serde_yaml::Value` and `noyalib::Value`: `Null`, `Bool`,
`Number`, `String`, `Sequence`, `Mapping`,
`Tagged(Box<TaggedValue>)`.

## Behavioural differences worth knowing

The three behavioural notes that apply to the `serde_yaml`
migration apply here. The full discussion lives in
[`MIGRATION-FROM-SERDE-YAML.md`](MIGRATION-FROM-SERDE-YAML.md);
the headlines:

1. **`Value::Tagged` is preserved through the `Value` data path.**
   `noyalib::from_str::<Value>("!Custom 'hi'\n")` returns
   `Value::Tagged(t)` (`t.tag() == "!Custom"`, `t.value() == Value::String("hi")`). To get
   the unwrapped inner, call `value.untag().as_str()`.
2. **YAML 1.2 strict booleans by default.** `country: NO`
   stays `Value::String("NO")`. Opt into legacy YAML 1.1
   boolean recognition via
   `ParserConfig::new().legacy_booleans(true)`.
3. **Multi-doc streams use `load_all_as`.** `noyalib::load_all_as::<T>(input)`
   returns `Vec<T>` directly.

## Drop-in compatibility shim

If you adopted `yaml_serde` to keep `use serde_yaml::*;`
imports compiling and you can't change call sites at all,
the noyalib compat shim does the same:

```toml
[dependencies]
noyalib = { version = "0.0", features = ["compat-serde-yaml"] }
```

```diff
-use serde_yaml::{from_str, to_string, Value, Mapping, Number};
+use noyalib::compat::serde_yaml::{from_str, to_string, Value, Mapping, Number};
```

The shim re-exports noyalib-native types under
`noyalib::compat::serde_yaml::*` — no transitive dep on
either upstream.

## Migration checklist

- [ ] Replace `yaml_serde = "0.10"` (or the package-rename
      pattern under `serde_yaml = { package = "yaml_serde" }`)
      → `noyalib = "0.0"`.
- [ ] Replace `use yaml_serde::*;` → `use noyalib::*;` (or
      `use noyalib::compat::serde_yaml::*;` for the shim path).
- [ ] If you exhaustively match `Value`, the 7 variant arms are
      already correct — no shape change.
- [ ] Decide YAML 1.1-vs-1.2 boolean handling: stay on YAML 1.2
      strict (the safer default) or opt into
      `legacy_booleans(true)`.
- [ ] Run your existing test suite. Failures almost always
      trace to one of the three behavioural notes above.

If you hit a snag, file an issue at
<https://github.com/sebastienrousseau/noyalib/issues> with the
input that misbehaves.
