#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Exercise the CLI manifest with pinned Scoop in an isolated Windows prefix."""

import argparse
import datetime
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

SCOOP_SOURCE = 'e6aa3b366bdee8ed138c1e0f7b85192ebdd35d0f'
RESTORE_PATH = r'''param([string]$ReceiptPath, [string]$ShimDirectory)
$ErrorActionPreference = 'Stop'
$before = Get-Content -Raw -LiteralPath $ReceiptPath | ConvertFrom-Json
$machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
if ($machine -cne $before.machine) { throw 'Machine PATH changed unexpectedly; refusing to overwrite it.' }
$current = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($current -ceq $before.user) { exit 0 }
$remaining = @($current -split ';' | Where-Object { $_ -and ($_ -ine $ShimDirectory) })
$expected = @($before.user -split ';' | Where-Object { $_ })
if (($remaining -join ';') -cne ($expected -join ';')) {
    throw 'User PATH changed beyond the test shim directory; refusing to overwrite it.'
}
[Environment]::SetEnvironmentVariable('Path', $before.user, 'User')
'''


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if os.name != 'nt':
        parser.error('Scoop acceptance requires Windows')
    cli, output = args.cli.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    record = json.loads((cli / 'pkg/release-artifacts.json').read_text())
    artifact = record['artifacts']['x86_64-pc-windows-msvc']
    prefix = output / 'scoop'
    manager = prefix / 'apps/scoop/current'
    manager.parent.mkdir(parents=True)
    for directory in ['buckets', 'shims', 'persist']:
        (prefix / directory).mkdir()
    env = os.environ.copy()
    env.update(SCOOP=str(prefix), SCOOP_GLOBAL=str(output / 'global'),
               SCOOP_CACHE=str(output / 'cache'), XDG_CONFIG_HOME=str(output / 'config'))
    seven_zip = Path(r'C:\Program Files\7-Zip')
    if seven_zip.exists():
        env['PATH'] = str(seven_zip) + os.pathsep + env['PATH']
    report = {
        'status': 'failed',
        'recorded_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'cli_source': subprocess.check_output(
            ['git', '-C', str(cli), 'rev-parse', 'HEAD'], text=True).strip(),
        'consumer_sha256': digest(Path(__file__)),
        'manifest_sha256': digest(cli / 'pkg/scoop/noya-cli.json'),
        'release_version': record['version'], 'release_source': record['source'],
        'manager_repo': 'https://github.com/ScoopInstaller/Scoop',
        'manager_commit': SCOOP_SOURCE, 'target': 'x86_64-pc-windows-msvc',
        'scope': 'Actual candidate manifest through pinned Scoop in a private prefix. '
                 'Published native bytes, not public bucket or candidate registry acceptance.',
        'steps': [],
    }

    def run(label, command, check=True):
        with (output / (label + '.log')).open('w') as log:
            result = subprocess.run(command, env=env, stdout=log,
                                    stderr=subprocess.STDOUT, timeout=300)
        report['steps'].append({'name': label, 'command': command,
                                'exit_code': result.returncode})
        print(label, result.returncode, flush=True)
        if check and result.returncode:
            raise RuntimeError(f'{label} failed; see {label}.log')
        return result.returncode

    scoop = ['pwsh', '-NoLogo', '-NoProfile', '-NonInteractive',
             '-File', str(manager / 'bin/scoop.ps1')]
    path_command = ['pwsh', '-NoLogo', '-NoProfile', '-NonInteractive', '-Command',
                    "[ordered]@{user=[Environment]::GetEnvironmentVariable('Path','User'); "
                    "machine=[Environment]::GetEnvironmentVariable('Path','Machine')} | ConvertTo-Json"]
    restore_script = output / 'restore-path.ps1'
    restore_script.write_text(RESTORE_PATH)
    restore_command = ['pwsh', '-NoLogo', '-NoProfile', '-NonInteractive',
                       '-File', str(restore_script), str(output / 'path-before.log'),
                       str(prefix / 'shims')]
    installing = False
    path_restored = False
    try:
        run('recipe-check', ['python', str(cli / 'scripts/check-distribution-recipes.py')])
        run('path-before', path_command)
        run('manager-init', ['git', 'init', str(manager)])
        run('manager-remote', ['git', '-C', str(manager), 'remote', 'add', 'origin', report['manager_repo']])
        run('manager-fetch', ['git', '-C', str(manager), 'fetch', '--depth', '1', 'origin', SCOOP_SOURCE])
        run('manager-checkout', ['git', '-C', str(manager), 'checkout', '--detach', SCOOP_SOURCE])
        actual = subprocess.check_output(['git', '-C', str(manager), 'rev-parse', 'HEAD'], text=True).strip()
        if actual != SCOOP_SOURCE:
            raise RuntimeError('Scoop source differs from its pinned commit')
        # Scoop supports a portable root config at <root>/config.json.
        (prefix / 'config.json').write_text(json.dumps({
            'root_path': str(prefix), 'global_path': str(output / 'global'),
            'cache_path': str(output / 'cache'), 'use_external_7zip': True,
            'aria2-enabled': False,
        }))
        run('manager-version', [*scoop, '--version'])
        installing = True
        run('install', [*scoop, 'install', '--no-update-scoop', '--arch', '64bit',
                        str(cli / 'pkg/scoop/noya-cli.json')])
        archives = [path for path in (output / 'cache').rglob('*.zip')
                    if digest(path) == artifact['sha256']]
        if len(archives) != 1:
            raise RuntimeError('Scoop cache does not contain the expected release archive')
        archive = archives[0]
        report['archive_sha256'] = digest(archive)
        run('archive-provenance', ['gh', 'attestation', 'verify', str(archive),
                                  '--repo', 'sebastienrousseau/noya-cli',
                                  '--signer-workflow',
                                  'sebastienrousseau/noya-cli/.github/workflows/release.yml',
                                  '--source-ref', 'refs/tags/v' + record['version'],
                                  '--format', 'json'])
        spec = importlib.util.spec_from_file_location(
            'native_consumer', cli / 'scripts/check-distribution-consumer.py')
        consumer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(consumer)
        installed = prefix / 'apps/noya-cli' / record['version']
        report['installed_versions'] = consumer.exercise(installed, record['version'])
        report['shim_versions'] = consumer.exercise(prefix / 'shims', record['version'])
        extracted = output / 'verified-archive'
        consumer.install(archive, artifact, extracted)
        assets = ['noyafmt.exe', 'noyavalidate.exe', 'LICENSE-MIT', 'LICENSE-APACHE']
        report['installed_assets'] = {path: digest(installed / path) for path in assets}
        if any(digest(extracted / path) != report['installed_assets'][path] for path in assets):
            raise RuntimeError('Installed binaries or licenses differ from the verified release')
        run('uninstall', [*scoop, 'uninstall', 'noya-cli', '--purge'])
        installing = False
        if (prefix / 'apps/noya-cli').exists() or any(
                (prefix / 'shims').glob('noya*')):
            raise RuntimeError('Scoop uninstall left package files or command shims')
        run('manager-clean-source', ['git', '-C', str(manager), 'diff', '--exit-code'])
        run('restore-persistent-path', restore_command)
        path_restored = True
        run('path-after', path_command)
        before_path = json.loads((output / 'path-before.log').read_text())
        after_path = json.loads((output / 'path-after.log').read_text())
        if before_path != after_path:
            raise RuntimeError('Scoop consumer did not restore the user and machine PATH')
        report.update(status='passed', provenance_verified=True,
                      uninstall_passed=True, persistent_path_restored=True)
    finally:
        try:
            if installing:
                report['cleanup_uninstall_exit'] = run(
                    'cleanup-uninstall', [*scoop, 'uninstall', 'noya-cli', '--purge'], check=False)
        finally:
            try:
                if not path_restored and (output / 'path-before.log').exists():
                    report['path_cleanup_exit'] = run('cleanup-persistent-path', restore_command, check=False)
            finally:
                (output / 'receipt.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
