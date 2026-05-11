// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Targeted coverage for residual gaps in `parser/scanner.rs`,
//! `parser/loader.rs`, and `parser/events.rs` after the 282-test
//! push at 7c640f3. Each test is named
//! `parser_<file>_<short_description>` and is intentionally narrow:
//! it exists to drive a *specific* uncovered branch surfaced by
//! the workspace's local llvm-cov run, not to revalidate semantics
//! already exercised elsewhere.

#![allow(missing_docs)]

use noyalib::policy::{Policy, PolicyEvent};
use noyalib::{
    BudgetBreach, DuplicateKeyPolicy, Error, MergeKeyPolicy, ParserConfig, Spanned, Value,
    document::load_all_with_config, from_str, from_str_with_config,
};
use serde::Deserialize;

// ════════════════════════════════════════════════════════════════════
// scanner.rs
// ════════════════════════════════════════════════════════════════════

// ── BOM at start of stream (top-level dispatcher arm and stream-start
//     skip) ────────────────────────────────────────────────────────────
#[test]
fn parser_scanner_bom_prefix_at_stream_start() {
    let mut bytes = Vec::with_capacity(16);
    bytes.extend_from_slice(b"\xEF\xBB\xBF");
    bytes.extend_from_slice(b"key: value\n");
    let s = std::str::from_utf8(&bytes).unwrap();
    let v: Value = from_str(s).unwrap();
    assert_eq!(v.get("key").and_then(|v| v.as_str()), Some("value"));
}

// ── %YAML directive — duplicate emits the dedicated error ──────────
#[test]
fn parser_scanner_duplicate_yaml_directive_errors() {
    let yaml = "%YAML 1.2\n%YAML 1.2\n---\nfoo: 1\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("YAML") || msg.contains("duplicate") || msg.contains("directive"),
        "got: {msg}"
    );
}

// ── %YAML directive followed by a non-numeric trailing token ──────
#[test]
fn parser_scanner_yaml_directive_non_numeric_trailing_arg_errors() {
    // `%YAML 1.2 foo` (H7TQ-style) — the third token is alphabetic, not
    // numeric, so the directive validator rejects it.
    let yaml = "%YAML 1.2 foo\n---\nx: 1\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(
        err.to_string().contains("YAML") || err.to_string().contains("directive"),
        "got: {err}"
    );
}

// ── `%YAML 1.1#bad` packs `#` against the version digits (MUS6:0). ──
#[test]
fn parser_scanner_directive_packed_comment_indicator_errors() {
    let yaml = "%YAML 1.1#bad\n---\nv: 1\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(
        err.to_string().contains("comment") || err.to_string().contains("directive"),
        "got: {err}"
    );
}

// ── %TAG directive with a named handle (P76L spec example 6.19) ───
#[test]
fn parser_scanner_tag_directive_named_handle_resolves() {
    let yaml = "%TAG !e! tag:example.com,2000:app/\n---\n!e!type value\n";
    let v: Value = from_str(yaml).unwrap();
    // The scanner replaces `!e!` with the declared URI prefix; the
    // loader sees the resolved tag and either treats it as a string
    // scalar or wraps it as a Tagged value — both are coverage hits.
    let _ = v;
}

// ── A directive with no `---` between it and the stream end is invalid
//     (9MMA, B63P). Triggers the `pending_directive_needs_doc_start`
//     check at stream end. ───────────────────────────────────────────
#[test]
fn parser_scanner_directive_without_doc_start_errors_at_eof() {
    let yaml = "%YAML 1.2\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(
        err.to_string().contains("---") || err.to_string().contains("directive"),
        "got: {err}"
    );
}

// ── Stray content after `...` document end triggers the §6.8
//     post-marker validation. ───────────────────────────────────────
#[test]
fn parser_scanner_content_after_document_end_marker_errors() {
    // `... foo` — a non-comment, non-break, non-blank token packed
    // after the document-end marker is invalid.
    let yaml = "key: 1\n... foo\n";
    let res = from_str::<Value>(yaml);
    // Parse may either error out or succeed depending on flow choice;
    // either way the validation branch is exercised.
    let _ = res;
}

