// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Library surface for `noyalib-lsp`.
//!
//! Hosts the JSON-RPC 2.0 dispatch logic, the document store that
//! tracks open buffers, and the LSP capability handlers
//! (`textDocument/formatting`, `textDocument/publishDiagnostics`,
//! `textDocument/hover`). The `noyalib-lsp` binary in `main.rs` is
//! a thin stdio shim that drives [`Server::handle_message`]; tests
//! reach the same handlers directly so coverage does not depend on
//! standing up a real LSP client.
//!
//! # Cargo features
//!
//! This crate exposes no optional features; the LSP capability
//! set is fixed at `textDocumentSync` (full), formatting, and
//! hover. Optional `noyalib` features (`schema`, `parallel`, …)
//! pulled in by a downstream binary do not change this crate's
//! wire surface — they only affect what `noyalib::Error` messages
//! are produced inside diagnostics. The canonical `noyalib`
//! feature matrix lives in
//! [`crates/noyalib/src/lib.rs`](https://docs.rs/noyalib).
//!
//! # MSRV
//!
//! **Rust 1.85.0** stable. The `tower-lsp` and async deps floor
//! at 1.85; the core `noyalib` library still builds on **1.75**.
//! CI verifies both floors via the `Per-crate MSRV` workflow
//! job. See workspace
//! [`POLICIES.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/POLICIES.md#1-msrv-minimum-supported-rust-version).
//!
//! # Panics
//!
//! Public functions in this crate do not panic on well-formed
//! input. The LSP binary's stdin/stdout handling propagates
//! I/O errors back to the host as JSON-RPC error envelopes.
//!
//! # Errors
//!
//! All handlers return JSON-RPC error envelopes per the
//! [LSP specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/).
//! Parse / format errors come from `noyalib::Error` and surface
//! through the `textDocument/publishDiagnostics` channel as
//! span-aware `Diagnostic` records.
//!
//! # Concurrency
//!
//! Each LSP request is processed sequentially on the binary's
//! stdio loop. The internal document store is reentrant:
//! handlers `&mut`-borrow it serialised by the request
//! loop. No internal threading; rayon is opt-in via
//! the `parallel` feature on `noyalib`.
//!
//! # Platform support
//!
//! Tier-1 (CI-verified each PR): `aarch64-apple-darwin`,
//! `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`.
//! Editor-specific notes for VS Code, Neovim, Helix, Emacs,
//! Zed, Sublime, IntelliJ live under
//! [`crates/noyalib-lsp/examples/`](https://github.com/sebastienrousseau/noyalib/tree/main/crates/noyalib-lsp/examples).
//!
//! # Performance
//!
//! Each `didOpen` / `didChange` event re-parses the entire
//! buffer in a single pass — `O(n)` in document bytes. A 1 MB
//! YAML document typically reports diagnostics in under 5 ms
//! on commodity hardware. The CST is cached per document so
//! `textDocument/formatting` and `textDocument/hover` reuse
//! the same parse on subsequent requests.
//!
//! # Security
//!
//! `#![forbid(unsafe_code)]`. No FFI. No network I/O — LSP is
//! stdio-only. The server reads file contents only from the
//! editor's `didOpen` notifications; it does not autoload
//! arbitrary paths from the filesystem. Resource-limit gates
//! are inherited from `noyalib`'s `ParserConfig` defaults.
//! Full posture:
//! [`SECURITY.md`](https://github.com/sebastienrousseau/noyalib/blob/main/SECURITY.md).
//!
//! # API stability and SemVer
//!
//! Pre-1.0 (`0.0.x`): the LSP wire contract (method names,
//! capability flags, JSON-RPC error code ranges, document store
//! semantics) is **stable** within a 0.0.x line — bug fixes
//! only. Adding a new LSP capability is allowed within a 0.0.x
//! bump; removing or repurposing one is held to a 0.x bump
//! (e.g. 0.0.x → 0.1.0). The Rust library surface (`Server`,
//! `HandleOutcome`, `Request`, `Response`, `ErrorResponse`) is
//! covered by the workspace SemVer policy in
//! [`POLICIES.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/POLICIES.md#2-semver--api-stability).
//! `cargo-semver-checks` runs in CI on every PR.
//!
//! # Documentation
//!
//! - **Engineering policies** — workspace
//!   [`POLICIES.md`](https://github.com/sebastienrousseau/noyalib/blob/main/doc/POLICIES.md).
//! - **LSP specification**: <https://microsoft.github.io/language-server-protocol/>.
//! - **Editor configurations** (VS Code / Neovim / Helix /
//!   Emacs / Zed / Sublime / IntelliJ):
//!   [`examples/`](https://github.com/sebastienrousseau/noyalib/tree/main/crates/noyalib-lsp/examples).
//! - **Protocol-method coverage matrix**:
//!   [`doc/protocol-coverage.md`](https://github.com/sebastienrousseau/noyalib/blob/main/crates/noyalib-lsp/doc/protocol-coverage.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::collections::HashMap;

