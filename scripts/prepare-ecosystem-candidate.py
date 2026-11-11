#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Prepare an unpublished family checkout without changing registry pins."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

PRODUCTS = ('noya-cli', 'noyalib-lsp', 'noyalib-mcp', 'noyalib-wasm', 'noyalib-serde-yaml')


def products(path):
    rows = json.loads(path.read_text())['products']
    if (len(rows) != len(PRODUCTS)
            or {row['repo'] for row in rows} != set(PRODUCTS)
            or any(not re.fullmatch('[0-9a-f]{40}', row['commit']) for row in rows)):
        raise ValueError('candidate must pin each of the five products to a full commit')
    return rows


def head(repo):
    return subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare(core, satellite, repo, commit):
    core, satellite = core.resolve(), satellite.resolve()
    if repo not in PRODUCTS or not re.fullmatch('[0-9a-f]{40}', commit) or head(satellite) != commit:
        raise ValueError('satellite commit does not match the pinned candidate')
    core_package = tomllib.loads((core / 'crates/noyalib/Cargo.toml').read_text())['package']
    product_manifest = tomllib.loads((satellite / 'Cargo.toml').read_text())
    package = product_manifest['package']
    version = core_package['version']
    if package['name'] != repo or package['version'] != version:
        raise ValueError('candidate package name or version does not match the core')
    dependency = product_manifest['dependencies']['noyalib']
    if isinstance(dependency, dict):
        if any(key in dependency for key in ('git', 'path', 'registry')):
            raise ValueError('candidate requires an exact core pin from crates.io')
        dependency = dependency['version']
    if dependency != '=' + version:
        raise ValueError('candidate requires an exact core pin')
    config = satellite / '.cargo/config.toml'
    if config.exists() or config.with_suffix('').exists():
        raise ValueError('refusing to overwrite existing Cargo configuration')
    locks = {label: digest(root / 'Cargo.lock')
             for label, root in (('core', core), ('satellite', satellite))}
    patch = '[patch.crates-io]\nnoyalib = { path = ' + json.dumps(str(core / 'crates/noyalib')) + ' }\n'
    config.parent.mkdir(exist_ok=True)
    with config.open('x') as stream:
        stream.write(patch)
    return {'core': str(core), 'satellite': str(satellite), 'repo': repo, 'version': version,
            'core_commit': head(core), 'satellite_commit': commit, 'locks': locks,
            'override_sha256': digest(config), 'registry_resolution_proof': False}


def verify(receipt):
    for label in ('core', 'satellite'):
        root = Path(receipt[label])
        if head(root) != receipt[label + '_commit']:
            raise ValueError(label + ' commit changed during candidate validation')
        if digest(root / 'Cargo.lock') != receipt['locks'][label]:
            raise ValueError(label + ' lockfile changed during candidate validation')
        subprocess.run(['git', '-C', str(root), 'diff', '--exit-code', 'HEAD'], check=True)
    config = Path(receipt['satellite']) / '.cargo/config.toml'
    if digest(config) != receipt['override_sha256']:
        raise ValueError('candidate override changed during validation')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest='mode', required=True)
    matrix = modes.add_parser('matrix')
    matrix.add_argument('manifest', type=Path)
    setup = modes.add_parser('prepare')
    setup.add_argument('--core', type=Path, required=True)
    setup.add_argument('--satellite', type=Path, required=True)
    setup.add_argument('--repo', required=True, choices=PRODUCTS)
    setup.add_argument('--commit', required=True)
    setup.add_argument('--receipt', type=Path, required=True)
    check = modes.add_parser('verify')
    check.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    if args.mode == 'matrix':
        print(json.dumps(products(args.manifest), separators=(',', ':')))
    elif args.mode == 'prepare':
        receipt = prepare(args.core, args.satellite, args.repo, args.commit)
        args.receipt.write_text(json.dumps(receipt, indent=2) + '\n')
        print('Prepared exact-source candidate; registry publication gates remain separate.')
    else:
        verify(json.loads(args.receipt.read_text()))
        print('Candidate sources, registry pins, locks and local override remain unchanged.')


if __name__ == '__main__':
    main()
