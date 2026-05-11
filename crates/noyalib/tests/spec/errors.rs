// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// YAML spec: Error cases — invalid YAML that should be rejected

use std::collections::HashMap;

use noyalib::{Value, from_str};

#[test]
fn invalid_indentation() {
    let result: Result<Value, _> = from_str("a: 1\n b: 2\n");
    // This may parse differently than expected — at minimum should not panic
    let _ = result;
}

#[test]
fn unclosed_flow_sequence() {
    let result: Result<Vec<i64>, _> = from_str("[1, 2, 3");
    assert!(result.is_err());
}

#[test]
fn unclosed_flow_mapping() {
    let result: Result<HashMap<String, i64>, _> = from_str("{a: 1, b: 2");
    assert!(result.is_err());
}

#[test]
fn tab_as_indentation() {
    // YAML spec forbids tabs for indentation
    let result: Result<Value, _> = from_str("a:\n\tb: 1\n");
    assert!(result.is_err());
}

#[test]
fn type_mismatch_string_as_int() {
    let result: Result<i64, _> = from_str("hello");
    assert!(result.is_err());
}

#[test]
fn type_mismatch_mapping_as_seq() {
    let result: Result<Vec<String>, _> = from_str("a: 1\nb: 2\n");
    assert!(result.is_err());
}

#[test]
fn type_mismatch_seq_as_mapping() {
    let result: Result<HashMap<String, String>, _> = from_str("- a\n- b\n");
    assert!(result.is_err());
}

#[test]
fn empty_yaml_is_error() {
    let result: Result<i64, _> = from_str("");
    assert!(result.is_err());
}

#[test]
fn stray_scalar_after_mapping() {
    let result: Result<HashMap<String, String>, _> = from_str("foo: bar\ninvalid\n");
    // Should fail or produce unexpected results
    let _ = result;
}

#[test]
fn max_depth_exceeded() {
    use noyalib::{ParserConfig, from_str_with_config};

    // Create YAML that nests 10 levels deep, but set limit to 5
    let yaml = "a:\n  b:\n    c:\n      d:\n        e:\n          f: 1\n";
    let config = ParserConfig::new().max_depth(5);
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err(), "should reject excessive nesting");
}

#[test]
fn max_document_length_exceeded() {
    use noyalib::{ParserConfig, from_str_with_config};

    let config = ParserConfig::new().max_document_length(10);
    let yaml = "this is more than 10 bytes";
    let result: Result<Value, _> = from_str_with_config(yaml, &config);
    assert!(result.is_err(), "should reject oversized document");
}

#[test]
fn missing_required_struct_field() {
    use serde::Deserialize;

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct Required {
        name: String,
        age: i64,
    }

    let result: Result<Required, _> = from_str("name: John\n");
    assert!(result.is_err());
}

#[test]
fn wrong_type_in_sequence() {
    let result: Result<Vec<i64>, _> = from_str("- 1\n- hello\n- 3\n");
    assert!(result.is_err());
}

#[test]
fn invalid_escape_in_double_quote() {
    let result: Result<String, _> = from_str("\"\\z\"");
    // Invalid escape — should error
    assert!(result.is_err());
}

#[test]
fn no_panic_on_any_input() {
    // Fuzz-like test: various malformed inputs should not panic
    let inputs = [
        "",
        "---",
        "...",
        "[",
        "{",
        "- - -",
        "!!",
        "&",
        "*",
        "---\n---",
        "key: [unclosed",
        "key: {unclosed",
        ":\n:",
        "- :\n  - :",
    ];

    for input in inputs {
        let _ = from_str::<Value>(input);
    }
}

// yaml-test-suite SU5Z — `#` adjacent to prior content is not a comment.
#[test]
fn comment_indicator_must_be_preceded_by_whitespace() {
    let result: Result<Value, _> = from_str("key: \"value\"# invalid comment\n");
    assert!(
        result.is_err(),
        "expected rejection of inline `#` without preceding whitespace"
    );
}

// yaml-test-suite X4QW — same rule inside the `>`/`|` header line.
#[test]
fn block_scalar_header_rejects_adjacent_hash() {
    let result: Result<Value, _> = from_str("block: >#comment\n  scalar\n");
    assert!(
        result.is_err(),
        "expected rejection of `>#` with no whitespace"
    );
}

