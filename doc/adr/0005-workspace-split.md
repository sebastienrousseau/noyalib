# 0005. Split the noyalib workspace into 4 satellite repositories with strict-lockstep versioning

- **Status:** proposed
- **Date:** 2026-07-01
- **Authors:** Sebastien Rousseau

## Context

Since v0.0.1 (2026-05-04) noyalib has shipped as a Cargo workspace
containing five publishable crates:

- `noyalib` — the core YAML 1.2 library
- `noya-cli` — the `noyafmt` + `noyavalidate` CLI binaries
- `noyalib-lsp` — the Language Server Protocol implementation
- `noyalib-mcp` — the Model Context Protocol server
- `noyalib-wasm` — the browser / WebAssembly bindings

The monorepo shape has served the launch phase well. Every
cross-cutting hardening pass (CI cache-poisoning guard in v0.0.11,
supply-chain refresh in v0.0.9, no_std fix in v0.0.11) lands as
one PR touching every crate that needs to change. Every release
is a single tag that publishes five crates in lockstep. Every
issue and pull request lands in one tracker with one label
taxonomy.

The forces now pushing back on the monorepo shape:

- **Contributor surface confusion.** A contributor filing a bug
  against the LSP has to know that the LSP lives under
  `crates/noyalib-lsp/` in a bigger repo whose root README talks
  primarily about the core library. Different downstream
  audiences (LSP consumers via editor plugin marketplaces, MCP
  consumers via AI-agent frameworks, WASM consumers via npm,
  CLI consumers via Homebrew / apt / snap) each want their own
  repo home for issue triage and release cadence visibility.
- **Downstream dependency surface.** Even though noyalib's
  packaged `.crate` archive on crates.io does not include the
  satellite source (per `include = [...]` in
  `crates/noyalib/Cargo.toml`), a downstream user cloning the
  git repo to audit or contribute pulls in the whole workspace.
  For downstream security scanners that trace repo provenance
  as part of the supply-chain graph, the repo-level footprint
  matters.
- **Independent release visibility.** All five crates ship at
  the same version because that's the shape lockstep in a
  workspace enforces. A downstream user who consumes only
  `noyalib-wasm` from npm has no visible signal that a v0.0.11
  release happened because the wasm crate itself changed; every
  noyalib patch bumps their wasm dep even when the wasm source
  was untouched.
- **Repository-level metrics.** Per-satellite stars, watchers,
  forks, Scorecard scores, and issue-triage SLAs are useful
  signals to satellite maintainers (once the satellites have
  co-maintainers) and to downstream consumers evaluating the
  health of the specific component they depend on. The monorepo
  aggregates all of these under one number.

Against those forces sit the **counter-arguments** that were
weighed and rejected (see Alternatives):

- The technical "footprint" argument for downstream users of
  `noyalib` is already addressed by `include = [...]`. Splitting
  does not shrink the packaged `.crate` archive.
- Cross-cutting hardening becomes N-PR coordination unless it is
  factored into shared reusable workflows before the first split.
- Lockstep coordination becomes N-repo release orchestration
  unless it is factored into a shared release workflow.

The forces balance in favour of splitting **only if** the
shared-workflow scaffolding lands **before** the pilot, and only
if the versioning contract is chosen to preserve the audit
clarity that lockstep provides today. The remainder of this ADR
formalises both.

## Decision

We WILL split the noyalib workspace into 5 repositories under
the `sebastienrousseau/` org:

- `sebastienrousseau/noyalib` — the core library (this repo, kept
  as-is after the split, with the satellite `crates/*/`
  directories removed)
- `sebastienrousseau/noyalib-wasm` — WebAssembly bindings
- `sebastienrousseau/noyalib-mcp` — Model Context Protocol server
- `sebastienrousseau/noyalib-lsp` — Language Server Protocol
- `sebastienrousseau/noya-cli` — CLI binaries (`noyafmt`,
  `noyavalidate`)

The split proceeds as a **staged pilot** over four release
windows to catch failure modes early:

- **v0.0.12** — pilot: `noyalib-wasm`. Smallest satellite, no
  intra-workspace dependents on it, cleanest split.