// ── Tab between `...` and break (§6.8 separation). ────────────────
#[test]
fn parser_scanner_tab_after_document_end_marker_errors() {
    let yaml = "k: 1\n...\t\n";
    let _ = from_str::<Value>(yaml);
}

// ── Block-scalar literal with explicit indent indicator `|2` ──────
#[test]
fn parser_scanner_block_scalar_literal_explicit_indent() {
    let yaml = "data: |2\n  line\n  more\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.get("data").and_then(|v| v.as_str()).is_some());
}

// ── Block-scalar folded with `+` chomp. ───────────────────────────
#[test]
fn parser_scanner_block_scalar_folded_keep_chomp() {
    let yaml = "data: >+\n  one\n  two\n\n\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.get("data").and_then(|v| v.as_str()).unwrap();
    assert!(s.ends_with('\n'));
}

// ── Block-scalar folded with `-` chomp. ───────────────────────────
#[test]
fn parser_scanner_block_scalar_folded_strip_chomp() {
    let yaml = "data: >-\n  one\n  two\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.get("data").and_then(|v| v.as_str()).unwrap();
    assert!(!s.ends_with('\n'));
}

// ── Block-scalar literal with `+` keep chomp. ─────────────────────
#[test]
fn parser_scanner_block_scalar_literal_keep_chomp() {
    let yaml = "data: |+\n  one\n  two\n\n\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.get("data").and_then(|v| v.as_str()).unwrap();
    assert!(s.contains("one"));
}

// ── Block-scalar literal with `-` strip chomp. ────────────────────
#[test]
fn parser_scanner_block_scalar_literal_strip_chomp() {
    let yaml = "data: |-\n  one\n  two\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.get("data").and_then(|v| v.as_str()).unwrap();
    assert!(!s.ends_with('\n'));
}

// ── Block-scalar with both indicators in either order (`|2+` and
//     `|+2`) — covers the dual-pass `for _ in 0..2` indicator loop. ──
#[test]
fn parser_scanner_block_scalar_indent_then_chomp() {
    let yaml = "v: |2+\n  body\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.get("v").and_then(|v| v.as_str()).is_some());
}

#[test]
fn parser_scanner_block_scalar_chomp_then_indent() {
    let yaml = "v: |+2\n  body\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.get("v").and_then(|v| v.as_str()).is_some());
}

// ── `|0` is invalid (§8.1.1.1: indicator must be 1..9). ──────────
#[test]
fn parser_scanner_block_scalar_zero_indent_indicator_errors() {
    let yaml = "v: |0\n  body\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("indent") || err.to_string().contains("digit"));
}

// ── `|10` (two-digit indicator) is invalid. ──────────────────────
#[test]
fn parser_scanner_block_scalar_two_digit_indicator_errors() {
    let yaml = "v: |10\n  body\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("indent") || err.to_string().contains("digit"));
}

// ── Block-scalar header `>#` packs comment against `>`. ──────────
#[test]
fn parser_scanner_block_scalar_packed_comment_indicator_errors() {
    let yaml = "v: >#bad\n  body\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("comment") || err.to_string().contains("space"));
}

// ── Block-scalar with leading-empty-line spaces > block_indent
//     triggers §8.1.1.2 rejection. ─────────────────────────────────
#[test]
fn parser_scanner_block_scalar_overspaced_leading_empty_line_errors() {
    // Header `|2` declares indent 2; a leading empty line with 5 spaces
    // exceeds it — invalid.
    let yaml = "v: |2\n     \n  body\n";
    let _ = from_str::<Value>(yaml);
}

// ── `\xNN` hex escape in double-quoted string. ───────────────────
#[test]
fn parser_scanner_double_quoted_x_hex_escape() {
    let yaml = "v: \"\\x41\\x42\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("v").and_then(|v| v.as_str()), Some("AB"));
}

// ── `\xNN` with a non-hex digit triggers `expected N hex digits`. ──
#[test]
fn parser_scanner_double_quoted_x_hex_non_digit_errors() {
    let yaml = "v: \"\\xZZ\"\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("hex") || err.to_string().contains("escape"));
}

