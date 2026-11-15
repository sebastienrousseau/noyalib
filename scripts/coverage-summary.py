#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Read measured line coverage from an llvm-cov JSON summary, never a column guess."""

import json
import math
from pathlib import Path
import sys


def line_coverage(path):
    report = json.loads(Path(path).read_text())
    if report.get('type') != 'llvm.coverage.json.export' or len(report['data']) != 1:
        raise ValueError('expected one combined llvm-cov export')
    lines = report['data'][0]['totals']['lines']
    count, covered, percent = lines['count'], lines['covered'], lines['percent']
    if type(count) is not int or type(covered) is not int:
        raise ValueError('coverage counts must be integers')
    if count < 0 or not 0 <= covered <= count:
        raise ValueError('invalid covered line count')
    if type(percent) not in (int, float) or not math.isfinite(percent):
        raise ValueError('coverage percentage must be finite')
    if not 0 <= percent <= 100:
        raise ValueError('coverage percentage out of range')
    if count == 0:
        raise ValueError('no coverable lines; not a measured percentage')
    measured = covered * 100 / count
    if abs(measured - percent) > 0.02:
        raise ValueError('coverage percentage disagrees with counts')
    return measured


if __name__ == '__main__':
    try:
        if len(sys.argv) != 2:
            raise ValueError('usage: coverage-summary.py REPORT.json')
        print(f'{line_coverage(sys.argv[1]):.6g}')
    except (OSError, ValueError, KeyError, TypeError, IndexError) as error:
        print(f'coverage unavailable: {error}', file=sys.stderr)
        sys.exit(1)
