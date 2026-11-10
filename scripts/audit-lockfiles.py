#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Audit every tracked Cargo lockfile, including fuzz and standalone test crates."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path.cwd())
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--no-fetch', action='store_true',
                        help='use the cached advisory DB; record its identity without claiming a refresh')
    args = parser.parse_args()
    root = args.root.resolve()
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    files = subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).decode().split('\0')
    locks = sorted(p for p in files if p and Path(p).name == 'Cargo.lock')
    revision = subprocess.run(['git', 'rev-parse', '--verify', 'HEAD'], cwd=root,
                              capture_output=True, text=True)
    report = {
        'generated_utc': datetime.now(timezone.utc).isoformat(),
        'source_commit': revision.stdout.strip() if revision.returncode == 0 else None,
        'worktree_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root)),
        'refresh_requested': not args.no_fetch,
        'manifests': sorted(p for p in files if p and Path(p).name == 'Cargo.toml'),
        'node_manifests': sorted(p for p in files if p and Path(p).name == 'package.json'),
        'node_lockfiles': sorted(p for p in files if p and Path(p).name in
                                 ['package-lock.json', 'npm-shrinkwrap.json', 'yarn.lock', 'pnpm-lock.yaml']),
        'lockfiles': [],
    }
    outcome = 0 if locks else 2
    for index, lock in enumerate(locks):
        path = root / lock
        row = {'lockfile': lock, 'manifest': str(Path(lock).with_name('Cargo.toml')),
               'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
        command = ['cargo', 'audit', '--json', '--deny', 'unsound', '--file', lock]
        if args.no_fetch or index:
            command.append('--no-fetch')
        row['command'] = command
        try:
            if not path.with_name('Cargo.toml').is_file():
                raise ValueError('tracked lockfile has no adjacent manifest')
            result = subprocess.run(command, cwd=root, capture_output=True, text=True)
            row['exit_code'] = result.returncode
            row['report'] = json.loads(result.stdout)
            row['stderr'] = result.stderr
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
    (output / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    return outcome


if __name__ == '__main__':
    raise SystemExit(main())
