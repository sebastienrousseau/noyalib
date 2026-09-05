# 0009. `Document::set_path`: parent-creating writes in the CST editor

- **Status:** accepted
- **Date:** 2026-08-30
- **Authors:** Noyalib contributors

## Context

A comment-preserving frontmatter editor needs to set a nested key
whose parents do not exist yet — the writers it replaces create them
(#327). Against v0.0.28:

- `set("menu.visible", …)` on `title: x` errors `path not found`;
- `insert_entry("menu", "visible", …)` errors `path not found: menu`
  and additionally requires the parent mapping to be non-empty;
- on `parse_document("")` both error, so an empty document can never
  receive its first key through the typed mutators.

Callers can splice a verbatim fragment with `set` at the deepest
existing ancestor, but then every consumer re-derives indentation,
style, and quoting that `Emit` already owns — and the empty-document
case has no ancestor to splice at.

## Decision

We add `Document::set_path(&mut self, path: &str, value: &Value)`:

- **whole path exists** — delegates to `set_value` (upsert,
  equal-value no-op, scalar-only replacement);
- **a block-mapping ancestor exists** — wraps `value` in one mapping
  per missing level and inserts the chain through
  `insert_entry_value`, which owns indentation (`indent_unit()`),
  quoting (`Emit`), and the typed-oracle rollback;
- **the document is empty** (comments, blank lines, or a bare `---`
  only) — renders the chain with the serializer at the document's
  indent unit and appends it after the existing bytes, so a comment
  header survives its document's first key. The candidate is parsed
  and compared against the expected value *before* the splice; an
  explicit `null` root fails that check and is refused byte-identical.

Refusals, all leaving the source byte-identical:

- an existing segment resolving to a scalar (`title.x` where `title`
  is a string) or a non-root null (filling an implicit null with a
  mapping stays a `set` fragment edit for now);
- a missing segment that is a sequence index — `set_path` creates
  mappings, never sequence items;
- flow or empty-`{}` ancestors — the flow inserters are #338's scope;
  `insert_entry_value`'s existing guards produce the clean error.

The composition is deliberate: `set_path` adds **no new splice
logic**. Every byte it writes goes through a mutator that already
carries the re-parse guard and the typed-value oracle, so the
correctness surface does not grow with the feature.

## Consequences

- **Positive:** the missing write shape lands (Renovate-style
  "ensure this nested key") without callers re-implementing style.
- **Positive:** the empty-document dead end is gone.
- **Negative:** one more path-addressed mutator whose error catalogue
  must stay in step with `set_value` / `insert_entry_value`.
- **Neutral:** flow ancestors and implicit-null parents refuse today;
  when #338 lands, `set_path` inherits flow support through
  `insert_entry_value` with no signature change.

## Alternatives considered

### `insert_entry_value(.., create_parents: true)`

A flag instead of a function. Works, but the common call site reads
worse (`set_path("menu.visible", v)` states the intent), and the
empty-document arm does not fit `insert_entry_value`'s contract of
addressing an *existing* mapping.

### Callers splice fragments at the deepest existing ancestor

The status quo. Rejected for the reasons in Context: every consumer
re-derives what `Emit` owns, and `parse_document("")` has no splice
site at all.
