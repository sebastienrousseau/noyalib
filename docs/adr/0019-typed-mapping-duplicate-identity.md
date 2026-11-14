<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# 0019. Settle typed mapping duplicate-key identity

- **Status:** accepted; local implementation verified; sustained acceptance pending
- **Date:** 2026-11-11
- **Authors:** Noyalib contributors

## Context

Fuzz input `crash-337857b0c28cc63b127643f0ad7afc11c7d6391c`
exposes an existing disagreement at signed core `6d2f324`. The original
88 bytes remain in the failed campaign evidence. The reduced input is
`{~: 1, ? }`; adding a tagged key forces typed AST fallback.

For a `BTreeMap<String, Value>`, the observed results are:

| Input | Typed keys | Untyped `Value` keys |
|---|---|---|
| `{~: 1, ? }` | `~` and the empty string | `null`, with the last value |
| `{!t x: 3, ~: 1, ? }` | `x` and the empty string | `x` and `null`, with the last value |

Likewise, `null: 1` followed by `~: 2` retains both written string keys
on streaming, but adding `z: !t 3` makes fallback retain only `~`.
The span loader deduplicates resolved null keys before typed key-text
restoration. Recovering a surviving key's spelling cannot restore an entry
that the resolved mapping has already replaced.

The archived JSON-target parsers preserve the written string keys. YAML
null spellings represent the same scalar key, while the configured
`DuplicateKeyPolicy` defaults to `Last`. These are different comparison
contracts. The fuzz run remains failed; this finding is not excluded or
normalized away.

## Decision

Apply the existing canonical duplicate policy consistently before typed
coercion. Preserve the written spelling of the surviving typed key;
keep untyped `Value` canonicalization and distinct-typed collision checks.
The maintainer accepted this decision on 2026-11-13. Under `Last`, retain
the first insertion position, last value and last scalar spelling. Under
`First`, retain the first position, value and spelling. `Error` refuses a
repeated canonical identity. Alias keys keep the loader's existing resolved
representation; an alias reference is not a new scalar spelling.

This affects equivalent null, boolean and numeric spellings, and visitors
that previously received repeated entries. It is not limited to `null` and
`~` in string maps. Distinct typed identities that stringify alike remain
collisions under every policy.

Streaming cannot retract an entry already delivered to a Serde visitor.
An implementation therefore needs to detect repeated canonical key
identities and select AST fallback, with measured memory and throughput
costs. It must handle aliases, resolved numeric and boolean spellings,
merge precedence, all duplicate policies and existing key budgets.
Apply the identity checks to skipped mappings too. Registered strip-through
tags use their resolved scalar identity; other tagged keys take the loader
path. The compatibility facade retains its repeated-struct-field refusal through
a private post-validation probe whose values are discarded.
Direct streaming and borrowing APIs retain their documented subset
and report their existing fallback error when materialization is required.

## Alternative

Preserve distinct written `String` keys consistently, matching archived
Serde YAML behavior. This requires a typed fallback representation that
retains entries before canonical deduplication and an explicit rule for
`First`, `Last`, `Error` and distinct-typed collisions. Turning every key
into a raw string would violate the established collision invariant.

## Acceptance

The maintainer's choice is recorded above.
Keep Rust 1.89, feature declarations and public signatures unchanged.
Require failing-first cases across streaming and fallback, std and alloc,
all duplicate policies, aliases and tagged keys. Re-run complete batteries,
the actual consumer corpus and sustained differential fuzzing. Document
any resulting parse behavior change as breaking under the patch policy.

The original 88-byte finding is retained as
[tracked evidence](../evidence/pending-typed-duplicate-identity.bin).
The full-family campaign includes it in the comparison corpus. The accepted
contract has observed failing-first policy, skipped-content and tag regressions.
The development oracle may adapt only repeated scalar-key spellings, after
proving the entire candidate JSON projection unchanged and comparing a fresh
archive parse. Unique keys, quoted identities and separate mappings remain
untouched. No input is removed from the campaign. Full-duration and independent
review acceptance remain required.

The [implementation receipt](../ecosystem-duplicate-identity-2026-11-13.json)
retains red/green evidence, full feature batteries, sanitizer inputs and local
throughput/RSS observations. Repeated-key workloads pay for loader fallback;
ignored maps also pay for canonical identity validation. Controlled performance
acceptance and independent review remain open.
