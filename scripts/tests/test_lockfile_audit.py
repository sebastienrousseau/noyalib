#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Exercise lockfile discovery, unsoundness enforcement and failure reporting."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / 'audit-lockfiles.py'


class LockfileAudit(unittest.TestCase):
    def run_audit(self, *, finding=False, broken=False, extra=False):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            for sub in ['', 'fuzz', 'tests/regression']:
                d = root / sub
                d.mkdir(parents=True, exist_ok=True)
                (d / 'Cargo.toml').write_text('[workspace]\n')
                (d / 'Cargo.lock').write_text('version = 4\n')
            if extra:
                (root / 'untracked').mkdir()
                (root / 'untracked/Cargo.lock').write_text('version = 4\n')
            subprocess.run(['git', 'add', 'Cargo.toml', 'Cargo.lock', 'fuzz', 'tests'],
                           cwd=root, check=True)
            tools = root / 'tools'
            tools.mkdir()
            cargo = tools / 'cargo'
            cargo.write_text('''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
with open(os.environ['CALLS'], 'a') as f:
    f.write(json.dumps(sys.argv[1:]) + '\\n')
if os.environ['BROKEN'] == '1':
    print('database unavailable', file=sys.stderr)
    sys.exit(2)
lock = sys.argv[sys.argv.index('--file') + 1]
found = os.environ['FINDING'] == '1' and lock == 'fuzz/Cargo.lock'
print(json.dumps({'database': {'last-commit': 'a' * 40},
    'vulnerabilities': {'found': False, 'count': 0, 'list': []},
    'warnings': {'unsound': [{'advisory': {'id': 'RUSTSEC-2026-0386'},
        'package': {'name': 'tokio', 'version': '1.53.1'}}]} if found else {}}))
sys.exit(1 if found and '--deny' in sys.argv and 'unsound' in sys.argv else 0)
''')
            cargo.chmod(0o755)
            env = {**os.environ, 'PATH': str(tools) + os.pathsep + os.environ['PATH'],
                   'FINDING': str(int(finding)), 'BROKEN': str(int(broken)),
                   'CALLS': str(root / 'calls.jsonl')}
            result = subprocess.run(['python3', str(SCRIPT), '--root', str(root),
                                     '--output-dir', str(root / 'reports')],
                                    env=env, capture_output=True, text=True)
            self.assertTrue((root / 'reports/summary.json').exists(), result.stderr)
            summary = json.loads((root / 'reports/summary.json').read_text())
            calls = [json.loads(line) for line in (root / 'calls.jsonl').read_text().splitlines()]
            return result, summary, calls

    def test_discovers_nested_tracked_locks_and_preserves_scope(self):
        run, summary, calls = self.run_audit(extra=True)
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual([x['lockfile'] for x in summary['lockfiles']],
                         ['Cargo.lock', 'fuzz/Cargo.lock', 'tests/regression/Cargo.lock'])
        self.assertEqual(len(calls), 3)
        self.assertTrue(all('--deny' in call and 'unsound' in call for call in calls))
        self.assertNotIn('--no-fetch', calls[0])
        self.assertTrue(all('--no-fetch' in call for call in calls[1:]))

    def test_unsound_fuzz_dependency_fails_but_other_locks_are_still_audited(self):
        run, summary, calls = self.run_audit(finding=True)
        self.assertEqual(run.returncode, 1, run.stderr)
        self.assertEqual(len(calls), 3)
        self.assertEqual(summary['lockfiles'][1]['status'], 'failed')
        self.assertIn('RUSTSEC-2026-0386', json.dumps(summary))

    def test_missing_database_is_an_error_never_a_clean_audit(self):
        run, summary, calls = self.run_audit(broken=True)
        self.assertEqual(run.returncode, 2, run.stderr)
        self.assertEqual(len(calls), 3)
        self.assertTrue(all(x['status'] == 'unavailable' for x in summary['lockfiles']))


if __name__ == '__main__':
    unittest.main()
