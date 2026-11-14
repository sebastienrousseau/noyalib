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
    // A later document prefix is handled by the documented policy below.
    // Blank lines may come first: the BOM then follows indentation on
    // what would be the first content line (`\n \u{feff}`).
    let displaced_start = !prefix.is_empty()
        && !line.is_empty()
        && prefix
            .chars()
            .all(|c| matches!(c, ' ' | '\t' | '\r' | '\n' | '\u{feff}'));
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
    // The same policy covers a line-start BOM before a document marker,
    // where it would open the next document's prefix (`-\r\u{feff}---`).
    let rest = &source[offset + '\u{feff}'.len_utf8()..];
    // After a `...` the next prefix is spec-valid and stays visible.
    let ended = prefix.split(['\r', '\n']).any(|l| l.starts_with("..."));
    let before_marker =
        line.is_empty() && !ended && (rest.starts_with("---") || rest.starts_with("..."));
    if later_prefix || before_marker && raised_here(source, offset) {
        return true;
    }
    // Inside a document (after content or a `---` line, before any `...`),
    // a BOM at a line start or after indentation is content too: only a
    // document prefix may hold one.
    let in_document = line.bytes().all(|b| matches!(b, b' ' | b'\t'))
        && prefix.contains(|c: char| !c.is_whitespace() && c != '\u{feff}')
        && !prefix.split(['\r', '\n']).any(|l| {
            let l = l.trim_start_matches('\u{feff}');
            l.starts_with("...")
        })
        // Only a column-0 BOM could open the next document's prefix.
        && !(line.is_empty() && (rest.starts_with("---") || rest.starts_with("...")));
    (displaced_start
        || repeated_start
        || in_document
        || line.chars().any(|c| !matches!(c, ' ' | '\t' | '\u{feff}')))
        && raised_here(source, offset)
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
                    && message.starts_with("byte order mark inside the stream")
        )
    })
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
            "[\u{feff},]",
            "{\u{feff}'}",
            "n:\n\u{feff}",
            "\n \u{feff}",
            "-\r \u{feff}",
            "---\r\u{feff}",
            "-\r\u{feff}-",
            "\u{feff} \u{feff}",
            "-\r\u{feff}---",
            "a:\r \u{feff}---",
            "{4e740,\rinf,\u{feff}}",
        ] {
            // The compatibility target classifies the shim's error.
            let config = noyalib::ParserConfig::serde_yaml_compat();
            let error =
                noyalib::from_str_with_config::<noyalib::Value>(source, &config).unwrap_err();
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
