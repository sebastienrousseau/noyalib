<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Migrating from `yaml-spanned` to `noyalib`

[`yaml-spanned`](https://crates.io/crates/yaml-spanned) is
`romnn`'s span-aware YAML parser built on `libyaml-safer`.
`0.0.3` (2025-12-27) is the current release. The crate's value
proposition is preserving source spans on every value so
diagnostics can point at the exact line and column.

Two things to know up front:

1. **`yaml-spanned` does not provide `to_string` / `to_writer`.**
   It is a parser / DOM-construction crate, not a serializer.
   If you used it for read-only or DOM-walk workloads, you
   were probably pairing it with `serde_yaml` (or another
   crate) for the emit side.
2. **`yaml-spanned` is built on `libyaml-safer`, which uses
   `unsafe` internally.** noyalib is `#![forbid(unsafe_code)]`
   end-to-end; the migration tightens the safety guarantee.

`noyalib` is a clean-room reimplementation of YAML 1.2 with the
same `serde` data model, no `unsafe` code, no `libyaml` C
dependency, and a *typed* span wrapper (`Spanned<T>`) that
covers the same diagnostic use case.

> **Crates.io / docs.rs state verified 2026-05-08.** If the
> upstream API has shifted since, file an issue and we'll update.

## TL;DR

If you used `yaml-spanned` for span info on a typed config:

```diff
-use yaml_spanned::{from_str, Value};
-let v: Value = from_str(input)?;
-// Walk v.span() / v.value to extract span+payload.
+use noyalib::{from_str, Spanned};
+#[derive(serde::Deserialize)]
+struct Config { name: Spanned<String>, port: Spanned<u16> }
+let cfg: Config = from_str(input)?;
+let line = cfg.name.start.line();
+let col  = cfg.name.start.column();
```

If you used `yaml-spanned` for general dynamic inspection
without typed targets:

```diff
-use yaml_spanned::{from_str, Value};
-let v: Value = from_str(input)?;
+use noyalib::{from_str, Value};
+let v: Value = from_str(input)?;
```

`noyalib::Value` covers the same 7-variant DOM. For per-value
spans on the dynamic path, parse through the CST:

```rust
let doc = noyalib::cst::parse_document(input)?;
let span = doc.span_at("server.port");  // (start, end) byte offsets
```

## Function-by-function mapping

| `yaml_spanned` 0.0 | `noyalib` |
|---|---|
| `yaml_spanned::from_str(input)` (returns `Spanned<Value>`) | `noyalib::from_str::<Spanned<T>>(input)` for typed; `from_str::<Value>(input)` for dynamic |
| `yaml_spanned::from_str_all(input)` | `noyalib::load_all_as::<Spanned<T>>(input)` |
| `yaml_spanned::from_str_lossy(input)` (drop-on-error) | `noyalib::load_all(input)` (yields `Result` per doc — handle errors explicitly rather than silently dropping) |
| `yaml_spanned::from_str_lossy_all(input)` | as above with `Vec`-collect |
| `yaml_spanned::from_str_lossy_iter(input)` | as above with iterator-style consumption |
| `yaml_spanned::from_value::<T>(spanned_value)` | `noyalib::from_value::<T>(&value)` |
| `yaml_spanned::to_value(t)` | `noyalib::to_value(&t)` |
| `yaml_spanned::Value` | `noyalib::Value` (same 7 variants) |
| `yaml_spanned::TaggedValue` | `noyalib::TaggedValue` |
| `yaml_spanned::Tag` | `noyalib::Tag` |
| `yaml_spanned::Builder` (programmatic Value construction) | `noyalib::Value::Mapping(…)` and the `From` / `Into` impls; `noyalib::cst::Document` for source-faithful programmatic edits |
| (no `to_string`) | `noyalib::to_string` — full serializer |
| (no `to_writer`) | `noyalib::to_writer` |
| (no `to_string_multi` / `to_writer_multi`) | `noyalib::to_string_multi` / `noyalib::to_writer_multi` |

### Span access

`yaml-spanned` returns a `Spanned<Value>` wrapper from every
parse entry. To read both the value and the source span, you
unpack the wrapper.

`noyalib::Spanned<T>` plays the same role at the typed deserialise
layer:

```rust
use noyalib::Spanned;
use serde::Deserialize;

#[derive(Deserialize)]
struct ServerConfig {
    host: Spanned<String>,
    port: Spanned<u16>,
}

let cfg: ServerConfig = noyalib::from_str(input)?;
println!(
    "host at line {}, col {}",
    cfg.host.start.line(),
    cfg.host.start.column(),
);
println!(
    "port range: bytes {}..{}",
    cfg.port.start.index(),
    cfg.port.end.index(),
);
```

For dynamic-path workloads (where `yaml-spanned` returned a
`Spanned<Value>`), use `noyalib::cst::parse_document` and query
spans by path:

```rust
let doc = noyalib::cst::parse_document(input)?;
if let Some((start, end)) = doc.span_at("server.host") {
    println!("server.host span: bytes {start}..{end}");
}
```

The CST layer also exposes comments at each node
(`doc.comments_at(path)`), which `yaml-spanned` does not
preserve.

### Lossy iterators

`yaml-spanned`'s `from_str_lossy*` family silently drops
documents that fail to parse. noyalib's `load_all` returns a
`DocumentIterator` yielding `Result<Value>` — the caller decides
whether to drop, log, or fail-fast:

```diff
-let docs: Vec<Spanned<Value>> = yaml_spanned::from_str_lossy_all(input);
+let docs: Vec<noyalib::Value> = noyalib::load_all(input)?
+    .filter_map(|res| res.ok())
+    .collect();
```

The explicit form is recommended — silent error-drop in a
parser is rarely what users actually want.

### `Builder` ↔ `Value` construction

`yaml-spanned`'s `Builder` lets you construct a `Value` tree
programmatically. noyalib exposes the same surface via the
`From` / `Into` impls and the variant constructors:

```diff
-let v = Builder::mapping()
-    .entry("name", Builder::string("noyalib"))
-    .entry("port", Builder::integer(8080))
-    .build();
+let mut m = noyalib::Mapping::new();
+m.insert("name".into(), noyalib::Value::from("noyalib"));
+m.insert("port".into(), noyalib::Value::from(8080_i64));
+let v = noyalib::Value::Mapping(m);
```

For source-faithful edits (preserving comments and
formatting), use `noyalib::cst::Document` instead — the
programmatic `Builder` shape doesn't track source bytes.

## Behavioural differences worth knowing

1. **`#![forbid(unsafe_code)]`.** `yaml-spanned` transitively
   uses `unsafe` (via `libyaml-safer`). noyalib forbids it
   throughout the workspace; the migration tightens the
   safety claim.
2. **Full YAML 1.2 surface.** noyalib supports YAML 1.2 strict
   booleans, the JSON-compatible schema, custom tag handling
   via `TagRegistry`, anchor / alias resolution, and merge
   keys (`<<`) — all of which `yaml-spanned` either inherits
   from `libyaml-safer` or doesn't expose.
3. **Spans on the typed path.** `Spanned<T>` is a serde
   wrapper, so it composes with any `#[derive(Deserialize)]`
   target — not just the dynamic `Value`.
4. **Comments preserved at the CST layer.**
   `noyalib::cst::Document::comments_at(path)` returns the
   comments attached to a path; useful for round-trip-faithful
   editors / formatters / linters.
5. **Built-in serialization.** `yaml-spanned` is parser-only;
   noyalib provides `to_string` / `to_writer` /
   `to_string_multi` / `to_writer_multi` /
   `to_fmt_writer` for the full emit surface.

## Drop-in compatibility shim

There's no `yaml-spanned`-shaped shim because the surface is
small. If you previously routed all your span work through
`yaml_spanned::Spanned<Value>`, the noyalib equivalent is
`noyalib::Spanned<Value>` — the same wrapper type at the same
position in the type system.

## Migration checklist

- [ ] Replace `yaml_spanned = "0.0"` → `noyalib = "0.0"`.
- [ ] Replace `use yaml_spanned::*;` → `use noyalib::*;` plus
      `use noyalib::Spanned;` where you wrap span-bearing fields.
- [ ] Where you used `Spanned<Value>` for span+dynamic, prefer
      a typed config with `Spanned<T>` per field, or fall back
      to `noyalib::cst::parse_document` for path-based span
      queries.
- [ ] If you used `from_str_lossy*`, replace with explicit
      `load_all(input)?.filter_map(...)` — silent drops are
      rarely what production code wants.
- [ ] If you used `Builder` for programmatic construction,
      switch to `Value::Mapping(...)` / `From<T>` impls (or
      `cst::Document` for source-faithful edits).
- [ ] Add `to_string` / `to_writer` calls if the project
      previously paired `yaml-spanned` (read) with another
      crate (write) — noyalib covers both sides.
- [ ] Run your existing test suite.

If you hit a snag, file an issue at
<https://github.com/sebastienrousseau/noyalib/issues>.
