//! Behavioural-parity fuzzer: the `compat-serde-yaml` shim against
//! the real archived `serde_yaml` 0.9.34, on arbitrary input.
//!
//! The 18-case contract suite pins the divergences a real migration
//! evaluation flagged; this target hunts for the ones nobody has
//! written down yet. Both sides parse into `serde_json::Value`;
//! a crash artefact means either a shim gap to close or a new
//! documented-divergence class to add below — every find is a
//! decision, never noise.
//!
//! Compared: accept/reject agreement and accepted-value equality.
//! Error wording and exact locations are the contract suite's job —
//! upstream's message set is unbounded and only the pinned classes
//! are promised.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../crates/noyalib/tests/properties/oracle_normalization.rs"]
mod oracle_normalization;

#[path = "../canonical_duplicates.rs"]
mod canonical_duplicates;

#[path = "../bom_classification.rs"]
mod bom_classification;

#[path = "../json_numbers.rs"]
mod json_numbers;

#[path = "../signed_radix.rs"]
mod signed_radix;

#[path = "../question_key.rs"]
mod question_key;

#[path = "../local_core_tags.rs"]
mod local_core_tags;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    if s.len() > 2048 {
        return;
    }

    let shim: Result<serde_json::Value, _> = noyalib::compat::serde_yaml::from_str(s);
    let upstream: Result<serde_json::Value, _> = serde_yaml::from_str(s);

    match (shim, upstream) {
        (Ok(a), Ok(b)) => {
            if !equivalent(&a, &b)
                && !anchor_name_outside_libyaml(s)
                && !local_core_tags::present(s)
                && !question_key::flow_entry_starts_with_indicator(s)
                && !yaml_1_1_line_break(s)
                && !canonical_duplicates::spelling_collision(
                    s,
                    &noyalib::ParserConfig::serde_yaml_compat(),
                )
                && !canonical_duplicates::tagged_key(s, &noyalib::ParserConfig::serde_yaml_compat())
                && !canonical_duplicates::aliased_duplicate(
                    s,
                    &noyalib::ParserConfig::serde_yaml_compat(),
                )
                && !oracle_agrees(
                    canonical_duplicates::spelling(s, &noyalib::ParserConfig::serde_yaml_compat()),
                    &a,
                )
                && !oracle_agrees(oracle_normalization::stream_bom_prefix(s), &a)
                && !oracle_agrees(oracle_normalization::collection_spelling(s), &a)
                && !oracle_agrees(oracle_normalization::unique_colliding_anchors(s), &a)
                && !oracle_agrees(oracle_normalization::terminated_spaced_block(s), &a)
                && !oracle_agrees(
                    canonical_duplicates::composed(
                        s,
                        &noyalib::ParserConfig::serde_yaml_compat(),
                        &|source| {
                            noyalib::compat::serde_yaml::from_str::<serde_json::Value>(source)
                                .is_ok_and(|value| equivalent(&a, &value))
                        },
                    ),
                    &a,
                )
            {
                panic!(
                    "shim != serde_yaml on accepted input (len {}):\n  shim     : {}\n  upstream : {}",
                    s.len(),
                    serde_json::to_string(&a).unwrap_or_default(),
                    serde_json::to_string(&b).unwrap_or_default(),
                );
            }
        }
        (Err(_), Err(_)) => {}
        (Ok(a), Err(e)) => {
            if !known_reject_divergence(s)
                && !oracle_agrees(
                    canonical_duplicates::spelling(s, &noyalib::ParserConfig::serde_yaml_compat()),
                    &a,
                )
                && !oracle_agrees(oracle_normalization::stream_bom_prefix(s), &a)
                && !oracle_agrees(oracle_normalization::collection_spelling(s), &a)
                && !oracle_agrees(
                    oracle_normalization::collection_spelling(s)
                        .and_then(|n| oracle_normalization::unique_colliding_anchors(&n)),
                    &a,
                )
            {
                panic!(
                    "shim accepted what serde_yaml rejects (len {}):\n  shim     : {}\n  upstream : {e}",
                    s.len(),
                    serde_json::to_string(&a).unwrap_or_default(),
                );
            }
        }
        (Err(e), Ok(b)) => {
            // YAML 1.2.2 §7.3.2 and §7.3.1 require the continuation
            // indentation pinned by yaml-test-suite QB6E. Libyaml also
            // accepts the under-indented single-quoted equivalent.
            if !known_reject_divergence(s)
                && !oracle_normalization::quoted_indent_rejection(&e.to_string())
                && !oracle_normalization::missing_comment_separator_rejection(&e.to_string())
                && !bom_classification::unquoted_content_bom_rejection(s, e.inner())
                && !flow_indent_rejection(&e.to_string())
                && !leading_space_line_rejection(&e.to_string())
                && !key_collision_rejection(&e.to_string())
                && !bom_column_shift(s)
            {
                panic!(
                    "shim rejected what serde_yaml accepts (len {}):\n  shim     : {e}\n  upstream : {}",
                    s.len(),
                    serde_json::to_string(&b).unwrap_or_default(),
                );
            }
        }
    }
});

