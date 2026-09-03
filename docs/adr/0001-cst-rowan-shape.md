# 0001. CST shape: parallel green tree, not unified AST

- **Status:** accepted
- **Date:** 2026-04-30
- **Authors:** Sebastien Rousseau

## Context

noyalib has two competing demands on its parse output:

1. **Data binding for application code.** `from_str::<MyConfig>(s)`
   should be fast, allocate little, and surface a strongly-typed
   value. The user does not care about whitespace, comments, or
   indentation choices — they want their struct.
2. **Lossless tooling.** `noyafmt`, `noyavalidate --fix`, the LSP
   server, and the MCP server all need to mutate one logical value
   in a YAML file *without* disturbing comments, indent style, blank
   lines, or sibling formatting. A round-trip through `from_str` /
   `to_string` will erase all of that — by design, since the YAML
   data model excludes formatting.

A naïve unified CST (one tree carrying both data and trivia) would
tax every consumer in case 1 with the cost of case 2 — extra
allocation, extra walk depth, extra match arms. That's the design
mistake `serde_yaml` 0.9 avoided by simply not having a CST; it's
also what made retrofitting one impossible.

`rust-analyzer`'s `rowan` and `taplo`'s TOML implementation both
solved this by splitting the parse output: a *green tree* (immutable,
byte-faithful, structurally shared) with a thin *red tree* layer for
node identity. Application code that only wants typed data never
touches the green tree; tooling code does.

## Decision

noyalib parses to **two parallel outputs**:

- **`Value` / `BorrowedValue` / `T` via `Deserializer`** — the data
  binding path. Fast, lean, allocates only what the data demands.
  Used by 95%+ of consumers. No formatting, no comments, no spans
  unless explicitly opted in via `Spanned<T>`.
- **`noyalib::cst::Document`** — the lossless CST path. Backed by a
  rowan-shaped green tree that retains every byte of the source.
  Mutation rewrites the tree structurally and re-emits byte-for-byte
  where untouched, minimally where edited.

Both paths share the **same scanner token stream** — there is no
second parser. The CST is constructed from the token stream lazily
when `cst::parse_document` is called; ordinary `from_str` users
never pay for it.

## Consequences

**Positive:**

- Application code stays the size it is — `from_str::<T>` benchmarks
  unchanged from the no-CST baseline.
- Tooling code gets a real CST instead of a YAML-as-strings parser
  fork; comments, indent style, and blank lines are all preserved
  byte-for-byte.
- One source of truth for *parsing*. The scanner runs once; the
  green tree and the value tree both consume its output.
- The CST is a stable foundation for ongoing scanner strictness work
  — fixes to indentation rigor land once, both consumers benefit.

**Negative:**

- Two trees instead of one means two construction paths to maintain.
  Drift risk: a scanner change that affects token boundaries must
  be reflected in *both* the loader (Value path) and the green-tree
  builder (CST path). Mitigated by a per-PR
  `cst_round_trip` integration test that asserts byte equality on
  every spec-suite case.
- The CST API is more complex than a pure data-binding surface. Users
  who try to use `cst::Document` for read-only access get a more
  verbose surface than `Value`; the docs steer them away from this.
- Memory: the green tree is allocated structurally per source byte.
  For 1 MB inputs that's roughly 2× the input size. The mutation API
  uses structural sharing so edits don't double again, but raw
  parse-and-hold cost is real.

**Neutral:**

- The CST module sits behind the `std` feature (uses thread-local
  storage for span attachment). `no_std` users get the data path
  only — which is what they wanted anyway.

## Alternatives considered

### Unified CST

One tree carries both typed data and trivia. Application code
walks the tree to reach values; tooling code walks the same tree
to reach formatting. Rejected because it taxes every `from_str`
caller with the trivia overhead, and because most YAML use is
data-binding — the heavy users would be paying for the tooling
users' feature.

### String-rewriting tooling without a CST

Tooling could re-implement a partial parser that finds the byte
range of a value, swaps the bytes in place, and skips full
parsing. This is what `yq` does. Rejected because it duplicates
the parsing logic across two implementations, drifts in subtle
ways, and can't handle structural edits (insert / delete a
sibling, change a value's quoting style) without reproducing the
full scanner anyway. We're already the parser — we should be the
editor too.

### Span-augmented Value (no separate tree)

Add `(start, end)` byte ranges to every `Value` node and call that
a "CST". This is what `serde_path_to_error` does for error
reporting. Rejected because spans alone don't preserve trivia —
no comments, no blank lines, no indent choices — so it doesn't
solve the lossless-tooling case. We do offer this in a lighter
form via `Spanned<T>` for diagnostic use cases that don't need
full fidelity.

## References

- Detailed design note: [`docs/design/green-tree.md`](../design/green-tree.md)
  (~480 lines, written 2026-04-30)
- `rowan` source-of-truth implementation:
  https://github.com/rust-analyzer/rowan
- `taplo` TOML CST: https://github.com/tamasfe/taplo
- `noyalib::cst::Document` API: [crates/noyalib/src/cst/document.rs](../../crates/noyalib/src/cst/document.rs)
- Round-trip lock test: `crates/noyalib/tests/cst_round_trip.rs`
