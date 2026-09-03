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

# README install snippets naming this crate.
for rel in ("README.md", f"crates/{name}/README.md"):
    f = root / rel
    if not f.is_file():
        continue
    stale = set()
    for line in f.read_text(encoding="utf-8").splitlines():
        if re.search(rf"\b{re.escape(name)}\s*=\s*[\"{{]", line):
            stale.update(v for v in re.findall(r"\d+\.\d+\.\d+", line) if v != version)
        # The noyalib-serde-yaml drop-in snippet carries its own
        # `=0.0.X` pin (lockstep: must equal this release). Only the
        # pin syntax is checked so prose/MSRV mentions do not trip it.
        if "noyalib-serde-yaml" in line:
            stale.update(v for v in re.findall(r"=(\d+\.\d+\.\d+)", line) if v != version)
    if stale:
        bad(rel, "mentions " + ", ".join(sorted(stale)))
    else:
        ok(rel, "install snippets current")

print()
if failed:
    print("Version mismatch. Fix these before tagging — a tag that fails the")
    print("release workflow's Validate job must be deleted and recreated.")
    sys.exit(1)
print(f"All version-bearing files agree on {version}.")
PY