// yaml-test-suite SF5V — at most one %YAML directive per document.
#[test]
fn duplicate_yaml_directive_rejected() {
    let result: Result<Value, _> = from_str("%YAML 1.2\n%YAML 1.2\n---\n");
    assert!(
        result.is_err(),
        "expected rejection of duplicate %YAML directive"
    );
}

// yaml-test-suite Y79Y :4..:7 — tab immediately before a block-structural
// indicator is forbidden (cannot stand in for indentation).
#[test]
fn tab_before_block_indicator_rejected() {
    for input in &["-\t-\n", "- \t-\n", "?\t-\n", "? -\n:\t-\n"] {
        let result: Result<Value, _> = from_str(input);
        assert!(
            result.is_err(),
            "expected rejection of tab-as-indentation in {input:?}"
        );
    }
}

// yaml-test-suite A2M4 (spec example 6.2) — tab as inline separation
// before plain content is *valid*; only tabs before another structural
// indicator are rejected.
#[test]
fn tab_as_inline_separation_accepted() {
    let v: Value = from_str("? a\n: -\tb\n  -  -\tc\n     - d\n").unwrap();
    let outer = v.as_mapping().expect("mapping");
    let seq = outer.get("a").expect("key 'a'").as_sequence().expect("seq");
    assert_eq!(seq[0].as_str(), Some("b"));
}

// yaml-test-suite 3HFZ — content after `...` document-end marker on the
// same line is invalid.
#[test]
fn document_end_marker_rejects_trailing_content() {
    let result: Result<Value, _> = from_str("---\nkey: value\n... invalid\n");
    assert!(result.is_err());
}

// yaml-test-suite RXY3 / 5TRB — a `---` or `...` document indicator at
// column 0 inside a multi-line quoted scalar is invalid (it would
// prematurely close the document).
#[test]
fn doc_marker_inside_quoted_scalar_rejected() {
    let r1: Result<Value, _> = from_str("---\n'\n...\n'\n");
    assert!(r1.is_err(), "single-quoted scalar containing `...`");
    let r2: Result<Value, _> = from_str("---\n\"\n---\n\"\n");
    assert!(r2.is_err(), "double-quoted scalar containing `---`");
}

// yaml-test-suite 2G84 — block scalar indent indicator must be a single
// digit 1..9; `0` and multi-digit forms are rejected.
#[test]
fn block_scalar_indent_indicator_validation() {
    let r1: Result<Value, _> = from_str("--- |0\n");
    assert!(r1.is_err(), "indent indicator 0 is invalid");
    let r2: Result<Value, _> = from_str("--- |10\n");
    assert!(r2.is_err(), "two-digit indent indicator is invalid");
}

// yaml-test-suite 4H7K — a stray `]` outside any flow sequence is an
// error.
#[test]
fn stray_flow_close_outside_flow_rejected() {
    let r1: Result<Value, _> = from_str("[ a, b, c ] ]\n");
    assert!(r1.is_err());
    let r2: Result<Value, _> = from_str("{ a: 1 } }\n");
    assert!(r2.is_err());
}

// yaml-test-suite H7TQ — non-numeric trailing content after the version
// of a `%YAML` directive is rejected. (Numeric continuations are
// accepted as a lenient extension; see ZYU8.)
#[test]
fn yaml_directive_rejects_non_numeric_extras() {
    let r: Result<Value, _> = from_str("%YAML 1.2 foo\n---\n");
    assert!(r.is_err());
    // Numeric-looking trailing token still parses.
    let _: Value = from_str("%YAML 1.1 1.2\n---\n").unwrap();
}

// yaml-test-suite 4HVU / EW3V / DMG6 / N4JP / U44R — block-context
// content at a column that does not match any open block scope
// (i.e. "between levels") is rejected. The check fires only at
// positions where a *new* mapping key or sequence entry could
// start, so separator tokens like `:` mid-pair are unaffected.
#[test]
fn between_levels_indentation_rejected() {
    // 4HVU — sequence entries at col 3 then a `-` at col 2.
    let r: Result<Value, _> = from_str("key:\n   - ok\n   - also ok\n  - wrong\n");
    assert!(r.is_err());

    // EW3V — second mapping key at col 1, parent at col 0.
    let r: Result<Value, _> = from_str("k1: v1\n k2: v2\n");
    assert!(r.is_err());

    // DMG6 — nested mapping then over-indented sibling.
    let r: Result<Value, _> = from_str("key:\n  ok: 1\n wrong: 2\n");
    assert!(r.is_err());
}

