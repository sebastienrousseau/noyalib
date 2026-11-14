#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Install the pinned CLI formula in a temporary tap and retain its receipt."""

import argparse
import datetime
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    cli = args.cli.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    record = json.loads((cli / 'pkg/release-artifacts.json').read_text())
    formula = cli / 'pkg/brew/noya-cli.rb'
    env = os.environ.copy()
    env.pop('HOMEBREW_NO_INSTALL_FROM_API', None)
    env.update(HOMEBREW_CACHE=str(output / 'cache'),
               HOMEBREW_LOGS=str(output / 'logs'),
               XDG_CONFIG_HOME=str(output / 'config'),
               HOMEBREW_NO_AUTO_UPDATE='1',
               HOMEBREW_NO_ANALYTICS='1',
               HOMEBREW_NO_INSTALL_CLEANUP='1',
               HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK='1',
               HOMEBREW_NO_ENV_HINTS='1')
    report = {
        'status': 'failed',
        'recorded_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'platform': platform.platform(),
        'architecture': platform.machine(),
        'cli_source': subprocess.check_output(
            ['git', '-C', str(cli), 'rev-parse', 'HEAD'], text=True).strip(),
        'consumer_sha256': digest(Path(__file__)),
        'formula_sha256': digest(formula),
        'release_version': record['version'],
        'release_source': record['source'],
        'scope': 'Actual candidate formula in a temporary local tap. '
                 'Published native bytes, not public tap or candidate registry acceptance.',
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

    def brew_value(label, *arguments):
        run(label, ['brew', *arguments])
        return (output / (label + '.log')).read_text().strip()

    created = False
    trusted = False
    tap = None
    rack = None
    name = 'noyalib/assurance/noya-cli'
    try:
        prefix = Path(brew_value('prefix', '--prefix'))
        repository = Path(brew_value('repository', '--repository'))
        cellar = Path(brew_value('cellar', '--cellar'))
        rack = cellar / 'noya-cli'
        tap = repository / 'Library/Taps/noyalib/homebrew-assurance'
        if tap.exists() or rack.exists() or any(
                os.path.lexists(prefix / 'bin' / tool)
                for tool in ['noyafmt', 'noyavalidate']):
            raise RuntimeError('Existing tap, CLI installation or command; refusing to alter it')
        report['prefix'] = str(prefix)
        report['brew_version'] = brew_value('brew-version', '--version')
        created = True
        run('tap-create', ['brew', 'tap-new', '--no-git', 'noyalib/assurance'])
        shutil.copy2(formula, tap / 'Formula/noya-cli.rb')
        # Homebrew 7 requires explicit trust; older releases have no trust command.
        version = report['brew_version'].split()[1].split('.')[0]
        if int(version) >= 7:
            run('trust-reviewed-formula', ['brew', 'trust', '--formula', name])
            trusted = True
        run('formula-info', ['brew', 'info', '--json=v2', name])
        run('install', ['brew', 'install', name])
        run('formula-test', ['brew', 'test', name])
        spec = importlib.util.spec_from_file_location(
            'native_consumer', cli / 'scripts/check-distribution-consumer.py')
        consumer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(consumer)
        report['installed_versions'] = consumer.exercise(prefix / 'bin', record['version'])
        assets = [
            'bin/noyafmt', 'bin/noyavalidate',
            'share/man/man1/noyafmt.1', 'share/man/man1/noyavalidate.1',
            'etc/bash_completion.d/noyafmt', 'etc/bash_completion.d/noyavalidate',
            'share/zsh/site-functions/_noyafmt', 'share/zsh/site-functions/_noyavalidate',
            'share/fish/vendor_completions.d/noyafmt.fish',
            'share/fish/vendor_completions.d/noyavalidate.fish',
        ]
        report['installed_assets'] = {path: digest(prefix / path) for path in assets}
        artifact_arch = {'arm64': 'aarch64', 'aarch64': 'aarch64',
                         'x86_64': 'x86_64', 'amd64': 'x86_64'}[platform.machine().lower()]
        artifact_os = {'Darwin': 'apple-darwin', 'Linux': 'unknown-linux-musl'}[platform.system()]
        artifact = record['artifacts'][artifact_arch + '-' + artifact_os]
        archive = Path(brew_value('archive-cache', '--cache', name))
        if digest(archive) != artifact['sha256']:
            raise RuntimeError('Homebrew cache does not match the recorded release archive')
        report['archive_sha256'] = digest(archive)
        extracted = output / 'verified-archive'
        consumer.install(archive, artifact, extracted)
        for tool in ['noyafmt', 'noyavalidate']:
            if digest(extracted / tool) != report['installed_assets']['bin/' + tool]:
                raise RuntimeError(f'Installed {tool} differs from the verified release')
        run('archive-provenance', ['gh', 'attestation', 'verify', str(archive),
                                  '--repo', 'sebastienrousseau/noya-cli',
                                  '--signer-workflow',
                                  'sebastienrousseau/noya-cli/.github/workflows/release.yml',
                                  '--source-ref', 'refs/tags/v' + record['version'],
                                  '--format', 'json'])
        report['provenance_verified'] = True
        run('uninstall', ['brew', 'uninstall', name])
        if rack.exists() or any(os.path.lexists(prefix / path) for path in assets):
            raise RuntimeError('Homebrew uninstall left package files or command links')
        report['uninstall_passed'] = True
        report['status'] = 'passed'
    finally:
        # The precondition proved this package absent before this test created its tap.
        if created and rack.exists():
            report['cleanup_uninstall_exit'] = run(
                'cleanup-uninstall', ['brew', 'uninstall', name], check=False)
        if trusted:
            if run('untrust-reviewed-formula', ['brew', 'untrust', '--formula', name], check=False):
                report['status'] = 'failed'
        if created:
            run('tap-remove', ['brew', 'untap', 'noyalib/assurance'], check=False)
            report['temporary_tap_removed'] = not tap.exists()
            if not report['temporary_tap_removed']:
                report['status'] = 'failed'
        (output / 'receipt.json').write_text(json.dumps(report, indent=2) + '\n')
    if report['status'] != 'passed':
        raise RuntimeError('Homebrew acceptance or cleanup failed')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
