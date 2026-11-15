<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# 0020. Enforce complete tag and anchor name bounds

- **Status:** proposed; implementation on the future branch for review
- **Date:** 2026-11-12
- **Authors:** Noyalib contributors

## Context

Hosted roundtrip fuzzing retained the 1,056-byte input
`crash-3eee8e14d8486eecc2c0280af136869ea275d727`. The scanner accepted
a secondary tag whose stored URI was 1,027 bytes. Serialization selected
verbatim spelling, whose reparse exceeded the existing 1,024-byte limit.
The scanning loops measured suffixes before advancing and could also
accept a final byte beyond the bound. Declared prefixes were uncounted.
The sibling anchor scanner accepted 1,025 bytes only at EOF.

These are implementation resource bounds, not YAML grammar restrictions.
Their meaning must remain consistent across source spellings and the
shared scanner's Value, typed and CST consumers.

## Proposed decision

Enforce the existing tag limit against the complete resolved URI before
cloning a declared prefix. Keep early scanning bounds and check the
combined byte length with saturating addition. Check the anchor name
length after scanning, including EOF. Alias names share the same helper.

Resolve directives only for shorthand tags. A verbatim URI retains its
written identity even when `%TAG !` redefines the primary handle; an
unused directive prefix consumes none of that verbatim URI's budget.
The non-specific `!` tag also ignores primary-handle redefinitions.
Reject empty expanded URIs and active prefixes containing an unspellable
`>` character. Validate control bytes across the whole directive line,
including fields consumed before trailing trivia is processed.

Keep local verbatim URIs outside the legacy scalar shorthand aliases.
Serialization must retain their identity when a local URI resembles a
built-in name, such as `!str` or `!!str`.

Keep the current tag character grammar, diagnostics, public configuration,
MSRV and feature declarations. Record the smaller acceptance set as a
breaking parse change in v0.0.61. Include exact-boundary ASCII and UTF-8
tests, directive expansion, all three consumers and both runtime profiles.
Retain the original fuzz input in std and alloc-only regression corpora.

## Consequences

- Accepted tags within the bound retain their value when serialized.
- Inputs relying on oversized resolved tags or EOF anchor names now fail.
- The bound applies to bytes, so a Unicode name reaches it in fewer
  characters. A declared prefix consumes part of the URI budget.

## Alternatives considered

Raising the bound would preserve the inconsistent accounting and move
the failure to a larger input. Checking only the written suffix would
leave declared-prefix expansion and serialization spelling inconsistent.
Rejecting additional URI characters would change a separate grammar
contract without addressing the resource-bound defect.

## References

- [Boundary tests](../../crates/noyalib/tests/limits/dos_hardening.rs)
- [Std regression input](../../fuzz/regressions/fuzz_roundtrip/crash-3eee8e14d8486eecc2c0280af136869ea275d727)
- [Alloc-only regression input](../../fuzz-nostd/regressions/fuzz_roundtrip_alloc_only/crash-3eee8e14d8486eecc2c0280af136869ea275d727)
- [Verbatim tag regressions](../../crates/noyalib/tests/regressions/verbatim_tags.rs)
- [YAML 1.2.2 node tags](https://yaml.org/spec/1.2.2/#691-node-tags)
