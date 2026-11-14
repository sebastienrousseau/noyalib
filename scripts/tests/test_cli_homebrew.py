#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Check refusal and partial-install cleanup without modifying a real prefix."""

import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

SCRIPT = Path(__file__).resolve().parents[1] / 'check-cli-homebrew.py'
SPEC = importlib.util.spec_from_file_location('homebrew_consumer', SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class HomebrewChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.cli = self.root / 'cli'
        (self.cli / 'pkg/brew').mkdir(parents=True)
        (self.cli / 'pkg/release-artifacts.json').write_text(json.dumps({
            'version': 'fixture', 'source': 'a' * 40,
        }))
        (self.cli / 'pkg/brew/noya-cli.rb').write_text('# fixture\n')
        self.prefix = self.root / 'prefix'
        self.cellar = self.prefix / 'Cellar'
        self.rack = self.cellar / 'noya-cli'
        self.tap = self.prefix / 'Library/Taps/noyalib/homebrew-assurance'
        self.output = self.root / 'output'
        self.commands = []

    def invoke(self, manager):
        original = subprocess.check_output

        def source(command, **kwargs):
            return 'a' * 40 if command[0] == 'git' else original(command, **kwargs)

        with mock.patch.object(MODULE.subprocess, 'check_output', side_effect=source), \
             mock.patch.object(MODULE.subprocess, 'run', side_effect=manager), \
             mock.patch.object(MODULE.platform, 'platform', return_value='fixture-platform'), \
             mock.patch('sys.argv', [str(SCRIPT), '--cli', str(self.cli),
                                     '--output', str(self.output)]):
            MODULE.main()

    def manager(self, command, *, stdout, **_):
        self.commands.append(command)
        values = {'--prefix': self.prefix, '--repository': self.prefix,
                  '--cellar': self.cellar, '--version': 'Homebrew 7.0.0'}
        operation = command[1]
        if operation in values:
            stdout.write(str(values[operation]) + '\n')
        elif operation == 'tap-new':
            (self.tap / 'Formula').mkdir(parents=True)
        elif operation == 'install':
            self.rack.mkdir(parents=True)
            (self.rack / 'partial').write_bytes(b'partial installation')
            return subprocess.CompletedProcess(command, 1)
        elif operation == 'uninstall':
            shutil.rmtree(self.rack)
        elif operation == 'untap':
            shutil.rmtree(self.tap)
        elif operation not in ['trust', 'untrust', 'info']:
            raise AssertionError(f'Unexpected package-manager operation: {command}')
        return subprocess.CompletedProcess(command, 0)

    def assert_existing_refused(self):
        with self.assertRaisesRegex(RuntimeError, 'Existing tap, CLI installation or command'):
            self.invoke(self.manager)
        self.assertTrue(all(command[1] in ['--prefix', '--repository', '--cellar']
                            for command in self.commands))
        receipt = json.loads((self.output / 'receipt.json').read_text())
        self.assertEqual(receipt['status'], 'failed')

    def test_existing_package_is_never_uninstalled(self):
        self.rack.mkdir(parents=True)
        marker = self.rack / 'existing'
        marker.write_bytes(b'existing installation')
        self.assert_existing_refused()
        self.assertEqual(marker.read_bytes(), b'existing installation')

    def test_existing_tap_is_never_removed(self):
        self.tap.mkdir(parents=True)
        marker = self.tap / 'existing'
        marker.write_bytes(b'existing formula')
        self.assert_existing_refused()
        self.assertEqual(marker.read_bytes(), b'existing formula')

    def test_dangling_command_link_is_never_replaced(self):
        binary = self.prefix / 'bin/noyafmt'
        binary.parent.mkdir(parents=True)
        binary.symlink_to('missing-target')
        self.assert_existing_refused()
        self.assertEqual(os.readlink(binary), 'missing-target')

    def test_partial_install_failure_cleans_only_test_state(self):
        other = self.cellar / 'other-package/keep'
        other.parent.mkdir(parents=True)
        other.write_bytes(b'unrelated package')
        with self.assertRaisesRegex(RuntimeError, 'install failed'):
            self.invoke(self.manager)
        self.assertFalse(self.rack.exists())
        self.assertFalse(self.tap.exists())
        self.assertEqual(other.read_bytes(), b'unrelated package')
        receipt = json.loads((self.output / 'receipt.json').read_text())
        self.assertEqual(receipt['status'], 'failed')
        self.assertEqual(receipt['cleanup_uninstall_exit'], 0)
        self.assertTrue(receipt['temporary_tap_removed'])


if __name__ == '__main__':
    unittest.main()