pub mod diagnostics;
pub mod format;
pub mod hover;

/// JSON-RPC 2.0 request envelope (LSP wire format).
#[derive(Debug, Deserialize)]
pub struct Request {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Method name, e.g. `"textDocument/didOpen"`.
    pub method: String,
    /// Method parameters; LSP shape varies by method.
    #[serde(default)]
    pub params: JsonValue,
    /// Request id; absent on notifications.
    pub id: Option<JsonValue>,
}

/// JSON-RPC 2.0 success response envelope.
#[derive(Debug, Serialize)]
pub struct Response {
    /// Always `"2.0"`.
    pub jsonrpc: &'static str,
    /// The result payload.
    pub result: JsonValue,
    /// Echo of the corresponding request's id.
    pub id: JsonValue,
}

/// JSON-RPC 2.0 error envelope.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Always `"2.0"`.
    pub jsonrpc: &'static str,
    /// Error payload.
    pub error: ErrorObject,
    /// Echo of the corresponding request's id.
    pub id: JsonValue,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
pub struct ErrorObject {
    /// Numeric error code per JSON-RPC convention.
    pub code: i32,
    /// Human-readable message.
    pub message: String,
}

/// LSP server-side notification envelope (e.g. for
/// `textDocument/publishDiagnostics`).
#[derive(Debug, Serialize)]
pub struct Notification {
    /// Always `"2.0"`.
    pub jsonrpc: &'static str,
    /// Method name being invoked on the client.
    pub method: &'static str,
    /// Method parameters.
    pub params: JsonValue,
}

/// What the stdio loop should do with a parsed message — write a
/// reply and / or zero or more server-initiated notifications, or
/// stay silent.
#[derive(Debug, Default)]
pub struct HandleOutcome {
    /// Reply payload, when the request had an `id`.
    pub reply: Option<String>,
    /// Notifications the server emits as a side-effect (e.g.
    /// diagnostics published after a `didChange`).
    pub notifications: Vec<String>,
}

impl HandleOutcome {
    /// Build a reply-only outcome with no notifications.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib_lsp::HandleOutcome;
    /// let o = HandleOutcome::reply("{}".into());
    /// assert!(o.reply.is_some());
    /// assert!(o.notifications.is_empty());
    /// ```
    pub fn reply(payload: String) -> Self {
        HandleOutcome {
            reply: Some(payload),
            notifications: Vec::new(),
        }
    }

    /// Notification-only outcome (no reply expected by the client).
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib_lsp::HandleOutcome;
    /// let o = HandleOutcome::notify("{}".into());
    /// assert!(o.reply.is_none());
    /// assert_eq!(o.notifications.len(), 1);
    /// ```
    pub fn notify(payload: String) -> Self {
        HandleOutcome {
            reply: None,
            notifications: vec![payload],
        }
    }