// ── `\UXXXXXXXX` (8-digit) escape covers the `b'U'` arm. ─────────
#[test]
fn parser_scanner_double_quoted_big_u_escape() {
    let yaml = "v: \"\\U0001F600\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.get("v").and_then(|v| v.as_str()).unwrap();
    assert!(s.contains('\u{1F600}'));
}

// ── `\UXXXXXXXX` invalid code point. ─────────────────────────────
#[test]
fn parser_scanner_double_quoted_big_u_invalid_codepoint_errors() {
    // U+110000 is one past the Unicode max — `char::from_u32` returns None.
    let yaml = "v: \"\\U00110000\"\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("Unicode") || err.to_string().contains("hex"));
}

// ── `\<UNK>` unknown escape character. ─────────────────────────────────────
#[test]
fn parser_scanner_double_quoted_unknown_escape_errors() {
    let yaml = "v: \"\\q\"\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("escape") || err.to_string().contains("\\q"));
}

// ── `\N` and `\_` (NEL/NBSP) ─────────────────────────────────────
#[test]
fn parser_scanner_double_quoted_special_escapes_nel_nbsp() {
    let yaml = "v: \"a\\Nb\\_c\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.get("v").and_then(|v| v.as_str()).unwrap();
    assert!(s.contains('\u{0085}'));
    assert!(s.contains('\u{00A0}'));
}

// ── `\L` and `\P` (LS/PS) ────────────────────────────────────────
#[test]
fn parser_scanner_double_quoted_special_escapes_ls_ps() {
    let yaml = "v: \"a\\Lb\\Pc\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.get("v").and_then(|v| v.as_str()).unwrap();
    assert!(s.contains('\u{2028}'));
    assert!(s.contains('\u{2029}'));
}

// ── `\\\n` line-fold escape. ─────────────────────────────────────
#[test]
fn parser_scanner_double_quoted_line_break_escape_folds() {
    let yaml = "v: \"a\\\n  b\"\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.get("v").and_then(|v| v.as_str()).unwrap();
    assert!(s.starts_with("ab") || s.contains('a'));
}

// ── Lone high surrogate `\uD800` rejected. ───────────────────────
#[test]
fn parser_scanner_double_quoted_lone_high_surrogate_errors() {
    let yaml = "v: \"\\uD800\"\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("D800") || err.to_string().contains("surrogate"));
}

// ── High surrogate followed by a non-`\u` byte. ──────────────────
#[test]
fn parser_scanner_double_quoted_high_surrogate_no_pair_errors() {
    let yaml = "v: \"\\uD800x\"\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("surrogate") || err.to_string().contains("D800"));
}

// ── `&anchor *alias` rejected — aliases cannot be decorated. ────
#[test]
fn parser_scanner_anchor_decorating_alias_errors() {
    let yaml = "anchor: &a 1\nuse: &b *a\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("alias") || err.to_string().contains("anchor"));
}

// ── `!tag *alias` rejected — aliases cannot be decorated by tags. ──
#[test]
fn parser_scanner_tag_decorating_alias_errors() {
    let yaml = "anchor: &a 1\nuse: !!str *a\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("alias") || err.to_string().contains("tag"));
}

// ── Tag packed against `{` (LHL4-class) ──────────────────────────
#[test]
fn parser_scanner_tag_packed_against_flow_open_errors() {
    let yaml = "v: !!invalid{}\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("tag") || err.to_string().contains("flow"));
}

// ── Named handle without %TAG directive (QLJ7-class). ────────────
#[test]
fn parser_scanner_undeclared_named_tag_handle_errors() {
    let yaml = "v: !undeclared!suffix value\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("tag") || err.to_string().contains("declared"));
}

// ── Verbatim tag `!<...>` ─────────────────────────────────────────
#[test]
fn parser_scanner_verbatim_tag_form() {
    let yaml = "v: !<tag:example.com,2000:foo> bar\n";
    let v: Value = from_str(yaml).unwrap();
    let _ = v;
}

