<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# 0016. Preserve plain scalar spelling on typed AST fallback

- **Status:** accepted
- **Date:** 2026-11-08
- **Authors:** Noyalib contributors
- **Supersedes:** [0006](./0006-plain-scalar-strings-opt-in.md)

## Context

`plain_scalar_strings` promises a scalar's source text for a typed
`String` or `char`. The streaming deserializer honors that promise,
but the AST deserializer formatted the resolved value. Adding an
unrelated tag could therefore change `3.10` into `3.1`, `0x1F` into
`31`, or a written null into an empty string. [Issue #494](https://github.com/sebastienrousseau/noyalib/issues/494)
reports this disagreement.

ADR-0006 recorded this loss as a limitation. A source span is already
available on the standard AST path, so the limitation is avoidable.
Alloc-only typed fallback needs to retain the same source context
when this option is enabled.

## Decision

Keep `plain_scalar_strings` opt-in and leave the resolved `Value` tree
typed. For string or character coercion of a number, boolean, or null,
prefer the scanner's plain scalar text at that value's source span.
The scanner handles property prefixes and folded plain scalars;
quoted and block strings keep their existing decoded values.

Traverse tagged wrappers when building the value-to-span map. Retain
inherited entry origins during typed merges, following the same merge
precedence and explicit-override rules as the resolved mapping. CST
parsing keeps only owned source entries, so this read metadata cannot
make inherited fields writable.

Use the existing span loader for alloc-only typed AST fallback when
`plain_scalar_strings` is enabled. It uses allocation without thread-local
storage. Ordinary alloc-only loads and the `Value` fast path continue
to use the span-free loader.

Source-free values retain canonical formatting. A substituted include
value cannot use the include filename as scalar text. Existing policies,
post-processing, alias budgets, and typed numeric deserialization remain
on their established paths.

When this option is enabled, alias costs also include the source bytes
that string coercion can restore from number, boolean, and null nodes.
Measure those bytes before expansion for scalar and collection anchors;
otherwise a long numeric spelling could be copied repeatedly while the
budget sees only the small resolved number.

## Consequences

- Streaming and AST coercion agree on the reported source spellings.
- `char` checks the source text's length rather than the formatted value.
- The enabled option changes parse behavior and requires a breaking fix
  classification under the repository's stability policy.
- Alloc-only targets compile the span loader. Typed fallback retains
  spans when this option needs them; ordinary alloc-only and direct
  `Value` loading keep their span-free route.
- A source-aware coercion scans its scalar fragment to recover its text;
  the default configuration does not perform this extra work.
- Shared regression cases run with and without the `std` feature in CI.

## Alternatives considered

### Canonical formatting on every AST fallback

That preserves the old implementation but violates the option's source
text promise and makes an unrelated tag affect another field's value.

### Turn every plain scalar into `Value::String`

That would make numeric targets and untyped values lose schema typing.
The option changes explicitly typed string targets only.

## References

- [Issue #494](https://github.com/sebastienrousseau/noyalib/issues/494)
- [Original opt-in decision](./0006-plain-scalar-strings-opt-in.md)
- Regression suite: `crates/noyalib/tests/regressions/issue_494.rs`
- Alloc-only target: `crates/noyalib/tests/issue_494.rs`
