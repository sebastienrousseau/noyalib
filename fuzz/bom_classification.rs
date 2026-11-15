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
        != "byte order mark inside a document: a BOM may only open a document prefix"
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
    // A BOM opening a later document prefix (section 9.2) loads.
    // Blank lines may come first: the BOM then follows indentation on
    // what would be the first content line (`\n \u{feff}`).
    let displaced_start = !prefix.is_empty()
        && !line.is_empty()
        && prefix
            .chars()
            .all(|c| matches!(c, ' ' | '\t' | '\r' | '\n' | '\u{feff}'));
    // A BOM straight after the stream BOM, or after a BOM opening a later
    // prefix, is document content (section 5.2): `#.\n\n\u{feff}\u{feff}`.
    let repeated_start = (!prefix.is_empty() && prefix.chars().all(|c| c == '\u{feff}'))
        || prefix.strip_suffix('\u{feff}').is_some_and(|before| {
            before.is_empty() || before.ends_with(['\n', '\r'])
        });
    let rest = &source[offset + '\u{feff}'.len_utf8()..];
    // Inside a document (after content or a `---` line, before any `...`),
    // a BOM at a line start or after indentation is content too: only a
    // document prefix may hold one.
    let in_document = line.bytes().all(|b| matches!(b, b' ' | b'\t'))
        && prefix.contains(|c: char| !c.is_whitespace() && c != '\u{feff}')
        && !prefix.split(['\r', '\n']).any(|l| {
            let l = l.trim_start_matches('\u{feff}');
            marker(l, "...")
        })
        // Only a column-0 BOM could open the next document's prefix.
        && !(line.is_empty() && (marker(rest, "---") || marker(rest, "...")));
    (displaced_start
        || repeated_start
        || in_document
        || line.chars().any(|c| !matches!(c, ' ' | '\t' | '\u{feff}')))
        && raised_here(source, offset)
}

/// The source without its line-start BOMs, when it has any past the first
/// byte. Where noyalib accepts the document, each opens a document prefix
/// (section 9.2) and is no text, while libyaml reads one after content as
/// part of the scalar (`.\n\u{feff}` loads ". \u{feff}"). The repair only
/// lets the archive read the same text; the values must still agree.
#[allow(dead_code)]
pub(crate) fn without_prefix_boms(source: &str) -> Option<String> {
    let mut out = String::with_capacity(source.len());
    let mut changed = false;
    for (at, c) in source.char_indices() {
        let line_start = source[..at].ends_with(['\n', '\r']);
        if c == '\u{feff}' && line_start {
            changed = true;
        } else {
            out.push(c);
        }
    }
    changed.then_some(out)
}

/// [`without_prefix_boms`] with the leading tabs of each blank or comment
/// line made spaces. libyaml refuses a line-start tab even there
/// (`$\n#\n\t`, `b\n#\n\t#`), which it otherwise read inside the scalar
/// the BOM extended. The line stays (emptying one between CR and LF would
/// join them into one break). The values must still agree.
#[allow(dead_code)]
pub(crate) fn without_prefix_boms_or_tab_lines(source: &str) -> Option<String> {
    let stripped = without_prefix_boms(source)?;
    let mut out = String::with_capacity(stripped.len());
    let mut rest = stripped.as_str();
    while !rest.is_empty() {
        let end = rest.find(['\n', '\r']).map_or(rest.len(), |at| at + 1);
        let (line, tail) = rest.split_at(end);
        let body = line.trim_end_matches(['\n', '\r']);
        let text = body.trim_start_matches([' ', '\t']);
        let lead = body.len() - text.len();
        if lead > 0 && (text.is_empty() || text.starts_with('#')) {
            out.push_str(&body[..lead].replace('\t', " "));
            out.push_str(&line[lead..]);
        } else {
            out.push_str(line);
        }
        rest = tail;
    }
    Some(out)
}

/// `text` opens with the document marker `mark` (`---` or `...`), which
/// must end at white space or a line end: `....` is a plain scalar.
fn marker(text: &str, mark: &str) -> bool {
    text.strip_prefix(mark)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t', '\r', '\n']))
}

