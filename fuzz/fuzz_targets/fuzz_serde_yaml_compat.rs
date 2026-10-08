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

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    if s.len() > 2048 {
        return;
    }

    let shim: Result<serde_json::Value, _> =
        noyalib::compat::serde_yaml::from_str(s);
    let upstream: Result<serde_json::Value, _> = serde_yaml::from_str(s);

    match (shim, upstream) {
        (Ok(a), Ok(b)) => {
            if !equivalent(&a, &b) && !anchor_name_outside_libyaml(s) {
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
            if !known_reject_divergence(s) {
                panic!(
                    "shim accepted what serde_yaml rejects (len {}):\n  shim     : {}\n  upstream : {e}",
                    s.len(),
                    serde_json::to_string(&a).unwrap_or_default(),
                );
            }
        }
        (Err(e), Ok(b)) => {
            if !known_reject_divergence(s) {
                panic!(
                    "shim rejected what serde_yaml accepts (len {}):\n  shim     : {e}\n  upstream : {}",
                    s.len(),
                    serde_json::to_string(&b).unwrap_or_default(),
                );
            }
        }
    }
});

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
        (V::Number(x), V::Number(y)) => x.as_f64() == y.as_f64(),
        (V::Array(x), V::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| equivalent(p, q))
        }
        (V::Object(x), V::Object(y)) => {
            x.len() == y.len()
                && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| equivalent(v, w)))
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
    if s.contains('!') || s.contains('%') || s.contains('?') || s.contains('&') || s.contains('*')
    {
        return true;
    }
    // Empty implicit keys (`: v`) are spec-valid — the official
    // yaml-test-suite blesses them and noyalib passes it 406/406 —
    // but libyaml wants a key before the `:` in block context
    // ("did not find expected key").
    if s.lines().any(|l| l.trim_start().starts_with(':')) {
        return true;
    }
    // libyaml rejects a tab that starts a line ("cannot start any
    // token"). The spec reads it as separation white space wherever it
    // is not indentation: a line of only white space or a comment is an
    // l-comment line (§6.6), so `\t` alone is a valid empty stream, and
    // a tab before a node is s-separate-in-line, so `\t$0` is the
    // scalar "$0" (yaml-test-suite 6CA3 indents a flow node so).
    // noyalib follows the spec and rejects tab indentation itself.
    if s.lines().any(|l| l.trim_start_matches(' ').starts_with('\t')) {
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
    let mut lines = s.lines().skip_while(|l| l.trim().is_empty());
    let Some(header) = lines.next() else {
        return false;
    };
    let header = header.strip_prefix("---").unwrap_or(header).trim_start();
    header.starts_with(['|', '>'])
        && lines.any(|l| l.starts_with(|c: char| !c.is_whitespace()))
}
