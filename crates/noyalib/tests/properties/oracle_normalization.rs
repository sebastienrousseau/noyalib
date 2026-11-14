// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Narrow, semantics-checked repairs for known development-oracle defects.

use noyalib::cst::{GreenChild, GreenNode, SyntaxKind};
use std::collections::BTreeMap;

type Lexeme = (SyntaxKind, usize, usize);
type Replacement = (usize, usize, String);

/// Remove a stream-prefix BOM only, then apply existing source repairs.
/// Callers still require complete candidate preservation and archive agreement.
pub(crate) fn stream_bom_prefix(source: &str) -> Option<String> {
    let source = source.strip_prefix('\u{feff}')?;
    Some(collection_spelling(source).unwrap_or_else(|| source.to_owned()))
}

/// YAML suite L24T retains a final break after surplus spaces at EOF.
/// Terminate that source line for archived parsers. Callers must first
/// prove the candidate value unchanged, then compare the entire oracle value.
pub(crate) fn terminated_spaced_block(source: &str) -> Option<String> {
    let tokens = lexical_tokens(source)?;
    let (kind, _, end) = tokens.last()?;
    let last_line = source.rsplit(['\n', '\r']).next()?;
    (matches!(kind, SyntaxKind::LiteralScalar | SyntaxKind::FoldedScalar)
        && *end == source.len()
        && !last_line.is_empty()
        && last_line.bytes().all(|byte| byte == b' '))
    .then(|| format!("{source}\n"))
}

// The compatibility target needs this classifier; the value-only target
// includes this shared module without using acceptance classifications.
#[allow(dead_code)]
pub(crate) fn quoted_indent_rejection(message: &str) -> bool {
    let Some((reason, _)) = message.split_once(" at line ") else {
        return false;
    };
    matches!(
        reason,
        "single-quoted continuation must be indented more than the parent block"
            | "double-quoted continuation must be indented more than the parent block"
    )
}

// YAML 1.2.2 section 6.6 requires separation before a comment marker.
#[allow(dead_code)]
pub(crate) fn missing_comment_separator_rejection(message: &str) -> bool {
    message.split_once(" at line ").is_some_and(|(reason, _)| {
        reason == "comment indicator '#' must be preceded by a space, tab, or line break"
    })
}

/// YAML 1.2.2 section 6.2 permits inline tab separation after a dash.
#[allow(dead_code)]
pub(crate) fn spaced_sequence_tabs(source: &str) -> Option<String> {
    let tokens = lexical_tokens(source)?;
    let replacements: Vec<_> = tokens
        .windows(2)
        .filter_map(|pair| {
            let (kind, start, end) = pair[1];
            if pair[0].0 != SyntaxKind::DashIndicator || kind != SyntaxKind::Whitespace {
                return None;
            }
            let text = source.get(start..end)?;
            text.contains('\t')
                .then(|| (start, end, text.replace('\t', " ")))
        })
        .collect();
    (!replacements.is_empty()).then(|| apply_replacements(source, replacements))
}

/// Replace empty-line tab separation, including consumed plain-scalar trivia.
fn spaced_empty_line_tabs(source: &str) -> Option<String> {
    let mut replacements = Vec::new();
    for (kind, start, end) in lexical_tokens(source)? {
        let text = source.get(start..end)?;
        if !matches!(kind, SyntaxKind::Whitespace | SyntaxKind::PlainScalar) || !text.contains('\t')
        {
            continue;
        }
        let mut offset = start;
        for line in text.split_inclusive(['\n', '\r']) {
            let body = line.trim_end_matches(['\n', '\r']);
            let end = offset + body.len();
            let line_start = source[..offset].rfind(['\n', '\r']).map_or(0, |i| i + 1);
            let line_start = if line_start == 0 && source.starts_with('\u{feff}') {
                '\u{feff}'.len_utf8()
            } else {
                line_start
            };
            let line_end = source[end..]
                .find(['\n', '\r'])
                .map_or(source.len(), |i| end + i);
            if body.contains('\t')
                && source[line_start..line_end]
                    .bytes()
                    .all(|b| b == b' ' || b == b'\t')
            {
                replacements.push((offset, end, body.replace('\t', " ")));
            }
            offset += line.len();
        }
    }
    (!replacements.is_empty()).then(|| apply_replacements(source, replacements))
}

