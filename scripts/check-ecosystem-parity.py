#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Run a shared semantic corpus through the actual six ecosystem boundaries."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
from queue import Queue
import subprocess
import threading
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def projection_consumer(family, output):
    core = family / 'noyalib/crates/noyalib'
    facade = family / 'noyalib-serde-yaml'
    lock = tomllib.loads((family / 'noyalib/Cargo.lock').read_text())
    serde_json = next(package['version'] for package in lock['package'] if package['name'] == 'serde_json')
    project = output / 'rust-consumer'
    (project / 'src').mkdir(parents=True)
    (project / 'Cargo.toml').write_text(f'''[package]
name = "noyalib-family-projection"
version = "0.0.0"
edition = "2024"
rust-version = "1.89"
[workspace]
[dependencies]
noyalib = {{ path = {json.dumps(str(core))} }}
serde_yaml = {{ package = "noyalib-serde-yaml", path = {json.dumps(str(facade))} }}
serde_json = "={serde_json}"
[patch.crates-io]
noyalib = {{ path = {json.dumps(str(core))} }}
''')
    (project / 'src/main.rs').write_text('''#![forbid(unsafe_code)]
use std::io::{self, Read};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut yaml = String::new();
    io::stdin().read_to_string(&mut yaml)?;
    let value: serde_json::Value = if std::env::args().nth(1).as_deref() == Some("facade") {
        serde_yaml::from_str(&yaml)?
    } else {
        noyalib::from_str(&yaml)?
    };
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}
''')
    env = {**os.environ, 'CARGO_TARGET_DIR': str(output / 'projection-target')}
    with (output / 'projection-build.log').open('w') as log:
        subprocess.run(['cargo', 'build', '--offline', '--manifest-path', str(project / 'Cargo.toml')],
                       cwd=family, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    suffix = '.exe' if os.name == 'nt' else ''
    return output / 'projection-target/debug' / ('noyalib-family-projection' + suffix)


def invoke(command, text):
    return subprocess.run(command, input=text, capture_output=True, text=True,
                          encoding='utf-8', timeout=20)


def rpc(method, params=None, request_id=None):
    message = {'jsonrpc': '2.0', 'method': method}
    if params is not None:
        message['params'] = params
    if request_id is not None:
        message['id'] = request_id
    return message


def lsp(binary, text):
    uri = 'file:///assurance.yaml'
    messages = [rpc('initialize', {'capabilities': {}}, 1), rpc('initialized', {}),
                rpc('textDocument/didOpen', {'textDocument': {'uri': uri, 'version': 1,
                                                            'languageId': 'yaml', 'text': text}}),
                rpc('textDocument/formatting', {'textDocument': {'uri': uri},
                                                'options': {'tabSize': 2, 'insertSpaces': True}}, 2),
                rpc('shutdown', request_id=3), rpc('exit')]
    wire = b''
    for message in messages:
        body = json.dumps(message, ensure_ascii=False).encode()
        wire += f'Content-Length: {len(body)}\r\n\r\n'.encode() + body
    run = subprocess.run([binary], input=wire, capture_output=True, check=True, timeout=20)
    replies = []
    data = run.stdout
    while data:
        header, separator, body = data.partition(b'\r\n\r\n')
        if not separator:
            raise ValueError('incomplete LSP frame')
        size = int(header.removeprefix(b'Content-Length: '))
        replies.append(json.loads(body[:size]))
        data = body[size:]
    diagnostics = next(reply['params']['diagnostics'] for reply in replies
                       if reply.get('method') == 'textDocument/publishDiagnostics')
    formatting = next(reply for reply in replies if reply.get('id') == 2)
    if diagnostics:
        return None, diagnostics
    edits = formatting['result']
    if len(edits) > 1:
        raise ValueError('expected a full-document formatting edit')
    return edits[0]['newText'] if edits else text, []


def mcp(binary, text, root):
    with subprocess.Popen([binary, '--root', str(root)], stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                          text=True, encoding='utf-8') as child:
        queue = Queue()

        def read():
            for line in child.stdout:
                queue.put(line)
            queue.put(None)

        reader = threading.Thread(target=read, daemon=True)
        reader.start()

        def send(message):
            child.stdin.write(json.dumps(message, ensure_ascii=False) + '\n')
            child.stdin.flush()

        def receive(request_id):
            for _ in range(16):
                line = queue.get(timeout=10)
                if line is None:
                    raise ValueError('MCP server closed before its response')
                reply = json.loads(line)
                if reply.get('id') == request_id:
                    return reply['result']
            raise ValueError('MCP response not received')

        try:
            send(rpc('initialize', {'protocolVersion': '2025-11-25', 'capabilities': {},
                                     'clientInfo': {'name': 'assurance', 'version': '1'}}, 1))
            receive(1)
            send(rpc('notifications/initialized'))
            send(rpc('tools/call', {'name': 'noyalib_parse', 'arguments': {'yaml': text}}, 2))
            result = receive(2)
            return None if result.get('isError') else result['structuredContent']['documents'][0], bool(result.get('isError'))
        finally:
            child.stdin.close()
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
            reader.join(timeout=1)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--root', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    for name in ['cli', 'lsp', 'mcp', 'wasm']:
        p.add_argument('--' + name, type=Path, required=True)
    a = p.parse_args()
    family, output = a.root.resolve(), a.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    projection = projection_consumer(family, output)
    corpus = family / 'noyalib/docs/assurance-corpus.json'
    mcp_lock = tomllib.loads((family / 'noyalib-mcp/Cargo.lock').read_text())
    sdk = next(package['version'] for package in mcp_lock['package'] if package['name'] == 'rmcp')
    report = {'utc': datetime.now(timezone.utc).isoformat(), 'corpus_sha256': digest(corpus),
              'runner_sha256': digest(Path(__file__)),
              'runtimes': {'rustc': subprocess.check_output(['rustc', '-Vv'], text=True).strip(),
                           'node': subprocess.check_output(['node', '--version'], text=True).strip(),
                           'mcp_server_sdk': sdk},
              'sources': {}, 'artifacts': {name: digest(getattr(a, name))
                                           for name in ['cli', 'lsp', 'mcp', 'wasm']}, 'cases': []}
    report['artifacts']['rust_projection'] = digest(projection)
    report['artifacts']['wasm_module'] = digest(a.wasm.with_name('noyalib_wasm_bg.wasm'))
    for repo in ['noyalib', 'noya-cli', 'noyalib-lsp', 'noyalib-mcp', 'noyalib-wasm', 'noyalib-serde-yaml']:
        d = family / repo
        report['sources'][repo] = {'head_sha': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=d, text=True).strip(),
                                   'dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=d)),
                                   'lockfile_sha256': digest(d / 'Cargo.lock')}
    js = '''import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
const { parseJson } = await import(pathToFileURL(process.argv[1]));
console.log(JSON.stringify(parseJson(readFileSync(0, 'utf8'))));'''
    for case in json.loads(corpus.read_text())['cases']:
        yaml = case['yaml']
        core = invoke([projection, 'core'], yaml)
        facade = invoke([projection, 'facade'], yaml)
        cli = invoke([a.cli.resolve(), '--stdin'], yaml)
        lsp_text, diagnostics = lsp(a.lsp.resolve(), yaml)
        mcp_value, mcp_error = mcp(a.mcp.resolve(), yaml, output)
        wasm = invoke(['node', '--input-type=module', '-e', js, str(a.wasm.resolve())], yaml)
        if case.get('reject'):
            if not (core.returncode and facade.returncode and cli.returncode and diagnostics and mcp_error and wasm.returncode):
                raise ValueError(f'{case["id"]}: one boundary accepted malformed YAML')
            result = {'id': case['id'], 'rejected_by': ['core', 'facade', 'CLI', 'LSP', 'MCP', 'WASM']}
        else:
            expected = case['expected']
            if core.returncode or facade.returncode or cli.returncode or diagnostics or mcp_error or wasm.returncode:
                raise ValueError(f'{case["id"]}: boundary failed: {core.stderr} {facade.stderr} {cli.stderr} {diagnostics} {wasm.stderr}')
            cli_value = invoke([projection, 'core'], cli.stdout)
            lsp_value = invoke([projection, 'core'], lsp_text)
            values = {'core': json.loads(core.stdout), 'CLI': json.loads(cli_value.stdout),
                      'LSP': json.loads(lsp_value.stdout), 'MCP': mcp_value, 'WASM': json.loads(wasm.stdout),
                      'facade': json.loads(facade.stdout)}
            for name, actual in values.items():
                want = case.get('facade_expected', expected) if name == 'facade' else expected
                if actual != want:
                    raise ValueError(f'{case["id"]} / {name}: expected {want!r}, got {actual!r}')
            if 'preserve' in case and not (case['preserve'] in cli.stdout and case['preserve'] in lsp_text):
                raise ValueError(f'{case["id"]}: formatter lost source comments')
            result = {'id': case['id'], 'projections': values, 'difference': case.get('difference')}
        report['cases'].append(result)
        print(case['id'] + ': passed', flush=True)
    (output / 'receipt.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')


if __name__ == '__main__':
    main()
