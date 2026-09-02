<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.14 Release Notes

The **loader-parity** cut. Closes a fast-path silent-collapse
of distinct-typed mapping keys, brings four DoS budgets on the
span-free loader up to par with the span-full loader, adds a
coarse-grained `Error::kind()` classifier, and lands five CST
`span_at` correctness fixes plus a scanner lone-CR fix from
the earlier commits on this branch.

No breaking API changes. No MSRV change (still 1.85). No new
runtime dependencies.

## Why this release exists

`from_str::<Value>` — the default typed-vs-`Value` entry point
in noyalib — used to route through the streaming deserializer
when the target was `Value`, and through a span-free
`NoSpanLoader` when streaming was ineligible. Both paths
silently collapsed distinct-typed mapping-key collisions:

```yaml
1: a
"1": b
```

The integer key `1` and the string key `"1"` both stringify to
`"1"`, so the second `insert` dropped the first. The CST
loader caught this and refused with `Error::KeyCollision`.
The default `Value`-target path did not — a real data-loss
regression relative to the `cst::parse_document` story.

The span-free loader also lacked the DoS budgets the span-full
loader carries (`max_sequence_length`, `max_mapping_keys`,
`max_merge_keys`), so an oversized flat sequence, mapping, or
`<<`-chain bypassed those guards on the `Value` fast path.

v0.0.14 closes both gaps.

## What changed

### Loader-parity (security)

- `NoSpanLoader::push_value` now runs the same distinct-typed
  collision check the span-full `Loader` has carried since
  v0.0.13. YAML with two keys that stringify to the same
  spelling but originate from different YAML scalar types is
  refused with `Error::KeyCollision` regardless of
  `DuplicateKeyPolicy`.
- `NoSpanLoader` now enforces `max_sequence_length`,
  `max_mapping_keys`, `max_merge_keys`, and the alias-bytes
  billion-laughs guard bounded by `max_document_length`. Same
  spellings, same error variants as the span-full loader.
- `NoSpanLoader` now honours `DuplicateKeyPolicy::First` /
  `Last` / `Error`. Previously always last-wins.
- `MergeKeyPolicy::Error` is now enforced on both loaders.
- The streaming path is now bypassed for the `Value` target
  (unless a `TagRegistry` is active) so the collision check is
  actually reachable. See `crates/noyalib/src/de.rs`
  `from_str_with_config` for the eligibility rule.
- `from_str_with_config` enforces `max_document_length` inline
  when streaming is skipped, so an oversized document can't
  reach the AST loader either way.

### Hot-path clone gate

- The typed-key `Value::clone()` retained on every mapping key
  is now skipped when the key is a merge key (`<<`) that will
  be buffered rather than inserted. `<<`-heavy documents no
  longer pay the clone cost. Gated by a `is_buffered_merge_key`
  check that reads the current `MergeKeyPolicy`.
- `debug_assert_eq!(map.len(), typed_keys.len())` now runs
  after every push in both loaders so a future policy branch
  desync surfaces in debug builds.

### API additions

- `Error::kind() -> ErrorKind` — coarse-grained routing
  without matching every variant of the `#[non_exhaustive]`
  `Error` enum. `ErrorKind` is itself `#[non_exhaustive]`;
  future variants land under an existing kind whenever
  possible.
- Anchor typo suggestions on the AST loaders: both `Loader`
  and `NoSpanLoader` now populate
  `Error::UnknownAnchorAt::suggestion` with the closest known
  anchor and its definition location, matching the streaming
  path's existing `build_unknown_anchor`.
- Special-value float keys (`nan` / `inf` / `-inf`) now use
  their canonical plain-scalar spelling when stringified,
  instead of Rust's `{:?}` output (`"NaN"`).

### CST span fixes (from earlier commits on this branch)

- Alias references resolve through to the anchor value's span.
- Block-collection value spans include their first line's
  indentation, so the returned slice re-parses to the
  selected value.
- Keep-chomped block scalars (`|+`, `>+`) retain their kept
  trailing blank lines in `span_at`.
- Implicit-null nodes report no span (`None`) instead of the
  indicator character's location.
- Distinct-typed key-collision guard on the span-full loader
  (already shipped; now mirrored on the span-free loader per
  above).

### Scanner fix

- Lone `\r` (classic-Mac CR-only line breaks) is now a valid
  line break, per YAML 1.2.2 §5.4.

### Testing / tooling

- `tests/no_span_loader_parity.rs` — nine tests covering the
  distinct-typed collision, `max_sequence_length`,
  `max_mapping_keys`, `max_merge_keys`, `MergeKeyPolicy::Error`
  refusal, `DuplicateKeyPolicy::First` selection, and merge-key
  no-collision behaviours on the `Value` fast path. Every test
  fails without the parity fix.
- `tests/error_kind.rs` — twelve tests pinning the
  `Error::kind()` mapping.
- `benches/mapping_key_clone.rs` — criterion bench for the
  integer-keyed, string-keyed, and merge-heavy mapping-key hot
  path. Baseline for future clone-cost regression detection.
- `fuzz/fuzz_targets/fuzz_no_span_loader.rs` — cross-checks
  `from_str::<Value>` against `cst::parse_document` on
  arbitrary input; any divergence panics. Corpus seeded with
  the collision reproducers.

## What did not change

- No breaking API changes. `Error` and `ErrorKind` are both
  `#[non_exhaustive]`, so the new variant and the new method
  are additive.
- No MSRV change. Still `rust-version = "1.85.0"`.
- No new runtime dependencies. `Cargo.lock` movement in this
  release comes from Dependabot bumps for `serde-saphyr`,
  `rustc-hash`, and `jsonschema` only.
- `#![forbid(unsafe_code)]` intact at every crate root.

## Follow-ups noted for v0.0.15

- `max_sequence_length` and `max_mapping_keys` currently
  surface as `Error::Serialize("… limit exceeded")` (historical
  spelling) and classify as `ErrorKind::Data` in the new
  classifier. Migrating both to `Error::Budget(BudgetBreach::
  MaxSequenceLength{…} | MaxMappingKeys{…})` would give the
  classifier full parity — worth doing but out of scope for
  v0.0.14.
- `cargo-semver-checks` on the release workflow to guard the
  0.0.x → 0.0.99 runway against accidental breaks.
