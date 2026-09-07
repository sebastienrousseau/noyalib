<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib cookbook

Task-shaped recipes. Every entry names the runnable example it is
distilled from, so the code below is never the only copy: run
`cargo run --example <name>` in `crates/noyalib` to see it work, and
read the example for the full version with error handling and comments.

## Reading

### Read a config file into a struct

```rust
use noyalib::from_str;

#[derive(serde::Deserialize)]
struct Config { name: String, port: u16 }

let cfg: Config = from_str(std::fs::read_to_string("app.yaml")?.as_str())?;
```

Example: `hello`, `config_macros`.

### Refuse a misspelt key instead of defaulting it

```rust
let cfg: Config = noyalib::from_str_strict(text)?;
// error: unknown field `retires`, did you mean `retries`?
```

Example: `strict_deserialise`, `suggest`.

### Read every document in a stream

```rust
for doc in noyalib::load_all::<Manifest>(text)? { deploy(doc)?; }
```

Example: `stream`, `read_iterator`. For very large streams split across
threads: `parallel`.

### Keep the line and column of every value

```rust
use noyalib::Spanned;
#[derive(serde::Deserialize)]
struct Rule { name: Spanned<String>, limit: Spanned<u64> }
```

Example: `source`, `diagnostic_path`, `errors`.

### Parse untrusted input with explicit limits

```rust
let cfg = noyalib::ParserConfig::new().max_depth(32).max_alias_expansions(64);
let v: noyalib::Value = cfg.from_str(untrusted)?;
```

Example: `harden_untrusted`, `secure`.

### Borrow instead of copy

```rust
let v: noyalib::BorrowedValue<'_> = noyalib::from_str_borrowed(text)?;
```

Example: `zero_copy_borrow`, `borrow`.

## Writing

### Serialise a struct

```rust
let text = noyalib::to_string(&cfg)?;
```

Example: `emit`, `style` (block versus flow, quoting choices).

### Control the emitted layout

Example: `style`, `preserve` (key order, comments through the lossless
path).

## Editing without losing anything

### Bump one value and keep every other byte

```rust
use noyalib::cst::parse_document;
let mut doc = parse_document(text)?;
doc.set("version", "0.0.39")?;
std::fs::write("Cargo.yaml", doc.to_string())?;
```

Example: `lossless_edit`, `cst_surgical_edit`, `modify`.

### Add, rename, move and remove entries in the file's own style

Example: `entry_api` (the chainable handle), `rename`, `patch`.

### Edit inside flow collections

Example: `cst_wrapped_flow_edit`.

### Turn aliases into inline copies before shipping a manifest

```rust
let n = doc.materialise_aliases_of("shared")?;
```

Example: `anchor_shared`, `alias`.

### Read and write comments

Example: `comments`, `comments_at`.

## Schemas and validation

### Validate a document against a JSON Schema

```rust
let report = noyalib::validate_against_schema(&value, &schema)?;
for v in report.violations() { eprintln!("{} at {}", v.message, v.path); }
```

Example: `schema_validation`, `validated_miette`.

### Generate a schema from your own types

```rust
let schema = noyalib::schema_for::<Config>();
```

Example: `schema`, `schema_ext`, `schema_compiled`.

### Fix what the schema says is wrong

Example: `validation` (`coerce_to_schema`), `validation_garde`,
`validation_validator`.

## Tags, merges and the odd corners of YAML

### Keep custom tags, or strip them all

```rust
let v: noyalib::Value = noyalib::from_str("!Color '#ff8800'")?;
let plain = v.untag();
```

Example: `tags`, `untagged`, `registry`, `variants`.

### Merge keys and anchors together

Example: `merge_keys_with_aliases`, `inherit`, `overlay`.

### Binary scalars

Example: `binary`.

### YAML 1.1 files (the Norway problem)

Example: `portable`, `smart`.

## Diagnostics

### Show an error with source context

Example: `diagnostic`, `ariadne_diagnostic`, `validated_miette`.

### Recover from a broken document in an editor

Example: `recovery_lenient`, `recursive`.

## Runtimes

### Parse on tokio without blocking

Example: `tokio_async_reader`, `async_io`.

### Build without the standard library

Example: `nostd`; the `noyalib-wasm` crate is the browser build of the same parser.

### Stream into structured logging with sval

Example: `sval_streaming`.

### Environment interpolation and includes

Example: `env`, `properties_interpolation`, `include`,
`include_directive`.

## Interop

### Convert to and from JSON and other serde formats

Example: `transcode`, `bridge`, `pipes`.

### Layered configuration with figment

Example: `figment`, `global`.

## Where next

- [USER-GUIDE.md](USER-GUIDE.md) for each feature in order.
- [MIGRATION.md](MIGRATION.md) if you are coming from another crate.
- [POLICIES.md](POLICIES.md) for every limit and its default.