// These repairs preserve the JSON projection and still require agreement
// on the complete value. They do not exclude documents from comparison.
fn oracle_agrees(normalized: Option<String>, actual: &serde_json::Value) -> bool {
    normalized
        .filter(|source| {
            noyalib::compat::serde_yaml::from_str::<serde_json::Value>(source)
                .is_ok_and(|value| equivalent(actual, &value))
        })
        .and_then(|source| serde_yaml::from_str::<serde_json::Value>(&source).ok())
        .is_some_and(|expected| equivalent(actual, &expected))
}

/// Known libyaml quirk (serde_yaml_ng and serde_yaml): it keeps the
/// YAML 1.1 line breaks, so NEL (U+0085), LS (U+2028) and PS (U+2029)
/// end a line, and `\u{85}` reads as null and `a\u{85}` as "a". YAML 1.2
/// §5.4 makes only LF and CR line breaks; those three are ordinary
/// printable characters, so noyalib keeps them in the scalar. Such
/// inputs are dropped rather than modelled. noyalib's reading is pinned
/// in tests/regressions/competitor_bugs.rs.
fn yaml_1_1_line_break(s: &str) -> bool {
    s.contains(['\u{85}', '\u{2028}', '\u{2029}'])
}

/// libyaml allows only `[A-Za-z0-9_-]` in an anchor name and ends the
/// name at anything else, where YAML 1.2.2 §6.9.2 allows any ns-char
/// but the flow indicators: `&f::!+` anchors an empty node named
/// `f::!+` (noyalib) or the scalar "::!+" under the name `f`
/// (libyaml). Such documents differ in value, not only in acceptance.
fn anchor_name_outside_libyaml(s: &str) -> bool {
    s.split('&').skip(1).any(|rest| {
        rest.chars()
            .take_while(|c| !c.is_whitespace() && !",[]{}".contains(*c))
            .any(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
    })
}

/// Value equivalence with numeric tolerance: `1` and `1.0` disagree
/// on integer-ness across resolvers without disagreeing on the data.
fn equivalent(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value as V;
    match (a, b) {
        (V::Number(x), V::Number(y)) => json_numbers::equal(x, y),
        (V::Number(n), V::String(s)) | (V::String(s), V::Number(n))
            if signed_radix::equal(s, n) || signed_radix::legacy_binary(s, n) =>
        {
            true
        }
        (V::Array(x), V::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| equivalent(p, q))
        }
        (V::Object(x), V::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| question_key::lookup(y, k).is_some_and(|w| equivalent(v, w)))
        }
        _ => a == b,
    }
}

