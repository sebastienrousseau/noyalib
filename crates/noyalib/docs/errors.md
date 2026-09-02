# noyalib::Error reference

Every error variant the library can emit, what triggers it, and
how to recover. The `Error` enum is `#[non_exhaustive]` —
downstream `match` should always have a `_` arm — so this list
captures the surface as of v0.0.1; future variants may be added
in a minor release.

The `Display` output and `miette` `code` / `help` annotations are
stable across patch versions; the variant *names* are part of the
public API and follow semver.

## Variant reference

| Variant | Display prefix | When emitted | Recovery |
|---|---|---|---|
| `Parse(String)` | `YAML parse error: …` | Generic scanner / parser failure with no precise location available | Inspect the message; usually indicates malformed YAML |
| `ParseWithLocation { message, location }` | `YAML parse error at L:C: …` | Scanner / parser failure with byte-precise source location | Use `Error::format_with_source(input)` to get a caret-pointed render; fix the source at `L:C` |
| `Serialize(String)` | `serialization error: …` | Failure inside `to_string` / `to_writer` — usually a `Serialize` impl returning an error | Check the upstream `Serialize` impl |
| `Deserialize(String)` | `deserialization error: …` | Generic deserialise failure with no location | Inspect the message; often a type-shape mismatch the resolver couldn't pinpoint |
| `DeserializeWithLocation { message, location }` | `deserialization error at L:C: …` | Deserialise failure with span attached (loader path) | Use `format_with_source` to point at the offending value |
| `Io(std::io::Error)` (std-feature only) | `I/O error: …` | `from_reader` / `to_writer` underlying I/O failed | Check the inner `io::Error::kind()` |
| `Custom(String)` | message verbatim | User-supplied error text via `serde_core::de::Error::custom` | Inspect message |
| `RecursionLimitExceeded { depth }` | `recursion depth limit exceeded: <n>` | Document nesting beyond `ParserConfig::max_depth` (default 128) | Increase `max_depth` if input is trusted; otherwise reject the input |
| `DuplicateKey(String)` | `duplicate key: <name>` | Mapping has the same key twice and `DuplicateKeyPolicy::Error` is set | Switch to `Last` or `First` policy, or fix the source |
| `RepetitionLimitExceeded` | `alias expansion limit exceeded` | Alias-expansion total exceeded `max_alias_expansions` (billion-laughs defence) | Increase the limit if input is trusted; otherwise reject |
| `UnknownAnchor(String)` | `unknown anchor: <name>` | `*name` references an anchor not previously defined in the document | Check anchor name spelling; ensure the `&name` definition precedes the `*name` reference |
| `UnknownAnchorAt { name, location, suggestion }` | `unknown anchor: <name> at L:C` | Same as `UnknownAnchor` but with span and Levenshtein-suggestion attached | Use the `suggestion` field for an autocorrect prompt; fix or define the anchor |
| `MissingField(String)` | `missing field: <name>` | Deserialising to a struct that requires a field absent from the YAML | Add the field to YAML or `#[serde(default)]` on the struct field |
| `UnknownField(String)` | `unknown field: <name>` | `from_str_strict` saw a key the target struct doesn't declare | Fix the typo or remove the key; or stop using `_strict` |
| `ScalarInMergeElement` | `scalar in merge element` | `<<:` value contains a scalar where mapping was required | Use a mapping or sequence-of-mappings as the merge value |
| `SequenceInMergeElement` | `sequence in merge element` | A sequence appears where the merge element expected a mapping | Restructure the merge to use mappings |
| `TaggedInMerge` | `tagged value in merge` | A `!tag`-annotated value appeared inside a merge expression | Strip the tag or restructure |
| `Invalid(String)` | `invalid YAML: …` | Generic invalid-construct catch-all (e.g. malformed flow-style) | Inspect message; fix source |
| `TypeMismatch { expected, found }` | `type mismatch: expected <t>, found <u>` | A typed deserialise expected `expected` but the YAML had `found` | Fix the YAML shape or change the target type |
| `Shared(Arc<Error>)` | delegates to inner | Multi-consumer error sharing (parallel paths) | Unwrap with `Arc::unwrap_or_clone` if you need ownership |
| `EndOfStream` | `unexpected end of stream` | Stream ended mid-document (truncated YAML) | Check the source for truncation; for streaming consumers, signal upstream that more bytes are needed |
| `MoreThanOneDocument` | `multiple documents in stream; expected exactly one` | `from_str` saw `---` with more than one document but only one was expected | Use `noyalib::load_all` / `load_all_as` for multi-doc streams |
| `ScalarInMerge` (legacy) | `scalar in merge` | Legacy variant kept for back-compat; superseded by `ScalarInMergeElement` | Same recovery as `ScalarInMergeElement` |
| `EmptyTag` | `empty tag` | A `!` was followed by no tag handle or suffix | Provide a tag (`!shopping`) or remove the indicator |
| `FailedToParseNumber(String)` | `failed to parse number: …` | A scalar tagged `!!int` or `!!float` couldn't be parsed | Check the literal syntax; for hex/oct, use `0x` / `0o` prefixes (or enable `legacy_octal_numbers` for bare `0644`) |
| `Message(String, Option<usize>)` | `serde error: …` | `serde_core::de::Error::custom` adapter with optional byte offset | Inspect message |

