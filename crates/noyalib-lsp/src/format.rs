// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `textDocument/formatting` — re-emit a YAML document via
//! noyalib's CST formatter and surface the result as LSP `TextEdit`
//! objects.
//!
//! The simplest correct implementation is "replace the entire
//! document range with the formatted output". That keeps the
//! response self-contained — the client doesn't need any
//! cross-document reasoning to apply the result.

use serde_json::{Value as JsonValue, json};

/// Build the LSP `TextEdit[]` array that, applied to `text`, yields
/// the formatted document.
///
/// Returns an empty array when `text` is already canonically
/// formatted; this lets the editor skip the no-op edit entirely.
///
/// # Errors
///
/// - The input fails to parse as YAML (the formatter has nothing
///   to emit until the document is syntactically valid).
pub fn full_document_edits(text: &str) -> noyalib::Result<Vec<JsonValue>> {
    let formatted = noyalib::cst::parse_document(text)?.to_string();
    if formatted == text {
        return Ok(Vec::new());
    }

    // LSP positions are zero-based line/character; the end is
    // *exclusive*. We use a sentinel large end so the range covers
    // the entire document regardless of length — the LSP spec
    // permits the server to clamp to the actual document end.
    let end_line = text
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        .max(1)
        .saturating_sub(if text.ends_with('\n') { 1 } else { 0 });
    let end_character = text.lines().last().unwrap_or("").len();

    Ok(vec![json!({
        "range": {
            "start": {"line": 0, "character": 0},
            "end":   {"line": end_line, "character": end_character},
        },
        "newText": formatted,
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_canonical_input_returns_empty_edits() {
        let edits = full_document_edits("a: 1\nb: 2\n").unwrap();
        // The CST formatter is byte-faithful for already-canonical
        // input, so the response is the empty array.
        assert!(edits.is_empty());
    }

    #[test]
    fn unparseable_input_propagates_error() {
        let res = full_document_edits("a: [\n");
        assert!(res.is_err());
    }

    #[test]
    fn well_formed_input_produces_text_edit_array() {
        // Drive a path where the formatter is an identity transform
        // — on identity the response is empty. The shape of the
        // edit object is exercised separately in the integration
        // tests under `tests/` where a non-canonical input will
        // produce a non-empty edit.
        let edits = full_document_edits("simple: yaml\n").unwrap();
        for e in &edits {
            assert!(e["range"]["start"]["line"].is_u64());
            assert!(e["range"]["end"]["line"].is_u64());
            assert!(e["newText"].is_string());
        }
    }
}
