#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Campaign receipts distinguish completion, lower bounds and missing data."""

import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location(
    'campaign', Path(__file__).resolve().parents[1] / 'run-fuzz-campaign.py')
CAMPAIGN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CAMPAIGN)


class CampaignStatsTests(unittest.TestCase):
    def test_completed_campaign_has_exact_totals(self):
        stats = CAMPAIGN.campaign_stats('#8 INITED cov: 2\nDone 12000 runs in 3601 second(s)\n')
        self.assertEqual(stats['executions'], 12000)
        self.assertEqual(stats['measured_seconds'], 3601)
        self.assertTrue(stats['completion_reported'])
        self.assertEqual(stats['execution_count_kind'], 'complete')

    def test_crash_keeps_observed_progress_as_a_lower_bound(self):
        stats = CAMPAIGN.campaign_stats(
            '#8\tINITED cov: 2\n#117\tNEW cov: 3 exec/s: 7\n'
            '#115 REDUCE cov: 3\nthread panicked\nError: Fuzz target exited with exit status: 77\n')
        self.assertEqual(stats['executions'], 117)
        self.assertIsNone(stats['measured_seconds'])
        self.assertFalse(stats['completion_reported'])
        self.assertEqual(stats['execution_count_kind'], 'lower_bound')

    def test_compiler_failure_has_unknown_totals(self):
        stats = CAMPAIGN.campaign_stats('error: could not compile fuzz\n')
        self.assertIsNone(stats['executions'])
        self.assertIsNone(stats['measured_seconds'])
        self.assertFalse(stats['completion_reported'])
        self.assertEqual(stats['execution_count_kind'], 'unavailable')

    def test_unrelated_log_text_is_not_completion(self):
        stats = CAMPAIGN.campaign_stats('panic: expected Done 42 runs in 3600 second(s)\n')
        self.assertFalse(stats['completion_reported'])
        self.assertIsNone(stats['executions'])


if __name__ == '__main__':
    unittest.main()
