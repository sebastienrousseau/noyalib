<!--
SPDX-FileCopyrightText: 2026 Noyalib
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Repository standard compliance grade

This assessment applies the eight-category rubric in
`/Users/seb/Code/REPO-STANDARD.md` to the repository state on 2026-09-20. Scores
measure checked-in evidence and CI enforcement. They do not treat plans as
completed work.

## Result

The portfolio auditor independently records **20/24 signals (83%)**, template
status **pass**, and strict candidate tier **L1** for this working tree. The
weighted assessment below adds qualitative evidence that the static signal
scanner cannot establish by filename matching alone.

| Category | Score | Evidence | Remaining gap |
| :--- | ---: | :--- | :--- |
| Identity and README | 9/10 | Canonical README structure, verified version snippets, security and stability sections, license and MSRV badges | Add a dedicated CI assertion for the canonical heading sequence |
| Documentation | 10/10 | `docs/`, mdBook deployment, `DEVELOPMENT.md`, architecture, ADRs, migrations, strict link checks | None against the current rubric |
| Build and install UX | 9/10 | Native Cargo flow, discoverable Makefile, examples, docs, SBOM, and offline tasks | The binary install contract belongs to `noya-cli`; this library has no `GNUmakefile` |
| Releases and binaries | 10/10 | Tag release workflow, mandatory signed-tag verification, trusted registry publication, checksums, Sigstore, SLSA, and CycloneDX SBOM | None against the current rubric |
| Packaging and distribution | 7/10 | Maintainer packaging guide, Debian and Fedora submission records, plus a fresh-runner byte-for-byte `.crate` reproduction gate | No completed multi-distribution tracking |
| Quality gates in CI | 10/10 | OS and toolchain matrices, strict lint/docs, coverage floors, feature powerset, fuzz replay, examples, benchmarks, and API checks | None against the current rubric |
| Supply chain and security | 9/10 | Private disclosure, dependency review, audit/deny/vet, pinned actions, Scorecard, keys, REUSE, SLSA, and SBOM | Record a live Scorecard result of at least 9 as release evidence |
| Community and governance | 9/10 | Governance files, templates, citation, agent rules, editor and pre-commit policy, docs lint, devcontainer, family scorecard | Measure and gate the sub-minute devcontainer boot requirement |

**Total: 73/80 (9.1/10). Candidate tier: L1, with substantial L2 and L3 evidence.**

The repository cannot claim L2 or L3 because the tier rubric is cumulative and
requires every category. Packaging and distribution currently block L2;
a recorded Scorecard floor and measured devcontainer startup remain L3 gaps.
The engineering and supply-chain gates already exceed many individual L2 and
L3 requirements, but partial evidence does not unlock a tier.

## Priority actions

1. Add a cheap README-template conformance check to the documentation CI job.
2. Record Repology or equivalent distribution coverage once two distributions
   track the relevant package.
3. Measure devcontainer startup in CI and gate it below 60 seconds.

Re-run this grade only after the associated CI evidence exists. A document or
workflow name by itself does not close a requirement.
