# 0004. Opt in to lossless `u64` integers

- **Status:** proposed
- **Date:** 2026-06-22
- **Authors:** Noyalib contributors

## Context

YAML 1.2 integer scalars are not limited to Rust's `i64` range.
noyalib's public `Number` model is currently:

```rust
pub enum Number {
    Integer(i64),
    Float(f64),
}
```

That shape leaves no lossless representation for unsigned integer
scalars in `(i64::MAX, u64::MAX]`. The gap appears in several
paths:

- `Serializer::serialize_u64` rejects `u64` values above
  `i64::MAX`.
- `Number::from(u64)` stores large values as `Float(v as f64)`,
  losing precision above `2^53`.
- Plain scalar resolution tries `i64` first and then falls through
  to `f64`, so `18446744073709551615` becomes a float-shaped value
  instead of a YAML integer.
- `Value`'s serde bridge casts `visit_u64` through `i64`, which
  wraps large unsigned values.

The repository already has a safe, tested `parse_decimal_u64`
implementation in `crates/noyalib/src/simd.rs`, so the parser does
not need new dependencies or `unsafe` code. The hard part is the
public API: `Number` is exported directly and through compatibility
modules, and migration docs currently describe the model as the
serde-yaml-like `Integer` / `Float` split.

## Decision

We will add lossless unsigned integer support as an opt-in surface.

When the `lossless-u64` Cargo feature is enabled, `Number` gains:

```rust
#[cfg(feature = "lossless-u64")]
Unsigned(u64)
```

The existing `Integer(i64)` variant stays in place. We will not
rename it to `Signed`, because keeping the variant name minimizes
source churn for callers that already construct or match signed
integers.

The feature is dependency-free and is not enabled by default. Runtime
behavior is controlled by explicit configuration so default parser and
compatibility behavior remain stable. The compatibility shim for
`serde_yaml` keeps the legacy `Integer` / `Float` contract and does
not opt into `Unsigned` values.

When both compile-time and runtime opt-ins are active:

- `u64` values in `(i64::MAX, u64::MAX]` serialize as plain YAML
  integer scalars.
- Plain and explicitly tagged `!!int` scalars in that range parse as
  unsigned integers rather than floats or strings.
- Typed `u64` deserialization round-trips those values losslessly.
- Values above `u64::MAX` remain outside the integer model and must
  not be rounded into `u64`.

## Consequences

- **Positive:** identifiers, counters, and other legitimate unsigned
  YAML integers can round-trip without application-level string
  workarounds.
- **Positive:** the implementation can reuse existing safe integer
  parsing primitives and preserves the workspace-wide zero-unsafe
  invariant.
- **Positive:** the default API and `compat-serde-yaml` migration
  story remain stable for callers that do not opt in.
- **Negative:** `--all-features` builds expose a third `Number`
  variant. Because `Number` is not `#[non_exhaustive]`, exhaustive
  downstream matches that enable this feature must add an `Unsigned`
  arm.
- **Negative:** parser, loader, streaming deserializer, borrowed AST,
  serde bridge, schema helpers, and tests all need coordinated updates
  to avoid reintroducing a lossy fallback.
- **Neutral:** migration and architecture documentation must describe
  the default `Integer` / `Float` model separately from the opt-in
  `Unsigned` model.

## Alternatives considered

### Always add `Unsigned(u64)`

This would make all valid `u64` scalars lossless by default.
Rejected for this release because it changes parser output and the
public `Number` enum shape for every user, contradicting the current
migration docs and SemVer policy.

### Rename `Integer(i64)` to `Signed(i64)`

This is more explicit, but it breaks every caller that constructs or
matches `Number::Integer`. Rejected in favour of adding
`Unsigned(u64)` while keeping the established signed variant name.

### Store large unsigned integers as strings

This avoids a public enum change, but changes the YAML type from
`!!int` to `!!str` and forces callers to recover numeric intent out of
band. Rejected because the goal is lossless integer serialization, not
lossless text preservation.

### Continue using `Float(f64)` for overflow

This preserves the current API, but it is lossy above `2^53` and makes
valid YAML integer scalars indistinguishable from floating-point data.
Rejected because it is the bug this ADR addresses.

## References

- YAML 1.2.2 core schema: <https://yaml.org/spec/1.2.2/#102-tags>
- `doc/POLICIES.md` — SemVer, feature, and zero-unsafe policies.
- `doc/adr/0002-yaml-1.2-default.md` — precedent for explicit runtime
  opt-in when compatibility and correctness trade off.
- `doc/adr/0003-zero-unsafe-policy.md` — safe numeric parsing
  requirement.