## Working with errors

### Get the location, if any

```rust
use noyalib::{from_str, Value};

let err = from_str::<Value>("a: [unclosed").unwrap_err();
if let Some(loc) = err.location() {
    eprintln!("error at line {}, column {}", loc.line(), loc.column());
}
```

### Render with source context

```rust
let source = "a: [unclosed";
let err = noyalib::from_str::<noyalib::Value>(source).unwrap_err();
let pretty = err.format_with_source(source);
eprintln!("{pretty}");
// error: YAML parse error at line 1:5: …
//   --> line 1:5
//   a: [unclosed
//       ^
```

### Render with rustc-style multi-line context

```rust
let pretty = err.format_with_source_radius(source, /* radius = */ 2);
```

### miette integration (with the `miette` feature)

`Error` implements `miette::Diagnostic` automatically when the
`miette` feature is on. Each variant carries:

- A stable error code (e.g. `noyalib::unknown_anchor`)
- A `help` string with concrete recovery action
- Source-location-attached `LabeledSpan`s for span-bearing variants

```sh
noyavalidate --schema schema.yaml input.yaml
# emits the full miette fancy renderer output
```

### Pattern-match safely

Because `Error` is `#[non_exhaustive]`, downstream `match` must
include a `_` arm:

```rust
use noyalib::Error;

match err {
    Error::Parse(_) | Error::ParseWithLocation { .. } => {
        // syntactic problem
    }
    Error::TypeMismatch { expected, found } => {
        eprintln!("expected {expected}, got {found}");
    }
    Error::UnknownField(name) => {
        eprintln!("unknown field: {name}");
    }
    _ => {
        // Future variants land here without breaking your match.
        eprintln!("unhandled error: {err}");
    }
}
```

## Source chains

Two variants chain to inner errors:

- `Error::Io(io_err)` — `source()` returns the underlying
  `std::io::Error`
- `Error::Shared(arc)` — `source()` returns the inner `Error`
  through the `Arc`

All other variants own their data and have no source.

## Error stability policy

Per [SECURITY.md](../../../SECURITY.md) and the workspace semver
guarantees:

- Variant names are public API; renaming or removing one is a
  major-version break.
- Adding new variants is a minor-version bump (the
  `#[non_exhaustive]` attribute makes this safe).
- `Display` output and `miette` codes / help strings are stable
  across patch versions but may be improved in minors.
- `Location` line/column are 1-indexed; byte index is 0-indexed.
  This shape matches `serde_yaml` 0.9 by-byte.

For the full migration mapping from `serde_yaml::Error` to
`noyalib::Error` see
[`docs/MIGRATION-FROM-SERDE-YAML.md`](../../../docs/MIGRATION-FROM-SERDE-YAML.md).