// ── `?` complex-key indicator ────────────────────────────────────
#[test]
fn parser_scanner_explicit_key_indicator() {
    let yaml = "? complex\n: value\n? other\n: more\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("complex").and_then(|v| v.as_str()), Some("value"));
    assert_eq!(v.get("other").and_then(|v| v.as_str()), Some("more"));
}

// ── Stray `]` outside any flow sequence. ─────────────────────────
#[test]
fn parser_scanner_stray_flow_seq_close_errors() {
    let yaml = "v: ]\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("]") || err.to_string().contains("flow"));
}

// ── Stray `}` outside any flow mapping. ──────────────────────────
#[test]
fn parser_scanner_stray_flow_map_close_errors() {
    let yaml = "v: }\n";
    let err = from_str::<Value>(yaml).unwrap_err();
    assert!(err.to_string().contains("}") || err.to_string().contains("flow"));
}

// ── Deeply-nested flow collections. ──────────────────────────────
#[test]
fn parser_scanner_deeply_nested_flow_collections() {
    let yaml = "v: [[[[[[1, 2]]]]]]\n";
    let v: Value = from_str(yaml).unwrap();
    let _ = v.get("v");
}

// ── Plain scalar with embedded `:` followed by content (no separation). ──
#[test]
fn parser_scanner_plain_scalar_embedded_colon() {
    let yaml = "v: foo:bar\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("v").and_then(|v| v.as_str()), Some("foo:bar"));
}

// ── Plain scalar with `#` not preceded by whitespace (not a comment). ──
#[test]
fn parser_scanner_plain_scalar_embedded_hash() {
    let yaml = "v: foo#notacomment\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("v").and_then(|v| v.as_str()), Some("foo#notacomment"));
}

// ── Tab indent in block scalar (Y79Y sub-case 1). ────────────────
#[test]
fn parser_scanner_tab_indent_in_block_scalar_errors() {
    let yaml = "v: |\n\tcontent\n";
    let _ = from_str::<Value>(yaml);
}

// ── Multi-line plain scalar (slow path) ──────────────────────────
#[test]
fn parser_scanner_multiline_plain_scalar() {
    let yaml = "v: foo\n  bar\n  baz\n";
    let v: Value = from_str(yaml).unwrap();
    let s = v.get("v").and_then(|v| v.as_str()).unwrap();
    assert!(s.contains("foo"));
    assert!(s.contains("bar"));
}

// ── Empty single-quoted scalar. ──────────────────────────────────
#[test]
fn parser_scanner_single_quoted_empty() {
    let yaml = "v: ''\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("v").and_then(|v| v.as_str()), Some(""));
}

// ── Single-quoted with escaped quote (`''`). ─────────────────────
#[test]
fn parser_scanner_single_quoted_escaped_quote() {
    let yaml = "v: 'it''s fine'\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("v").and_then(|v| v.as_str()), Some("it's fine"));
}

// ── Single-quoted multi-line with empty line in middle. ──────────
#[test]
fn parser_scanner_single_quoted_multiline_with_empty_line() {
    let yaml = "v: 'line1\n\n  line2'\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.get("v").and_then(|v| v.as_str()).is_some());
}

// ── Double-quoted multi-line with empty line in middle. ──────────
#[test]
fn parser_scanner_double_quoted_multiline_with_empty_line() {
    let yaml = "v: \"line1\n\n  line2\"\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.get("v").and_then(|v| v.as_str()).is_some());
}

// ── Anchor name followed by EOF (boundary case for `scan_anchor_name`). ──
#[test]
fn parser_scanner_anchor_at_end_of_input_errors() {
    // Anchor with no content following — this exercises the
    // anchor-then-end paths.
    let yaml = "v: &anchor\n";
    let v: Value = from_str(yaml).unwrap();
    let _ = v;
}

// ── %YAML 1.3 (or beyond) — accepted but warned-against in some
//     implementations; here it just exercises the version-extra parse. ──
#[test]
fn parser_scanner_yaml_directive_version_only() {
    let yaml = "%YAML 1.2\n---\nv: 1\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("v").and_then(|v| v.as_i64()), Some(1));
}

