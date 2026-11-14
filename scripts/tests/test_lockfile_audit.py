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
    def run_audit(self, *, finding=False, broken=False, extra=False, mismatched=False, missing_db=False):
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
            db = root / 'db'
            if not missing_db:
                subprocess.run(['git', 'init', '-q', str(db)], check=True)
                (db / 'fixture.md').write_text('Committed advisory database fixture')
                subprocess.run(['git', 'add', 'fixture.md'], cwd=db, check=True)
                subprocess.run(['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                                '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null',
                                'commit', '-qm', 'Advisory fixture'], cwd=db, check=True)
                (db / 'uncommitted.md').write_text('Must not be scanned')
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
db = Path(sys.argv[sys.argv.index('--db') + 1])
assert (db / 'fixture.md').is_file()
assert not (db / 'uncommitted.md').exists()
lock = sys.argv[sys.argv.index('--file') + 1]
found = os.environ['FINDING'] == '1' and lock == 'fuzz/Cargo.lock'
print(json.dumps({'database': {'last-commit': 'a' * 40 if os.environ['MISMATCHED'] == '1' else None},
    'vulnerabilities': {'found': False, 'count': 0, 'list': []},
    'warnings': {'unsound': [{'advisory': {'id': 'RUSTSEC-2026-0386'},
        'package': {'name': 'tokio', 'version': '1.53.1'}}]} if found else {}}))
sys.exit(1 if found and '--deny' in sys.argv and 'unsound' in sys.argv else 0)
''')
            cargo.chmod(0o755)
            env = {**os.environ, 'PATH': str(tools) + os.pathsep + os.environ['PATH'],
                   'FINDING': str(int(finding)), 'BROKEN': str(int(broken)), 'MISMATCHED': str(int(mismatched)),
                   'CALLS': str(root / 'calls.jsonl')}
            result = subprocess.run(['python3', str(SCRIPT), '--root', str(root),
                                     '--output-dir', str(root / 'reports'), '--no-fetch', '--db', str(db)],
                                    env=env, capture_output=True, text=True)
            self.assertTrue((root / 'reports/summary.json').exists(), result.stderr)
            summary = json.loads((root / 'reports/summary.json').read_text())
            calls_file = root / 'calls.jsonl'
            calls = [json.loads(line) for line in calls_file.read_text().splitlines()] if calls_file.exists() else []
            return result, summary, calls

    def test_discovers_nested_tracked_locks_and_preserves_scope(self):
        run, summary, calls = self.run_audit(extra=True)
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual([x['lockfile'] for x in summary['lockfiles']],
                         ['Cargo.lock', 'fuzz/Cargo.lock', 'tests/regression/Cargo.lock'])
        self.assertEqual(len(calls), 3)
        self.assertTrue(all(x['advisory_db_commit'] == summary['advisory_db_commit'] for x in summary['lockfiles']))
        self.assertTrue(all('--deny' in call and 'unsound' in call for call in calls))
        self.assertTrue(all('--no-fetch' in call for call in calls))
        self.assertEqual(len(summary['advisory_db_commit']), 40)

    def test_unsound_fuzz_dependency_fails_but_other_locks_are_still_audited(self):
        run, summary, calls = self.run_audit(finding=True)
        self.assertEqual(run.returncode, 1, run.stderr)
        self.assertEqual(len(calls), 3)
        self.assertEqual(summary['lockfiles'][1]['status'], 'failed')
        self.assertIn('RUSTSEC-2026-0386', json.dumps(summary))

    def test_audit_reporting_a_different_database_is_unavailable(self):
        run, summary, calls = self.run_audit(mismatched=True)
        self.assertEqual(run.returncode, 2, run.stderr)
        self.assertEqual(len(calls), 3)
        self.assertTrue(all(x['status'] == 'unavailable' for x in summary['lockfiles']))

    def test_missing_database_snapshot_never_runs_an_audit(self):
        run, summary, calls = self.run_audit(missing_db=True)
        self.assertEqual(run.returncode, 2, run.stderr)
        self.assertFalse(calls)
        self.assertTrue(all(x['status'] == 'unavailable' for x in summary['lockfiles']))

    def test_missing_audit_json_is_an_error_never_a_clean_audit(self):
        run, summary, calls = self.run_audit(broken=True)
        self.assertEqual(run.returncode, 2, run.stderr)
        self.assertEqual(len(calls), 3)
        self.assertTrue(all(x['status'] == 'unavailable' for x in summary['lockfiles']))


if __name__ == '__main__':
    unittest.main()