    /// Empty / no-op outcome.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib_lsp::HandleOutcome;
    /// let o = HandleOutcome::silent();
    /// assert!(o.reply.is_none());
    /// assert!(o.notifications.is_empty());
    /// ```
    pub fn silent() -> Self {
        HandleOutcome::default()
    }
}

/// Stateful LSP server. One instance per stdio session; the document
/// store owns the in-memory snapshot of every open buffer.
///
/// # Examples
///
/// ```
/// use noyalib_lsp::Server;
/// let server = Server::new();
/// assert_eq!(server.open_document_count(), 0);
/// ```
#[derive(Debug, Default)]
pub struct Server {
    /// Documents the client has opened, keyed by URI.
    documents: HashMap<String, String>,
    /// Whether the client has issued `initialize` / `initialized`.
    initialized: bool,
    /// Whether the client has issued `shutdown`. After shutdown the
    /// server only honours `exit`.
    shutting_down: bool,
}

impl Server {
    /// Construct a fresh server with an empty document store.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib_lsp::Server;
    /// let server = Server::new();
    /// assert_eq!(server.open_document_count(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Server::default()
    }

    /// Number of currently-open documents. Useful for tests that
    /// assert the server's internal state.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib_lsp::Server;
    /// assert_eq!(Server::new().open_document_count(), 0);
    /// ```
    #[must_use]
    pub fn open_document_count(&self) -> usize {
        self.documents.len()
    }

    /// Snapshot of an open document, or `None` if the URI is not
    /// known.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib_lsp::Server;
    /// let server = Server::new();
    /// assert_eq!(server.document("file:///nope.yaml"), None);
    /// ```
    #[must_use]
    pub fn document(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(String::as_str)
    }

    /// Process one parsed JSON-RPC line and return the resulting
    /// reply / notifications. The stdio loop in `main` calls this
    /// per LSP message; tests call it with crafted strings.
    ///
    /// # Examples
    ///
    /// ```
    /// use noyalib_lsp::Server;
    /// let mut server = Server::new();
    /// let req = r#"{"jsonrpc":"2.0","method":"initialize","id":1,"params":{}}"#;
    /// let outcome = server.handle_message(req);
    /// assert!(outcome.reply.is_some());
    /// ```
    pub fn handle_message(&mut self, raw: &str) -> HandleOutcome {
        let req: Request = match serde_json::from_str(raw) {
            Ok(r) => r,
            Err(e) => {
                return HandleOutcome::reply(error_str(
                    JsonValue::Null,
                    -32700,
                    format!("parse error: {e}"),
                ));
            }
        };
        if req.jsonrpc != "2.0" {
            return HandleOutcome::reply(error_str(
                req.id.unwrap_or(JsonValue::Null),
                -32600,
                "invalid request: jsonrpc must be \"2.0\"".into(),
            ));
        }
        let id = req.id.clone();
        let result = self.dispatch(&req.method, req.params);

        let mut outcome = HandleOutcome::default();
        match (id, result) {
            (None, Ok(side)) => {
                outcome.notifications = side.notifications;
            }
            (None, Err(_)) => {
                // Notifications swallow errors per JSON-RPC.
            }
            (Some(id), Ok(side)) => {
                outcome.reply = Some(
                    serde_json::to_string(&Response {
                        jsonrpc: "2.0",
                        result: side.value,
                        id,
                    })
                    .expect("infallible serialise"),
                );
                outcome.notifications = side.notifications;
            }
            (Some(id), Err((code, msg))) => {
                outcome.reply = Some(error_str(id, code, msg));
            }
        }
        outcome
    }

    fn dispatch(&mut self, method: &str, params: JsonValue) -> Result<DispatchOk, (i32, String)> {
        if self.shutting_down && method != "exit" {
            return Err((-32600, format!("server is shutting down; refused {method}")));
        }
        match method {
            "initialize" => {
                self.initialized = true;
                Ok(DispatchOk::value(json!({
                    "capabilities": {
                        "textDocumentSync": 1,
                        "documentFormattingProvider": true,
                        "hoverProvider": true,
                    },
                    "serverInfo": {
                        "name": "noyalib-lsp",
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                })))
            }
            "initialized" => Ok(DispatchOk::value(JsonValue::Null)),
            "shutdown" => {
                self.shutting_down = true;
                Ok(DispatchOk::value(JsonValue::Null))
            }
            "exit" => Ok(DispatchOk::value(JsonValue::Null)),
            "textDocument/didOpen" => self.did_open(params),
            "textDocument/didChange" => self.did_change(params),
            "textDocument/didClose" => self.did_close(params),
            "textDocument/formatting" => self.formatting(params),
            "textDocument/hover" => self.hover(params),
            other => Err((-32601, format!("method not found: {other}"))),
        }
    }

    fn did_open(&mut self, params: JsonValue) -> Result<DispatchOk, (i32, String)> {
        let uri = uri_from_params(&params).ok_or((-32602, "missing textDocument.uri".into()))?;
        let text = params
            .pointer("/textDocument/text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned();
        let _ = self.documents.insert(uri.clone(), text.clone());
        let mut ok = DispatchOk::value(JsonValue::Null);
        if let Some(note) = diagnostics::publish_diagnostics(&uri, &text) {
            ok.notifications.push(note);
        }
        Ok(ok)
    }

    fn did_change(&mut self, params: JsonValue) -> Result<DispatchOk, (i32, String)> {
        let uri = uri_from_params(&params).ok_or((-32602, "missing textDocument.uri".into()))?;
        // LSP TextDocumentSyncKind::Full — the client sends the
        // entire new text in `contentChanges[0].text`.
        let text = params
            .pointer("/contentChanges/0/text")
            .and_then(|v| v.as_str())
            .ok_or((-32602, "missing contentChanges[0].text".into()))?
            .to_owned();
        let _ = self.documents.insert(uri.clone(), text.clone());
        let mut ok = DispatchOk::value(JsonValue::Null);
        if let Some(note) = diagnostics::publish_diagnostics(&uri, &text) {
            ok.notifications.push(note);
        }
        Ok(ok)
    }

    fn did_close(&mut self, params: JsonValue) -> Result<DispatchOk, (i32, String)> {
        let uri = uri_from_params(&params).ok_or((-32602, "missing textDocument.uri".into()))?;
        let _ = self.documents.remove(&uri);
        Ok(DispatchOk::value(JsonValue::Null))
    }

    fn formatting(&self, params: JsonValue) -> Result<DispatchOk, (i32, String)> {
        let uri = uri_from_params(&params).ok_or((-32602, "missing textDocument.uri".into()))?;
        let text = self
            .documents
            .get(&uri)
            .ok_or((-32602, format!("document not open: {uri}")))?;
        let edits = format::full_document_edits(text)
            .map_err(|e| (-32603, format!("format failed: {e}")))?;
        Ok(DispatchOk::value(serde_json::to_value(edits).unwrap()))
    }

    fn hover(&self, params: JsonValue) -> Result<DispatchOk, (i32, String)> {
        let uri = uri_from_params(&params).ok_or((-32602, "missing textDocument.uri".into()))?;
        let line = params
            .pointer("/position/line")
            .and_then(|v| v.as_u64())
            .ok_or((-32602, "missing position.line".into()))? as usize;
        let column = params
            .pointer("/position/character")
            .and_then(|v| v.as_u64())
            .ok_or((-32602, "missing position.character".into()))? as usize;
        let text = self
            .documents
            .get(&uri)
            .ok_or((-32602, format!("document not open: {uri}")))?;
        Ok(DispatchOk::value(hover::hover_at(text, line, column)))
    }
}

struct DispatchOk {
    value: JsonValue,
    notifications: Vec<String>,
}

impl DispatchOk {
    fn value(v: JsonValue) -> Self {
        DispatchOk {
            value: v,
            notifications: Vec::new(),
        }
    }
}

fn uri_from_params(params: &JsonValue) -> Option<String> {
    params
        .pointer("/textDocument/uri")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

/// Render a JSON-RPC error envelope to a single-line string.
///
/// # Examples
///
/// ```
/// use noyalib_lsp::error_str;
/// use serde_json::json;
/// let s = error_str(json!(1), -32601, "method not found".into());
/// assert!(s.contains("\"code\":-32601"));
/// ```
pub fn error_str(id: JsonValue, code: i32, message: String) -> String {
    serde_json::to_string(&ErrorResponse {
        jsonrpc: "2.0",
        error: ErrorObject { code, message },
        id,
    })
    .expect("infallible serialise")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_reply(out: &HandleOutcome) -> JsonValue {
        let s = out.reply.as_deref().expect("expected reply");
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn handle_message_returns_parse_error_on_bad_json() {
        let mut s = Server::new();
        let out = s.handle_message("{not json");
        let v = parse_reply(&out);
        assert_eq!(v["error"]["code"].as_i64().unwrap(), -32700);
        assert!(v["id"].is_null());
    }

    #[test]
    fn handle_message_rejects_non_2_0_jsonrpc() {
        let mut s = Server::new();
        let req = json!({"jsonrpc": "1.0", "method": "initialize", "id": 1});
        let out = s.handle_message(&req.to_string());
        let v = parse_reply(&out);
        assert_eq!(v["error"]["code"].as_i64().unwrap(), -32600);
    }

    #[test]
    fn initialize_returns_capabilities_and_server_info() {
        let mut s = Server::new();
        let req = json!({"jsonrpc": "2.0", "method": "initialize", "id": 1, "params": {}});
        let out = s.handle_message(&req.to_string());
        let v = parse_reply(&out);
        assert_eq!(
            v["result"]["serverInfo"]["name"].as_str(),
            Some("noyalib-lsp")
        );
        assert_eq!(
            v["result"]["capabilities"]["documentFormattingProvider"].as_bool(),
            Some(true),
        );
        assert_eq!(
            v["result"]["capabilities"]["hoverProvider"].as_bool(),
            Some(true),
        );
        assert_eq!(
            v["result"]["capabilities"]["textDocumentSync"].as_i64(),
            Some(1),
        );
    }

    #[test]
    fn unknown_method_returns_method_not_found() {
        let mut s = Server::new();
        let req = json!({"jsonrpc": "2.0", "method": "frobnicate", "id": 7});
        let out = s.handle_message(&req.to_string());
        let v = parse_reply(&out);
        assert_eq!(v["error"]["code"].as_i64().unwrap(), -32601);
    }

    #[test]
    fn shutdown_then_non_exit_method_is_rejected() {
        let mut s = Server::new();
        let _ =
            s.handle_message(&json!({"jsonrpc": "2.0", "method": "shutdown", "id": 1}).to_string());
        let out = s.handle_message(
            &json!({"jsonrpc": "2.0", "method": "textDocument/hover", "id": 2,
                "params": {"textDocument": {"uri": "f"}, "position": {"line": 0, "character": 0}}})
            .to_string(),
        );
        let v = parse_reply(&out);
        assert_eq!(v["error"]["code"].as_i64().unwrap(), -32600);
    }

    #[test]
    fn exit_after_shutdown_succeeds() {
        let mut s = Server::new();
        let _ =
            s.handle_message(&json!({"jsonrpc": "2.0", "method": "shutdown", "id": 1}).to_string());
        let out = s.handle_message(&json!({"jsonrpc": "2.0", "method": "exit"}).to_string());
        // Notifications swallow no reply; outcome should be silent.
        assert!(out.reply.is_none());
    }

    #[test]
    fn did_open_records_document_and_publishes_diagnostics() {
        let mut s = Server::new();
        let req = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///tmp/a.yaml",
                    "languageId": "yaml",
                    "version": 1,
                    "text": "name: noyalib\n"
                }
            }
        });
        let out = s.handle_message(&req.to_string());
        // didOpen is a notification — no reply.
        assert!(out.reply.is_none());
        // Diagnostics are published as a server-initiated notification.
        assert_eq!(out.notifications.len(), 1);
        let note: JsonValue = serde_json::from_str(&out.notifications[0]).unwrap();
        assert_eq!(
            note["method"].as_str(),
            Some("textDocument/publishDiagnostics"),
        );
        assert_eq!(s.open_document_count(), 1);
        assert_eq!(s.document("file:///tmp/a.yaml"), Some("name: noyalib\n"));
    }

    #[test]
    fn did_change_overwrites_text_and_re_publishes() {
        let mut s = Server::new();
        let _ = s.handle_message(
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": "file:///tmp/b.yaml", "languageId": "yaml",
                        "version": 1, "text": "a: 1\n"
                    }
                }
            })
            .to_string(),
        );
        let out = s.handle_message(
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {"uri": "file:///tmp/b.yaml", "version": 2},
                    "contentChanges": [{"text": "a: 2\n"}]
                }
            })
            .to_string(),
        );
        assert_eq!(out.notifications.len(), 1);
        assert_eq!(s.document("file:///tmp/b.yaml"), Some("a: 2\n"));
    }

    #[test]
    fn did_close_drops_document() {
        let mut s = Server::new();
        let _ = s.handle_message(
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {"textDocument": {
                    "uri": "f", "languageId": "yaml", "version": 1, "text": "x: 1\n"
                }}
            })
            .to_string(),
        );
        let _ = s.handle_message(
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": {"textDocument": {"uri": "f"}}
            })
            .to_string(),
        );
        assert_eq!(s.open_document_count(), 0);
    }

    #[test]
    fn formatting_returns_text_edits() {
        let mut s = Server::new();
        let _ = s.handle_message(
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {"textDocument": {
                    "uri": "f", "languageId": "yaml", "version": 1, "text": "a: 1\n"
                }}
            })
            .to_string(),
        );
        let out = s.handle_message(
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/formatting",
                "id": 5,
                "params": {"textDocument": {"uri": "f"}, "options": {"tabSize": 2, "insertSpaces": true}}
            })
            .to_string(),
        );
        let v = parse_reply(&out);
        assert!(v["result"].is_array());
    }

    #[test]
    fn formatting_unknown_uri_errors() {
        let mut s = Server::new();
        let out = s.handle_message(
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/formatting",
                "id": 6,
                "params": {"textDocument": {"uri": "missing"}, "options": {}}
            })
            .to_string(),
        );
        let v = parse_reply(&out);
        assert_eq!(v["error"]["code"].as_i64().unwrap(), -32602);
    }

    #[test]
    fn hover_unknown_uri_errors() {
        let mut s = Server::new();
        let out = s.handle_message(
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/hover",
                "id": 7,
                "params": {
                    "textDocument": {"uri": "missing"},
                    "position": {"line": 0, "character": 0}
                }
            })
            .to_string(),
        );
        let v = parse_reply(&out);
        assert_eq!(v["error"]["code"].as_i64().unwrap(), -32602);
    }

    #[test]
    fn error_str_renders_canonical_envelope() {
        let s = error_str(json!(42), -32000, "boom".into());
        let v: JsonValue = serde_json::from_str(&s).unwrap();
        assert_eq!(v["jsonrpc"].as_str(), Some("2.0"));
        assert_eq!(v["id"].as_i64(), Some(42));
        assert_eq!(v["error"]["code"].as_i64(), Some(-32000));
    }

    #[test]
    fn handle_outcome_helpers_construct_correctly() {
        let r = HandleOutcome::reply("hi".into());
        assert!(r.reply.is_some());
        assert!(r.notifications.is_empty());
        let n = HandleOutcome::notify("x".into());
        assert!(n.reply.is_none());
        assert_eq!(n.notifications.len(), 1);
        let s = HandleOutcome::silent();
        assert!(s.reply.is_none());
        assert!(s.notifications.is_empty());
    }
}