/// YAML 1.2.2 sections 6.2 and 9.2 permit prefix tab separation and suffixes.
/// Libyaml refuses some such prefixes. Repair only those leading tokens;
/// callers still require full candidate-value preservation and oracle agreement.
fn spaced_stream_prefix(source: &str) -> Option<String> {
    let mut replacements = Vec::new();
    for (kind, start, end) in lexical_tokens(source)? {
        match kind {
            SyntaxKind::Whitespace => {
                let text = source.get(start..end)?;
                if text.contains('\t') {
                    replacements.push((start, end, text.replace('\t', " ")));
                }
            }
            SyntaxKind::Newline | SyntaxKind::Comment | SyntaxKind::Bom => {}
            SyntaxKind::DocEnd => replacements.push((start, end, " ".repeat(end - start))),
            _ => break,
        }
    }
    (!replacements.is_empty()).then(|| apply_replacements(source, replacements))
}

/// Quote omitted block keys identified by their CST entry roles. Callers
/// must preserve the complete candidate value and compare a fresh oracle.
fn quoted_empty_block_keys(source: &str) -> Option<String> {
    fn walk(node: &GreenNode, offset: usize, replacements: &mut Vec<Replacement>) {
        if node.kind() == SyntaxKind::MappingEntry {
            let mut position = offset;
            for child in node.children() {
                if child.text_len() == 0
                    || matches!(
                        child,
                        GreenChild::Token {
                            kind: SyntaxKind::Whitespace
                                | SyntaxKind::Newline
                                | SyntaxKind::Comment
                                | SyntaxKind::Bom,
                            ..
                        }
                    )
                {
                    position += child.text_len();
                    continue;
                }
                if matches!(
                    child,
                    GreenChild::Token {
                        kind: SyntaxKind::ColonIndicator,
                        ..
                    }
                ) {
                    replacements.push((position, position, "\"\"".to_owned()));
                }
                break;
            }
        }
        let mut position = offset;
        for child in node.children() {
            if let GreenChild::Node(node) = child {
                walk(node, position, replacements);
            }
            position += child.text_len();
        }
    }
    let document = noyalib::cst::parse_document(source).ok()?;
    let mut replacements = Vec::new();
    walk(document.syntax(), 0, &mut replacements);
    (!replacements.is_empty()).then(|| apply_replacements(source, replacements))
}

/// YAML suite FP8R permits zero-indented root block content. Give every
/// content line one additional space for the archive, retaining relative
/// indentation and all line breaks. Callers still compare complete values.
fn indented_root_block(source: &str) -> Option<String> {
    let tokens = lexical_tokens(source)?;
    // A `---` marker and node properties (`&a`, `!t`) may precede the
    // block scalar header.
    let (kind, start, end) = tokens.into_iter().find(|(kind, _, _)| {
        !matches!(
            kind,
            SyntaxKind::Whitespace
                | SyntaxKind::Newline
                | SyntaxKind::Comment
                | SyntaxKind::Bom
                | SyntaxKind::AnchorMark
                | SyntaxKind::TagMark
                | SyntaxKind::DocStart
        )
    })?;
    if !matches!(kind, SyntaxKind::LiteralScalar | SyntaxKind::FoldedScalar) {
        return None;
    }
    let text = source.get(start..end)?;
    let header_end = text.find(['\r', '\n'])?;
    if text[..header_end]
        .split_whitespace()
        .next()?
        .bytes()
        .any(|b| b.is_ascii_digit())
    {
        return None;
    }
    let break_len = if text[header_end..].starts_with("\r\n") {
        2
    } else {
        1
    };
    let body_start = start + header_end + break_len;
    let body = source.get(body_start..end)?;
    if !body.split(['\r', '\n']).any(|line| {
        line.as_bytes()
            .first()
            .is_some_and(|b| !matches!(b, b' ' | b'\t'))
    }) {
        return None;
    }
    let mut replacements = vec![(body_start, body_start, " ".to_owned())];
    for (offset, byte) in body.bytes().enumerate() {
        if (byte == b'\n' || (byte == b'\r' && body.as_bytes().get(offset + 1) != Some(&b'\n')))
            && offset + 1 < body.len()
        {
            let position = body_start + offset + 1;
            replacements.push((position, position, " ".to_owned()));
        }
    }
    Some(apply_replacements(source, replacements))
}

