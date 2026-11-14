// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Adapt the archived JSON-key contract to ADR-0019 without dropping comparisons.

use noyalib::cst::{GreenChild, GreenNode, SyntaxKind};

type Token = (SyntaxKind, usize, usize);
type Edit = (usize, usize, String);
/// An entry's tokens, its end boundary, and whether it is a flow entry.
type Entry = (Vec<Token>, usize, bool);

struct Key {
    canonical: String,
    identity: noyalib::Value,
    start: usize,
    end: usize,
    spelling: String,
    /// An empty key gains a separating space so it cannot fuse with
    /// a preceding anchor (`&d,` must become `&d "~",`).
    empty: bool,
    /// A flow key whose token spans a line break is replaced with that
    /// separation folded to one space, so the next token cannot start an
    /// under-indented line after the quoted spelling.
    tail: &'static str,
}

/// Change only scalar keys in a repeated canonical group to the last
/// JSON-key spelling. Callers must prove the entire candidate projection
/// unchanged, then compare it with a fresh independent archive parse.
pub(crate) fn spelling(source: &str, config: &noyalib::ParserConfig) -> Option<String> {
    let prepared = crate::oracle_normalization::collection_spelling(source);
    let source = prepared.as_deref().unwrap_or(source);
    let document = noyalib::cst::parse_document_with_config(source, config).ok()?;
    let mut edits = Vec::new();
    walk(document.syntax(), 0, &mut |entries| {
        let keys = entries
            .iter()
            .filter_map(|(tokens, boundary, flow)| key(tokens, *boundary, *flow, source, config))
            .collect();
        group(keys, &mut edits);
    });
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

/// True when two distinct keys of one mapping share a JSON-key spelling
/// (`{? , !}`: a null key and the empty string both become ""). The JSON
/// projection then depends on insertion order after ADR-0019 resolution,
/// not on parsing, so no archive comparison applies.
pub(crate) fn spelling_collision(source: &str, config: &noyalib::ParserConfig) -> bool {
    let Ok(document) = noyalib::cst::parse_document_with_config(source, config) else {
        return false;
    };
    let mut collision = false;
    walk(document.syntax(), 0, &mut |entries| {
        let keys: Vec<_> = entries
            .iter()
            .filter_map(|(tokens, boundary, _)| written_key(tokens, *boundary, source, config))
            .collect();
        collision |= keys.iter().enumerate().any(|(i, (canonical, spelling))| {
            keys[i + 1..]
                .iter()
                .any(|(other, spelled)| spelled == spelling && other != canonical)
        });
    });
    collision
}

/// The canonical `Value` key and JSON-key spelling of an entry's whole
/// key text, properties included.
fn written_key(
    tokens: &[Token],
    boundary: usize,
    source: &str,
    config: &noyalib::ParserConfig,
) -> Option<(noyalib::Value, String)> {
    let significant = |t: &&Token| {
        !matches!(
            t.0,
            SyntaxKind::Whitespace
                | SyntaxKind::Newline
                | SyntaxKind::Comment
                | SyntaxKind::QuestionIndicator
        )
    };
    let colon = tokens.iter().find(|t| t.0 == SyntaxKind::ColonIndicator);
    let first = tokens.iter().find(significant)?;
    if colon.is_none() && first.0 == SyntaxKind::Document {
        return None;
    }
    let end = colon.map_or(boundary, |t| t.1);
    let start = if Some(first) == colon { end } else { first.1 };
    let raw = source.get(start..end)?.trim();
    let wrapped = format!("{{{raw} : 0}}");
    let resolved: noyalib::Value = noyalib::from_str_with_config(&wrapped, config).ok()?;
    let (canonical, _) = resolved.as_mapping()?.iter().next()?;
    let map: serde_json::Value = noyalib::from_str_with_config(&wrapped, config).ok()?;
    let spelling = map.as_object()?.keys().next()?.clone();
    Some((noyalib::Value::from(canonical.clone()), spelling))
}

fn key(
    tokens: &[Token],
    boundary: usize,
    flow: bool,
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
            // An anchor names the node without changing its identity;
            // the caller's re-parse rejects a rewrite an alias observes.
            SyntaxKind::Whitespace
            | SyntaxKind::Newline
            | SyntaxKind::Comment
            | SyntaxKind::QuestionIndicator
            | SyntaxKind::AnchorMark => {}
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
    // An empty key sits right after its `?` or anchor, else before its
    // colon: a colon or block boundary may lie past a line break.
    let colon = tokens
        .iter()
        .position(|t| t.0 == SyntaxKind::ColonIndicator);
    let empty_at = tokens[..colon.unwrap_or(tokens.len())]
        .iter()
        .rev()
        .find(|t| matches!(t.0, SyntaxKind::QuestionIndicator | SyntaxKind::AnchorMark))
        .map(|token| token.2)
        .or_else(|| colon.map(|at| tokens[at].1))
        .unwrap_or(boundary);
    let (start, end) = scalar.unwrap_or((empty_at, empty_at));
    // A plain token can span its trailing break; keep that separator
    // so a following comment stays preceded by whitespace.
    let full = source.get(start..end)?;
    let raw = full.trim_end();
    let folds = flow && scalar.is_some() && full[raw.len()..].contains(['\n', '\r']);
    let (end, tail) = if folds {
        (end, " ")
    } else {
        (start + raw.len(), "")
    };
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
        empty: scalar.is_none(),
        tail,
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
            edits.extend(duplicates.into_iter().map(|previous| {
                let space = if previous.empty { " " } else { "" };
                (
                    previous.start,
                    previous.end,
                    format!("{space}{replacement}{}", previous.tail),
                )
            }));
        }
    }
}

