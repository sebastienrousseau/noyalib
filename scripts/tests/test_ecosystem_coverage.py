#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Exercise the scorecard coverage probe through its real shell entry point."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
HARNESS = Path(os.environ.get('SCORECARD_HARNESS', ROOT / 'scripts/ecosystem-scorecard.sh'))


class EcosystemCoverage(unittest.TestCase):
    def probe(self, *, count=100, covered=94, percent=94.0, result=0, tool=True,
              malformed=False):
        with tempfile.TemporaryDirectory() as scratch:
            work = Path(scratch)
            payload = {'type': 'llvm.coverage.json.export', 'data': [{'totals': {
                'lines': {'count': count, 'covered': covered, 'percent': percent},
                'regions': {'count': 100, 'covered': 83, 'percent': 83.0},
                'functions': {'count': 100, 'covered': 91, 'percent': 91.0},
                'branches': {'count': 100, 'covered': 23, 'percent': 23.0},
            }}]}
            fixture = work / 'fixture.json'
            fixture.write_text('not coverage' if malformed else json.dumps(payload))
            cargo = work / 'cargo'
            cargo.write_text('''#!/usr/bin/env python3
import os, pathlib, sys
if '--json' in sys.argv:
    data = pathlib.Path(os.environ['COVERAGE_FIXTURE']).read_text()
    if '--output-path' in sys.argv:
        pathlib.Path(sys.argv[sys.argv.index('--output-path') + 1]).write_text(data)
    else:
        print(data)
else:
    print('Filename Regions Missed Regions Cover Functions Missed Functions Executed Lines Missed Lines Cover Branches Missed Branches Cover')
    print('TOTAL 100 17 83.00% 100 9 91.00% 100 6 94.00% 100 77 23.00%')
sys.exit(int(os.environ['COVERAGE_RC']))
''')
            cargo.chmod(0o755)
            text = HARNESS.read_text()
            start = text.index('  if [ "$WITH_COVERAGE" = 1 ]')
            end = text.index('  # ---- RELEASE INTEGRITY', start)
            shell = '''set -uo pipefail
run() { local logfile="$1"; shift; "$@" >"$logfile" 2>&1; }
have() { [ "$COVERAGE_TOOL" = 1 ]; }
record() { printf '%s\\t%s\\t%s\\n' "$2" "$5" "$6"; }
fdiv() { awk -v a="$1" -v b="$2" 'BEGIN{printf "%.4f", a/b}'; }
fclamp() { awk -v a="$1" 'BEGIN{if(a>1)a=1; printf "%.4f", a}'; }
repo=fixture
dir="$WORK"
DEEP=0
fa=''
WITH_COVERAGE=1
coverage_probe() {
''' + text[start:end] + '\n}\ncoverage_probe\n'
            env = {**os.environ, 'PATH': str(work) + os.pathsep + os.environ['PATH'],
                   'HERE': str(ROOT / 'scripts'), 'WORK': scratch,
                   'COVERAGE_FIXTURE': str(fixture), 'COVERAGE_RC': str(result),
                   'COVERAGE_TOOL': '1' if tool else '0'}
            run = subprocess.run(['bash', '-c', shell], env=env,
                                 capture_output=True, text=True)
            self.assertEqual(run.returncode, 0, run.stderr)
            return run.stdout.strip().split('\t')

    def test_lines_are_not_regions_functions_or_branches(self):
        metric, measured, score = self.probe()
        self.assertEqual(metric, 'line_coverage')
        self.assertEqual(measured, '94% lines')
        self.assertAlmostEqual(float(score), 94 / 95, places=4)

    def test_failed_or_timed_out_measurement_cannot_score_a_report(self):
        for code in [1, 124]:
            with self.subTest(code=code):
                self.assertEqual(self.probe(result=code)[2], 'NA')

    def test_zero_coverable_lines_are_unmeasured(self):
        self.assertEqual(self.probe(count=0, covered=0, percent=0)[2], 'NA')

    def test_missing_tool_is_unmeasured(self):
        self.assertEqual(self.probe(tool=False)[2], 'NA')

    def test_malformed_or_inconsistent_totals_are_unmeasured(self):
        for kwargs in [{'malformed': True}, {'percent': float('nan')},
                       {'percent': 101}, {'covered': 101}, {'percent': 12}]:
            with self.subTest(kwargs=kwargs):
                self.assertEqual(self.probe(**kwargs)[2], 'NA')


