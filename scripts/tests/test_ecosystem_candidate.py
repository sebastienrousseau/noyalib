#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Exercise candidate source identity, version and override refusal gates."""

import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'prepare-ecosystem-candidate.py'
SPEC = importlib.util.spec_from_file_location('candidate', SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CandidateChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.core = self.root / 'core'
        self.product = self.root / 'product'
        for repo in (self.core, self.product):
            repo.mkdir()
            subprocess.run(['git', 'init', '-q', str(repo)], check=True)
        manifest = self.core / 'crates/noyalib/Cargo.toml'
        manifest.parent.mkdir(parents=True)
        manifest.write_text('[package]\nname = "noyalib"\nversion = "0.0.61"\n')
        (self.product / 'Cargo.toml').write_text(
            '[package]\nname = "noya-cli"\nversion = "0.0.61"\n'
            '[dependencies]\nnoyalib = "=0.0.61"\n')
        for repo in (self.core, self.product):
            (repo / 'Cargo.lock').write_text('version = 4\n')
            subprocess.run(['git', '-C', str(repo), 'add', 'Cargo.toml' if repo == self.product
                            else 'crates/noyalib/Cargo.toml', 'Cargo.lock'], check=True)
            subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture',
                            '-c', 'user.email=fixture@example.invalid',
                            '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null',
                            'commit', '-qm', 'test: fixture'], check=True)
        self.commit = subprocess.check_output(
            ['git', '-C', str(self.product), 'rev-parse', 'HEAD'], text=True).strip()

    def prepare(self, commit=None):
        return MODULE.prepare(self.core, self.product, 'noya-cli', commit or self.commit)

    def test_local_override_keeps_registry_pin_and_lock(self):
        before = (self.product / 'Cargo.lock').read_bytes()
        receipt = self.prepare()
        self.assertFalse(receipt['registry_resolution_proof'])
        self.assertEqual((self.product / 'Cargo.lock').read_bytes(), before)
        self.assertIn('noyalib = "=0.0.61"', (self.product / 'Cargo.toml').read_text())
        MODULE.verify(receipt)

    def test_existing_configuration_is_never_overwritten(self):
        path = self.product / '.cargo/config.toml'
        path.parent.mkdir()
        path.write_text('# existing configuration\n')
        with self.assertRaisesRegex(ValueError, 'existing Cargo configuration'):
            self.prepare()
        self.assertEqual(path.read_text(), '# existing configuration\n')

    def test_wrong_commit_is_refused_before_writing(self):
        with self.assertRaisesRegex(ValueError, 'commit'):
            self.prepare('0' * 40)
        self.assertFalse((self.product / '.cargo').exists())

    def test_version_or_pin_drift_is_refused(self):
        path = self.product / 'Cargo.toml'
        path.write_text(path.read_text().replace('=0.0.61', '=0.0.60'))
        with self.assertRaisesRegex(ValueError, 'exact core pin'):
            self.prepare()
        self.assertFalse((self.product / '.cargo').exists())

    def test_changed_lock_is_refused_after_build(self):
        receipt = self.prepare()
        (self.product / 'Cargo.lock').write_text('changed by build\n')
        with self.assertRaisesRegex(ValueError, 'lockfile changed'):
            MODULE.verify(receipt)

    def test_manifest_rejects_duplicate_or_unpinned_products(self):
        products = [{'repo': name, 'commit': 'a' * 40} for name in MODULE.PRODUCTS]
        path = self.root / 'candidate.json'
        path.write_text(json.dumps({'products': products}))
        self.assertEqual(len(MODULE.products(path)), 5)
        for invalid in ([products[0]] * 5,
                        [*products[:-1], {'repo': '../outside', 'commit': 'a' * 40}],
                        [*products[:-1], {'repo': products[-1]['repo'], 'commit': 'main'}]):
            path.write_text(json.dumps({'products': invalid}))
            with self.assertRaises(ValueError):
                MODULE.products(path)


if __name__ == '__main__':
    unittest.main()
