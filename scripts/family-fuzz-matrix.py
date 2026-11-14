#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Discover all fuzz targets in pinned sibling ecosystem checkouts."""

import argparse
import importlib.util
import json
from pathlib import Path
import subprocess


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


CANDIDATE = module('family_candidate', 'prepare-ecosystem-candidate.py')
CAMPAIGN = module('family_campaign', 'run-fuzz-campaign.py')


def matrix(family):
    core = Path(family) / 'noyalib'
    sources = [{'repo': 'noyalib', 'commit': CANDIDATE.head(core)},
               *CANDIDATE.products(core / 'docs/ecosystem-candidate.json')]
    rows = []
    for source in sources:
        checkout = Path(family) / source['repo']
        if CANDIDATE.head(checkout) != source['commit']:
            raise ValueError(source['repo'] + ' commit differs from the candidate')
        unchanged = subprocess.run(
            ['git', '-C', str(checkout), 'diff', '--quiet', 'HEAD', '--', 'fuzz', 'fuzz-nostd'],
            check=False,
        )
        if unchanged.returncode:
            raise ValueError(source['repo'] + ' fuzz sources changed after checkout')
        targets = CAMPAIGN.inventory(checkout)
        if not targets:
            raise ValueError(source['repo'] + ' has no fuzz targets')
        rows.extend({**source, **target} for target in targets)
    return {'include': rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('family', type=Path)
    args = parser.parse_args()
    print(json.dumps(matrix(args.family), separators=(',', ':')))


if __name__ == '__main__':
    main()
