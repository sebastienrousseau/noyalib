#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Run a bounded fuzz campaign and retain its source, inputs and actual outcome."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(root):
    rows = []
    for scope in ['fuzz', 'fuzz-nostd']:
        manifest = root / scope / 'Cargo.toml'
        if manifest.is_file():
            for target in tomllib.loads(manifest.read_text()).get('bin', []):
                name = target['name']
                if not re.fullmatch(r'[A-Za-z][A-Za-z0-9_-]*', name):
                    raise ValueError(f'invalid fuzz target name: {name!r}')
                rows.append({'scope': scope, 'target': name})
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--matrix', action='store_true')
    parser.add_argument('--scope', choices=['fuzz', 'fuzz-nostd'], default='fuzz')
    parser.add_argument('--target')
    parser.add_argument('--seconds', type=int, default=3600)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    root = Path(subprocess.check_output(['git', 'rev-parse', '--show-toplevel'], text=True).strip())
    targets = inventory(root)
    if args.matrix:
        if not targets:
            parser.error('no fuzz targets found')
        print(json.dumps({'include': targets}))
        return 0
    if not args.output or not args.target or not 1 <= args.seconds <= 3600:
        parser.error('provide --output, --target and a duration from 1 to 3600 seconds')
    if {'scope': args.scope, 'target': args.target} not in targets:
        parser.error('target is not declared in the selected fuzz manifest')
    out = args.output.resolve()
    if out == root or root in out.parents:
        parser.error('place campaign output outside the source checkout')
    out.mkdir(parents=True, exist_ok=False)
    corpus = out / 'corpus'
    artifacts = out / 'artifacts'
    corpus.mkdir()
    artifacts.mkdir()
    fuzz = root / args.scope
    seeds = fuzz / 'corpus/seed'
    regressions = fuzz / 'regressions' / args.target
    inputs = [directory for directory in [seeds, regressions] if directory.is_dir()]
    input_hashes = {str(file.relative_to(root)): digest(file)
                    for directory in inputs for file in sorted(directory.rglob('*')) if file.is_file()}
    lock = fuzz / 'Cargo.lock'
    lock_hash = digest(lock)
    command = ['cargo', '+nightly', 'fuzz', 'run', '--fuzz-dir', args.scope, args.target,
               str(corpus), *map(str, inputs), '--', f'-max_total_time={args.seconds}',
               '-timeout=10', '-rss_limit_mb=1024', f'-artifact_prefix={artifacts}{os.sep}']
    report = {'started_utc': datetime.now(timezone.utc).isoformat(),
              'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
              'worktree_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root)),
              'rustc': subprocess.check_output(['rustc', '+nightly', '-Vv'], cwd=root, text=True).strip(),
              'scope': args.scope, 'target': args.target, 'requested_seconds': args.seconds,
              'initial_inputs_sha256': input_hashes, 'lockfile_sha256': lock_hash, 'command': command}
    start = time.monotonic()
    log = out / 'campaign.log'
    env = {**os.environ, 'CARGO_TARGET_DIR': str(out / 'target')}
    with log.open('w') as stream:
        result = subprocess.run(command, cwd=root, env=env, stdout=stream, stderr=subprocess.STDOUT)
    raw = log.read_text(errors='replace')
    done = re.findall(r'Done (\d+) runs in (\d+) second\(s\)', raw)
    executions, seconds = map(int, done[-1]) if done else (0, 0)
    locks_match = digest(lock) == lock_hash
    passed = result.returncode == 0 and executions > 0 and seconds >= args.seconds and locks_match
    report.update(status='passed' if passed else 'failed', exit_code=result.returncode,
                  executions=executions, measured_seconds=seconds, wall_seconds=time.monotonic() - start,
                  lockfile_unchanged=locks_match, log_sha256=digest(log),
                  findings=sorted(file.name for file in artifacts.iterdir() if file.is_file()))
    (out / 'receipt.json').write_text(json.dumps(report, indent=2) + '\n')
    print(f'{args.scope}/{args.target}: {report["status"]}; {executions} executions in {seconds}s; {out}')
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
