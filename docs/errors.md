<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Error reference

Every error `noyalib` can return, what raises it, and the stable
code a tool can match on.

**Generated** by `scripts/generate-reference-docs.sh` from
`crates/noyalib/src/error.rs`. Do not edit by hand: re-run the
script. `tests/reference_docs_are_complete.rs` fails when this file
and the source disagree, so a new variant cannot land undocumented.

## Kinds

`Error::kind()` collapses the variants below into a small set, so
callers can route on a category without matching a `#[non_exhaustive]`
enum.

| Kind | Meaning |
| --- | --- |
| `ErrorKind::Syntax` | The document was malformed: unterminated flow collection, bad indentation, invalid escape, unexpected token, etc. |
| `ErrorKind::Io` | An I/O operation failed while reading the input. |
| `ErrorKind::Budget` | A configurable DoS budget or hard security limit was exceeded (recursion depth, alias expansion, mapping size, merge-key count, …). Covers every `Error::Budget` breach as well as [`Error::RecursionLimitExceeded`] and [`Error::RepetitionLimitExceeded`]. |
| `ErrorKind::Policy` | A user-supplied [`crate::policy::Policy`] rejected the document, or a policy-only structural rule (merge-value shape, scalar-in-merge) tripped. |
| `ErrorKind::KeyCollision` | Two distinct-typed YAML keys collapsed to the same string key — see [`Error::KeyCollision`]. |
| `ErrorKind::IntegerOverflow` | A plain decimal integer beyond `u64::MAX` was refused under [`crate::ParserConfig::integer_overflow_errors`] — see [`Error::IntegerOverflow`]. |
| `ErrorKind::NonScalarKey` | A non-scalar mapping key was refused under [`crate::NonScalarKeyPolicy::Error`] — see [`Error::NonScalarKey`]. |
| `ErrorKind::DuplicateKey` | A genuine duplicate key was refused under [`crate::DuplicateKeyPolicy::Error`]. |
| `ErrorKind::EndOfStream` | The parser reached end-of-stream where more input was required. |
| `ErrorKind::Data` | The document parsed but the requested target type could not be built (missing field, unknown field, type mismatch, bad tag, unknown anchor, …). Covers the whole serde-facing "shape doesn't match" family. |
| `ErrorKind::Other` | The error came in via [`Error::Custom`] or [`Error::Message`] (usually from a `serde_core::de::Error` bridge) and doesn't map cleanly to any of the other kinds. |

## Variants

32 variants. `code()` is the stable identifier exposed
through `miette::Diagnostic`; it is part of the public surface and
changes only in a breaking release.

