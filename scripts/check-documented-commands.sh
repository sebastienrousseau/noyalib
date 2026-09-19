#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Every shell command the documentation tells a reader to run must
# resolve to something that exists.
#
# WHY THIS EXISTS
#
# `check-readme-examples.sh` compiles the Rust blocks and
# `check-install-guides.sh` builds the dependency lines. Nothing looked
# at the `sh` blocks — so `cargo xtask pgo-build` sat in README.md, and
# `cargo xtask vendor` in pkg/VERIFY.md's supply-chain reproduction
# steps, for 31 releases after `crates/xtask/` was deleted. Both had
# been failing with "package ID specification `xtask` did not match any
# packages" the entire time.
#
# Checked, for each fenced sh/bash/console block:
#
#   make <target>        the target exists in the Makefile
#   scripts/<x>.sh       the file exists
#   cargo <subcommand>   a builtin, a known third-party subcommand, or
#                        an alias defined in .cargo/config.toml
#
#   bash scripts/check-documented-commands.sh

set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

exec python3 - <<'PY'
import pathlib, re, subprocess, sys

# History and roadmaps quote the commands of their own day, exactly as
# verify-release-versions.sh excludes them for version numbers.
SKIP = ("CHANGELOG.md", "docs/release-notes/", "docs/adr/", "docs/PLAN.md")

# Third-party cargo subcommands the docs legitimately tell a reader to
# `cargo install`. Add one here only after checking it is a real,
# installable command — the point of this list is that anything NOT on
# it is presumed a typo or a retired tool.
KNOWN_CARGO = {
    # builtins
    "add", "bench", "build", "check", "clean", "clippy", "doc", "fetch",
    "fix", "fmt", "generate-lockfile", "help", "init", "install",
    "locate-project", "login", "metadata", "new", "owner", "package",
    "pkgid", "publish", "remove", "report", "run", "rustc", "rustdoc",
    "search", "test", "tree", "uninstall", "update", "vendor",
    "verify-project", "version", "yank",
    # third-party, all named in the docs and all installable
    "about", "audit", "auditable", "deb", "deny", "depgraph", "fuzz",
    "generate-rpm", "hack", "insta", "kani", "llvm-cov", "machete",
    "miri", "msrv", "nextest", "semver-checks", "spellcheck", "udeps",
    "vet", "wix", "workspaces",
}

root = pathlib.Path(".")
makefile = (root / "Makefile").read_text() if (root / "Makefile").is_file() else ""
targets = set(re.findall(r"^([A-Za-z_][\w.-]*)\s*:", makefile, re.M))

cargo_cfg = root / ".cargo" / "config.toml"
aliases = set()
if cargo_cfg.is_file():
    text = cargo_cfg.read_text()
    block = re.search(r"^\[alias\](.*?)(?=^\[|\Z)", text, re.S | re.M)
    if block:
        aliases = set(re.findall(r"^([a-z][\w-]*)\s*=", block.group(1), re.M))

bad = []
scanned = 0


def command_lines(text):
    """Every place the docs put a runnable command.

    Fenced sh blocks are the obvious half. The other half is an inline
    code span in a paragraph — which is where `cargo xtask pgo-build`
    actually sat in README.md, so a gate that reads only fences misses
    the very defect it was written for. A span counts only when it
    *starts* with a command word, so `Cargo.toml` and prose spans do
    not match.
    """
    for _lang, block in re.findall(r"```(sh|bash|console|shell)\n(.*?)```", text, re.S):
        for raw in block.splitlines():
            line = raw.strip()
            # A comment is prose, not an instruction: "# cargo will
            # fetch ..." must not read as a `cargo will` invocation.
            if line and not line.startswith("#"):
                yield line.lstrip("$ ").strip()
    # Strip fenced blocks first so their contents are not re-read here.
    prose = re.sub(r"```.*?```", "", text, flags=re.S)
    for span in re.findall(r"`([^`\n]+)`", prose):
        span = span.strip()
        if re.match(r"^(cargo|make|\.?/?scripts/)\s*", span):
            yield span


# Tracked files only. Walking the tree also picked up build output —
# `target-book/` is not matched by a "/target/" test — so the set of
# scanned files, and the count reported, changed with whatever happened
# to be built. A gate whose input depends on leftover artefacts is not
# a gate.
tracked = subprocess.run(["git", "ls-files", "*.md"],
                         capture_output=True, text=True, check=True).stdout.split()
for rel in sorted(tracked):
    path = pathlib.Path(rel)
    if not path.is_file() or any(s in rel for s in SKIP):
        continue
    text = path.read_text(errors="replace")
    # Some documents describe a *sibling repository's* commands —
    # docs/packaging.md explains noya-cli's `make install` / `make
    # assets`, which correctly do not exist in this Makefile. Such a
    # file opts out explicitly rather than the check being loosened for
    # everyone, so the exemption is visible in the file it applies to.
    if "commands-gate: external" in text:
        continue
    scanned += 1
    if True:
        for line in command_lines(text):
            # `[A-Za-z_]` not `[a-z]`: a Makefile target may start with
            # an underscore, and the first version of this pattern let
            # `make __no_such_target__` through its own self-test.
            for t in re.findall(r"(?:^|&&\s*|\|\s*)make\s+([A-Za-z_][\w.-]*)", line):
                if t not in targets:
                    bad.append(f"{rel}: `make {t}` is not a Makefile target")

            for sh in re.findall(r"(?:^|[\s(])\.?/?(scripts/[\w./-]+\.sh)", line):
                if not (root / sh).is_file():
                    bad.append(f"{rel}: `{sh}` does not exist")

            # Anchored to a command position: start of line, or after a
            # pipe / && / env assignments, so prose never matches.
            m = re.match(
                r"^(?:[A-Z_][A-Z0-9_]*=\S*\s+)*cargo\s+(?:\+\S+\s+)?([a-z][a-z0-9-]*)",
                line,
            )
            if m:
                sub = m.group(1)
                if sub not in KNOWN_CARGO and sub not in aliases:
                    bad.append(
                        f"{rel}: `cargo {sub}` is neither a known subcommand "
                        f"nor an alias in .cargo/config.toml"
                    )

if bad:
    print("documented commands that do not resolve:")
    for b in sorted(set(bad)):
        print(f"  {b}")
    sys.exit(1)
print(f"documented commands: all resolve ({scanned} files scanned)")
PY
