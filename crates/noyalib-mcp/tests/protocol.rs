// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! End-to-end protocol tests for `noyalib-mcp`.
//!
//! Spawns the binary and drives it via stdin/stdout with the same
//! JSON-RPC 2.0 messages a real MCP client (Claude, Cursor, Zed)
//! would send. Asserts the wire-format contract.

#![allow(missing_docs)]

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_noyalib-mcp")
}

/// Send a sequence of JSON-RPC messages, return the responses in
/// the order they came back. Notification messages (no `id`) get no
/// reply, so the response count may be smaller than the input.
fn round_trip(messages: &[Value]) -> Vec<Value> {
    let mut child = Command::new(bin())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn noyalib-mcp");

    let mut stdin = child.stdin.take().expect("stdin");
    for m in messages {
        writeln!(stdin, "{}", serde_json::to_string(m).unwrap()).unwrap();
    }
    drop(stdin);

    let stdout = child.stdout.take().expect("stdout");
    let reader = BufReader::new(stdout);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str(&line).expect("response is JSON"));
    }
    let status = child.wait().expect("wait");
    assert!(status.success(), "server exited with non-zero status");
    out
}

fn tempfile(contents: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "noyalib-mcp-test-{}-{}.yaml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn initialize_returns_protocol_version_and_server_info() {
    let resp = round_trip(&[json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {},
        "id": 1
    })]);
    assert_eq!(resp.len(), 1);
    assert_eq!(resp[0]["jsonrpc"], "2.0");
    assert_eq!(resp[0]["id"], 1);
    assert_eq!(resp[0]["result"]["serverInfo"]["name"], "noyalib-mcp");
    assert!(resp[0]["result"]["protocolVersion"].is_string());
    assert!(resp[0]["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn tools_list_announces_get_and_set() {
    let resp = round_trip(&[json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 7
    })]);
    let tools = resp[0]["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"noyalib_get"));
    assert!(names.contains(&"noyalib_set"));
    // Every tool must have an inputSchema (clients use it for arg
    // validation and prompt-generation).
    for t in tools {
        assert!(t["inputSchema"].is_object());
    }
}

#[test]
fn tool_call_get_reads_value_at_path() {
    let path = tempfile("name: noyalib\nport: 8080\n");
    let resp = round_trip(&[json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "noyalib_get",
            "arguments": { "file": path.to_str().unwrap(), "path": "port" }
        },
        "id": 11
    })]);
    let text = resp[0]["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert_eq!(text, "8080");
}

#[test]
fn tool_call_set_preserves_comments() {
    let path = tempfile(
        "# version is bumped by Renovate\n\
         version: 0.0.1  # do not edit by hand\n\
         name: noyalib\n",
    );
    let _resp = round_trip(&[json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "noyalib_set",
            "arguments": {
                "file": path.to_str().unwrap(),
                "path": "version",
                "value": "0.0.2"
            }
        },
        "id": 13
    })]);
    let after = std::fs::read_to_string(&path).unwrap();
    // The CST guarantee: only the touched span changes.
    assert!(after.contains("version: 0.0.2"));
    assert!(after.contains("# version is bumped by Renovate"));
    assert!(after.contains("# do not edit by hand"));
    assert!(after.contains("name: noyalib"));
}

#[test]
fn unknown_method_returns_error() {
    let resp = round_trip(&[json!({
        "jsonrpc": "2.0",
        "method": "definitely/not/a/method",
        "id": 99
    })]);
    assert_eq!(resp[0]["error"]["code"], -32601);
}

#[test]
fn notification_gets_no_reply() {
    // A `notifications/initialized` message has no id, so the
    // server should process it silently — no response on stdout.
    let resp = round_trip(&[
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
        // Send a regular request after to keep the round-trip
        // collector unblocked and prove the server stays alive.
        json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "id": 1
        }),
    ]);
    assert_eq!(resp.len(), 1, "notification must not produce a response");
    assert_eq!(resp[0]["id"], 1);
}
