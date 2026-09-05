# 0013. Located duplicate-key and key-collision errors as sibling variants

- **Status:** accepted
- **Date:** 2026-09-05
- **Authors:** Noyalib contributors

## Context

Under `DuplicateKeyPolicy::Error` a repeated mapping key is refused
with `Error::DuplicateKey(String)`, which carries the key's text and
nothing else: `duplicate key: name`. In a long configuration with many
`name:` or `enabled:` keys the message does not say which one, while
the typed rejections have said where they are since v0.0.30
(`site.name: type mismatch: expected string, found sequence at line 2,
column 9`) and `serde_yaml` reported the same input as `site: duplicate
entry with key "name" at line 3 column 3`. `Error::KeyCollision(String)`,
raised when two distinct-typed keys collapse to one string, has the
same gap.

The information is at hand where the refusals fire. The span-aware
loader holds the key's span; the span-less loader sees the key's byte
offset when it pushes the key; both keep the frame stack that
`frames_path` already turns into a dotted path for `IntegerOverflow`.
The streaming deserializer refuses at the key event and has neither,
but a typed read that fails there without a location is re-run through
the span-aware loader (`locate_streaming_error`), so it can defer to
that loader's report. Issue #378 asked which shape the located error
should take, since either touches the public error enum.

## Decision

Two new variants, siblings of the existing ones rather than changes to
them:

```rust
Error::DuplicateKeyAt { key: String, path: String, location: Location }
Error::KeyCollisionAt { key: String, path: String, location: Location }
```

`path` is the dotted path of the entry, key included (`site.name`,
`items.0.name`); `location` is where the second key begins. They
display as `site.name: duplicate key "name" at line 3, column 3` and
`m.1: distinct mapping keys collide after string conversion: 1 (...) at
line 3, column 3`.

`kind()`, `code()`, the help text, and the i18n summary treat each pair
alike, and `location()` returns the position, so a caller that
dispatches on `ErrorKind` or reads the location sees no difference
between the two forms beyond the extra information. The parsers that
know the position -- the span-aware and span-less loaders, and through
them every `from_str` entry point and `cst::parse_document` -- raise the
located forms. `locate_streaming_error` keeps a located duplicate-key
or key-collision refusal from the AST retry whole instead of folding
the streaming twin's message into a `DeserializeWithLocation`, which
would have changed the kind.

The location-less variants stay: they are constructible by callers,
documented, and the streaming walker still raises them on the paths
that have no AST retry.

This is the precedent `UnknownAnchor` / `UnknownAnchorAt` set.

## Consequences

- **Positive:** an operator reading the error knows which key and
  where; the message has the shape the typed errors already use.
- **Positive:** additive. `Error` is `#[non_exhaustive]`, so a match
  outside the crate already has a wildcard arm, and every accessor
  answers the same for the new forms.
- **Negative:** a caller that matched `Error::DuplicateKey(_)` to detect
  the refusal and did not also check `kind()` now misses the located
  form; `kind()` was the documented way and stays it.
- **Negative:** two more variants on a large enum, and two rules to keep
  in sync (`kind`, `code`, help, i18n) -- the sibling pattern's known
  cost.
- **Neutral:** the span-less loader now keeps one byte offset per open
  mapping key.

## Alternatives considered

### Add the fields to `DuplicateKey` itself

`DuplicateKey { key, path, location }` would be one variant instead of
two, but changes a variant that callers construct and match today: a
breaking change for a release line at 0.0.x that has not broken the
error enum before. Rejected for now; folding the pairs into one variant
is the natural move at the next deliberate break, and this ADR is the
record for it.

### Wrap the refusal in `ParseWithLocation`

Reuses an existing located variant with no new surface, but the error
would then report `ErrorKind::Syntax`, and a caller dispatching on
`ErrorKind::DuplicateKey` -- the documented way to detect the policy
refusal -- would stop seeing it. Rejected.

### Give the streaming walker its own location and path

Possible (the key event carries a span and the walker holds the input),
but the walker tracks no path, and the typed read already retries
through the span-aware loader on a location-less error, which reports
both. Rejected as duplicate work; the walker's location-less refusal
is superseded by the retry's.

## References

- #378 -- the ask.
- #353 -- the typed rejections' path prefix, whose shape this follows.
- `crates/noyalib/src/parser/loader.rs` -- `duplicate_key_at`,
  `key_collision_at`, `Loader::frames_path`, `NoSpanLoader::frames_path`.
- `crates/noyalib/src/de.rs` -- `locate_streaming_error`.
- `crates/noyalib/tests/duplicate_key_location.rs`.