class EcosystemProvenance(unittest.TestCase):
    def test_full_revision_and_large_dirty_worktree_are_preserved(self):
        with tempfile.TemporaryDirectory() as scratch:
            work = Path(scratch)
            git = work / 'git'
            git.write_text("#!/usr/bin/env python3\nimport signal, sys\nsignal.signal(signal.SIGPIPE, signal.SIG_DFL)\nif sys.argv[1] == 'rev-parse':\n    print('a' * (12 if '--short' in sys.argv else 40))\nelse:\n    for i in range(5000):\n        print(' M modified-file-' + str(i) + '-with-a-long-path.txt')\n")
            git.chmod(0o755)
            (work / 'Cargo.toml').write_text('[package]\nversion = "0.0.61"\n')
            text = HARNESS.read_text()
            start = text.index('  local sha dirty ver')
            end = text.index('  # ---- CORRECTNESS', start)
            code = 'set -uo pipefail\ndeclare -A REPO_SHA REPO_DIRTY REPO_VERSION\ndim() { :; }\nprobe() {\n  local repo=fixture\n' + text[start:end] + '\n  printf \'%s|%s\' "${REPO_SHA[fixture]}" "${REPO_DIRTY[fixture]}"\n}\nprobe\n'
            result = subprocess.run(['bash', '-c', code], cwd=work,
                                    env={**os.environ, 'PATH': str(work) + os.pathsep + os.environ['PATH']},
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, 'a' * 40 + '|dirty')


class EcosystemTestMeasurement(unittest.TestCase):
    def measure(self, build=0, tests=0, summary='test result: ok. 2 passed; 0 failed; 0 ignored;'):
        with tempfile.TemporaryDirectory() as scratch:
            text = HARNESS.read_text()
            start = text.index('  local log="$WORK/$repo.test"')
            end = text.index('  # YAML 1.2 spec conformance', start)
            bootstrap = 'set -uo pipefail\nfeat_args() { :; }\nrun() {\n local log="$1"; shift\n printf \'%s\\n\' "$*" >> "$WORK/calls"\n case " $* " in\n  *" --no-run "*) : > "$log"; return "$BUILD_RC" ;;\n  *) printf \'%s\\n\' "$TEST_SUMMARY" > "$log"; return "$TEST_RC" ;;\n esac\n}\nrecord() { printf \'%s\\t%s\\n\' "$5" "$6"; }\nfdiv() { awk -v a="$1" -v b="$2" \'BEGIN{if(b==0)print "0";else printf "%.4f", a/b}\'; }\nprobe() {\n local repo=fixture\n'
            code = bootstrap + text[start:end] + '\n}\nprobe\n'
            env = {**os.environ, 'WORK': scratch, 'PROBE_TIMEOUT': '1', 'BUILD_TIMEOUT': '1',
                   'BUILD_RC': str(build), 'TEST_RC': str(tests), 'TEST_SUMMARY': summary}
            result = subprocess.run(['bash', '-c', code], env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            return result.stdout.strip().split('\t'), (Path(scratch) / 'calls').read_text().splitlines()

    def test_build_timeout_is_unmeasured_and_does_not_rebuild(self):
        row, calls = self.measure(build=124)
        self.assertEqual(row[-1], 'NA')
        self.assertEqual(len(calls), 1)

    def test_actual_build_failure_stays_a_failure_without_duplicate_build(self):
        row, calls = self.measure(build=101)
        self.assertEqual(float(row[-1]), 0)
        self.assertEqual(len(calls), 1)

    def test_zero_executed_tests_cannot_claim_a_measured_pass_ratio(self):
        row, _ = self.measure(summary='test result: ok. 0 passed; 0 failed; 0 ignored;')
        self.assertEqual(row[-1], 'NA')

    def test_actual_test_failure_and_timeout_remain_distinct(self):
        row, _ = self.measure(tests=1, summary='test result: FAILED. 1 passed; 1 failed; 0 ignored;')
        self.assertEqual(float(row[-1]), 0.5)
        row, _ = self.measure(tests=124)
        self.assertEqual(row[-1], 'NA')


if __name__ == '__main__':
    unittest.main()
