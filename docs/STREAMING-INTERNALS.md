<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Streaming implementation notes

Private-source design and regression notes for `crates/noyalib/src/streaming.rs`.
Short invariants remain beside the implementation. These longer explanations
live here to preserve context within the unchanged distributed-crate size budget.

## Event routing

Streaming YAML deserializer that operates directly on parser events.

Bypasses the intermediate `Value` AST for typed deserialization,
avoiding materialization for supported inputs. Anchors and aliases are
handled natively via event buffering and replay. The common
`<<: *anchor` merge-key pattern is expanded natively. Repeated canonical
mapping keys require AST fallback before the final visitor receives entries.
Direct use returns a fallback error; `crate::from_str` retries the loader.

### Where events come from

Every event reaches the deserializer through one of two sources, and
the distinction is the source of this module's sharpest edges:

* **the parser**, for text not yet read; and
* **the replay stack**, for buffered events being re-emitted, the
  contents of an anchored node standing in for an alias, or a mapping
  injected by a `<<:` merge.

Anything that reads an event must handle *both*. Alias resolution once
ran on the parser branch only, so an alias arriving through replay was
stored as though it were fully processed while still being an
`Event::Alias`, and deserialised as the literal anchor name. The replay
stack only exists after a merge injects something, which is why the bug
reproduced solely when an alias-valued entry followed a `<<:` key and
vanished when the same entry was written above it (#301).

`StreamingDeserializer::process_event` is now the single place that
resolves an alias, and `StreamingDeserializer::anchor_and_record` the
single place that does anchor bookkeeping, so a future branch cannot
quietly acquire a partial copy.

### The lookahead slot

One event of lookahead is parked in `current`, and it may hold either a
**raw** parser event or a **processed** one, see `Lookahead`. The two
are not interchangeable:

* the merge-key path fills the slot deliberately raw, because it must
  see `<<: *base` as an unresolved `Event::Alias` in order to treat it
  as a merge instruction rather than a value; while
* a processed event must never be put through processing again, because
  recording is append-only, a second pass puts the event into an
  in-flight anchor buffer twice, and every alias to that anchor then
  replays a duplicated stream.

Both consumers used to infer which kind they held, in opposite
directions. The state now travels with the event.

## Lookahead states

What state the one-event lookahead slot is in.

The slot is written by three different paths and read by two, and the
two kinds of event are not interchangeable:

* `Raw` is straight off the parser. An `Event::Alias` is still visible
  *as an alias*, which the merge-key path depends on, it needs to see
  `<<: *base` before anything resolves it.
* `Processed` has been through alias resolution, `handle_anchor` and
  `maybe_record`. Putting it through those again is not idempotent:
  a second `maybe_record` pushes the event into the in-flight anchor
  buffer twice, so every alias to that anchor replays a corrupted
  stream.

Before this existed both kinds shared one `Option<Event>` field, and
each consumer guessed. `next_event` assumed processed and returned raw
events unresolved, which is why an alias used as a merge-key override
value (`y: *other` beside `<<: *b`) came back unresolved (#301, found
by @mathstuf). `next_parser_event` assumed the opposite and would
double-record an already-processed one.

Making the state travel with the event turns both of those from a
guess into a match arm.

## Taking a parked event

Take the event out, whichever state the slot was in.

`next_event` hands a parked event back unchanged in both states,
though for different reasons: a `Processed` one has already been
resolved, anchored and recorded, and doing that again would
double-record it; a `Raw` one is parked deliberately by the
merge-key path via `peek_parser_event` and then dropped with
`skip_event`, so resolving it on the way out would expand a
`<<: *base` that is meant to be consumed whole.

Same action, different justifications, so this is one method
rather than two match arms that look like a distinction.

## Anchor recording

Anchor bookkeeping + recording, without alias resolution.

Split out because `next_parser_event` needs exactly this pair and must
*not* resolve: it is the raw accessor the merge-key path relies on to
see an `Event::Alias` as an alias. Three call sites carried their own
copy before, which is the same duplication that let alias resolution
go missing from one branch and produce #301.

Not idempotent: `maybe_record` appends, so calling it twice on one
event puts that event into the in-flight anchor buffer twice.

## End-of-input validation

Drain remaining events to surface any errors lurking past
the point at which `T::deserialize` was satisfied. Without
this, lenient inputs like `--- key1: value1\n    key2: value2`
would lazily yield `Value::String("key1")` because the
serde visitor stops at the first complete node, the
subsequent tokens that violate the spec (e.g. "mapping
values are not allowed in this context") would never be
fetched. Stop *at* `StreamEnd` (querying past it would
return a benign "parser has already finished" error);
propagate any error encountered before that.

A second `DocumentStart` here means the stream carries more
than one document, `from_str`/`from_str_with_config` only
support exactly one (`from_str_multi` is the multi-document
entry point). See #351.

## Special floating-point spellings

Parse a plain scalar as a float, rejecting the bare special forms.

Rust's `f64::from_str` accepts `nan`, `inf` and `infinity` in any
case, with an optional sign. YAML 1.2 spells those `.nan`, `.inf`
and `-.inf`, with a leading dot, and the dotted forms are matched
explicitly by the resolver before this is reached.

Accepting the bare spellings here resolved a plain scalar like
`nAn` to a float, which threw away its original text. Used as a
mapping key it came back as `nan`, so `nAn: null` did not
round-trip; the `roundtrip_value` proptest found it. Bare forms are
left as strings, which is what the spec asks for.

An explicit `!!float nan` is unaffected: tagged scalars resolve in
`parser/loader.rs`, where the tag makes the intent unambiguous.

## Scalar resolution settings

Resolve a plain scalar according to YAML 1.2's implicit-typing
rules, with these `ParserConfig` settings exposed:

- `no_schema`, when `true`, keep every plain scalar as a
  string. Useful for schema-strict pipelines where YAML's
  implicit resolution of `true` → `Bool(true)` or `null` → `Null`
  produces unwanted ambiguity.
- `legacy_octal`, when `true`, accept YAML 1.1-style bare
  `0`-prefix octal literals (e.g. `0644`) in addition to the
  YAML 1.2 `0o644` form. Off by default to honour YAML 1.2's
  stricter integer schema.
- `legacy_sexagesimal`, when `true`, accept YAML 1.1-style
  colon-separated base-60 numbers (`60:00` → 3 600,
  `1:30:00` → 5 400). Off by default; YAML 1.2 dropped the
  sexagesimal schema.
- `lossless_u64`, when `true` and the `lossless-u64` feature is
  enabled, resolve unsigned integer scalars up to `u64::MAX`
  before considering a float fallback.

## Sexagesimal integers

Parse a YAML 1.1 sexagesimal integer like `60:00` or
`-1:30:00`, colon-separated base-60 components evaluated
left-to-right. Returns `None` if the shape doesn't fit:

- At least one `:` is required (otherwise this overlaps with
  plain integer parsing).
- Each component must be one or more decimal digits.
- Components other than the first must be `00..=59` to avoid
  accepting nonsense like `1:99`.
- The leading sign (`-` or `+`) applies to the whole value.

## Lookahead tests

Unit coverage for the lookahead slot's two states.

The integration tests in `tests/streaming_alias_lookahead.rs` drive
this through real documents, which is the behaviour that matters.
These pin the type itself, because the whole point of `Lookahead`
is that a consumer can no longer confuse a raw parser event with a
processed one, and a silent change to the accessors would put that
confusion straight back.