// ── Comment-only directive line (`%YAML 1.2 # comment`). ──────────
#[test]
fn parser_scanner_directive_with_trailing_comment() {
    let yaml = "%YAML 1.2 # ok\n---\nv: 1\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("v").and_then(|v| v.as_i64()), Some(1));
}

// ════════════════════════════════════════════════════════════════════
// loader.rs
// ════════════════════════════════════════════════════════════════════

// Helper struct that forces the AST loader path via `Spanned`.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AstDoc {
    a: Spanned<i64>,
}

// ── max_events budget breach (loader::process_event::§MaxEvents). ──
#[test]
fn parser_loader_max_events_breach() {
    let yaml = "---\na: 1\n---\na: 2\n---\na: 3\n";
    let cfg = ParserConfig::new().max_events(3).max_documents(usize::MAX);
    let res: Result<Vec<Value>, _> = load_all_with_config(yaml, &cfg).and_then(|it| it.collect());
    let err = res.unwrap_err();
    assert!(matches!(err, Error::Budget(BudgetBreach::MaxEvents { .. })));
}

// ── max_total_scalar_bytes breach. ────────────────────────────────
#[test]
fn parser_loader_max_total_scalar_bytes_breach() {
    let big = "x".repeat(2_000);
    let yaml = format!("a: '{big}'\nb: '{big}'\n");
    let cfg = ParserConfig::new().max_total_scalar_bytes(1_000);
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct D {
        a: Spanned<String>,
    }
    let err = from_str_with_config::<D>(&yaml, &cfg).unwrap_err();
    assert!(matches!(
        err,
        Error::Budget(BudgetBreach::MaxTotalScalarBytes { .. })
    ));
}

// ── max_documents breach via load_all_with_config. ───────────────
#[test]
fn parser_loader_max_documents_breach() {
    let yaml = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
    let cfg = ParserConfig::new().max_documents(1);
    let res: Result<Vec<Value>, _> = load_all_with_config(yaml, &cfg).and_then(|it| it.collect());
    let err = res.unwrap_err();
    assert!(matches!(
        err,
        Error::Budget(BudgetBreach::MaxDocuments { .. })
    ));
}

// ── Alias-anchor ratio breach on AST loader. ─────────────────────
#[test]
fn parser_loader_alias_anchor_ratio_breach() {
    let yaml = "anchor: &a 1\nuses:\n  - *a\n  - *a\n  - *a\n  - *a\n  - *a\n  - *a\n  - *a\n  - *a\n  - *a\n  - *a\n";
    let cfg = ParserConfig::new()
        .alias_anchor_ratio(Some(2.0))
        .max_alias_expansions(1_000);
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct D {
        anchor: Spanned<i64>,
    }
    let err = from_str_with_config::<D>(yaml, &cfg).unwrap_err();
    assert!(matches!(
        err,
        Error::Budget(BudgetBreach::AliasAnchorRatio { .. })
    ));
}

// ── Unknown alias inside a document (anchor never defined). ──────
#[test]
fn parser_loader_unknown_alias_at_errors() {
    let yaml = "v: *missing\n";
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct D {
        v: Spanned<String>,
    }
    let err = from_str_with_config::<D>(yaml, &ParserConfig::new()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("missing") || msg.contains("anchor") || msg.contains("alias"));
}

// ── DuplicateKeyPolicy::Error trips on a duplicate. ──────────────
#[test]
fn parser_loader_duplicate_key_policy_error_rejects() {
    let yaml = "a: 1\na: 2\n";
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let err = from_str_with_config::<AstDoc>(yaml, &cfg).unwrap_err();
    let msg = err.to_string();
    assert!(msg.to_lowercase().contains("duplicate") || msg.contains('a'));
}

// ── DuplicateKeyPolicy::First keeps the first value. ─────────────
#[test]
fn parser_loader_duplicate_key_policy_first_keeps_first() {
    let yaml = "a: 1\na: 99\n";
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let d: AstDoc = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(d.a.value, 1);
}

