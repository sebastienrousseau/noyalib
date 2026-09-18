#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Generate docs/errors.md and docs/internals.md from the source.
#
# WHY THIS EXISTS
#
# Both documents are inventories: every error variant, every module.
# Hand-written, an inventory is wrong the first time someone adds an
# item and forgets the doc — and nothing notices, because prose has no
# compiler. Generating them from the source means the source is the one
# place a thing is declared.
#
# The generator is not the guarantee, though: someone can forget to run
# it. `tests/reference_docs_are_complete.rs` is the guarantee — it reads
# the source and the documents and fails when they disagree. Run this,
# commit the result, and the test confirms you did.
#
#   bash scripts/generate-reference-docs.sh

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

exec python3 - <<'PY'
import pathlib, re, subprocess

SRC = pathlib.Path("crates/noyalib/src")
error_rs = (SRC / "error.rs").read_text()


def doc_comment(lines, i):
    """Collect the `///` block immediately above line `i`, minus examples."""
    out, j = [], i - 1
    while j >= 0 and lines[j].strip().startswith("///"):
        out.append(lines[j].strip()[3:].strip())
        j -= 1
    out.reverse()
    cut = next((k for k, l in enumerate(out) if l.startswith("# ")), len(out))
    return " ".join(x for x in out[:cut] if x).strip()


def variants(block_name):
    m = re.search(rf"^pub enum {block_name}\b.*?\{{(.*?)^\}}", error_rs, re.S | re.M)
    body = m.group(1)
    lines = body.splitlines()
    found = []
    for i, line in enumerate(lines):
        vm = re.match(r"^    ([A-Z][A-Za-z0-9]*)\s*(\{|\(|,)", line)
        if vm:
            found.append((vm.group(1), doc_comment(lines, i)))
    return found


kinds = variants("ErrorKind")
errs = variants("Error")

# `code()` is the stable string a tool matches on; read it from the impl.
codes = dict(re.findall(r'Self::([A-Za-z0-9]+)[^=]*=>\s*"(noyalib::[a-z_]+)"', error_rs))

out = ["<!-- SPDX-FileCopyrightText: 2026 Noyalib -->",
       "<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->",
       "",
       "# Error reference",
       "",
       "Every error `noyalib` can return, what raises it, and the stable",
       "code a tool can match on.",
       "",
       "**Generated** by `scripts/generate-reference-docs.sh` from",
       "`crates/noyalib/src/error.rs`. Do not edit by hand: re-run the",
       "script. `tests/reference_docs_are_complete.rs` fails when this file",
       "and the source disagree, so a new variant cannot land undocumented.",
       "",
       "## Kinds",
       "",
       "`Error::kind()` collapses the variants below into a small set, so",
       "callers can route on a category without matching a `#[non_exhaustive]`",
       "enum.",
       "",
       "| Kind | Meaning |",
       "| --- | --- |"]
for name, doc in kinds:
    out.append(f"| `ErrorKind::{name}` | {doc or '—'} |")

out += ["", "## Variants", "",
        f"{len(errs)} variants. `code()` is the stable identifier exposed",
        "through `miette::Diagnostic`; it is part of the public surface and",
        "changes only in a breaking release.",
        "",
        "| Variant | `code()` | Raised when |",
        "| --- | --- | --- |"]
for name, doc in errs:
    code = codes.get(name, "`noyalib::error`")
    out.append(f"| `Error::{name}` | `{code}` | {doc or '—'} |")

out += ["", "## Reading an error against its source", "",
        "Every located variant renders with the offending line when given the",
        "input it came from:", "",
        "```rust",
        "# use noyalib::Value;",
        "let input = \"port: [unclosed\";",
        "let err = noyalib::from_str::<Value>(input).unwrap_err();",
        "println!(\"{}\", err.format_with_source(input));",
        "```", "",
        "`render`, `format_with_source_radius` and",
        "`format_with_source_truncated` are the same idea with control over",
        "how much context is shown. See the API reference for the full set.",
        ""]
