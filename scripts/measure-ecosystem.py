#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Prepare and measure comparable ecosystem benchmarks with source-bound receipts."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess


BENCHES = {'noyalib': 'streaming_vs_value', 'noya-cli': 'cli_dispatch',
           'noyalib-lsp': 'lsp_handlers', 'noyalib-mcp': 'mcp_tools',
           'noyalib-serde-yaml': 'shim_overhead', 'noyalib-wasm': None}


def capture(command, cwd):
    return subprocess.check_output(command, cwd=cwd, text=True).strip()


def source(root):
    tracked = subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).decode().split('\0')
    inputs = sorted(p for p in tracked if p and (p.startswith(('src/', 'crates/', 'benches/'))
                    or p in ('Cargo.toml', 'Cargo.lock')) and (root / p).is_file())
    digest = hashlib.sha256()
    for name in inputs:
        digest.update(name.encode() + b'\0' + hashlib.sha256((root / name).read_bytes()).digest())
    return {'head_sha': capture(['git', 'rev-parse', 'HEAD'], root),
            'dirty': bool(capture(['git', 'status', '--porcelain'], root)),
            'inputs_sha256': digest.hexdigest(),
            'lockfile_sha256': hashlib.sha256((root / 'Cargo.lock').read_bytes()).hexdigest(),
            'rustc': capture(['rustc', '-Vv'], root), 'cargo': capture(['cargo', '-V'], root)}


def execute(command, root, env, log):
    with log.open('w') as stream:
        subprocess.run(command, cwd=root, env=env, stdout=stream, stderr=subprocess.STDOUT, check=True)


def prepare(root, output, repos):
    output.mkdir(parents=True, exist_ok=True)
    record = output / 'prepared.json'
    # A targeted rebuild must preserve the other verified artifacts.
    prepared = json.loads(record.read_text()) if record.is_file() else {}
    core_source = source(root / "noyalib")
    for name in repos:
        directory = root / name
        bench = BENCHES[name]
        target = output / ('build-' + name)
        env = {**os.environ, 'CARGO_TARGET_DIR': str(target)}
        log = output / (name + '-build.log')
        if bench:
            command = ['cargo', 'bench', '--locked', '--bench', bench, '--no-run', '--message-format=json']
            execute(command, directory, env, log)
            artifacts = []
            for line in log.read_text(errors='replace').splitlines():
                if line.startswith('{'):
                    message = json.loads(line)
                    if (message.get('reason') == 'compiler-artifact' and message.get('executable')
                            and message['target']['name'] == bench):
                        artifacts.append(message['executable'])
            if len(artifacts) != 1:
                raise ValueError(f'{name}: expected one executable for {bench}, got {artifacts}')
            artifact = Path(artifacts[0])
            command_to_measure = [str(artifact), '--bench', '--warm-up-time', '1',
                                  '--measurement-time', '2', '--sample-size', '20', '--noplot']
        else:
            package = output / 'wasm-node-package'
            command = ['wasm-pack', 'build', '--release', '--target', 'nodejs',
                       '--out-dir', str(package), '--', '--locked']
            execute(command, directory, env, log)
            artifact = package / 'noyalib_wasm_bg.wasm'
            command_to_measure = ['node', 'benches/bench.js']
        prepared[name] = {'source': source(directory), 'core_source': core_source,
                          'build_command': command,
                          'measurement_command': command_to_measure, 'artifact': str(artifact),
                          'artifact_bytes': artifact.stat().st_size,
                          'artifact_sha256': hashlib.sha256(artifact.read_bytes()).hexdigest()}
        record.write_text(json.dumps(prepared, indent=2) + '\n')
        print(f'{name}: built {artifact.name}', flush=True)