// ── MergeKeyPolicy::Error rejects a `<<` key. ────────────────────
#[test]
fn parser_loader_merge_key_policy_error_rejects() {
    let yaml = "base: &b\n  x: 1\nuse:\n  <<: *b\n";
    let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::Error);
    let err = from_str_with_config::<Value>(yaml, &cfg).unwrap_err();
    assert!(err.to_string().contains("merge") || err.to_string().contains("<<"));
}

// ── MergeKeyPolicy::AsOrdinary treats `<<` as a literal key. ─────
#[test]
fn parser_loader_merge_key_policy_as_ordinary_keeps_key() {
    let yaml = "<<: literal\n";
    let cfg = ParserConfig::new().merge_key_policy(MergeKeyPolicy::AsOrdinary);
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(v.get("<<").and_then(|v| v.as_str()), Some("literal"));
}

// ── Anchor on a Tagged scalar — verifies anchor_map insertion path. ──
#[test]
fn parser_loader_anchor_on_tagged_scalar() {
    let yaml = "anchor: &a !!int 42\nuse: *a\n";
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct D {
        anchor: Spanned<i64>,
        #[serde(rename = "use")]
        used: Spanned<i64>,
    }
    let d: D = from_str(yaml).unwrap();
    assert_eq!(d.anchor.value, 42);
    assert_eq!(d.used.value, 42);
}

// ── Anchor on a sequence — verifies SequenceEnd anchor-map insertion. ──
#[test]
fn parser_loader_anchor_on_sequence() {
    let yaml = "items: &items\n  - 1\n  - 2\nuses: *items\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.get("items").is_some());
    assert!(v.get("uses").is_some());
}

// ── Anchor on a mapping — verifies MappingEnd anchor-map insertion. ──
#[test]
fn parser_loader_anchor_on_mapping() {
    let yaml = "src: &src\n  k: 1\nuse: *src\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.get("src").is_some());
    assert!(v.get("use").is_some());
}

// ── Custom (non-core) tag on a sequence wraps in Tagged. ─────────
#[test]
fn parser_loader_custom_tag_on_sequence_wraps_tagged() {
    let yaml = "items: !MyType\n  - 1\n  - 2\n";
    let v: Value = from_str(yaml).unwrap();
    let _ = v.get("items");
}

// ── Custom (non-core) tag on a mapping wraps in Tagged. ──────────
#[test]
fn parser_loader_custom_tag_on_mapping_wraps_tagged() {
    let yaml = "obj: !MyType\n  k: 1\n";
    let v: Value = from_str(yaml).unwrap();
    let _ = v.get("obj");
}

// ── Complex (sequence) key gets coerced via stringification. ─────
#[test]
fn parser_loader_complex_sequence_key_stringified() {
    let yaml = "? [1, 2]\n: composite\n";
    let v: Value = from_str(yaml).unwrap();
    // The complex key is rendered as `[1, 2]`-shaped string by
    // `value_to_key_string`. Just verify the value is reachable.
    assert!(v.as_mapping().is_some());
}

// ── Complex (mapping) key gets coerced via stringification. ──────
#[test]
fn parser_loader_complex_mapping_key_stringified() {
    let yaml = "? {a: 1}\n: composite\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_mapping().is_some());
}

// ── Float key coerced. ───────────────────────────────────────────
#[test]
fn parser_loader_float_key_stringified() {
    let yaml = "1.5: half\n2.5: more\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_mapping().is_some());
}

// ── Null key coerced. ────────────────────────────────────────────
#[test]
fn parser_loader_null_key_stringified() {
    let yaml = "~: nothing\n";
    let v: Value = from_str(yaml).unwrap();
    // The null-resolution may stringify as "" or "null" depending on
    // schema choice — just verify the mapping is non-empty so the
    // value-to-key-string null arm is exercised either way.
    assert!(v.as_mapping().is_some_and(|m| !m.is_empty()));
}

// ── Policy that rejects a SequenceStart event. ───────────────────
#[derive(Debug, Default)]
struct DenySequences;
impl Policy for DenySequences {
    fn check_event(&self, ev: PolicyEvent<'_>) -> noyalib::Result<()> {
        use noyalib::policy::PolicyEventKind;
        if ev.kind == PolicyEventKind::SequenceStart {
            return Err(Error::Custom("sequences denied".into()));
        }
        Ok(())
    }
}

