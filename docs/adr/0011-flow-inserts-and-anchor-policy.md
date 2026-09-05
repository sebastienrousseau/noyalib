# 0011. Flow-collection inserts, flow renames, and one anchored-node policy

- **Status:** accepted
- **Date:** 2026-08-31
- **Authors:** Noyalib contributors

## Context

Frontmatter corpora write lists as `tags: [a, b]` and small mappings
as `menu: {visible: false}` — in the reporting consumer's corpus,
36,113 sequence sites are flow-style — yet every insertion mutator
refused flow targets, empty collections had "no entry to anchor
indentation on", a root flow mapping was "stray content", and
`rename_key` called flow renames "a follow-up" (#338).

Separately, the mutators disagreed about writes inside anchored
values: `rename_key` and the inserters refused (naming the anchor and
pointing at `materialise_aliases_of`), while `set_value` and `remove`
silently changed every `*name` and `<<` merge site along with the
addressed path.

## Decision

**Flow splices.** Single-line flow collections accept inserts:

- `insert_entry_value` on a flow mapping — `{a: 1}`, `{}`, or a whole
  document spelled as one — splices `, key: value` (no comma into an
  empty body) before the closing `}`;
- `push_back_value` on a flow sequence splices `, value` before `]`
  (`[]` receives its first member);
- `insert_after_value` splices `, value` after the addressed item's
  span;
- `rename_key` renames flow-mapping keys; a new key whose plain
  spelling would read as flow structure (`,` `[` `]` `{` `}`) is
  double-quoted.

Members are rendered flow-safe: strings through the flow-context
speller (so `b, c` and multi-line strings double-quote), collections
through the serializer's flow style. A flow collection spanning more
than one line refuses — its separators cannot be located from the
span alone, the stance `remove` already takes. Every flow splice runs
under the same snapshot → re-parse → typed-oracle guard as the block
paths.

**One anchored-node policy: refuse, with the escape hatch.** A write
into a value that live `*name` sites share lands at every one of
them. All mutators now refuse it the same way — naming the anchor and
pointing at `materialise_aliases_of` — `set_value` and `remove`
included. An **equal-value** `set_value` stays a byte no-op wherever
it points: a no-op is harmless.

Refusing was chosen over "accept, alias sites follow" because it is
the option that loses no information: a caller who *wants* the
propagation can materialise or edit the anchor deliberately, while a
caller who didn't know about the anchor is protected. It also matches
what most of the surface already did — only `set_value` and `remove`
changed behaviour.

## Consequences

- **Positive:** the dominant real-world layouts stop being dead ends
  for typed edits.
- **Positive:** the mutators give one answer about anchors instead of
  three.
- **Breaking (behavioural):** `set_value` / `remove` inside an
  anchored value now refuse where they used to silently edit every
  alias site. Recorded in the CHANGELOG; callers get the
  `materialise_aliases_of` guidance in the error.
- **Negative:** multi-line flow collections still refuse inserts; the
  member-per-line layout needs line-ownership rules (#294) to splice
  well.

## Alternatives considered

### Accept everywhere, alias sites follow

YAML's own semantics, and one less refusal — but a silent multi-site
edit from a single-path call is exactly the surprise the integrity
oracles exist to prevent, and `rename_key` + the inserters had
already picked refusal. Consistency with the safer half won.

### Docs-only (leave the disagreement, document it)

The issue floated it as a fallback. Rejected: a policy that differs
by method is not a policy.
