// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reject only the archive's proven unquoted-content BOM acceptance.

/// YAML 1.2.2 section 5.2 forbids a BOM inside unquoted document content.
/// Keep possible document prefixes and quoted content outside this classifier.
#[allow(dead_code)]
pub(crate) fn unquoted_content_bom_rejection(source: &str, error: &noyalib::Error) -> bool {
    let noyalib::Error::ParseWithLocation { message, location } = error else {
        return false;
    };
    if message
        != "byte order mark inside the stream \u{2014} a BOM is only allowed at the very start"
    {
        return false;
    }
    let offset = location.index();
    if !source
        .get(offset..)
        .is_some_and(|rest| rest.starts_with('\u{feff}'))
    {
        return false;
    }
    let prefix = &source[..offset];
    let line = prefix.rsplit(['\r', '\n']).next().unwrap_or_default();
    // A stream-leading BOM must precede horizontal separation (section 9.1.1).
    // A later document prefix remains outside this rule. Parsing the prefix
    // also keeps unclosed quoted content outside the classifier.
    let displaced_start = !prefix.is_empty() && prefix.bytes().all(|b| matches!(b, b' ' | b'\t'));
    // A BOM straight after the stream BOM is document content (section 5.2).
    let repeated_start = !prefix.is_empty() && prefix.chars().all(|c| c == '\u{feff}');
    // Documented policy (v0.0.29, 612fc07): the scanner rejects
    // every BOM after the stream start, including one opening a later
    // document prefix after blank or comment lines, which section 5.2 and
    // l-yaml-stream would allow. libyaml skips it.
    let later_prefix = line.is_empty()
        && !prefix.is_empty()
        && prefix.split(['\r', '\n']).all(|l| {
            let l = l.trim_start_matches('\u{feff}').trim();
            l.is_empty() || l.starts_with('#')
        });
    if later_prefix {
        return true;
    }
    (displaced_start
        || repeated_start
        || line.chars().any(|c| !matches!(c, ' ' | '\t' | '\u{feff}')))
        && (noyalib::cst::parse_document(prefix).is_ok() || unquoted_once_removed(source, offset))
}

/// An open flow collection's prefix never parses (`{\u{feff}}`), so
/// also accept a source that parses once this and every later BOM is
/// removed, with the offset outside every quoted scalar of the result.
fn unquoted_once_removed(source: &str, offset: usize) -> bool {
    // Later BOMs are rejected the same way; drop them too.
    let without = [&source[..offset], &source[offset..].replace('\u{feff}', "")].concat();
    let Ok(document) = noyalib::cst::parse_document(&without) else {
        return false;
    };
    let mut position = 0;
    let mut stack = vec![document.syntax()];
    while let Some(node) = stack.pop() {
        for child in node.children() {
            match child {
                noyalib::cst::GreenChild::Node(child) => stack.push(child),
                noyalib::cst::GreenChild::Token { kind, .. } => {
                    let end = position + child.text_len();
                    let quoted = matches!(
                        kind,
                        noyalib::cst::SyntaxKind::SingleQuotedScalar
                            | noyalib::cst::SyntaxKind::DoubleQuotedScalar
                    );
                    if quoted && position < offset && offset < end {
                        return false;
                    }
                    position = end;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unquoted_bom_oracle_difference_requires_an_actual_located_rejection() {
        for source in [
            "- \u{feff}\t",
            "key: \u{feff}value",
            "- - \u{feff}leaf",
            " \u{feff}\tUo\t)/ ",
            "\t\u{feff}value",
            "{\u{feff}}",
            "\u{feff}\u{feff}",
            "\n\u{feff}",
            "# c\n\u{feff}a: 1\n",
            "{a: \u{feff}b, c: \u{feff}d}",
        ] {
            let error = noyalib::from_str::<noyalib::Value>(source).unwrap_err();
            assert!(
                unquoted_content_bom_rejection(source, &error),
                "{source:?}: {error}"
            );
        }
    }

    #[test]
    fn bom_classifier_does_not_hide_prefix_quote_or_other_error_failures() {
        let message =
            "byte order mark inside the stream \u{2014} a BOM is only allowed at the very start";
        for (source, offset) in [
            ("\u{feff}key: value", 0),
            ("a: 1\n...\n\u{feff}---\nb: 2", 9),
            ("key: '\u{feff}value'", 6),
            ("key: \"\u{feff}value\"", 6),
            ("- \u{feff}", 3),
            ("[a, '\u{feff}b']", 5),
            ("{\"\u{feff}\": 1}", 2),
        ] {
            let error = noyalib::Error::parse_at(message, source, offset);
            assert!(
                !unquoted_content_bom_rejection(source, &error),
                "{source:?}"
            );
        }
        let source = "- \u{feff}";
        let other = noyalib::Error::parse_at("different syntax error", source, 2);
        assert!(!unquoted_content_bom_rejection(source, &other));
    }
}
