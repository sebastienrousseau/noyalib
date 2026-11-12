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
    // A possible document prefix is not an established content position.
    // Parsing the prefix also keeps unclosed quoted content outside the rule.
    line.chars().any(|c| !matches!(c, ' ' | '\t' | '\u{feff}'))
        && noyalib::cst::parse_document(prefix).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unquoted_bom_oracle_difference_requires_an_actual_located_rejection() {
        for source in ["- \u{feff}\t", "key: \u{feff}value", "- - \u{feff}leaf"] {
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
