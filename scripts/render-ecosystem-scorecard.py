#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Render saved scorecard observations without rerunning probes."""

import argparse
import json
from pathlib import Path
import re


def cell(value):
    return str(value).replace('|', r'\|').replace('\n', ' ')


def render(report):
    lines = [
        '',
        f"**Rating: {report['grade']} ({report['weighted_score'] * 100:.1f}% weighted).** "
        f"{report['probes_measured']} measured; {report['probes_na']} N/A.",
        '',
        f"Measured {report['generated_utc']} on `{report['host']}`, {report['rustc']}. "
        f"Rubric executed: {report['rubric_executed_pct']}%.",
        '',
        '| Repository | Version | Source | Worktree |',
        '|---|---|---|---|',
    ]
    for repo, source in report['repos'].items():
        if not re.fullmatch(r'[a-f0-9]{40}', source['sha']):
            raise ValueError(f'{repo}: report has no full source SHA')
        lines.append(f"| `{cell(repo)}` | {cell(source['version'])} | "
                     f"`{source['sha']}` | {cell(source['worktree'])} |")
    lines.extend(['', '| Repo | Metric | Measured | Score | Probe |', '|---|---|---|---:|---|'])
    for row in report['measurements']:
        score = row['score']
        if score is not None and not 0 <= score <= 1:
            raise ValueError(f"invalid score for {row['id']}")
        label = 'n/a' if score is None else f'{score:.2f}'
        lines.append(f"| `{cell(row['repo'])}` | {cell(row['id'])} | {cell(row['value'])} | "
                     f"{label} | `{cell(row['probe'])}` |")
    lines.extend(['', f"<sub>Recorded by the scorecard harness v{report['harness_version']}. "
                  "Render the saved JSON rather than editing the table.</sub>", ''])
    return '\n'.join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report', type=Path)
    parser.add_argument('markdown', type=Path)
    args = parser.parse_args()
    report = json.loads(args.report.read_text())
    text = args.markdown.read_text()
    pattern = r'(^<!-- SCORECARD:BEGIN[^\n]*-->\n).*?(^<!-- SCORECARD:END[^\n]*-->)'
    matches = list(re.finditer(pattern, text, flags=re.M | re.S))
    if len(matches) != 1:
        parser.error('expected one generated scorecard block')
    block = matches[0]
    args.markdown.write_text(text[:block.start()] + block[1] + render(report) + block[2] + text[block.end():])


if __name__ == '__main__':
    main()