/// noyalib itself refuses this BOM at this offset. The scanner raises
/// that error only in unquoted content (a quoted BOM is text), which
/// keeps quoted and synthetic locations outside the classifier, and also
/// covers prefixes that are not documents on their own (`[\u{feff},]`).
fn raised_here(source: &str, offset: usize) -> bool {
    // Either profile may reach the BOM: in the default one `4e740` is an
    // infinity that collides with an `inf` key before it (`{4e740,\rinf,\u{feff}}`).
    [
        noyalib::ParserConfig::default(),
        noyalib::ParserConfig::serde_yaml_compat(),
    ]
    .iter()
    .any(|config| {
        matches!(
            noyalib::from_str_with_config::<noyalib::Value>(source, config),
            Err(noyalib::Error::ParseWithLocation { ref message, ref location })
                if location.index() == offset
                    && message.starts_with("byte order mark inside a document")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unquoted_bom_oracle_difference_requires_an_actual_located_rejection() {
        // A BOM opening a later line that is no document prefix: both
        // profiles refuse it, and the classifier recognises the refusal.
        for source in [
            "\u{feff}\u{feff}",
            "-\r\u{feff}-",
            "s\r\u{feff}\u{feff}",
            "#.\n\n\u{feff}\u{feff}",
        ] {
            for config in [
                noyalib::ParserConfig::default(),
                noyalib::ParserConfig::serde_yaml_compat(),
            ] {
                let error =
                    noyalib::from_str_with_config::<noyalib::Value>(source, &config).unwrap_err();
                assert!(
                    unquoted_content_bom_rejection(source, &error),
                    "{source:?}: {error}"
                );
            }
        }
        // A BOM opening a token after other text on its line: since
        // v0.0.62 the compatibility profile reads it as text, as libyaml
        // does, so only the default profile's refusal is classified.
        for source in [
            "- \u{feff}\t",
            "key: \u{feff}value",
            "- - \u{feff}leaf",
            " \u{feff}\tUo\t)/ ",
            "\t\u{feff}value",
            "{\u{feff}}",
            "{a: \u{feff}b, c: \u{feff}d}",
            "[\u{feff},]",
            "{\u{feff}'}",
            "\n \u{feff}",
            "-\r \u{feff}",
            "\u{feff} \u{feff}",
            "a:\r \u{feff}---",
        ] {
            let compat = noyalib::ParserConfig::serde_yaml_compat();
            assert!(
                noyalib::from_str_with_config::<noyalib::Value>(source, &compat).is_ok(),
                "{source:?}"
            );
            let error = noyalib::from_str::<noyalib::Value>(source).unwrap_err();
            assert!(
                unquoted_content_bom_rejection(source, &error),
                "{source:?}: {error}"
            );
        }
        // The default profile stops earlier here (`4e740` is an infinity
        // that collides with the `inf` key), so only the compatibility
        // profile reaches the BOM, which it now reads as text.
        let source = "{4e740,\rinf,\u{feff}}";
        let compat = noyalib::ParserConfig::serde_yaml_compat();
        assert!(noyalib::from_str_with_config::<noyalib::Value>(source, &compat).is_ok());
    }

    #[test]
    fn prefix_bom_repair_drops_only_line_start_boms() {
        assert_eq!(without_prefix_boms(".\n\u{feff}").as_deref(), Some(".\n"));
        assert_eq!(
            without_prefix_boms("a\r\u{feff}\r\u{feff}---").as_deref(),
            Some("a\r\r---")
        );
        assert_eq!(without_prefix_boms("\u{feff}a: \u{feff}b"), None);
        assert_eq!(without_prefix_boms("a\n \u{feff}"), None);
    }

    #[test]
    fn tab_line_repair_spaces_blank_and_comment_line_tabs() {
        assert_eq!(
            without_prefix_boms_or_tab_lines("\n$\n\u{feff}#\n\n \t\r\n\t a\n").as_deref(),
            Some("\n$\n#\n\n  \r\n\t a\n")
        );
        // Found by fuzz_serde_yaml_compat: the line between CR and LF stays.
        assert_eq!(
            without_prefix_boms_or_tab_lines("a\r\t\n\u{feff}").as_deref(),
            Some("a\r \n")
        );
        // Found by fuzz_serde_yaml_compat: a tab before a comment.
        assert_eq!(
            without_prefix_boms_or_tab_lines("b\n\u{feff}#\n\t#\t\n").as_deref(),
            Some("b\n#\n #\t\n")
        );
        assert_eq!(without_prefix_boms_or_tab_lines("a\n\t\n"), None);
    }

    #[test]
    fn bom_classifier_does_not_hide_prefix_quote_or_other_error_failures() {
        let message =
            "byte order mark inside a document: a BOM may only open a document prefix";
        for (source, offset) in [
            ("\u{feff}key: value", 0),
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
