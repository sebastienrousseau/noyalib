# Green Tree — Side-Table CST for Round-Tripping Edits

**Status:** design proposal · **Author:** Sebastien Rousseau (with Claude
assistance) · **Last updated:** 2026-04-30

## TL;DR

Add a second, parallel parse output: an immutable byte-faithful **green
tree** that retains every byte of the source — content, whitespace,
comments, line breaks, indentation. Keep `Value` and the
`StreamingDeserializer` exactly as they are. Expose the green tree
through a new `Document` type whose mutation API rewrites the tree
structurally, then re-emits the modified tree byte-for-byte where
untouched and minimally where edited.

This is the architecture popularised by `rowan` (rust-analyzer) and
`taplo` (TOML). It is the pragmatic alternative to a unified CST,
which would tax every consumer who only wants `from_str::<MyType>(…)`.

## Goals

1. **100% lossless round-trip when nothing is edited.** `parse → emit`
   is byte-identical for any well-formed input.
2. **Local minimal diffs on edit.** Mutating one scalar rewrites only
   the bytes for that scalar; surrounding indentation, comments,
   blank lines, and other-key content are preserved verbatim.
3. **Zero performance impact on the existing fast path.** Users who
   call `from_str::<T>` or `StreamingDeserializer` see no change in
   compile-time size, runtime allocation, or throughput.
4. **One source of truth for parsing.** The green tree builds from the
   same scanner token stream that already feeds `Value` — no second
   parser, no divergence.
5. **Stable foundation for ongoing strictness fixes.** Indentation
   rigor, document hygiene, and other lenient-cluster fixes target
   the existing scanner/loader; the green tree retrofit must not
   require redoing them.

## Non-goals

- Replacing `Value`. `Value` remains the data API for serde-style use.
- Comment *attachment* semantics (which comment "belongs" to which
  node). The capture surface in `src/comments.rs` is sufficient for
  most consumers; node-attached comments can be a follow-up.
- Cross-document editing. `Document` is single-document; multi-doc
  streams are a `Vec<Document>` (see API).