/// Inputs where the shim and upstream legitimately disagree about
/// accept/reject. Every entry is a documented, deliberate divergence
/// — extend it only with a comment saying which side is right and
/// why the difference stays.
fn known_reject_divergence(s: &str) -> bool {
    // The shim (noyalib) is a spec-complete YAML parser; libyaml
    // rejects a number of spec-valid constructs, and vice versa
    // accepts some spec-invalid ones. Documents using tags,
    // directives, explicit keys, or anchors/aliases sit squarely in
    // that implementation-gap territory — libyaml's anchor-name
    // charset, for instance, is alphanumerics where the spec allows
    // nearly any ns-char (`&\L` is a legal anchor upstream rejects).
    // These are excluded from the accept/reject comparison; their
    // *value* comparison above still runs whenever both sides
    // accept.
    if s.contains('!') || s.contains('%') || s.contains('?') || s.contains('&') || s.contains('*') {
        return true;
    }
    // Empty implicit keys (`: v`) are spec-valid — the official
    // yaml-test-suite blesses them and noyalib passes it 406/406 —
    // but libyaml wants a key before the `:` in block context
    // ("did not find expected key").
    if s.split(['\r', '\n']).any(|l| {
        // `- : v` is an empty key in a compact mapping.
        let mut l = l.trim_start();
        while let Some(rest) = l.strip_prefix("- ") {
            l = rest.trim_start();
        }
        l.starts_with(':')
    }) {
        return true;
    }
    // libyaml rejects a tab that starts a line ("cannot start any
    // token"). The spec reads it as separation white space wherever it
    // is not indentation: a line of only white space or a comment is an
    // l-comment line (§6.6), so `\t` alone is a valid empty stream, and
    // a tab before a node is s-separate-in-line, so `\t$0` is the
    // scalar "$0" (yaml-test-suite 6CA3 indents a flow node so).
    // noyalib follows the spec and rejects tab indentation itself. Recognize
    // LF, CRLF and lone CR here, matching the production scanner.
    if s.split(['\r', '\n'])
        .any(|l| l.trim_start_matches(' ').starts_with('\t'))
    {
        return true;
    }
    // A tab after a block sequence's `-` is s-separate-in-line (§6.2);
    // libyaml rejects it the same way.
    if s.split(['\r', '\n'])
        .any(|l| l.trim_start_matches(' ').starts_with("-\t"))
    {
        return true;
    }
    // A flow entry that opens with `?` or `:` (yaml-test-suite 652Z,
    // CFD4); see `question_key`.
    if question_key::flow_entry_starts_with_indicator(s) {
        return true;
    }
    // NEL, LS and PS end a line for libyaml (YAML 1.1); see
    // `yaml_1_1_line_break`.
    if yaml_1_1_line_break(s) {
        return true;
    }
    // A flow mapping key may span lines (production 145, yaml-test-suite
    // NJ66); libyaml applies its single-line implicit-key rule.
    if multiline_flow_key(s) {
        return true;
    }
    // In flow context a `:` may follow its key across white space or a
    // line break (yaml-test-suite 4MUZ, NJ66) and may sit directly before
    // `:` or a flow indicator (`{m :}`, `{m\nl::}`). libyaml rejects
    // those ("found unexpected ':'"); the spec reads them as written.
    if s.contains(['{', '[']) && flow_colon_libyaml_rejects(s) {
        return true;
    }
    // `-` directly before a flow indicator cannot start a plain scalar:
    // ns-plain-first needs a ns-plain-safe character after it, so `{-}`
    // is an error. libyaml reads the scalar "-".
    if lone_dash_before_flow_indicator(s) {
        return true;
    }
    // A stream may open with a document suffix before its document
    // (l-yaml-stream: `l-document-suffix+ l-document-prefix*
    // l-any-document?`), so `...\nx` is the document "x". libyaml finds
    // no node content after the leading `...`.
    if leading_document_end(s) {
        return true;
    }
    // libyaml misses a `---` or `...` marker straight after the stream
    // BOM and reads it as plain text (`\u{feff}---\n>f` loads "--- >f");
    // the BOM is transparent to the document markers (section 9.1.1).
    if s.strip_prefix('\u{feff}')
        .is_some_and(|rest| rest.starts_with("---") || rest.starts_with("..."))
    {
        return true;
    }
    // A `#` straight after a block scalar header (`|#`, `>-#`) is not a
    // comment: §6.6 wants white space before it, and yaml-test-suite
    // X4QW is the error case. libyaml reads it as one.
    if block_header_touches_comment(s) {
        return true;
    }
    // A root-level block scalar may hold content at column 0: its
    // parent indentation is -1 (yaml-test-suite FP8R, "Zero indented
    // block scalar"). libyaml ends the scalar at the first column-0
    // line and reads what follows as more of the stream.
    root_block_scalar_with_column_0_content(s)
}

/// A `|` or `>` with only indicators (`+`, `-`, digits) before a `#`.
fn block_header_touches_comment(s: &str) -> bool {
    s.match_indices(['|', '>']).any(|(i, _)| {
        s[i + 1..]
            .trim_start_matches(|c: char| c == '+' || c == '-' || c.is_ascii_digit())
            .starts_with('#')
    })
}

/// The document is a root block scalar (`|` or `>`, after an optional
/// `---`) with a non-blank line at column 0 below its header.
fn root_block_scalar_with_column_0_content(s: &str) -> bool {
    // LF, CRLF and lone CR, matching the production scanner.
    let mut lines = s
        .split(['\r', '\n'])
        .skip_while(|l| l.trim().is_empty() || l.trim_start().starts_with('#'));
    let Some(mut header) = lines.next() else {
        return false;
    };
    // The header may follow a bare `---` line, blank and comment lines.
    if header.trim_end() == "---" {
        let Some(next) = lines.find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        else {
            return false;
        };
        header = next;
    }
    let header = header.strip_prefix("---").unwrap_or(header).trim_start();
    header.starts_with(['|', '>']) && lines.any(|l| l.starts_with(|c: char| !c.is_whitespace()))
}

