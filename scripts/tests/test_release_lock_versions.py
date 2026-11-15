#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Reject stale family versions in tracked standalone Cargo locks."""

from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / 'verify-release-versions.sh'
VERSION = '0.0.61'


class ReleaseLockVersions(unittest.TestCase):
    def check_tree(self, nested, *, name='noyalib', tracked=True, workspace=False):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / 'scripts').mkdir()
            shutil.copyfile(SCRIPT, root / 'scripts/verify-release-versions.sh')
            package = f'[package]\nname = "{name}"\nversion = "{VERSION}"\n'
            if workspace:
                member = root / 'crates/noyalib'
                member.mkdir(parents=True)
                (member / 'Cargo.toml').write_text(package)
                (root / 'Cargo.toml').write_text('[workspace]\nmembers = ["crates/noyalib"]\n')
            else:
                (root / 'Cargo.toml').write_text(package)
            (root / 'Cargo.lock').write_text(
                f'version = 4\n[[package]]\nname = "{name}"\nversion = "{VERSION}"\n')
            (root / 'fuzz').mkdir()
            (root / 'fuzz/Cargo.lock').write_text('version = 4\n' + ''.join(
                f'[[package]]\nname = "{pkg}"\nversion = "{version}"\n'
                for pkg, version in nested))
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            subprocess.run(['git', 'add', 'Cargo.toml', 'Cargo.lock'], cwd=root, check=True)
            if tracked:
                subprocess.run(['git', 'add', 'fuzz/Cargo.lock'], cwd=root, check=True)
            return subprocess.run(['bash', 'scripts/verify-release-versions.sh'], cwd=root,
                                  capture_output=True, text=True)

    def test_current_family_and_unrelated_dependencies_pass(self):
        result = self.check_tree([('noyalib', VERSION), ('serde', '1.0.228')])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_workspace_works_in_a_renamed_checkout(self):
        result = self.check_tree([('noyalib', VERSION)], workspace=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_stale_nested_core_fails(self):
        result = self.check_tree([('noyalib', '0.0.50')])
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn('fuzz/Cargo.lock', result.stdout)

    def test_satellite_nested_core_fails(self):
        result = self.check_tree([('noyalib-mcp', VERSION), ('noyalib', '0.0.60')],
                                 name='noyalib-mcp')
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn('fuzz/Cargo.lock', result.stdout)

    def test_every_family_entry_is_checked(self):
        result = self.check_tree([('noyalib', VERSION), ('noya-cli', '0.0.60')])
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_untracked_build_locks_are_ignored(self):
        result = self.check_tree([('noyalib', '0.0.50')], tracked=False)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
