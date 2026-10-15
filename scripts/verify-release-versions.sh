#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Noyalib. All rights reserved.
set -euo pipefail

# Assert every version-bearing file in this repository agrees with the
# crate version, before a tag exists.
#
# Written after v0.0.28. The release prep bumped Cargo.toml, CHANGELOG
# and README and missed `server.json` and `glama.json` in noyalib-mcp.
# The mismatch surfaced only after `git push origin v0.0.28`, in the
# release workflow's Validate job:
#
#     server.json version (0.0.27) does not match tag (0.0.28)
#
# Recoverable, because Validate runs before anything publishes — but it
# cost a tag deletion and a re-tag. Every check that job performs is
# reproduced here so the same mismatch fails locally in a second.
#
# Usage:
#   scripts/verify-release-versions.sh            # against Cargo.toml
#   scripts/verify-release-versions.sh 0.0.29     # against an intended version
#   scripts/verify-release-versions.sh v0.0.29    # a leading v is accepted

cd "$(cd "$(dirname -- "$0")/.." && pwd)"
exec python3 - "${1:-}" <<'PY'
import json, pathlib, re, sys

root = pathlib.Path.cwd()
wanted = sys.argv[1].lstrip("v") if len(sys.argv) > 1 and sys.argv[1] else ""

GREEN, RED, DIM, OFF = "\033[32m", "\033[31m", "\033[2m", "\033[0m"
failed = []


def ok(what, detail):
    print(f"  {GREEN}ok{OFF}    {what:<34} {detail}")


def bad(what, detail):
    print(f"  {RED}FAIL{OFF}  {what:<34} {detail}")
    failed.append(what)


def skip(what, detail):
    print(f"  {DIM}·     {what:<34} {detail}{OFF}")


def package_field(key):
    """Read a key from the [package] table of the crate's manifest."""
    for candidate in (root / "crates" / root.name / "Cargo.toml", root / "Cargo.toml"):
        if not candidate.is_file():
            continue
        text = candidate.read_text(encoding="utf-8")
        block = re.search(r"^\[package\](.*?)(?=^\[|\Z)", text, re.S | re.M)
        if not block:
            continue
        m = re.search(rf'^{key}\s*=\s*"([^"]+)"', block.group(1), re.M)
        if m:
            return m.group(1)
    return ""


version, name = package_field("version"), package_field("name")
if not version:
    sys.exit("Could not read the crate version from Cargo.toml")

if wanted and wanted != version:
    sys.exit(
        f"Cargo.toml says {version}, you asked for {wanted}.\n"
        "Bump Cargo.toml first — it is the source of truth."
    )

print(f"Verifying every version-bearing file against {name} {version}\n")


def locked_version(pkg):
    lock = root / "Cargo.lock"
    if not lock.is_file():
        return None
    m = re.search(rf'^name = "{re.escape(pkg)}"\nversion = "([^"]+)"',
                  lock.read_text(encoding="utf-8"), re.M)
    return m.group(1) if m else None


# Cargo.lock — this crate's own entry.
own = locked_version(name)
if own is None:
    skip("Cargo.lock", f"no entry for {name}")
elif own == version:
    ok("Cargo.lock", own)
else:
    bad("Cargo.lock", f"{own} — run cargo check to refresh")

# Satellites pin the core exactly; the pin and its lock entry must move
# together or CI, which builds --locked, fails on a stale lock.
manifest = (root / "Cargo.toml").read_text(encoding="utf-8") if (root / "Cargo.toml").is_file() else ""
pin = re.search(r'noyalib\s*=\s*\{\s*version\s*=\s*"=([^"]+)"', manifest)
if pin and name != "noyalib":
    if pin.group(1) == version:
        ok("Cargo.toml noyalib pin", f"={pin.group(1)}")
    else:
        bad("Cargo.toml noyalib pin", f"={pin.group(1)} — lockstep requires ={version}")
    core = locked_version("noyalib")
    if core is None:
        skip("Cargo.lock noyalib", "absent")
    elif core == version:
        ok("Cargo.lock noyalib", core)
    else:
        bad("Cargo.lock noyalib", f"{core} — stale; CI builds --locked")