/// Compose the proven stream-prefix, separation and collection spelling repairs.
#[allow(dead_code)]
pub(crate) fn collection_spelling(source: &str) -> Option<String> {
    let prefix = spaced_stream_prefix(source);
    let separated = spaced_sequence_tabs(prefix.as_deref().unwrap_or(source)).or(prefix);
    let separated = spaced_empty_line_tabs(separated.as_deref().unwrap_or(source)).or(separated);
    let quoted = quoted_empty_block_keys(separated.as_deref().unwrap_or(source)).or(separated);
    let quoted = quoted_empty_flow_keys(quoted.as_deref().unwrap_or(source)).or(quoted);
    let indented = indented_root_block(quoted.as_deref().unwrap_or(source)).or(quoted);
    terminated_spaced_block(indented.as_deref().unwrap_or(source)).or(indented)
}

/// Give each declaration a unique name only when the archive's anchor IDs
/// collide at an alias. Callers must still compare the entire parsed value.
pub(crate) fn unique_colliding_anchors(source: &str) -> Option<String> {
    let tokens = lexical_tokens(source)?;
    let mut declarations = BTreeMap::new();
    let mut archive_ids = BTreeMap::new();
    let mut archive_owners = BTreeMap::new();
    let mut next = 0;
    let mut collision = false;
    let mut aliases = Vec::new();
    let mut replacements = Vec::new();
    for (kind, start, end) in tokens {
        match kind {
            SyntaxKind::AnchorMark => {
                let name = source.get(start + 1..end)?;
                // Both archive loaders allocate IDs from distinct-name
                // count. Redefining a name lets a later name reuse its ID.
                let archive_id = archive_ids.len();
                let _ = archive_ids.insert(name, archive_id);
                let _ = archive_owners.insert(archive_id, next);
                let _ = declarations.insert(name, next);
                replacements.push((start, end, format!("&noya_oracle_{next}")));
                next += 1;
            }
            SyntaxKind::AliasMark => {
                let name = source.get(start + 1..end)?;
                let declaration = *declarations.get(name)?;
                let archive_id = *archive_ids.get(name)?;
                collision |= archive_owners.get(&archive_id) != Some(&declaration);
                aliases.push((archive_id, declaration));
                replacements.push((start, end, format!("*noya_oracle_{declaration}")));
            }
            _ => {}
        }
    }
    // The archive composes values after loading the complete document.
    // A later ID overwrite can therefore corrupt an earlier alias too.
    collision |= aliases
        .iter()
        .any(|(id, declaration)| archive_owners.get(id) != Some(declaration));
    collision.then(|| apply_replacements(source, replacements))
}