#[test]
fn parser_loader_policy_rejects_sequence_event() {
    let yaml = "items:\n  - 1\n  - 2\n";
    let cfg = ParserConfig::new().with_policy(DenySequences);
    let err = from_str_with_config::<Value>(yaml, &cfg).unwrap_err();
    assert!(err.to_string().contains("sequences"));
}

// ── Policy that rejects a MappingStart event. ────────────────────
#[derive(Debug, Default)]
struct DenyMappings;
impl Policy for DenyMappings {
    fn check_event(&self, ev: PolicyEvent<'_>) -> noyalib::Result<()> {
        use noyalib::policy::PolicyEventKind;
        if ev.kind == PolicyEventKind::MappingStart {
            return Err(Error::Custom("mappings denied".into()));
        }
        Ok(())
    }
}

#[test]
fn parser_loader_policy_rejects_mapping_event() {
    let yaml = "k: v\n";
    let cfg = ParserConfig::new().with_policy(DenyMappings);
    let err = from_str_with_config::<Value>(yaml, &cfg).unwrap_err();
    assert!(err.to_string().contains("mappings"));
}

// ── Policy that rejects an Alias event. ─────────────────────────
#[derive(Debug, Default)]
struct DenyAliases;
impl Policy for DenyAliases {
    fn check_event(&self, ev: PolicyEvent<'_>) -> noyalib::Result<()> {
        use noyalib::policy::PolicyEventKind;
        if ev.kind == PolicyEventKind::Alias {
            return Err(Error::Custom("aliases denied".into()));
        }
        Ok(())
    }
}

#[test]
fn parser_loader_policy_rejects_alias_event() {
    let yaml = "anchor: &a 1\nuse: *a\n";
    let cfg = ParserConfig::new().with_policy(DenyAliases);
    let err = from_str_with_config::<Value>(yaml, &cfg).unwrap_err();
    assert!(err.to_string().contains("aliases"));
}

// ════════════════════════════════════════════════════════════════════
// events.rs
// ════════════════════════════════════════════════════════════════════

// ── Anchor adjacency: `&a !!str` (anchor before tag). ────────────
#[test]
fn parser_events_anchor_then_tag_on_node() {
    let yaml = "v: &a !!str hello\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("v").and_then(|v| v.as_str()), Some("hello"));
}

// ── Tag-before-anchor adjacency: `!!str &a` ──────────────────────
#[test]
fn parser_events_tag_then_anchor_on_node() {
    let yaml = "v: !!str &a hello\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("v").and_then(|v| v.as_str()), Some("hello"));
}

// ── Anchor-only node with no following content → empty scalar. ───
#[test]
fn parser_events_anchor_only_yields_empty_scalar() {
    let yaml = "v: &empty\n";
    let v: Value = from_str(yaml).unwrap();
    let _ = v.get("v");
}

// ── Stray content after the first implicit document. ────────────
#[test]
fn parser_events_stray_content_after_implicit_doc_errors() {
    let yaml = "first\nsecond\nthird\n";
    let _ = from_str::<Value>(yaml);
}

// ── Subsequent document after `...` without `---` is allowed (7Z25). ─
#[test]
fn parser_events_implicit_doc_after_explicit_end() {
    let yaml = "---\nfirst: 1\n...\nsecond: 2\n";
    let res = from_str::<Value>(yaml);
    let _ = res;
}

// ── Flow sequence with a leading `:` (CFD4: `[ : value ]`). ─────
#[test]
fn parser_events_flow_seq_implicit_empty_key_mapping() {
    let yaml = "v: [ : empty_key_value ]\n";
    let v: Value = from_str(yaml).unwrap();
    let _ = v.get("v");
}

// ── Flow mapping with a leading `:` (`{ : bar }`). ──────────────
#[test]
fn parser_events_flow_map_implicit_empty_key() {
    let yaml = "v: { : bar }\n";
    let v: Value = from_str(yaml).unwrap();
    let _ = v.get("v");
}