- Schema-aware editing (e.g. "set the value of `version` to a string
  even if it parses as a number"). Caller's responsibility to provide
  the right fragment.

## What already exists

The retrofit lands on top of substantial infrastructure that this
crate already carries:

| Source | What it provides |
| --- | --- |
| `src/parser/scanner.rs` (`Span`, `Token`) | Every emitted token already carries `start..end` byte offsets into the input. The scanner does not need changes for green-tree construction. |
| `src/parser/scanner.rs::ScannedComment` | Comments are captured with byte spans and an `inline` flag. The capture API is in `src/comments.rs` (`load_comments`). |
| `src/parser/events.rs` | Token stream is consumed into a typed `Event` stream (`MappingStart`, `Scalar`, …). Each event carries a `Span`. |
| `src/parser/loader.rs` | Events are folded into `Value`. This is the *current* AST builder. |
| `src/span_context.rs::SpanTree` | An auxiliary tree of spans paired with each `Value` node, populated during loading. |
| `src/spanned.rs::Spanned<T>` | Public type pairing a `T` with its source span; used in serde flows. |

The header comment in `src/comments.rs:24-27` already anticipated this
work:

> The building blocks are here: the scanner now preserves comment
> spans, so a future commit can layer an AST side-table on top
> without re-plumbing the parser.

That commit is what this document specifies.

## Architecture

### Two trees, one parse

```
                                    ┌─────────────────────────┐
                                    │   StreamingDeserializer │  ← unchanged
                                    └─────────────────────────┘
                                                 ▲
            ┌──── tokens ────┬──── events ───────┴── (no value built)
            │                │
       ┌────┴────┐       ┌───┴────────┐
       │ Scanner │ ────▶ │ EventStream│
       └────┬────┘       └───┬────────┘
            │                │
            └──── events ────┼─────▶ ┌────────────────┐
                             │       │ value::Loader  │  ── Value
                             │       └────────────────┘
                             └─────▶ ┌────────────────┐
                                     │  cst::Builder  │  ── GreenNode (Arc)
                                     └────────────────┘            │
                                                                   ▼
                                                         ┌──────────────────┐
                                                         │     Document     │
                                                         │   (mutable API)  │
                                                         └──────────────────┘
```

- `Value` and `GreenNode` are **independent products** of the same
  parse. Either can be built alone (cheap path) or both together (CST
  path) at minimal extra cost.
- The scanner runs once. Token spans are reused.
- Mutation happens on `Document`, never on `Value` directly.

### Green tree shape

The green tree is a `rowan`-style untyped, immutable tree where every
node either:

- Holds source text directly (a leaf `Token`), or
- Holds a sequence of child green nodes plus the syntax kind.

**Critically**, leaves include trivia — whitespace runs, newlines,
indentation runs, comments — not just content tokens. The
concatenation of every leaf's text in document order reproduces the
input byte-for-byte.

Sketch of the core types (lives in a new `src/cst/` module):

```rust
// src/cst/mod.rs

/// Kind of a syntax node or token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SyntaxKind {
    // ── trivia (leaves only) ────────────────────────
    Whitespace,
    Newline,
    Comment,
    // ── structural punctuation (leaves) ─────────────
    DashIndicator,        // `-`
    QuestionIndicator,    // `?`
    ColonIndicator,       // `:`
    AnchorMark,           // `&name`
    AliasMark,            // `*name`
    TagMark,              // `!tag`
    DocStart,             // `---`
    DocEnd,               // `...`
    // ── scalar leaves ───────────────────────────────
    PlainScalar,
    SingleQuotedScalar,
    DoubleQuotedScalar,
    LiteralScalar,
    FoldedScalar,
    // ── composite nodes ─────────────────────────────
    Document,
    BlockMapping,
    BlockSequence,
    FlowMapping,
    FlowSequence,
    BlockMappingEntry,    // `key: value` (with surrounding trivia)
    FlowMappingEntry,
    BlockSequenceEntry,   // `- value` (with surrounding trivia)
    FlowSequenceEntry,
    Key,                  // wraps the key node + the `:`
    Value,                // wraps the value node
}

/// Immutable green node — `Arc`-shared, byte-faithful.
#[derive(Debug, Clone)]
pub struct GreenNode {
    kind: SyntaxKind,
    text_len: usize,            // total byte length, cached
    children: Arc<[GreenChild]>,
}

#[derive(Debug, Clone)]
enum GreenChild {
    Node(GreenNode),
    Token { kind: SyntaxKind, text: Box<str> },
}

impl GreenNode {
    pub fn kind(&self) -> SyntaxKind { self.kind }
    pub fn text_len(&self) -> usize { self.text_len }
    pub fn children(&self) -> impl Iterator<Item = &GreenChild>;
    /// Concatenation of every descendant token's text in document
    /// order. For an unmutated tree, equals the original input.
    pub fn text(&self) -> String;
}
```

The "red" side is implicit: a `SyntaxNode` cursor that walks the
green tree and tracks parent pointers and absolute byte offsets on
demand. Following `rowan`, this is computed lazily — not stored. We
do not need a separate `RedNode` type; cursors satisfy the typed-API
requirement.

### Public API

A new top-level `Document` type sits on top of the green tree:

```rust
// src/cst/document.rs (new file)

/// A YAML document with byte-faithful source preservation.
///
/// Parses input and retains every byte — content, whitespace,
/// comments — so that an unmodified `Document` re-emits identical
/// bytes. Edits rewrite only the affected fragment; surrounding text
/// is preserved verbatim.
///
/// Multi-document streams are returned as `Vec<Document>` from
/// [`parse_stream`].
#[derive(Debug, Clone)]
pub struct Document {
    green: GreenNode,
    /// Cached byte buffer; invalidated on edit.
    cached_emit: OnceLock<String>,
}

/// Parse a single-document YAML string into an editable `Document`.
pub fn parse_document(input: &str) -> Result<Document>;

/// Parse a multi-document stream.
pub fn parse_stream(input: &str) -> Result<Vec<Document>>;

impl Document {
    /// The root syntax node.
    pub fn syntax(&self) -> SyntaxNode<'_>;

    /// Re-emit the document. Byte-identical to the original input
    /// when no edits have been applied.
    pub fn to_string(&self) -> String;

    // ── Typed read-only access ──────────────────────────────────
    pub fn as_value(&self) -> Value;          // one-shot conversion

    // ── Typed mutation (Renovate-class) ─────────────────────────
    pub fn get(&self, path: impl AsPath) -> Option<NodeRef<'_>>;
    pub fn set(&mut self, path: impl AsPath, value: impl Emit);
    pub fn remove(&mut self, path: impl AsPath);

    // ── Span-based mutation (linter/formatter-class) ────────────
    /// Replace the bytes at `span` with `text`. The caller is
    /// responsible for the replacement being syntactically valid in
    /// that position; `Document` re-validates the result before
    /// committing.
    pub fn replace_span(&mut self, span: Span, text: &str)
        -> Result<(), EditError>;
}

/// Path into a YAML document — `JsonPath`-style.
pub trait AsPath { /* "/foo/bar/0" or [Segment::Key("foo"), …] */ }

/// Source for a value being inserted. Implementations exist for
/// `&str`, `i64`, `f64`, `bool`, `Value`, and any `Serialize` type.
/// The `Document` formats the new value to match the existing scalar
/// style at the target site (literal/folded/plain/quoted) wherever
/// possible, and otherwise emits the simplest valid form.
pub trait Emit { fn emit(&self, ctx: EmitCtx<'_>) -> String; }
```

### Four design decisions

#### 1. Separate parse entry point — yes

`parse_document(input)` is a new top-level function alongside
`from_str` / `from_slice` / `load_all`. We do **not** add a
`Value::with_trivia()` constructor; `Value` does not gain a hidden
green-tree field. This keeps the fast path's type signature
unchanged and its size and layout untouched, and prevents accidental
opt-in (an `Emit` of a `Value` from `parse_document` is unambiguous;
an `Emit` of a `Value` from `from_str` is impossible at the
type-system level).

Trade-off: callers who want CST behaviour must opt in explicitly. We
think that is the correct default.

#### 2. Both typed *and* span-based mutation

- **Typed (`get` / `set` / `remove`)** is the headline API. It is what
  Renovate-class consumers want: "set the version at
  `dependencies.serde` to `2.0`."
- **Span-based (`replace_span`)** is the formatter / linter API.
  Users who already operate on spans (e.g. consumers of
  `Spanned<T>`) get a low-level escape hatch.

Both paths converge on the same green-tree rewrite logic. `set` is
implemented as `replace_span(node.span(), formatted)`.

#### 3. Re-emission policy: leave indentation alone

When `set` replaces a value, the surrounding indentation, comments,
and blank lines are preserved verbatim. We do **not** re-indent a
sibling block when one entry's value changes shape, even if the
result is "ugly" by a hand-written style.

This matches `taplo`'s policy. Aggressive reformatting is
out-of-scope for `Document`; consumers who want it run their own
formatter.

The one exception: when `set` introduces a multi-line value where a
single-line value lived (e.g. a literal block scalar replacing a
plain scalar), `Document` indents the new lines to match the entry's
existing column.

#### 4. Anchor / alias under mutation

If an edit targets a node that has an anchor or is the target of an
alias elsewhere in the document:

- **Edit a node with an anchor** → keep the anchor, change the
  content. Aliases continue to refer to the (now-different) node.
- **Edit a node that is aliased elsewhere** (the alias resolves to
  it) → emit a warning; the alias now resolves to a different value.
  This is allowed by default. A strict mode (`Document::strict_aliases`)
  errors instead.
- **Edit an alias node** (`*foo`) directly → forbidden. `set` on an
  alias returns `EditError::AliasNotEditable`. To change the value
  the alias points to, mutate the anchored node.

This is the area where most "CST for YAML" projects accumulate
scars; conservative defaults plus an opt-in strict mode is the only
position that has held up in practice (see `serde_yaml`'s and
`yaml-rust2`'s issues archives).

## Migration plan

Phased; each phase is releasable and independently useful.

### Phase 0 — design lock-in

This document. No code changes.

### Phase 1 — green-tree builder

- New `src/cst/` module: `SyntaxKind`, `GreenNode`, `GreenChild`,
  `SyntaxNode`, `Builder`.
- The builder consumes the existing `Event` stream from
  `src/parser/events.rs` plus the un-skipped trivia from the scanner
  (`ScannedComment` plus a new internal trivia capture for whitespace
  and newlines, which are currently elided by `skip_to_next_token`
  but whose spans are easy to retain).
- New API: `parse_document(input) -> Result<Document>` and
  `parse_stream(input) -> Result<Vec<Document>>`. Read-only initially:
  no `set` or `replace_span` yet.
- New test: `tests/cst/round_trip.rs` — for a corpus of inputs
  (existing `tests/yaml-test-suite/` cases), assert
  `parse_document(s).unwrap().to_string() == s`.
- Acceptance criterion: 100% byte-identical re-emit on every test
  suite case the existing parser accepts.

### Phase 2 — typed mutation

- `Document::get / set / remove` and the `AsPath` / `Emit` traits.
- `tests/cst/mutation.rs` — for each of N curated cases, apply a
  named edit, check the resulting `to_string()` against a golden
  file under `tests/cst/golden/`.

### Phase 3 — span-based mutation + edit validation

- `Document::replace_span`. Re-validates by re-parsing the resulting
  text and checking that the affected node still parses to a
  syntactically valid scalar/collection.
- `EditError` type with the alias / multi-line / shape-mismatch
  variants.

### Phase 4 — round-trip strictness fixes

The remaining 41 lenient cases (indentation rigor, document hygiene,
implicit-key multi-line) are now revisited. The strictness fixes
land at the **scanner / loader** level — the green-tree builder
inherits them automatically. There is no green-tree-specific
strictness work.

This phase only follows phases 1–3 because some of the remaining
strictness fixes (notably indentation rigor) require richer
trivia tracking that the green-tree builder will already need. We
land the trivia capture once.

### Performance contract

Phase 1's introduction of green-tree construction must not slow down
existing entry points. The verification is two-fold:

- The `criterion` benches under `benches/` (`benchmarks`,
  `comparison`, `architecture`, `validation_overhead`) gate at ±5%
  against `main`. CI fails on regression.
- The `parse_document` path is benchmarked separately. Its overhead
  vs `from_str::<Value>` should be ≤2× — comparable to `taplo`'s
  ratio. The streaming path stays the undisputed champion for
  `from_str::<MyType>` use.

## Open questions

- **YAML 1.1 vs 1.2 directive handling.** The spec is loose on
  preserving directive *text* across a round-trip. Decision needed:
  does `parse_document → set → to_string` preserve the original
  `%YAML 1.2` directive byte-for-byte, or canonicalise it? Default
  proposal: preserve verbatim (consistent with the indentation
  policy).
- **Preserving non-canonical scalar styles.** A plain `42` and a
  quoted `"42"` deserialise to the same `Value`, but their CST
  representations differ. `set(path, 42)` on a quoted-style site
  should emit `"42"`, not `42`. Decision: yes — `Emit` carries an
  `EmitCtx` that knows the existing style at the target.
- **Does `Document` implement `serde::Deserialize`?** Probably not —
  forces consumers through `Document::as_value()` which is the
  intended boundary. But `Document::as_value()` should be cheap; the
  green tree already has all the data.
- **Memory layout.** `GreenChild::Token { text: Box<str> }` allocates
  per leaf. For very large documents (k8s-class manifests), an arena
  may be worth the complexity. Defer to phase 1 measurements.

## Test / regression strategy

- **Round-trip property test.** Every yaml-test-suite case that
  parses must round-trip byte-identically. Wired into
  `tests/yaml_compliance_report.rs`'s existing infrastructure.
- **Golden mutation tests.** Per phase 2 above; `cargo test
  --test cst_mutation -- --bless` to regenerate.
- **Fuzz harness.** Extend the existing `fuzz/` corpus with a
  green-tree round-trip target — for any input the parser accepts,
  `parse_document → to_string → parse_document → to_string` must be
  a fixed point.
- **Anchor/alias scar tests.** A dedicated test file in
  `tests/cst/anchors.rs` for the four scenarios in §4.

## What this design explicitly avoids

- **`rowan` as a hard dependency.** We replicate the green-node
  pattern directly. `rowan` is excellent but pulls in `text-size`,
  `countme`, and friends; the implementation here is small enough
  (≤500 LoC) that the dependency cost is not justified for a YAML
  parser. If we need the typed-syntax-tree generator later, we can
  reconsider.
- **A second tokenizer.** The green tree is built from the *same*
  scanner output that already feeds `Value`. Strictness fixes in
  the scanner improve both immediately and stay improved.
- **Implicit fallback to lenient parsing on edit.** If `replace_span`
  produces invalid YAML, the edit is rejected with `EditError`.
  `Document` never silently emits invalid output.

## References

- `rowan` (rust-analyzer) — green-tree pattern.
- `taplo` (TOML) — single-document edit-aware API; closest analogue
  in the broader ecosystem.
- YAML 1.2.2 §6 – §9 — production rules informing the green-tree
  granularity.
- `tests/yaml-test-suite/` — round-trip corpus.

---
THE ARCHITECT ᛫ Sebastien Rousseau ᛫ https://sebastienrousseau.com
THE ENGINE ᛞ EUXIS ᛫ Enterprise Unified Execution Intelligence System ᛫ https://euxis.co
