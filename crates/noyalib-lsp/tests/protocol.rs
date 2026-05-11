// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! End-to-end protocol tests for `noyalib-lsp`.
//!
//! Spawns the binary and drives it via stdin / stdout with the
//! same `Content-Length`-framed JSON-RPC 2.0 messages a real LSP
//! client (VS Code, Zed, Neovim) sends. Asserts the wire-format
//! contract.

#![allow(missing_docs)]

use serde_json::{Value, json};
use std::io::{Read, Write};
use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_noyalib-lsp")
}

fn frame(payload: &Value) -> Vec<u8> {
    let body = serde_json::to_string(payload).unwrap();
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

/// Send a sequence of LSP messages, return every framed reply +
/// notification produced before the server hits EOF.
fn round_trip(messages: &[Value]) -> Vec<Value> {
    let mut child = Command::new(bin())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn noyalib-lsp");

    let mut stdin = child.stdin.take().expect("stdin");
    for m in messages {
        stdin.write_all(&frame(m)).unwrap();
    }
    drop(stdin);

    let mut stdout = child.stdout.take().expect("stdout");
    let mut all = Vec::new();
    let _ = stdout.read_to_end(&mut all);
    let text = String::from_utf8_lossy(&all);

    // Naive frame splitter: find every `Content-Length:` block.
    let mut out = Vec::new();
    let mut rest = text.as_ref();
    while let Some(idx) = rest.find("Content-Length:") {
        let after_header = &rest[idx..];
        let Some(header_end) = after_header.find("\r\n\r\n") else {
            break;
        };
        let header = &after_header[..header_end];
        let length: usize = header
            .lines()
            .find_map(|l| {
                l.strip_prefix("Content-Length:")
                    .map(str::trim)
                    .and_then(|n| n.parse().ok())
            })
            .unwrap_or(0);
        let body_start = idx + header_end + 4;
        if body_start + length > rest.len() {
            break;
        }
        let body = &rest[body_start..body_start + length];
        if let Ok(v) = serde_json::from_str(body) {
            out.push(v);
        }
        rest = &rest[body_start + length..];
    }
    let _ = child.wait();
    out
}

// ── tests ─────────────────────────────────────────────────────

#[test]
fn initialize_returns_capabilities() {
    let resps = round_trip(&[json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "id": 1,
        "params": {}
    })]);
    assert_eq!(resps.len(), 1);
    let r = &resps[0];
    assert_eq!(
        r["result"]["serverInfo"]["name"].as_str(),
        Some("noyalib-lsp")
    );
    assert_eq!(
        r["result"]["capabilities"]["documentFormattingProvider"].as_bool(),
        Some(true),
    );
}

#[test]
fn did_open_publishes_diagnostics_notification() {
    let resps = round_trip(&[
        json!({"jsonrpc": "2.0", "method": "initialize", "id": 1, "params": {}}),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///tmp/x.yaml",
                    "languageId": "yaml",
                    "version": 1,
                    "text": "a: 1\n"
                }
            }
        }),
    ]);
    // 1 reply (initialize) + 1 notification (publishDiagnostics).
    assert!(resps.len() >= 2);
    let note = resps
        .iter()
        .find(|r| r["method"].as_str() == Some("textDocument/publishDiagnostics"))
        .expect("expected publishDiagnostics notification");
    assert_eq!(note["params"]["uri"].as_str(), Some("file:///tmp/x.yaml"));
}

#[test]
fn formatting_round_trip_returns_text_edits_array() {
    let resps = round_trip(&[
        json!({"jsonrpc": "2.0", "method": "initialize", "id": 1, "params": {}}),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": "file:///tmp/y.yaml",
                "languageId": "yaml",
                "version": 1,
                "text": "name: noyalib\n"
            }}
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/formatting",
            "id": 2,
            "params": {
                "textDocument": {"uri": "file:///tmp/y.yaml"},
                "options": {"tabSize": 2, "insertSpaces": true}
            }
        }),
    ]);
    let fmt_reply = resps
        .iter()
        .find(|r| r["id"].as_i64() == Some(2))
        .expect("formatting reply");
    assert!(fmt_reply["result"].is_array());
}

#[test]
fn hover_round_trip_returns_markdown_or_null() {
    let resps = round_trip(&[
        json!({"jsonrpc": "2.0", "method": "initialize", "id": 1, "params": {}}),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": "file:///tmp/z.yaml",
                "languageId": "yaml",
                "version": 1,
                "text": "k: v\n"
            }}
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "id": 3,
            "params": {
                "textDocument": {"uri": "file:///tmp/z.yaml"},
                "position": {"line": 0, "character": 0}
            }
        }),
    ]);
    let hover_reply = resps
        .iter()
        .find(|r| r["id"].as_i64() == Some(3))
        .expect("hover reply");
    let result = &hover_reply["result"];
    // null or { contents: { kind: 'markdown', value: ... } }.
    assert!(result.is_null() || result["contents"]["kind"].as_str() == Some("markdown"));
}
