# 0007. Add `prefer_single_quotes` as an opt-in serializer config flag

- **Status:** accepted
- **Date:** 2026-08-30
- **Authors:** Noyalib contributors

## Context

`write_string` quotes a scalar only when its content forces it (a
reserved word, a leading `-`, an embedded `:`, and so on — see
`NEEDS_QUOTE_BYTE` / `FIRST_CHAR_QUOTE`), and once quoting is needed it
always double-quotes. Several downstream users want single-quoted
output for anything a single-quoted scalar can represent — it is
easier to read when the content itself contains backslashes or double
quotes, and it matches the house style a number of hand-written YAML
files already use. There is currently no way to get that without a
manual post-process pass over the emitted string.

`SerializerConfig` already carries several opt-in, purely additive
style flags (`quote_all`, `compact_list_indent`); a quote-style
preference is the same shape of decision.

## Decision

We add `SerializerConfig::prefer_single_quotes(bool)`, default `false`.
Output is byte-for-byte unchanged unless a caller sets it.

When set, and `write_string` has already decided a scalar needs
quoting, it single-quotes the scalar (doubling any embedded `'`)
instead of double-quoting it — *unless* the scalar contains a
character single-quoted style cannot carry at all: a control
character (tab, CR, LF, the rest of the C0/C1 ranges) or any other
non-printable code point. Single-quoted style has no escape mechanism
beyond doubling the quote character itself, so such a scalar still
double-quotes regardless of the setting. The empty string quotes as
`''` rather than `""` when the option is set.

The flag does not change *whether* a scalar gets quoted, only *which*
quote style is used once quoting is already required — a scalar that
stays plain today (e.g. `it's`) stays plain either way.

## Consequences

- **Positive:** callers who want single-quoted output for readability
  or house-style reasons get it without a post-process string pass.
- **Positive:** purely additive — the field defaults to `false` and no
  existing caller's output changes.
- **Negative:** one more flag on an already-large `SerializerConfig`;
  contributors touching `write_string`'s quoting decision must keep
  the single-quote-safety check (`single_quote_safe`) in sync with
  `write_double_quoted`'s escape set.
- **Neutral:** `quote_all` is unaffected — it already always
  single-quotes regardless of content, which is a separate,
  pre-existing behavior this ADR does not change.

## Alternatives considered

### A `ScalarStyle::SingleQuoted` variant instead of a new flag

`SerializerConfig::scalar_style` already has a `SingleQuoted` variant,
but it is not consulted anywhere in `write_string` today (a
pre-existing gap, not something this change should paper over by
quietly wiring in a different, narrower meaning). Piggy-backing the
"prefer single quotes when quoting is needed" behavior onto that
variant would conflate two different decisions ("force single-quoted
style unconditionally" vs. "prefer it only when quoting was already
necessary") under one flag. A dedicated boolean keeps the new
behavior's scope explicit and reversible without touching the
unrelated `scalar_style` gap.

### Always prefer single quotes when safe (no opt-in)

Simpler, but it is a silent output-format change for every existing
caller of `to_string`/`to_string_with_config` with a default config.
Rejected because "default output does not change" is the norm every
other `SerializerConfig` flag added so far has followed.

## References

- `crates/noyalib/src/ser.rs` — `write_string`, `single_quote_safe`,
  `write_single_quoted`, `write_double_quoted`.
- YAML 1.2.2 §7.3.1 (Single-Quoted Style) / §7.3.2 (Double-Quoted
  Style): <https://yaml.org/spec/1.2.2/#731-single-quoted-style>
