// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Tool registry for the MCP server.
//!
//! Each entry in [`descriptors`] is the JSON Schema that a client
//! sees from `tools/list`; [`call`] is the dispatch entry point for
//! `tools/call`. Tools delegate the actual YAML work to noyalib's
//! `cst::Document` so edits round-trip with comments, indentation,
//! and sibling entries preserved byte-for-byte.

use noyalib::cst::parse_document;
use serde_json::{Value as JsonValue, json};
use std::fs;

/// Descriptors returned to MCP clients via `tools/list`.
pub fn descriptors() -> Vec<JsonValue> {
    vec![
        json!({
            "name": "noyalib_get",
            "description": "Read the YAML value at a dotted/indexed path \
                in the given file. Returns the source slice exactly — no \
                re-quoting, no canonicalisation. Preserves comments and \
                formatting for any later `noyalib_set`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the YAML file on disk."
                    },
                    "path": {
                        "type": "string",
                        "description": "Dotted/indexed path into the YAML, \
                            e.g. `server.host` or `items[0].name`."
                    }
                },
                "required": ["file", "path"]
            }
        }),
        json!({
            "name": "noyalib_set",
            "description": "Set the YAML value at a dotted/indexed path \
                in the given file. Only the touched span is rewritten — \
                every comment, blank line, and sibling entry is preserved \
                byte-for-byte. Useful for Renovate-style version bumps and \
                config patches by AI agents.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "Path to the YAML file on disk."
                    },
                    "path": {
                        "type": "string",
                        "description": "Dotted/indexed path into the YAML."
                    },
                    "value": {
                        "type": "string",
                        "description": "Replacement value as a YAML \
                            fragment (e.g. `0.0.2`, `\\\"hello\\\"`, \
                            `[1, 2, 3]`). Must parse in the target \
                            position; the document is left unchanged on \
                            parse error."
                    }
                },
                "required": ["file", "path", "value"]
            }
        }),
    ]
}

/// `tools/call` dispatcher. Returns the JSON-RPC `result` payload on
/// success, or `(code, message)` for an error envelope.
pub fn call(params: JsonValue) -> Result<JsonValue, (i32, String)> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (-32602, "missing field: name".to_string()))?;
    let args = params.get("arguments").cloned().unwrap_or(JsonValue::Null);

    match name {
        "noyalib_get" => tool_get(&args),
        "noyalib_set" => tool_set(&args),
        _ => Err((-32601, format!("unknown tool: {name}"))),
    }
}

/// Wrap a tool result string into the MCP `tools/call` reply shape.
fn ok_text(text: String) -> JsonValue {
    json!({
        "content": [
            { "type": "text", "text": text }
        ]
    })
}

fn tool_get(args: &JsonValue) -> Result<JsonValue, (i32, String)> {
    let file = arg_str(args, "file")?;
    let path = arg_str(args, "path")?;
    let src = fs::read_to_string(file).map_err(|e| (-32000, format!("read {file}: {e}")))?;
    let doc = parse_document(&src).map_err(|e| (-32001, format!("parse {file}: {e}")))?;
    match doc.get(path) {
        Some(value) => Ok(ok_text(value.to_string())),
        None => Err((-32002, format!("path not found in {file}: {path}"))),
    }
}

fn tool_set(args: &JsonValue) -> Result<JsonValue, (i32, String)> {
    let file = arg_str(args, "file")?;
    let path = arg_str(args, "path")?;
    let value = arg_str(args, "value")?;
    let src = fs::read_to_string(file).map_err(|e| (-32000, format!("read {file}: {e}")))?;
    let mut doc = parse_document(&src).map_err(|e| (-32001, format!("parse {file}: {e}")))?;
    doc.set(path, value)
        .map_err(|e| (-32003, format!("set {path} = {value}: {e}")))?;
    write_atomic(file, doc.to_string().as_bytes())
        .map_err(|e| (-32000, format!("write {file}: {e}")))?;
    Ok(ok_text(format!(
        "set {path} = {value} in {file} (lossless: comments and formatting preserved)"
    )))
}

/// Write `bytes` to `file` atomically: write to a sibling temp
/// file, fsync it, then `rename` over the target. The rename is
/// atomic on POSIX and `MoveFileExW(MOVEFILE_REPLACE_EXISTING |
/// MOVEFILE_WRITE_THROUGH)` semantics on Windows, so concurrent
/// readers always see either the old or the new contents — never
/// a half-written truncation. The fsync also closes a Windows
/// race where `fs::write` returned before the kernel page cache
/// flushed, leaving a freshly-spawned reader to observe the old
/// bytes.
fn write_atomic(file: &str, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::path::Path;
    let target = Path::new(file);
    let parent = target.parent().unwrap_or(Path::new("."));
    let stem = target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("noyalib-set");
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = parent.join(format!(".{stem}.{pid}.{nanos}.tmp"));
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, target)
}

