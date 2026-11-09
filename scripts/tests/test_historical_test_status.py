#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Exercise the workflow's historical-test publisher without calling GitHub."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest


WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/security.yml"
HISTORICAL_SHA = "0c54c2c7fbe212e3dc662666f92f615105f8345d"
DISPATCH_SHA = "c2ca19a39236ef1fcb1ccaaebd597d82b8956573"


class HistoricalTestStatus(unittest.TestCase):
    def setUp(self):
        match = re.search(
            r"(?ms)^          # BEGIN HISTORICAL TEST STATUS\n"
            r"(.*?)^          # END HISTORICAL TEST STATUS$",
            WORKFLOW.read_text(),
        )
        self.assertIsNotNone(match, "historical test evidence publisher is absent")
        self.code = textwrap.dedent(match.group(1))
        self.scratch = tempfile.TemporaryDirectory(prefix="noyalib-test-status-")
        self.addCleanup(self.scratch.cleanup)
        self.capture = Path(self.scratch.name) / "request.json"
        gh = Path(self.scratch.name) / "gh"
        gh.write_text(
            f"#!{sys.executable}\n"
            "import json, os, pathlib, sys\n"
            "pathlib.Path(os.environ['CAPTURE']).write_text(json.dumps(\n"
            "    {'args': sys.argv[1:], 'body': json.load(sys.stdin)}))\n"
        )
        gh.chmod(0o700)

    def report(self, **overrides):
        env = {
            "PATH": self.scratch.name + os.pathsep + os.defpath,
            "CAPTURE": str(self.capture),
            "GH_TOKEN": "test-token",
            "GITHUB_REPOSITORY": "example/noyalib",
            "GITHUB_RUN_ID": "12345",
            "GITHUB_SERVER_URL": "https://github.com",
            "GITHUB_SHA": DISPATCH_SHA,
            "EVENT_NAME": "workflow_dispatch",
            "RUN_TESTS": "true",
            "TARGET_SHA": HISTORICAL_SHA,
            "TESTED_SHA": HISTORICAL_SHA,
            "TEST_RESULT": "success",
        }
        env.update(overrides)
        result = subprocess.run(
            [sys.executable, "-c", self.code], env=env, capture_output=True, text=True
        )
        request = json.loads(self.capture.read_text()) if self.capture.exists() else None
        return result, request

    def test_success_belongs_to_tested_commit_and_actual_run(self):
        result, request = self.report()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("repos/example/noyalib/statuses/" + HISTORICAL_SHA, request["args"])
        self.assertNotIn(DISPATCH_SHA, " ".join(request["args"]))
        self.assertEqual(request["body"]["state"], "success")
        self.assertEqual(request["body"]["context"], "ci/historical-tests")
        self.assertEqual(
            request["body"]["target_url"],
            "https://github.com/example/noyalib/actions/runs/12345",
        )

    def test_failure_never_records_success(self):
        result, request = self.report(TEST_RESULT="failure")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(request["body"]["state"], "failure")

    def test_cancelled_or_skipped_tests_record_error(self):
        for outcome in ["cancelled", "skipped"]:
            with self.subTest(outcome=outcome):
                result, request = self.report(TEST_RESULT=outcome, TESTED_SHA="")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(request["body"]["state"], "error")

    def test_other_commit_cannot_receive_success(self):
        result, request = self.report(TESTED_SHA=DISPATCH_SHA)
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(request)

    def test_unconfirmed_checkout_cannot_receive_success(self):
        result, request = self.report(TESTED_SHA="")
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(request)

    def test_invalid_or_missing_commit_is_rejected(self):
        for sha in ["", "main", HISTORICAL_SHA + "; exit 0"]:
            with self.subTest(sha=sha):
                result, request = self.report(TARGET_SHA=sha)
                self.assertNotEqual(result.returncode, 0)
                self.assertIsNone(request)

    def test_unfinished_or_unknown_result_is_rejected(self):
        for outcome in ["", "in_progress", "unknown"]:
            with self.subTest(outcome=outcome):
                result, request = self.report(TEST_RESULT=outcome)
                self.assertNotEqual(result.returncode, 0)
                self.assertIsNone(request)

    def test_pull_request_event_cannot_publish(self):
        result, request = self.report(EVENT_NAME="pull_request")
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(request)

    def test_dispatch_without_requested_tests_cannot_publish(self):
        result, request = self.report(RUN_TESTS="false")
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(request)


if __name__ == "__main__":
    unittest.main()
