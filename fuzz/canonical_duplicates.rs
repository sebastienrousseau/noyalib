// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Adapt the archived JSON-key contract to ADR-0019 without dropping comparisons.

use noyalib::cst::{GreenChild, GreenNode, SyntaxKind};

type Token = (SyntaxKind, usize, usize);
type Edit = (usize, usize, String);

struct Key {
    canonical: String,
    identity: noyalib::Value,
    start: usize,
    end: usize,
    spelling: String,
}

/// Change only scalar keys in a repeated canonical group to the last
/// JSON-key spelling. Callers must prove the entire candidate projection
/// unchanged, then compare it with a fresh independent archive parse.
pub(crate) fn spelling(source: &str, config: &noyalib::ParserConfig) -> Option<String> {
    let prepared = crate::oracle_normalization::collection_spelling(source);
    let source = prepared.as_deref().unwrap_or(source);
    let document = noyalib::cst::parse_document(source).ok()?;
    let mut edits = Vec::new();
    walk(document.syntax(), 0, source, config, &mut edits);
    if edits.is_empty() {
        return None;
    }
    edits.sort_by_key(|edit| edit.0);
    let mut normalized = source.to_owned();
    for (start, end, replacement) in edits.into_iter().rev() {
        normalized.replace_range(start..end, &replacement);
    }
    Some(normalized)
}

fn key(
    tokens: &[Token],
    boundary: usize,
    source: &str,
    config: &noyalib::ParserConfig,
) -> Option<Key> {
    if tokens.iter().all(|token| {
        matches!(
            token.0,
            SyntaxKind::Whitespace | SyntaxKind::Newline | SyntaxKind::Comment
        )
    }) {
        return None;
    }
    let mut scalar = None;
    for &(kind, start, end) in tokens {
        match kind {
            SyntaxKind::ColonIndicator => break,
            SyntaxKind::Whitespace
            | SyntaxKind::Newline
            | SyntaxKind::Comment
            | SyntaxKind::QuestionIndicator => {}
            SyntaxKind::PlainScalar
            | SyntaxKind::SingleQuotedScalar
            | SyntaxKind::DoubleQuotedScalar
                if scalar.is_none() =>
            {
                scalar = Some((start, end))
            }
            _ => return None,
        }
    }
    let empty_at = tokens
        .iter()
        .find(|t| t.0 == SyntaxKind::ColonIndicator)
        .map_or(boundary, |token| token.1);
    let (start, end) = scalar.unwrap_or((empty_at, empty_at));
    let raw = source.get(start..end)?;
    if raw.trim() == "<<" {
        return None;
    }
    let identity: noyalib::Value = noyalib::from_str_with_config(raw, config).ok()?;
    let wrapped = format!("{{{raw}: 0}}");
    let resolved: noyalib::Value = noyalib::from_str_with_config(&wrapped, config).ok()?;
    let canonical = resolved.as_mapping()?.keys().next()?.clone();
    let map: serde_json::Value = noyalib::from_str_with_config(&wrapped, config).ok()?;
    let spelling = map.as_object()?.keys().next()?.clone();
    Some(Key {
        canonical,
        identity,
        start,
        end,
        spelling,
    })
}

fn group(keys: Vec<Key>, edits: &mut Vec<Edit>) {
    for (last, key) in keys.iter().enumerate() {
        if keys[last + 1..]
            .iter()
            .any(|next| next.canonical == key.canonical && next.identity == key.identity)
        {
            continue;
        }
        let duplicates: Vec<_> = keys[..=last]
            .iter()
            .filter(|previous| {
                previous.canonical == key.canonical && previous.identity == key.identity
            })
            .collect();
        if duplicates.len() > 1 {
            let replacement = serde_json::to_string(&key.spelling).expect("string serialization");
            edits.extend(
                duplicates
                    .into_iter()
                    .map(|previous| (previous.start, previous.end, replacement.clone())),
            );
        }
    }
}

fn walk(
    node: &GreenNode,
    offset: usize,
    source: &str,
    config: &noyalib::ParserConfig,
    edits: &mut Vec<Edit>,
) {
    let mut position = offset;
    let mut tokens = Vec::new();
    let mut keys = Vec::new();
    for child in node.children() {
        let end = position + child.text_len();
        match child {
            GreenChild::Node(child) => {
                if node.kind() == SyntaxKind::BlockMapping
                    && child.kind() == SyntaxKind::MappingEntry
                {
                    let mut at = position;
                    let entry: Vec<_> = child
                        .children()
                        .map(|part| {
                            let end = at + part.text_len();
                            let kind = match part {
                                GreenChild::Token { kind, .. } => *kind,
                                _ => SyntaxKind::Document,
                            };
                            let token = (kind, at, end);
                            at = end;
                            token
                        })
                        .collect();
                    keys.extend(key(&entry, end, source, config));
                }
                tokens.push((SyntaxKind::Document, position, end));
                walk(child, position, source, config, edits);
            }
            GreenChild::Token { kind, .. } if node.kind() == SyntaxKind::FlowMapping => {
                match kind {
                    SyntaxKind::OpenBrace => {}
                    SyntaxKind::Comma | SyntaxKind::CloseBrace => {
                        keys.extend(key(&tokens, position, source, config));
                        tokens.clear();
                    }
                    _ => tokens.push((*kind, position, end)),
                }
            }
            _ => {}
        }
        position = end;
    }
    group(keys, edits);
}

#[cfg(test)]
mod tests {
    use super::spelling;
    use noyalib::ParserConfig;

    fn check(source: &str) {
        for config in [ParserConfig::default(), ParserConfig::serde_yaml_compat()] {
            let before: serde_json::Value = noyalib::from_str_with_config(source, &config).unwrap();
            let normalized = spelling(source, &config).expect("canonical duplicates");
            let after: serde_json::Value =
                noyalib::from_str_with_config(&normalized, &config).unwrap();
            assert_eq!(before, after, "{normalized:?}");
            let oracle: serde_json::Value = serde_yaml_ng::from_str(&normalized)
                .unwrap_or_else(|error| panic!("{error}; {normalized:?}"));
            assert_eq!(before, oracle, "{normalized:?}");
            let oracle: serde_json::Value = serde_yaml::from_str(&normalized).unwrap();
            assert_eq!(before, oracle, "{normalized:?}");
        }
    }

    #[test]
    fn original_finding_and_minimized_null_duplicates_keep_all_values() {
        check(include_str!(
            "../crates/noyalib/tests/fixtures/canonical-duplicate-identity.bin"
        ));
        check("{~: 1, ? }");
        check("null: 1\n~: 2\nsibling: [7, {other: 9}]\n");
    }

    #[test]
    fn numeric_boolean_and_nested_duplicate_keys_use_last_spelling() {
        for pair in [
            "True: 1, true: 2",
            "0x1F: 1, 31: 2",
            "+1: 1, 1: 2",
            "1.0: 1, 1e0: 2",
        ] {
            check(&format!("outer: {{{pair}, sibling: [3, 4]}}\nafter: 5\n"));
        }
    }

    #[test]
    fn unique_quoted_tagged_and_cross_mapping_keys_are_not_rewritten() {
        for source in [
            "{~: 1}",
            "{~: 1,}",
            "{0.0: 1, -0.0: 2}",
            "{empty: {}}",
            "{'~': 1, 'null': 2}",
            "{a: {~: 1}, b: {null: 2}}",
            "{!t null: 1, null: 2}",
        ] {
            assert!(
                spelling(source, &ParserConfig::default()).is_none(),
                "{source:?}"
            );
        }
    }
}
