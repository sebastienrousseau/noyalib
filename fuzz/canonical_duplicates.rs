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
    /// The key's plain one-line text. Quoting keeps the archive reading
    /// flow keys (`{d:,}`), but an alias elsewhere observes an anchored
    /// key's type; duplicates share a canonical value, so in a group with
    /// an anchored key the others take the last key's plain text instead.
    plain: Option<String>,
    anchored: bool,
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
            .filter_map(|(tokens, boundary, flow)| {
                written_key(tokens, *boundary, *flow, source, config)
            })
            .collect();
        collision |= keys.iter().enumerate().any(|(i, (canonical, spelling))| {
            keys[i + 1..]
                .iter()
                .any(|(other, spelled)| spelled == spelling && other != canonical)
        });
    });
    collision
}

/// Apply every repair in turn, keeping each only when `keeps` proves the
/// candidate's value unchanged, so independent oracle defects compose
/// (an anchor-ID collision beside duplicate keys or an L24T block).
pub(crate) fn composed(
    source: &str,
    config: &noyalib::ParserConfig,
    keeps: &dyn Fn(&str) -> bool,
) -> Option<String> {
    use crate::oracle_normalization as repair;
    let steps: [&dyn Fn(&str) -> Option<String>; 5] = [
        &repair::stream_bom_prefix,
        &repair::collection_spelling,
        &repair::terminated_spaced_block,
        &repair::unique_colliding_anchors,
        &|current| spelling(current, config),
    ];
    let mut current = source.to_owned();
    let mut changed = false;
    for step in steps {
        if let Some(next) = step(&current).filter(|next| keeps(next)) {
            current = next;
            changed = true;
        }
    }
    changed.then_some(current)
}

/// True when a mapping key carries a tag (`{!!float 0}`). Typed targets
/// restore the written spelling only for bare plain keys
/// (`is_canonicalised_plain_key`); a tagged key keeps its canonical
/// spelling ("0.0"), where the archive keeps the written text ("0").
pub(crate) fn tagged_key(source: &str, config: &noyalib::ParserConfig) -> bool {
    let Ok(document) = noyalib::cst::parse_document_with_config(source, config) else {
        return false;
    };
    let mut tagged = false;
    walk(document.syntax(), 0, &mut |entries| {
        tagged |= entries.iter().any(|(tokens, _, _)| {
            tokens
                .iter()
                .take_while(|t| t.0 != SyntaxKind::ColonIndicator)
                .any(|t| t.0 == SyntaxKind::TagMark)
        });
    });
    tagged
}

/// True when a mapping has duplicate keys and one of its keys is an
/// alias (`{&a x, *a}`): the repair cannot respell an alias, so no
/// archive comparison applies.
pub(crate) fn aliased_duplicate(source: &str, config: &noyalib::ParserConfig) -> bool {
    let mut strict = config.clone();
    strict.duplicate_key_policy = noyalib::DuplicateKeyPolicy::Error;
    if noyalib::from_str_with_config::<noyalib::Value>(source, &strict).is_ok() {
        return false;
    }
    let Ok(document) = noyalib::cst::parse_document_with_config(source, config) else {
        return false;
    };
    let mut aliased = false;
    walk(document.syntax(), 0, &mut |entries| {
        aliased |= entries.iter().any(|(tokens, _, _)| {
            tokens
                .iter()
                .take_while(|t| t.0 != SyntaxKind::ColonIndicator)
                .any(|t| t.0 == SyntaxKind::AliasMark)
        });
    });
    aliased
}

