#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Exercise the consumer adapter against a real subprocess protocol peer."""

import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    'ecosystem_parity', Path(__file__).parents[1] / 'check-ecosystem-parity.py')
PARITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PARITY)

PEER = '''import json, sys
for line in sys.stdin:
    request = json.loads(line)
    if request.get('id') == 1:
        result = {{}}
    elif request.get('id') == 2:
        result = {{'structuredContent': {{'documents': [{{'ok': True}}]}}}}
    else:
        continue
    print(json.dumps({{'jsonrpc': '2.0', 'id': request['id'], 'result': result}}), flush=True)
    if request.get('id') == 2 and {status}:
        sys.exit({status})
'''


class McpExitTests(unittest.TestCase):
    def invoke(self, status):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            peer = root / 'peer.py'
            peer.write_text(PEER.format(status=status))
            popen = subprocess.Popen

            def launch(command, **kwargs):
                return popen([sys.executable, str(peer), *map(str, command[1:])], **kwargs)

            with patch.object(PARITY.subprocess, 'Popen', launch):
                return PARITY.mcp('fixture-peer', 'ok: true\n', root)

    def test_success_requires_response_and_successful_exit(self):
        self.assertEqual(self.invoke(0), ([{'ok': True}], None))

    def test_a_valid_response_cannot_hide_a_failed_server_exit(self):
        with self.assertRaisesRegex(ValueError, 'MCP server exited with status 1'):
            self.invoke(1)


if __name__ == '__main__':
    unittest.main()