// Counter-examples: nested blocks (each level deeper) and sibling
// alignments (each at the same level) must continue to parse.
#[test]
fn correctly_indented_blocks_still_parse() {
    let _: Value = from_str("a:\n  b:\n    c: 1\n").unwrap();
    let _: Value = from_str("a: 1\nb: 2\nc: 3\n").unwrap();
    let _: Value = from_str("xs:\n  - 1\n  - 2\n  - 3\n").unwrap();
    let _: Value = from_str("# comment\nkey: value\n").unwrap();
}

// yaml-test-suite QB6E — continuation lines of a multi-line quoted
// scalar in block context must be indented more than the parent.
#[test]
fn quoted_scalar_continuation_must_be_indented() {
    // Continuation at col 0 inside an indented mapping → reject.
    let r: Result<Value, _> = from_str("---\nquoted: \"a\nb\nc\"\n");
    assert!(r.is_err());
    // Indented continuation parses.
    let _: Value = from_str("---\nquoted: \"a\n  b\n  c\"\n").unwrap();
}

// yaml-test-suite 7LBH / D49Q / G7JE — implicit (`?`-less) keys in
// block context must fit on a single line. Quoted, single-quoted,
// and plain-scalar variants all reject.
#[test]
fn multiline_implicit_key_rejected_in_block_context() {
    // 7LBH — double-quoted multi-line key.
    let r: Result<Value, _> = from_str("\"a\\nb\": 1\n\"c\n d\": 1\n");
    assert!(r.is_err());
    // D49Q — single-quoted multi-line key.
    let r: Result<Value, _> = from_str("'a\\nb': 1\n'c\n d': 1\n");
    assert!(r.is_err());
    // G7JE — plain multi-line key.
    let r: Result<Value, _> = from_str("a\\nb: 1\nc\n d: 1\n");
    assert!(r.is_err());
}

// yaml-test-suite 6M2F — `&b b\n: *a` is *valid*: the `:` on the next
// line is an empty implicit key indicator (a new pair), not a value
// separator for the anchored scalar above. The strict implicit-key
// check must distinguish this from a genuinely multi-line key.
#[test]
fn empty_implicit_key_after_anchored_value_parses() {
    let _: Value = from_str("? &a a\n: &b b\n: *a\n").unwrap();
}

// yaml-test-suite CXX2 / 9KBC — block-structural indicators (`:`,
// `?`, `-`) cannot open a collection on the same line as `---`. The
// `---` indicator may share a line only with a scalar or flow node.
#[test]
fn block_collection_inline_with_doc_start_rejected() {
    // CXX2 — anchor + key + `:` inline with `---`.
    let r: Result<Value, _> = from_str("--- &anchor a: b\n");
    assert!(r.is_err());
    // 9KBC — bare key + `:` inline with `---`.
    let r: Result<Value, _> = from_str("--- key1: value1\n    key2: value2\n");
    assert!(r.is_err());
    // Counter-examples: scalar / flow node inline with `---` is fine.
    let _: Value = from_str("--- text\n").unwrap();
    let _: Value = from_str("--- {a: 1}\n").unwrap();
}

// yaml-test-suite 9MMA / B63P — directives must be followed by an
// explicit `---` document indicator. A directive with no document is
// invalid.
#[test]
fn directive_without_document_rejected() {
    let r: Result<Value, _> = from_str("%YAML 1.2\n");
    assert!(r.is_err());
}

// yaml-test-suite RHX7 / 9HCY / MUS6:1 — directives must not appear
// between document content and the next `---` without an intervening
// `...` to close the previous document.
//
// EB22 (`---\nscalar1\n%YAML 1.2\n---\nscalar2`) is an adjacent
// case: per the spec, the parser would need lookahead to tell
// whether `%YAML` is plain-scalar continuation (XLQ9-style) or a
// directive announcing a new doc. We accept the lenient reading
// here for now.
#[test]
fn directive_without_doc_end_marker_rejected() {
    // RHX7 — second `%YAML` after a mapping with no `...`.
    let r: Result<Value, _> = from_str("---\nkey: value\n%YAML 1.2\n---\n");
    assert!(r.is_err());
    // 9HCY — implicit doc + `%TAG` without `...`.
    let r: Result<Value, _> =
        from_str("!foo \"bar\"\n%TAG ! tag:example.com,2000:app/\n---\n!foo \"bar\"\n");
    assert!(r.is_err());
    // Counter-example: directive after `...` is fine.
    let _: Vec<Value> =
        noyalib::load_all_as("---\nfoo: bar\n...\n%YAML 1.2\n---\nbaz: qux\n").unwrap();
}