/// The canonical `Value` key and JSON-key spelling of an entry's whole
/// key text, properties included.
fn written_key(
    tokens: &[Token],
    boundary: usize,
    flow: bool,
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
    let explicit = tokens.iter().any(|t| t.0 == SyntaxKind::QuestionIndicator);
    // Without a colon the key ends with its last significant token, so a
    // trailing comment (`{!\n#\n}`) stays out of the key text.
    let end = colon.map_or_else(
        || {
            tokens
                .iter()
                .rev()
                .find(significant)
                .map_or(boundary, |t| t.2)
        },
        |t| t.1,
    );
    // An empty explicit key (`? `) has no significant token at all.
    let start = match tokens.iter().find(significant) {
        Some(first) if colon.is_none() && first.0 == SyntaxKind::Document => return None,
        Some(first) if Some(first) != colon => first.1,
        Some(_) => end,
        None if explicit => end,
        None => return None,
    };
    let raw = source.get(start..end)?.trim();
    // A block scalar key cannot sit in a flow wrapper; its value is a
    // string, spelled as itself.
    if raw.starts_with(['|', '>']) {
        let value: noyalib::Value = noyalib::from_str_with_config(raw, config).ok()?;
        let spelling = value.as_str()?.to_owned();
        return Some((value, spelling));
    }
    let wrapped = if flow {
        format!("{{{raw} : 0}}")
    } else {
        format!("{raw} : 0\n")
    };
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
    let mut plain_token = false;
    let mut anchored = false;
    for &(kind, start, end) in tokens {
        match kind {
            SyntaxKind::ColonIndicator => break,
            // An anchor names the node without changing its identity;
            // the caller's re-parse rejects a rewrite an alias observes.
            SyntaxKind::Whitespace
            | SyntaxKind::Newline
            | SyntaxKind::Comment
            | SyntaxKind::QuestionIndicator => {}
            SyntaxKind::AnchorMark => anchored = true,
            SyntaxKind::PlainScalar
            | SyntaxKind::SingleQuotedScalar
            | SyntaxKind::DoubleQuotedScalar
                if scalar.is_none() =>
            {
                scalar = Some((start, end));
                plain_token = kind == SyntaxKind::PlainScalar;
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
    // A block key may hold flow indicators (`9,~0 ,ault`); read it in
    // its own context.
    let wrapped = if flow {
        format!("{{{raw}: 0}}")
    } else {
        format!("{raw}: 0\n")
    };
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
        plain: (plain_token && !raw.is_empty() && !raw.contains(['\n', '\r']))
            .then(|| raw.to_owned()),
        anchored,
    })
}

/// A YAML double-quoted spelling. JSON escapes cover C0 controls but
/// leave NEL, LS, PS and the rest of C1 raw, which libyaml reads as
/// line breaks or refuses; escape those the YAML way.
fn quoted(spelling: &str) -> String {
    let json = serde_json::to_string(spelling).expect("string serialization");
    let mut out = String::with_capacity(json.len());
    for c in json.chars() {
        match c {
            '\u{85}' => out.push_str("\\N"),
            '\u{2028}' => out.push_str("\\L"),
            '\u{2029}' => out.push_str("\\P"),
            '\u{80}'..='\u{9f}' => out.push_str(&format!("\\x{:02x}", c as u32)),
            _ => out.push(c),
        }
    }
    out
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
            // With an anchored key in the group, reuse the last key's plain
            // text; it keeps the type an alias of the anchored key observes.
            let reuse = duplicates.iter().any(|d| d.anchored) && key.plain.is_some();
            let replacement = if reuse {
                key.plain.clone().unwrap_or_default()
            } else {
                quoted(&key.spelling)
            };
            let keep_last = reuse;
            edits.extend(
                duplicates
                    .into_iter()
                    .filter(|previous| !(keep_last && core::ptr::eq(*previous, key)))
                    .map(|previous| {
                        let space = if previous.empty { " " } else { "" };
                        (
                            previous.start,
                            previous.end,
                            format!("{space}{replacement}{}", previous.tail),
                        )
                    }),
            );
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
        check("n: {0,&defaults +0}\ne: *defaults");
        check("d: {&defaults +0,0}\ne: *defaults");
        check("{~,d:,&e,d}");
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
    fn repairs_compose_across_independent_defects() {
        let config = ParserConfig::default();
        let keeps = |source: &str| {
            let before: serde_json::Value =
                noyalib::from_str_with_config("? ~\n?\n9,~0 ,ault:\n9,~0 ,ault:", &config).unwrap();
            noyalib::from_str_with_config::<serde_json::Value>(source, &config)
                .is_ok_and(|after| after == before)
        };
        let normalized =
            super::composed("? ~\n?\n9,~0 ,ault:\n9,~0 ,ault:", &config, &keeps).unwrap();
        let oracle: serde_json::Value = serde_yaml_ng::from_str(&normalized).unwrap();
        assert!(keeps(&normalized));
        let before: serde_json::Value =
            noyalib::from_str_with_config("? ~\n?\n9,~0 ,ault:\n9,~0 ,ault:", &config).unwrap();
        assert_eq!(before, oracle, "{normalized:?}");
    }

    #[test]
    fn respelled_keys_escape_yaml_line_breaks() {
        assert_eq!(
            super::quoted("a\u{85}b\u{2028}\u{9f}\u{1b}"),
            r#""a\Nb\L\x9f\u001b""#
        );
        check("{0, +0, \"x\\Ny\", \"x\\Ny\"}");
    }

    #[test]
    fn tagged_keys_are_reported() {
        let config = ParserConfig::default();
        assert!(super::tagged_key("{!!float 0}", &config));
        assert!(super::tagged_key("!t k: 1\n", &config));
        assert!(!super::tagged_key("k: !t 1\n", &config));
        assert!(!super::tagged_key("{a: !!float 0}", &config));
    }

    #[test]
    fn duplicate_groups_with_an_alias_key_are_reported() {
        let config = ParserConfig::default();
        assert!(super::aliased_duplicate(
            "{&d,&d,? *d\n:\n2,&2 02,2}",
            &config
        ));
        assert!(super::aliased_duplicate("{&a x: 1, *a : 2}", &config));
        assert!(!super::aliased_duplicate("{&a x: 1, y: *a}", &config));
        assert!(!super::aliased_duplicate("{~: 1, null: 2}", &config));
    }

    #[test]
    fn distinct_keys_sharing_a_json_spelling_are_reported() {
        let config = ParserConfig::default();
        for source in [
            "{? ,!,?\n:\n2}",
            "{!1:,&d : 0,!1:,}",
            "{!, : 1}",
            "! : 1\n: 2\n",
            "{!,~,? }",
            "{~,&d,!\n#\n}",
            "?\r? >\r?\r: 4",
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
