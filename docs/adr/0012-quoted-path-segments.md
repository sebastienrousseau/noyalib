# 0012. Bracket-quoted key segments in the query-path grammar

- **Status:** accepted
- **Date:** 2026-09-04
- **Authors:** Noyalib contributors

## Context

One grammar, `parse_query_path` in `crates/noyalib/src/path.rs`, is
behind every path string the crate reads: `Value::get_path` and
`query`, the borrowed reads, and every path-taking `cst::Document`
method (`set`, `set_value`, `set_path`, `remove`, `rename_key`,
`span_at`, `key_span`, the comment editors, and `Entry`). It reads `.`
as a segment separator, `[n]` as a sequence index, `*` and `[*]` as
wildcards, and `..` as recursive descent, and it has no way to say
"this text is a key". A mapping key that contains one of those
characters therefore cannot be named at all.

Such keys are common: Kubernetes labels and annotations
(`app.kubernetes.io/name`), version buckets (`v0.26-dev`), URL paths
with an extension, glob patterns. At v0.0.32 the gap is not only a
missing feature but a source of silent wrong writes, because the
grammar is lenient: `remove("a[x]")` drops the bracket segment and
removes `a`; `set_path("a.")` drops the trailing separator and writes
`a`; `insert_entry_value` on an existing `*` appends a second `*`
entry, because its existing-key guard covered `.` and `[` only.

Issue #288 met this from the insert side. It fixed the insert anchor by
reading the span tree directly and explicitly deferred the rest: "that
is the wider question of whether the path grammar should grow an
escape or quoting form". Issue #388 asks that question.

## Decision

The grammar gains a bracket-quoted key segment: `["key"]` or `['key']`
is one mapping key, whatever the text between the quotes contains, and
inside either quote style `\` escapes the next character. It composes
with the rest of the grammar without a separator, the way an index
does: `items[0]["a.b"].c`. The JSONPath convention, so the spelling is
already familiar.

Because the change is in the one shared parser, every path-taking API
gains it at once, and no method grows a segment-list twin.

Three spelling helpers make the public `noyalib::path` module, so a
caller never composes a quoted segment by hand: `quote_key` spells one
key as a quoted segment, `push_key` appends a key to a path in
whichever form reads back as that key (plain with a `.` when the
grammar would read it as itself, quoted otherwise), and `join_keys`
builds a path from literal keys.

Inside the crate the same helpers replace the two places that composed
`"{path}.{key}"` by hand (`insert_entry`, `insert_entry_value`) and the
prefix re-renderer `set_path` uses when it creates a missing level. The
`.`/`[` refusal in the two inserters goes: an existing key of any
spelling is now an upsert. `rename_key`'s stricter bracket check
accepts a quoted segment and still refuses an unquoted non-index one.

The grammar stays lenient where it was: an unquoted bracket segment
that is neither an index nor `*` is still dropped, and an unterminated
quoted key runs to the end of the path.

## Consequences

- **Positive:** every key is addressable through every API, and the
  three silent wrong writes above become writes to the named key.
- **Positive:** additive. A path with no `["` or `['` after a bracket
  parses exactly as before; `push_key` keeps plain keys plain, so
  paths the crate composes for itself and shows in error messages do
  not change for ordinary keys.
- **Negative:** one more form in a grammar that is documented by
  example rather than by a spec; `Path::Display` (the diagnostics
  location type) still prints a key plain, so its output is not
  guaranteed to parse back as a query path. Making it do so is a
  separate decision.
- **Neutral:** `path` is now a public module. `Path` stays re-exported
  at the crate root; `QuerySegment` and `parse_query_path` stay
  crate-private.

## Alternatives considered

### A segment-list API beside the string one

`set_value_at(&[Segment], value)` and twins for every mutator, with a
`Segment` enum of literal key and index. Rejected: it doubles the
surface of a dozen methods and leaves `Value::get_path`, `Entry`, and
the comment editors either untouched or doubled too. A quoting form in
the shared parser reaches all of them with one change.

### Backslash escapes in dot notation

`a\.b` for a key holding a dot. Rejected: `\` is legal key text today,
so the change would not be additive, and a key holding `[`, `]`, or
`*` would need the same treatment for each character. Bracket quoting
escapes only inside the quotes.

### Refuse such keys at the mutators

Detect the misread and error instead of writing. Rejected as the only
measure: it stops the wrong writes but leaves the keys unreachable,
and callers with such keys on disk (a frontmatter editor, a Kubernetes
manifest tool) still have to fall back to a whole-document rewrite.

## References

- #288 -- the insert-side fix that deferred this question.
- #388 -- the ask this ADR answers.
- `crates/noyalib/src/path.rs` -- `parse_query_path`, `quote_key`,
  `push_key`, `join_keys`.
- `crates/noyalib/tests/cst_quoted_path_segments.rs`.
- JSONPath bracket notation (RFC 9535 §2.3.1):
  <https://www.rfc-editor.org/rfc/rfc9535#name-name-selector>
