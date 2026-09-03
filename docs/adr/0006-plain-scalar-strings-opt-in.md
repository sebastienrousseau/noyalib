# 0006. Opt in to literal text for a `String`/`char` target reading a plain scalar

- **Status:** proposed
- **Date:** 2026-08-30
- **Authors:** Noyalib contributors

## Context

A typed `String` field asks serde for a string. A plain YAML scalar's
source text is always one, but noyalib's implicit-typing resolver
runs ahead of the target type: `password: 123456` against
`struct Smtp { password: String }` resolves `123456` to
`Scalar::Int`/`Value::Number` first, then both deserializers
(`streaming.rs`'s `deserialize_str` and `de/deserializer.rs`'s
`deserialize_str`) refuse it as a "non-string scalar" type mismatch.
The same refusal hits `~`, `null`, `true`, and an empty value. `serde_yaml`
0.9.34 (measured) does not refuse any of these — a `String` target
receives the scalar's literal text (`"123456"`, `"~"`, `""`, …)
regardless of what an untyped `deserialize_any` would have inferred.

Implicit typing is a fallback for untyped targets, not a constraint on
an explicitly-typed field, so the refusal is arguably a bug
(issue #344). But `String` fields silently accepting any scalar shape is also
an observable behaviour change for every existing caller — a config
loader that relied on the refusal to catch a misquoted key
(`timeout: 30` where `"30"` was intended) would stop erroring. Two
call sites need the same decision: the streaming deserializer's
`deserialize_str`/`deserialize_string`/`deserialize_char`, and the
`Value`-AST `Deserializer`'s equivalents (reached whenever the
streaming fast path is bypassed, or via `from_value`).

## Decision

We will add the behaviour as an opt-in `ParserConfig` flag,
`plain_scalar_strings` (default `false`), rather than changing the
default contract.

When enabled, a plain scalar deserializes into a `String` (or `char`)
target as its source text even where the YAML 1.2 schema resolves it
to a number, boolean, or null. Off by default, both deserializers keep
refusing a non-string plain scalar for a `String` target — the
existing, tested contract. Quoted scalars are strings either way,
unaffected by the flag.

The flag threads through `parser::ParseConfig` (so the streaming
deserializer's `self.config.plain_scalar_strings` sees it directly,
without going through the AST) and through `Deserializer`'s per-call
options (alongside the existing `ignore_binary_tag_for_string`
toggle), propagated through every descent site the latter already
covers (`descend`, `ValueSeqAccess`, `ValueMapAccess`). It does **not**
join `stream_eligible`'s exclusion list in `de.rs` — unlike
`ignore_binary_tag_for_string`, which only ever mattered on the AST
path and therefore forces the AST loader, `plain_scalar_strings` is
consulted directly inside the streaming deserializer, so a document
opting into it stays on the fast path.

## Consequences

- **Positive:** callers migrating from `serde_yaml`-shaped configs
  (numeric-looking secrets, environment-style values) get an explicit,
  documented escape hatch instead of pre-quoting every field or
  patching the crate.
- **Positive:** the default contract — and every existing test in the
  crate — is untouched. `git diff v0.0.28 -- crates/noyalib/tests`
  shows only the new test file.
- **Positive:** the flag reaches the streaming fast path directly
  (via `ParseConfig`), so opting in does not cost the AST-loader
  detour the way `ignore_binary_tag_for_string` does.
- **Negative:** another boolean on an already-large `ParserConfig`;
  contributors adding a new scalar-resolution edge case must now
  check two call sites (streaming and AST) plus the propagation
  structs on the AST side.
- **Negative:** the two deserializers cannot round-trip identically
  under the flag for every input — the AST path loses a null scalar's
  original spelling (`~`/`null`/empty all format back as `""`, since
  `Value::Null` carries no text) and an integer literal's non-decimal
  spelling (`0x1F` becomes `"31"`, since `Number::Integer` only stores
  the parsed value). The streaming path is exact for both because it
  reads the source text directly. Documented on the AST helper
  (`scalar_as_text`) rather than hidden.
- **Neutral:** the flag is a scalar-shape decision only; it does not
  interact with `deserialize_any`, `Option<T>`'s null short-circuit,
  numeric/bool targets, or untagged-enum dispatch, all of which keep
  routing through `deserialize_any` unchanged.

## Alternatives considered

### Change the default contract directly

This is what issue #344 describes as the fix and is closer to
`serde_yaml`'s behaviour out of the box. Rejected for this release: it
is an observable breaking change for every caller relying on the
refusal as an implicit type check, and CI has no way to distinguish
"a config typo now silently stringifies" from "the fix worked."

### Gate the option on `stream_eligible` (force the AST path)

This would have reused the existing `ignore_binary_tag_for_string`
pattern verbatim — no new code path in the streaming deserializer.
Rejected because it taxes every document that opts in with an
unnecessary AST-loader parse, and because the two paths already
disagree on null/hex text preservation; forcing everything through the
AST path would silently make that disagreement the *only* observable
behaviour, hiding the streaming path's better fidelity.

### A separate `StrictString` newtype wrapper

Callers could wrap fields needing the loose behaviour
(`StrictString(String)`) with a custom `Deserialize` impl instead of a
global config flag. Rejected: it requires touching every affected
struct field rather than one parser-config toggle, and does not help
callers who don't control the target type (e.g. deserializing into a
third-party struct).

## References

- Issue #344.
- `docs/adr/0004-lossless-u64-integers.md` — precedent for an opt-in
  runtime flag that keeps the default model stable.
- `crates/noyalib/tests/string_target_plain_scalar.rs` — the
  acceptance/guard test suite for this flag.