def summarize_criterion(criterion):
    groups = {}
    for path in sorted(criterion.rglob('new/estimates.json')):
        key = str(path.parent.parent.relative_to(criterion))
        estimates = json.loads(path.read_text())
        median = estimates['median']['point_estimate']
        if not math.isfinite(median) or median <= 0:
            raise ValueError(f'invalid median for {key}')
        groups[key] = {'median_ns': median, 'mean_ns': estimates['mean']['point_estimate'],
                       'mean_confidence_interval_ns': estimates['mean']['confidence_interval']}
    if not groups:
        raise ValueError('Criterion emitted no measurements')
    return groups


def peak_rss(log):
    text = log.read_text(errors='replace')
    if platform.system() == 'Darwin':
        found = re.findall(r'(\d+)\s+maximum resident set size', text)
        return int(found[-1]) if found else None
    found = re.findall(r'Maximum resident set size \(kbytes\):\s*(\d+)', text)
    return int(found[-1]) * 1024 if found else None


def measure(root, output, reps):
    prepared = json.loads((output / 'prepared.json').read_text())
    host = {'os': platform.system(), 'release': platform.release(), 'arch': platform.machine(),
            'logical_cpus': os.cpu_count()}
    if platform.system() == 'Darwin':
        try:
            host['cpu'] = subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'],
                                                  cwd=root, stderr=subprocess.DEVNULL, text=True).strip()
        except (OSError, subprocess.CalledProcessError):
            host['cpu'] = 'unavailable'
    report = {'started_utc': datetime.now(timezone.utc).isoformat(), 'host': host,
              'repetitions': reps, 'repositories': {}}
    for name, row in prepared.items():
        directory = root / name
        if source(root / 'noyalib')['inputs_sha256'] != row['core_source']['inputs_sha256']:
            raise ValueError(f'{name}: core source changed since build')
        if source(directory)['inputs_sha256'] != row['source']['inputs_sha256']:
            raise ValueError(f'{name}: benchmark source changed since build')
        artifact = Path(row['artifact'])
        if hashlib.sha256(artifact.read_bytes()).hexdigest() != row['artifact_sha256']:
            raise ValueError(f'{name}: benchmark artifact changed since build')
        runs = []
        for repetition in range(1, reps + 1):
            work = output / name / f'run-{repetition}'
            work.mkdir(parents=True, exist_ok=False)
            env = {**os.environ, 'CRITERION_HOME': str(work / 'criterion'),
                   'NOYALIB_WASM_NODE_PKG': str(output / 'wasm-node-package'),
                   'NOYALIB_WASM_BENCH_JSON': str(work / 'wasm.json')}
            load = os.getloadavg()
            timing = ['/usr/bin/time', '-l' if platform.system() == 'Darwin' else '-v']
            execute([*timing, *row['measurement_command']], directory, env, work / 'run.log')
            results = (summarize_criterion(work / 'criterion') if BENCHES[name]
                       else json.loads((work / 'wasm.json').read_text())['benchmarks'])
            runs.append({'load_average': load, 'peak_rss_bytes': peak_rss(work / 'run.log'),
                         'raw_directory': str(work.relative_to(output)), 'results': results})
            print(f'{name}: measurement {repetition}/{reps}', flush=True)
        variance = {}
        for label in runs[0]['results']:
            samples = [run['results'][label]['median_ns'] for run in runs]
            median = statistics.median(samples)
            variance[label] = {'median_ns': median, 'min_ns': min(samples), 'max_ns': max(samples),
                               'relative_range': (max(samples) - min(samples)) / median}
        report['repositories'][name] = {**row, 'runs': runs, 'repeatability': variance}
        (output / 'baseline.json').write_text(json.dumps(report, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['prepare', 'measure'])
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--repo', action='append', choices=BENCHES)
    parser.add_argument('--repetitions', type=int, default=3)
    args = parser.parse_args()
    if not 3 <= args.repetitions <= 10:
        parser.error('use three to ten repetitions')
    root, output = args.root.resolve(), args.output.resolve()
    if args.phase == 'prepare':
        prepare(root, output, args.repo or list(BENCHES))
    else:
        measure(root, output, args.repetitions)


if __name__ == '__main__':
    main()