# JSON manifests that carry a version of their own.
#
# Only the repository root is checked. A nested wrapper such as
# noyalib-mcp's pkg/npm-wrapper/package.json is deliberately excluded:
# its version is rewritten from the tag during publish
# (`npm version --allow-same-version "$TAG_VERSION"`), so the committed
# value is expected to lag and flagging it would be a false alarm.
for fname in ("server.json", "glama.json", "package.json"):
    f = root / fname
    if not f.is_file():
        continue
    try:
        got = json.loads(f.read_text(encoding="utf-8")).get("version", "")
    except json.JSONDecodeError as e:
        bad(fname, f"invalid JSON: {e}")
        continue
    if not got:
        skip(fname, "no version field")
    elif got == version:
        ok(fname, got)
    else:
        bad(fname, got)

# Container image tags embedded in those manifests move with the
# version, or the registry entry points at the previous image.
for fname in ("server.json", "glama.json"):
    f = root / fname
    if not f.is_file():
        continue
    for ref in sorted(set(re.findall(r"ghcr\.io/[\w./-]+:\d+\.\d+\.\d+", f.read_text(encoding="utf-8")))):
        (ok if ref.rsplit(":", 1)[1] == version else bad)(f"{fname} image tag", ref)

# The changelog must have promoted this version out of [Unreleased].
# CITATION.cff carries its own version field (added in the v0.0.31
# cycle); a release prep that skips it ships a stale citation.
citation = root / "CITATION.cff"
if citation.is_file():
    text = citation.read_text(encoding="utf-8")
    m = re.search(r"^version: (\S+)$", text, re.M)
    if m and m.group(1) == version:
        ok("CITATION.cff", f"version {m.group(1)}")
    else:
        bad("CITATION.cff", f"says {m.group(1) if m else 'nothing'} — update the version field")

changelog = root / "CHANGELOG.md"
if changelog.is_file():
    if re.search(rf"^## \[v?{re.escape(version)}\]", changelog.read_text(encoding="utf-8"), re.M):
        ok("CHANGELOG.md", f"has a [v{version}] section")
    else:
        bad("CHANGELOG.md", f"no [v{version}] heading — still under [Unreleased]?")

# Install snippets naming this crate: the READMEs and every page under
# docs/ (the v0.0.33 cycle found GETTING_STARTED.md and two migration
# guides carrying snippets the gate never read).
snippet_files = [root / "README.md", root / "crates" / name / "README.md"]
snippet_files += sorted((root / "docs").rglob("*.md")) if (root / "docs").is_dir() else []
for f in snippet_files:
    rel = str(f.relative_to(root))
    # Release notes and decision records quote the versions of their
    # own day; they are history, not install instructions.
    if not f.is_file() or "release-notes" in rel or "/adr/" in rel:
        continue
    stale = set()
    for line in f.read_text(encoding="utf-8").splitlines():
        if re.search(rf"\b{re.escape(name)}\s*=\s*[\"{{]", line):
            stale.update(v for v in re.findall(r"\d+\.\d+\.\d+", line) if v != version)
        # Satellite crates pin the core exactly and move with it
        # (ADR-0005 strict lockstep), so a snippet telling a reader to
        # depend on one must name this release too. Matched by shape
        # rather than by an explicit list: a hardcoded list only had
        # noyalib-serde-yaml in it, and MIGRATION-WORKSPACE-SPLIT.md
        # sat on `noyalib-wasm = "=0.0.14"` for 31 releases because
        # nothing looked at the other satellites. `tag = "vX.Y.Z"` on
        # a git dependency is caught by the same line scan.
        if re.search(r"\b(?:noyalib-[a-z][a-z-]*|noya-[a-z][a-z-]*)\s*=\s*[\"{]", line):
            stale.update(v for v in re.findall(r"\d+\.\d+\.\d+", line) if v != version)
        # A published image tag and a git tag are install instructions
        # too. MIGRATION-WORKSPACE-SPLIT.md told readers to `cosign
        # verify ...:0.0.14` and to `git subtree add ... v0.0.14` in
        # the same breath as an up-to-date crates.io pin, because the
        # gate only ever looked at Cargo dependency syntax.
        stale.update(
            v for v in re.findall(r"ghcr\.io/[\w./-]+:(\d+\.\d+\.\d+)", line)
            if v != version
        )
        stale.update(
            v for v in re.findall(
                r"github\.com/[\w.-]+/(?:noyalib|noyalib-[a-z-]+|noya-[a-z-]+)\s+v(\d+\.\d+\.\d+)",
                line,
            )
            if v != version
        )
    if stale:
        bad(rel, "mentions " + ", ".join(sorted(stale)))
    else:
        ok(rel, "install snippets current")

