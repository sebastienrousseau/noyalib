#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Exercise archive, image and container refusal boundaries without a daemon."""

import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

SCRIPT = Path(__file__).resolve().parents[1] / 'check-cli-arch.py'
SPEC = importlib.util.spec_from_file_location('arch_consumer', SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ArchChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.cli = self.root / 'cli'
        (self.cli / 'pkg/aur').mkdir(parents=True)
        (self.cli / 'pkg/aur/PKGBUILD').write_text('# recipe fixture\n')
        self.bytes = b'archive fixture'
        self.record = {
            'version': 'fixture', 'source': 'a' * 40,
            'artifacts': {'x86_64-unknown-linux-musl': {
                'url': 'https://example.invalid/archive.tar.gz',
                'sha256': hashlib.sha256(self.bytes).hexdigest(),
            }},
        }
        self.output = self.root / 'output'
        self.commands = []
        self.failure = None

    def manager(self, command, **_):
        self.commands.append(command)
        key = command[0] + (' ' + command[1] if len(command) > 1 else '')
        refused = key == self.failure or key == 'docker run'
        return subprocess.CompletedProcess(command, 1 if refused else 0)

    def invoke(self):
        (self.cli / 'pkg/release-artifacts.json').write_text(json.dumps(self.record))
        with mock.patch.object(MODULE.subprocess, 'check_output', return_value='a' * 40), \
             mock.patch.object(MODULE.subprocess, 'run', side_effect=self.manager), \
             mock.patch.object(MODULE.urllib.request, 'urlopen',
                               return_value=io.BytesIO(self.bytes)) as download, \
             mock.patch('sys.argv', [str(SCRIPT), '--cli', str(self.cli),
                                     '--output', str(self.output)]):
            with self.assertRaises(RuntimeError) as error:
                MODULE.main()
        self.assertEqual(json.loads((self.output / 'receipt.json').read_text())['status'], 'failed')
        return str(error.exception), download.call_count

    def test_bad_archive_never_reaches_a_container(self):
        self.record['artifacts']['x86_64-unknown-linux-musl']['sha256'] = '0' * 64
        error, _ = self.invoke()
        self.assertIn('does not match its release digest', error)
        self.assertFalse(any(command[0] in ['cosign', 'docker'] for command in self.commands))

    def test_failed_image_signature_never_pulls_or_executes(self):
        self.failure = 'cosign verify'
        error, _ = self.invoke()
        self.assertIn('image-signature failed', error)
        self.assertFalse(any(command[0] == 'docker' for command in self.commands))

    def test_bad_recipe_never_downloads_or_installs(self):
        self.failure = 'python3 ' + str(self.cli / 'scripts/check-distribution-recipes.py')
        error, downloads = self.invoke()
        self.assertIn('recipe-check failed', error)
        self.assertEqual(downloads, 0)
        self.assertEqual(len(self.commands), 1)

    def test_failed_container_cleans_only_its_own_name(self):
        self.failure = 'docker run'
        error, _ = self.invoke()
        self.assertIn('package-manager failed', error)
        started = next(command for command in self.commands if command[:2] == ['docker', 'run'])
        name = started[started.index('--name') + 1]
        self.assertTrue(name.startswith('noyalib-arch-assurance-'))
        self.assertEqual(started[started.index('--network') + 1], 'none')
        self.assertEqual(self.commands[-1], ['docker', 'rm', '--force', name])


if __name__ == '__main__':
    unittest.main()
