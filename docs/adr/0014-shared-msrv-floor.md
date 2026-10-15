# 0014. Share one MSRV floor, 1.89.0, with the maintainer's other crates

- **Status:** proposed
- **Date:** 2026-10-13
- **Authors:** Noyalib contributors

## Context

The lockstep family (noyalib, noya-cli, noyalib-lsp, noyalib-mcp,
noyalib-serde-yaml, noyalib-wasm) declared 1.86.0, except noyalib-mcp,
whose MCP SDK needs 1.88.0. The maintainer's other Rust crates sit at
several floors between 1.56 and 1.92, which makes the support promise
different for every crate a user combines.

`docs/POLICIES.md` §1 says the MSRV moves only when the toolchain the
project builds and tests at moves, never for tidiness. Nothing in the
family needs 1.89: `criterion 0.8` (a dev-dependency) sets the
toolchain-driven floor at 1.86, and the MCP SDK sets noyalib-mcp's at
1.88.

## Decision

Every crate in the family declares `rust-version = "1.89.0"`, the floor
the maintainer chose for all of his Rust crates. This is a maintainer
decision, made explicitly, and is the only raise recorded without a
toolchain reason. The rule in `docs/POLICIES.md` §1 stands for every
later raise.

## Consequences

- **Positive:** one floor across the family and the maintainer's other
  crates; noyalib-mcp no longer differs from the rest; the opt-in
  `compare-saphyr` feature (1.88+) is now inside the floor.
- **Negative:** users on 1.86, 1.87 or 1.88 lose new releases with no
  technical gain. They can stay on v0.0.57.
- **Neutral:** the 12-month guarantee holds (1.89.0 shipped
  2025-08-07).

## Alternatives considered

### Keep 1.86.0 (1.88.0 for noyalib-mcp)

Would have followed the written policy. Rejected by the maintainer in
favour of one floor across all of his crates.

## References

- `docs/POLICIES.md` §1, `docs/MSRV-AND-DEPRECATION.md`
- The v0.0.16 raise from 1.85 to 1.86 (its CHANGELOG entry)
