# Contributing to noyalib

Contributions are welcome. This guide covers the essentials.

## Prerequisites

- **Rust 1.86.0+** for the core `noyalib` crate
- Same **1.86.0** floor for `noya-cli` and `noyalib-lsp`
  (their dep trees include edition-2024 transitives — see
  [`docs/diagrams/dependency-graph.md`](./docs/diagrams/dependency-graph.md))
- Git with [commit signing](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits) configured

## Getting Started

```sh
git clone https://github.com/sebastienrousseau/noyalib.git
cd noyalib
make
```

`make` runs `cargo check`, `cargo clippy`, and `cargo test` in sequence.

## Branch naming

Use Conventional Commits-flavoured prefixes. The branch prefix tells
reviewers what kind of change to expect before they open the diff:

| Prefix | Use when… | Example |
|---|---|---|
| `feat/` | adding user-visible behaviour or public API | `feat/yaml-1.1-mode` |
| `fix/` | fixing a bug whose behaviour change is observable | `fix/scanner-empty-key-panic` |
| `perf/` | speeding up code without changing behaviour | `perf/swar-decimal-parse` |
| `refactor/` | restructuring code without behaviour change | `refactor/extract-resolver` |
| `docs/` | docs-only changes | `docs/adr-cst-shape` |
| `test/` | tests-only changes | `test/cover-merge-key-edge-cases` |
| `ci/` | CI / build / packaging changes | `ci/pin-toolchain-1.86` |
| `chore/` | dependency bumps and similar housekeeping | `chore/bump-criterion-0.5` |

The current release branch is `feat/v0.0.1`. Open PRs against that
branch (or `main` once v0.0.1 ships).

## Making changes

1. Fork the repository and create a branch using the prefixes above.
2. Write code. Match the local style of the file you're touching —
   read 2–3 neighbours before introducing a new pattern.
3. **Add or update tests in the same commit / PR** as the
   behaviour change. Never as a follow-up.
4. Run the full check suite:

```sh
make           # check + clippy + test
make fmt       # rustfmt --check
make deny      # cargo-deny supply-chain audit
```

5. Commit with `git commit -S` (signed).

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/) format.
The scope is the crate or subsystem touched:

```
<type>(<scope>): <imperative summary>

<optional body explaining the why>

<optional footer with breaking-change notes, issue refs>
```

Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `ci`,
`chore`, `build`, `revert`.

Scopes: crate name (`noyalib`, `noyalib-lsp`, `noyalib-mcp`,
`noyalib-wasm`, `noya-cli`, `xtask`) or subsystem (`parser`,
`compat`, `cst`, `bench`).

Examples:

```
feat(parser): YAML 1.1 mode via `version()` toggle
fix(scanner): clamp `key_end` to `sk.index` to prevent slice panic
perf(simd): SWAR 8-byte stride for plain-scalar boundary scan
docs(adr): add 0003 zero-unsafe-policy
test(borrowed): cover anchor namespace reset between documents
```

Sign every commit (`git commit -S`). Unsigned commits won't be
merged. Set up GPG or SSH signing per the
[GitHub guide](https://docs.github.com/en/authentication/managing-commit-signature-verification).

## Pull requests

- Open against the current release branch (`feat/v0.0.1` for now).
- Title follows the same Conventional Commits format as commits.
- Body includes:
  - **What changed** in 1–3 bullets
  - **Why** in plain English
  - **Test plan** — what the reviewer should expect green
- Keep PRs focused. One logical change per PR; mechanical
  refactors and behaviour changes get separate PRs for blame
  hygiene.
- CI must be green: clippy `-D warnings`, all tests, formatter,
  REUSE compliance, supply-chain audit, MSRV per-crate, Miri
  focused pass, coverage gate, fuzz smoke. See
  [`docs/TESTING.md`](./docs/TESTING.md) for the full layer breakdown.
- One approval is required to merge.

## Code standards

- `#![forbid(unsafe_code)]` workspace-wide. **No `unsafe` blocks,
  ever.** See [ADR 0003](./docs/adr/0003-zero-unsafe-policy.md).
- All public items require documentation (`#![warn(missing_docs)]`).
- Public docstring rule: lead with one-line summary; include
  `# Examples` with working code; include `# Errors` for fallible
  functions; include `# Panics` if any path can panic; include
  `# Safety` for unsafe (we don't have any, so this never applies).
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` must pass.
- `cargo fmt --check` must pass.
- New behaviour ships with new tests *in the same commit*.
- New deps must come with a one-line rationale in the commit
  message body. A new lockfile entry is a code change.

## Architectural decisions

For changes that touch the parse output shape, the public API
surface, the dependency floor, or core invariants like the unsafe
policy: write an [ADR](./docs/adr/) before opening the PR. The
template lives at [`docs/adr/TEMPLATE.md`](./docs/adr/TEMPLATE.md).

The bar is "would I want a future contributor to read this before
proposing the opposite?" — if yes, ADR. If no, commit message
suffices.

## Reporting issues

Open an issue on GitHub. Include:

- A minimal YAML input that reproduces the problem.
- Expected behaviour vs. actual behaviour.
- Rust version (`rustc --version`).
- noyalib version (`grep '^version' crates/noyalib/Cargo.toml`).

For security issues, **do not file a public issue.** See
[SECURITY.md](./SECURITY.md) for the disclosure process.

## License

By contributing, you agree that contributions are licensed under
the same dual license as the project: [MIT](LICENSE-MIT) or
[Apache 2.0](LICENSE-APACHE).
