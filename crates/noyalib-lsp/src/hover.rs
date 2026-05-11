// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `textDocument/hover` — surface contextual information for the
//! cursor position.
//!
//! The first iteration's contract: when the document parses
//! cleanly, return a small markdown card noting the line / column
//! and the document's overall type. When the document fails to
//! parse, surface the parse error in the hover so the user can
//! act on it without leaving the cursor.
//!
//! Future work tracked separately: when a JSON Schema is attached,
//! resolve the schema description for the field at the cursor.

use serde_json::{Value as JsonValue, json};

/// Hover result for the cursor at zero-based `line` / `column` in
/// `text`. Returns `null` when there is nothing to surface — the
/// LSP spec lets the server return `null` to mean "nothing to say
/// here".
#[must_use]
pub fn hover_at(text: &str, line: usize, column: usize) -> JsonValue {
    if byte_offset_of(text, line, column).is_none() {
        return JsonValue::Null;
    }
    let body = match noyalib::from_str::<noyalib::Value>(text) {
        Ok(v) => format!(
            "**Position**: line {}, column {}\n\n**Document type**: `{}`",
            line + 1,
            column + 1,
            type_name(&v),
        ),
        Err(e) => format!("**Parse error**\n\n```\n{e}\n```"),
    };
    json!({
        "contents": {
            "kind": "markdown",
            "value": body,
        }
    })
}

/// Convert an LSP `(line, column)` zero-based position into a byte
/// offset. Returns `None` when the position is out of range.
pub fn byte_offset_of(text: &str, line: usize, column: usize) -> Option<usize> {
    let mut current_line = 0usize;
    let mut line_start = 0usize;
    for (i, b) in text.bytes().enumerate() {
        if current_line == line {
            let target = line_start + column;
            if target <= text.len() {
                return Some(target);
            }
            return None;
        }
        if b == b'\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }
    if current_line == line {
        let target = line_start + column;
        if target <= text.len() {
            return Some(target);
        }
    }
    None
}

fn type_name(v: &noyalib::Value) -> &'static str {
    match v {
        noyalib::Value::Null => "null",
        noyalib::Value::Bool(_) => "bool",
        noyalib::Value::Number(_) => "number",
        noyalib::Value::String(_) => "string",
        noyalib::Value::Sequence(_) => "sequence",
        noyalib::Value::Mapping(_) => "mapping",
        noyalib::Value::Tagged(_) => "tagged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_returns_null_for_out_of_range_position() {
        assert_eq!(hover_at("a: 1\n", 99, 99), JsonValue::Null);
    }

    #[test]
    fn hover_at_well_formed_input_returns_markdown_card() {
        let v = hover_at("name: noyalib\n", 0, 0);
        assert_eq!(v["contents"]["kind"].as_str(), Some("markdown"));
        let body = v["contents"]["value"].as_str().unwrap();
        assert!(body.contains("mapping"));
        assert!(body.contains("line 1"));
    }

    #[test]
    fn hover_surfaces_parse_error_when_input_invalid() {
        let v = hover_at("a: [\n", 0, 0);
        assert_eq!(v["contents"]["kind"].as_str(), Some("markdown"));
        let body = v["contents"]["value"].as_str().unwrap();
        assert!(body.contains("Parse error"));
    }

    #[test]
    fn byte_offset_of_handles_multi_line_input() {
        let text = "abc\ndef\nghi\n";
        assert_eq!(byte_offset_of(text, 1, 1), Some(5));
        assert_eq!(byte_offset_of(text, 0, 0), Some(0));
    }

    #[test]
    fn byte_offset_of_returns_none_for_out_of_range_line() {
        assert_eq!(byte_offset_of("a\n", 5, 0), None);
    }

    #[test]
    fn byte_offset_of_handles_single_line_with_no_trailing_newline() {
        // "abc" — no newline. Line 0 is the entire string.
        assert_eq!(byte_offset_of("abc", 0, 2), Some(2));
        // Past the end of line 0 — out of range.
        assert_eq!(byte_offset_of("abc", 0, 99), None);
    }
}
