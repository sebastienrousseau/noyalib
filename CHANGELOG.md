# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Bracket-quoted key segments in the path grammar** (#388,
  ADR-0012): `labels["app.kubernetes.io/name"]` or `['a[0]']`
  addresses a mapping key the grammar would otherwise read as
  structure (`.`, `[`, `]`, `*`), in every path-taking API at once --
  `Value::get_path` and `query`, the borrowed reads, and every
  `cst::Document` mutator and locator (`set`, `set_value`, `set_path`,
  `remove`, `rename_key`, `span_at`, `key_span`, the comment editors,
  `Entry`). `\` escapes the next character inside the quotes. The
  `path` module is public and spells such paths: `quote_key` for one
  segment, `push_key` to append a key in whichever form reads back as
  that key, `join_keys` for a whole path from literal keys.

### Fixed

- **`insert_entry` and `insert_entry_value` upsert a key holding `.`,
  `[`, `]`, or `*`** instead of refusing it, and the upsert of an
  existing `*` no longer appends a second `*` entry (the existing-key
  guard covered `.` and `[` only). The `.`/`[` refusal from #288 is
  gone. `set_path` re-spells a quoted ancestor when it creates a
  missing level under one, and `rename_key`'s bracket check accepts a
  quoted segment while still refusing an unquoted non-index one
  (`servers[web]`), now naming the quoted spelling in the error.

## [v0.0.32] - 2026-09-03

### Fixed

- **`span_at` for a block sequence in value position reports the
  sequence's full extent** (#375, breaking on the spans axis): the
  loader seals a block sequence's span at its last item (a block
  shape has no end token, and the end event carries the next
  token's span), flow sequences keep their `]`, and the green-tree
  fast path defers indentless sequences to the typed cache it
  cannot measure structurally. Anchored flow sequences keep their
  closing bracket via property-aware flow detection. The whole-tree
  span invariant suite now asserts parent containment strictly,
  with no carve-outs.

### Removed

- The `doc/` tombstone directory (the v0.0.31 transition pointer).

## [v0.0.31] - 2026-09-03

### Added

- **`cst::parse_document_with_config` / `cst::parse_stream_with_config`**
  (#372, #373 — implemented by @zoosky): the lossless CST honors a
  caller-supplied `ParserConfig`, mirroring `from_str_with_config`.
  The `Document` keeps the configuration for every re-parse of its
  own source — the typed cache behind `as_value`, `validate`, the
  `replace_span` safety net, the comment-edit value guard, and
  schema coercion — so a merge-heavy values file that only trips the
  `alias_anchor_ratio` heuristic gets its byte-preserving path back
  and stays readable and editable after edits. Disabling the ratio
  does not loosen the absolute amplification budgets, and the
  default entry points are unchanged.
- **The rendered User Manual (Phase 3).** `docs/` is an mdBook root
  whose chapters are the existing Markdown files in place; `docs.yml`
  builds it beside the strict rustdoc into one Pages site (landing at
  `/`, API at `/noyalib/`, manual at `/manual/`);
  `scripts/check-docs-links.sh` gates every chapter and relative
  link. Every README in the family opens Documentation with the same
  four entry points.
- **Phase 4/5 groundwork and contributor DX** (with the noya-cli
  half in its own repository): `docs/packaging.md` addressed to
  distro maintainers (licensing, MSRV policy, the lockstep pin
  model, offline builds, artefact verification), a manual chapter
  for it, `.devcontainer/`, `.pre-commit-config.yaml`,
  `CITATION.cff`, and `AGENTS.md` stating the invariants an
  AI-assisted contribution must respect.

- **Automation resilience, adapted from zfb's harness** (the project
  whose evaluation gated the serde_yaml migration): self-tests for
  the release gate scripts (`scripts/tests/run.sh`, CI job
  `gate-selftests` — each gate proven against known-good and
  known-bad fixtures); corpus integrity and category-coverage
  meta-tests pinning the 18-case contract by sha256; a shipped-size
  monitor with declared budgets (`scripts/size-budgets.toml`); a
  whole-tree span-invariant suite (`tests/span_tree_invariants.rs`),
  which immediately surfaced the long-standing #375 block-sequence
  span quirk, now carved out and tracked; `actionlint` for every
  workflow file (shared, satellite-consumable); a weekly
  registry drift net (registry-drift-net.yml) installing every published crate and npm
  package clean-room; and a bare-container musl smoke in the
  noya-cli release matrix.

### Fixed

- **A comment edit can no longer break the document** (found by
  `fuzz_editors` the moment the v0.0.31 branch ran it): comment text
  containing a carriage return ended the comment token in YAML, so
  the remainder leaked into the document. `set_comment` now refuses
  line-breaking text (inline comments take a single line; `Before`
  still splits on `\n` by documented design) and leaves the
  document byte-identical. The `-0b0` differential finding is
  pinned spec-side: YAML 1.2 has no binary resolution, the compat
  shim keeps the 1.1 reading.

### Changed

- **Repository layout, Phase 1 of the structure plan.** `doc/` is
  now `docs/` (a `doc/README.md` tombstone catches deep links from
  pre-v0.0.31 published documentation); the `noyafmt.1` /
  `noyavalidate.1` manpages and the shell completions moved to the
  noya-cli repository beside their generator; `DEVELOPMENT.md` at
  the root is the single developer entry point; `.editorconfig`,
  `.markdownlint.yaml` and `.codespellrc` land with a per-push
  `docs-lint` CI gate (shared-docs-lint.yml, consumable by the
  satellites) enforcing the family layout contract.
- **The CI duration monitor learns declared budgets**: an
  `EXPECTED_MIN_BASELINE` floor (725s, reason documented inline)
  covers intentional job-set changes like the v0.0.30 gates, while
  accidental regressions still fire against the rolling median.
||||||| e33ba3c

## [v0.0.30] - 2026-09-02

### Changed

- **README accuracy and ecosystem coverage.** The ecosystem section
  now lists all six crates including the new `noyalib-serde-yaml`
  drop-in (with its one-line migration snippet), states the
  own-repos + lockstep model instead of "five crates ship from this
  workspace", and adds the crate to the MSRV table. The install
  table sheds six channels that do not exist yet (Homebrew, AUR,
  Scoop, Nix, VS Code Marketplace, Open VSX are distribution-phase
  work; npm and GHCR were verified real and stay), and fixes a dead
  monorepo source-install path. "Capabilities in 0.0.1" is retitled
  to "Capabilities at a glance" with its stale shim description
  updated to the v0.0.29 behavioural contract. The release version
  gate now also checks the drop-in snippet's `=0.0.X` pin.

### Added

- **Two regression gates in per-push CI.** `fuzz-regression` builds
  all twelve fuzz targets and replays the seed corpus plus
  `fuzz/regressions/` — a new tracked corpus of fifteen minimized
  crash inputs from previously-fixed fuzz findings — with `-runs=0`,
  so a fixed crash staying fixed is now checked on every push
  instead of at the weekly soak. `each-feature` runs
  `cargo hack check --each-feature`, catching a single broken
  feature at PR time instead of at the weekly powerset run.
- One deliberate `fuzz_diff` divergence promoted to a unit pin:
  `&a:` is an anchor named `a:` on a null document (YAML 1.2
  §6.9.2 allows `:` in anchor names); serde_yaml_ng reads a mapping
  with an empty key. Pinned in `tests/competitor_bugs.rs`.

### Fixed

- **`Spanned<T>` and error locations for tagged/anchored nodes now
  anchor at the node's properties** (`!tag` / `&anchor`), not at the
  content — a node's span includes its properties, matching
  serde_yaml/libyaml marks. This closes the final partial in the
  18-case serde_yaml contract: `custom-explicit-tag` now reports
  `1:8:7` / Display column 8 exactly as upstream, the pin
  Takazudo/zudo-front-builder#2755 names as its re-evaluation
  trigger. CST reads and edits are unaffected: `resolve_span`
  strips leading property tokens back to the content, so
  `set_value` never splices over an anchor and `span_at` semantics
  are unchanged. Two of zfb's protected assertions are also ported
  as contract pins: the EOF one-past-the-flow-sequence location
  convention (single- and multi-line) and the column-counts-
  characters / index-counts-bytes contract their UTF-16 conversion
  depends on.

- `fuzz_no_span_loader` now encodes the documented v0.0.29
  asymmetry from #351: `from_str` refuses a multi-document stream
  while `cst::parse_document` reads the first document — that
  refusal is exempt from its loader-parity rule (the seed corpus
  was tripping the stale invariant).

### Removed

- **The `robotics` module and feature**, completing the one-release
  deprecation cycle started in v0.0.29. `StrictFloat` /
  `StrictFloatError` live on as
  `lossless_float::{LosslessFloat, LosslessFloatError}` (feature
  `lossless-float`); the `Degrees` / `Radians` unit newtypes leave
  the crate — they have no dependence on noyalib, and
  `examples/scientific.rs` shows the copy-into-your-own-code
  migration. The behavioural pins from the robotics test suites are
  ported to `lossless-float` tests, and the
  `robotics_polymorphism` example is gone with the module.

## [v0.0.29] - 2026-08-31

Thirteen fixes and seven additions, spanning all three pillars — the
serde deserializer, the emitter, and the CST editors — closing every
issue open at the start of the cycle (#327–#355). One documented
behavioural change: writes inside aliased anchors now refuse
uniformly (ADR-0011). **The headline: `compat-serde-yaml` became a
behavioural shim** — drop-in now means behaviour, not just names.

### Added

- **`compat-serde-yaml` is a behavioural shim, pinned by the
  18-case `serde_yaml` contract suite.** The shim's entry points
  now parse under `ParserConfig::serde_yaml_compat()` and its
  `Error` (a newtype since this release, boxed like upstream's)
  renders `serde_yaml` 0.9's wording and locations. Concretely:
  `<<` merge keys stay literal entries with resolved alias values;
  `0123` is a string and `0b11` is 3; a literal `1e999` stays a
  string; `u64::MAX` keeps full precision on the `serde_json` path
  and one past it refuses with `u64_over: JSON number out of range
  at line 1 column 11`; `[a, b]:` refuses with `invalid type:
  sequence, expected a string key` at `1:1:0`; transitive alias
  expansion is budgeted exactly as upstream (`repetition limit
  exceeded`, jumps ≤ events × 100 — the rule read out of upstream's
  own source); parse errors adopt libyaml's phrasing for the
  recognisable classes and its end-of-input location convention
  (line after the last, column 1). The engine behind it: six new
  `ParserConfig` knobs (`leading_zero_integer_strings`,
  `legacy_binary_numbers`, `float_overflow_strings`,
  `integer_overflow_errors`, `non_scalar_key_policy` with the new
  `NonScalarKeyPolicy`, `alias_jump_event_factor`) and two new
  error variants (`Error::IntegerOverflow` carrying the field
  path, `Error::NonScalarKey`), all usable outside the shim.
  `tests/serde_yaml_contract.rs` vendors the evaluation corpus a
  real migration candidate built
  (Takazudo/zudo-front-builder#2787, where noyalib 0.0.28 diverged
  on 11 of 18 and was rejected) with expectations captured live
  from `serde_yaml 0.9.34` — 18/18 pass; the one documented
  partial is the custom-tag refusal anchoring its location at the
  value rather than the tag. A `noyalib-serde-yaml` satellite
  (Cargo package-rename drop-in: zero source changes) follows.

- **`CompiledSchema`: compile a JSON Schema once, validate many**
  (#329, ADR-0008). `CompiledSchema::compile(&schema)?.validate(&v)`
  front-loads the schema compile that `validate_against_schema`
  repeats per call; the builder opts in to `format` assertion
  (annotation-only by default under Draft 2020-12) and registers
  custom formats; `iter_errors` returns structured violations with
  the instance path and offending keyword. `validate_against_schema`
  is now compile-then-validate through the same type, so the
  external-`$ref` and recursion hardening covers both paths.

- **`set_value` accepts collections, in the target node's style**
  (#328, ADR-0010). `tags: [a, b]` set to `[a, c]` stays flow; a block
  sequence stays block at its own column; nested mixed shapes render
  through the serializer at the document's `indent_unit()`. The
  candidate is parsed and compared against the expected typed value
  before any byte moves. Replacing a *scalar* with a collection stays
  refused — that layout decision belongs to `set`.

- **Flow and empty collections in the insertion mutators, and flow
  renames** (#338, ADR-0011). `insert_entry_value` splices
  `, key: value` into single-line flow mappings (`{a: 1}`, `{}`, a
  root-level `{…}` document); `push_back_value` and
  `insert_after_value` do the same for flow sequences (`[]` included);
  `rename_key` renames flow-mapping keys, double-quoting a new key
  whose plain spelling would read as flow structure. Members render
  flow-safe (`b, c` and multi-line strings double-quote). Multi-line
  flow collections still refuse, byte-identically.

- **One policy for writes inside anchored nodes** (#338, ADR-0011).
  All mutators now refuse a write into a value that live `*name`
  sites share, naming the anchor and pointing at
  `materialise_aliases_of` — `set_value` and `remove` included, which
  previously edited every alias and merge site silently. An
  equal-value `set_value` stays a byte no-op. **Behavioural change**:
  callers relying on the silent propagation must edit the anchor
  deliberately or materialise the aliases first.

- **`Document::set_path`: parent-creating writes in the CST editor**
  (#327, ADR-0009). `doc.set_path("menu.visible", &true.into())` on
  `title: x` creates the missing `menu:` level on the way; an empty
  document (comments, blank lines, or a bare `---` only) receives its
  first key with the header preserved. Missing levels indent at the
  document's `indent_unit()`, quoting stays with `Emit`, and every
  byte goes through the existing oracle-guarded mutators. A
  single-line flow ancestor creates its missing levels as flow
  members (#338); an existing segment that resolves to a scalar, a
  non-root null, a multi-line flow ancestor, or a missing sequence
  index refuses cleanly with the source byte-identical.

- **`SerializerConfig::prefer_single_quotes`** (#361, #352). Opt-in: strings
  that must be quoted but need no escapes are written `'like this'`
  instead of `"like this"`. Strings containing characters that only
  double quotes can spell (line breaks, non-printables) still get
  double quotes, unchanged by the flag.

- **`ParserConfig::plain_scalar_strings`** (#359, #344, ADR-0006). Opt-in:
  a `String`/`char` target reading a plain scalar receives the literal
  text (`no` stays `"no"`, `1.0` stays `"1.0"`) instead of a type
  error, matching what most other YAML libraries do.

- **Weekly feature-powerset sweep and a real SBOM.** A scheduled
  `cargo hack --feature-powerset --depth 2` workflow checks every
  feature pair (309 combinations; `nightly-simd` and the bench-only
  `compare-saphyr` excluded), and the release pipeline now emits a
  CycloneDX 1.5 `SBOM.cdx.json` — attested (SLSA), sigstore-signed,
  and attached to the GitHub Release alongside the human-readable
  `SBOM.txt`, which was never a machine-readable SBOM format.

- **OSS-Fuzz integration and a YAML token dictionary.** The project
  definition (`fuzz/oss-fuzz/`: project.yaml, Dockerfile, build.sh)
  ships in-tree, verified end-to-end against the real
  `base-builder-rust` image (`infra/helper.py build_fuzzers` with
  local source: all 11 targets compiled under ASAN with corpora and
  dictionaries staged). Submission to `google/oss-fuzz` is a
  maintainer PR — see `fuzz/oss-fuzz/README.md`. `fuzz/yaml.dict`
  gives libFuzzer the scanner's structural tokens whole; with it,
  local sweeps found every round-trip bug below within minutes.

### Fixed

- **Round-trip integrity: seven emit/parse bugs found by fuzzing
  with the new dictionary.** Every one produced output that parsed
  to something else or not at all:
  - verbatim tags accepted control characters (and line breaks, and
    the empty `!<>`); shorthand tag suffixes accepted control
    characters and the non-URI `>` — all rejected now, and a tag
    body *no* spelling can carry (controls; `>`) is emitted as the
    quoted-key single-entry mapping it is indistinguishable from in
    the serde data model;
  - tags whose body holds shorthand-unsafe characters (`,`, flow
    indicators, an interior `!`, blanks) were emitted raw and split
    at the first such byte — the verbatim `!<...>` form is used;
  - a mid-document BOM was read as plain-scalar content and emitted
    back unquoted, where re-parse stream-skips it and reinterprets
    the scalar as markup — the scanner now rejects BOMs after the
    stream start (§5.2), and both emitters force-quote strings
    containing one (`﻿`);
  - raw control characters (NUL, DEL, …) were accepted as plain and
    single-quoted scalar content (`a: b\0c` parsed) — rejected per
    §5.1 c-printable, and both emitters now escape DEL and the
    sub-0x20 range so their own output stays parseable;
  - a multi-line string used as a mapping *key* was emitted as a
    `|-` block — not grammar in key position at all — and now emits
    double-quoted;
  - a scalar starting with `...` was emitted plain at column 0,
    where it reads back as the document-end marker — quoted now,
    like the `-`-leading family already was.
  The `fuzz_diff` oracle also learned the verified ecosystem
  divergences (serde_yaml_ng's 1.1-flavoured leading-zero and
  signed-radix integers, merge-key literalism, block-scalar comment
  stripping and chomping), with noyalib's spec-correct readings
  pinned in `tests/competitor_bugs.rs`. Known-open at cut: minor
  divergence classes still surface on long fuzz runs (e.g. a bare
  `:`-shaped document, `null` vs `{"": null}`) — continuous
  triage is what the OSS-Fuzz onboarding is for.

- **Three more spec-strictness gaps, found by the new
  `fuzz_serde_yaml_compat` parity fuzzer** (the shim vs the real
  archived `serde_yaml 0.9.34`, value-and-verdict differential):
  block scalar *content* and raw double-quoted runs accepted
  control characters (§5.1 c-printable now enforced uniformly —
  escapes remain the way to carry controls); a block scalar header
  accepted trailing content on its own line (`>-\n` read a literal
  `\n` as content — §8.1.1 allows only blanks and a comment
  there); and the reserved indicators `@` and `` ` `` could start a
  plain scalar (§5.10 forbids exactly that). The fuzzer's
  documented-divergence allowlist records where the spec and
  libyaml legitimately part ways (anchor-name charsets, empty
  implicit keys, tags/directives/explicit keys).

- **`simd::parse_decimal_u64` / `parse_decimal_i64` could accept a
  non-digit block** — found by the new Kani proof harness on its
  first run, as a concrete counterexample. The SWAR validator's
  whole-register subtract/add propagated carries *between byte
  lanes*, so an 8-byte block mixing bytes below `'0'` with bytes
  above `'9'` (e.g. `[0x07, '+', '0', '9', '.', 0x07, 1, 1]`)
  cancelled its own evidence and parsed as `Some(…)` — violating
  the documented "malformed input never produces a garbage answer"
  contract for direct callers of the public functions. The YAML
  parser itself was not affected: its only internal call site
  pre-validates every byte with `is_ascii_digit()` first. The
  validation is now two carry-free nibble checks, and the
  `#[cfg(kani)]` harnesses in `simd.rs` prove, against a naive
  per-byte reference: the 8-digit fold exhaustively over all 2^64
  blocks; value equivalence on all-digit slices up to one chunk;
  rejection of any non-digit anywhere in chunk-plus-tail; and sign
  handling for the signed wrapper. (An earlier revision of this
  entry claimed 20/21-byte equivalence proofs; the accumulator
  multiplies past one chunk exceed any CI-sane solver budget, so
  multi-chunk composition and the overflow boundaries stay with
  the unit tests and proptest — they are `checked_mul` arithmetic,
  not SWAR.) The `kani-proofs` workflow re-runs all four proofs
  weekly under kissat, ~2.5 minutes of solving total.

- **Five features did not compile without `std`** — found by the
  powerset sweep's first run. `ariadne`, `robotics`, `include`,
  `schema`, and `validate-schema` alone (or paired with each other /
  `fast-float`) failed on missing prelude imports or deny-level
  qualification lints; all five now build `no_std`+alloc. `miette`
  now *implies* `std` in the feature graph — miette 7's `Diagnostic`
  supertrait is `std::error::Error`, so the combination never
  compiled and no consumer could depend on it. Behavioural note: a
  multi-document `!include` file is now rejected instead of silently
  truncated to its first document (the include path parses through
  the every-target checked entry point, matching `from_str`'s
  single-document policy, #351).

- **A GPG-less release could not publish.** The release workflow's
  asset list relied on `nullglob` to drop the `.asc` entries when
  GPG signing is skipped, but `artifacts/SBOM.txt.asc` was a literal
  path — `nullglob` only removes unmatched *patterns* — so
  `gh release create` failed on the missing file for any fork
  without the signing key. The entries are spelled as real globs
  now.

- **CST: the verbatim inserters are containment-guarded** (#221
  sub-ask 5, completing what the structural oracle started in
  v0.0.21). `insert_entry`'s new-key splice ran with no oracle at
  all: a lone `U+000D` in the fragment — a YAML line break the
  `\n`-only branch test never saw — escaped into sibling territory,
  and a key was spliced verbatim. The key half is now a *name*:
  `<<` and non-printables refuse, a non-plain-safe spelling is
  quoted automatically (as `rename_key` documents), and the
  existing-key check reads the mapping's own entries, so
  `insert_entry("m", "a.b", …)` adds the literal `a.b` key instead
  of resolving `m.a.b` through the path syntax and overwriting a
  nested entry. The `guarded_insert` oracle additionally pins the
  container's growth to exactly the one entry asked for —
  `push_back("s", "v\n  - w")` appended two items entirely inside
  the container, where the outside-shape check could not see them —
  covering `insert_entry`, `push_back` and `insert_after`.

- **Typed rejections from `from_str` now carry the source location**
  (#356, #330). The streaming fast-path raises serde's own wording but
  cannot say where; the AST path knows where but words it differently.
  A failed streaming parse is now re-run through the span-aware path
  and the caller sees the streaming message with the AST location.

- **…and the field path** (#353). A rejection inside a nested value
  prefixes its message with the path of the field it is about —
  `server.port: invalid type: string "x", expected u16` — the way
  `serde_yaml` reports it. Sequence indices are bracketed
  (`a.groups[1].count`). Derived on the error path only, by walking
  the parsed document once; errors at the root and errors from
  `from_value` are unchanged, and `location()` is unaffected.

- **Value/deserializer parity** (#360; #348, #349, #350, #351): `Number`'s `Display` agrees
  with the serializer on floats; the null document deserializes into
  an empty map or struct; a tagged scalar keeps its tag when `Value`
  is reached through serde; and a multi-document stream is rejected
  by the single-document entry points instead of silently returning
  the first document.

- **Emitter round-trip fixes** (#357; #345, #346, #347, #354, #355): plain scalars at dash and tab
  boundaries are quoted; space-leading block scalars carry an
  indentation indicator; `|+` block scalars no longer gain a newline
  every round trip; `Value::Tagged` is emitted as a tagged value, not
  a map keyed by the tag; `compact_list_indent` applies at every
  depth; and a colon or hash is quoted only where YAML gives it
  meaning.

- **Digit-leading strings are written plain when they read back as
  strings** (#358, #339): `2026-12-31`, `1.2.3` and `3rd` no longer
  acquire quotes they never needed; anything that would read back as
  a number (`42`, `1e3`, `007`, `0x1F`) stays quoted.

- **CR, NEL, LS and PS force double-quoted style** (#362, #335), in
  `to_string` and the CST writers both, spelled with their named
  escapes (`\r`, `\N`, `\L`, `\P`). Any other style either normalises
  them into ordinary line breaks or emits raw bytes that read back as
  line breaks.

- **CST: `set_value` with an equal value leaves the bytes alone**
  (#363, #337). A no-op write no longer reformats the scalar it did not
  change.

- **CST: `remove` handles entries that share their line with a
  sequence dash** (`- name: x`), and implicit-null items (#364,
  #336).

- **CST: `remove` of a merge-provided key refuses with an error
  instead of panicking** (#340, #334). A key supplied by `<<:` owns no
  bytes in the mapping it lands in.

- **CST: `set_value` spells strings for flow context** (#343, #332).
  Inside `[…]` / `{…}`, characters like `,` and `]` are structural
  anywhere in a plain scalar, so replacement values are quoted under
  the flow rules and flow edits re-parse the whole collection.

- **CST: a new block literal keeps a trailing comment out of the
  scalar** (#342, #333). Lifting `key: x  # note` into a multi-line literal
  no longer swallows `# note` into the block's content.

- **Scanner: a block scalar indicator inside a flow collection is
  rejected** (#341, #331). `[|`, `{>` and friends are not valid YAML; they
  now fail to parse instead of producing a surprising document.

No breaking API change: both new flags are opt-in and default off.
No MSRV change (still 1.86.0).

### Changed

- **`robotics` is deprecated; `StrictFloat` is now
  `lossless_float::LosslessFloat`** (feature `lossless-float`). The
  refuse-to-lose-precision float was never robotics-specific — it is
  the floating-point sibling of `lossless-u64`, and its new home
  needs nothing beyond the mandatory `serde_core` (the old one
  pulled `dep:serde` for derives). The `robotics` module and feature
  survive one release as a deprecated compat surface (`robotics`
  implies `lossless-float`; `StrictFloat`/`StrictFloatError` are
  deprecated aliases), then go — along with the `Degrees`/`Radians`
  unit newtypes, which are ~40 lines of domain code with no
  dependence on noyalib and belong in the consumer's own tree.

## [v0.0.28] - 2026-08-23

Two CST and scanner correctness fixes, both about an *implicit null* —
a mapping entry whose value is absent. One could not be written to; the
other was not recognised at end of input.

### Fixed

- **Inserting over an implicit null appended a duplicate key** (#310,
  fixed in #311). `a:` followed by an insertion emitted a second `a`
  entry rather than filling the empty one. The load-back oracle could
  not see it: the loader resolves duplicates last-wins, so the document
  round-tripped to the right value while the *bytes* carried a
  duplicate. It also relocated the entry to the end of the mapping,
  which stranded any trailing comment on the key it had just shadowed.

- **A `:` at end of input was not read as a mapping indicator** (#312,
  fixed in #313). `a:` and `a:\n` are the same document one byte apart
  — the trailing newline is not content — yet the first loaded as the
  scalar `"a:"` and the second as `{a: null}`. The plain-scalar scanner
  substituted a NUL for the absent byte after the colon, and NUL is not
  in `IS_BLANK_OR_BREAK`, so the scalar swallowed the colon instead of
  stopping at it.

  Four faces, all now matching PyYAML and Psych:

  | Input | Before | After |
  |---|---|---|
  | `"a:"` | `String("a:")` | `{a: null}` |
  | `"a: 1\nb:"` | **parse error** | `{a: 1, b: null}` |
  | `"p:\n  a:"` | `{p: String("a:")}` | `{p: {a: null}}` |
  | `"- a:"` | `[String("a:")]` | `[{a: null}]` |

  The second was a hard parse error on valid YAML and is the one most
  likely to be hit — it needs only a mapping with a blank last value and
  no trailing newline, which `printf` without `\n`, heredocs and
  generated fragments all produce. The other three were silent wrong
  values.

No public API change. No MSRV change (still 1.86.0).

## [v0.0.27] - 2026-08-21

Two correctness fixes in alias and merge-key handling, both found by
consumers pointing a real workload at a published release.

### Fixed

- **An alias used as a value beside a merge key came back unresolved**
  (#301, reported and diagnosed by
  [@mathstuf](https://github.com/mathstuf), fixed in #304).

  ```yaml
  base: &b   { x: 1, y: 1 }
  other: &other 2
  overridden:
    <<: *b
    y: *other        # deserialised as the string "other", not 2
  ```

  `peek_event` and `next_event` each read from either the replay stack or
  the parser, and resolved aliases on the **parser branch only** — then
  labelled both results processed. An alias arriving through replay was
  therefore stored as though it were fully processed while still being an
  `Event::Alias`. The replay stack only exists once a merge has injected
  something, which is why the same document works with the alias written
  above the `<<:` line and fails below it.

  Alias resolution now runs whichever branch the event came from, in one
  `process_event`; `anchor_and_record` is the single place doing anchor
  bookkeeping; and the lookahead slot is typed (`Lookahead::{Raw,
  Processed}`) so its two consumers stop inferring which kind they hold,
  in opposite directions. Inline alias resolutions 3 -> 1, anchor/record
  pairs 3 -> 1.

- **Only a plain `<<` scalar is a merge key.** The YAML merge type gives
  `tag:yaml.org,2002:merge` to a **plain** `<<`; a quoted `"<<"`, and an
  alias resolving to the string `<<`, both resolve to
  `tag:yaml.org,2002:str`. Both were being read as merge instructions.

  By the time a key reaches the mapping arms it is a
  `Value::String("<<")` however it was written, so eligibility is now
  decided at each scalar-resolution site and carried in the frame.
  `loader.rs` holds two complete loaders, each with its own frame enum
  and its own pair of merge checks — four sites in all. The first attempt
  patched one and changed nothing observable.

  `Value::apply_merge` remains style-blind by nature and is documented as
  such: it operates on an already-built `Value`, where presentation does
  not exist.

  > **Behaviour change.** A document where a quoted `"<<"` or an alias to
  > `<<` currently triggers a merge will stop merging and gain a literal
  > `<<` key instead. This is spec-correct, but it is **silent** — no
  > error is raised. If you rely on either spelling, quote-strip it to a
  > plain `<<` before upgrading.

### Added

- `merge_keys_with_aliases` example — merge keys and aliases in one
  mapping, and which spellings of `<<` are merges.
- Benchmarks for `<<:` expansion (`merge_key_single_anchor`,
  `merge_key_sequence_of_anchors`, `merge_key_quoted_is_ordinary`, and a
  `merge_key_absent_baseline`). The existing `merge_small` /
  `merge_nested` / `merge_concat` time `Value::merge()`, an API method on
  an already-parsed value; nothing measured merge-key expansion during
  parsing.
- 63 tests across three tiers: 13 unit, 35 integration (the merge-key
  matrix asserts every case on **both** the streaming and AST paths), and
  8 regression, plus a differential oracle comparing `from_str::<Value>`
  against `load_all`, which shares no lookahead code.

### Changed

- `doc/ECOSYSTEM.md` and `doc/scorecard.json` regenerated against this
  release. The previous scorecard's `audit_vulnerabilities` rows were
  never actually measured — `cargo audit` was exiting 101 under a
  shadowing shell alias and the probe's fallback read that as zero
  advisories. That probe was fixed in v0.0.26; this is the first
  scorecard where those rows are earned.

## [v0.0.26] - 2026-08-20

### Fixed

- **`remove` left a whitespace-only line in a wrapped flow collection**
  (#294, reported and fixed by [@zoosky](https://github.com/zoosky),
  PR #296). A flow collection written one member per line —

  ```yaml
  ports: [
    80,
    443,
  ]
  ```

  — lost the member but kept its indentation, so `remove("ports[0]")`
  wrote `  ` onto a line that had held content. The value round-tripped
  unchanged, so this was never corruption; it was trailing whitespace in
  a patch, which `git diff --check`, `yamllint`'s `trailing-spaces` and
  `editorconfig-checker` all reject. A library whose promise is that an
  edit touches only what the path names should not hand its caller a diff
  their own lint refuses.

  `flow_member_range` took the member and exactly one separator — right
  for `{x: 1, y: 2}`, and all a single-line collection ever needs — but
  nothing then asked whether the line had anything left on it. The block
  path had always answered that same question the other way:
  `owned_entry_range` takes the entry's whole line, indentation included.
  Same operation, same shape, opposite answers.

  The member now takes its whole line when — and only when — it is alone
  on it. The condition is "alone on its line", not "the collection is
  wrapped", so an opening indicator, a sibling member, a trailing comment
  or the closing indicator all keep the line standing and leave those
  outputs byte-identical.

  Unreachable before #285: a wrapped flow collection did not parse, so
  nothing downstream of the scanner had ever seen one. That is the second
  time in two releases that fixing a parse refusal exposed a defect it
  had been hiding.

  The fix is `absorb_emptied_line`, named for the existing
  `absorb_head_comments` it sits beside. Its doc comment draws one
  boundary worth repeating: a comment left on the line keeps the line,
  because what a comment stranded by a removal *means* is the caller's
  question, not something a whitespace rule should decide.

  42 tests added — 13 from @zoosky's PR, plus 18 integration and 11 unit
  tests here — and the `cst_wrapped_flow_edit` example. The unit tests
  were written against an independent implementation of the same fix and
  pass unmodified against this one, which is the closest thing to a
  second opinion a single codebase gets.

- **The ecosystem scorecard scored a security probe it had not run.**
  `cargo audit --json` exits 101 where a user-defined `audit = "audit"`
  alias in `~/.cargo/config.toml` shadows the subcommand and recurses.
  The probe's fallback counted `RUSTSEC` occurrences in whatever landed on
  stdout, so an error message yielded "0 advisories" and a clean pass —
  the harness's own "no credit for unmeasured work" rule violated in the
  place it matters most. It now invokes `cargo-audit` directly and treats
  unparsable output as N/A. Exit status alone cannot decide this, because
  `cargo-audit` also exits non-zero when it genuinely finds advisories;
  the discriminator is whether `.vulnerabilities.count` parses. The real
  count for this release is 0 across 272 crates, confirmed by running the
  binary directly.

## [v0.0.25] - 2026-08-19

**Four fixes from [@zoosky](https://github.com/zoosky)**, all found while
adopting v0.0.24 in [yqr](https://github.com/zoosky/yqr), and all cases
where the previous behaviour produced or refused something this codebase
already disagreed with elsewhere.

Each arrived as a reproduction against a published version, a diagnosis
naming the responsible function, a fix, and tests — including the cases
that had to keep failing. Three of the four were found by pointing a real
consumer at a release and reporting what broke, which is the kind of
testing a library cannot do for itself.

### Fixed

- **`remove` wrote an empty collection at its key's own column** (#283,
  PR #284). A block sequence may sit at its key's column — `on:` /
  `- push` is what nearly every GitHub Actions and Ansible file looks
  like. What replaces it may not: `{}` / `[]` is a block *mapping value*,
  and one sharing its key's column does not re-parse as that key's value.

  ```yaml
  on:            ->   on:            # before: Ok(()), and unreadable
  - push              []
  jobs: {}            jobs: {}
  ```

  The inconsistency was visible from inside: delete the `jobs:` line and
  the identical removal was *refused* by the oracle, because the guard
  re-parses with this parser, which accepts `on:\n[]\njobs: {}` and
  rejects `on:\n[]`. Same shape, same output spelling — `Ok` with a
  sibling, refused without one.

  `sole_entry_range` took the indent from the removed entry's own line,
  which for this layout *is* the key's column. The constraint is
  "strictly deeper than the key", and the two coincide for every layout
  except this one. The parent key's offset is now threaded down and the
  indent clamped to the key's column + 2 when the entry's own indent does
  not already clear it. A root collection, or one reached through a
  sequence item, has no parent key and is unchanged.

  The head-comment run from #280 is still absorbed at the entry's **own**
  column, where those comment lines actually sit — only the replacement
  moves.

- **A wrapped flow collection was refused when its closing indicator sat
  at the parent's column** (#285, PR #286):

  ```yaml
  ports: [
    80,
    443,
  ]
  ```

  A *read* refusal, so nothing downstream ran. The indentation check
  exists so that flow content continuing across a line break cannot be
  ambiguous with sibling block content (yaml-test-suite 9C9N) — but that
  rationale is about **content**. A line whose first character is `]` or
  `}` cannot begin block content, so there is nothing to be ambiguous
  with; the rule was reaching the terminator too.

  The asymmetry was already in the tree: the same closer at column 0 is
  accepted at the root, where `self.indent` is `-1`. Only a flow inside a
  block mapping refused it.

  Under-indented flow **content** stays refused, deliberately — that is
  9C9N's rule and this does not touch it. `ports: [` / `80,` / `]` is
  still an error, as is 9C9N itself, whose third line opens with a scalar
  rather than the indicator.

- **A new key could not be inserted into a mapping whose keys contain a
  `.`, `[` or `*`, and the refusal blamed a `<<` merge that was not
  there** (#288, PR #289).

  ```yaml
  labels:
    app.kubernetes.io/name: web
    app.kubernetes.io/component: frontend
  ```

  `insert_entry("labels", "tier", "frontend")` refused. That is the
  standard Kubernetes label convention, so the shape is everywhere.

  Two sites took the last key from the **typed** view, composed it back
  into a path *string*, and re-parsed it — `mapping_insert_anchor`, and
  `insert_entry`, which duplicated the logic inline. `parse_query_path`
  splits on `.`, `[` and `*` unconditionally, so no such key survives the
  round trip and every entry looked span-less. With no anchor left, the
  only error the function knew about fired: the merge one.

  Two defects nested — a path round trip that no such key survives, and a
  diagnostic asserting a cause rather than reporting an observation.

  The anchor never needed a path. `mapping_insert_anchor` now walks the
  span tree's entries directly, through a new `resolve_tree`, and
  `insert_entry` shares it instead of keeping its own copy. Three things
  fall out, each tested: keys holding `[`, `*` or quoted dots insert
  correctly; a mapping whose last entry is an implicit null anchors on
  that entry's key line, so a sibling lands **after** it rather than
  above it; and a mapping with both a `<<` merge and an entry of its own
  anchors on the entry. A merge-**only** mapping still refuses, leading
  with what was observed.

  Insert only — `set`, `remove`, `rename_key` and `swap_items` still
  address through `parse_query_path`, so a dotted key stays out of reach
  for them. Whether the path grammar should grow an escape form is a
  separate question.

- **An inserted scalar was quoted because some unrelated line was
  quoted** (#290). The dominance vote counted only quoted scalars against
  each other — plain ones did not vote — so a single quoted scalar
  anywhere decided the spelling of every later insertion:

  ```yaml
  quoted: "30"        # four lines away, untouched by the edit
  labels:
    app: web
    tier: "frontend"  # before — the sibling is plain
  ```

  On a Kubernetes manifest the vote was settled by `value: "30"` in a
  container's env block, arbitrarily far from the labels being edited.
  Nothing was *wrong* with the value — it round-trips and the document
  stays valid — but the diff a reviewer saw was a quoted value among
  plain ones. It also disagreed with `set`, which writes plain at the
  same site.

  `EmitCtx`'s doc already stated the intent — an implementation should
  "match the file it is landing in" — so the radius was wrong rather than
  the idea. Insertion now learns from the collection it lands in and only
  falls back to the document-wide vote when that collection has no scalar
  values to learn from.

  Two details decided by implementing it:

  - **Only values vote, not keys.** Counting scalar tokens across the
    site's byte range cannot tell one from the other, and mapping keys
    are almost always plain — `a: "one"` / `b: "two"` would tie two plain
    keys against two quoted values and pick plain, the opposite of what
    the site says. The entry values are read from the span tree instead.
  - **Plain needs a strict majority.** A tie means the site is genuinely
    mixed (`a: 1` beside `b: 'two'`), and there the quoting already
    present is the better guide. Every case #290 reports has plain
    winning outright.

  `Document::dominant_quote_style` is public, documented, and pinned by
  three doctests, so its behaviour is unchanged — this narrows what
  *insertion* asks, not what that function answers. Of the two options in
  the report this is the second, which leaves the public API alone.

- **`remove` refuses an alias-valued entry instead of silently doing
  nothing** (PR #292). **Behaviour change**: a call that previously
  returned `Ok(())` now returns an error.

  ```yaml
  a: &x 1
  b: *x     # remove("b") -> Ok(()), document unchanged
  ```

  An alias resolves *through* to its anchor, so the value span for `b`
  is the anchor's bytes on another line — before `b`'s own key. The
  range arithmetic degenerated to an empty splice, so the call removed
  nothing and reported success.

  Refusing is what `SpanTree::Alias`'s own documentation already
  prescribes: a write there would splice the anchor's bytes, which
  belong to a different key. The message names the entry and the reason.
  Removing the anchor's own entry still works, as does `replace_span`
  for callers who want the bytes gone deliberately.

  Present since at least v0.0.24 — reproduced identically on that tag —
  and surfaced by the corrected fuzz invariant below, which asserts that
  an accepted removal changed the source.

### Testing

- **The differential fuzz target's remove invariant was unsound** (PR
  #291). `fuzz_editors` asserted that an accepted `remove` shrinks the
  parsed value. It does not, under duplicate mapping keys: `Value`
  deduplicates and the last duplicate wins, so removing one of two `5:`
  entries deletes a line from the source while both parses still show
  three keys. A correct edit failed the assertion and `fuzz-diff` went
  red on `main`.

  Asserting the *source* gets shorter is also wrong — removing the only
  entry rewrites `"::\n"` to `"{}\n"`, which is correct and exactly as
  long. What holds is that the source changed, and that is what the
  target now asserts; the node count is kept only in the non-strict
  direction, which still catches a removal that takes a parent with it.

  `remove` itself was correct throughout. Both behaviours are pinned in
  `tests/cst_remove_fuzz_regressions.rs` so they run under the normal
  suite rather than only under a nightly fuzz job.

### Changed

- Per-release notes moved from the repository root to
  [`doc/release-notes/`](doc/release-notes/README.md), renamed to match
  their tags exactly (`doc/release-notes/v0.0.17.md` documents
  `v0.0.17`). Moved with `git mv`, so history follows. Links that
  pointed at the old root paths were updated; deep links to
  `RELEASE-NOTES-v0.0.N.md` on `main` will need adjusting.

- Version → 0.0.25.

## [v0.0.24] - 2026-08-18

### Fixed

- **`remove` stranded a sole entry's head comment** (#280, reported by
  **@zoosky**). The same comment on the same entry was taken when the
  entry had a sibling and left behind when it did not:

  ```yaml
  # before, with a sibling            # before, as the last entry
  a:                                  a:
    # documents x                       # documents x
    x: 1                                x: 1
    y: 2                              b: 2

  # after: comment removed with x     # after: comment stranded above {}
  a:                                  a:
    y: 2                                # documents x
                                        {}
                                      b: 2
  ```

  The two arms derived their range from different things.
  `Removal::Line` goes through `owned_entry_range`, which calls
  `absorb_head_comments` and so owns the contiguous same-indent comment
  run above the entry. `Removal::SoleEntry` replaced the *collection's*
  span, and a collection starts at its first entry's **content** —
  below the comment — so the run was never absorbed. Returned `Ok`, and
  invisible to the typed oracle, because a comment is not in the typed
  value.

  Both paths now share `sole_entry_range`. Two consequences worth
  naming:

  - The splice can begin *above* the entry, so the entry's own leading
    whitespace falls inside the replaced range and is written back —
    otherwise `a:` loses its value entirely rather than gaining `{}`.
  - **Flow collections are excluded.** `a: {x: 1}` starts at the `{`
    part way along a line whose earlier bytes belong to the key. There
    is no head-comment run to own there, and those bytes are not
    indentation: treating them as such rewrote `a: {x: 1}` as `   {}`
    and lost the key. Caught by an existing test.

  Unchanged, and now pinned: a comment detached by a blank line, or at a
  different column, is not the entry's and stays put; an inline trailing
  comment sits inside the collection span and was always removed
  correctly.

### Changed

- Dependency updates rolled into this release, superseding #273, #274,
  #275, #276, #277 and #278:
  - `jsonschema` 0.49.6 → 0.49.9
  - `hashbrown` 0.15.5 → 0.17.1
  - `github/codeql-action/{analyze,init,upload-sarif}` 4.37.6 → 4.37.7
  - `taiki-e/install-action` 2.85.10 → 2.86.1

## [v0.0.23] - 2026-08-16

### Added

- **`Document::remove` now covers flow members and sole entries**
  (#221, sub-ask 4 — completes the issue). Both classes previously
  refused. The refusals were correct at the time: v0.0.21 turned them
  from *silent data loss* into errors, because a flow member shares its
  line with its siblings **and its parent**, so "delete the line"
  deleted the parent — and for a one-entry document, the document.

  - **Flow members.** `a: {x: 1, y: 2}` → `remove("a.x")` → `a: {y: 2}`;
    `a: [1, 2, 3]` → `remove("a[1]")` → `a: [1, 3]`. The member's own
    span is spliced along with exactly one separator: the comma *after*
    it, or — for the last member — the comma *before* it, so neither
    `{, y: 2}` nor `{x: 1, }` can result. A separator parked on another
    line is deliberately not matched; a multi-line flow collection
    refuses rather than splicing bytes it cannot account for.

  - **Sole entries.** Removing the last entry of a collection now writes
    that collection out explicitly — `a:\n  x: 1` becomes `a:\n  {}`,
    a sole sequence item leaves `[]`, and a single-key document becomes
    `{}`. Deleting the bytes alone would leave a dangling `a:`, which
    re-parses as **null**: a type change, not a removal.

  The document's trailing newline survives the rewrite. A collection's
  span can run to the end of its last line, so overwriting that range
  wholesale would take the final newline with it — harmless to a parser,
  but this is a lossless CST, where a vanished trailing newline is a
  whole-file diff and a failing end-of-file check.

  `remove_subtree` was **not** added. The issue offered it as an
  alternative to extending `remove`, and extending `remove` is the path
  that was taken, so a second entry point would be a synonym rather than
  a capability.

### Changed

- **`swap_items` / `move_item` exchange whole entries, not value bytes**
  (#269). An item now moves with the lines it owns — its head-comment
  run included — which is the same range `remove` deletes
  (`owned_entry_range`). Previously only the two value spans moved, so
  every comment stayed with the *slot*: swapping `- one  # first` and
  `- two  # second` produced `- two  # first`, and a `# about one`
  header stayed above index 0 while the item it described moved away.
  At `Ok`, and invisible to the typed oracle, which compares values and
  cannot see a comment.

  This is a **behaviour change, not a bug fix**: the old semantics were
  deliberate and tested (`swap_preserves_inline_comment_position`, "the
  comment annotates the slot"). What changed the call is that `remove`
  decides the same question the other way for the same bytes, and two
  mutators in one crate holding opposite views of who owns a comment is
  the harder thing to defend. That test is rewritten, with the reasoning
  in the test body.

  Falling out of the same change: multi-line and differently-indented
  items now exchange whole entries rather than values, so an item's own
  indentation travels with it. A **flow** sequence keeps the value-span
  exchange — its members have no lines of their own.

  Fixes three doc-comment claims that never held: `swap_items`
  documented refusals for flow sequences, multi-line items and
  differently-indented items, none of which fired.
- Tests that pinned the old refusals now pin the completed behaviour.
  The data-loss suite (`remove_flow_data_loss.rs`) keeps its original
  property — the parent, the siblings and the rest of the document must
  survive — and asserts the exact narrowed output instead of an error,
  so a return of the v0.0.21 bug still fails it just as loudly.

## [v0.0.22] - 2026-08-13

### Fixed

- **Splices adopt the document's own line break instead of assuming `\n`**
  (#261, thanks **@zoosky**) — an edit that added a line wrote a hard-coded
  `\n` whatever the document used. The mutators already derived a splice's
  *indentation* from the site; the terminator was the one thing still
  assumed. Affected `insert_entry` / `push_back` / `insert_after` and their
  `_value` counterparts, `set_leading_comment`, and
  `set_comment(Before | Inline)`.

  The inline case was the worst of them. It spliced at `line_end_from` —
  the index of the `\n` — which inside a `\r\n` lands **between the two**
  and stranded a lone `\r`. `set_inline_comment` was already correct, since
  it splices at the node's span end, so the two APIs for the same operation
  disagreed. They now agree, and a test pins that.

  `document_break` (and `comment_line_break`, its counterpart in
  `annotated.rs`) reports `"\r\n"` only when the document is **wholly**
  CRLF — at least one break, and every `\n` preceded by a `\r`. So it reads
  the document's convention rather than guessing one:

  - `leading_break_for_splice` returns it instead of `"\n"`
  - `indent_continuation_lines` takes it, so a multi-line emission grows
    CRLF on *every* line rather than only the last
  - the inline-comment splice moves to a new `line_break_start`, which
    steps back over a `\r`

  Deliberate non-changes, each with a test: a **mixed-ending** document
  keeps the `\n` default (there is no convention to honour, and picking one
  would rewrite bytes the caller did not ask about); a document with **no
  break at all** likewise; a document whose **last line is unterminated**
  still reads the convention from the breaks it does have. `set_value` and
  `remove` never added a line and are pinned as controls.

  No data was lost by the old behaviour — values round-tripped, and the
  inline case stayed valid because YAML 1.2 accepts a lone `\r` as a break.
  What a caller got was a file returning with two or three terminators in
  it, which for a lossless CST shows up as a whole-file diff on Windows, or
  a `.gitattributes` / CI line-ending check firing.

  No public API changes shape: `document_break` and `comment_line_break`
  are private, and `indent_continuation_lines` is private and gained a
  parameter.

  Cross-checked against a real consumer: yqr carries a local workaround
  that restores the convention at emit time. With that workaround
  **disabled** and yqr pointed at this change, its full suite passes
  (163/163) including five CRLF regression tests; against unpatched 0.0.21
  with the workaround disabled, three of those fail on exactly this
  property. The change subsumes the workaround.

### Changed

- Install snippets across `README.md`, `crates/noyalib/README.md`,
  `MIGRATION.md`, `GETTING_STARTED.md`, `doc/USER-GUIDE.md` and
  `doc/pre-commit.md` now read `0.0.22`. They had been left on `0.0.18` —
  never bumped for 0.0.19, 0.0.20 or 0.0.21 — so this clears three
  releases of drift.

## [v0.0.21] - 2026-08-12

Includes the work prepared as **0.0.20**, which was merged to `main` but
never tagged and so never reached crates.io. The published sequence goes
0.0.19 → 0.0.21.

### Added

- **Comment mutation on the CST** (#221, sub-ask 1):

  ```rust
  pub enum CommentPosition { Inline, Before }
  Document::set_comment(path, position, text)
  Document::remove_comment(path, position)
  ```

  Both go through `replace_span`, so they inherit its guard. A leading
  block takes the node's own indentation; inline removal takes the
  padding with it; an unresolvable path is an error, not a silent no-op.

- **`no_std` support on bare-metal targets** (#210) —
  `thumbv7em-none-eabihf`, `riscv32imac-unknown-none-elf` and
  `aarch64-unknown-none` build, and are covered by a CI matrix. The
  issue documented two root causes; there were four:
  - `indexmap`, `rustc-hash` and `memchr` lacked
    `default-features = false`, so `extern crate std` reached
    bare-metal. The crate's own `std` feature now turns theirs back
    on — load-bearing, because without it `FxHashMap`/`FxHashSet` stop
    existing on *hosted* builds.
  - `FxHashMap`/`FxHashSet` do not exist without std. The no_std
    prelude aliases them onto **hashbrown keyed by the same
    `FxBuildHasher`**, so hashing behaviour is identical everywhere.
  - `IndexMap`'s default hasher is std-only, which would have added a
    third type parameter to the public `Mapping::into_inner` /
    `from_inner`. The no_std prelude defaults `S` instead, so **no
    public API changes shape for hosted callers**.
  - `core` has no `f64::fract` / `f64::mul_add`; these are backed by
    **libm** on no_std.

  `wasm32-unknown-unknown` passed throughout the bug only because it
  has std available, which masked the problem entirely.

- **`fuzz_editors`** — a structured fuzz target for the edit API, as
  opposed to the byte-oriented parser targets. It applies a generated
  edit to a generated document and asserts that a refusal leaves the
  source byte-identical, that a comment edit never changes the value,
  and that an accepted `remove` shrinks the document. Runs 30 s per
  push in CI, and unlike `fuzz_diff` it is not `continue-on-error`.

- **Schema-validator hardening tests** — external `$ref` was already
  refused and recursion already bounded, but nothing asserted either.

- **`doc/MSRV-AND-DEPRECATION.md`** — the MSRV and deprecation policy,
  previously applied from memory.

### Fixed

- **`Document::remove` deleted more than it was asked to.** Given
  `a: {x: 1, y: 2}`, `remove("a.x")` deleted the entire document; with a
  preceding entry it deleted all of `a`. The typed oracle guarded only
  multi-line edits, and in a flow collection an entry shares its line
  with its siblings *and its parent*. The fast path now requires the
  entry to own its line; anything else goes through the oracle.

- **A `set` fragment could reach outside its path.**
  `set("a", "v\nc: 3")` added a new top-level key `c` and returned
  `Ok` — valid YAML, so the re-parse guard could not see it. A
  structural oracle now requires the shape outside the edited path to
  be unchanged. It compares *shape*, not values, because an edit to an
  anchored node legitimately changes its aliases elsewhere.

- **`push_back` and `insert_after` had the same hole** — both now go
  through `guarded_insert`.

- **Two `Emit` defects** found by a new round-trip property test, which
  passed locally and failed in CI on a different seed:
  - a scalar ending in a colon was emitted plain, producing `a: a:`,
    which does not parse;
  - a lone newline produced a block-literal header with an empty body.

  Neither corrupted data — the oracle refused — but `set_value`, the
  API documented as the *safe* route, could fail on valid input.

- **A comment edit could change the document's value**, found by
  `fuzz_editors` against the API added in this same release: appending
  ` #` to a block scalar makes the text scalar *content*, not a
  comment. `set_comment`/`remove_comment` now require the value to be
  unchanged and roll back otherwise, so the invariant is enforced
  rather than the contexts enumerated.

### Changed

- **`build.rs` now has a written contract**, asserted in CI by grepping
  for capability: no network, no filesystem, at most one subprocess, no
  `[build-dependencies]`.

- **`ROADMAP-TO-10.md` reconciled with the tree.** It had drifted on
  eight counts — public function count, test count, which epics had
  shipped, the state of #210 — each recorded rather than silently
  overwritten.

### Notes

`#221` is not closed: `remove_subtree`, sole-entry and flow-member
removal remain. They now refuse safely rather than corrupting.

## [v0.0.19] - 2026-08-11

### Fixed

- **`from_str_strict` rejected every populated `Option` field** (#239).
  Reported as `Option<String>` failing on `""`, which reads like an
  empty-string edge case; in fact every `Option<T>` field with a value
  failed, for every `T`. `option` sat in the
  `forward_to_deserialize_any!` list, so `deserialize_option` fell
  through to `deserialize_any`, which hands the visitor a concrete
  scalar. Only null worked — `deserialize_any` maps `Value::Null` to
  `visit_unit`, which serde accepts as `None` — likely why the bug
  survived, since the null path is the one most likely to be tested.
  `from_str` was never affected. Thanks to **@kshpytsya** for the
  report and the minimal repro.

- **Bare `nan` / `inf` spellings destroyed a scalar's text.** A mapping
  key `nAn` came back as `nan`, so `nAn: null` did not round-trip;
  found by the `roundtrip_value` property test.
  `resolve_plain_scalar` fell back to `s.parse::<f64>()`, and Rust
  accepts `nan`, `inf` and `infinity` in any case with an optional
  sign. YAML 1.2 spells the specials with a leading dot. Bare
  spellings now stay strings; an explicit `!!float nan` is unaffected.

- **`Document::remove` now takes the trivia the entry owns, and only
  that** (#225). Three cases produced a silently wrong document rather
  than a refusal:
  - a contiguous run of full-line comments directly above the entry — its
    head comment — survived the removal and silently became documentation
    for the *following* entry. It is now removed with the entry. A blank
    line still detaches the run, so a document header set off by one
    survives the removal of the first entry, and a comment at a different
    indentation is left alone.
  - a keep-chomped (`|+` / `>+`) block scalar's kept trailing blank lines
    are content rather than separation, and were stranded in the document
    after the entry was removed. They now go with it.
  - a comment *after* the entry's last content line was swallowed. Such a
    comment lies outside the value span (`span_at` already excludes it)
    and conventionally documents whatever comes next, so it is now left
    in place. A comment *interleaved* inside a multi-line value is inside
    the span and continues to go with the entry.

  The entry range is now derived from the same value-span boundary
  `span_at` reports, so `remove` and `span_at` no longer disagree about
  where an entry ends. A removal whose range covers more than one line —
  which now includes a single-line entry with a head comment — goes
  through the existing re-parse and typed-value guard; the single-line
  fast path is unchanged.

  Thanks to **@zoosky**.

### Changed

- **Configured clippy lints were never applied** (#228).
  `[package.metadata.clippy] warn-lints = [...]` is not a table cargo
  reads, so none of the configured lints were in effect. It is now a
  real `[lints.clippy]` table with `cargo`, `pedantic` and `nursery` at
  warn. Test files no longer carry `allow(clippy::all)`, so the suite is
  linted too.

- **`serde_core` is now a direct dependency** (#227), replacing `serde`
  with derive.

- Clippy-driven refactors, all from **@EdJoPaTo** with authorship
  preserved: `i64::try_from` in place of a `as i64` cast (#240),
  `pedantic`/`nursery` autofixes (#241), `use Trait as _` for traits
  imported only for their methods (#242), `format!("literal")` →
  `"literal".to_string()` (#243), and an unconditional
  `impl core::error::Error for ParseNumberError` (#256).

- **Coverage gate rebaselined 96% → 95% (functions).** The threshold
  left under three functions of slack, and a no-op rename
  (`MappingAny::with_capacity` → `Self::with_capacity`) moved the
  measurement. The same commit reported 78 uncovered then 77 on
  consecutive runs of an identical command. Line (94%) and region (93%)
  floors are unchanged — those were stable across every run.

### Dependencies

Two consolidation waves, 19 Dependabot pull requests. serde-saphyr
0.0.29 → 1.0.1, bytes 1.12.0 → 1.12.1, jsonschema 0.49.2 → 0.49.6,
validator 0.19.0 → 0.21.0, sval 2.20.0 → 2.21.0, schemars 1.2.1 →
1.2.2, the tokio-stack group, and the workflow action groups
(#257, #238). serde-saphyr's major bump is bench-only — optional, gated behind
`compare-saphyr`, and the benches were verified to compile against
1.0.1 rather than the API being assumed stable.

Nine crates came up unvetted after the bumps. Refreshing imported
audits covered four outright with genuine third-party audits —
fancy-regex, jsonschema, referencing, regex-automata — rather than
exempting them; the remaining five already had exemptions.
**Nine unvetted crates produced zero new exemptions.** `libm` likewise
entered via a refreshed audit.

## [v0.0.18] - 2026-07-31

### Added

- **`Document::rename_key(path, new_key)`** — first-class,
  re-parse-guarded mapping-key rename (#221, gap 2). The path
  addresses the entry the same way `set` / `remove` do; only the
  key token's bytes are rewritten — the `:`, the value,
  whitespace, comments, and sibling entries survive verbatim.
  The new key's spelling is style-matched to the key it replaces:
  a plain key stays plain when the plain spelling re-parses to
  exactly that string, a quoted key keeps its quote style, and
  quoting is forced only when the plain spelling would re-parse
  to something else (`a: b`, `-flag`, `8080`). Renaming a key to
  its own name is a byte-preserving no-op, decided on the decoded
  key so a plain `true:` is never requoted. Refuses:
  sibling-duplicate renames (reported separately when the
  colliding sibling comes from a `<<` merge), flow-mapping
  entries, alias keys, keys produced by a `<<` merge, paths
  reached through an alias, entries inside an anchored value that
  has alias references, bracket path segments that are not
  indices (`servers[web]`), `<<` as the new key, and new keys
  carrying non-printable characters. After the splice the
  document must re-parse to the old value with exactly that one
  key renamed, or the edit is rolled back and the failure is
  reported in the operation's own terms.
- **`Document::key_span(path)`** — read-only byte span of a
  mapping entry's key token, the companion to `span_at` (which
  returns the value span). Exposes, read-only, the same key site
  `rename_key` rewrites, so tooling can report duplicate keys with
  positions or drive a "rename key" code action without walking
  the green tree by hand (#221). Returns `None` for sites that own
  no simple scalar key — sequence indices, alias (`*name`) sites,
  and keys provided by a `<<` merge.
- **`Document::swap_items(path, i, j)`** — exchange two items of a
  block sequence, rewriting only the two items' value bytes; the
  `- ` indicators, indentation and every other item stay
  byte-identical (#221, gap 3). Guarded like the other mutators:
  the result must re-parse and its typed value must equal the
  original with exactly items `i` and `j` exchanged, or the edit
  rolls back. Swapping an index with itself, or two equal values,
  is a byte-preserving no-op.
- **`Document::move_item(path, from, to)`** — move a block-sequence
  item to a new index, shifting the items in between (#221, gap 3).
  Applied as a run of adjacent `swap_items` steps, so it inherits
  the structure-preservation and per-step guard, and the whole move
  is atomic: a refused step rolls the document back to its state
  before the call.
- **`Document::set_inline_comment(path, text)`** and
  **`Document::remove_inline_comment(path)`** — first-class mutation
  of the trailing `#` comment on a single-line node (#221, gap 1).
  `set` replaces an existing inline comment in place (keeping its
  separating whitespace) or appends `  # <text>` after the value;
  `remove` takes the separating whitespace with it. Both are guarded
  like the other mutators — the edit must re-parse and leave the
  typed value unchanged (a comment carries no data), or it rolls
  back. Multi-line nodes and newlines in the text are refused;
  removing a comment that is not there is a no-op.
- **`Document::set_leading_comment(path, text)`** and
  **`Document::remove_leading_comment(path)`** — mutation of the
  leading comment block above a single-line mapping key (#221, gap 1).
  `set` renders `text` as one `#`-prefixed line per `\n` segment at the
  key's indentation, replacing an existing block in place or inserting
  one above the entry; `remove` deletes the block. Same re-parse +
  value-unchanged guard with rollback. Multi-line / nested entries and
  sequence-item leading blocks remain a follow-up.
- **The `Emit` auto-formatting tier** (#221, gap 5) — `cst::Emit` and
  `cst::EmitCtx`, plus **`Document::insert_entry_value`**,
  **`Document::push_back_value`** and
  **`Document::insert_after_value`**, the typed counterparts of the
  three fragment-taking insertion mutators. Where `insert_entry` /
  `push_back` / `insert_after` splice their `&str` verbatim — so a
  fragment holding `a: b`, a leading `-` or a `#` becomes YAML
  *syntax*, which the existing guard cannot catch because the result
  is still valid YAML — the `_value` methods emit the spelling that
  re-parses to exactly the value given. Strings that would change type
  or structure are quoted (`8080`, `true`, `- x`, `a: b`), keys are
  quoted only when they must be, multi-line strings become block
  scalars, nested collections are emitted at the file's detected
  indent, and the file's dominant quote style is followed wherever it
  faithfully represents the data. `Emit` pairs `emit` with
  `expected_value`, and every splice must re-parse **and** load back
  as the pre-edit value with exactly that one insertion applied, or it
  rolls back. Refuses: `<<` and non-printable keys, tagged values,
  growing an existing scalar entry into a collection, replacing a key
  holding `.` or `[` (unaddressable by the path syntax — inserting one
  is fine), insertions inside an aliased anchor (named, with the
  `materialise_aliases_of` fix suggested), and empty mappings /
  sequences that offer no indent anchor. `Entry::insert_value` and
  `Entry::or_insert_value` now route through this tier — closing the
  same hole on the `Entry` API, whose key was previously spliced
  verbatim — and `Entry` gains `push_back_value` / `insert_after_value`.

### Changed

- **`Document::remove` now removes multi-line and nested block
  values** (#221, gap 4) — a key whose value is a nested mapping,
  block sequence, or block scalar deletes the whole entry (key/`-`
  through its last owned line), where it was previously refused. The
  multi-line splice is guarded by an eager re-parse and a typed-value
  oracle (the document minus exactly that path) with rollback on any
  mismatch; the single-line case keeps its original fast path.
  Removing the sole entry of a block, and flow-collection entries,
  remain refused.
- **`BudgetBreach::MaxSequenceLength` and `BudgetBreach::MaxMappingKeys`**
  — the sequence-width (`max_sequence_length`) and mapping-width
  (`max_mapping_keys`) caps now trip these structured
  [`Error::Budget`] variants instead of an opaque `Error::Serialize`
  string, so a DoS-aware caller routing on `ErrorKind::Budget`
  classifies width-based resource exhaustion alongside every other
  budget breach. (Both `#[non_exhaustive]` additions.)

### Fixed

- **`Document::rename_anchor` now refuses a colliding target name.**
  Renaming `&a` to a name another anchor already declares (e.g. `&b`)
  used to succeed and leave two `&b` declarations, silently making every
  `*b` alias resolve to the last one (YAML 1.2.2 §7.1) — a refactor that
  changed the document's meaning. It now returns an error and leaves the
  document byte-for-byte unchanged (a no-op `old == new` rename is still
  allowed). The `# Errors` docs were corrected to describe the actual
  single-splice, all-or-nothing behaviour, and `rename_anchor` is now
  demonstrated in the `cst_surgical_edit` example and listed in the
  User Guide's mutator table.
- **`insert_entry` / `push_back` / `insert_after` no longer corrupt a
  document whose last line has no terminator.** They splice at the end
  of the anchor entry's line, which for a file not ending in `\n` is
  the end of the source — so the new entry landed on the tail of the
  old one (`a: 1  b: 2`) and the splice was rejected as a parse error.
  The new text now opens with the line break the document lacks.

### Security

- **`ParserConfig::max_nodes` is now enforced.** The documented AST
  node budget (default 250 000; 25 000 under `strict()`) was defined
  and defaulted but never counted, so a node-dense payload — a long
  run of empty collections (`[]`/`{}`) that minimises scalar bytes and
  stays under `max_events` — was bounded four times looser than
  documented. Both loaders now count each scalar/sequence/mapping node
  and trip [`BudgetBreach::MaxNodes`] at the cap. Legitimate large
  documents are unaffected (a 5 000-package `pnpm-lock.yaml` is ~70k
  nodes); deliberately oversized inputs raise the budget as before.
- Added the **`harden_untrusted`** example — a tour of hardening a
  parser against hostile YAML (`DenyAnchors` / `DenyTags` /
  `MaxScalarLength` policies plus the `max_depth`,
  `max_alias_expansions`, and `max_nodes` budgets), each shown
  refusing a matching attack (anchor injection, custom tags, oversized
  scalars, a billion-laughs alias bomb, an empty-collection node bomb,
  and deep nesting) while still accepting a real configuration.
- Added adversarial DoS regression suites (`dos_hardening`,
  `max_nodes_budget`) covering deep-**flow** rejection, the width-cap
  budget variants, merge-key amplification, and the `max_nodes` cap;
  and a `reject_node_bomb` case in the `architecture` security
  benchmark.

## [v0.0.17] - 2026-07-25

A **lockstep-only** cut. The core crate has **no code or behaviour change**
since v0.0.16 — `main` was byte-identical to the v0.0.16 tag. It is
republished at 0.0.17 solely so the satellite crates, which carry real
fixes, can pin `=0.0.17` under the ADR-0005 strict-lockstep contract.

### Satellite fixes shipping in this lockstep

- **`noyalib-lsp`** — `textDocument/formatting` is no longer a silent
  no-op. It used a byte-faithful CST round-trip and always returned an
  empty edit list; it now calls `cst::format`. (The v0.0.16 changelog
  claimed this was fixed; it was not — this is the actual fix.)
- **`noyalib-lsp` / `noya-cli`** — `crossbeam-epoch` bumped to 0.9.20
  (RUSTSEC-2026-0204, invalid-pointer-dereference), which was present in
  their v0.0.16 lockfiles via a transitive dependency.

### Repository hardening (all crates, CI/docs only)

- Coverage, MSRV, CodeQL, and OpenSSF Scorecard gates brought to parity
  across the four satellites; `noyalib-wasm` gained a CI `wasm-test`
  job (`wasm-pack test --node`) gating its wasm-bindgen surface.
- Upstream cargo-vet audit imports added to the satellites; branch-
  protection tightened so commit signing is unskippable.

## [v0.0.16] - 2026-07-22

A **build-fix + dependency-refresh** cut. `main` was left unbuildable
under `--all-targets` by a feature-gated import, and the dependency set
had drifted. No public API change (`cargo-semver-checks` green); two
deserialiser error-message strings change wording — see *Fixed*.

Full narrative: [`doc/release-notes/v0.0.16.md`](doc/release-notes/v0.0.16.md).

Lockstep versioning: `noyalib` bumps `0.0.15` → `0.0.16`.
Satellites publish `=0.0.16` from their own repos:
- [`sebastienrousseau/noyalib-wasm@0.0.16`](https://github.com/sebastienrousseau/noyalib-wasm)
- [`sebastienrousseau/noyalib-mcp@0.0.16`](https://github.com/sebastienrousseau/noyalib-mcp)
- [`sebastienrousseau/noyalib-lsp@0.0.16`](https://github.com/sebastienrousseau/noyalib-lsp)
- [`sebastienrousseau/noya-cli@0.0.16`](https://github.com/sebastienrousseau/noya-cli)

> **Satellite note — `noyalib-lsp` carries a user-facing fix in this
> cut.** `textDocument/formatting` was a silent no-op: the server derived
> its output from a byte-faithful CST round-trip, so it always returned
> an empty `TextEdit[]` and no editor ever saw a formatting change. Fixed
> by calling `cst::format`. See that repo's `CHANGELOG.md`. Nothing in
> the core crate changed as a result — the bug was in the LSP wrapper.

### Fixed

- **`cargo check --all-targets` no longer fails on default features.**
  `tests/coverage_value_serde.rs` imported `to_value` unconditionally,
  but its only consumer is gated behind `#[cfg(feature =
  "lossless-u64")]`. With default features the import was unused and
  `-D unused` promoted it to a hard error, breaking the test build. The
  import is now gated by the same `cfg` rather than removed — removing
  it would have compiled by default while breaking every
  `--all-features` build.
- **`expected integer, found integer`.** Deserialising a
  `Number::Unsigned` above `i64::MAX` into an `i64` rendered both sides
  of the mismatch with the same word, because `type_name` collapses
  `Integer` and `Unsigned` onto one label — the message told the caller
  nothing. It now reads `type mismatch: expected signed integer (i64),
  found unsigned integer <n>, above i64::MAX`. Requires `lossless-u64`.
  The branch was reachable but untested, which is why the text had never
  been read by a human; it now has an exact-match regression test.
- **The mirror case**: a negative `Number::Integer` deserialised into a
  `u64` reported `found integer` without mentioning the sign. It now
  reads `type mismatch: expected unsigned integer, found negative
  integer <n>`.

  > Both are message-wording changes only. No `Error` variant, no
  > `Error::kind()` classification, and no miette code changed. Error
  > wording is explicitly outside the stability contract (variant names
  > are stable) — but if you match on the rendered string, match on
  > `Error::TypeMismatch { .. }` instead.

### Changed — MSRV 1.85 → 1.86

- **The minimum supported Rust version is now 1.86.0 — for the core
  library as well as every satellite.** 1.86 is the lowest toolchain the
  project can be **built and tested** on: `criterion 0.8` (a
  dev-dependency) declares `rust-version = 1.86`, so `cargo check
  --all-targets`, the bench suite and the coverage gate all fail on
  1.85 with `criterion@0.8.2 requires rustc 1.86`.
  The library *alone* still compiles on 1.85 (`cargo +1.85.0 check
  --lib` succeeds) — but no CI leg can run there, so a 1.85 claim would
  be an MSRV the project cannot verify, and nothing would catch the day
  it broke. **We publish the floor we test.** If you are pinned to 1.85
  and cannot move, v0.0.15 remains available and this is the only reason
  to stay on it.
- **The bump policy is now explicit: never speculative.** The MSRV moves
  only when the toolchain we build and test at actually moves — a
  dependency raising its floor, or a language feature we adopt — and
  never for tidiness or "headroom". As a standing guarantee, noyalib
  will not require a rustc newer than 12 months old at release time
  (1.86.0 shipped 2025-04-03). Recorded in `doc/POLICIES.md` §1.
- **The historical split MSRV is gone.** Docs previously claimed the
  core library floored at 1.75 while satellites sat at 1.85 — an
  inconsistency that had already drifted out of sync with the actual
  `rust-version` (1.85). The whole lockstep set now shares one floor,
  1.86.0, recorded in `doc/POLICIES.md`.
- The MSRV CI job was renamed `msrv-1-85-core` → `msrv-core` so future
  bumps do not churn the job (and required-status-check) name.
  **Action required:** if `msrv-1-85-core` is a required status check in
  branch protection, update the rule to `msrv-core` or PRs will block
  on a check that no longer reports.

### Changed — MSRV bump policy

- **An MSRV bump is now a patch, not a minor-version event, while the
  project is on `0.0.x`.** `doc/POLICIES.md` previously declared a core
  MSRV bump a *minor-version event*; taken literally, this release
  would have had to be `0.1.0`. That rule was written when `0.1.0` was
  the next planned cut, but §2 of the same document commits to
  iterating `0.0.2 … 0.0.99` before graduating to `0.1.0` — so there is
  no minor slot to spend, and the two rules contradicted each other.
  The policy is revised to match the actual `0.0.x` posture: MSRV bumps
  ship as patches, in lockstep, and must be called out under an
  explicit `### Changed — MSRV` heading (as here).
- This reverts to a genuine minor-version event at `1.0`, per the gates
  in `PLAN.md`. Recorded in `doc/POLICIES.md` §1 with the superseded
  rule quoted in full rather than deleted.
- The opt-in, bench-only `compare-saphyr` feature remains outside the
  MSRV gate — `serde-saphyr` uses let-chains and needs rustc 1.88+.
  `--all-features` therefore still requires a newer toolchain than the
  declared MSRV, as before.

### Changed — dependencies

- **`jsonschema` 0.46 → 0.48.** Optional, behind `validate-schema`. The
  consumed surface is unchanged — `validator_for`, `JsonType` and
  `error::{TypeKind, ValidationErrorKind}`, used from
  `src/schema_validate.rs` and `src/cst/coerce.rs`. No behavioural delta
  was observed: the 55 tests across `schema_validate`, `schema_codegen`,
  `coverage_schema_validate`, `coerce_to_schema`, `coerce_to_schema_extra`
  and `cst_schema_tag_audit` pass unmodified against 0.48.
- **Three new transitive crates** arrive with it, all reachable only
  under `validate-schema`: `jsonschema-value`, `strum` and
  `strum_macros` (a proc-macro). Each is recorded as a version-pinned
  `safe-to-deploy` exemption in `supply-chain/config.toml` pending an
  upstream audit, and all three clear the `cargo-deny` licence
  allowlist.
- **The lockfile is otherwise unchanged from `main`.** An earlier
  revision of this branch refreshed 71 crates to their latest
  semver-compatible versions; that was reverted before merge because
  `cargo-vet` exemptions are version-pinned, so the refresh invalidated
  81 of them at once. The shipped lockfile is `main`'s baseline plus the
  `jsonschema` bump above — a six-crate vet surface a reviewer can
  actually check. The Dependabot PRs the refresh had superseded (the
  `serde` group #201, the `tokio-stack` group #202) were closed rather
  than merged, so those updates are **not** in this release; they will
  be re-proposed against the next published version and land
  individually, each with its own reviewable `cargo-vet` delta.

### Changed — shared workflows

- `shared-msrv-core.yml`'s default `toolchain` input moved `1.85.0` →
  `1.86.0`. Satellites that call the workflow **without** passing an
  explicit `toolchain:` will adopt the 1.86 floor the moment they bump
  their pinned SHA. Satellites that pass the input are unaffected.
- Pinned action SHAs refreshed across every workflow
  (`dtolnay/rust-toolchain`, `github/codeql-action` v3 → v4.37.1,
  `EmbarkStudios/cargo-deny-action` v2.0.20 → v2.1.1) and the
  `Dockerfile.full` builder image moved `rust:1.96-bookworm` →
  `rust:1.97-bookworm`, digest-pinned as before.

### Documentation

- **Corrected a `no_std` instruction that verified nothing.**
  `doc/POLICIES.md` recommended `--no-default-features --features
  minimal`, but `minimal = ["std"]` — it is a *dependency-budget* alias
  that turns `std` back on while dropping `itoa`, `ryu` and
  `serde_ignored`. That command silently produced a `std` build. The
  correct invocation is `--no-default-features` alone, which is what CI
  has always run.
- **Scoped the `no_std` claim to hosted targets.** Bare-metal `*-none`
  targets do not build today — `indexmap`, `rustc-hash` and `memchr` are
  declared without `default-features = false`, and the crate uses
  `rustc_hash::FxHashMap`, which needs `std`. Both blockers are
  pre-existing and are now documented as a deliberate non-goal.
- **`src/lib.rs`'s MSRV section** still declared 1.85 and a third,
  different bump policy ("ships a major version") that matched neither
  `doc/POLICIES.md` before this release nor after it. Rewritten to match
  §1 of `doc/POLICIES.md`, which is the single source of truth.
- **`cargo-vet` exemption conventions** documented in `doc/POLICIES.md`
  §10 — including what `suggest = false` means and why exemptions are
  never added in bulk. They live there rather than inline because
  `cargo vet fmt` strips comments from `supply-chain/config.toml`.
- Added `doc/release-notes/v0.0.16.md`, matching the per-release file
  `doc/CII-BEST-PRACTICES.md` claims for every tagged release. No CI gate
  asserts this; noted as a follow-up.
- Added `examples/strict_deserialise.rs` — the default-on
  `strict-deserialise` feature had no example. Linked from
  `doc/USER-GUIDE.md` §5.

### Testing (no behavioural change)

- Coverage for public API with **zero** prior test references —
  `Mapping::get_index_of`, `cst::Document::comments_at`, `schema_for`,
  `schema_for_yaml` — found by cross-referencing every `pub fn` in
  `src/` against the whole `tests/` tree.
- Coverage for the `lossless-u64` `Number::Unsigned` widening and
  overflow-rejection arms, and the `cst::format` flow-collection paths.
  Exercising the overflow arm is what surfaced the
  `expected integer, found integer` defect fixed above.
- `interpolate_properties_redacted` had no coverage while its strict and
  lossy siblings were well covered; it now has four tests, including the
  negative assertion that is the point of the API (the placeholder name
  must not leak into the error).

## [v0.0.15] - 2026-07-11

The **loader-parity completion + coverage-hardening** cut. Finishes the
three-loader DoS-budget parity started in v0.0.14 by extending the
remaining budgets to the `NoSpanLoader` fast path and the
distinct-typed-key collision guard to the streaming loader, then drives a
workspace-wide coverage campaign (≈16 files to effective-100%) with no
change to public API or behaviour beyond the parity fixes.

Lockstep versioning: `noyalib` bumps `0.0.14` → `0.0.15`.
Satellites publish `=0.0.15` from their own repos:
- [`sebastienrousseau/noyalib-wasm@0.0.15`](https://github.com/sebastienrousseau/noyalib-wasm)
- [`sebastienrousseau/noyalib-mcp@0.0.15`](https://github.com/sebastienrousseau/noyalib-mcp)
- [`sebastienrousseau/noyalib-lsp@0.0.15`](https://github.com/sebastienrousseau/noyalib-lsp)
- [`sebastienrousseau/noya-cli@0.0.15`](https://github.com/sebastienrousseau/noya-cli)

### Fixed — loader parity (security)

- **`NoSpanLoader` DoS-budget parity, completed.** The `Value` fast path
  now also enforces `max_events`, the total-scalar-bytes budget, and the
  `alias_anchor_ratio` — the three budgets still span-full-only after
  v0.0.14. All three loaders (streaming, span-full `Loader`,
  `NoSpanLoader`) now enforce the same DoS budgets, with cross-path
  tests (`no_span_loader_parity`).
- **Distinct-typed key-collision guard on the streaming loader.** The
  guard that raises `Error::KeyCollision` for `1: a` vs `"1": b` (added
  to the AST paths in v0.0.14) now also runs on the streaming
  deserialiser, closing the last loader where the collision could
  silently collapse (`key_collision_streaming`).

### Testing / tooling

- Workspace coverage campaign: ~16 files driven to effective-100%
  (`de`, `include`, `schema_validate`, `compat/serde_yaml`, `base64`,
  `cst/coerce`, `error`, `ser`, `value/number`, `cst/green`, `recovery`,
  and more). Region/line/function coverage rises across the workspace;
  no behavioural change.
- `make coverage-gap` restored under `cargo-llvm-cov ≥ 0.8.7` (the
  empty `--ignore-filename-regex` is now guarded, matching CI).

## [v0.0.14] - 2026-07-07

The **loader-parity** cut. Fixes a fast-path silent-collapse of
distinct-typed mapping keys plus three DoS-budget parity gaps
between the span-full and span-free loaders, adds a coarse
`Error::kind()` classifier for downstream routing, and lands
five CST `span_at` fixes and one scanner lone-CR fix.

Lockstep versioning: `noyalib` bumps `0.0.13` → `0.0.14`.
Satellites publish `=0.0.14` from their own repos:
- [`sebastienrousseau/noyalib-wasm@0.0.14`](https://github.com/sebastienrousseau/noyalib-wasm)
- [`sebastienrousseau/noyalib-mcp@0.0.14`](https://github.com/sebastienrousseau/noyalib-mcp)
- [`sebastienrousseau/noyalib-lsp@0.0.14`](https://github.com/sebastienrousseau/noyalib-lsp)
- [`sebastienrousseau/noya-cli@0.0.14`](https://github.com/sebastienrousseau/noya-cli)

### Fixed — loader parity (security)

- **`from_str::<Value>` no longer silently collapses distinct-
  typed key collisions.** The fast `Value` path
  (`parse_one_value` → `NoSpanLoader`) previously accepted
  `1: a\n"1": b\n` and dropped the first entry — data loss.
  The `NoSpanLoader` now runs the same distinct-typed-key
  check as the span-full loader and raises
  `Error::KeyCollision` instead. The streaming fast-path is
  bypassed for the `Value` target when no tag registry is
  active, so the collision check is reachable there too.
- **DoS-budget parity on the `Value` fast path.** `NoSpanLoader`
  now enforces `max_sequence_length`, `max_mapping_keys`,
  `max_merge_keys`, `max_document_length` (via `alias_bytes`),
  and `MergeKeyPolicy::Error` — all previously span-full-only.
- **`DuplicateKeyPolicy` parity.** `NoSpanLoader` now honours
  `First` / `Last` / `Error` on the `Value` fast path instead
  of always defaulting to last-wins.
- **`from_str_with_config` enforces `max_document_length` inline.**
  Previously the check lived only on the streaming path; typed
  targets that dropped through to the AST loader could parse
  oversized documents.
- **Merge-key clone gate.** The typed-key `Value::clone()`
  retained on every mapping key for the collision check is now
  skipped when the key is a merge key (`<<`) that will be
  buffered rather than inserted — `<<`-heavy documents no
  longer pay the clone cost.

### Added — API & tooling

- **`Error::kind() -> ErrorKind`** — coarse-grained
  classification for downstream error routing without
  pattern-matching every variant of the `#[non_exhaustive]`
  `Error` enum. `ErrorKind` is itself `#[non_exhaustive]`.
- **Anchor typo suggestions on the AST loader.** Both `Loader`
  and `NoSpanLoader` now populate `Error::UnknownAnchorAt`'s
  `suggestion` field with the closest-known-anchor name and its
  definition location — parity with the streaming path's
  `build_unknown_anchor`.
- **Special-value float keys.** `nan` / `inf` / `-inf` mapping
  keys now stringify to their canonical plain-scalar form
  (`"nan"`, `"inf"`, `"-inf"`) instead of Rust's `{:?}` output
  (`"NaN"`), so keyed lookups match how the YAML was written.
- **New criterion bench: `mapping_key_clone`** — guards the
  ordinary vs merge-heavy mapping-key hot path against a
  future regression.
- **New fuzz target: `fuzz_no_span_loader`** — cross-checks
  `from_str::<Value>` against `cst::parse_document` on
  arbitrary input; any divergence is a parity bug.

### Fixed — CST spans

Five `span_at` correctness fixes from the earlier commits on
this branch:

- Alias references resolve through to the anchor value's span.
- Block-collection value spans include their first line's
  indentation, so the returned slice re-parses to the selected
  value.
- Keep-chomped block scalars (`|+`, `>+`) retain their kept
  trailing blank lines in `span_at`.
- Implicit-null nodes report no span (`None`) instead of the
  indicator character's location.

### Fixed — scanner

- Lone `\r` (classic-Mac CR-only line breaks) is now a valid
  line break for YAML 1.2.2 §5.4 compliance.

### Added — coverage (examples + benches)

Closes the "zero examples / zero benches" gaps identified by
the coverage audit. Every public module now has at least one
example and every performance-relevant path has a bench:

- `examples/interner.rs` — key deduplication on a
  Kubernetes-shaped 10 000-record workload.
- `examples/parallel.rs` — sequential (`load_all_as`) vs
  parallel (`parallel::parse`) with real speedup measurement.
- `examples/simd.rs` — single/multi-byte search + prebuilt
  `ByteBitmap` + stateful `SimdScanner`.
- `benches/interner.rs` — naïve `String` vs `Arc` vs
  `KeyInterner` on realistic Kubernetes keys.
- `benches/parallel.rs` — sweeps document-size to expose the
  break-even between sequential and parallel.
- `benches/borrowed_vs_value.rs` — `BorrowedValue` vs `Value`
  throughput on a string-heavy workload.

### Notes for the `no_std` and typed-target audit

- The distinct-typed key-collision check is now enforced on
  every parse of `Value` — `std` and `no_std`.
  `no_default_features` users get the same defensive behaviour
  as `std` users.
- The `Error::kind()` classifier reports `Budget` for every
  `BudgetBreach` variant and for `RecursionLimitExceeded` /
  `RepetitionLimitExceeded`. Note: `max_sequence_length` and
  `max_mapping_keys` currently surface as `Error::Serialize`
  (historical spelling) and classify as `Data` — worth folding
  under `Error::Budget` in v0.0.15 for full parity.

## [v0.0.13] - 2026-07-05

The **workspace-split completion** cut. All three remaining
satellites — `noyalib-mcp`, `noyalib-lsp`, `noya-cli` — leave
the monorepo per ADR-0005 in a single bundled release,
alongside `noyalib-wasm` which had already split at v0.0.12.
The parent repo becomes a **single-crate** repo hosting only
the `noyalib` library core.

Also folds in three PR takeovers:

- **`lossless-u64`** opt-in feature from @canardleteer's PR #117
  (via PR #142 rebase).
- **Duplicate mapping key last-wins fix** from @zoosky's PR #143.

Lockstep versioning: `noyalib` bumps `0.0.12` → `0.0.13`.
All four satellites publish `=0.0.13` from their own repos:
- [`sebastienrousseau/noyalib-wasm@0.0.13`](https://github.com/sebastienrousseau/noyalib-wasm)
- [`sebastienrousseau/noyalib-mcp@0.0.13`](https://github.com/sebastienrousseau/noyalib-mcp)
- [`sebastienrousseau/noyalib-lsp@0.0.13`](https://github.com/sebastienrousseau/noyalib-lsp)
- [`sebastienrousseau/noya-cli@0.0.13`](https://github.com/sebastienrousseau/noya-cli)

### Added — workspace split (v0.0.13 pilot)

- **`noyalib-mcp` extracted** to
  [`sebastienrousseau/noyalib-mcp`](https://github.com/sebastienrousseau/noyalib-mcp)
  via `git subtree split`. 14 commits with authorship
  preserved. Multi-channel release (crates.io + npm wrapper +
  GHCR container + MCP Registry) with SLSA L3 + sigstore on
  every channel.
- **Post-implementation update** in ADR-0005 records the
  pilot's concrete outcome (subtree extraction, lockstep
  enforcement, `pull-requests: read` applied preemptively per
  v0.0.12 lesson).

### Added — lossless-u64 feature (PR #117 takeover, PR #142)

- Opt-in `lossless-u64` Cargo feature: adds
  `Number::Unsigned(u64)` behind `cfg(feature = "lossless-u64")`
  so integer values in `[i64::MAX + 1, u64::MAX]` round-trip
  losslessly instead of falling back to `Float`. Runtime opt-in
  on the parser side via
  `ParserConfig::lossless_u64_integers(bool)`; serializer side
  is compile-time-only.
- `Number` enum marked `#[non_exhaustive]` so future numeric
  variants don't break match arms.
- Dedicated CI feature-matrix guard with isolated
  `CARGO_TARGET_DIR` per leg (defaults + all-features).
- 6 DoS-budget × u64 proptest properties in
  `dos_limits_lossless_u64` module — every parser DoS budget
  must fire before scalar resolution, and no `u64::MAX`-adjacent
  scalar can silently wrap to negative `Integer`.
- Original submission credit: @canardleteer's
  [PR #117](https://github.com/sebastienrousseau/noyalib/pull/117).
  ADR-0004 documents the design.

### Removed — workspace split (v0.0.13 pilot)

- `crates/noyalib-mcp/` directory (history preserved on
  satellite). Root `Cargo.toml` workspace member list drops
  the entry.
- `.github/workflows/mcp-inspect.yml`,
  `.github/workflows/publish-mcp.yml` — moved to satellite.
- `pkg/npm-mcp-wrapper/`, `pkg/docker/Dockerfile.mcp` — moved
  to satellite.
- `server.json`, `glama.json` — moved to satellite.
- `release.yml` version cross-check now covers 2 in-workspace
  satellites; publish loop drops `noyalib-mcp`.
- `release-binaries.yml` drops the `npm-publish-mcp` job and
  the `container-publish` matrix's `noyalib-mcp` row.
- Coverage `ignore-filename-regex` in `ci.yml`,
  `shared-coverage.yml`, and `coverage-gap-report.sh` no
  longer references `crates/noyalib-mcp/`.
- README ecosystem table + per-crate README pointers link to
  the satellite. `doc/USER-GUIDE.md` and `doc/ARCHITECTURE.md`
  reflect the split.

## [v0.0.12] - 2026-07-02

Three threads land together in this cut:

1. **`noyalib-wasm` split** — first satellite to leave the
   monorepo under [ADR-0005](doc/adr/0005-workspace-split.md).
   Moves to
   [`sebastienrousseau/noyalib-wasm`](https://github.com/sebastienrousseau/noyalib-wasm)
   with 11 commits of history preserved. Strict-lockstep
   versioning contract: satellite pins `noyalib = "=0.0.12"`.
2. **MCP-discoverability** — registers `noyalib-mcp` with the
   official Model Context Protocol Registry (via OCI packaging),
   adds MCP-spec conformance CI, ships a Glama directory
   manifest, and cross-links the sibling banking MCP servers.
3. **Workspace-split CI shared-workflows** (phases 1–4, PRs
   #135–#139): 20+ new `shared-*.yml` reusable workflows,
   `ci.yml` refactored to delegate to them (-480 lines), CI
   duration monitor, crates.io ownership harness, and ADR-0005.

Lockstep versioning: 4 in-workspace publishable crates
(`noyalib`, `noyalib-mcp`, `noyalib-lsp`, `noya-cli`) bump from
`0.0.11` → `0.0.12`. `noyalib-wasm` releases the same `0.0.12`
from the satellite repo. `xtask` stays at `0.0.1` per workspace
convention.

### Added — workspace split (v0.0.12 pilot)

- **`noyalib-wasm` extracted** to
  [`sebastienrousseau/noyalib-wasm`](https://github.com/sebastienrousseau/noyalib-wasm)
  via `git subtree split`. 11 commits with authorship
  preserved. Satellite consumes reusable workflows from this
  repo pinned by SHA; a hardening pass here propagates within
  48h via Dependabot.
- **Permissions-gotcha table** added to ADR-0005 §Shared
  reusable workflows — v0.0.13 / v0.0.14 / v0.0.15 satellites
  MUST union `pull-requests: read` into their caller
  `ci.yml permissions:` block or first CI runs will
  startup_failure with 0 scheduled jobs. Discovered on the
  pilot; documented so successors don't repeat.
- **Post-implementation update** in ADR-0005 records the
  pilot's concrete outcome (subtree extraction, lockstep
  enforcement, ruleset applied).

### Removed — workspace split (v0.0.12 pilot)

- `crates/noyalib-wasm/` directory (history preserved on the
  satellite repo). Root `Cargo.toml` workspace member list
  drops the entry.
- `release.yml` version cross-check now covers 3 in-workspace
  satellites; the `noyalib-wasm` cross-check moves to the
  satellite's own release workflow.
- `release-binaries.yml` drops the `npm-publish` job for
  `@sebastienrousseau/noyalib-wasm`. That publish now runs from the
  satellite repo.
- Coverage `ignore-filename-regex` in `.github/workflows/ci.yml`,
  `shared-coverage.yml`, and `scripts/coverage-gap-report.sh`
  no longer references `crates/noyalib-wasm/src/lib.rs`.
- README ecosystem table + per-crate README pointers link to
  the satellite repo. `doc/USER-GUIDE.md` and
  `doc/ARCHITECTURE.md` reflect the split.

### Added — MCP registry work (noyalib-mcp only)

- **Official MCP Registry integration.** `noyalib-mcp` is now
  registered with the official Model Context Protocol Registry
  (`registry.modelcontextprotocol.io`) as
  `io.github.sebastienrousseau/noyalib-mcp`. A new `server.json` at
  the repo root provides the registry metadata using
  `registryType: oci` (crates.io is not a supported registryType, so
  the OCI image at `ghcr.io/sebastienrousseau/noyalib-mcp` is the
  package artefact). The `noyalib-mcp` README carries an
  `mcp-name: io.github.sebastienrousseau/noyalib-mcp` marker used by
  the registry for OCI ownership verification.
- **Auto-publish workflow** (`.github/workflows/publish-mcp.yml`) —
  on every `v*.*.*` tag push:
  1. Builds and pushes the OCI image (reusing the existing
     production-grade `pkg/docker/Dockerfile.mcp` — distroless,
     non-root, signed) to GHCR.
  2. Authenticates to the MCP Registry via GitHub OIDC (no secrets
     required), syncs the tag version into `server.json`, and runs
     `mcp-publisher publish`.
- **Protocol conformance CI** (`.github/workflows/mcp-inspect.yml`) —
  builds `noyalib-mcp` release binary, then runs
  `@modelcontextprotocol/inspector --cli` against `tools/list`.
  Path-filtered to the `noyalib-mcp` / `noyalib-core` / `noyalib-schema`
  paths to keep the CI budget bounded.
- **Glama directory manifest** (`glama.json`) — Glama listing with
  OCI runtime spec (`docker run -i ghcr.io/sebastienrousseau/noyalib-mcp`).
- **Suite discoverability.** `crates/noyalib-mcp/README.md` now
  cross-links sibling banking MCP servers (`pain001-mcp`,
  `bankstatementparser-mcp`, `camt053-mcp`, `acmt001-mcp`) under a
  "Related MCP Servers" section, positioning `noyalib-mcp` as
  structured-data tooling that complements the ISO 20022 servers.
- **REUSE compliance** — pre-commit hook auto-registered `server.json`
  under `REUSE.toml` license aggregation.

### Added — workspace CI (pre-existing on branch)

Reviewed via separate PRs #135–#139:

- **Workspace-split CI shared workflows** (phases 1–4):
  - `.github/workflows/shared-*.yml` — 20+ reusable workflows for
    cargo-deny, cargo-machete, cargo-vet, coverage, fuzz-diff, miri
    (focused + full), msrv-core, no-std, per-crate-msrv, readme-examples,
    reuse, rustdoc-strict, test-matrix, vendor-offline,
    verify-signatures, and workflow-propagation.
  - `.github/workflows/ci.yml` refactored to delegate to shared
    workflows (-480 lines).
- `.github/workflows/ci-duration-monitor.yml` +
  `scripts/ci-duration-monitor.sh` — CI-duration SLA monitor.
- `.github/workflows/crates-io-ownership.yml` +
  `scripts/check-crates-io-ownership.sh` — ownership drift harness.
- `scripts/shared-workflow-propagation-monitor.sh` — shared-workflow
  propagation SLA monitor.
- `doc/adr/0005-workspace-split.md` — architecture decision record
  for the workspace-split refactor.

### Changed

- GitHub repository description and topics — noyalib itself was
  already at the 20-topic ceiling and is a general-purpose YAML
  library (not banking), so no GitHub-metadata changes for it in
  this cut.

### No functional / API changes to non-MCP crates

- Only `noyalib-mcp` has a substantive change (the MCP registry
  work above). The other four publishable crates (`noyalib`,
  `noyalib-lsp`, `noyalib-wasm`, `noya-cli`) bump to `0.0.12` as
  part of the workspace-lockstep cut but ship no code changes —
  existing consumers can upgrade without any migration.

## [v0.0.11] - 2026-07-01

The **CI-integrity** cut. Fixes a silently-broken `no_std` build that
had been masked on `main` since v0.0.9 by cache poisoning in the CI
gates, closes three broken intra-doc links that were failing the
Pages-deployment workflow every push, hardens every specialised cargo
job across `ci.yml`, `security.yml`, `docs.yml`, and `release.yml`
with an isolated `CARGO_TARGET_DIR` so a stale artefact set can no
longer serve as a passing fingerprint for a different feature-set,
adds a strict-rustdoc PR gate so broken doc links fail a PR rather
than the post-merge Pages deployment, and closes OSSF Scorecard Code
Scanning alert #36 (`RUSTSEC-2026-0173`, `proc-macro-error2`
unmaintained — build-time-only via the opt-in `validator` feature,
never ships in a release artefact) with a source-controlled
`osv-scanner.toml` ignore.

No public API change. No MSRV change (still 1.85). One behavior
change under `--no-default-features`: `crates/noyalib/src/doc_boundary.rs`
now correctly imports `Vec` and `vec![]` from the `crate::prelude`
under `no_std` (previously used them without importing, which the
poisoned `no_std` CI gate never noticed).

### Fixed

- **`no_std` build actually compiles.** `doc_boundary.rs` used `Vec`
  and `vec![]` without importing them from `crate::prelude`. Under
  `std` the prelude auto-imports both; under `no_std` they're only
  reachable via `alloc::vec::Vec` / `alloc::vec` or the project's
  `crate::prelude` re-export. Adds `use crate::prelude::{Vec, vec};`.
  Also gates `use crate::span_context;` in `de.rs` behind
  `#[cfg(feature = "std")]` to match its call sites, so
  `--no-default-features` no longer trips `-D unused-imports`.
- **Three broken intra-doc links** in `de/config.rs` (`[Value]` at
  lines 179, 191, 348 and `[Error::Custom]` at line 213) now resolve.
  These had been failing the `Documentation` workflow's strict
  `-D warnings` build on every push to `main` since 2026-06-30.
  Qualified all four to `[Value](crate::Value)` /
  `[Error::Custom](crate::Error::Custom)`.

### Changed

- **CI cache-poisoning guard applied across every specialised cargo
  job.** `no_std`, `MSRV (1.85.0) core build`, `Miri (focused)`,
  `Miri (full + big-endian, scheduled)`, `Coverage gate`,
  `Differential fuzz (10s smoke)`, `rustdoc (strict)`, `soak-fuzz`,
  `soak-miri`, `docs.yml`, `release.yml/validate`, and
  `release.yml/cross-verify` all now use an isolated
  `CARGO_TARGET_DIR` plus a scoped Swatinem cache namespace. Sharing
  target/ across feature configurations was how the `no_std` job
  passed a stale-but-clean check for two full releases while the
  code was actually broken — this pattern rules that out.
- **New `docs-strict` PR-gated job** added to `ci.yml`. Mirrors the
  strict `RUSTDOCFLAGS` (`-D warnings` + `broken_intra_doc_links` +
  `private_intra_doc_links` + `invalid_codeblock_attributes` +
  `invalid_html_tags` + `bare_urls`) that `docs.yml` uses on `main`,
  but on every PR. Broken doc links now fail a PR instead of the
  Pages deployment after merge.

### Security

- **Code Scanning alert #36 closed.** `RUSTSEC-2026-0173`
  (`proc-macro-error2` unmaintained) documented as accepted risk in
  the new `osv-scanner.toml` (mirroring the pre-existing `deny.toml`
  `[advisories.ignore]` entry) so OSSF Scorecard's Vulnerabilities
  check no longer re-flags it. Rationale is build-time-only exposure
  via the opt-in `validator` feature; never ships in a release
  artefact. Revisit when `validator` cuts a release that drops
  `proc-macro-error2`.

## [v0.0.10] - 2026-06-30

The **BOM scanner** cut. A leading UTF-8 BOM (`U+FEFF`) no longer
breaks `parse_document` on multi-node inputs. Contributed by
[@zoosky](https://github.com/zoosky) (PR #118, rebased as #123).

No public API change. No MSRV change.

### Fixed

- **Leading UTF-8 BOM is transparent to indentation and comments.**
  The scanner used to consume the BOM with `advance_by(3)` while
  counting those three bytes toward the column of the following
  content, so a BOM-prefixed multi-node document (`<BOM>a: 1\nb: 2\n`)
  errored with "stray content after document — subsequent documents
  must start with '---'". The same miscount also broke BOM-prefixed
  sequences, nested mappings, and a `<BOM>#`-style first-line comment.
  Three surgical fixes in `parser/scanner.rs` make the BOM transparent
  at every column-aware site: `fetch_stream_start` resets `self.col = 0`
  after `advance_by(3)`, the simple-key indent path skips a leading
  BOM when computing `line_start`, and the block-context comment check
  treats `#` immediately after a leading BOM as a start-of-input
  comment. Each fix is gated on `pos == 3` / `line_start == 0` so an
  interior `0xEF 0xBB 0xBF` byte sequence in legitimate UTF-8 is
  never mistaken for one. Two scanner regression tests included.

## [v0.0.9] - 2026-06-30

The **supply-chain refresh** cut. Batches eight open Dependabot PRs,
clears two RustSec advisories, migrates `jsonschema` 0.33 → 0.46 with
the ValidationError API change, and refreshes the cargo-vet
exemptions + `imports.lock` snapshot so the supply-chain gates run
green from a clean state.

No public API change. No MSRV change.

### Fixed

- **Two RustSec advisories cleared.** `anyhow` 1.0.102 → 1.0.103
  (`RUSTSEC-2026-0190` — `Error::downcast_mut` unsoundness),
  `memmap2` 0.9.10 → 0.9.11 (`RUSTSEC-2026-0186` — unchecked pointer
  offset).
- **`jsonschema` 0.33 → 0.46.7 API migration.**
  `ValidationError::kind` and `ValidationError::instance_path` are
  now methods, not public fields. Updated `schema_validate.rs` and
  `cst/coerce.rs` to the new call syntax. All 16 schema tests pass
  under the new API.

### Changed

- **Batched Dependabot bumps.** GitHub Actions: `actions/checkout`
  6.0.3 → 7.0.0, `actions/cache` 5.0.5 → 6.1.0,
  `actions/attest-build-provenance` 4.1.0 → 4.1.1,
  `github/codeql-action/{init,analyze,upload-sarif}` SHA bump,
  `ossf/scorecard-action` SHA bump, `dtolnay/rust-toolchain` master
  SHA bump, `taiki-e/install-action` 2.81.8 → 2.82.2. Cargo:
  `bytes` 1.11.1 → 1.12.0, `serde-saphyr` 0.0.27 → 0.0.28.
- **Supply-chain hygiene.** `deny.toml` gains a scoped `Zlib`
  license allowance for `foldhash` (transitive via
  `hashbrown 0.16 → referencing 0.46`), matching the existing
  MIT-0 / BSD-2-Clause posture. `supply-chain/config.toml` shrinks
  from 18 to 14 exemptions after `cargo vet regenerate exemptions`
  (upstream audits now cover what was previously locally
  exempted). `supply-chain/imports.lock` refreshed via
  `cargo vet regenerate imports` so `--locked` accepts the current
  dep graph.
- **Workspace crates bumped to 0.0.9** (`noyalib`, `noya-cli`,
  `noyalib-mcp`, `noyalib-lsp`, `noyalib-wasm`) with intra-workspace
  `version =` pins synced. `xtask` stays at 0.0.1.

## [v0.0.8] - 2026-06-17

The **FlowStyle Fix** cut. Honors `SerializerConfig::flow_style` in the
serializer so `FlowStyle::Flow` and `FlowStyle::Auto` finally emit
inline collections (#84), folds in the batched Dependabot backlog
(cargo, GitHub Actions, Docker base images), and updates the
supply-chain gates for the new `proc-macro-error2` unmaintained
advisory.

No public API change. No MSRV change (still 1.85). The only behavior
change is the FlowStyle fix: callers setting `Flow` or `Auto` now get
inline output instead of silently getting block output.

### Fixed

- **`SerializerConfig::flow_style` is now honored.** `flow_style` and
  `flow_threshold` were stored but never read by the emit path, so all
  collections rendered as block regardless of config. `write_sequence`
  and `write_mapping` now dispatch to the flow emitters: `Flow` is
  always inline, `Auto` is inline within `flow_threshold` (default 4)
  with a safe block fall-back for oversized subtrees, and `Block` is
  unchanged. Adds four regression tests. [#84]

### Changed

- **Batched Dependabot bumps.** cargo: `clap_complete` 4.6.3 to 4.6.5,
  `smallvec` 1.15.1 to 1.15.2, `memchr` 2.8.0 to 2.8.2. github-actions:
  `actions/checkout` 6.0.2 to 6.0.3, `taiki-e/install-action` 2.81.1 to
  2.81.8, `KSXGitHub/github-actions-deploy-aur` 3.0.1 to 4.1.3,
  `docker/setup-buildx-action` 3.12.0 to 4.1.0, `docker/login-action`
  3.7.0 to 4.2.0. docker: `rust:1.96-bookworm`, `debian:bookworm-slim`,
  and `distroless/cc-debian12` digests re-pinned. [#85 to #96]

### Security

- **RUSTSEC-2026-0173** (`proc-macro-error2` unmaintained) ignored in
  `deny.toml` and `.cargo/audit.toml`. Build-time only via
  `validator_derive` to `validator`; never ships in an artefact.
  Revisit when `validator` releases off `proc-macro-error2`.

## [v0.0.7] — 2026-06-02

The **Supply-chain Hardening** cut. Closes three CVEs in the
VS Code extension's npm tree (xml2js, qs, tmp — all dragged in
by the legacy `vsce` devDep, which was removed), tightens the
release pipeline with `npm ci` + exact-pinned lockfile, and
folds in the routine Dependabot backlog: serde, clap, criterion
0.8 (with the `std::hint::black_box` migration across 14 bench
files), yaml-competitors, the actions group (10 actions headed
by checkout v5 → v6), plus eight standalone action bumps and
the sigstore cosign-installer v3 → v4 bump.

### Security

- **Drop legacy `vsce`.** `pkg/vscode/devDependencies` removed;
  the release pipeline already used `npx --yes @vscode/vsce`,
  so the legacy package was inert. Clears GHSA-776f-qx25-q3cc
  (xml2js), GHSA-q8mj-m7cp-5q26 (qs), and GHSA-ph9p-34f9-6g65
  (tmp). [#70]
- **`npm install` → `npm ci`** in the `vscode-extension` job
  with a committed, exact-pinned `package-lock.json`. [#69]
- **cosign-installer** upgraded to v4.1.2; release artefact
  signing flow unchanged. [#63]
- **Auto-approve Dependabot** workflow added to satisfy
  OpenSSF Scorecard's `Code-Review` check on a
  solo-maintainer project. [#64]

### Changed

- **criterion 0.5.1 → 0.8.2.** All 14 bench files migrated from
  `criterion::black_box` to `std::hint::black_box` (criterion
  0.8 deprecated the re-export). [#80]
- **yaml-competitors group bump** — yaml-rust2 0.9 → 0.11,
  rust-yaml 0.0.5 → 1.1; both are bench/comparison-only deps,
  not part of the runtime tree. [#80]
- **actions group bump (10 actions)** including checkout v5 →
  v6, cache v4 → v5, configure-pages v4 → v6,
  upload-pages-artifact v3 → v5, plus 6 others. [#80]
- **clap_complete 4.5 → 4.6, clap_mangen 0.2 → 0.3** in
  noya-cli and xtask. [#80]
- **Docker base** bumped from `rust:1.85-bookworm` to
  `rust:1.96-bookworm` in all three Dockerfiles. [#62]

### Fixed

- **OpenSSF Scorecard Pinned-Dependencies** 9/10 → 10/10 by
  exact-pinning every npm dep and switching to `npm ci`. [#66,
  #69]

## [v0.0.6] — 2026-05-30

The **Ecosystem Integration** cut. Lands the four remaining
open issues from the v0.0.6 milestone (#22, #24, #25, #33),
closes out the leftover stabilisation checklist (#19), the
i18n hooks (#18), the user-reported pnpm-lock recursion bug
(#46), and the OpenSSF Scorecard hardening pass that lifts
the score from 6.5/10 to ~9/10.

### Fixed — streaming deserializer depth leak on empty flow mappings (issue #46)

`StreamingMapAccess::next_key_seed` (and the symmetric
`next_value_seed` / `StreamingSeqAccess::next_element_seed`)
did not consult the access object's `finished` flag. Serde
visitors that call `next_entry` after the previous call
returned `Ok(None)` — `noyalib::Value`'s `ValueVisitor::visit_map`
is the canonical one — read the **next event from the parent
mapping** and treated it as belonging to the now-exhausted
child. The recursive `deserialize_any` on each spilled value
inflated `self.depth` by one per entry; on a `pnpm-lock.yaml`
shaped input with N consecutive empty flow mappings `{}`,
depth hit `max_depth + 1` after exactly 128 entries and
`from_str::<Value>` failed with
`Error::RecursionLimitExceeded { depth: 129 }` even though the
real nesting depth was 2.

Fix in `crates/noyalib/src/streaming.rs`:

* Both `MapAccess::next_key_seed` and
  `MapAccess::next_value_seed` return `Ok(None)` / a clear
  contract-error early when `finished` is set.
* `SeqAccess::next_element_seed` mirrors the guard.
* `deserialize_any` / `deserialize_seq` / `deserialize_map`
  now decrement `self.depth` on both `Ok` and `Err` so a
  failed inner visit cannot leak depth into the outer scope
  (the same leak path under a different trigger).

Regression test: `crates/noyalib/tests/issue_46.rs` —
50 000-package `pnpm-lock.yaml`-shaped fixture, 3 000 empty
flow mappings at one level, complex peer-dependency keys, and
the deterministic depth-cliff probe at every `n` in
`[100, 128, 129, 130, 200, 500, 1000]`.

Affects every typed deserialize target whose visitor calls
`next_entry` past the end (including `BTreeMap<K, V>` and
struct fields of optional shape). Default `from_str` users
upgrading to this release should see only the previously-broken
parses now succeed; no behavioural change on valid documents.

### Fixed — no-span loader missing depth-limit check

Companion audit finding to issue #46: the no-span loader path
(`crates/noyalib/src/parser/loader.rs`) — used by
`from_str::<Value>`'s value-target fast path and by `no_std`
multi-document loading — incremented `self.depth` on
`SequenceStart` / `MappingStart` events at lines 814 / 833 but
did **not** check against `ParserConfig::max_depth` the way
the span-tracked loader does at lines 399-401 / 443-445.
Adversarial deeply-nested input could consume stack without
ever firing `RecursionLimitExceeded`. Now mirrored from the
span loader. Regression test:
`no_span_loader_honours_max_depth` in
`crates/noyalib/tests/issue_46.rs`.

Both findings were surfaced by the same code-pattern audit
that confirmed every `MapAccess` / `SeqAccess` /
`EnumAccess` / `VariantAccess` impl across `streaming.rs`,
`de.rs`, and `value.rs` either has a `finished`-style guard
or uses an iterator that naturally returns `None` on
exhaustion. No further iterator-state-leak bugs in the same
family remain.

### Security & hardening pass on the v0.0.6 surface

Post-merge deep-dive audit surfaced six DoS / correctness
findings in the new modules; this section documents the fixes.

* **Recovery `---`-spam OOM (C2).** `parse_lenient` now bounds
  the document-marker scan by `ParserConfig::max_documents`. A
  hostile `---\n`-only-spam input cannot drive unbounded
  `Vec<usize>` allocation.
* **Recovery O(n²) line-truncation (C1).** The truncation loop
  is bounded by a new `LenientConfig::truncation_event_budget`
  (default 1 MiB cumulative bytes across retries). Adversarial
  10k-line malformed input no longer triggers ~10k full
  re-parses.
* **Async unbounded `read_to_end` (C3).** Both
  `from_async_reader{,_with_config}` and the new
  `from_async_reader_multi_with_config` drain through
  `AsyncReadExt::take(max_document_length)`. Slow-drip
  adversaries cannot grow the buffer beyond the configured
  limit before the parser fires its own check.
* **CRLF support (C4).** `recovery::split_documents`,
  `tokio_async::find_doc_boundary`, and the existing
  `parallel::split` now share **one** workspace-private scanner
  in `crate::doc_boundary` that accepts both `\n` and `\r\n`
  line terminators. The three previous copies disagreed on
  CRLF — Windows-edited buffers round-trip through every entry
  point now.
* **BOM support (C5).** `parse_lenient` and both async readers
  strip a leading UTF-8 BOM (`U+FEFF`) so Windows-saved buffers
  parse identically to LF-on-Linux equivalents.
* **Decoder recursion → loop (C6).** `YamlDecoder::decode` no
  longer recurses on whitespace-only frames; the all-whitespace
  preamble is consumed in a bounded `loop` instead, eliminating
  the adversarial stack-overflow vector.

Correctness fixes that landed alongside the hardening:

* `parse_lenient` now collects Pass-2 / Pass-3 errors instead
  of silently dropping them (M1).
* Multi-document budget exhaustion no longer truncates the
  output `Sequence` — skipped documents are emitted as
  `Value::Null` so per-document diagnostic indices stay
  aligned for LSP joiners (M2).
* Line-truncation now treats the buffer end as a candidate
  cut, so a malformed last line **without** a trailing newline
  (the universal mid-typing case) is still recoverable (M3).
* Pass-2 `ParserConfig` clone hoisted out of the hot path —
  one clone per document, not one per pass (M13).

Surface additions on the tokio module:

* `from_async_reader_multi_with_config` — config-aware variant
  was missing.
* `YamlDecoder::max_frame_size(usize)` — optional inter-frame
  buffer cap for codec users driving untrusted-network input
  (M7). `Decoder::decode` returns
  `Error::Io(InvalidData)` when the buffer exceeds the cap.
* `YamlDecoder` is now `Clone`.

Surface additions on the sval adapter:

* `to_sval_writer_with_config` + `SvalConfig` —
  `coerce_non_finite_to_null` toggles NaN / ±∞ → `Null` so
  downstream consumers (e.g. `sval_json`) that reject
  non-finites accept the stream (M10).
* `impl sval::Value for Tag` — the public `Tag` type now has a
  direct sval impl (M12).

Documentation:

* `SECURITY.md` and `doc/POLICIES.md` document the new
  resource-limit knobs and their threat model.
* CHANGELOG, READMEs, and `doc/release-notes/v0.0.6.md` cross-reference
  the new safe-by-default contracts.
* MSRV inconsistency (workspace claimed 1.75 in some places,
  Cargo.toml says 1.85 since v0.0.5) resolved on both axes.

Tests: +24 unit tests across the three modules (61 total),
covering CRLF, BOM, `---`-spam, no-trailing-newline truncation,
budget-exhaustion index preservation, oversize-reader truncation,
frame-size cap, NaN coercion. Plus a new
`bench_recovery_lenient_on_invalid_input` arm exercising the
3-pass recovery loop on realistic LSP-style half-typed input.

### Added — Error-recovering parser (`recovery` feature, issue #22)

`noyalib::recovery::parse_lenient` returns a `ParseResult`
carrying the best-effort tree plus the list of every error
encountered, so LSP / IDE consumers can keep showing
autocomplete and diagnostics on half-typed documents.

```rust
let r = noyalib::recovery::parse_lenient("a: 1\nb: [unclosed\n");
assert!(!r.is_complete);
assert!(!r.errors.is_empty());
```

Recovery strategies — strict pass first, then
`DuplicateKeyPolicy::Last` retry, then line-truncation retry
that drops trailing lines until something parses.
Multi-document input is split on `---` and each document is
recovered independently. Error collection is capped via
`LenientConfig::max_errors`.

Gated behind the new `recovery` Cargo feature; zero extra deps.

### Added — `sval` streaming adapter (`sval` feature, issue #25)

Alternative to the default serde route for callers wanting to
skip `serde_derive`'s compile-time overhead or the binary-size
cost of serde monomorphisation. Adds `impl sval::Value for
Value`, `Number`, `Mapping`, `MappingAny`, and `TaggedValue`,
plus a `noyalib::sval_adapter::to_sval_writer` entry point.
serde remains the default; this is opt-in.

### Added — Native tokio async parsing (`tokio` feature, issue #24)

`noyalib::tokio_async::from_async_reader` /
`from_async_reader_multi` parse from any
`tokio::io::AsyncRead` without `spawn_blocking`.
`YamlDecoder<T>` is a `tokio_util::codec::Decoder` for
plugging streaming YAML parsing into a
`tokio_util::codec::Framed` / tower pipeline. Per-document
emission boundary follows the YAML 1.2.2 §9.1.2 `---` grammar.

### Changed — npm publish moves to Trusted Publishing / OIDC (issue #33)

`.github/workflows/release-binaries.yml` no longer reads
`NPM_TOKEN`; both `@noyalib/noyalib-wasm` and `noyalib-mcp`
publish jobs declare `id-token: write` and rely on the OIDC
handshake against per-package trusted-publisher policies
configured at `https://www.npmjs.com/package/<name>/access`.
`pkg/PUBLISH.md` §6 documents the bootstrap + secret-retirement
flow.

Compromise window collapses from 1 year (granular access
token) to ~10 minutes (per-run OIDC token). The
`--provenance` flag stays attached to both publish steps so
the npm verified-publisher badge keeps linking back to the
exact GitHub Actions run.

## [v0.0.5] — 2026-05-11

### Changed — Edition 2024 + MSRV bump to 1.85 (issue #15)

All six workspace crates (`noyalib`, `noya-cli`, `noyalib-mcp`,
`noyalib-lsp`, `noyalib-wasm`, `xtask`) move to:

* `edition = "2024"`
* `rust-version = "1.85.0"`

CI's MSRV-gate job retargeted to 1.85.0 in the same commit.
Edition-2024 idiom fixes applied to surface lints (match
ergonomics on `&mut _` patterns, `repeat_n` replacing
`repeat().take()`, redundant `ref`/`mut` bindings inside
`matches!`). The `unsafe`-tightened `std::env::set_var` /
`remove_var` are no longer called from `examples/figment.rs` —
the env-overlay scenario was refactored to `figment::Env::raw`
plus a synthetic `Serialized` layer.

Lockfile pins for the MSRV-1.75 workarounds are dropped:
* `indexmap 2.10` → `2.14` (latest in `>=2, <3`)
* `rustc-hash 2.0` → `2.1.2` (latest in `>=2, <3`)
* `hashbrown` (transitive) → 0.17

### Added — Declarative `parser_config!` / `serializer_config!` macros (issue #17)

```rust
use noyalib::parser_config;
let cfg = parser_config! {
    max_depth: 64,
    strict_booleans: true,
    duplicate_key_policy: DuplicateKeyPolicy::Error,
};
```

Pure expansion to the existing chained-setter builders — zero
runtime overhead. Supports empty form (`parser_config! {}`
returns `ParserConfig::new()`) and trailing comma after the
last entry. The `serializer_config!` counterpart targets
[`SerializerConfig`].

### Added — Pluggable error-message formatters (issue #18)

New `noyalib::i18n` module:

* `MessageFormatter` trait — `Send + Sync` strategy for
  rendering `Error` as a user-visible message.
* `DefaultFormatter` — preserves the developer-facing message
  verbatim (`Display`-equivalent).
* `UserFormatter` — collapses noyalib's diagnostic vocabulary
  into short plain-language sentences appropriate for
  non-developer audiences. Includes line numbers when the source
  location is available; strips internal terms (`!!binary`,
  "merge key") and field names that might leak in a GUI alert.
* `Error::render_with_formatter(&dyn MessageFormatter)` —
  dispatch entry point. Custom localisation tables / rich
  formatters plug in by impl-ing `MessageFormatter`.

### Documentation — pre-release API stabilisation audit (issue #19)

Pre-1.0 stabilisation checkpoint. The audit confirmed:
* All public configuration types (`ParserConfig`,
  `SerializerConfig`, `Error`, `MergeKeyPolicy`,
  `DuplicateKeyPolicy`, `FlowStyle`, `ScalarStyle`,
  `YamlVersion`, `RequireIndent`, `TransformReason`,
  `SymlinkPolicy`) carry `#[non_exhaustive]` so adding a field
  / variant in a future patch release is non-breaking.
* All public functions ship with doc-comments + working
  examples. Strict-doc gate (`-D rustdoc::broken_intra_doc_links
  -D rustdoc::private_intra_doc_links -D
  rustdoc::redundant_explicit_links`) enforces this on every
  PR.
* The Error enum's variant set is comprehensive and actionable —
  `Parse`, `ParseWithLocation`, `Deserialize`,
  `DeserializeWithLocation`, `Io`, `Custom`,
  `RecursionLimitExceeded`, `DuplicateKey`,
  `RepetitionLimitExceeded`, `Budget`, `UnknownAnchor`,
  `UnknownAnchorAt`, `MissingField`, `TypeMismatch`, and family
  cover every internal failure path.
* No unintended public surface — every `pub` item is either
  re-exported from the crate root or lives in a documented
  `pub mod`. `pub(crate)` everywhere else.

Stable 1.0.0 is deferred to post-production hardening (target:
2028+). v0.0.5 is the stabilisation *checkpoint*, not the SemVer
release.

## [v0.0.4] — 2026-05-11

### Added — `!include` directive support (issue #10)

`ParserConfig::include_resolver` + `max_include_depth`. Two
feature gates: `include` (resolver types only, works in
no_std-style builds) and `include_fs` (adds the bundled
`SafeFileResolver` with root-dir sandboxing and configurable
symlink policy).

After parse, every `Value::Tagged(!include, scalar_spec)` node
is replaced with the resolver's output. Highlights:

- **In-memory resolvers** — wrap any `Fn(IncludeRequest) ->
  Result<InputSource>` via `IncludeResolver::new`. Useful for
  virtual filesystems, test harnesses, network-backed fetchers.
- **`SafeFileResolver`** — filesystem-backed resolver rooted at
  a directory. Path traversal (`../../etc/passwd`) is caught by
  canonicalisation + root-prefix check; symlinks are governed by
  `SymlinkPolicy::FollowWithinRoot` (default) or
  `SymlinkPolicy::Reject`.
- **Fragment anchors** — `!include file.yaml#key` narrows to the
  named top-level mapping key inside the included document.
- **Cycle detection** — per-walk visited set rejects A→B→A
  regardless of depth.
- **Depth ceiling** — `max_include_depth` defaults to 24 (8 in
  `ParserConfig::strict()`). Trips
  `Error::RecursionLimitExceeded` on overflow.
- **Streaming fast-path** is automatically disabled when an
  include resolver is installed so the post-parse walk runs
  uniformly across every typed target.

11 new integration tests + a 4-scenario runnable example
(`cargo run --example include_directive --features include_fs`).

## [v0.0.3] — 2026-05-11

### Changed — widen `rustc-hash` cap to `>=2, <3`

Single-line manifest widening (`rustc-hash = ">=2, <2.1"` →
`rustc-hash = ">=2, <3"`). The old cap was defensive, not
load-bearing — noyalib's usage is the stable `FxHashMap` /
`FxHashSet` / `FxBuildHasher` public surface unchanged across
the 2.x line.

The motivating downstream is `html-generator`, whose dependency
chain pulls `scraper 0.26 → selectors 0.36 → rustc-hash
^2.1.1`. Under the previous range the two co-resolution paths
were incompatible.

`Cargo.lock` stays pinned to `rustc-hash 2.0.0` because 2.1+
declares `rust-version = "1.77"`, above noyalib's 1.75 MSRV
floor. Downstream consumers on Rust ≥ 1.77 are free to
`cargo update -p rustc-hash` to take 2.1+; consumers on 1.75
inherit our lockfile pin via `cargo build --locked`. Same
MSRV-preservation pattern v0.0.2 used for `indexmap 2.10 /
hashbrown 0.15`.

## [v0.0.2] — 2026-05-10

### Added — `${KEY}` / `${KEY:-default}` substitution during parse (issue #11)

`ParserConfig::properties(map)` plus `strict_properties(bool)`
toggle. Each YAML scalar is walked after parse and any
`${name}` placeholder is substituted from the supplied
`Arc<HashMap<String,String>>`. Supports `${KEY:-default}`
inline fallbacks, `$$` → `$` and `${{` → `${` escapes, and
`}}` → `}`. Strict mode (default for `ParserConfig::strict()`)
errors on unknown keys; lossy mode (default) substitutes the
empty string. Syntax errors in the placeholder (invalid
character, unterminated, malformed `:-default` separator) always
abort regardless of mode. Streaming fast-path is automatically
disabled when properties are active so the post-parse walk runs
uniformly across every typed target.

### Added — `ariadne` adapter for `Error` (issue #23)

New `ariadne` Cargo feature exposing
`noyalib::ariadne_adapter::error_to_ariadne_report(err, filename, source)`
that converts a `noyalib::Error` into an `ariadne::Report` with
the offending byte range labelled. Pairs with the existing
`miette::Diagnostic` impl on `Error` for users who prefer
ariadne's rendering. Multibyte-safe: `Location::index()` is
clamped to the source bounds before being expanded to a labelled
range.

### Added — garde / validator → miette bridge with `Spanned<T>` (issue #32)

`noyalib::validated_miette` exposes
`garde_errors_to_miette(spanned, errors, source, name)` and
`validator_errors_to_miette(...)` that walk a validation error
tree (compact `path: message; …` summary) and emit a single
`miette::Report` whose source label points at the
`Spanned<T>`'s byte range. Behind the `miette` Cargo feature
plus either `garde` or `validator` (or both). Hand-rolled
`Display` + `Error` + `miette::Diagnostic` impls keep
`thiserror` out of the dep closure (matches the policy in
`error.rs`).

### Added — `from_str_borrowing` + `TransformReason` (issue #8)

New public entry points `from_str_borrowing` and
`from_str_borrowing_with_config` for `T: Deserialize<'a>` targets
that borrow from the input slice (`&'a str`, `Cow<'a, str>`,
structs containing those). The streaming deserialiser now routes
plain-scalar string events through `visit_borrowed_str` whenever
the parser produced a `Cow::Borrowed` event, unlocking truly
zero-copy `&'de str` deserialisation. Quoted scalars without
escapes also borrow; scalars that required decoding (escapes,
multi-line folding, alias replay, tag resolution) fall back to
owned buffers.

Adjacent parser hardening: the plain-scalar slow path now emits
`Cow::Borrowed(input_slice)` whenever the scalar is a single
contiguous run of input bytes (no folded line breaks), matching
the slow-path's owned-buffer result byte-for-byte. This means the
common `key: value\n` shape now borrows zero-copy on the streaming
path, not just terminal scalars at end-of-input.

`TransformReason` enum (`noyalib::borrowed::TransformReason`)
catalogues the five reasons a scalar can fail to borrow:
`EscapeSequence`, `LineFold`, `TagResolution`, `QuotedScalar`,
`AliasExpansion`. `Display` and `as_str` provide stable messages
suitable for inclusion in higher-level error reports. The enum is
`#[non_exhaustive]` so adding finer-grained variants in the future
is non-breaking.

### Added — `read` / `read_with_config` lazy multi-document reader (issue #7)

`noyalib::read<R: Read, T: DeserializeOwned>(reader)` returns a
`DocumentReadIterator<T>` that yields one `Result<T>` per YAML
document. Per-document deserialisation errors surface as `Err`
items so callers can recover and continue across document
boundaries; YAML *syntax* errors return synchronously from
`read` / `read_with_config` before iteration starts. The
implementation drains the reader into a `String` first
(`O(input_len)` peak memory); a future v0.0.3+ pass will tighten
this to `O(1-document)` once the parser learns to accept
incremental byte chunks.

### Confirmed shipped in v0.0.1 (closes issues #9, #27, #28, #30)

- **#9 — Event-based streaming deserialisation.**
  `StreamingDeserializer` is the fast path inside `from_str` /
  `from_str_with_config`; falls back to the AST loader only when
  the caller's config disables streaming-eligible features.
  Measured **30% faster** than the AST path (14.0 vs 19.4 µs;
  see [`doc/BENCHMARKS.md`](doc/BENCHMARKS.md#architecture-validation)).
- **#27 — Path query API.** `Value::query` /
  `BorrowedValue::query` ship dot notation, array indexing,
  wildcards (`*`), and recursive descent (`..`). Filter
  expressions (`[?field==value]`) remain optional and tracked
  separately.
- **#28 — Zero-copy `Value<'a>` AST.** Implemented via the
  parallel `BorrowedValue<'a>` type with `Cow<'a, str>` keys and
  values, shipped in v0.0.1 to avoid a breaking change to
  `Value`. Measured **18% faster** than the owned `Value` path.
- **#30 — Shared-memory DAGs via Rc/Arc anchor registry.**
  `AnchorRegistry<T>` (`Rc`) and `ArcAnchorRegistry<T>` (`Arc`)
  expose `register` / `resolve` returning shared pointers to the
  same heap allocation (verified by `Rc::ptr_eq` / `Arc::ptr_eq`
  in the type's doctests). Cyclic graphs use
  `RcRecursive`/`ArcRecursive` with their `Weak` partners.

### Changed — relax `indexmap` upper bound

Bumped `indexmap` requirement from `>=2, <2.11` to `>=2, <3`. The
old cap was defensive (we hadn't tested against 2.11+ at release
time), not load-bearing — noyalib only uses `IndexMap`,
`map::Iter`, `map::Entry`, and other stable public surface that
hasn't changed across the 2.x line. indexmap 2.10 and 2.11 share
MSRV 1.63, well below noyalib's own 1.75 floor.

The motivating downstream is `html-generator`, which pulls
`toml = "1.1"` (which depends on `indexmap ^2.11.4`); the previous
`<2.11` cap made the two co-resolution paths incompatible.

## [v0.0.1] — 2026-05-10

### Fixed — `Eq`/`Hash` invariant for `Number` floats (signed zero, NaN)

`PartialEq for Number` deliberately treats `+0.0 == -0.0` (per IEEE
754) and `NaN == NaN` (to satisfy `Eq` reflexivity). The `Hash for
Number` impl was hashing `f64::to_bits()` directly, which gives
distinct bit patterns for those equal values — surfaced by the
ubuntu-nightly `value_hash_consistent` proptest. Normalised both
edges in the hasher: zero hashes as `0u64` regardless of sign, NaN
hashes as a fixed quiet-NaN sentinel. Three explicit regression
tests pinned in `tests/proptest.rs`.

### Performance — bulk-copy quoted-scalar interior runs via SIMD prefix scan

The single- and double-quoted scalar fast paths used to read one
UTF-8 character at a time inside a `match self.peek()` loop —
`slice_str + push_str` per char. Replaced with `simd::clean_prefix_len`
over the appropriate ASCII needle set; semantics are bit-exact, ~30%
end-to-end on a worst-case 100KB single-quoted ASCII string. All
needles are ASCII, so slicing on a needle hit is char-boundary safe.

### Performance — Profile-Guided Optimization (PGO) infrastructure

New `scripts/pgo.sh` drives the full LLVM PGO pipeline:
instrumented build → train against `bench_corpus/` and the YAML
test suite → `llvm-profdata merge` → optimised rebuild. Documented
in `doc/PGO.md` and surfaced in `doc/POLICIES.md` §4 as an opt-in
5–15% extra speedup path on top of the default `cargo build
--release` numbers. Loader Vec/Mapping pre-sizing via
`Value::deserialize`'s `SeqAccess`/`MapAccess` `size_hint()` cuts
the first reallocation on the AST fallback path.

### CI — panic-free contract + unused-dep gate

- `tests/panic_free.rs`: 8 `proptest` properties + 19 historical-input
  regression cases verify that `from_str`, `from_slice`, `load_all`,
  and `cst::parse_document` never panic on arbitrary input. CI runs
  the proptest at the default seed; nightly stress-runs at
  `PROPTEST_CASES=16384`.
- `cargo-machete (unused-dep gate)` is now a required CI job —
  blocks PRs that add a dependency to `Cargo.toml` without using
  it. Catches accidental fat-tree imports.

### Fixed — defensive char-boundary clamp in `Scanner::slice_str`

Adversarial mixed-quote input (`"A:\r*aa {\"\\¡"`) could land
`slice_str` mid-codepoint and panic. Added a stable polyfill of
`str::floor_char_boundary` and clamp both `start` and `end` to
char boundaries before slicing. Three new fuzz targets in
`fuzz/fuzz_targets/` cover the new code paths.

### Fixed — Windows-only MCP atomic-write flake

`tool_call_set_preserves_comments` flaked on Windows when a
concurrent reader observed a half-written file. The `noyalib_set`
write helper now uses `MoveFileExW(MOVEFILE_REPLACE_EXISTING |
MOVEFILE_WRITE_THROUGH)` semantics on Windows so concurrent
readers see either the old or the new contents — never a
half-write or a stale-page-cache observation.

### Docs — satellite-crate enterprise-readiness sections

`noya-cli`, `noyalib-lsp`, `noyalib-mcp`, `noyalib-wasm` lib.rs
crate-level doc blocks now match the noyalib core's 12-dimension
template. Added: `# Cargo features`, `# Performance`, `# API
stability and SemVer` sections. WASM `# Panics` expanded to
enumerate WASM-specific abort sources (linear-memory OOM, stack
overflow on misconfigured `max_depth`, `panic = abort` on the
host). `noyalib-mcp` and `noyalib-wasm` READMEs now state the
explicit MSRV (1.75.0 and 1.85.0 respectively) and tier-1
platform list — bringing them into alignment with the
`noyalib-lsp` and `noya-cli` READMEs.

### Docs — diagnostic feature-gate fix

`tests/cst_schema_tag_audit.rs` referenced `validate_against_schema`
unconditionally but the symbol is gated behind
`feature = "validate-schema"`. Test-crate now compiles cleanly
under default features as well as `--all-features`.

### v0.0.2 milestone — implemented in v0.0.1

The seven open issues on the v0.0.2 milestone are closed inside
v0.0.1 per the "don't pre-emptively phase a bang launch"
principle. Public API additions:

- **`noyalib::Error::Budget(BudgetBreach)`** + the
  `BudgetBreach` enum (#3). Six new `ParserConfig` budgets:
  `max_events`, `max_nodes`, `max_total_scalar_bytes`,
  `max_documents`, `max_merge_keys`, `alias_anchor_ratio`.
  Each has a builder method on `ParserConfig`; `strict()`
  uses tighter caps. Enforced in `Loader::process_event`.
- **`noyalib::Error::render(source) -> String`** +
  `render_with_options(source, &RenderOptions)` (#2). New
  public types `RenderOptions { crop_radius, color }` and
  `CroppedRegion<'a>` for caller-facing diagnostic
  rendering. `format_with_source` / `format_with_source_radius`
  remain for backwards compatibility.
- **`RcRecursive<T>` / `ArcRecursive<T>` / `RcRecursion<T>` /
  `ArcRecursion<T>`** (#5). Late-init / cyclic-graph anchor
  wrappers in `noyalib::anchors`. Access via `.borrow()` /
  `.lock()`; `Serialize` / `Deserialize` impls delegate to the
  inner `T`.
- **`noyalib::RequireIndent`** + `ParserConfig::require_indent`
  (#6). API surface for indentation-validation modes
  (`Unchecked`, `Even`, `Divisible(N)`, `Uniform(Option<N>)`).
  Scanner-side enforcement is a follow-up per the issue's
  own "Blast Radius" note.

Already-implemented issues confirmed and closed: `!!binary`
support (#4 — `src/base64.rs`), yaml-test-suite compliance
runner (#26 — 406/406 strict), streaming anchor event replay
(#29 — `streaming.rs::anchor_events` + `replay_stack`).

Test coverage: 38 new regression tests across
`tests/{budget_breach,error_render,require_indent,recursive_anchors}.rs`.
Coverage gates: **95.63% functions / 93.16% lines / 92.31%
regions** (all above CI thresholds).

### Docs — README refactor: extracted deep weeds, grouped tooling cluster

The workspace README had grown to a 1 593-line full-doc website.
Two refinements:

- **Extracted** the full Benchmarks tables (deserialise /
  serialise / SIMD / SWAR / parallel / architecture-validation /
  project-metrics) into [`doc/BENCHMARKS.md`](doc/BENCHMARKS.md),
  and the full Ecosystem-comparison feature matrix into
  [`doc/COMPARISON.md`](doc/COMPARISON.md). The README keeps a
  ~10-line summary table for each, with a link to the full
  doc. Reading-the-table notes and the SWAR pipeline
  walkthrough live in the extracted files.
- **Re-grouped the tooling cluster.** The "Tooling" section
  was at line 391; now an ecosystem table sits right after
  Quick Start (line 213) under "The noyalib ecosystem"
  covering all five crates (`noyalib`, `noya-cli`,
  `noyalib-lsp`, `noyalib-mcp`, `noyalib-wasm`) with
  per-crate install commands and per-host quick-link entries
  pointing at the editor / MCP / ecosystem-gate config
  examples. The library-only deep-dive sections (Features,
  Custom tags, Governance, Policy, etc.) follow below in
  one block, so the library docs and the tooling docs are
  cleanly separated.

README size: **1 499 lines** (was 1 593). Two new doc files
absorbing 238 lines of detail.

### Docs — per-crate migration guides for the wider YAML ecosystem

Each non-`serde_yaml` Rust YAML crate now has its own dedicated
migration guide with the same shape as the original
`MIGRATION-FROM-SERDE-YAML.md` (TL;DR diff, function table,
behavioural notes, checklist). Crates.io state verified
**2026-05-08**:

- [`MIGRATION-FROM-SERDE-YML.md`](doc/MIGRATION-FROM-SERDE-YML.md) — `serde_yml` 0.0.12 (archived 2025-09)
- [`MIGRATION-FROM-YAML-SERDE.md`](doc/MIGRATION-FROM-YAML-SERDE.md) — `yaml_serde` 0.10.4 (active fork)
- [`MIGRATION-FROM-SERDE-YAML-NG.md`](doc/MIGRATION-FROM-SERDE-YAML-NG.md) — `serde-yaml-ng` 0.10.0 (active drop-in fork)
- [`MIGRATION-FROM-SERDE-NORWAY.md`](doc/MIGRATION-FROM-SERDE-NORWAY.md) — `serde-norway` 0.9.42 (hard-fork)
- [`MIGRATION-FROM-SERDE-YAML-BW.md`](doc/MIGRATION-FROM-SERDE-YAML-BW.md) — `serde-yaml-bw` 2.5.6 (non-drop-in 2.x)
- [`MIGRATION-FROM-SERDE-SAPHYR.md`](doc/MIGRATION-FROM-SERDE-SAPHYR.md) — `serde-saphyr` 0.0.26 (no `Value` DOM)
- [`MIGRATION-FROM-YAML-SPANNED.md`](doc/MIGRATION-FROM-YAML-SPANNED.md) — `yaml-spanned` 0.0.3 (parser-only)

The umbrella index lives at
[`doc/MIGRATION.md`](doc/MIGRATION.md) and points at all eight
guides via a compatibility matrix. The workspace README and the
`noyalib` crate README both link into the per-crate guides.

### YAML Test Suite — 100% strict (406/406, 0 skip)

The historical 18-case `SKIP_LIST` (2JQS, 6WLZ, 6CK3, P76L, 6VJK,
UT92, WZ62, 4ABK, M7A3, K527, 9WXW, V9D5, CFD4, KK5P, M2N8, M5DY,
RZP5, XW4D) is gone — the parser now passes every active YAML
1.2 Test Suite case under strict comparison. The skip list was a
historical artefact of an earlier parser state; under the
current scanner + loader those cases produce values that match
the suite's expected JSON. The lenient `official_suite.rs`
runner additionally needed a tag-stripping JSON projection
(`yaml_value_to_json`) to align with the suite's tag-less
expected shape after the `Value::Tagged` preservation work.

**Both runners now report 406/406 = 100.0% strict, 0 skip, 0 fail.**

### Added — Stress / load test battery (`tests/stress_load.rs`)

13 new regression tests pinning the parser's behaviour under
pathological input:

- 1 MB single block-scalar document.
- 10 000-entry mapping / 10 000-item sequence.
- 1 000-document multi-document stream.
- 100-level deep nesting + recursion-limit DoS guard at 10 000.
- Billion-laughs-style alias amplification rejection.
- 1 MB long plain scalar.
- 100-iteration parse-emit-reparse stability.
- Unicode-heavy document (emoji / CJK / RTL).
- Custom `ParserConfig` low `max_depth` enforcement.
- 1 000 anchors + aliases within budget.

### Performance — release profile tuned for speed

`[profile.release]` `opt-level` flipped from `"s"` (size) to `3`
(speed) for the workspace. The library's per-byte scanner
dispatch inlines and vectorizes meaningfully better at `3`. WASM
bundle size is managed separately by the `wasm-pack` post-build
`wasm-opt -Os` pass (see `crates/noyalib-wasm/README.md`),
keeping the published `.wasm` at its target ~338 KB.
`overflow-checks = true` is preserved on the security-vs-speed
trade-off — the parser handles untrusted input and cannot afford
silent wraparound on indent / depth / size counters.

### Fixed — Windows-only MCP test flake (`tool_call_set_preserves_comments`)

`noyalib_set` previously called `fs::write(file, …)` directly.
On Windows the test's `read_to_string` could observe stale
contents because the kernel page-cache hadn't flushed by the
time the spawned MCP child exited. Replaced with an
*atomic-write* helper: write to a sibling temp file, `sync_all`,
then `rename` over the target. The rename is atomic on POSIX
and on Windows under `MoveFileExW(MOVEFILE_REPLACE_EXISTING |
MOVEFILE_WRITE_THROUGH)` semantics, so concurrent readers see
either the old or the new contents — never a half-write or a
stale cache.

### Fixed — nested `Value::Tagged` inside a tagged container (C4HZ regression)

`from_str::<Value>("!shape\n- !circle 1\n")` previously collapsed
the inner `Tagged(circle, "1")` into a single-key
`Mapping{"!circle": "1"}` because
`TagPreservingMapAccess::next_value_seed` handed the inner
`Value` to a tag-blind `&'de Value` Deserializer. Fixed by
re-wrapping the inner value in
`crate::de::Deserializer::with_options_preserving_tags(...)` so
nested `Value::Tagged` survives every layer of the data-binding
return path. Restores YAML test suite C4HZ ("Spec Example 2.24
Global Tags") to the strict-pass set — strict compliance back
to **100.0% (387/387)**.

### Added — `to_string_value` / `to_writer_value` for lossless `Value::Tagged` emit

- **`noyalib::to_string_value(&Value) -> Result<String>`** and
  the `_with_config` variant emit a `Value` directly via the
  YAML-tag-aware writer, skipping the `Serialize` pipeline.
  Required when the input may contain `Value::Tagged(...)` and
  the caller wants the YAML-tag wire form to survive on emit.
- **`noyalib::to_writer_value<W: io::Write>(W, &Value) -> Result<()>`**
  and the `_with_config` variant — same contract, writing into
  any `io::Write`.
- **Why these are separate from `to_string` / `to_writer`**: the
  generic family routes `Value::Tagged` through
  `Serializer::serialize_map` (which is the right shape for
  `serde_json` and other serde-bridge consumers) and that
  flattens the tag into a single-entry map on emit. Exposing the
  YAML-tag-aware path under a distinct name keeps the
  `Serialize`-trait contract clean while giving `Value` users a
  lossless emit option.

### Migration notice (pre-launch — applies before v0.0.1 is tagged)

Two source-level changes ship in `[Unreleased]` that downstream
crates touching the published `from_*` family will see. Both are
non-breaking for typed deserialise; they affect only the
`from_str::<Value>` and `from_value::<Value>` shapes.

1. **Tag preservation**: a `from_str::<Value>("!Custom 'hi'\n")`
   that previously returned `Value::String("hi")` now returns
   `Value::Tagged(Tag("!Custom"), Value::String("hi"))`. Code
   that read tagged scalars via `as_str` / `as_i64` / etc. needs
   either a wrapper unwrap (`value.untag_ref().as_str()`), a
   typed deserialise (`#[derive(serde::Deserialize)] struct Foo`), or a
   tag-aware `match`. See the migration recipe in
   [`doc/MIGRATION-FROM-SERDE-YAML.md`](doc/MIGRATION-FROM-SERDE-YAML.md#1-valuetagged-is-a-7th-variant--and-noyalib-preserves-scalar-tags-too).
2. **`T: 'static` bound** on the public `from_str` /
   `from_str_with_config` / `from_slice*` / `from_reader*` /
   `from_value` family. Every real-world `DeserializeOwned` type
   already satisfies it (the HRTB on its own already disallows
   borrowed lifetimes); the `'static` is what lets noyalib detect
   at the call site whether `T == Value` and engage the
   tag-preserving fast path. Add `+ 'static` to bound expressions
   in any wrapper functions you wrote on top of noyalib's
   `from_*`. Trait signatures from external crates (e.g.
   `figment::Format::from_str`) that drop `'static` are handled
   by a private internal entry point — your existing
   `impl Format for ...` keeps compiling.

### Added — Custom-tag scalar `Value::Tagged` surfacing on the default deserialise path

- **`from_str::<Value>("!Custom 'hi'")`** now returns
  `Value::Tagged(Tag("!Custom"), Value::String("hi"))` instead
  of unwrapping to the inner `Value::String("hi")`. The tag
  survives the data-binding return path so downstream consumers
  can dispatch on it. Tagged sequences and tagged mappings
  already worked via the AST loader; this closes the gap for
  scalars.
- **Typed targets are unchanged.** A `#[derive(serde::Deserialize)]
  struct Foo { x: u8 }` against `!Foo {x: 1}` still sees through
  the tag — that's the correct behaviour for the typed path and
  the only one that lets schema-tagged inputs deserialise into
  bare structs.
- **Mechanism**: the `from_str_with_config` / `from_value` entry
  points detect `T == Value` via [`std::any::TypeId`] and engage
  a `preserve_tags` flag on the noyalib `Deserializer`. When the
  flag is on, tagged values are surfaced through a magic-key
  MapAccess that `Value::deserialize`'s visitor recognises and
  reconstructs as `Value::Tagged`. Other Deserializers
  (`serde_json`, `figment`, FlatMap-shaped flatten extras) never
  see the magic shape.
- **API change**: the public `from_str` / `from_str_with_config`
  / `from_slice*` / `from_reader*` / `from_value` family now
  carries a `T: 'static` bound (in addition to the existing
  `for<'de> Deserialize<'de>`). This is a soft constraint that
  every real-world `DeserializeOwned` type already satisfies —
  the HRTB itself disallows borrowed lifetimes — and unlocks the
  TypeId-driven dispatch above. `figment` integration uses a
  private non-`'static` typed entry-point so its `Format::from_str`
  signature stays compatible.
- 4 regression tests retargeted from the old transparent-unwrap
  behaviour to the new tag-preserving contract:
  `tests/de.rs::test_deserialize_tagged_value`,
  `tests/coverage_100.rs::loader_tag_primary_empty_suffix` and
  `loader_custom_tag_with_inner_resolution`,
  `tests/coverage_boost.rs::loader_span_custom_tag_empty_suffix`,
  `tests/tag_registry.rs::unregistered_tag_on_scalar_falls_back_to_string`
  and `empty_registry_is_no_op`.

### Added — Truncated error formatters

- **`Error::format_with_source_truncated(source, max_chars)`**
  and **`Error::format_with_source_radius_truncated(source,
  radius, max_chars)`** — bridge-channel-friendly variants of
  the existing snippet renderers. Cap rendered diagnostics at a
  caller-supplied character budget, truncating on a UTF-8
  character boundary and appending an ASCII `...` ellipsis. Use
  for log lines, Slack messages, Sentry tags, or any sink with a
  hard length budget.
- Truncation contract: `<= max_chars` characters in the
  output; UTF-8-aligned cut; `...` appended unless `max_chars <
  3` (in which case the prefix that fits is returned without an
  ellipsis).
- Four unit tests cover the under-budget passthrough, the
  over-budget ellipsis, the tiny-budget ellipsis-drop, and
  multi-byte character alignment.

### Fixed — JSON-style UTF-16 surrogate pair pairing in `\uXXXX` escapes

- Double-quoted YAML scalars now accept `𝄞` (high + low
  surrogate) and combine the two halves into the corresponding
  supplementary-plane code point via the UTF-16 algorithm.
  Previously these escapes errored as "invalid Unicode code point
  U+D834" because `char::from_u32` rejects surrogate halves
  outright. JSON-emitting YAML producers commonly emit pair form,
  so the rejection was a real interop hit.
- Lone, reversed, and truncated surrogates remain rejected with
  the same error shape — the change is additive, not relaxed.
- 13 integration tests in `tests/json_surrogate_escape.rs` cover
  musical G clef (U+1D11E), grinning face emoji (U+1F600),
  multiple pairs in sequence, BMP-escape interleaving, and every
  rejection path.

### Added — Borrowed-path alias resolution

- **`BorrowedValue<'a>`** now eagerly resolves YAML anchors
  (`&name`) and aliases (`*name`). The anchored value is stored
  in a side-table; each alias clones it into the tree. String
  fields stay `Cow::Borrowed` so the clone is mostly free —
  only sequences and mappings actually duplicate, matching the
  owned `Value` path's behaviour.
- Alias-bomb defence: total expansions are capped by
  `ParserConfig::max_alias_expansions`, the same limit the owned
  path enforces.
- Aliases used as mapping keys coerce scalars to `Cow<'a, str>`
  (string / bool / number / null); non-scalar key aliases error
  rather than silently coercing — keeps the `Mapping` key type
  honest.
- Anchor namespace resets on `DocumentEnd` per spec.
- Previously the borrowed path errored with "aliases not
  supported in borrowed mode"; that message is gone. API surface
  is unchanged — the `BorrowedValue` enum and constructors are
  byte-identical.
- 12 integration tests in `tests/borrowed_alias_resolution.rs`
  cover scalar / sequence / mapping anchors, alias-as-key,
  multi-doc namespace isolation, unknown-anchor errors,
  expansion-cap defence, and round-trip parity with the owned
  path.

### Added — YAML 1.1 mode toggle

- **`ParserConfig::version(YamlVersion::V1_1)`** — single-call
  preset that flips the three resolver-table differences between
  YAML 1.2 (default) and 1.1 on as a bundle: `yes`/`no`/`on`/`off`
  booleans, bare-`0` octal `0644`, sexagesimal `60:00`. Selecting
  `V1_2` resets the trio so a config can be reverted without
  rebuilding from scratch.
- The fine-grained `legacy_booleans` / `legacy_octal_numbers` /
  `legacy_sexagesimal` flags remain available for callers who
  want to mix and match (e.g. "1.1 booleans but reject octal
  `0644`"). `version()` sets the preset; individual flags refine.
- New public type **`noyalib::YamlVersion`** with `V1_1` /
  `V1_2` variants. `Default::default()` is `V1_2`.
- 11 integration tests in `tests/yaml_version.rs` cover default
  behaviour, the 1.1 preset, the 1.2 reset, override-after-preset
  composability, and a Kubernetes-flavoured mixed-1.1-isms
  document round-trip.

### Added — `compat-serde-yaml` symbol parity

- **`Deserializer`** and **`Serializer`** types now re-export
  under `noyalib::compat::serde_yaml`. Existing
  `serde_yaml::Deserializer` / `Serializer` references compile
  unchanged after the prefix swap.
- New **`compat::serde_yaml::{value, mapping, with}`**
  sub-modules mirror the upstream layout. Migrating code that
  imports via the path form (`serde_yaml::value::Tag`,
  `#[serde(with = "serde_yaml::with::singleton_map")]`) only
  needs a search-and-replace on the prefix.
- The `with` sub-module re-exports all four
  `singleton_map_*` helpers + `nested_singleton_map`.
- 5 new tests in `compat/serde_yaml.rs` — the compat suite is
  now 13/13 green.

### Added — Lean / minimal dependency profile

- New **`fast-int`**, **`fast-float`**, and
  **`strict-deserialise`** Cargo features make `itoa`, `ryu`,
  and `serde_ignored` optional. All three are on by default —
  the lean profile is opt-out.
- New **`minimal`** meta-feature alias — equivalent to
  `default-features = false, features = ["std"]` — drops the
  three deps for FIPS / embedded / audit-heavy environments.
  Numeric formatting falls back to `core::fmt` (slower; output
  remains valid YAML); the `from_str_strict` /
  `from_slice_strict` / `from_reader_strict` typo-detection
  helpers are absent.
- Default profile: 8 runtime deps. Lean profile: 5 — drops
  `itoa`, `ryu`, `serde_ignored`. Verified via `cargo tree`.
- README's Install section documents the trade-off.

### Added — Strict deserialise on every input shape

- **`noyalib::from_slice_strict<T>`** and
  **`noyalib::from_reader_strict<R, T>`** — same unknown-field
  detection semantics as `from_str_strict`, but accepting `&[u8]`
  and `impl io::Read` directly so callers already holding bytes
  or a reader don't have to round-trip through `String` to opt
  in. Both gated behind `#[cfg(feature = "std")]` to match the
  existing string-input variant; both re-exported from the crate
  root.
- Five new integration tests in `tests/ux_diagnostics.rs` cover
  happy path + typo detection on both new helpers, plus
  invalid-UTF-8 rejection on the slice path. Doc-tests on each
  helper give an executable usage example.
- README "Strict deserialise" section gains an input-shape × API
  matrix (`&str` / `&[u8]` / `impl io::Read` × lenient / strict).

### Added — Ecosystem-citizen examples

- Six new examples that show noyalib slotting into the standard
  Rust configuration / validation / diagnostics toolbox without
  custom glue:
  - `include` — `$include`-key modular configs (Argo CD / JSON
    Schema `$ref`-style cross-file references) with cycle detection.
  - `figment` — layered defaults / YAML / env composition through
    the `figment::Provider` we already ship under the `figment`
    feature; demonstrates per-environment overlay chains.
  - `validation_garde` — declarative logic validation via the
    `garde` crate paired with `Validated<T>`.
  - `validation_validator` — same scenario through the
    `validator` crate (Actix / Axum / Rocket idiom) paired with
    `ValidatedValidator<T>`.
  - `diagnostic_path` — `serde_path_to_error` integration that
    pinpoints the offending nested key (including sequence indices
    such as `server.replicas[1].weight`) in deeply structured
    documents.
  - `robotics_polymorphism` — tagged-enum dispatch + the
    `Degrees` / `Radians` / `StrictFloat` newtypes from the
    `robotics` feature, illustrating unit-aware parsing on a
    Tree-Planting Robot mission plan.
- The `figment` Cargo dep now activates its `env` feature so the
  example chain (`Yaml::string` → `Env::prefixed`) compiles
  without consumers having to opt into it themselves.

### Added — Key interner

- **`noyalib::interner::KeyInterner`** — `&str` → `Arc<str>`
  deduplication primitive for memory-efficient repeated-key
  workloads. Each call to `intern(key)` returns a shared
  `Arc<str>`; the first call allocates, every subsequent call
  with the same key bytes returns a clone of the cached entry.
- Targets the Kubernetes-shaped use case where keys like
  `metadata`, `labels`, `name`, `apiVersion`, `selector` repeat
  thousands of times across a stream. For 20-byte keys repeated
  10 000 times, footprint drops from ~200 KB to ~20 bytes +
  `Arc` pointers.
- Public surface: `KeyInterner::new`, `with_capacity(n)`,
  `intern(&str) -> Arc<str>`, `get(&str) -> Option<Arc<str>>`,
  `len`, `is_empty`, `clear`.
- The `Mapping` public API is **unchanged** — `Mapping<String,
  Value>` is preserved so existing call sites compile clean. A
  future major version may swap the internal storage to
  `Arc<str>` and use the interner transparently during parse;
  v0.0.1 ships the primitive without that breaking change.
- 7 unit tests covering basic intern semantics, distinct-key
  separation, empty-string handling, `get` lookup,
  `clear` semantics, and a Kubernetes-key-set dedup smoke test.

### Changed — CST short-pointer compression

- `GreenChild::Token { len }` is now `u32` (was `usize`). YAML
  documents are bounded at 4 GiB by the parser's
  `max_document_length` cap, so a `u32` is sufficient. The
  narrower field drops `GreenChild::Token` from 24 bytes to 8
  bytes on a 64-bit target — meaningfully better L1/L2 cache
  locality on tree traversals.
- `GreenNode.text_len` is similarly narrowed from `usize` to
  `u32` (private field; public `text_len()` accessor still
  returns `usize` for ergonomic call-site arithmetic).
- Public `text_len()` accessors on `GreenChild` and `GreenNode`
  preserve their `usize` return type — the narrower storage is
  widened at the API boundary so existing callers continue to
  compile.
- 387/387 strict YAML 1.2 test-suite pass preserved (0 failures,
  19 deliberate skips out of 406 variant assertions); full test
  suite + doctest sweep green.

### Added — Parallel multi-document parsing

- **`noyalib::parallel::parse<T>(input) -> Result<Vec<T>>`** and
  **`noyalib::parallel::values(input) -> Result<Vec<Value>>`** —
  pre-scan `---` document boundaries on a single thread, then
  deserialise each document in parallel via Rayon. Targets
  multi-document streams (telemetry logs, audit exports,
  Kubernetes-resource snapshots) where single-thread parsing is
  CPU-bound.
- **`noyalib::parallel::split(input) -> Vec<&str>`** — the
  standalone document-boundary pre-scanner. Useful when the caller
  wants to drive their own concurrency primitives (async tasks,
  custom thread pools).
- Gated behind the `parallel` Cargo feature (off by default —
  pulls in `rayon` only when the user asks for it).
- 10 unit tests covering boundary detection edge cases (no
  separators, empty input, implicit first document, mid-line
  `---`, dashes followed by non-whitespace) and end-to-end
  correctness against `load_all_as`.

### Added — SWAR decimal-integer parser

- **`noyalib::simd::parse_decimal_u64` / `parse_decimal_i64`** —
  branch-free 8-digits-per-cycle SWAR pipeline replacing the
  stdlib byte-by-byte loop. Three pair-wise multiply-add phases
  fold a `u64` chunk of ASCII digits into the parsed value with
  no per-byte branch.
- Plumbed into the streaming integer resolver
  (`crate::streaming::parse_integer`); base-10 plain scalars now
  flow through the SWAR path. Hex / octal / sign-prefixed paths
  retain stdlib for spec-correct overflow semantics.
- Bench results (`benches/numeric_parse.rs`):
  - 3-digit input: parity with stdlib (SWAR doesn't engage).
  - **8 digits: 2.17× faster** (8.12 ns → 3.74 ns).
  - **19 digits: 2.38× faster** (22 ns → 9.25 ns).
  - **i64::MAX / i64::MIN: 2.5× faster.**
  - **Bulk parse of 1000 integers: 47 % faster.**
- Validation: every byte checked in `b'0'..=b'9'` before
  arithmetic; `wrapping_mul` is intentional in the SWAR pipeline
  (high bits discarded by downstream shift-and-mask) and the
  validator rejects malformed input. Overflow returns `None`.
- 11 unit tests including baseline equivalence against
  `<u64 as FromStr>::from_str` across 19 representative values
  (covers `i64::MIN`, `i64::MAX`, `u64::MAX`, sign handling).
- 387/387 strict YAML 1.2 test-suite pass preserved (0 failures,
  19 deliberate skips out of 406 variant assertions).

### Added — Canonical scanner needle constants

- **`simd::BLOCK_PLAIN_NEEDLES`** / **`simd::FLOW_PLAIN_NEEDLES`** /
  **`simd::LINE_BREAK_NEEDLES`** — public `&[u8]` constants
  documenting the YAML 1.2 plain-scalar boundary candidate sets
  per parser context. Future scanner refactors can reach the
  canonical set via these names without re-deriving them.

### Added — Structural bitmask discovery (`simdjson`-style)

- **`SimdScanner::structural_bitmask_32(&[u8; 32]) -> u32`** — load
  a 32-byte chunk and produce a dense bitmask where bit `i` is set
  iff `chunk[i]` is in the scanner's needle set. The building
  block of `simdjson`-style structural discovery: instead of
  walking the haystack and stopping at every delimiter, callers
  drain the mask via `mask.trailing_zeros()` + `mask & (mask - 1)`
  and advance the parser state machine directly from one delimiter
  to the next.
- **`StructuralIter`** — iterator that walks every structural-byte
  position in a haystack of arbitrary length. Handles the chunk
  loop, the partial-chunk tail, and the cached-bit drain
  internally so callers see one stream of byte offsets in order.
- Bench results (`benches/structural_bitmask.rs`, real YAML-shaped
  input):
  - **Stable Rust**: 4.2× faster than the existing memchr-loop
    structural-discovery path across 4 KiB / 64 KiB / 1 MiB.
  - **Nightly with `nightly-simd`**: 9.2× faster than the same
    baseline (single `Simd<u8, 32>` chunk + branchless
    `to_bitmask()` per 32-byte window).
- Five unit tests + cross-needle-set baseline equivalence check
  (every YAML-relevant arity 1 / 2 / 3 / 7 / 10) and four
  `StructuralIter` correctness tests covering chunk-boundary
  straddles, partial tails, and 2 KiB adversarial inputs against a
  scalar baseline.

### Removed — `thiserror` runtime dependency

- noyalib's `Error` enum no longer derives `thiserror::Error`. The
  `Display` and `std::error::Error` impls are now hand-written
  (matching the previous `#[error(...)]` format strings byte-for-
  byte, so all `Display` output is stable across the migration).
- Drops the proc-macro from every downstream crate's compile graph
  — meaningful for downstream build times in big workspaces.
- The runtime dep list is now: `serde`, `indexmap`, `rustc-hash`,
  `itoa`, `ryu`, `memchr`, `smallvec`. Every other dep is feature-
  gated and off by default.

### Changed — `serde` defaults synchronised with our `std` feature

- `serde = { default-features = false, features = ["derive",
  "alloc"] }` plus `std = ["serde/std"]` so `cargo build
  --no-default-features` actually compiles in no_std mode (serde's
  `de::Error` super-trait `StdError` resolves to a no_std-friendly
  bound when serde is itself in no_std mode).

### Removed — `serde_yaml` 0.9 upstream dependency

- The `compat-serde-yaml` shim **no longer pulls in the
  unmaintained `serde_yaml` 0.9 crate**. Every type the shim
  exposes (`Value`, `Mapping`, `Number`, `Sequence`, `Tag`,
  `TaggedValue`, `Error`, `Location`) is a noyalib-native type
  re-exported under the `serde_yaml` name; downstream
  `cargo audit` / `cargo deny` runs no longer pick up the
  archived advisory chain.
- The previous direct `From<noyalib::Value> for ::serde_yaml::Value`
  / `TryFrom<::serde_yaml::Value> for noyalib::Value` impls are
  removed. Mid-migration codebases route in-flight upstream values
  through the Serde data model instead — the universal-translator
  path the Serde ecosystem already provides for every JSON-shaped
  AST pair: `noyalib::to_value(&upstream_serde_yaml_value)?`.

### Added — Release-candidate examples and benches

- **`examples/entry_api.rs`** — surgical Kubernetes manifest
  patching via the `Document::entry` proxy API. Demonstrates
  `or_insert` / `insert_value` / `set` chained edits with every
  comment, indent, and sibling preserved byte-for-byte.
- **`examples/flattened.rs`** — `Flattened<T>` capture pattern:
  typed view + raw metadata view from one parse pass.
- **`examples/schema_validation.rs`** — library-level
  `schema_for` + `validate_against_schema` + `coerce_to_schema`
  pipeline. Mirrors what `noyavalidate --fix` does on the CLI.
- **`benches/streaming_vs_value.rs`** — head-to-head throughput
  comparison between `StreamingDeserializer` and
  `from_str::<Value>` across small / medium / large workloads,
  plus a dedicated `BTreeMap` MapAccess scenario.
- **`benches/large_doc_soak.rs`** — 1 MiB / 10 MiB / 50 MiB soak
  benchmark catching quadratic regressions and SIMD hot-path
  regressions on long-input workloads.

### Changed — MSRV-1.75 hardening

- Pinned `indexmap` to `2.10.0` and `rustc-hash` to `2.0.0` so the
  resolver does not pull manifests requiring Rust 2024 edition
  (Cargo 1.85+) and breaking the MSRV-1.75 check.
- Removed unused `yaml_lib` dev-dependency (its manifest also
  required edition 2024).
- Promoted `serde-saphyr` (optional `compare-saphyr` feature):
  the saphyr lineage adopted edition 2024 across all available
  versions; gating it lets the comparison benchmarks still run on
  newer toolchains while keeping the default 1.75 build path
  clean.
- Demoted `pub` → `pub(crate)` on internal `Span`, `Token`,
  `ScanError`, `ParsedDocument`, `SubtreeContext` fields to
  satisfy the workspace `unreachable_pub = "forbid"` lint on
  Rust 1.75 (the lint behaviour tightened between 1.75 and the
  current stable, so the existing `pub` declarations on
  `pub(crate)` parents only failed under the older toolchain).

### Added — Streaming `!!binary`

- **`StreamingDeserializer` honours `!!binary` natively** — `serde_bytes`
  byte targets (`Vec<u8>` with `#[serde(with = "serde_bytes")]`,
  `serde_bytes::ByteBuf`) now decode RFC 4648 base64 directly inside
  the streaming path without falling back to the AST. Mirrors the
  AST-path type contract: untagged plain scalars that resolve to
  int / float / bool / null produce a `TypeMismatch` rather than
  silently coercing their UTF-8 representation to bytes.

### Added — Schema-driven type coercion (surgical `--fix`)

- **`noyalib::coerce_to_schema(value, schema) -> Result<usize>`** —
  walks JSON Schema 2020-12 type-mismatch errors against an
  in-memory `Value` and coerces string-shaped values into the
  schema's expected type when the parse succeeds. Targets the most
  common hand-written-YAML failure mode: `port: "8080"` gets
  rewritten to `port: 8080` automatically when the schema says
  `port: integer`.
- Handles three coercions: `String → Integer`, `String → Number`,
  `String → Boolean`. Unparseable inputs are left in place so the
  caller can surface the residue via a follow-up
  `validate_against_schema` call.
- Iterative fix-loop (capped at 1024 passes) re-runs validation
  after each coercion so cascading errors converge cleanly.
- 8 integration tests in `tests/coerce_to_schema.rs` cover the
  three target types, nested objects, sequence items, mixed
  valid / fixable / unfixable inputs, and the no-op case.

### Added — Portable-SIMD structural scanner

- **`SimdScanner` type** in `noyalib::simd` — build-once,
  scan-many byte-set finder optimised for parser inner loops.
  Stable Rust uses the existing memchr / SWAR / bitmap path; the
  new `nightly-simd` Cargo feature widens the inner loop to a
  32-byte `Simd<u8, 32>` chunk via `core::simd` portable SIMD,
  broadcasting each needle and OR-ing equality masks for
  branch-free structural detection.
- **`build.rs` toolchain probe** — emits `cfg(noyalib_nightly)`
  when `rustc --version` reports a nightly channel, so the
  `feature(portable_simd)` attribute is gated on both the user's
  feature flag and the actual compiler — `--all-features` on
  stable continues to compile cleanly.
- Both code paths are exhaustively cross-checked against a scalar
  baseline across needle widths 2 / 4 / 8 / 10 and haystack lengths
  spanning the SIMD chunk boundary (31 / 32 / 33 / 64 / 128 / 1024).

### Added — Pluggable parser policies

- **`noyalib::policy` module** — `Policy` trait with
  `check_event(&PolicyEvent)` and `check_value(&Value)` hooks for
  enforcing organisational "Safe YAML" constraints during parsing.
- **Built-in policies**: `DenyAnchors` (rejects `&name` definitions
  and `*name` aliases — covers the billion-laughs vector and
  audit-readability concerns), `DenyTags` (rejects custom tags
  while permitting YAML 1.2 core tags), `MaxScalarLength(n)` (caps
  individual scalar size in bytes).
- **`ParserConfig::with_policy(p)`** — register one or more
  policies; they run in registration order during the AST loader's
  event walk. The streaming fast-path is bypassed automatically
  when any policy is registered, ensuring uniform enforcement.
- 11 integration tests in `tests/policy.rs` cover each built-in
  policy, custom-policy composition, short-circuit-on-first-error
  semantics, and streaming-path bypass.

## [0.0.1] - 2026-05-04

The launch release. Sections below catalogue every capability the
library ships at launch, grouped by theme. See
[`doc/design/`](doc/design/) for the architecture rationale and
the commit history on `main` for per-change context.

### Added — Property interpolation

- **`Value::interpolate_properties(&map)`** — substitute `${name}`
  references inside string scalars from a property map. Walks
  recursively into sequences, mappings, and tagged values; map
  keys are left unchanged so the schema stays stable. `${{` and
  `}}` escapes preserve literal `${` / `}`. Returns
  `Error::Custom` on unknown placeholders.
- **`Value::interpolate_properties_lossy(&map)`** — same walk,
  but unknown placeholders substitute the empty string instead of
  erroring. Suitable for env-var expansion where missing
  variables should silently degrade.
- Placeholder names match `[A-Za-z_][A-Za-z0-9_.]*` so dotted
  hierarchies like `${db.host}` work.

### Added — serde-ecosystem interop

- **`serde_path_to_error` interop** — verified by
  `tests/serde_ecosystem.rs`; the path through nested structures
  and sequences is reported correctly when wrapping noyalib's
  `Deserializer`.
- **`serde_ignored` interop** — same test file confirms unknown
  fields at the top level and at any depth are surfaced through
  the standard wrapper without noyalib-specific integration.

### Added — `figment` provider

- **`figment` Cargo feature** — pulls in `figment` 0.10 and
  exposes `noyalib::figment::Yaml`, a drop-in `Format` + `Provider`
  that plugs into `Figment::merge` / `Figment::join` chains the
  same way `figment::providers::Toml` and
  `figment::providers::Json` do.
- 8 integration tests in `tests/figment_provider.rs` cover
  string/file extraction, layered merge / join semantics, parse-
  and missing-field error propagation, nested struct round-trip,
  and YAML 1.2 anchor + alias resolution through the provider.

### Added — `ParserConfig` knobs

Four additive `ParserConfig` toggles, all defaulting to YAML 1.2
spec behaviour (zero impact on existing callers):

- **`merge_key_policy`** with [`crate::MergeKeyPolicy`] —
  `Auto` (default) preserves YAML 1.2 §10.2 merge semantics;
  `AsOrdinary` keeps `<<` as a literal key in the resulting
  mapping; `Error` rejects any document containing a `<<` key.
  When set to non-`Auto`, the deserializer routes through the
  AST loader (the streaming path hard-wires the YAML 1.2
  semantics).
- **`no_schema`** — when `true`, every plain scalar surfaces as
  a `Value::String` regardless of whether it would normally
  resolve to `null` / `bool` / int / float. The "Norway problem"
  fix: schema strictness is opt-in. Quoted scalars and explicit
  tags (`!!int`, `!!bool`) are unaffected.
- **`legacy_octal_numbers`** — when `true`, accepts YAML
  1.1-style bare `0`-prefix octal literals (`0644` → 420) in
  addition to the YAML 1.2 `0o644` form. Numerics with `8` or
  `9` digits fall through to decimal even with the toggle on.
- **`ignore_binary_tag_for_string`** — when `true`,
  deserializing `!!binary "ABCD"` into a `String` target yields
  the literal base64 source string rather than rejecting on tag
  mismatch. The canonical bytes path (`Vec<u8>`,
  `serde_bytes::ByteBuf`) is unaffected — it always decodes the
  base64 payload. Useful for migrations from Python pyyaml-style
  applications that treat the tag as advisory.

### Added — `Flattened<T>` capture wrapper

- **`noyalib::Flattened<T>`** — pairs a typed deserialization of
  `T` with the underlying [`Value`] tree captured from the
  source. Solves the "I want `#[serde(flatten)]` plus the dynamic
  view for span lookup / unknown-field detection / schema
  validation" use case that the built-in residue types
  (`HashMap<String, Value>` etc.) erase. Deserializes by
  capturing the input as a [`Value`] first, then re-running
  `T::deserialize` against the captured tree via
  [`crate::from_value`]. Both `flattened.value: T` and
  `flattened.raw: Value` are exposed; `Deref<Target = T>` makes
  the typed view ergonomic. Round-trip transparency on
  serialize: only the typed view is emitted, mirroring
  `Spanned<T>`.

### Added — `legacy_sexagesimal` ParserConfig toggle

- **`ParserConfig::legacy_sexagesimal(true)`** — accept YAML
  1.1-style colon-separated base-60 numbers (`60:00` → 3 600,
  `1:30:00` → 5 400, `-1:30:00` → -5 400) as integers.
  Fractional last-component variant (`1:30:00.5` → 5 400.5)
  resolves to a float. Off by default; YAML 1.2 dropped the
  sexagesimal schema. Robust against false positives:
  components other than the first are clamped to 0..=59 and
  ISO-8601 timestamps with embedded `:` colons are correctly
  classified as strings, not as sexagesimal.

### Added — `JsonSchema` for `noyalib::Value`

- **`impl JsonSchema for noyalib::Value`** (gated by the
  `schema` feature) — emits the JSON Schema 2020-12 idiom for
  "any JSON-expressible value": a `oneOf` union of null,
  boolean, number, string, array, and object, with the array /
  object cases referencing the same `YamlValue` definition
  recursively. Lets users derive [`schemars::JsonSchema`] on a
  struct that has a `Value` field (e.g. an envelope type whose
  `payload` is "any user-supplied YAML") without writing a
  custom impl.

### Added — Mutable-Value experience for the CST

- **`Entry::or_insert(default)`** / **`or_insert_with(f)`** /
  **`or_insert_value(default)`** — std-collections-style
  ergonomics on top of the existing path-shaped Entry handle.
  Returns `Ok(true)` when the splice ran (path was vacant),
  `Ok(false)` when the path was already occupied. Top-level
  keys and sequence-index paths get actionable errors that
  redirect to `Document::set` and `push_back`/`insert_after`
  respectively.
- **`Entry::and_modify(f)`** — closure runs only when the path
  resolves; receives a `&mut Document` for arbitrary
  cross-path mutations. Returns `self` so the standard
  `and_modify(...).or_insert(...)` pattern composes.
- **`Document::rename_anchor(old, new)`** — atomic rename of
  every `&old` declaration and every `*old` reference in one
  operation. Returns the count of touched sites. The whole
  rename is performed as a single `replace_span` over the
  document so intermediate states with mismatched anchor /
  alias names are never observed. Validates `new` against YAML
  1.2 §6.9.2 (no flow indicators or whitespace).

### Added — Style heuristics for CST inserts

- **`Document::dominant_quote_style()`** returns the file's
  preferred scalar quote style (`Plain`, `SingleQuoted`, or
  `DoubleQuoted`) by tallying every quoted scalar in the green
  tree and breaking ties in favour of the simpler form. Plain
  mapping keys are deliberately ignored — the question is
  "when the user *did* quote a value, what did they reach
  for?".
- **`Document::dominant_flow_style()`** returns the dominant
  collection layout (`FlowStyle::Block` or `FlowStyle::Auto`)
  by counting Block vs Flow mappings and sequences.
- **`Entry::insert_value`** now consumes both heuristics: a new
  `Value::String` value gets the file's dominant quote style
  applied to the spliced fragment (manual quoting since the
  serializer's `scalar_style` config does not affect top-level
  scalars); collections continue to splice in block form for
  multi-line emissions. The `dominant_flow_style()` accessor
  is exposed for callers who want to wrap typed collections in
  `fmt::FlowMap` / `fmt::FlowSeq` before serializing.

### Added — Multi-line error snippets

- **`Error::format_with_source_radius(source, radius)`** —
  rustc-style error rendering with `radius` lines of context
  above and below the offending line. Output uses a fixed-width
  gutter (line numbers right-aligned to the widest), a `|` rule,
  and a caret line under the offending column. Falls back to
  plain `Display` when the error has no location or the location
  is past EOF.
- The original [`crate::Error::format_with_source`] is preserved
  byte-for-byte; the radius variant is purely additive.

### Added — Spec compliance

- **Native YAML 1.2 scanner and parser**, written entirely in safe
  Rust — `#![forbid(unsafe_code)]` at the crate root.
- **100% YAML Test Suite strict compliance**: 387/387 attempted
  variant assertions pass, 0 failures, 19 deliberate skips out
  of 406 total. The skip list is tracked alongside the harness in
  `tests/yaml_compliance_report.rs` so the gap is explicit and
  audit-friendly; each new correctness fix lands with the
  corresponding suite case unblocked.
- Full serde `Serialize` and `Deserialize` support including
  `#[serde(flatten)]`, `#[serde(default)]`, `#[serde(rename)]`,
  enum representations (externally-tagged, internally-tagged,
  adjacently-tagged, untagged).
- **Multi-document streams**: `load_all`, `load_all_as`,
  `to_string_multi`, `to_writer_multi`, `from_str_multi` (under
  the `compat-serde-yaml` feature) — `---` / `...` separators
  honoured, byte-faithful concatenation when paired with the
  CST.
- **YAML 1.1 compatibility** via `ParserConfig::legacy_booleans`:
  resolves `yes`/`no`/`on`/`off`/`y`/`n` as booleans (the
  "Norway problem" — opt-in, never silent).
- **Strict-mode hardening**: `ParserConfig::strict_booleans`,
  depth limits, document-length cap, alias-expansion cap,
  duplicate-key policy, recursion-depth probe.

### Added — Frictionless migration from `serde_yaml`

- **Comment-aware reads** (`load_comments`, `Comment`,
  `CommentKind`) — extract leading / trailing / standalone
  comments without touching the typed `Value` path.
- **`noyafmt` CLI**: lossless YAML formatter that round-trips
  through the CST, normalising whitespace and quoting without
  changing semantics.
- **`noyalib-mcp`**: Model Context Protocol server exposing
  `parse`, `format`, `get`, `set`, `validate` tools — drop-in
  for any LLM agent that needs YAML manipulation.
- **WASM playground** (`noyalib-wasm`): 201 KB
  `wasm32-unknown-unknown` build with browser demo.

#### Added — `serde_yaml` compat shim

- **`compat-serde-yaml` feature**: drop-in surface for the
  unmaintained `serde_yaml` 0.9 crate.
- Type-level parity with `serde_yaml::Value`,
  `serde_yaml::Mapping`, `serde_yaml::Number` via `From` /
  `TryFrom` conversions both directions, with
  `SerdeYamlConversionError { NonStringKey, UnrepresentableNumber }`
  for the lossy edges.
- `noyalib::compat::serde_yaml::Error` re-export wrapping
  `noyalib::Error` with location parity.
- **`Document::validate`**: non-panicking sibling of `ensure_cache`
  for callers that want to surface invalid-source errors as
  `Result` rather than via lazy panic.

#### Added — `!!binary` first-class support

- **`!!binary` tag** with RFC 4648 base64 codec
  (`src/base64.rs`, hand-rolled, whitespace-tolerant decoder).
- `serde_bytes::Bytes` / `ByteBuf` round-trip including
  multi-line block-scalar form, inline form, quoted form, and
  the full 0..=255 byte range.
- `Value::Tagged` carries `Tag::new("!!binary")` for callers
  that walk the typed tree.

#### Added — `Spanned<Value>` flatten guard

- Bare `Value` as the target of `#[serde(flatten)]` collects
  unmatched keys into a `Value::Mapping` exactly as
  `serde_yaml` / `serde_json` users expect.
- `Spanned<Value>` in a `#[serde(flatten)]` position now errors
  with a clear, actionable message pointing at the working
  alternative (bare `Value` + `Document::span_at`) instead of
  the bare `missing_field` gibberish that resulted from serde's
  `FlatStructAccess` filtering.

### Added — Lossless editing API

- **Side-table CST** (`noyalib::cst`) for byte-faithful
  round-tripping: `parse_document(s)?.to_string() == s` for any
  input the parser accepts.
- `Document::source`, `Document::span_at`, `Document::get`,
  `Document::comments_at`, `Document::syntax`,
  `Document::as_value` for read access by path.
- `Document::set`, `Document::set_value`, `Document::remove`,
  `Document::push_back`, `Document::insert_after`,
  `Document::replace_span` for mutation — every edit is
  byte-faithful outside the spliced region; comments, blank
  lines, and sibling formatting survive verbatim.
- **Incremental repair**: localised `replace_span` re-parses the
  smallest enclosing block; Document-scope re-parse only on
  shape inversion.
- **Lazy `Value` / `SpanTree`**: typed cache invalidated rather
  than re-parsed eagerly — successive edits in a batch don't
  pay the parser cost; the deferred parse runs once on the
  first read (~6× single edit).
- **Green-tree path resolution**: walks the structural CST
  directly, skipping the typed cache for the common
  set-then-set pattern (~7.6× batch).
- **Relative-len leaves**: O(log N) splice — the green node only
  stores child lengths, not absolute byte ranges (~37× over
  baseline).

#### Added — `Entry` API

- **`Document::entry(path) -> Entry<'_>`** path-shaped mutable
  handle, complementing the functional `set` / `remove` /
  `push_back` / `insert_after` methods (both stay first-class).
- 12 methods on `Entry`: `path`, `exists`, `get`, `span_at`,
  `comments`, `set`, `set_value`, `remove`, `insert`,
  `insert_value`, `push_back`, `insert_after`, plus chained
  drill-down via `Entry::entry(child)` with smart path
  composition (`items[0]` not `items.[0]`).
- New primitive `Document::insert_entry` — mapping-side
  analogue of `push_back` for sequences.

#### Added — automatic indent detection

- **`Document::indent_unit()`**: detects 2- / 3- / 4-space block
  indents from non-empty/non-comment line deltas; defaults to 2
  when undetectable. Tab-indented lines short-circuit.
- `Entry::insert_value` and `Document::insert_entry` plumb the
  detected unit into the serializer so inserts conform to the
  surrounding file's convention.
- Bug fix bundled: `column_of_key_at` now walks back to the
  actual key line (not the value's first byte), so a sibling
  insert under a parent whose last value is a nested block
  lands at the correct column.

#### Added — anchor management

- **`Document::anchors()`**, **`aliases()`**, **`aliases_of(name)`**:
  every `&name` / `*name` lexeme in source order with byte spans.
- **`Document::materialise_alias_at(byte_pos)`**: replace `*name`
  with the source bytes of `&name`'s scalar value, leaving the
  alias's site independent of any future edits to the anchor.
- **`Document::materialise_aliases_of(name)`**: bulk; reverse
  source-order so each splice's offsets stay valid.
- Propagation contract documented: edits to anchored values are
  visible at every alias site after the next load (because
  aliases are pointers in YAML's data model).
- Multi-line block-valued anchors return a clear "follow-up"
  error pointing at `Document::anchors()` + `replace_span()`
  for manual splicing — out of scope for v0.0.1.

### Added — Schema contracts

#### Added — JSON Schema codegen

- **`schema` Cargo feature** (off by default).
- **`pub use schemars::JsonSchema`** — derive imported via
  `noyalib`, no second crate dep for users.
- **`schema_for::<T>() -> Result<Value>`**: schema as a
  `noyalib::Value` tree.
- **`schema_for_yaml::<T>() -> Result<String>`**: schema as YAML
  text for sharing / version control.
- Honours `#[doc]` (→ `description`), `#[serde(default)]` (drops
  from `required`), `#[serde(rename)]` (renames property), and
  emits `minimum`/`maximum` for fixed-width integers.

#### Added — Schema validation and enhanced CLI

- **`validate-schema` Cargo feature** (implies `schema`).
- **`validate_against_schema(value, schema) -> Result<()>`**:
  enforce a JSON Schema 2020-12 contract against parsed YAML.
  Multiple violations aggregated with RFC 6901 JSON-pointer
  paths.
- **`validate_against_schema_str(yaml, schema_yaml)`**:
  convenience for raw text.
- **`noyavalidate -s/--schema PATH`**: validate each parsed
  document against the schema (YAML or JSON; both parse).
  Multi-doc streams prefix each failing document with
  `[document N]`.
- **`noyavalidate --fix`**: in-place lossless reformat via the
  CST formatter. Stdin + `--fix` keeps stdout clean for piping.
- **Critical guard**: `--fix` does NOT run when `--schema`
  rejects the input — otherwise a buggy file would be silently
  rewritten with the violation in place.

### Added — SIMD primitives and hot-path integration

- **`noyalib::simd` module**: pure-safe Rust multi-byte search
  primitives.
- `find_any_of(haystack, needles) -> Option<usize>` — dispatches
  to `memchr` for arity 1/2/3, SWAR (8-byte-stride packed
  membership lookup) for arity 4+.
- `clean_prefix_len(haystack, needles)` — length of the leading
  no-needle run; the "skip-clean-prefix" call shape.
- `ByteBitmap` + `bitmap_for(needles)` + `find_byte_in_bitmap` —
  256-bit bitmap surface for callers amortising bitmap
  construction across many calls with the same needle set.
- **Hot-path integration**: the plain-scalar inner loop in
  `fetch_plain_scalar` skips ahead via `clean_prefix_len`
  before applying the state-dependent boundary rules.
  Equivalence-tested against the byte-by-byte baseline; YAML
  1.2 official suite stays at 100% with the integration on.
- **Throughput** (Apple M1, criterion --quick, 64 KiB sparse
  haystack): arity-3 memchr 29 GiB/s vs scalar 509 MiB/s
  (~58×); arity-8 SWAR 1.45 GiB/s vs scalar 270 MiB/s (~5.4×).
- **`unsafe_code = "forbid"` invariant preserved** — no
  `core::arch::*` intrinsics, no platform-specific deps.

### Performance

Benchmarked on Apple M4, Rust 1.94 stable:

| Benchmark | noyalib | serde\_yaml\_ng | Improvement |
|---|---|---|---|
| Serialize (simple) | 358 ns | 1.41 us | **75% faster** |
| Serialize (nested) | 2.80 us | 8.32 us | **66% faster** |
| Deserialize (simple) | 1.39 us | 2.79 us | **50% faster** |
| Deserialize (nested) | 9.16 us | 17.3 us | **47% faster** |
| Deserialize (large) | 0.83 ms | 1.49 ms | **44% faster** |

CST-only metrics (Apple M1, criterion --quick, batch of 500
single-key edits):

| Optimisation | Speedup |
|---|---|
| Incremental repair | baseline |
| Lazy `Value`/`SpanTree` | ~6× single edit |
| Green-tree path resolution | ~7.6× batch |
| Relative-len leaves | ~37× over baseline |

### Added — API surface (foundation)

- `Value`, `Mapping`, `MappingAny`, `Sequence`, `Number`, `Tag`,
  `TaggedValue` types.
- `from_str`, `from_slice`, `from_reader`, `from_value`
  deserialization functions.
- `to_string`, `to_writer`, `to_fmt_writer`, `to_value`
  serialization functions.
- All functions available with `_with_config` variants for
  custom security / formatting limits.
- `SerializerConfig` with indent, flow style, scalar style,
  block scalars, document markers, `quote_all`,
  `compact_list_indent`, `folded_wrap_chars`, `min_fold_chars`.
- `ParserConfig` with depth limits, document-length limits,
  alias-expansion caps, duplicate-key policy,
  `strict_booleans`, `legacy_booleans`.
- **`Streaming` deserializer** (`StreamingDeserializer`):
  bypasses the `Value` AST for typed deserialization (50%
  faster than the Value-based path).
- **`BorrowedValue<'a>`**: zero-copy AST that borrows strings
  from input — 18% faster than the owned `Value`.
- **Path queries**: `value.query("items[*].name")` with
  wildcards (`*`) and recursive descent (`..`).
- **`Spanned<T>`** for tracking source line, column, and byte
  offset of deserialized values.
- **`apply_merge()`** for YAML merge key (`<<`) expansion.
- **`Path` type** for structured error location tracking.
- **Anchor & alias support**: `RcAnchor`, `ArcAnchor`,
  `RcWeakAnchor`, `ArcWeakAnchor`, `AnchorRegistry`,
  `ArcAnchorRegistry`.
- **`fmt` module**: `FlowSeq`, `FlowMap`, `LitStr`, `FoldStr`,
  `Commented`, `SpaceAfter`.
- **`with` module**: `singleton_map`, `singleton_map_optional`,
  `singleton_map_recursive`, `singleton_map_with`.
- **YAML 1.2 spec-schemas**: `validate_yaml_core_schema`,
  `validate_yaml_json_schema`, `validate_yaml_failsafe_schema`,
  `is_yaml_failsafe_compatible`, `is_yaml_json_compatible`.
- **`miette` diagnostic integration** (`miette` feature): rich
  terminal diagnostics with error codes, help text, source
  spans.
- **`garde` / `validator` integration** (`garde` / `validator`
  features): declarative post-deserialise validation via
  `Validated<T>` / `ValidatedValidator<T>`.
- **`#[non_exhaustive]`** on `ParserConfig`, `SerializerConfig`,
  `FlowStyle`, `ScalarStyle`.
- **`#[must_use]`** on 83 query methods.

### Added — Tooling & CLIs

- **`noyavalidate`**: validate YAML syntax (and optional JSON
  Schema) with rich `miette` diagnostics; supports `--schema
  PATH` (enforces a JSON Schema 2020-12 contract) and `--fix`
  (in-place lossless reformat through the CST).
- **`noyafmt`**: lossless CST-driven formatter.
- **`noyalib-mcp`**: Model Context Protocol server (separate
  workspace member).
- **`noyalib-wasm`**: WASM bindings + browser playground
  (separate workspace member).

### Added — Examples

- **60 branded examples** under `crates/noyalib/examples/`, each
  with the animated spinner UI from `examples/support.rs`.
- Categorised into Core, Spec, Logic & Security, DX, Advanced,
  Future-Proof, Deep Rust, Final, Platform, and Competitive
  Features.

### Added — Testing

- **2,200+ tests** including YAML spec compliance,
  property-based tests (`proptest`), competitor parity tests
  (`yaml-rust2`, `serde-saphyr`, `yaml_lib`, `rust-yaml`,
  `serde_yaml_ng`), and edge cases.
- **9 fuzz targets** (`cargo fuzz`) — five generic
  (`fuzz_parse`, `fuzz_roundtrip`, `fuzz_from_value`,
  `fuzz_multi_doc`, `fuzz_strict`) plus four targeted regression
  fuzzers (`fuzz_borrowed_alias`, `fuzz_diff`,
  `fuzz_double_quoted`, `fuzz_yaml_v1_1`). Seed corpus committed
  under `fuzz/corpus/seed/`.
- **Differential fuzz smoke** in CI (10 s per push).
- **Soak fuzz** (weekly, 1 hour per target) under
  `.github/workflows/security.yml`.
- **YAML 1.2 official suite vendored** under
  `tests/yaml-test-suite/` (MIT, upstream).
- **Cross-platform CI**: Linux, macOS, Windows × stable,
  1.75.0 (MSRV), nightly. Nightly is `continue-on-error`.

### Added — Supply chain & governance

- **`#![forbid(unsafe_code)]`** at the crate root.
- **`unreachable_pub = "forbid"`**, `non_ascii_idents = "forbid"`,
  full `clippy::all + pedantic + cargo + nursery` policy.
- **MSRV pinned at 1.75.0** with a dedicated CI job.
- **`cargo-deny`** licenses + advisories + bans + sources.
- **`cargo-vet`** with the Mozilla, Google, Bytecode Alliance,
  Embark, ISRG audit imports plus a bootstrap exemption list.
- **`cargo-semver-checks`** on every PR (gated against
  pre-publication state until the first crates.io release).
- **OpenSSF Scorecard** badge.
- **CodeQL** static analysis.
- **REUSE.software 3.3 compliance** — every file has SPDX
  copyright + license headers, blanket `REUSE.toml`
  annotations cover meta / CI / docs / fixtures.
- **SLSA L3 provenance** + **sigstore** signing in the
  release workflow.
- **SHA256 / SHA512 checksums** + **SBOM** generated per
  release.
- **`Assisted-by:` trailer** auto-injected on every commit per
  the Linux kernel coding-assistants standard.
- **Signed commits** (SSH ed25519) verified by CI.

### Added — `no_std` posture

- Full `#![no_std]` support: `default-features = false` keeps
  the `alloc`-only build working. Core parsing / serialization
  (`from_str`, `to_string`, `Value`, schemas) and the streaming
  deserializer all run without `std`.
- I/O functions (`from_reader`, `to_writer`),
  `Spanned<T>` deserialization (thread-local storage), the
  `cst` module, and the `noyavalidate` / `noyafmt` CLIs require
  the `std` feature.
- **CI enforces `cargo check --no-default-features` on every
  push.**

### Added — Cargo feature matrix

The full `[features]` block of `crates/noyalib/Cargo.toml`. Three
default-on optional features (`fast-int`, `fast-float`,
`strict-deserialise`) opt out via `default-features = false`; all
other optional features opt in.

| Feature | Default | Pulls in |
|---|---|---|
| `std` | yes | (none — gates std-only items) |
| `fast-int` | yes | `itoa` 1 (branchless integer formatting) |
| `fast-float` | yes | `ryu` 1 (branchless float formatting) |
| `strict-deserialise` | yes | `serde_ignored` 0.1 (`from_*_strict`) |
| `minimal` | no | meta-alias for `std` only — drops the three above |
| `miette` | no | `miette` 7 rich diagnostics |
| `garde` | no | `garde` 0.22 derive-based validation |
| `validator` | no | `validator` 0.19 derive-based validation |
| `compat-serde-yaml` | no | name-for-name shim (no upstream dep) |
| `schema` | no | `schemars` 1.2 + `serde_json` (codegen) |
| `validate-schema` | no | implies `schema` + `jsonschema` 0.33 |
| `figment` | no | `figment` 0.10 `Yaml` Provider |
| `parallel` | no | `rayon` 1.10 (`parallel::parse` / `values` / `split`) |
| `simd` | no | `noyalib::simd::*` primitives + parser hot path |
| `nightly-simd` | no | nightly rustc — 32-byte `StructuralIter` (implies `simd`) |
| `compare-saphyr` | no | dev-only — `serde-saphyr` for cross-library benches |
| `robotics` | no | `Degrees` / `Radians` / `StrictFloat` newtypes |
| `noyavalidate` | no | binary feature: `std` + `miette` + `validate-schema` |
| `wasm-opt` | no | size-tuned WASM build profile |

[Unreleased]: https://github.com/sebastienrousseau/noyalib/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/sebastienrousseau/noyalib/releases/tag/v0.0.1
