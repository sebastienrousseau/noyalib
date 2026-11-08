<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# 0015. Grow a scalar into a typed collection explicitly

- **Status:** accepted
- **Date:** 2026-11-08
- **Authors:** Zoo Sky, Noyalib contributors

## Context

[`set_value`](./0010-typed-collection-set-value.md) replaces collections
in their existing style but refuses to grow a scalar into a collection.
An editor changing `tags: solo` into a list must currently render and
place a YAML fragment itself. That repeats the library's indentation,
quoting, comment handling, and validation work.

[PR #505](https://github.com/sebastienrousseau/noyalib/pull/505),
contributed by [Zoo Sky (@zoosky)](https://github.com/zoosky), supplies
an explicit typed operation for this layout change.

## Decision

Add `Document::grow_value(path, value)` for a sequence or mapping value.
An existing collection delegates to the established collection
replacement operation. An equal collection is a byte-level no-op.

At a scalar site, render flow inside a flow collection and block
elsewhere, using the document's indent unit. A mapping's inline scalar
moves one indent step past its key. An own-line scalar or sequence item
keeps its content column. Preserve comments on the parent key or sequence
indicator line, or above an own-line collection. Preserve the document's
LF, CRLF, or CR convention; mixed input uses LF for newly added breaks.

Parse the complete candidate under the document's existing configuration
and compare it with the expected typed document before splicing. Refuse
alias tokens, values inside shared anchors, property-led scalars, missing
paths, and unsupported layouts without changing the source.

## Consequences

- Callers can grow typed values without implementing a fragment renderer.
- `set_value` retains its existing refusal for scalar-to-collection edits.
- The new operation adds public API and layout cases to maintain.
- Candidate validation parses the proposed document before applying it.
- Typed equality cannot detect lost comments or changed line endings;
  byte-exact regression tests must cover those independently.

## Alternatives considered

### Grow implicitly through `set_value`

That would change an existing operation's layout decisions. A separate
method makes the caller's request to grow the value explicit.

### Keep using caller-rendered fragments

That duplicates formatting and placement logic in every editor and gives
callers no typed assertion that only the intended value changed.

## References

- [Typed collection replacement](./0010-typed-collection-set-value.md)
- [Shared anchor policy](./0011-flow-inserts-and-anchor-policy.md)
- [PR #505](https://github.com/sebastienrousseau/noyalib/pull/505)
- Regression suite: `crates/noyalib/tests/cst/cst_grow_value.rs`
