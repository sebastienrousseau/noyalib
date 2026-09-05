#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Documentation link gate: every chapter in docs/SUMMARY.md exists,
# and every relative Markdown link under docs/ resolves against the
# repository tree (parent traversal allowed — links that climb out
# of docs/ degrade to GitHub links in the rendered manual but must
# exist on disk). Replaces mdbook-linkcheck, which lags mdBook 0.5.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
python3 - <<'PY'
import os, re, sys
bad = []
summary = open('docs/SUMMARY.md', encoding='utf-8').read()
for m in re.finditer(r'\]\(([^)#\s]+\.md)\)', summary):
    p = os.path.normpath(os.path.join('docs', m.group(1)))
    if not os.path.exists(p):
        bad.append(f'SUMMARY.md -> {m.group(1)}')
for root, dirs, files in os.walk('docs'):
    for fn in files:
        if not fn.endswith('.md'):
            continue
        f = os.path.join(root, fn)
        text = open(f, encoding='utf-8', errors='ignore').read()
        for m in re.finditer(r'\]\((?!https?://|#|mailto:)([^)#\s]+)', text):
            target = m.group(1)
            # Skips: rustdoc pseudo-links quoted in release notes,
            # the ADR template's NNNN placeholder, and site-relative
            # links into the rustdoc half of the deployed Pages site
            # (exists on the website, not on disk).
            if target.startswith('crate::') or 'NNNN' in target or target.startswith('../noyalib/'):
                continue
            p = os.path.normpath(os.path.join(root, target))
            if not os.path.exists(p):
                bad.append(f'{f} -> {target}')
if bad:
    print('broken documentation links:')
    print('\n'.join(sorted(set(bad))))
    sys.exit(1)
print('docs links: all resolve')
PY
