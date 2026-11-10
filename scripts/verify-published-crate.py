#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Verify a public crate, its signed tag and its release provenance."""

import argparse
from datetime import datetime, timezone
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile
from tempfile import TemporaryDirectory
import tomllib
from urllib.request import Request, urlopen


FAMILY = ('noyalib', 'noya-cli', 'noyalib-lsp', 'noyalib-mcp', 'noyalib-wasm', 'noyalib-serde-yaml')
SIGNER = '4B7F16C909C7A8EE9BED338A4F047EDF5F90F638'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(*command):
    result = subprocess.run(command, capture_output=True, check=True, timeout=120)
    return result.stdout


def api(endpoint):
    return json.loads(run('gh', 'api', '--method', 'GET', endpoint))


def fetch(url, maximum):
    request = Request(url, headers={'User-Agent': 'noyalib-release-verification'})
    with urlopen(request, timeout=45) as response:
        data = response.read(maximum + 1)
    require(len(data) <= maximum, f'oversized response: {url}')
    return data


def verify(name, version, source, tag_object, keys, output):
    repo = f'sebastienrousseau/{name}'
    tag = f'v{version}'
    ref = f'refs/tags/{tag}'
    prefix = f'repos/{repo}'
    live_ref = api(f'{prefix}/git/ref/tags/{tag}')
    require(live_ref['object']['sha'] == tag_object and live_ref['object']['type'] == 'tag',
            'the public ref is not the expected annotated tag')
    signed = api(f'{prefix}/git/tags/{tag_object}')
    require(signed['tag'] == tag and signed['object']['type'] == 'commit'
            and signed['object']['sha'] == source and signed['verification']['verified'],
            'tag signature or source commit did not match')
    release = api(f'{prefix}/releases/tags/{tag}')
    require(release['tag_name'] == tag and not release['draft'] and not release['prerelease'],
            'release is absent, draft or prerelease')
    crate = f'{name}-{version}.crate'
    assets = {asset['name']: asset for asset in release['assets']}
    expected = {crate, crate + '.asc', crate + '.bundle'}
    require(expected <= assets.keys(), 'crate or detached verification material is absent')
    output.mkdir(parents=True, exist_ok=False)
    run('gh', 'release', 'download', tag, '--repo', repo, '--dir', str(output),
        '--pattern', crate, '--pattern', crate + '.asc', '--pattern', crate + '.bundle')
    blob = output / crate
    archive = blob.read_bytes()
    sha = hashlib.sha256(archive).hexdigest()
    require(len(archive) == assets[crate]['size'], 'release asset size mismatch')
    require(assets[crate].get('digest') in (None, f'sha256:{sha}'), 'release asset digest mismatch')
    registry = json.loads(fetch(f'https://crates.io/api/v1/crates/{name}/{version}', 1024 * 1024))['version']
    require(registry['crate'] == name and registry['num'] == version and not registry['yanked']
            and registry['checksum'] == sha, 'registry identity or checksum mismatch')
    require(fetch(f'https://static.crates.io/crates/{name}/{crate}', 64 * 1024 * 1024) == archive,
            'registry archive differs from the release asset')
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as packed:
        def read(member):
            path = packed.getmember(f'{name}-{version}/{member}')
            require(path.isfile() and path.size <= 1024 * 1024, f'unsafe metadata member: {member}')
            with packed.extractfile(path) as stream:
                return stream.read()

        vcs = json.loads(read('.cargo_vcs_info.json'))['git']
        require(vcs['sha1'] == source and not vcs.get('dirty', False), 'crate source metadata mismatch')
        manifest = tomllib.loads(read('Cargo.toml').decode())
        require(manifest['package']['name'] == name and manifest['package']['version'] == version,
                'crate manifest version mismatch')
        require(manifest['package']['rust-version'] == '1.89.0', 'unexpected Rust minimum')
        require('patch' not in manifest, 'published manifest contains a dependency patch')
        if name != 'noyalib':
            core = manifest['dependencies']['noyalib']
            require(core.get('version') == '=' + version and not {'git', 'path'} & core.keys(),
                    'satellite does not pin the matching registry core')

    workflow = f'{repo}/.github/workflows/release.yml'
    identity = f'https://github.com/{workflow}@{ref}'
    with TemporaryDirectory(prefix='noyalib-gpg-') as keyring:
        run('gpg', '--homedir', keyring, '--batch', '--import', str(keys))
        signature = run('gpg', '--homedir', keyring, '--batch', '--status-fd=1',
                        '--verify', str(output / (crate + '.asc')), str(blob)).decode()
        require(f'[GNUPG:] VALIDSIG {SIGNER} ' in signature, 'unexpected detached-signature fingerprint')
    run('cosign', 'verify-blob', '--certificate-identity', identity, '--certificate-oidc-issuer',
        'https://token.actions.githubusercontent.com', '--bundle', str(output / (crate + '.bundle')), str(blob))
    provenance = json.loads(run('gh', 'attestation', 'verify', str(blob), '--repo', repo,
                                '--signer-workflow', workflow, '--source-ref', ref,
                                '--deny-self-hosted-runners', '--format', 'json'))
    matches = [row['verificationResult']['signature']['certificate'] for row in provenance
               if row['verificationResult']['signature']['certificate'].get('sourceRepositoryDigest') == source
               and row['verificationResult']['signature']['certificate'].get('subjectAlternativeName') == identity]
    require(matches, 'GitHub provenance did not bind the exact source and workflow')
    receipt = {
        'verified_utc': datetime.now(timezone.utc).isoformat(), 'repo': repo, 'version': version,
        'tag': tag, 'tag_object': tag_object, 'source': source, 'release': release['html_url'],
        'registry_sha256': sha, 'registry_matches_github_asset': True,
        'gpg_fingerprint': SIGNER, 'sigstore_identity': identity,
        'workflow_invocation': matches[0]['runInvocationURI'],
    }
    (output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(f'{repo} {tag}: verified registry, signed release asset and source provenance')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repo', choices=FAMILY)
    parser.add_argument('version')
    parser.add_argument('--source', required=True)
    parser.add_argument('--tag-object', required=True)
    parser.add_argument('--keys', type=Path, default=Path(__file__).resolve().parents[1] / 'KEYS.asc')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'0\.0\.\d+', args.version) or not all(
            re.fullmatch(r'[a-f0-9]{40}', value) for value in (args.source, args.tag_object)):
        parser.error('provide a lockstep version and full source and tag-object SHAs')
    verify(args.repo, args.version, args.source, args.tag_object, args.keys.resolve(), args.output.resolve())


if __name__ == '__main__':
    main()
