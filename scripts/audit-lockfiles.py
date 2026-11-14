#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Audit every tracked Cargo lockfile, including fuzz and standalone test crates."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
from tempfile import TemporaryDirectory


ADVISORY_URL = "https://github.com/RustSec/advisory-db.git"


def git_head(directory):
    if not directory.is_dir():
        return None
    result = subprocess.run(['git', 'rev-parse', '--verify', 'HEAD'], cwd=directory,
                            capture_output=True, text=True)
    return result.stdout.strip() if result.returncode == 0 else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path.cwd())
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--no-fetch', action='store_true',
                        help='use the cached advisory DB; record its identity without claiming a refresh')
    parser.add_argument('--db', type=Path,
                        default=Path(os.environ.get('CARGO_HOME', Path.home() / '.cargo')) / 'advisory-db')
    args = parser.parse_args()
    root = args.root.resolve()
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    files = subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).decode().split('\0')
    locks = sorted(p for p in files if p and Path(p).name == 'Cargo.lock')
    report = {
        'generated_utc': datetime.now(timezone.utc).isoformat(),
        'source_commit': git_head(root),
        'worktree_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root)),
        'refresh_requested': not args.no_fetch,
        'manifests': sorted(p for p in files if p and Path(p).name == 'Cargo.toml'),
        'node_manifests': sorted(p for p in files if p and Path(p).name == 'package.json'),
        'node_lockfiles': sorted(p for p in files if p and Path(p).name in
                                 ['package-lock.json', 'npm-shrinkwrap.json', 'yarn.lock', 'pnpm-lock.yaml']),
        'lockfiles': [],
    }
    with TemporaryDirectory(prefix='noyalib-advisory-db-') as temp:
        database = Path(temp) / 'snapshot'
        source = str(args.db.resolve()) if args.no_fetch else ADVISORY_URL
        clone = subprocess.run(
            ['git', 'clone', '--quiet', '--no-local', '--depth', '1', '--no-tags', '--', source, str(database)],
            cwd=root, capture_output=True, text=True)
        db_commit = git_head(database)
        if clone.returncode or not db_commit:
            report['advisory_db_error'] = clone.stderr.strip() or 'advisory database has no source commit'
            for lock in locks:
                report['lockfiles'].append({'lockfile': lock, 'status': 'unavailable',
                                            'error': report['advisory_db_error']})
                print(f'{lock}: unavailable', flush=True)
            outcome = 2
        else:
            report['advisory_db_commit'] = db_commit
            outcome = audit(root, locks, database, db_commit, report)
    (output / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    return outcome


def audit(root, locks, database, db_commit, report):
    outcome = 0 if locks else 2
    for lock in locks:
        path = root / lock
        manifest = path.with_name('Cargo.toml')
        row = {'lockfile': lock, 'manifest': str(Path(lock).with_name('Cargo.toml'))}
        command = ['cargo', 'audit', '--json', '--deny', 'unsound', '--file', lock, '--db', str(database)]
        command.append('--no-fetch')
        row['command'] = command
        try:
            if not manifest.is_file():
                raise ValueError('tracked lockfile has no adjacent manifest')
            row['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
            row['manifest_sha256'] = hashlib.sha256(manifest.read_bytes()).hexdigest()
            result = subprocess.run(command, cwd=root, capture_output=True, text=True)
            row['exit_code'] = result.returncode
            row['stderr'] = result.stderr
            row['report'] = json.loads(result.stdout)
            reported = row['report'].get('database', {}).get('last-commit')
            row['advisory_db_commit'] = db_commit
            if git_head(database) != db_commit or (reported and reported != db_commit):
                raise ValueError('advisory database changed during the scan')
            if hashlib.sha256(path.read_bytes()).hexdigest() != row['sha256']:
                raise ValueError('lockfile changed during the scan')
            vulnerabilities = row['report']['vulnerabilities']
            warnings = row['report'].get('warnings', {})
            row['status'] = 'failed' if (result.returncode or vulnerabilities['found']
                                        or warnings.get('unsound')) else 'passed'
            if row['status'] == 'failed':
                outcome = max(outcome, 1)
        except (OSError, ValueError, KeyError, TypeError) as error:
            row['status'] = 'unavailable'
            row['error'] = str(error)
            outcome = 2
        report['lockfiles'].append(row)
        print(f"{lock}: {row['status']}", flush=True)
    return outcome


if __name__ == '__main__':
    raise SystemExit(main())