/// See `known_reject_divergence`.
fn flow_colon_libyaml_rejects(s: &str) -> bool {
    let b = s.as_bytes();
    b.iter().enumerate().any(|(i, &c)| {
        c == b':'
            && (i > 0 && matches!(b[i - 1], b' ' | b'\t' | b'\n' | b'\r')
                || b.get(i + 1)
                    .is_some_and(|n| matches!(n, b':' | b',' | b']' | b'}')))
    })
}

/// See `known_reject_divergence`.
fn lone_dash_before_flow_indicator(s: &str) -> bool {
    let b = s.as_bytes();
    b.windows(2).enumerate().any(|(i, w)| {
        w[0] == b'-'
            && matches!(w[1], b',' | b']' | b'}')
            && (i == 0 || matches!(b[i - 1], b' ' | b'\t' | b'\n' | b'\r' | b'[' | b'{' | b','))
    })
}

/// A flow continuation line must be indented past the parent block
/// (yaml-test-suite 9C9N is the error case); libyaml accepts `- {\na}`.
fn flow_indent_rejection(message: &str) -> bool {
    message.starts_with("flow content must be indented more than the surrounding block")
}

/// See `known_reject_divergence`: a `:` value indicator in a flow
/// collection whose key text, since the entry opened, crosses a break.
/// Quoted scalars are skipped whole, so their `,` and `:` do not count.
fn multiline_flow_key(s: &str) -> bool {
    if !s.contains(['{', '[']) {
        return false;
    }
    let b = s.as_bytes();
    let (mut entry, mut quote, mut after_quote) = (0, None, false);
    for (i, &c) in b.iter().enumerate() {
        if let Some(q) = quote {
            // `''` inside single quotes and `\"` inside double quotes escape.
            let escaped = q == b'"' && i > 0 && b[i - 1] == b'\\';
            if c == q && !escaped {
                quote = None;
                after_quote = true;
            }
            continue;
        }
        match c {
            // A quote opens a scalar only at a node's start, not inside
            // a plain scalar (`\\"`).
            b'\'' | b'"'
                if b[..i]
                    .iter()
                    .rev()
                    .find(|p| !p.is_ascii_whitespace())
                    .is_none_or(|p| matches!(p, b'{' | b'[' | b',' | b':' | b'?')) =>
            {
                quote = Some(c);
            }
            b'{' | b'[' | b',' => entry = i + 1,
            b':' if after_quote || b.get(i + 1).is_none_or(|n| n.is_ascii_whitespace()) => {
                let key = &b[entry..i];
                let first = key.iter().position(|p| !p.is_ascii_whitespace());
                if first.is_some_and(|f| key[f..].iter().any(|p| matches!(p, b'\n' | b'\r'))) {
                    return true;
                }
            }
            _ => {}
        }
        after_quote = after_quote && c.is_ascii_whitespace();
    }
    false
}

/// YAML 1.2.2 section 8.1.1.1 (yaml-test-suite 5LLU): leading empty lines
/// of a block scalar may not hold more spaces than its content
/// indentation. libyaml accepts them.
fn leading_space_line_rejection(message: &str) -> bool {
    message.starts_with("a leading all-space line must not have too many spaces")
}

/// See `known_reject_divergence`.
fn leading_document_end(s: &str) -> bool {
    s.trim_start_matches('\u{feff}')
        .split(['\r', '\n'])
        .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .and_then(|l| l.strip_prefix("..."))
        .is_some_and(|rest| {
            let rest = rest.trim_start();
            rest.is_empty() || rest.starts_with('#')
        })
}

/// noyalib refuses distinct typed keys that stringify alike
/// (`{.inf, inf}`) under every duplicate policy (ADR-0019); the archive
/// keeps both written spellings as JSON keys.
fn key_collision_rejection(message: &str) -> bool {
    message.contains("distinct mapping keys collide after string conversion")
}

/// libyaml counts a stream BOM as a column, so the first line's nodes
/// sit one column right of where the spec puts them (section 9.1.1,
/// `\u{feff}I: _\n /:`). When the archive refuses the same text without
/// that BOM, the disagreement is the shift, not the document.
fn bom_column_shift(s: &str) -> bool {
    s.strip_prefix('\u{feff}')
        .is_some_and(|rest| serde_yaml::from_str::<serde_json::Value>(rest).is_err())
}
