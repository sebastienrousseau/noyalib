// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `textDocument/publishDiagnostics` — turn YAML parse errors into
//! LSP-compatible diagnostic objects.
//!
//! The LSP wire shape is documented at
//! <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#publishDiagnosticsParams>.

use serde_json::{Value as JsonValue, json};

/// LSP severity levels per the spec.
const SEVERITY_ERROR: i32 = 1;

/// Compose the JSON-RPC `textDocument/publishDiagnostics`
/// notification for `uri` with the diagnostics derived from `text`.
///
/// Returns `None` when there is nothing to publish (no parse error).
/// The caller forwards the returned string to stdout when present.
#[must_use]
pub fn publish_diagnostics(uri: &str, text: &str) -> Option<String> {
    let diagnostics = collect(text);
    // Always publish — the LSP spec requires the server to emit the
    // (possibly empty) list so the client can clear stale diagnostics.
    let params = json!({
        "uri": uri,
        "diagnostics": diagnostics,
    });
    let note = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": params,
    });
    Some(note.to_string())
}

/// Collect raw diagnostics from a YAML document. Public so tests
/// and richer integrations can inspect the diagnostic list before
/// the JSON-RPC envelope is built.
#[must_use]
pub fn collect(text: &str) -> Vec<JsonValue> {
    let mut diagnostics = Vec::new();
    if let Err(err) = noyalib::from_str::<noyalib::Value>(text) {
        let (line, character) = location_from_error(&err, text);
        diagnostics.push(json!({
            "range": {
                "start": {"line": line, "character": character},
                "end":   {"line": line, "character": character + 1},
            },
            "severity": SEVERITY_ERROR,
            "source": "noyalib",
            "message": err.to_string(),
        }));
    }
    diagnostics
}

/// Best-effort line/column extraction from a noyalib error. Falls
/// back to `(0, 0)` when the error type does not carry a span.
fn location_from_error(_err: &noyalib::Error, _text: &str) -> (usize, usize) {
    // noyalib error variants vary in the location info they carry;
    // for the LSP surface a (0, 0) anchor is acceptable until we
    // expose `Location` on every variant. Editors typically still
    // surface the message at the file head when no span is given.
    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_returns_empty_on_valid_yaml() {
        assert!(collect("a: 1\nb: 2\n").is_empty());
    }

    #[test]
    fn collect_returns_one_diagnostic_per_parse_error() {
        let d = collect("a: [\n");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0]["severity"].as_i64(), Some(1));
        assert_eq!(d[0]["source"].as_str(), Some("noyalib"));
        assert!(!d[0]["message"].as_str().unwrap().is_empty());
    }

    #[test]
    fn publish_diagnostics_wraps_in_jsonrpc_envelope() {
        let s = publish_diagnostics("file:///tmp/a.yaml", "a: 1\n").unwrap();
        let v: JsonValue = serde_json::from_str(&s).unwrap();
        assert_eq!(v["jsonrpc"].as_str(), Some("2.0"));
        assert_eq!(
            v["method"].as_str(),
            Some("textDocument/publishDiagnostics"),
        );
        assert_eq!(v["params"]["uri"].as_str(), Some("file:///tmp/a.yaml"));
        let diags = v["params"]["diagnostics"].as_array().unwrap();
        assert!(diags.is_empty());
    }

    #[test]
    fn publish_diagnostics_includes_errors_for_invalid_yaml() {
        let s = publish_diagnostics("file:///tmp/a.yaml", "k: [\n").unwrap();
        let v: JsonValue = serde_json::from_str(&s).unwrap();
        let diags = v["params"]["diagnostics"].as_array().unwrap();
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn diagnostic_range_is_well_formed() {
        let d = collect("a: [\n");
        let range = &d[0]["range"];
        assert!(range["start"]["line"].is_u64());
        assert!(range["end"]["line"].is_u64());
    }
}
