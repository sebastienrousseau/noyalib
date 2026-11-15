#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Pin completeness and checkout identity for distributed family campaigns."""

import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'family-fuzz-matrix.py'
SPEC = importlib.util.spec_from_file_location('family_matrix', SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class FamilyMatrixTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.family = Path(self.temp.name)
        self.refs = []
        for name in ('noyalib', *MODULE.CANDIDATE.PRODUCTS):
            repo = self.family / name
            repo.mkdir()
            subprocess.run(['git', 'init', '-q', str(repo)], check=True)
            manifest = repo / 'fuzz/Cargo.toml'
            manifest.parent.mkdir()
            manifest.write_text('[[bin]]\nname = "exercise"\n')
            if name == 'noyalib':
                portable = repo / 'fuzz-nostd/Cargo.toml'
                portable.parent.mkdir()
                portable.write_text('[[bin]]\nname = "portable"\n')
            if name == 'noyalib-mcp':
                with manifest.open('a') as out:
                    out.write('[[bin]]\nname = "session_lifecycle"\n')
            self.commit(repo)
            if name != 'noyalib':
                self.refs.append({'repo': name, 'commit': MODULE.CANDIDATE.head(repo)})
        self.manifest = self.family / 'noyalib/docs/ecosystem-candidate.json'
        self.manifest.parent.mkdir()
        self.save_refs()

    @staticmethod
    def commit(repo):
        subprocess.run(['git', '-C', str(repo), 'add', 'fuzz'], check=True)
        if (repo / 'fuzz-nostd').is_dir():
            subprocess.run(['git', '-C', str(repo), 'add', 'fuzz-nostd'], check=True)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture',
                        '-c', 'user.email=fixture@example.invalid',
                        '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null',
                        'commit', '-qm', 'test: inventory fixture'], check=True)

    def save_refs(self):
        self.manifest.write_text(json.dumps({'products': self.refs}))

    def test_discovers_every_product_and_portable_scope(self):
        rows = MODULE.matrix(self.family)['include']
        self.assertEqual(len(rows), 8)
        self.assertEqual({row['repo'] for row in rows},
                         {'noyalib', *MODULE.CANDIDATE.PRODUCTS})
        self.assertIn({'repo': 'noyalib-mcp', 'commit': self.refs[2]['commit'],
                       'scope': 'fuzz', 'target': 'session_lifecycle'}, rows)
        self.assertTrue(any(row['scope'] == 'fuzz-nostd' for row in rows))

    def test_wrong_checkout_is_refused(self):
        self.refs[0]['commit'] = '0' * 40
        self.save_refs()
        with self.assertRaisesRegex(ValueError, 'commit'):
            MODULE.matrix(self.family)

    def test_missing_product_ref_is_refused(self):
        self.refs.pop()
        self.save_refs()
        with self.assertRaises(ValueError):
            MODULE.matrix(self.family)

    def test_no_product_can_silently_have_zero_targets(self):
        repo = self.family / 'noyalib-mcp'
        (repo / 'fuzz/Cargo.toml').write_text('[package]\nname = "empty"\n')
        self.commit(repo)
        self.refs[2]['commit'] = MODULE.CANDIDATE.head(repo)
        self.save_refs()
        with self.assertRaisesRegex(ValueError, 'no fuzz targets'):
            MODULE.matrix(self.family)

    def test_unsafe_target_name_is_refused(self):
        repo = self.family / 'noyalib-mcp'
        (repo / 'fuzz/Cargo.toml').write_text('[[bin]]\nname = "../escape"\n')
        self.commit(repo)
        self.refs[2]['commit'] = MODULE.CANDIDATE.head(repo)
        self.save_refs()
        with self.assertRaisesRegex(ValueError, 'invalid fuzz target'):
            MODULE.matrix(self.family)

    def test_changed_tracked_manifest_is_refused(self):
        repo = self.family / 'noyalib-mcp'
        (repo / 'fuzz/Cargo.toml').write_text('[[bin]]\nname = "uncommitted"\n')
        with self.assertRaisesRegex(ValueError, 'changed'):
            MODULE.matrix(self.family)


if __name__ == '__main__':
    unittest.main()
