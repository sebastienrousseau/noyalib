<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.6 — Release Notes

The **Ecosystem Integration** cut. Lands the four remaining
open issues from the v0.0.6 milestone (#22, #24, #25, #33) and
closes out the leftover stabilisation checklist (#19).

## Highlights

* **Error-recovering parser.** `noyalib::recovery::parse_lenient`
  returns a `ParseResult` carrying the best-effort tree plus the
  list of every error encountered. LSP / IDE consumers can keep
  showing autocomplete and diagnostics on half-typed documents.
  New `recovery` Cargo feature; no extra deps.
* **`sval` streaming adapter.** Alternative to the default serde
  route for callers wanting to skip `serde_derive`'s compile-time
  overhead. New `sval` Cargo feature.
* **Native tokio async parsing.** `from_async_reader` /
  `from_async_reader_multi` parse from any `tokio::io::AsyncRead`
  without `spawn_blocking`. `YamlDecoder<T>` is a
  `tokio_util::codec::Decoder` for plugging streaming YAML
  parsing into a tower-middleware chain. New `tokio` Cargo
  feature.
* **npm Trusted Publishing.** The release pipeline drops the
  long-lived `NPM_TOKEN` and uses OIDC to authenticate. Compromise
  window collapses from 1 year to ~10 minutes.

## What ships

### `noyalib::recovery` (issue #22, P1)

```rust
use noyalib::recovery::parse_lenient;
let r = parse_lenient("a: 1\nb: [unclosed\n");
assert!(!r.is_complete);
assert!(!r.errors.is_empty());
// `r.value` is the best-effort recovered tree.
```

Recovery strategies — strict pass first, then
`DuplicateKeyPolicy::Last` retry, then line-truncation retry
that drops trailing lines until something parses. Multi-document
input is split on `---` and each document is recovered
independently. Error collection is capped via
`LenientConfig::max_errors`.

For multi-document input the result's `value` is a
`Value::Sequence` of per-document values (recovered or `Null`);
for single-document input it is the recovered document directly.

### `noyalib::sval_adapter` (issue #25, P2)

Adds `impl sval::Value for Value`, `Number`, `Mapping`,
`MappingAny`, and `TaggedValue`, plus a
`noyalib::sval_adapter::to_sval_writer` entry point that streams
a noyalib value graph to any `sval::Stream` consumer.

serde remains the default; `sval` is opt-in for callers who want
to avoid the binary-size cost of serde monomorphisation or the
~5–10s compile overhead of `serde_derive` on large projects.

### `noyalib::tokio_async` (issue #24, P2)

```rust
// Single document
let pkg: Pkg = noyalib::tokio_async::from_async_reader(&mut reader).await?;

// Multi document
let docs: Vec<Pkg> = noyalib::tokio_async::from_async_reader_multi(&mut reader).await?;

// Codec — for tower middleware chains
let framed = tokio_util::codec::FramedRead::new(
    reader,
    noyalib::tokio_async::YamlDecoder::<Pkg>::new(),
);
```

The codec emits one parsed `T` per `---`-delimited document.
Per-document boundary detection follows the YAML 1.2.2 §9.1.2
`---` grammar — column-0 marker followed by whitespace or EOL.

### npm Trusted Publishing (issue #33, security)

`.github/workflows/release-binaries.yml` no longer reads
`NPM_TOKEN`; both `@noyalib/noyalib-wasm` and `noyalib-mcp`
publish jobs declare `id-token: write` and rely on the OIDC
handshake against per-package trusted-publisher policies
configured at `https://www.npmjs.com/package/<name>/access`.
`pkg/PUBLISH.md` §6 documents the bootstrap + secret-retirement
flow.

The `--provenance` flag stays attached to both publish steps so
the npm verified-publisher badge keeps linking back to the
exact GitHub Actions run that produced the artefact.

### API stabilisation checklist (issue #19, P1)

Folded into the v0.0.5 + v0.0.6 cuts:

* `#[non_exhaustive]` on all public configuration types — done
  via v0.0.5 audit. `ParseResult` (new in v0.0.6) follows the
  same pattern; `LenientConfig` deliberately does not, so
  callers can use struct-literal syntax (`LenientConfig { … }`).
* All public functions have doc-comments with examples — the
  workspace `missing_docs = "warn"` + the strict-doc gate on
  CI enforce it.
* `Error` enum's variant set is comprehensive and actionable —
  no new variants needed; `i18n::MessageFormatter` (v0.0.5)
  surfaces a customisable user-facing rendering layer.
* `CHANGELOG.md` updated with all changes since v0.0.1 —
  present.
* Benchmark suite — `crates/noyalib/benches/comparison.rs`
  compares against `serde_yaml_ng`, `yaml-rust2`,
  `yaml-spanned`, with optional `serde-saphyr` via the
  `compare-saphyr` feature. (`serde_yml` was retired from the
  comparison in v0.0.6 — see the OpenSSF Scorecard pass below.)

## Bug fixes

### `pnpm-lock.yaml` recursion-limit false positive (issue #46)

`from_str::<Value>` on a `pnpm-lock.yaml`-shaped input failed
with `Error::RecursionLimitExceeded { depth: 129 }` even when
the document was only a few levels deep. Root cause:
`StreamingMapAccess::next_key_seed` did not check the access
object's `finished` flag, so serde visitors that call
`next_entry` after `Ok(None)` — `ValueVisitor::visit_map` does
this — read the next event from the **parent** mapping and
treated it as belonging to the now-exhausted child. The
recursive `deserialize_any` on each spilled value inflated
`self.depth` by exactly one per empty flow `{}`, hitting
`max_depth + 1` after 128 entries.

Fix: `finished`-guard early-returns in the three
`StreamingMapAccess` / `StreamingSeqAccess` iterators, plus
balanced `depth` decrement on both `Ok` and `Err` in
`deserialize_any` / `deserialize_seq` / `deserialize_map` so
a failed inner visit can't leak depth either.

Companion audit finding: the `NoSpanLoader` path
(`crates/noyalib/src/parser/loader.rs`) — used by
`from_str::<Value>`'s value-target fast path and by `no_std`
multi-document loading — incremented `self.depth` on
`SequenceStart` / `MappingStart` but **did not check**
against `max_depth` (the span-tracked loader does).
Adversarial deeply-nested input through that path could
consume stack without ever firing `RecursionLimitExceeded`.
Now mirrored from the span loader.

10 regression tests in `crates/noyalib/tests/issue_46.rs`:
50 000-package full `pnpm-lock-v9` shape, 3 000 consecutive
empty flow mappings, deterministic depth-cliff probe at
`n ∈ [100, 128, 129, 130, 200, 500, 1000]`, complex
peer-dependency keys, 200-level-deep sequence for the no-span
path.

No API change. Documents that previously failed with
`RecursionLimitExceeded` now parse cleanly; documents that
previously parsed are unchanged.

## Compatibility

* MSRV unchanged from v0.0.5 (1.85, edition 2024).
* Default-features behaviour unchanged.
* All new functionality is gated behind opt-in Cargo features
  (`recovery`, `sval`, `tokio`); no impact on existing call
  sites that don't enable them.
* Issue #46 fix + no-span depth-check fix are pure bug fixes
  — no API change.

## Acknowledgements

Thanks to the LSP-tooling community (issue #22 reporters) and
the npm Security team's Trusted Publishing rollout for shaping
the API surface here.
