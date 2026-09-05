# Summary

[Introduction](index.md)

# Using noyalib

- [User guide](USER-GUIDE.md)
- [Choosing a YAML library](COMPARISON.md)
- [Benchmarks](BENCHMARKS.md)
- [Profile-guided optimisation](PGO.md)

# Migrating to noyalib

- [Migration overview](MIGRATION.md)
- [From serde_yaml](MIGRATION-FROM-SERDE-YAML.md)
- [From serde_yaml_ng](MIGRATION-FROM-SERDE-YAML-NG.md)
- [From serde_yml](MIGRATION-FROM-SERDE-YML.md)
- [From serde-saphyr](MIGRATION-FROM-SERDE-SAPHYR.md)
- [From serde-norway](MIGRATION-FROM-SERDE-NORWAY.md)
- [From serde-yaml-bw](MIGRATION-FROM-SERDE-YAML-BW.md)
- [From yaml-serde](MIGRATION-FROM-YAML-SERDE.md)
- [From yaml-spanned](MIGRATION-FROM-YAML-SPANNED.md)

# Under the hood

- [Architecture](ARCHITECTURE.md)
  - [The green tree](design/green-tree.md)
  - [Dependency graph](diagrams/dependency-graph.md)
- [Testing and verification](TESTING.md)

# Policies

- [Engineering policies](POLICIES.md)
- [MSRV and deprecation](MSRV-AND-DEPRECATION.md)
- [CII best practices](CII-BEST-PRACTICES.md)
- [Pre-commit hooks](pre-commit.md)
- [Packaging for distributions](packaging.md)

# The ecosystem

- [Ecosystem map and scorecard](ECOSYSTEM.md)

# Decisions

- [Architecture decision records](adr/README.md)
  - [0001 — CST rowan shape](adr/0001-cst-rowan-shape.md)
  - [0002 — YAML 1.2 by default](adr/0002-yaml-1.2-default.md)
  - [0003 — Zero-unsafe policy](adr/0003-zero-unsafe-policy.md)
  - [0004 — Lossless u64 integers](adr/0004-lossless-u64-integers.md)
  - [0005 — Workspace split](adr/0005-workspace-split.md)
  - [0006 — Plain-scalar strings opt-in](adr/0006-plain-scalar-strings-opt-in.md)
  - [0007 — prefer_single_quotes option](adr/0007-prefer-single-quotes-option.md)
  - [0008 — Compiled schema](adr/0008-compiled-schema.md)
  - [0009 — set_path parent creation](adr/0009-set-path-parent-creation.md)
  - [0010 — Typed collection set_value](adr/0010-typed-collection-set-value.md)
  - [0011 — Flow inserts and anchor policy](adr/0011-flow-inserts-and-anchor-policy.md)
  - [0012 — Quoted path segments](adr/0012-quoted-path-segments.md)

# Releases

- [Release notes](release-notes/README.md)