# docs/ECOSYSTEM.md's rating table is generated by
# `scripts/ecosystem-scorecard.sh --inject`, and the document opens by
# promising that every number in it is produced by a command. Nothing
# enforced that: the weekly workflow gates the *live* score against a
# floor and never compares it to the *committed* table, so the table sat
# at 0.0.33 for thirty-one releases while the gate stayed green.
#
# Only the `tree X.Y.Z` figures are checked. They are the family's own
# versions, which move in lockstep, so a release that did not regenerate
# the table shows up here immediately. The volatile figures — download
# counts, OpenSSF scores, and the `crates.io X.Y.Z` column, which
# legitimately lags the tree between the core publish and the satellite
# publishes — are deliberately not gated: a check that fails on honest
# drift gets switched off, and then nothing is checked at all.
eco = root / "docs" / "ECOSYSTEM.md"
if eco.is_file():
    trees = sorted(set(re.findall(r"tree (\d+\.\d+\.\d+)", eco.read_text(encoding="utf-8"))))
    stale = [t for t in trees if t != version]
    if not trees:
        skip("docs/ECOSYSTEM.md", "no generated rating table")
    elif stale:
        bad("docs/ECOSYSTEM.md", "rating table measured at "
            + ", ".join(stale)
            + " — regenerate with scripts/ecosystem-scorecard.sh --network --inject docs/ECOSYSTEM.md")
    else:
        ok("docs/ECOSYSTEM.md", f"rating table measured at {version}")

# The MSRV. `rust-version` in Cargo.toml is the floor Cargo enforces;
# every other place that states it (CI, make targets, README, policy
# docs) must name the same floor, or a raise leaves a stale promise
# behind. 1.89 and 1.89.0 state the same floor. Each site must still
# match its pattern, so a reworded statement fails here instead of
# silently dropping out of the check.
def same_floor(stated, floor):
    def norm(v):
        return v + ".0" if v.count(".") == 1 else v
    return norm(stated) == norm(floor)