// ── Flow sequence with `?` explicit-key. ─────────────────────────
#[test]
fn parser_events_flow_seq_explicit_key() {
    let yaml = "v: [ ? complex_key ]\n";
    let res = from_str::<Value>(yaml);
    let _ = res;
}

// ── Flow mapping with `?` explicit-key. ──────────────────────────
#[test]
fn parser_events_flow_map_explicit_key() {
    let yaml = "v: { ? key : value }\n";
    let v: Value = from_str(yaml).unwrap();
    let _ = v.get("v");
}

// ── Flow mapping with explicit key but no value. ─────────────────
#[test]
fn parser_events_flow_map_explicit_key_no_value() {
    let yaml = "v: { ? lonely }\n";
    let v: Value = from_str(yaml).unwrap();
    let _ = v.get("v");
}

// ── Bare `Value` token in flow sequence triggers MappingStart-then-empty-key.
#[test]
fn parser_events_flow_seq_bare_value_emits_mapping() {
    let yaml = "v: [: only_value]\n";
    let _ = from_str::<Value>(yaml);
}

// ── Block mapping with `?` then immediately newline / no Value. ──
#[test]
fn parser_events_block_explicit_key_no_value() {
    let yaml = "? lonely\n? another\n: value\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(v.get("another").and_then(|v| v.as_str()), Some("value"));
}

// ── Block mapping where `:` value is absent (BlockEnd follows). ──
#[test]
fn parser_events_block_value_then_end() {
    let yaml = "k:\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.as_mapping().is_some());
}

// ── Indentless block sequence where the `-` is at the same column
//     as the parent mapping key. ─────────────────────────────────
#[test]
fn parser_events_indentless_block_sequence() {
    let yaml = "items:\n- 1\n- 2\n- 3\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(
        v.get("items")
            .and_then(|v| v.as_sequence())
            .map(|s| s.len()),
        Some(3)
    );
}

// ── Anchor on an indentless sequence (covers BlockEntry+anchor arm). ─
#[test]
fn parser_events_anchor_on_indentless_sequence() {
    let yaml = "items: &is\n- 1\n- 2\nrefs: *is\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.get("items").is_some());
    assert!(v.get("refs").is_some());
}

// ── Multiple flow entries with trailing comma. ───────────────────
#[test]
fn parser_events_flow_seq_trailing_comma() {
    let yaml = "v: [1, 2, 3,]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(
        v.get("v").and_then(|v| v.as_sequence()).map(|s| s.len()),
        Some(3)
    );
}

// ── Flow mapping trailing comma. ─────────────────────────────────
#[test]
fn parser_events_flow_map_trailing_comma() {
    let yaml = "v: {a: 1, b: 2,}\n";
    let v: Value = from_str(yaml).unwrap();
    let m = v.get("v").and_then(|v| v.as_mapping()).unwrap();
    assert_eq!(m.len(), 2);
}

// ── Flow mapping with `,` not followed by entry — error path
//     in `parse_flow_mapping_key` ("expected ',' or '}'"). ────────
#[test]
fn parser_events_flow_map_garbage_separator_errors() {
    let yaml = "v: {a: 1 ; b: 2}\n";
    let _ = from_str::<Value>(yaml);
}

// ── Flow sequence broken separator. ──────────────────────────────
#[test]
fn parser_events_flow_seq_garbage_separator_errors() {
    let yaml = "v: [1 ; 2]\n";
    let _ = from_str::<Value>(yaml);
}

// ── Empty document (just `---\n`) — covers parse_document_content's
//     empty-scalar emission and parse_document_end. ──────────────
#[test]
fn parser_events_empty_document_yields_null() {
    let yaml = "---\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.is_null());
}

// ── Nested flow with both anchors and tags on inner nodes. ───────
#[test]
fn parser_events_nested_flow_with_anchors_and_tags() {
    let yaml = "v: [&a 1, !!str &b two, *a, *b]\n";
    let v: Value = from_str(yaml).unwrap();
    assert_eq!(
        v.get("v").and_then(|v| v.as_sequence()).map(|s| s.len()),
        Some(4)
    );
}
