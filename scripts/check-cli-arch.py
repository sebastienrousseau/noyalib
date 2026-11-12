#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Build, install and remove the CLI PKGBUILD in a verified offline container."""

import argparse
import datetime
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import urllib.request
import uuid

IMAGE = ('ghcr.io/archlinux/archlinux@sha256:'
         '19bc4b6c53d9089ea6aa506ecabc1ddc5aee1024a2ceb9a195b6c8a04fdf2418')
IDENTITY = (r'https://gitlab\.archlinux\.org/archlinux/archlinux-docker/'
            r'/\.gitlab-ci\.yml@refs/tags/v[0-9]+\.0\.[0-9]+')
WORKLOAD = r'''#!/bin/bash
set -euo pipefail
expected_version="$1"
useradd --create-home --uid 1000 builder
chown -R builder:builder /assurance/work
cd /assurance/work
runuser -u builder -- makepkg --noconfirm
packages=(noyalib-bin-*.pkg.tar.*)
test "${#packages[@]}" -eq 1
sha256sum "${packages[0]}" > /assurance/package.sha256
pacman --noconfirm -U "${packages[0]}"
pacman -Qkk noyalib-bin
test "$(noyafmt --version)" = "noyafmt $expected_version"
test "$(noyavalidate --version)" = "noyavalidate $expected_version"
printf 'a:   1\n' | noyafmt --stdin > /assurance/formatted.yaml
printf 'a: 1\n' > /assurance/expected.yaml
cmp /assurance/formatted.yaml /assurance/expected.yaml
set +e
printf 'a: [\n' | noyavalidate > /assurance/invalid.stdout 2> /assurance/invalid.stderr
invalid_status=$?
set -e
test "$invalid_status" -eq 1
test -s /assurance/invalid.stderr
assets=(/usr/bin/noyafmt /usr/bin/noyavalidate
  /usr/share/man/man1/noyafmt.1.gz /usr/share/man/man1/noyavalidate.1.gz
  /usr/share/bash-completion/completions/noyafmt
  /usr/share/bash-completion/completions/noyavalidate
  /usr/share/zsh/site-functions/_noyafmt /usr/share/zsh/site-functions/_noyavalidate
  /usr/share/fish/vendor_completions.d/noyafmt.fish
  /usr/share/fish/vendor_completions.d/noyavalidate.fish
  /usr/share/licenses/noyalib-bin/LICENSE-MIT
  /usr/share/licenses/noyalib-bin/LICENSE-APACHE)
sha256sum "${assets[@]}" > /assurance/installed.sha256
pacman --noconfirm -R noyalib-bin
for asset in "${assets[@]}"; do
  test ! -e "$asset"
  test ! -L "$asset"
done
if pacman -Q noyalib-bin; then exit 1; fi
printf 'passed\n' > /assurance/package-manager-passed
'''


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    cli, output = args.cli.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    work = output / 'work'
    work.mkdir()
    record = json.loads((cli / 'pkg/release-artifacts.json').read_text())
    artifact = record['artifacts']['x86_64-unknown-linux-musl']
    recipe = cli / 'pkg/aur/PKGBUILD'
    report = {
        'status': 'failed',
        'recorded_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'cli_source': subprocess.check_output(
            ['git', '-C', str(cli), 'rev-parse', 'HEAD'], text=True).strip(),
        'consumer_sha256': digest(Path(__file__)),
        'recipe_sha256': digest(recipe),
        'release_version': record['version'], 'release_source': record['source'],
        'image': IMAGE, 'target': 'x86_64-unknown-linux-musl',
        'scope': 'Actual candidate PKGBUILD and makepkg/pacman in an isolated container. '
                 'Published native bytes, not public AUR or candidate registry acceptance.',
        'steps': [],
    }

    def run(label, command, timeout=300, check=True):
        with (output / (label + '.log')).open('w') as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT,
                                    timeout=timeout)
        report['steps'].append({'name': label, 'command': command,
                                'exit_code': result.returncode})
        print(label, result.returncode, flush=True)
        if check and result.returncode:
            raise RuntimeError(f'{label} failed; see {label}.log')
        return result.returncode

    container = 'noyalib-arch-assurance-' + uuid.uuid4().hex
    started = False
    try:
        run('recipe-check', ['python3', str(cli / 'scripts/check-distribution-recipes.py')])
        shutil.copy2(recipe, work / 'PKGBUILD')
        archive = work / artifact['url'].rsplit('/', 1)[1]
        with urllib.request.urlopen(artifact['url'], timeout=60) as response, archive.open('xb') as stream:
            shutil.copyfileobj(response, stream)
        if digest(archive) != artifact['sha256']:
            raise RuntimeError('Downloaded native archive does not match its release digest')
        report['archive_sha256'] = digest(archive)
        run('archive-provenance', ['gh', 'attestation', 'verify', str(archive),
                                  '--repo', 'sebastienrousseau/noya-cli',
                                  '--signer-workflow',
                                  'sebastienrousseau/noya-cli/.github/workflows/release.yml',
                                  '--source-ref', 'refs/tags/v' + record['version'],
                                  '--format', 'json'])
        run('image-signature', ['cosign', 'verify', IMAGE,
                               '--certificate-identity-regexp', IDENTITY,
                               '--certificate-oidc-issuer', 'https://gitlab.archlinux.org',
                               '--output', 'json'])
        run('image-pull', ['docker', 'pull', '--platform', 'linux/amd64', IMAGE])
        (output / 'consumer.sh').write_text(WORKLOAD)
        started = True
        run('package-manager', ['docker', 'run', '--rm', '--name', container,
                               '--network', 'none', '--platform', 'linux/amd64',
                               '--mount', f'type=bind,source={output},target=/assurance',
                               IMAGE, 'bash', '/assurance/consumer.sh', record['version']],
            timeout=900)
        started = False  # --rm removed the completed container.
        if (output / 'package-manager-passed').read_text() != 'passed\n':
            raise RuntimeError('Missing successful package-manager result')
        installed = {}
        for line in (output / 'installed.sha256').read_text().splitlines():
            sha, path = line.split(None, 1)
            installed[path] = sha
        spec = importlib.util.spec_from_file_location(
            'native_consumer', cli / 'scripts/check-distribution-consumer.py')
        consumer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(consumer)
        extracted = output / 'verified-archive'
        consumer.install(archive, artifact, extracted)
        for tool in ['noyafmt', 'noyavalidate']:
            if digest(extracted / tool) != installed['/usr/bin/' + tool]:
                raise RuntimeError(f'Installed {tool} differs from the verified release')
        if digest(work / 'PKGBUILD') != report['recipe_sha256']:
            raise RuntimeError('Package build changed the candidate recipe')
        report.update(status='passed', installed_assets=installed,
                      package_sha256=(output / 'package.sha256').read_text().split()[0],
                      provenance_verified=True, image_signature_verified=True,
                      package_build_network='none', uninstall_passed=True)
    finally:
        if started:
            report['container_cleanup_exit'] = run(
                'container-cleanup', ['docker', 'rm', '--force', container], check=False)
        (output / 'receipt.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