/// Quote flow spellings the archived parsers cannot load, preserving values.
/// Callers must reparse and compare the entire resulting JSON value.
pub(crate) fn quoted_empty_flow_keys(source: &str) -> Option<String> {
    let tokens = lexical_tokens(source)?;
    let mut previous = None;
    let mut flow = Vec::new();
    let mut replacements = Vec::new();
    for (kind, start, end) in tokens {
        match kind {
            SyntaxKind::Whitespace
            | SyntaxKind::Newline
            | SyntaxKind::Comment
            | SyntaxKind::Bom => continue,
            SyntaxKind::ColonIndicator
                if matches!(
                    previous,
                    Some(SyntaxKind::OpenBrace | SyntaxKind::OpenBracket | SyntaxKind::Comma)
                ) && !flow.is_empty() =>
            {
                replacements.push((start, start, "\"\"".to_owned()));
            }
            SyntaxKind::PlainScalar if !flow.is_empty() => {
                // YAML 1.2.2 §7.3.3 permits ':' followed by a plain-safe
                // character, including in a flow collection.
                // A plain token can span its trailing break; replace only
                // the scalar so a following comment keeps its separation.
                let raw = source.get(start..end)?;
                let token_end = end;
                let text = raw.trim_matches([' ', '\t', '\r', '\n']);
                let start =
                    start + (raw.len() - raw.trim_start_matches([' ', '\t', '\r', '\n']).len());
                let end = start + text.len();
                let rest = source.get(end..)?;
                let compact_empty_value =
                    rest.starts_with(":,") || rest.starts_with(":}") || rest.starts_with(":]");
                if text.contains(['\n', '\r']) {
                    // Flow mappings permit multiline YAML keys (production
                    // 145); libyaml applies its single-line key restriction.
                    // Read the text in flow context, where a `...` line is
                    // content rather than a document end marker.
                    let noyalib::Value::Sequence(items) =
                        noyalib::from_str(&format!("[{text}]")).ok()?
                    else {
                        return None;
                    };
                    let [noyalib::Value::String(value)] = items.as_slice() else {
                        return None;
                    };
                    // A trailing break folds into one space, so the next
                    // token is not left on an under-indented line.
                    let quoted = serde_json::to_string(&value).ok()?;
                    if source[end..token_end].contains(['\n', '\r']) {
                        replacements.push((start, token_end, format!("{quoted} ")));
                    } else {
                        replacements.push((start, end, quoted));
                    }
                } else if text.starts_with(':') || compact_empty_value {
                    replacements.push((start, end, serde_json::to_string(text).ok()?));
                }
            }
            SyntaxKind::OpenBrace | SyntaxKind::OpenBracket => flow.push(kind),
            SyntaxKind::CloseBrace | SyntaxKind::CloseBracket => {
                let _ = flow.pop();
            }
            _ => {}
        }
        previous = Some(kind);
    }
    (!replacements.is_empty()).then(|| apply_replacements(source, replacements))
}

/// The default i64 model retains out-of-range radix literals as strings.
/// Only an exact u64 oracle value may match that spelling (ADR-0004).
// The compatibility fuzzer includes this module but opts into u64 resolution.
#[allow(dead_code)]
pub(crate) fn default_radix_value(source: &str) -> Option<u64> {
    let (digits, radix) = source
        .strip_prefix("0x")
        .map(|rest| (rest, 16))
        .or_else(|| source.strip_prefix("0o").map(|rest| (rest, 8)))?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let value = u64::from_str_radix(digits, radix).ok()?;
    (value > i64::MAX as u64).then_some(value)
}

fn lexical_tokens(source: &str) -> Option<Vec<Lexeme>> {
    fn walk(node: &GreenNode, offset: &mut usize, tokens: &mut Vec<Lexeme>) {
        for child in node.children() {
            match child {
                GreenChild::Node(node) => walk(node, offset, tokens),
                GreenChild::Token { kind, .. } => {
                    let end = *offset + child.text_len();
                    if end > *offset {
                        tokens.push((*kind, *offset, end));
                    }
                    *offset = end;
                }
            }
        }
    }
    // Tokens are lexical: merge semantics must not refuse a source the
    // compatibility profile reads with `<<` as an ordinary key.
    let config = noyalib::ParserConfig::new().merge_key_policy(noyalib::MergeKeyPolicy::AsOrdinary);
    let document = noyalib::cst::parse_document_with_config(source, &config).ok()?;
    let mut tokens = Vec::new();
    walk(document.syntax(), &mut 0, &mut tokens);
    Some(tokens)
}

