<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.10 Release Notes

The **BOM scanner** cut. A single-theme patch release focused entirely
on scanner correctness for UTF-8-BOM-prefixed multi-node documents.
Contributed by [@zoosky](https://github.com/zoosky) — original PR
[#118](https://github.com/sebastienrousseau/noyalib/pull/118), rebased
onto post-v0.0.9 `main` as
[#123](https://github.com/sebastienrousseau/noyalib/pull/123) with
authorship preserved byte-for-byte.

No public API change. No MSRV change (still 1.85).

## Highlights

* **Leading UTF-8 BOM is now transparent to indentation and comments.**
  A leading UTF-8 BOM (`U+FEFF`, three bytes `0xEF 0xBB 0xBF`) used
  to make any document with more than one node fail to parse on the
  strict `parse_document` path. The scanner consumed the BOM with
  `advance_by(3)` while counting those three bytes toward the column
  of the following content, so:

  ```
  <BOM>a: 1
  b: 2
  ```

  errored with `stray content after document — subsequent documents
  must start with '---'`.

  The same column miscount also broke BOM-prefixed sequences, nested
  mappings, and a `<BOM>#`-style first-line comment. Single-node
  BOM-prefixed documents parsed *by accident* — there was no
  following sibling to trip the dedent check, so the bug went
  unnoticed for a long time.

* **Root cause was three co-dependent scanner bugs.** A leading BOM
  is a zero-width stream prefix: the content that follows begins at
  column 0. Three code paths in `crates/noyalib/src/parser/scanner.rs`
  treated the consumed BOM bytes as content preceding the next token:

  1. `fetch_stream_start` consumed the BOM with `advance_by(3)`,
     which added 3 to the incremental column counter.
  2. The simple-key indent recomputed a key's column from its byte
     offset (`sk.index - line_start`); on the first line
     `line_start` was 0, so the BOM's three bytes counted as
     indentation. The first key landed at column 3, and a following
     sibling at column 0 unrolled the mapping — a premature document
     end (or a split mapping).
  3. The block-context comment check rejected a `#` whose preceding
     byte was not whitespace/break — after a BOM that byte is `0xBF`.

* **Three surgical fixes.**
  * `fetch_stream_start` now resets `self.col = 0` after
    `advance_by(3)`.
  * The simple-key indent path skips a leading BOM when computing
    `line_start`.
  * The block-context comment check treats `#` immediately after a
    leading BOM as a start-of-input comment.

  Each fix is gated on a `pos == 3` or `line_start == 0 &&
  starts_with(&[0xEF, 0xBB, 0xBF])` check so the new behaviour only
  fires on a *leading* BOM — interior `0xEF` bytes in legitimate
  UTF-8 codepoints are never mistaken for one.

* **BOM stays byte-faithful in the CST.** The BOM is still recorded
  as a `Bom` trivia leaf, so `parse_document(s).to_string() == s`
  holds — the round-trip semantics that CST-based tooling depends on
  are preserved. BOM-prefixed files now round-trip instead of
  erroring.

## Behaviour matrix

| input | before | after |
|---|---|---|
| `<BOM>a: 1\nb: 2\n` | parse error | round-trips |
| `<BOM>- 1\n- 2\n` | parse error | round-trips |
| `<BOM>a:\n  b: 1\n` | parse error | round-trips |
| `<BOM>a: 1\r\nb: 2\r\n` | parse error | round-trips |
| `<BOM># c\nname: x\n` | parse error | round-trips |
| `<BOM>a: 1\n` (single node) | round-trips | round-trips |

## Tests

Two new scanner-level regression tests in
`crates/noyalib/src/parser/scanner.rs::tests`:

* `test_scanner_leading_bom_multi_key_mapping` — asserts a
  BOM-prefixed multi-key block mapping scans to the same token
  stream as the same document without the BOM, and that both keys
  plus the closing `BlockEnd` are present.
* `test_scanner_leading_bom_is_transparent` — asserts BOM-prefixed
  block sequence, nested mapping, and leading-comment inputs each
  scan identically to their BOM-less counterparts via a shared
  `scan_kinds` helper that produces precise diff diagnostics on
  regression.

## Note on why existing coverage missed this

The pre-existing BOM tests in
`crates/noyalib/tests/coverage_100.rs` (`scanner_bom_at_start`,
`scanner_bom_utf8`) both use single-key mappings — exactly the
"parses by accident" case @zoosky's writeup called out. The 100%
line-coverage gate held but missed the behaviour gap. The two new
regression tests close that gap at the token-stream level.

## Public API / behaviour

No change on the public API. The one user-visible behaviour change
is that BOM-prefixed multi-node inputs which previously errored now
parse cleanly. This is exclusively a bug fix — no downstream code
was relying on the erroring behaviour.

## Verification

`cargo test parser::scanner::tests::test_scanner_leading_bom` on
`fix/leading-bom-column-reset-rebased` before merge:

```
running 2 tests
test parser::scanner::tests::test_scanner_leading_bom_multi_key_mapping ... ok
test parser::scanner::tests::test_scanner_leading_bom_is_transparent ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 162 filtered out
```

CI 100% green: 21 of 21 required checks succeed (build/test on
Ubuntu / macOS / Windows × stable / nightly, MSRV 1.85, `no_std`,
Miri focused, coverage gate ≥95%, cargo-deny, cargo-vet,
cargo-machete, cargo-semver-checks, differential fuzz, CodeQL,
Dependency Review, REUSE.software compliance, signed-commit
verification, vendor + offline build).

## Credits

Contributed by [@zoosky](https://github.com/zoosky) — Zoo Sky.
Original PR was retargeted from `main` to `feat/v0.0.9`, then rebased
onto post-v0.0.9 main on his behalf as PR #123 (author attribution
preserved, `Co-authored-by: Zoo Sky <zoosky@gmail.com>` in the
squash-merge commit).