MSRV_SITES = [
    (".github/workflows/ci.yml", r'^\s+toolchain: "([0-9.]+)"'),
    (".github/workflows/shared-msrv-core.yml", r'^\s+default: "([0-9.]+)"'),
    (".github/PULL_REQUEST_TEMPLATE.md", r"Considered MSRV \(([0-9.]+)\)"),
    ("CONTRIBUTING.md", r"\*\*Rust ([0-9.]+)\+\*\* for the core"),
    ("CONTRIBUTING.md", r"Same \*\*([0-9.]+)\*\* floor"),
    ("DEVELOPMENT.md", r"\| Rust stable \| ([0-9.]+) or later"),
    ("DEVELOPMENT.md", r"\| `msrv-core` \| `cargo \+([0-9.]+) check`"),
    ("GOVERNANCE.md", r"\(MSRV\) is \*\*([0-9.]+)\*\*"),
    ("SUPPORT.md", r"\(MSRV\): \*\*([0-9.]+)\*\*"),
    ("README.md", r"badge/MSRV-([0-9.]+)-"),
    ("README.md", r'alt="MSRV ([0-9.]+)"'),
    ("README.md", r"Rust \*\*([0-9.]+) or newer\*\*"),
    ("README.md", r"^\| (?:`noyalib`|Satellite crates) \| Rust ([0-9.]+) \|"),
    ("crates/noyalib/README.md", r"\*\*MSRV: Rust ([0-9.]+)\*\*"),
    ("crates/noyalib/src/lib.rs", r"Rust \*\*([0-9.]+)\*\* stable"),
    ("crates/noyalib/src/lib.rs", r"on `rustc ([0-9.]+)` for every PR"),
    ("crates/noyalib/src/lib.rs", r"the same `([0-9.]+)` floor"),
    ("docs/POLICIES.md", r"^\| `noya[a-z-]*`(?: \([a-z ]+\))? \| \*{0,2}([0-9.]+)\*{0,2} \|"),
    ("docs/README-REFERENCE.md", r"badge/MSRV-([0-9.]+)-"),
    ("docs/README-REFERENCE.md", r'alt="MSRV ([0-9.]+)"'),
    ("docs/README-REFERENCE.md", r"\*\*Rust ([0-9.]+) or newer\.\*\*"),
    ("docs/README-REFERENCE.md", r"^\| `noya[a-z-]*`(?: \([a-z ]+\))? \| \*{0,2}([0-9.]+)\*{0,2} \|"),
    ("docs/diagrams/dependency-graph.md", r'MSRV ([0-9.]+)"\]'),
    ("docs/diagrams/dependency-graph.md", r"^\| `noya[a-z-]*` \| \*{0,2}([0-9.]+)\*{0,2} \|"),
    ("docs/MSRV-AND-DEPRECATION.md", r"\*\*Current MSRV: ([0-9.]+)\*\*"),
    ("docs/GLOSSARY.md", r"floor of \*\*([0-9.]+)\*\*"),
    ("docs/GETTING_STARTED.md", r"\*\*Rust ([0-9.]+)\+\*\*"),
    ("docs/PGO.md", r"\*\*rustc ([0-9.]+)\+\*\*"),
    ("docs/PGO.md", r"noyalib's MSRV \(([0-9.]+)\)"),
    ("docs/packaging.md", r"currently \*\*([0-9.]+)\*\*"),
    ("docs/BENCHMARKS.md", r"\| \*\*MSRV\*\* \| Rust ([0-9.]+)"),
    ("docs/PLAN.md", r'live values are edition 2024, `rust-version = "([0-9.]+)"`'),
]

msrv = package_field("rust-version")
if not msrv:
    skip("MSRV", "no rust-version in Cargo.toml")
else:
    for rel, pattern in MSRV_SITES:
        f = root / rel
        label = f"MSRV {rel}"
        if not f.is_file():
            bad(label, "file is missing")
            continue
        found = [v.rstrip(".") for v in re.findall(pattern, f.read_text(encoding="utf-8"), re.M)]
        if not found:
            bad(label, f"no statement matches {pattern!r}")
            continue
        stale = sorted({v for v in found if not same_floor(v, msrv)})
        if stale:
            bad(label, f"says {', '.join(stale)}, Cargo.toml says {msrv}")
        else:
            ok(label, f"{msrv} ({len(found)} statement{'s' if len(found) > 1 else ''})")

print()
if failed:
    print("Version mismatch. Fix these before tagging — a tag that fails the")
    print("release workflow's Validate job must be deleted and recreated.")
    sys.exit(1)
print(f"All version-bearing files agree on {version}.")
PY
