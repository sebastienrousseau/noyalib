// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Narrow, semantics-checked repairs for known development-oracle defects.

use noyalib::cst::{GreenChild, GreenNode, SyntaxKind};
use std::collections::BTreeMap;

type Lexeme = (SyntaxKind, usize, usize);
type Replacement = (usize, usize, String);

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

/// Quote omitted flow keys for a string-keyed JSON comparison. Callers
/// must reparse with the oracle and compare the entire resulting value.
pub(crate) fn quoted_empty_flow_keys(source: &str) -> Option<String> {
    let tokens = lexical_tokens(source)?;
    let mut previous = None;
    let mut flow = Vec::new();
    let mut replacements = Vec::new();
    for (kind, start, _) in tokens {
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
    let document = noyalib::cst::parse_document(source).ok()?;
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
