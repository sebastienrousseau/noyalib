<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migrating from `serde_yml` to `noyalib`

[`serde_yml`](https://crates.io/crates/serde_yml) was a
continuation fork of `serde_yaml` after upstream archival. The
`serde_yml` repo was itself **archived 2025-09-03**; `0.0.12`
(2024-08-25) is the last published release. The fork inherits
the full `serde_yaml` 0.9 API shape including the 7-variant
`Value` and the `with::singleton_map*` family.

`noyalib` is a clean-room reimplementation of YAML 1.2 with the
same `serde` data model, no `unsafe` code, and no `libyaml` C
dependency.

> **Crates.io / docs.rs state verified 2026-05-08.** If the
> upstream API has shifted since, file an issue and we'll update.

## TL;DR

```diff
-[dependencies]
-serde_yml = "0.0"
+[dependencies]
+noyalib = "0.0"
```

```diff
-use serde_yml::Value;
-let v: Value = serde_yml::from_str(input)?;
-let s        = serde_yml::to_string(&v)?;
+use noyalib::Value;
+let v: Value = noyalib::from_str(input)?;
+let s        = noyalib::to_string(&v)?;
```

That is the entire migration for the typical call site —
`serde_yml`'s public surface mirrors `serde_yaml`'s
function-for-function and `noyalib`'s does the same.

## Function-by-function mapping

| `serde_yml` 0.0 | `noyalib` |
|---|---|
| `serde_yml::from_str::<T>` | `noyalib::from_str::<T>` |
| `serde_yml::from_slice::<T>` | `noyalib::from_slice::<T>` |
| `serde_yml::from_reader::<R, T>` | `noyalib::from_reader::<R, T>` |
| `serde_yml::from_value::<T>` | `noyalib::from_value::<T>` |
| `serde_yml::to_string` | `noyalib::to_string` |
| `serde_yml::to_writer` | `noyalib::to_writer` |
| `serde_yml::to_value` | `noyalib::to_value` |
| `serde_yml::Value` | `noyalib::Value` (same 7 variants) |
| `serde_yml::Mapping` | `noyalib::Mapping` |
| `serde_yml::Number` | `noyalib::Number` |
| `serde_yml::Error` | `noyalib::Error` |
| `serde_yml::Deserializer` | `noyalib::Deserializer` |
| `serde_yml::Serializer` | `noyalib::Serializer` |
| `serde_yml::with::singleton_map` | `noyalib::with::singleton_map` |
| `serde_yml::with::singleton_map_recursive` | `noyalib::with::singleton_map_recursive` |
| `serde_yml::with::singleton_map_optional` | `noyalib::with::singleton_map_optional` |
| `serde_yml::with::singleton_map_with` | `noyalib::with::singleton_map_with` |
| (n/a) | `noyalib::from_str_strict::<T>` — error on unknown keys |
| (n/a) | `noyalib::Spanned<T>` — source-location wrapper |
| (n/a) | `noyalib::cst::Document` — lossless byte-faithful edits |
| (n/a) | `noyalib::ParserConfig` — explicit resource limits |

`serde_yml`'s `Value` enum has the same 7 variants as
`serde_yaml`'s and `noyalib`'s — `Null`, `Bool`, `Number`,
`String`, `Sequence`, `Mapping`, `Tagged(Box<TaggedValue>)`. No
match-arm changes are required.

## Behavioural differences worth knowing

The same three behavioural notes that apply to the
`serde_yaml` migration apply here. See
[`MIGRATION-FROM-SERDE-YAML.md`](MIGRATION-FROM-SERDE-YAML.md)
for the full discussion; the headlines:

1. **`Value::Tagged` is preserved through the `Value` data path.**
   `noyalib::from_str::<Value>("!Custom 'hi'\n")` returns
   `Value::Tagged(t)` (`t.tag() == "!Custom"`, `t.value() == Value::String("hi")`), not the
   transparent-unwrapped `Value::String("hi")`. `serde_yml`'s
   behaviour matched `serde_yaml`'s pre-Tagged behaviour. To get
   the unwrapped string, call `value.untag().as_str()`.
2. **YAML 1.2 strict booleans by default.** `country: NO` parses
   as `Value::String("NO")`, not `Value::Bool(false)`. Opt into
   YAML 1.1 boolean recognition via
   `ParserConfig::new().legacy_booleans(true)` if you need the
   old behaviour for round-trip compatibility with legacy
   files.
3. **Multi-doc streams use `load_all_as`.** `serde_yml`'s
   `Deserializer::from_str(...).map(...)` pattern maps to
   `noyalib::load_all_as::<T>(input)?` returning `Vec<T>`.

## Drop-in compatibility shim

Because `serde_yml`'s API is a verbatim mirror of `serde_yaml`'s,
the same compat shim works:

```toml
[dependencies]
noyalib = { version = "0.0", features = ["compat-serde-yaml"] }
```

```diff
-use serde_yml::{from_str, to_string, Value, Mapping, Number};
+use noyalib::compat::serde_yaml::{from_str, to_string, Value, Mapping, Number};
```

The shim re-exports noyalib-native types under
`noyalib::compat::serde_yaml::*` — no transitive dependency on
the archived `serde_yaml` 0.9 nor `serde_yml` 0.0.

## Migration checklist

- [ ] Replace `serde_yml = "0.0"` → `noyalib = "0.0"` (or
      `noyalib = { version = "0.0", features = ["compat-serde-yaml"] }`
      for the shim path).
- [ ] Replace `use serde_yml::*;` → `use noyalib::*;` (or
      `use noyalib::compat::serde_yaml::*;`).
- [ ] If you exhaustively match `Value`, the 7 variant arms are
      already correct — no shape change.
- [ ] Decide YAML 1.1-vs-1.2 boolean handling: stay on YAML 1.2
      strict (the safer default) or opt into
      `legacy_booleans(true)`.
- [ ] Run `cargo audit`. The migration removes any transitive
      `serde_yaml` 0.9 advisory exposure.
- [ ] Run your existing test suite. Failures almost always
      trace to one of the three behavioural notes above.

If you hit a snag, file an issue at
<https://github.com/sebastienrousseau/noyalib/issues> with the
input that misbehaves.
