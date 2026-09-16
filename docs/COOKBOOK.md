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

# if false {
let cfg: Config = from_str(std::fs::read_to_string("app.yaml")?.as_str())?;
# }
# let cfg: Config = from_str("name: api\nport: 8080\n")?;
# assert_eq!(cfg.port, 8080);
```

Example: `hello`, `config_macros`.

### Refuse a misspelt key instead of defaulting it

```rust
# #[derive(serde::Deserialize)]
# struct Config { name: String, port: u16 }
# let text = "name: api\nport: 8080\n";
let cfg: Config = noyalib::from_str_strict(text)?;
// error: unknown field `retires`, did you mean `retries`?
```

Example: `strict_deserialise`, `suggest`.

### Read every document in a stream

```rust
# #[derive(serde::Deserialize)]
# struct Manifest { kind: String }
# fn deploy(_m: Manifest) -> Result<(), noyalib::Error> { Ok(()) }
# let text = "kind: Service\n---\nkind: Ingress\n";
for doc in noyalib::load_all_as::<Manifest>(text)? { deploy(doc)?; }
```

Example: `stream`, `read_iterator`. For very large streams split across
threads: `parallel`.

### Keep the line and column of every value

```rust
use noyalib::Spanned;
#[derive(serde::Deserialize)]
struct Rule { name: Spanned<String>, limit: Spanned<u64> }
# let r: Rule = noyalib::from_str("name: cap\nlimit: 5\n")?;
# assert_eq!(r.limit.start.line(), 2);
```

Example: `source`, `diagnostic_path`, `errors`.

### Parse untrusted input with explicit limits

```rust
# let untrusted = "a: 1\n";
let cfg = noyalib::ParserConfig::new().max_depth(32).max_alias_expansions(64);
let v: noyalib::Value = noyalib::from_str_with_config(untrusted, &cfg)?;
```

Example: `harden_untrusted`, `secure`.

### Borrow instead of copy

```rust
# let text = "a: 1\n";
let v: noyalib::borrowed::BorrowedValue<'_> = noyalib::borrowed::from_str_borrowed(text)?;
```

Example: `zero_copy_borrow`, `borrow`.

## Writing

### Serialise a struct

```rust
# #[derive(serde::Serialize)]
# struct Config { name: String, port: u16 }
# let cfg = Config { name: "api".into(), port: 8080 };
let text = noyalib::to_string(&cfg)?;
```

Example: `emit`, `style` (block versus flow, quoting choices).

### Control the emitted layout

Example: `style`, `preserve` (key order, comments through the lossless
path).

## Editing without losing anything

### Bump one value and keep every other byte

```rust
# let text = "version: 0.0.43  # keep this comment\n";
use noyalib::cst::parse_document;
let mut doc = parse_document(text)?;
doc.set("version", "0.0.44")?;
# assert_eq!(doc.to_string(), "version: 0.0.44  # keep this comment\n");
# if false {
std::fs::write("Cargo.yaml", doc.to_string())?;
# }
```

Example: `lossless_edit`, `cst_surgical_edit`, `modify`.

### Add, rename, move and remove entries in the file's own style

Example: `entry_api` (the chainable handle), `rename`, `patch`.

### Edit inside flow collections

Example: `cst_wrapped_flow_edit`.

### Turn aliases into inline copies before shipping a manifest

```rust
# use noyalib::cst::parse_document;
# let mut doc = parse_document("shared: &shared\n  a: 1\nuse: *shared\n")?;
let n = doc.materialise_aliases_of("shared")?;
# assert_eq!(n, 1);
```

Example: `anchor_shared`, `alias`.

### Read and write comments

Example: `comments`, `comments_at`.

## Schemas and validation

### Validate a document against a JSON Schema

```rust
# let value: noyalib::Value = noyalib::from_str("port: \"nope\"\n")?;
# let schema: noyalib::Value = noyalib::from_str(
#     "type: object\nproperties:\n  port:\n    type: integer\n")?;
// One call, one error: stops at the first violation.
let ok = noyalib::validate_against_schema(&value, &schema).is_ok();

// Every violation instead, each with the JSON pointer that reached it.
let compiled = noyalib::CompiledSchema::compile(&schema)?;
for v in compiled.iter_errors(&value)? {
    eprintln!("{} at {}", v.message, v.instance_path);
}
# assert!(!ok);
```

Example: `schema_validation`, `validated_miette`.

### Generate a schema from your own types

```rust
# #[derive(noyalib::JsonSchema)]
# struct Config { port: u16 }
let schema = noyalib::schema_for::<Config>()?;
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
# assert_eq!(plain.as_str(), Some("#ff8800"));
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
