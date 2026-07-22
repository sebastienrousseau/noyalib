# Glossary

Domain vocabulary used across noyalib's documentation, code, and
commit messages. When a term has both a YAML-spec meaning and a
noyalib-specific meaning, both are listed.

## YAML specification terms

**Anchor.** A `&name` marker placed before a node so the node can be
referenced later via an *alias*. Per the YAML 1.2 spec each document
has its own anchor namespace; noyalib resets the anchor table on
`DocumentEnd`.

**Alias.** A `*name` reference that resolves to the value previously
labelled by an anchor of the same name within the same document.
noyalib resolves aliases eagerly on both the owned (`Value`) and
borrowed (`BorrowedValue`) paths.

**Block style.** Indentation-driven layout — the multi-line form most
configuration files use. Contrast with *flow style*. noyalib's
formatter defaults to block style for non-leaf collections.

**Flow style.** JSON-shaped inline layout — `[a, b, c]`,
`{k: v}`. Mostly used for short collections inside a block document.

**Plain scalar.** A scalar with no quoting — `port: 8080`. The
resolver determines its type from its lexical form (the *core
schema* in YAML 1.2; the *broad schema* in YAML 1.1).

**Folded scalar.** A `>`-introduced multi-line scalar that joins
lines with a space, preserving paragraph breaks.

**Literal scalar.** A `|`-introduced multi-line scalar that preserves
every newline byte for byte.

**Tag.** An explicit type annotation — `!!str`, `!!int`, or a custom
URI like `!shopping`. noyalib carries tags through the parse and
emits them on serialise where present.

**Document marker.** `---` (start) or `...` (end). A YAML stream is
a sequence of zero or more documents; the markers are optional for
the first document but required between subsequent ones.

**Directive.** A `%`-prefixed line at the document head — `%YAML
1.2` for version, `%TAG !x! tag:example.com,2026:` for tag prefix
shorthands. Per the spec, at most one `%YAML` directive per
document.

**Merge key.** The `<<` key. When its value is a mapping (or a
sequence of mappings), the merged keys are inherited unless
overridden. noyalib supports three policies via
`MergeKeyPolicy::{Auto, Disabled, Strict}`.

**Surrogate pair.** A two-`\uXXXX` JSON-style escape that combines
into a single supplementary-plane code point via the UTF-16
algorithm. noyalib pairs them in double-quoted scalars; lone
surrogates are rejected.

**Sexagesimal.** Base-60 integer notation — `1:30` reads as
`1*60 + 30 = 90`. YAML 1.2 dropped this from the core schema;
noyalib accepts it under `legacy_sexagesimal` (auto-on with
`YamlVersion::V1_1`).

**Norway problem.** YAML 1.1 resolves bare `no` to `false`. The
country code for Norway is `NO`. Configuration files that store
two-letter country codes were silently mis-typed for years until
YAML 1.2 narrowed the boolean lexicon to `true` / `false`.
noyalib reproduces the 1.1 behaviour under `legacy_booleans`.

## noyalib architecture terms

**`Value`.** The owned, dynamically-typed YAML value enum —
`Null` / `Bool` / `Number` / `String` / `Sequence` / `Mapping` /
`Tagged`. Heap-allocated; the natural counterpart to
`serde_json::Value`.

**`BorrowedValue<'a>`.** The zero-copy variant where string scalars
and mapping keys are `Cow<'a, str>` and borrow directly from the
input buffer. The shape is otherwise identical to `Value`.

**`Mapping`.** noyalib's string-keyed map type. Insertion-ordered
(backed by `IndexMap`). Mapping keys that the YAML resolver
produces as non-strings (numbers, booleans, sequences) are
stringified on entry.

**`MappingAny`.** The `Value`-keyed map type for documents whose
mapping keys are themselves complex YAML values (sequences,
mappings, etc.). Use this when string-keyed `Mapping` would lose
fidelity.

**Streaming deserializer.** The pull-style `from_str::<T>` path that
binds directly into `T` without materialising a `Value`
intermediate. Faster and lower-memory than the loader path; the
default for typed deserialisation.

**Loader.** The path that materialises `Value` first, then runs
`Deserializer<'_>` against it. Used for dynamic-shape inputs and
the strict-mode and span-tracking helpers.

**Green tree.** The immutable, byte-faithful CST that backs
`noyalib::cst::Document`. Stores every byte of the source —
content, whitespace, comments, indentation, line breaks. Edits
rewrite the tree structurally and re-emit byte-for-byte where
untouched.

**Span tree.** A side-table mapping every `Value` node to its
source byte range. Built lazily by the loader when the caller
requests `Spanned<T>` deserialisation or path-aware diagnostics.

**Spanned scalar.** A scalar value paired with its source byte
range (`start..end`). Used by `Spanned<T>` for editor diagnostics
and JSON-Schema `$id`-style cross-references.

**Strict deserialise.** `from_str_strict<T>` and friends — surface
input keys that the target type does not declare as a typed
`Error::UnknownField` listing every offending path. Backed by
`serde_ignored`.

**CST mutation.** Editing through `noyalib::cst::Document` rather
than re-serialising a modified `T`. Preserves comments, indent
choices, and sibling formatting; the diff is local to the edited
span.

**Tag registry.** A user-supplied `TagRegistry` that maps custom
`!tag` URIs to handlers. Lets the streaming path strip or
transform application-specific tags inline rather than routing
through the AST fallback.

## Cargo / build terms

**MSRV.** Minimum Supported Rust Version. The noyalib core library
floor is **1.75.0**; the satellite crates with edition-2024
transitive deps are **1.86.0**.

**`#![forbid(unsafe_code)]`.** A crate-root attribute that makes
any `unsafe` block a compile error. Applied to every crate in the
workspace; non-negotiable.

**Lean profile.** The `--no-default-features --features ["std"]`
build (or the `minimal` meta-feature alias) — drops `itoa`,
`ryu`, and `serde_ignored` for FIPS / embedded / audit-heavy
environments. 5 runtime deps instead of 8.

**`compat-serde-yaml`.** The drop-in shim crate feature that
exposes a name-for-name surface compatible with `serde_yaml` 0.9.
Every type it exposes is a noyalib-native type re-exported under
the `serde_yaml` name; no upstream dep.

**Doctest.** A Rust code block (` ```rust `) inside a docstring or
README that `cargo test --doc` compiles and runs. noyalib has
~384 doctests; the noyalib README is wired into doctesting via
`#[doc = include_str!("../README.md")]`.