fn arg_str<'a>(args: &'a JsonValue, key: &str) -> Result<&'a str, (i32, String)> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| (-32602, format!("missing string argument: {key}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Allocate a unique scratch path under the system temp dir so
    /// parallel test runs don't collide.
    fn temp_path(label: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("noyalib-mcp-{label}-{pid}-{id}.yml"))
    }

    fn write_temp(label: &str, contents: &str) -> PathBuf {
        let p = temp_path(label);
        fs::write(&p, contents).unwrap();
        p
    }

    // ── descriptors ────────────────────────────────────────────────

    #[test]
    fn descriptors_lists_both_tools_with_input_schemas() {
        let d = descriptors();
        assert_eq!(d.len(), 2);
        let names: Vec<&str> = d.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"noyalib_get"));
        assert!(names.contains(&"noyalib_set"));
        for tool in &d {
            assert!(tool["description"].is_string());
            assert_eq!(tool["inputSchema"]["type"].as_str(), Some("object"));
            assert!(tool["inputSchema"]["required"].is_array());
        }
    }

    // ── call dispatcher ────────────────────────────────────────────

    #[test]
    fn call_rejects_missing_name() {
        let err = call(json!({})).unwrap_err();
        assert_eq!(err.0, -32602);
        assert!(err.1.contains("name"));
    }

    #[test]
    fn call_rejects_unknown_tool() {
        let err = call(json!({"name": "frobnicate", "arguments": {}})).unwrap_err();
        assert_eq!(err.0, -32601);
        assert!(err.1.contains("frobnicate"));
    }

    #[test]
    fn call_routes_to_get() {
        let p = write_temp("call-get", "name: noyalib\n");
        let v = call(json!({
            "name": "noyalib_get",
            "arguments": { "file": p.to_str().unwrap(), "path": "name" }
        }))
        .unwrap();
        let text = v["content"][0]["text"].as_str().unwrap();
        assert_eq!(text, "noyalib");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn call_routes_to_set() {
        let p = write_temp("call-set", "version: 1\n");
        let v = call(json!({
            "name": "noyalib_set",
            "arguments": {
                "file": p.to_str().unwrap(),
                "path": "version",
                "value": "2"
            }
        }))
        .unwrap();
        assert!(
            v["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("set version")
        );
        let updated = fs::read_to_string(&p).unwrap();
        assert_eq!(updated, "version: 2\n");
        let _ = fs::remove_file(&p);
    }

    // ── tool_get error paths ───────────────────────────────────────

    #[test]
    fn tool_get_missing_file_arg_errors() {
        let err = tool_get(&json!({"path": "k"})).unwrap_err();
        assert_eq!(err.0, -32602);
    }

    #[test]
    fn tool_get_missing_path_arg_errors() {
        let err = tool_get(&json!({"file": "/tmp/x.yml"})).unwrap_err();
        assert_eq!(err.0, -32602);
    }

    #[test]
    fn tool_get_unreadable_file_errors() {
        let err = tool_get(&json!({
            "file": "/this/path/definitely/does/not/exist.yml",
            "path": "k"
        }))
        .unwrap_err();
        assert_eq!(err.0, -32000);
    }

    #[test]
    fn tool_get_unparseable_yaml_errors() {
        let p = write_temp("get-parse", "key: [\n");
        let err = tool_get(&json!({
            "file": p.to_str().unwrap(),
            "path": "key"
        }))
        .unwrap_err();
        assert_eq!(err.0, -32001);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn tool_get_path_not_found_errors() {
        let p = write_temp("get-missing", "a: 1\n");
        let err = tool_get(&json!({
            "file": p.to_str().unwrap(),
            "path": "missing"
        }))
        .unwrap_err();
        assert_eq!(err.0, -32002);
        let _ = fs::remove_file(&p);
    }

    // ── tool_set error paths ───────────────────────────────────────

    #[test]
    fn tool_set_missing_args_errors() {
        let err = tool_set(&json!({})).unwrap_err();
        assert_eq!(err.0, -32602);
    }

    #[test]
    fn tool_set_unreadable_file_errors() {
        let err = tool_set(&json!({
            "file": "/this/path/does/not/exist.yml",
            "path": "k",
            "value": "v"
        }))
        .unwrap_err();
        assert_eq!(err.0, -32000);
    }

    #[test]
    fn tool_set_unparseable_source_errors() {
        let p = write_temp("set-parse", "k: [\n");
        let err = tool_set(&json!({
            "file": p.to_str().unwrap(),
            "path": "k",
            "value": "v"
        }))
        .unwrap_err();
        assert_eq!(err.0, -32001);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn tool_set_unknown_path_errors() {
        let p = write_temp("set-bad-path", "a: 1\n");
        let err = tool_set(&json!({
            "file": p.to_str().unwrap(),
            "path": "missing.path",
            "value": "v"
        }))
        .unwrap_err();
        assert_eq!(err.0, -32003);
        let _ = fs::remove_file(&p);
    }
}