fn apply_replacements(source: &str, replacements: Vec<Replacement>) -> String {
    let mut normalized = source.to_owned();
    for (start, end, replacement) in replacements.into_iter().rev() {
        normalized.replace_range(start..end, &replacement);
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_block_repair_reaches_past_node_properties() {
        // Found by fuzz_serde_yaml_compat on `&8\t>\r#`.
        for source in ["---\n>\n#", "--- &a >\n#"] {
            let normalized = indented_root_block(source).expect("root block repair");
            let before: serde_json::Value = noyalib::from_str(source).unwrap();
            let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
            assert_eq!(before, after, "{normalized:?}");
        }
        let source = "&8\t>\r#";
        let normalized = indented_root_block(source).expect("root block repair");
        let before: serde_json::Value = noyalib::from_str(source).unwrap();
        let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
        assert_eq!(before, after, "{normalized:?}");
    }

    #[test]
    fn anchor_repair_reads_tokens_under_a_scalar_merge_value() {
        // Found by fuzz_serde_yaml_compat: `<<: *a` naming a scalar is an
        // ordinary entry in the compatibility profile.
        let source = "t: &defaults \nd: &defaults\n b\no:\n  <<: *defaults\n  p: &d";
        let normalized = unique_colliding_anchors(source).expect("anchor ID collision");
        assert_eq!(
            normalized,
            "t: &noya_oracle_0 \nd: &noya_oracle_1\n b\no:\n  <<: *noya_oracle_1\n  p: &noya_oracle_2"
        );
    }

    #[test]
    fn quoted_multiline_flow_value_keeps_comment_separation() {
        // Found by fuzz_diff on `{&d,~:\n2\n~\n#\n}`.
        let source = "{&d,~:\n2\n~\n#\n}";
        let normalized = quoted_empty_flow_keys(source).unwrap();
        assert_eq!(normalized, "{&d,~:\n\"2 ~\" #\n}");
        let before: serde_json::Value = noyalib::from_str(source).unwrap();
        let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
        assert_eq!(before, after);
        // Found by fuzz_serde_yaml_compat: the trailing break folds away.
        let source = "- {6400.0,0\r!\n,}";
        let normalized = quoted_empty_flow_keys(source).unwrap();
        assert_eq!(normalized, "- {6400.0,\"0 !\" ,}");
        let before: serde_json::Value = noyalib::from_str(source).unwrap();
        let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
        assert_eq!(before, after);
        // Found by fuzz_diff: a `...` continuation line is flow content.
        let source = "{? :e\n...}";
        let normalized = quoted_empty_flow_keys(source).unwrap();
        assert_eq!(normalized, "{? \":e ...\"}");
        let before: serde_json::Value = noyalib::from_str(source).unwrap();
        let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn stream_bom_repair_preserves_content_and_composes_block_repairs() {
        for source in ["\u{feff}name: test\n", "\u{feff}>\n>\n >\n "] {
            let normalized = stream_bom_prefix(source).unwrap();
            let before: serde_json::Value = noyalib::from_str(source).unwrap();
            let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
            assert_eq!(before, after);
        }
        for source in ["name: '\u{feff}'\n", "'\u{feff}'", "name: test\n"] {
            assert!(stream_bom_prefix(source).is_none(), "{source:?}");
        }
    }

    #[test]
    fn root_block_indentation_repair_preserves_alloc_values() {
        for source in [">\r- 74 \t", "|\ncontent\n", ">\r\nfirst\r\nsecond\r\n"] {
            let normalized = collection_spelling(source).expect("root block indentation");
            let before: serde_json::Value = noyalib::from_str(source).unwrap();
            let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
            assert_eq!(before, after);
        }
    }

    #[test]
    fn compact_omitted_key_repair_preserves_alloc_values() {
        for source in [
            "- :",
            "- : value\n",
            "- - : value\n",
            "- :\n- normal\n",
            "a: 1\r: value\n",
            ": value\n",
        ] {
            let normalized = collection_spelling(source).expect("compact omitted key");
            let before: serde_json::Value = noyalib::from_str(source).unwrap();
            let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
            assert_eq!(before, after);
        }
    }

    #[test]
    fn leading_stream_suffix_repair_preserves_alloc_values() {
        for source in ["...\nvalue\n", "...\n'...'\n", "...\n"] {
            let normalized = collection_spelling(source).expect("leading stream suffix");
            let before: noyalib::Value = noyalib::from_str(source).unwrap();
            let after: noyalib::Value = noyalib::from_str(&normalized).unwrap();
            assert_eq!(before, after);
        }
    }

    #[test]
    fn terminal_spaced_block_normalization_preserves_l24t_values() {
        for source in ["foo: |\n  x\n   ", "foo: >\n  x\n   "] {
            let normalized = terminated_spaced_block(source).expect("spaced terminal line");
            let before: noyalib::Value = noyalib::from_str(source).unwrap();
            assert_eq!(before["foo"].as_str(), Some("x\n \n"));
            let after: noyalib::Value = noyalib::from_str(&normalized).unwrap();
            assert_eq!(before, after);
            assert_eq!(normalized, format!("{source}\n"));
        }
        for source in ["foo: |\n  x", "foo: plain   ", "foo: |\n  x\n   \n"] {
            assert!(terminated_spaced_block(source).is_none(), "{source:?}");
        }
    }

    #[test]
    fn leading_colon_flow_plain_scalars_remain_compared() {
        let source = "{:m, other: [:value]}";
        let normalized = quoted_empty_flow_keys(source).expect("leading-colon flow scalar");
        let before: serde_json::Value = noyalib::from_str(source).unwrap();
        let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
        assert_eq!(before, after);
        assert_eq!(normalized, "{\":m\", other: [\":value\"]}");
        assert_eq!(before[":m"], serde_json::Value::Null);
        assert_eq!(before["other"][0], ":value");
    }

    #[test]
    fn multiline_flow_mapping_keys_remain_compared() {
        let source = include_str!("../fixtures/multiline-flow-oracle.yaml");
        let normalized = quoted_empty_flow_keys(source).expect("flow spelling repair");
        let before: serde_json::Value = noyalib::from_str(source).unwrap();
        let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
        assert_eq!(before, after);
        assert!(normalized.contains("\"1\\nab\""), "{normalized}");
        assert_eq!(before["1\nab"][0], 2);
        assert_eq!(before[":m"], serde_json::Value::Null);
    }

    #[test]
    fn default_radix_range_difference_is_exact_and_bounded() {
        for source in ["0x8000000000000000", "0o1000000000000000000000"] {
            assert_eq!(default_radix_value(source), Some(i64::MAX as u64 + 1));
            let value: noyalib::Value = noyalib::from_str(source).unwrap();
            assert_eq!(value.as_str(), Some(source));
        }
        assert_eq!(default_radix_value("0xffffffffffffffff"), Some(u64::MAX));
        for source in [
            "0x7fffffffffffffff",
            "0x10000000000000000",
            "0x+ffffffffffffffff",
            "-0xffffffffffffffff",
            "0Xffffffffffffffff",
            "0xffff_ffff_ffff_ffff",
            "18446744073709551615",
            "0b111",
            "0x",
            "0xgg",
        ] {
            assert_eq!(default_radix_value(source), None, "{source}");
        }
    }

    #[test]
    fn flow_plain_spelling_retains_internal_separation() {
        let source = "{:m  `1ab(: [2, 3, {c: 24}],dst: 1\n}\n";
        let normalized = quoted_empty_flow_keys(source).expect("flow spelling");
        let before: serde_json::Value = noyalib::from_str(source).unwrap();
        let after: serde_json::Value = noyalib::from_str(&normalized).unwrap_or_else(|error| {
            panic!("{error}; {normalized:?}; {:?}", lexical_tokens(source))
        });
        assert_eq!(before, after, "{normalized:?}");
    }

    #[test]
    fn flow_plain_continuation_can_start_with_quotes() {
        let source = "{\n4\n,? :e&\n\n''}\n";
        let normalized = quoted_empty_flow_keys(source)
            .unwrap_or_else(|| panic!("{:?}", lexical_tokens(source)));
        let before: serde_json::Value = noyalib::from_str(source).unwrap();
        let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
        assert_eq!(before, after, "{normalized:?}");
    }

    #[test]
    fn sequence_separator_tabs_do_not_change_scalar_content() {
        for source in ["-\t) ", "items:\n  -\t'value\tkept'\n"] {
            let normalized = spaced_sequence_tabs(source).expect("inline sequence separator");
            let before: serde_json::Value = noyalib::from_str(source).unwrap();
            let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
            assert_eq!(before, after);
            if source.starts_with("items:") {
                assert_eq!(after["items"][0], "value\tkept");
            }
        }
        for source in ["text: '-\tquoted'\n", "text: |\n  -\tblock\n"] {
            assert!(spaced_sequence_tabs(source).is_none());
        }
    }

    #[test]
    fn sequence_and_flow_repairs_preserve_the_complete_value_together() {
        let source = "-\t{:m}\n";
        let normalized = collection_spelling(source).expect("both collection spellings");
        assert_eq!(normalized, "- {\":m\"}\n");
        let before: serde_json::Value = noyalib::from_str(source).unwrap();
        let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn prefix_and_empty_line_tabs_preserve_the_complete_value() {
        for source in [
            "\u{feff}\tC",
            "\u{feff}\t'quoted'\n",
            "\u{feff}\t{a: b}\n",
            "- /\r\t\t",
            "- /\n\t\t",
            "- /\r\n\t\t",
            "-  \r\t\t",
            "-  \n\t\t",
            "-  \r\n\t\t",
        ] {
            let normalized = collection_spelling(source).expect("empty-line separation");
            assert_eq!(normalized, source.replace('\t', " "));
            let before: serde_json::Value = noyalib::from_str(source).unwrap();
            let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
            assert_eq!(before, after);
        }
        for source in ["text: 'value\tkept'\n", "text: |\n  \t\n"] {
            assert!(collection_spelling(source).is_none(), "{source:?}");
        }
    }

    #[test]
    fn compact_empty_flow_values_remain_compared() {
        for (source, expected) in [
            ("{a:, other: 1}", "{\"a\":, other: 1}"),
            ("{a:}", "{\"a\":}"),
            ("[d:]", "[\"d\":]"),
        ] {
            let normalized = quoted_empty_flow_keys(source).expect("compact empty value");
            assert_eq!(normalized, expected);
            let before: serde_json::Value = noyalib::from_str(source).unwrap();
            let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
            assert_eq!(before, after);
            assert_eq!(before["a"], serde_json::Value::Null);
        }
    }

    #[test]
    fn later_id_collision_does_not_change_an_earlier_alias() {
        let source = "first: &x old\nsecond: &x new\ncopy: *x\nlast: &y leaf\n";
        let normalized = unique_colliding_anchors(source).expect("later collision affects alias");
        let before: noyalib::Value = noyalib::from_str(source).unwrap();
        let after: noyalib::Value = noyalib::from_str(&normalized).unwrap();
        assert_eq!(before, after);
        assert_eq!(before["copy"].as_str(), Some("new"));
    }

    #[test]
    fn quoted_indent_oracle_difference_requires_the_specific_scanner_error() {
        for style in ["single-quoted", "double-quoted"] {
            assert!(quoted_indent_rejection(&format!(
                "{style} continuation must be indented more than the parent block at line 4 column 1"
            )));
        }
        for unrelated in [
            "unterminated single-quoted scalar at line 4 column 1",
            "single-quoted scalar contains an invalid character at line 4 column 1",
            "an unrelated parse error",
        ] {
            assert!(!quoted_indent_rejection(unrelated));
        }
    }

    #[test]
    fn unseparated_comment_archive_difference_is_specific() {
        let error = noyalib::from_str::<noyalib::Value>("key: 'can'#t stop'\n")
            .expect_err("a comment after a quoted scalar requires separation");
        assert!(error.to_string().contains("comment indicator '#'"));
        assert!(missing_comment_separator_rejection(
            "comment indicator '#' must be preceded by a space, tab, or line break at line 1 column 11"
        ));
        #[cfg(feature = "compat-serde-yaml")]
        {
            let error =
                noyalib::compat::serde_yaml::from_str::<serde_json::Value>("key: 'can'#t stop'\n")
                    .unwrap_err();
            assert!(missing_comment_separator_rejection(&error.to_string()));
        }
        assert!(!missing_comment_separator_rejection(
            "unterminated single-quoted scalar at line 1 column 1"
        ));
        let value: noyalib::Value = noyalib::from_str("key: 'can#t stop'\n").unwrap();
        assert_eq!(value["key"].as_str(), Some("can#t stop"));
    }

    #[test]
    fn original_under_indented_quote_remains_rejected() {
        let source = include_str!("../fixtures/quoted-continuation-indent.yaml");
        assert!(noyalib::from_str::<noyalib::Value>(source).is_err());
        #[cfg(feature = "compat-serde-yaml")]
        {
            let error = noyalib::compat::serde_yaml::from_str::<serde_json::Value>(source)
                .expect_err("continuation must be indented beyond the containing block");
            assert!(quoted_indent_rejection(&error.to_string()));
        }
    }

    #[test]
    fn archive_anchor_id_collision_is_repaired_without_changing_values() {
        let source = "first: &x old\nsecond: &x {nested: &y leaf}\ncopy: *x\n";
        let normalized = unique_colliding_anchors(source).expect("colliding archive IDs");
        assert_eq!(
            normalized,
            "first: &noya_oracle_0 old\nsecond: &noya_oracle_1 {nested: &noya_oracle_2 leaf}\ncopy: *noya_oracle_1\n"
        );
        let before: noyalib::Value = noyalib::from_str(source).unwrap();
        let after: noyalib::Value = noyalib::from_str(&normalized).unwrap();
        assert_eq!(before, after);
        assert_eq!(before["copy"], before["second"]);
    }

    #[test]
    fn noncolliding_and_same_name_nested_anchors_are_not_excluded() {
        for source in [
            "a: &x {nested: &y leaf}\nb: *x\n",
            "a: &x [&x inner]\nb: *x\n",
            "text: '&x &x *x'\na: &x leaf\nb: *x\n",
            "a: &x old\nb: &x new\nc: &y leaf\nd: *y\n",
        ] {
            assert_eq!(unique_colliding_anchors(source), None, "{source}");
        }
    }

    #[test]
    fn omitted_flow_keys_preserve_the_string_key_projection() {
        for (source, expected) in [
            ("{: null}", "{\"\": null}"),
            ("{x: { : foo}, : bar}", "{x: { \"\": foo}, \"\": bar}"),
            ("[: foo]", "[\"\": foo]"),
        ] {
            let normalized = quoted_empty_flow_keys(source).expect("omitted key");
            assert_eq!(normalized, expected);
            let before: serde_json::Value = noyalib::from_str(source).unwrap();
            let after: serde_json::Value = noyalib::from_str(&normalized).unwrap();
            assert_eq!(before, after);
        }
    }

    #[test]
    fn quoted_comment_and_block_scalar_text_is_untouched() {
        for source in [
            "{s: '{: not a key}', x: 'a,:b'}",
            "a: |\n  {: x}\n",
            "a: value # {: x}\n",
            "{a:b}",
        ] {
            assert_eq!(quoted_empty_flow_keys(source), None, "{source}");
        }
    }
}
