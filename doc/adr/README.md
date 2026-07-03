# Architecture Decision Records

This directory holds the architectural decisions that shape
noyalib — the choices that would be expensive to reverse and
that contributors should understand before proposing structural
changes.

## Format

Every ADR uses [Michael Nygard's format](https://github.com/joelparkerhenderson/architecture-decision-record/tree/main/locales/en/templates/decision-record-template-by-michael-nygard)
— see [`TEMPLATE.md`](./TEMPLATE.md). Sections are:

- **Status:** proposed / accepted / superseded / deprecated
- **Context:** what forces are at play
- **Decision:** what we're doing
- **Consequences:** what becomes easier and harder

ADRs are **immutable** once accepted — if a decision changes, the
old ADR moves to "superseded" and a new ADR is added with a
reference back. Nothing is silently rewritten.

## Index

| # | Title | Status |
|---|---|---|
| [0001](./0001-cst-rowan-shape.md) | CST shape: parallel green tree, not unified | accepted |
| [0002](./0002-yaml-1.2-default.md) | YAML 1.2 strict semantics by default; 1.1 opt-in | accepted |
| [0003](./0003-zero-unsafe-policy.md) | `#![forbid(unsafe_code)]` workspace-wide | accepted |
| [0004](./0004-lossless-u64-integers.md) | Opt in to lossless `u64` integers | proposed |
| [0005](./0005-workspace-split.md) | Split workspace into 4 satellite repositories with strict-lockstep versioning | accepted |

## When to add an ADR

Add one when you're about to make a decision that:

- Is hard to reverse (changes the data model, public API surface,
  dependency floor, or core invariants like the unsafe policy)
- Will surprise a future contributor reading the code
- Has plausible alternatives that someone might propose later

Don't add ADRs for routine implementation choices — those go in
commit messages and code comments. The bar is "would I want a
new contributor to read this before proposing the opposite?"

## When *not* to add an ADR

- The decision is captured by the type system or a test
- The decision is implicit in well-known Rust idiom
- The decision is genuinely reversible without consequence
