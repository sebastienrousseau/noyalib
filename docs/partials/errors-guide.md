<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!--
Prose appendix for docs/errors.md.

scripts/generate-reference-docs.sh generates the inventory half of
that document (kinds, variants) and appends this file verbatim. Edit
the guidance here; never edit docs/errors.md, which is overwritten.

This file is not in the readme-examples CI list because its blocks are
checked in their assembled position, as part of docs/errors.md.
-->

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

<!-- doctest-preamble
let source = "a: [unclosed";
let err = noyalib::from_str::<noyalib::Value>(source).unwrap_err();
-->
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

<!-- doctest-preamble
let err = noyalib::from_str::<noyalib::Value>("a: [unclosed").unwrap_err();
-->
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
        eprintln!("unhandled error");
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

Per [SECURITY.md](../SECURITY.md) and the workspace semver
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
[`MIGRATION-FROM-SERDE-YAML.md`](MIGRATION-FROM-SERDE-YAML.md).
