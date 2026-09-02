# 0008. `CompiledSchema`: compile a JSON Schema once, validate many

- **Status:** accepted
- **Date:** 2026-08-30
- **Authors:** Noyalib contributors

## Context

`validate_against_schema(value, schema)` converts **both** arguments to
JSON and compiles the schema with `jsonschema::validator_for` on every
call. For a one-off check that is the right trade-off; for a content
pipeline that validates thousands of documents against a handful of
schemas (the reporter validates every page's frontmatter on every
rebuild — #329), it compiles the same schema thousands of times.

Two adjacent gaps compound it:

- `jsonschema` is not re-exported, so a caller who wants to compile
  once must depend on the crate directly and keep its version in step
  with noyalib's.
- Under Draft 2020-12 `format` is an annotation unless
  `should_validate_formats(true)` is set at build time;
  `validate_against_schema` exposes no way to set it, so
  `format: date` silently accepts `01/15/2024`.

## Decision

Under the `validate-schema` feature we add:

```rust
pub struct CompiledSchema { /* jsonschema::Validator */ }
impl CompiledSchema {
    pub fn compile(schema: &Value) -> Result<Self>;
    pub fn builder(schema: &Value) -> CompiledSchemaBuilder;
    pub fn validate(&self, value: &Value) -> Result<()>;
    pub fn iter_errors(&self, value: &Value) -> Result<Vec<SchemaViolation>>;
}
```

`CompiledSchemaBuilder` carries `validate_formats(bool)` and
`with_format(name, impl Fn(&str) -> bool)`. `SchemaViolation` is a
plain struct: instance path (RFC 6901), the schema keyword that raised
the violation, and the message.

`validate_against_schema` is reimplemented as
`CompiledSchema::compile(schema)?.validate(value)`, so the one-shot
path and the compiled path are the same code. The hardening pinned by
`tests/schema_hardening.rs` — external `$ref` refused, recursion
bounded — therefore covers the compiled path by construction, and
`tests/schema_compiled.rs` pins it again directly.

We do **not** re-export the `jsonschema` crate. The compiled type is
the API; a re-export would freeze a third-party crate's whole surface
into ours and make every `jsonschema` major bump a noyalib breaking
change.

## Consequences

- **Positive:** N validations against one schema cost one compile.
- **Positive:** format assertion and custom formats become reachable,
  opt-in, without touching the default behaviour of
  `validate_against_schema`.
- **Positive:** structured violations (`iter_errors`) replace
  string-parsing the aggregated error message.
- **Negative:** three new public types to keep stable
  (`CompiledSchema`, `CompiledSchemaBuilder`, `SchemaViolation`).
- **Neutral:** `coerce_to_schema` still compiles per call; it walks
  its own error stream and is not on the hot path this ADR addresses.

## Alternatives considered

### `pub use jsonschema;`

Smaller diff, and the issue offered it as acceptable. Rejected: it
welds noyalib's semver to `jsonschema`'s entire public API, and the
builder we expose covers the two configuration points consumers
actually asked for.

### A schema cache inside `validate_against_schema`

A keyed global cache would speed up existing callers with no API
change, but needs an eviction policy, hashing of arbitrary `Value`
trees on every call, and shared-state reasoning in a crate that
otherwise has none. An explicit handle is simpler and faster.
