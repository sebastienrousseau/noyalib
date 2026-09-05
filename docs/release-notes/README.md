<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Release notes

Per-release narrative notes, newest first. One file per tag, named
exactly for the tag it documents — `v0.0.17.md` is `v0.0.17` — so a
version is enough to find its notes without consulting this index.

These complement [`CHANGELOG.md`](../../CHANGELOG.md) rather than
duplicating it. The changelog is the complete, structured record of every
release under [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
these files carry the narrative for the releases that had one — why a
change was made, what it broke, what to do about it.

Notes exist for **v0.0.1 – v0.0.17**. From **v0.0.18** onward the
changelog entries carry that narrative themselves, so there are no
separate files; see the changelog for those releases.

| Version | Date | Notes |
|---|---|---|
| [`v0.0.17`](v0.0.17.md) | 2026-07-25 | Lockstep-only release; no code change in the core crate. |
| [`v0.0.16`](v0.0.16.md) | 2026-07-24 | Build fix, MSRV raise, and dependency refresh — `main` had been left unbuildable under `cargo check --all-targets`. |
| [`v0.0.15`](v0.0.15.md) | 2026-07-12 | Completes loader parity and hardens coverage. |
| [`v0.0.14`](v0.0.14.md) | 2026-07-09 | The loader-parity cut; closes a fast-path silent-collapse bug. |
| [`v0.0.13`](v0.0.13.md) | 2026-07-05 | Splits `noyalib-mcp`, `noyalib-lsp` and `noya-cli` into their own repositories. |
| [`v0.0.12`](v0.0.12.md) | 2026-07-03 | The workspace-split pilot; `noyalib-wasm` leaves the monorepo first. |
| [`v0.0.11`](v0.0.11.md) | 2026-07-01 | CI-integrity housekeeping. |
| [`v0.0.10`](v0.0.10.md) | 2026-06-30 | Scanner correctness for UTF-8 BOM-prefixed multi-node documents. |
| [`v0.0.9`](v0.0.9.md) | unreleased | Supply-chain refresh batching eight Dependabot updates. Never tagged. |
| [`v0.0.8`](v0.0.8.md) | 2026-06-17 | The `FlowStyle` fix. |
| [`v0.0.7`](v0.0.7.md) | 2026-06-02 | Supply-chain hardening. |
| [`v0.0.6`](v0.0.6.md) | 2026-05-30 | Ecosystem integration — the four remaining v0.0.6 milestone issues. |
| [`v0.0.5`](v0.0.5.md) | 2026-05-11 | Polish and stabilisation: edition 2024 and MSRV 1.85. |
| [`v0.0.4`](v0.0.4.md) | 2026-05-11 | Opens the v0.0.4 cycle with the `!include` directive. |
| [`v0.0.3`](v0.0.3.md) | 2026-05-11 | Surgical patch widening the `rustc-hash` dependency range. |
| [`v0.0.2`](v0.0.2.md) | 2026-05-10 | Zero-copy borrowed deserialisation and lazy parsing. |
| [`v0.0.1`](v0.0.1.md) | 2026-05-10 | First publishable release — a pure-Rust YAML 1.2 implementation with full serde integration. |

## Conventions

- **Filename equals tag.** `docs/release-notes/<tag>.md`. The mapping is
  one-to-one with git tags and GitHub releases, so a link can be
  constructed from a version alone.
- **Notes are immutable.** A published file records what was true at
  that release. Corrections belong in a later release's notes or in the
  changelog, not in edits to history.
- **The changelog is the index of record.** Every release appears there;
  only some have a narrative file here.

> `v0.0.9` was never tagged. Its notes are kept for continuity of the
> record; the published sequence goes `v0.0.8` → `v0.0.10`.