pathlib.Path("docs/errors.md").write_text("\n".join(out))
print(f"docs/errors.md: {len(kinds)} kinds, {len(errs)} variants")

# ── internals.md ────────────────────────────────────────────────
mods = []
for p in sorted(SRC.rglob("*.rs")):
    rel = p.relative_to(SRC)
    if rel.name == "lib.rs":
        continue
    text = p.read_text()
    head = ""
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("//!"):
            candidate = s[3:].strip()
            if candidate:
                head = candidate
                break
        elif s and not s.startswith("//"):
            break
    loc = len(text.splitlines())
    mods.append((str(rel), loc, head))

# Hot paths: the longest functions, which are where the time goes and
# where a change is most likely to cost something.
hot = subprocess.run(
    ["python3", "-c", r'''
import os, re, sys
def strip(src):
    out=[];i=0;n=len(src)
    while i<n:
        c=src[i]
        if c=="/" and i+1<n and src[i+1]=="/":
            j=src.find("\n",i); j=n if j<0 else j; out.append(" "*(j-i)); i=j
        elif c=='"':
            j=i+1
            while j<n:
                if src[j]=="\\": j+=2; continue
                if src[j]=='"': j+=1; break
                j+=1
            out.append("".join(ch if ch=="\n" else " " for ch in src[i:j])); i=j
        else: out.append(c); i+=1
    return "".join(out)
rows=[]
for root,_,fs in os.walk("crates/noyalib/src"):
    for f in fs:
        if not f.endswith(".rs"): continue
        p=os.path.join(root,f); lines=strip(open(p).read()).split("\n")
        for i,l in enumerate(lines):
            m=re.match(r"^\s*(pub(\(\w+\))?\s+)?(async\s+)?(const\s+)?fn\s+([A-Za-z0-9_]+)", l)
            if not m: continue
            d=0;st=False;end=i
            for j in range(i,len(lines)):
                for ch in lines[j]:
                    if ch=="{": d+=1;st=True
                    elif ch=="}":
                        d-=1
                        if st and d==0: break
                if st and d==0: end=j;break
            if st: rows.append((end-i+1, p.replace("crates/noyalib/",""), m.group(5)))
rows.sort(reverse=True)
for n,p,f in rows[:12]: print(f"{n}\t{p}\t{f}")
'''],
    capture_output=True, text=True).stdout.strip().splitlines()

out = ["<!-- SPDX-FileCopyrightText: 2026 Noyalib -->",
       "<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->",
       "",
       "# Internals",
       "",
       "A map of the crate for people changing it. Read",
       "[ARCHITECTURE.md](ARCHITECTURE.md) first for *why* the pieces are",
       "shaped this way; this is *where* they are.",
       "",
       "**Generated** by `scripts/generate-reference-docs.sh`. Do not edit by",
       "hand. `tests/reference_docs_are_complete.rs` fails when a module",
       "exists that this map does not list.",
       "",
       "## Hot paths",
       "",
       "The longest functions in the crate. Length is a proxy, not a verdict —",
       "a scanner's token dispatch is long because a YAML token has many",
       "shapes — but these are where the time goes and where a change is most",
       "likely to cost something. The `benches/` directory measures them.",
       "",
       "| Lines | Location | Function |",
       "| --- | --- | --- |"]
for row in hot:
    n, path, fn = row.split("\t")
    out.append(f"| {n} | `{path}` | `{fn}` |")

out += ["", "## Module map", "",
        f"{len(mods)} modules.", "",
        "| Module | Lines | Purpose |",
        "| --- | --- | --- |"]
for rel, loc, head in mods:
    out.append(f"| `{rel}` | {loc} | {head or '—'} |")
out.append("")
pathlib.Path("docs/internals.md").write_text("\n".join(out))
print(f"docs/internals.md: {len(mods)} modules, {len(hot)} hot paths")
PY