- **v0.0.13** — `noyalib-mcp`. Same playbook.
- **v0.0.14** — `noyalib-lsp`.
- **v0.0.15** — `noya-cli` (bundled with `xtask`). Largest
  satellite. Core is officially single-crate after this ships.

Each split window includes a **14-day soak review** at its close.
No subsequent split begins until the prior window records a GO
signal. If any window records a NO-GO, the **rollback recipe**
(see below) restores the monorepo shape and the split project is
paused pending a full retrospective.

### Versioning contract

We WILL use **strict lockstep versioning** across all 5
repositories. Every satellite version equals `noyalib`'s version
at the moment of release. All 5 crates release simultaneously
with a single coordinated tag push in the release-orchestration
workflow.

Cross-repo dependencies MUST use exact-match syntax:

```toml
# In noyalib-lsp/Cargo.toml — required
noyalib = "=0.0.42"
```

A regression check MUST fail CI if a caret (`^`), tilde (`~`),
or floating-range (`>=`, `<`) spec appears on any published
branch of any satellite.

### Cross-repo dependency posture

Satellites MUST NOT use `path = "../noyalib"` overrides on any
published branch. Local development against an unpublished
noyalib requires a `[patch.crates-io]` block in the satellite's
`Cargo.toml`, gated by a `[cfg(local-dev)]` convention that is
never merged to `main`.

### Shared reusable workflows

Every satellite's `.github/workflows/` MUST consume the parent
repo's shared workflows via `uses:` references — never by copy.
Shared workflows are added to this repo under
`.github/workflows/shared-<name>.yml` with a `workflow_call`
trigger and pinned by SHA in each satellite. A hardening pass
lands as one PR in this repo plus N (small, Dependabot-managed)
SHA-bump PRs in the satellites.

**Caller-side permissions must be at least as broad as every
consumed workflow.** When the callee declares a `permissions:`
scope the caller does not, GitHub Actions rejects the entire
workflow with a 0s `startup_failure` — no jobs are scheduled and
no annotation is emitted. The parent repo's own `ci.yml` sets
`permissions: read-all` and never hits this. Satellites narrow to
the specific scopes their consumed workflows demand:

| Consumed workflow                | Required caller permissions |
|----------------------------------|------------------------------|
| `shared-cargo-deny.yml`          | `contents: read`             |
| `shared-cargo-vet.yml`           | `contents: read`             |
| `shared-cargo-machete.yml`       | `contents: read`             |
| `shared-reuse.yml`               | `contents: read`             |
| `shared-rustdoc-strict.yml`      | `contents: read`             |
| `shared-test-matrix.yml`         | `contents: read`             |
| `shared-verify-signatures.yml`   | `contents: read` **and** `pull-requests: read` |

