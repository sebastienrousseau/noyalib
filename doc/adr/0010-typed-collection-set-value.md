# 0010. `set_value` accepts collections, emitted in the target node's style

- **Status:** accepted
- **Date:** 2026-08-31
- **Authors:** Noyalib contributors

## Context

`set_value` was documented scalar-only, so replacing a
collection-valued key (`tags`, `menu`, `media`) went through
`set(path, fragment)` with verbatim text — and the caller had to
detect the target's style first, because `noyalib::to_string` renders
block style and splicing `- a\n- c` over `tags: [a, b]` fails the
re-parse guard (`block sequence entries are not allowed in this
context`, #328). Every consumer re-derived what the emitter owns.

`ROADMAP-TO-10.md` A4 deliberately keeps `set` verbatim; this is the
**typed** path, not a change to `set`.

## Decision

`set_value` accepts `Value::Sequence` / `Value::Mapping` when the
**target node is itself a collection**, rendering in the target's own
style:

- a flow node (`[…]` / `{…}`, or any node inside a flow collection)
  is replaced by the serializer's single-line flow rendering;
- a block node is replaced by a block rendering at the document's
  `indent_unit()`, with every continuation line shifted to the old
  value's own column.

The span comes from the loader's span tree (the green resolver
mis-spans an indentless `tags:\n- a` sequence), trimmed of the
leading indent it sweeps up. Before any byte moves, the candidate
document is parsed and compared against the expected typed value —
the document with exactly this path replaced — so a rendering the
site cannot hold refuses byte-identically.

Replacing a **scalar** with a collection stays refused: growing a
value onto its own lines is a layout decision `set` expresses with a
fragment. Equal-value writes stay byte no-ops (#337's rule, already
in place, applies unchanged).

## Consequences

- **Positive:** `doc.set_value("tags", &value)` now covers the
  collection-valued frontmatter keys downstream editors replace,
  with no style detection in the caller.
- **Positive:** the oracle-first ordering means the failure mode is a
  clean refusal, never a corrupted document.
- **Negative:** one more rendering path (`replace_collection_value`)
  whose style decisions must stay in step with the serializer's.
- **Neutral:** comments *inside* the replaced collection go with it —
  they document bytes that are being replaced; bytes outside the span
  are untouched and pinned by test.

## Alternatives considered

### A separate `set_collection_value`

The issue offered it. One method with a documented collection arm
reads better at the call site; the scalar/collection split is a fact
about the argument, not two different intents.

### Callers render the fragment themselves

The status quo; rejected for the duplication described in Context.