/// Hand each mapping's entries to `visit`.
fn walk(node: &GreenNode, offset: usize, visit: &mut dyn FnMut(&[Entry])) {
    let mut position = offset;
    let mut tokens = Vec::new();
    let mut entries = Vec::new();
    for child in node.children() {
        let end = position + child.text_len();
        match child {
            GreenChild::Node(child) => {
                if node.kind() == SyntaxKind::BlockMapping
                    && child.kind() == SyntaxKind::MappingEntry
                {
                    entries.push((entry_tokens(child, position), end, false));
                }
                tokens.push((SyntaxKind::Document, position, end));
                walk(child, position, visit);
            }
            GreenChild::Token { kind, .. } if node.kind() == SyntaxKind::FlowMapping => {
                match kind {
                    SyntaxKind::OpenBrace => {}
                    SyntaxKind::Comma | SyntaxKind::CloseBrace => {
                        entries.push((core::mem::take(&mut tokens), position, true));
                    }
                    _ => tokens.push((*kind, position, end)),
                }
            }
            _ => {}
        }
        position = end;
    }
    visit(&entries);
}

fn entry_tokens(entry: &GreenNode, mut at: usize) -> Vec<Token> {
    entry
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
        .collect()
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
    fn anchored_null_duplicates_keep_their_anchor() {
        check(include_str!(
            "regressions/fuzz_diff/crash-b1e46c04a8525d9cba6b58a02c5b12a9a5fdfabf"
        ));
        check("{&d,~}");
        check("{&d : 1, ~: 2}");
        check("&d null: 1\n~: 2\n");
        check("{&d,~\n#\n}");
        check("{&d,~:\n2\n~\n#\n}");
        check("? ~\n?\n? ");
        check("{? &y,~}");
        check("{~\n,&d}");
        // A duplicate only where `06` resolves as the integer 6.
        let config = ParserConfig::default();
        let normalized = spelling("{6\n,06}", &config).expect("default duplicates");
        assert_eq!(normalized, "{\"06\" ,\"06\"}");
        let oracle: serde_json::Value = serde_yaml_ng::from_str(&normalized).unwrap();
        assert_eq!(oracle, serde_json::json!({"06": null}));
        check("? &a\n? ~\n");
        check("? ~\r?\r:");
    }

    #[test]
    fn distinct_keys_sharing_a_json_spelling_are_reported() {
        let config = ParserConfig::default();
        for source in [
            "{? ,!,?\n:\n2}",
            "{!1:,&d : 0,!1:,}",
            "{!, : 1}",
            "! : 1\n: 2\n",
        ] {
            assert!(super::spelling_collision(source, &config), "{source:?}");
        }
        for source in [
            "{~: 1, null: 2}",
            "{a: 1, b: 2}",
            "{&d,~}",
            "{!,~}",
            "{'': 1, b: 2}",
        ] {
            assert!(!super::spelling_collision(source, &config), "{source:?}");
        }
    }

    #[test]
    fn compat_float_duplicates_beside_a_tagged_key() {
        let config = ParserConfig::serde_yaml_compat();
        let source = "{04,.0,+.0,!\t4}";
        let normalized = spelling(source, &config);
        let normalized = normalized.expect("canonical duplicates");
        let before: serde_json::Value = noyalib::compat::serde_yaml::from_str(source).unwrap();
        let after: serde_json::Value = noyalib::compat::serde_yaml::from_str(&normalized).unwrap();
        assert_eq!(before, after);
        let oracle: serde_json::Value = serde_yaml::from_str(&normalized).unwrap();
        assert_eq!(before, oracle, "{normalized:?}");
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