| Variant | `code()` | Raised when |
| --- | --- | --- |
| `Error::Parse` | `noyalib::parse` | Error during YAML parsing. |
| `Error::ParseWithLocation` | ``noyalib::error`` | Error during YAML parsing with location information. |
| `Error::Serialize` | `noyalib::serialize` | Error during serialization. |
| `Error::Deserialize` | `noyalib::deserialize` | Error during deserialization. |
| `Error::DeserializeWithLocation` | ``noyalib::error`` | Error during deserialization with location information. |
| `Error::Io` | `noyalib::io` | — |
| `Error::Custom` | ``noyalib::error`` | Custom error message. |
| `Error::RecursionLimitExceeded` | `noyalib::recursion_limit` | Error when recursion depth limit is exceeded. |
| `Error::DuplicateKey` | `noyalib::duplicate_key` | Error when a duplicate key is encountered. |
| `Error::DuplicateKeyAt` | ``noyalib::error`` | A duplicate mapping key, with where it is. The located form of [`Self::DuplicateKey`], raised under [`crate::DuplicateKeyPolicy::Error`] by the parsers that know the key's position -- every `from_str` entry point and the CST parser. `path` is the dotted path of the entry (`site.name`; a sequence index counts as a segment, `items.0.name`), `location` where the second occurrence begins. [`Self::kind`] reports [`ErrorKind::DuplicateKey`] for both forms and [`Self::location`] returns the position. |
| `Error::KeyCollision` | `noyalib::key_collision` | Two distinct-typed keys collapsed to the same string key. The mapping key model is `Mapping<String, Value>`, so keys are stringified. Distinct YAML keys that share a spelling — e.g. the integer `1` and the string `"1"`, or `true` and `"true"` — would silently overwrite each other, losing an entry. This is raised instead, carrying the collapsed string key. Unlike [`Self::DuplicateKey`], it fires regardless of `DuplicateKeyPolicy` because it is data loss, not an authored duplicate. |
| `Error::KeyCollisionAt` | ``noyalib::error`` | Two distinct-typed keys collapsed to the same string key, with where the second one is. The located form of [`Self::KeyCollision`], raised by the parsers that know the key's position. `path` is the dotted path of the entry, `location` where the colliding key begins. [`Self::kind`] reports [`ErrorKind::KeyCollision`] for both forms. |
| `Error::RepetitionLimitExceeded` | `noyalib::repetition_limit` | Repetition limit exceeded (security limit against billion-laughs). |
| `Error::IntegerOverflow` | `noyalib::integer_overflow` | A plain decimal integer beyond `u64::MAX` was refused under [`crate::ParserConfig::integer_overflow_errors`]. |
| `Error::NonScalarKey` | `noyalib::non_scalar_key` | A non-scalar mapping key was refused under [`crate::NonScalarKeyPolicy::Error`]. |
| `Error::Budget` | `noyalib::budget` | A configurable parser budget was exceeded. Carries a [`BudgetBreach`] identifying which limit fired, the configured cap, and (where meaningful) the observed value at the moment the cap tripped. Distinct from the older [`Error::RecursionLimitExceeded`] / [`Error::RepetitionLimitExceeded`] variants — those stay for backwards compatibility on the depth / alias-expansion limits; new budgets in the v0.0.2 expansion (`max_events`, `max_nodes`, `max_total_scalar_bytes`, `max_documents`, `max_merge_keys`, `alias_anchor_ratio`) all flow through `Error::Budget`. |
| `Error::UnknownAnchor` | `noyalib::unknown_anchor` | Unknown anchor encountered. |
| `Error::UnknownAnchorAt` | ``noyalib::error`` | Unknown anchor encountered at a specific location. |
| `Error::MissingField` | `noyalib::missing_field` | Missing field in a mapping. |
| `Error::UnknownField` | `noyalib::unknown_field` | Unknown field in a mapping (with `deny_unknown_fields`). |
| `Error::ScalarInMergeElement` | ``noyalib::error`` | Scalar encountered where a mapping was expected during merge. |
| `Error::SequenceInMergeElement` | ``noyalib::error`` | Sequence encountered where a mapping was expected during merge. |
| `Error::TaggedInMerge` | ``noyalib::error`` | Tagged value encountered during merge. |
| `Error::Invalid` | ``noyalib::error`` | Generic invalid construct error. |
| `Error::TypeMismatch` | `noyalib::type_mismatch` | A type mismatch error. |
| `Error::Shared` | ``noyalib::error`` | Shared error instance (Arc-wrapped for cloning). |
| `Error::EndOfStream` | `noyalib::eof` | End of stream reached unexpectedly. |
| `Error::MoreThanOneDocument` | `noyalib::multi_document` | More than one document found where one was expected. |
| `Error::ScalarInMerge` | ``noyalib::error`` | Scalar in merge (legacy variant). |
| `Error::EmptyTag` | ``noyalib::error`` | Empty tag encountered. |
| `Error::FailedToParseNumber` | ``noyalib::error`` | Failed to parse a number. |
| `Error::Message` | ``noyalib::error`` | A message error from Serde (compat variant). |

## Reading an error against its source

Every located variant renders with the offending line when given the
input it came from:

```rust
# use noyalib::Value;
let input = "port: [unclosed";
let err = noyalib::from_str::<Value>(input).unwrap_err();
println!("{}", err.format_with_source(input));
```

`render`, `format_with_source_radius` and
`format_with_source_truncated` are the same idea with control over
how much context is shown. See the API reference for the full set.