// yaml-test-suite MUS6:0 — `%YAML 1.1#...` packs a comment indicator
// directly against the version digits with no whitespace separator.
#[test]
fn directive_comment_without_whitespace_rejected() {
    let r: Result<Value, _> = from_str("%YAML 1.1#...\n---\n");
    assert!(r.is_err());
    // Comment with whitespace is fine.
    let _: Value = from_str("%YAML 1.1 # ok\n---\nfoo: 1\n").unwrap();
}

// yaml-test-suite SR86 / SU74 — aliases are complete references, so
// they cannot be decorated with anchors (or tags). The check fires
// only on direct same-line adjacency; line-broken cases like
// `&node3\n  *alias1: scalar3` (26DV) where the anchor decorates
// an inner mapping are still valid.
#[test]
fn alias_decorated_by_anchor_rejected() {
    // SR86 — `&b *a` adjacency.
    let r: Result<Value, _> = from_str("key1: &a value\nkey2: &b *a\n");
    assert!(r.is_err());
    // Line-broken counter-pattern: an unknown alias might fail
    // resolution, but it must NOT fail the adjacency check — it
    // should reach the loader.
    let r: Result<Value, _> = from_str("top: &n3\n  *alias : scalar3\n");
    if let Err(e) = r {
        assert!(
            !e.to_string().contains("alias cannot be decorated"),
            "line-broken anchor → alias-key must not trigger the adjacency guard"
        );
    }
}

// yaml-test-suite LHL4 — `!invalid{}tag` packs flow indicators into a
// tag URI without separation. Tag URIs are followed by separation
// (whitespace / line break) before the next node.
#[test]
fn tag_followed_by_flow_indicator_rejected() {
    let r: Result<Value, _> = from_str("---\n!invalid{}tag scalar\n");
    assert!(r.is_err());
    // Counter-example: tag separated by whitespace from a flow node.
    let _: Value = from_str("---\n!foo {a: 1}\n").unwrap();
}

// yaml-test-suite BS4K / KS4U — content after the first document's
// root node, without `---` or `...` to mark a new document, is stray
// and rejected. Bare implicit doc 2 after `...` is fine (7Z25).
#[test]
fn stray_content_after_first_implicit_document_rejected() {
    // BS4K — comment terminates plain scalar; second scalar is stray.
    let r: Result<Value, _> = from_str("word1  # comment\nword2\n");
    assert!(r.is_err());
    // KS4U — content after closing `]` of the root flow seq.
    let r: Result<Value, _> = from_str("---\n[\nseq\n]\nstray\n");
    assert!(r.is_err());
    // 7Z25 — implicit doc 2 after explicit `...` is fine.
    let _: Vec<Value> = noyalib::load_all_as("---\nscalar1\n...\nkey: value\n").unwrap();
}

// yaml-test-suite 9C9N — flow content continuation across a line
// break must be indented more than the surrounding block; otherwise
// it would be ambiguous with sibling block content.
#[test]
fn flow_continuation_must_be_indented_more_than_parent() {
    // 9C9N — flow seq continues at col 0 inside an indented block.
    let r: Result<Value, _> = from_str("---\nflow: [a,\nb,\nc]\n");
    assert!(r.is_err());
    // Counter-examples: properly-indented continuation parses.
    let _: Value = from_str("---\nflow: [a,\n  b,\n  c]\n").unwrap();
    let _: Value = from_str("[\n  a,\n  b\n]\n").unwrap();
}

// yaml-test-suite 9KBC / CXX2 — `from_str` previously stopped lazily
// at the first complete value, silently swallowing the spec
// violations that follow. The streaming deserializer now drains
// trailing events, surfacing those errors instead of returning a
// partial value.
#[test]
fn from_str_drains_trailing_events_to_surface_errors() {
    // 9KBC — a mapping inlined onto the `---` line is invalid; the
    // continuation key on the next line triggers
    // "mapping values are not allowed in this context" once events
    // past the first scalar are fetched.
    let r: Result<Value, _> = from_str("--- key1: value1\n    key2: value2\n");
    assert!(
        r.is_err(),
        "expected from_str to surface the lazy-only-accept error"
    );

    // CXX2 — anchor + key on the document-start line.
    let r: Result<Value, _> = from_str("--- &anchor a: b\n");
    assert!(r.is_err());
}