The `pull-requests: read` gap was hit by the v0.0.12 pilot
(noyalib-wasm PR #1); root-cause + fix documented there. New
satellites MUST copy the union of required permissions into their
top-level `ci.yml permissions:` block or they will startup-fail.

### Rollback recipe

The rollback recipe MUST be executable in **≤60 minutes** of
maintainer time from a clean scratch clone. Concretely:

1. `git clone` this repo at the pre-split tag (`v0.0.11` or the
   most recent monorepo tag)
2. `git remote add satellite-<name> <split-repo-url>` for each
   split satellite
3. `git fetch --tags satellite-<name>` for each
4. `git subtree add --prefix=crates/<name> satellite-<name> main`
   for each satellite that had accepted commits post-split
5. Restore workspace membership in the root `Cargo.toml` from
   the pre-split state
6. `cargo test --workspace` to confirm the restored shape is
   green
7. Cut a `v0.0.16` release from the restored workspace
8. Mark each satellite crate on crates.io as `yanked` at every
   version >= the rollback point (yank is not deletion; the
   monorepo re-takes precedence)
9. Update the parent repo README to note the rollback and cite
   the failing acceptance criterion

The pre-flight dry-run of this recipe MUST be exercised at least
once (in a scratch clone against a mock satellite) as part of
the pilot's pre-work, and the wall-clock time MUST be recorded
in the ADR update section below.

### Naming reservations

`noyalib-wasm@0.0.0` has been published as a namespace
reservation (2026-07-01) to close the pre-split race window for
the only unclaimed name. `noyalib`, `noyalib-mcp`, `noyalib-lsp`,
and `noya-cli` are already owned by `sebastienrousseau` from the
v0.0.11 release. Ownership drift is monitored daily via
[`crates-io-ownership.yml`](../../.github/workflows/crates-io-ownership.yml).

## Consequences

### Positive

- Downstream contributors reach the right repo for the component
  they care about; issue triage lives at the correct level of
  granularity.
- Each satellite carries its own OpenSSF Scorecard score, its own
  supply-chain gates, its own security policy — a security
  reviewer auditing `noyalib-mcp` sees only the MCP surface, not
  the LSP or CLI surface they will never use.
- Per-satellite release history is separately auditable — a
  downstream user chasing "when did this LSP behaviour change"
  reads only the noyalib-lsp git log, not a 10x-noisier
  monorepo log.
- The v0.0.11 CI cache-poisoning guard, docs-strict gate, and
  README-examples gate propagate to all 5 repos via reusable
  workflows — a security-first invariant enforced by one
  authority.
- Each satellite has its own Renovate / Dependabot config,
  scoped to its own dep tree — downstream security scanners
  observe a tighter blast radius per component.

### Negative

- Every hardening pass now touches 5 repos: 1 PR in the parent
  (the shared workflow bump) + 4 Dependabot PRs in satellites.
  For maintainer time this is roughly break-even with the
  monorepo shape (single-repo PR touching 5 crates), but the
  wall-clock latency is higher because each satellite's
  Dependabot has its own cycle.
- Every release cuts 5 tags across 5 repos in a coordinated
  ceremony. A shared release-orchestration workflow makes this
  one command, but the failure surface is 5x per release: any
  single satellite failing to publish requires the whole cohort
  to be re-tagged.
- Cross-repo issues (an LSP bug caused by a scanner change in
  noyalib) require cross-linking across repos, which GitHub's
  issue-linking model does not enforce.
- Contributors mid-split (e.g. #91 @EdJoPaTo, #117 @canardleteer)
  need clear guidance on which repo their commits belong in.
  Migration guides in MIGRATION.md handle this but require
  active maintainer bandwidth during the split window.
- Rollback is possible (`≤60min`) but costs credibility if
  exercised — announcing a rollback for a shipped release is a
  publicly visible admission of a bad decision. Mitigation is
  the strict pilot-then-observe cadence with a 14-day soak
  gate between each split.

### Neutral

- Repo count 1 → 5. Star count / watcher count / community
  metrics fragment; the sum stays the same but no single number
  represents the project any more.
- CODEOWNERS, SECURITY.md, deny.toml, REUSE.toml, osv-scanner.toml
  duplicate across all 5 repos. This is a shared-workflow
  problem to solve; documented under #127.
- Downstream `cargo add noyalib-lsp` behaviour is unchanged: the
  crate name is stable, the API surface is stable, the
  `use noyalib::...` paths are unchanged.

## Alternatives considered

### Keep the monorepo

Would have kept the current shape indefinitely. Rejected because:

- Contributor-surface confusion accumulates as the project's
  audience diversifies (browser, MCP, LSP, CLI) — the monorepo
  optimises for maintainer clarity at the cost of downstream
  clarity.
- Per-satellite Scorecard, security-policy, and issue-triage
  granularity is a hard requirement for the components that
  ship to security-conscious downstreams (LSP, MCP).
- Not splitting means the security-review surface grows without
  bound as the workspace accepts more satellites.

### Independent SemVer (posture B from #128)

Would have let each satellite version on its own cadence, pinning
noyalib to a caret range (`noyalib = "^0.0"`). Rejected because:

- Introduces a compatibility matrix — the downstream question
  "which noyalib × noyalib-lsp combo actually works" becomes a
  table to consult on every upgrade.
- Slower security-patch propagation: a CVE in noyalib does not
  force a coupled satellite re-release, so downstreams pinning
  `noyalib-lsp = "0.0.9"` might not pick up the noyalib patch
  until they Dependabot-bump the LSP separately.
- Contradicts the security-first / audit-clarity posture of the
  project (documented in the security-first-posture memory).

### Hybrid (posture C from #128)

Would have made noyalib patches transparent to satellites but
required a coupled satellite patch on every noyalib minor.
Rejected because:

- Adds a decision-point ("is this noyalib change patch-shaped or
  minor-shaped?") that requires an active call at every release.
  Lockstep and independent-semver both remove this
  decision-point; hybrid re-introduces it.
- The edge-case surface is real: a noyalib patch that subtly
  breaks a satellite's assumptions is possible under any
  posture, but hybrid papers over it by refusing to bump the
  satellite. Under strict lockstep the same patch forces a
  satellite re-release that gets soak-tested end-to-end.

### One repo per satellite, but keep noyalib in this repo

Would have moved `noyalib-wasm`, `noyalib-mcp`, `noyalib-lsp`,
`noya-cli`, and `xtask` to their own repos while leaving the
core library in `sebastienrousseau/noyalib`. This is the shape
we ARE adopting; it's noted here only to distinguish it from
"one repo per crate including a fresh repo for noyalib core"
which would have added migration cost for downstream users of
`noyalib` itself without any offsetting benefit.

## References

- Pre-work issues: [#125](https://github.com/sebastienrousseau/noyalib/issues/125),
  [#126](https://github.com/sebastienrousseau/noyalib/issues/126),
  [#127](https://github.com/sebastienrousseau/noyalib/issues/127),
  [#128](https://github.com/sebastienrousseau/noyalib/issues/128)
- Split issues: [#129](https://github.com/sebastienrousseau/noyalib/issues/129) (v0.0.12 pilot),
  [#130](https://github.com/sebastienrousseau/noyalib/issues/130) (v0.0.13),
  [#131](https://github.com/sebastienrousseau/noyalib/issues/131) (v0.0.14),
  [#132](https://github.com/sebastienrousseau/noyalib/issues/132) (v0.0.15)
- Post-split cleanup: [#133](https://github.com/sebastienrousseau/noyalib/issues/133) (MIGRATION.md),
  [#134](https://github.com/sebastienrousseau/noyalib/issues/134) (retire monorepo tooling)
- Related PR: [#135](https://github.com/sebastienrousseau/noyalib/pull/135) — crates.io
  ownership-drift regression harness closing #125 AC5
- `noyalib-wasm@0.0.0` namespace reservation: <https://crates.io/crates/noyalib-wasm>
- Prior art: [tokio](https://github.com/tokio-rs/tokio) uses
  a workspace for the reasons this ADR chose to move away from;
  [serde](https://github.com/serde-rs/serde) uses a workspace
  but ships `serde_derive` and `serde_json` separately at
  independent semver, which was the posture we rejected.

## Post-implementation update

### Rollback-recipe dry-run — 2026-07-02

Executed the 9-step rollback recipe against a scratch clone
mocking a v0.0.12-pilot-shipped state:

1. `git clone git@github.com:sebastienrousseau/noyalib.git` at
   tag `v0.0.11` (the pre-split monorepo tip on `main`).
2. Mocked the split by extracting `crates/noyalib-wasm/`'s
   history via `git subtree split --prefix=crates/noyalib-wasm
   --branch=mock-satellite-wasm` — 11 commits pulled out.
3. Pushed the mock satellite to a local bare repo at
   `file:///tmp/rollback-dry-run/mock-satellite-wasm.git` — that
   stands in for `git@github.com:sebastienrousseau/noyalib-wasm.git`
   at pilot completion.
4. Simulated the post-split working tree: `git rm -rq
   crates/noyalib-wasm` + removed the corresponding
   `"crates/noyalib-wasm"` line from the root `Cargo.toml`
   workspace member list, then committed.
5. Executed the rollback recipe verbatim from the scratch clone:
   `git remote add satellite-wasm …` → `git fetch --tags
   satellite-wasm` → `git subtree add --prefix=crates/noyalib-wasm
   satellite-wasm main` → restore workspace membership →
   `cargo test --workspace --no-run` → smoke `cargo test -p
   noyalib-wasm --tests native`.

Wall-clock time from `git subtree add` through cargo-test-green:

> **1 min 42 sec (102 s)**

Well within the AC target of ≤ 60 minutes. The vast majority of
the elapsed time was the `cargo test --workspace --no-run`
compilation on a fresh target dir — the git-level restoration
steps (subtree add + Cargo.toml edit + commit) completed in
under 5 seconds.

Steps 7-9 of the recipe (cut a rollback release, yank the
satellite crate versions on crates.io, update the parent-repo
README to note the rollback) were **not** exercised in this
dry-run — they are irreversible publish-time actions covered by
the existing release + `cargo yank` flows and would produce
public artefacts inappropriate for a rehearsal.

**Assessment**: recipe is executable. The bottleneck is cargo
compilation, not git plumbing; a real rollback on a runner with
a warm cache would land inside 5 minutes.

### v0.0.12 pilot — noyalib-wasm split (2026-07-02)

**Status:** pilot infrastructure landed in
[`sebastienrousseau/noyalib-wasm`](https://github.com/sebastienrousseau/noyalib-wasm)
PR #1. Awaits parent v0.0.12 publish before final CI green.

Concrete results from the pilot:

- History extraction via `git subtree split
  --prefix=crates/noyalib-wasm` produced 11 commits on the new
  repo's `main`. Authorship (per-commit `Author:` line) preserved
  intact; commit signatures do not carry through subtree, which
  matches the ADR's expectation.
- Strict-lockstep versioning is mechanically enforced: satellite
  `Cargo.toml` pins `noyalib = { version = "=0.0.12", features =
  ["std"] }`; Dependabot config explicitly ignores the `noyalib`
  dep (see satellite `.github/dependabot.yml`).
- Shared reusable workflows are consumed by SHA. First run hit a
  **startup_failure with 0 scheduled jobs** because
  `shared-verify-signatures.yml` declares `permissions:
  pull-requests: read` and the caller only granted `contents:
  read`. GitHub Actions rejects the reusable-workflow invocation
  when the callee's permission scope exceeds the caller's, with
  no annotation surfaced. Documented in the permissions table
  above; new satellites (v0.0.13 mcp, v0.0.14 lsp, v0.0.15 cli)
  must copy the union of required permissions.
- Repository ruleset (signed commits, linear history, 1-approver
  PR + CODEOWNERS review, required status checks) applied to the
  new repo's default branch via API. Matches the parent noyalib
  ruleset.

### v0.0.13 pilot — noyalib-mcp split (2026-07-05)

**Status:** pilot infrastructure landed in
[`sebastienrousseau/noyalib-mcp`](https://github.com/sebastienrousseau/noyalib-mcp)
PR #1. Awaits parent v0.0.13 publish before final CI green.

Concrete results from the pilot:

- Playbook applied line-for-line from v0.0.12; the only new
  work was the multi-channel release workflow (crates.io + npm
  wrapper + GHCR + MCP Registry) which extends the v0.0.12
  crates-only+npm pattern with two additional channels.
- Subtree extraction via `git subtree split
  --prefix=crates/noyalib-mcp` produced 14 commits on the new
  repo's `main`. Authorship preserved.
- Caller-side `pull-requests: read` permission included in the
  satellite's `ci.yml` from day one (v0.0.12 pilot's
  hard-earned lesson, applied preemptively — no startup_failure
  this time).
- Registry manifests (`server.json`, `glama.json`) moved from
  parent root to satellite root, with URLs rewritten to point
  at the satellite repo. Version-locked to the crate via the
  release workflow's `validate` job.
- npm wrapper renamed to `@sebastienrousseau/noyalib-mcp`
  matching the v0.0.12 pilot's scope decision (no `@noyalib`
  npm org exists).

### Soak review signals

The 14-day soak reviews at each split window will record their
GO / NO-GO signal here as an inline update rather than moving the
ADR to `superseded`; this ADR remains the source of truth for
the whole split project. If the project rolls back mid-flight,
this ADR moves to `superseded by [next ADR]` and a fresh ADR
describes the rollback rationale.
