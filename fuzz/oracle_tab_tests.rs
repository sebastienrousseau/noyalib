// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Preserve complete development-oracle values and diagnostic comparisons.

#[path = "../crates/noyalib/tests/properties/oracle_normalization.rs"]
#[expect(unexpected_cfgs, reason = "shared core tests name core-only features")]
mod normalization;

#[test]
fn bom_prefixed_empty_line_tabs_still_compare_the_complete_value() {
    for source in [
        "\u{feff}\t",
        "\u{feff} \t\nkey: value\n",
        "\u{feff}\t\rkey: value\r",
        "\u{feff}\t\r\nkey: value\r\n",
    ] {
        let before: serde_json::Value = noyalib::compat::serde_yaml::from_str(source).unwrap();
        assert!(serde_yaml::from_str::<serde_json::Value>(source).is_err());
        let normalized = normalization::collection_spelling(source).expect("BOM line separation");
        assert_eq!(normalized, source.replace('\t', " "));
        let after: serde_json::Value = noyalib::compat::serde_yaml::from_str(&normalized).unwrap();
        let oracle: serde_json::Value = serde_yaml::from_str(&normalized).unwrap();
        assert_eq!(before, after);
        assert_eq!(before, oracle);
    }
}

#[test]
fn bom_tab_repair_preserves_scalar_content_and_nonempty_lines() {
    for source in [
        "\u{feff}'a\tb'\n",
        "\u{feff}\"a\tb\"\n",
        "\u{feff}|\n  \t\n",
        "\u{feff}>\n  \t\n",
        "\u{feff}plain\ttext\n",
    ] {
        assert!(
            normalization::collection_spelling(source).is_none(),
            "{source:?}"
        );
    }
}

#[test]
fn decimal_underflow_json_errors_match_the_archive() {
    for source in [
        "-9223372036854775809\n",
        "a: -9223372036854778508\n",
        "outer:\n  a: -9223372036854778508\n",
    ] {
        let actual =
            noyalib::compat::serde_yaml::from_str::<serde_json::Value>(source).unwrap_err();
        let expected = serde_yaml::from_str::<serde_json::Value>(source).unwrap_err();
        assert_eq!(actual.to_string(), expected.to_string());
        let actual = actual.location().map(|p| (p.line(), p.column(), p.index()));
        let expected = expected
            .location()
            .map(|p| (p.line(), p.column(), p.index()));
        assert_eq!(actual, expected);
    }
}

#[test]
fn leading_stream_suffixes_preserve_complete_oracle_comparison() {
    for (source, expected) in [
        ("\n...\n---- a\n\n- b\n- c\n", "\n   \n---- a\n\n- b\n- c\n"),
        ("...\n", "   \n"),
        ("...\n...\nvalue\n", "   \n   \nvalue\n"),
        ("...\n'...'\n", "   \n'...'\n"),
        (
            "\u{feff}# prefix\r\n...\r\nvalue\r\n",
            "\u{feff}# prefix\r\n   \r\nvalue\r\n",
        ),
        ("...\rvalue\r", "   \rvalue\r"),
    ] {
        let before: serde_json::Value = noyalib::compat::serde_yaml::from_str(source).unwrap();
        assert!(serde_yaml::from_str::<serde_json::Value>(source).is_err());
        let normalized =
            normalization::collection_spelling(source).expect("leading suffix spelling");
        assert_eq!(normalized, expected);
        let after: serde_json::Value = noyalib::compat::serde_yaml::from_str(&normalized).unwrap();
        let oracle: serde_json::Value = serde_yaml::from_str(&normalized).unwrap();
        assert_eq!(before, after);
        assert_eq!(before, oracle);
    }
}

#[test]
fn stream_suffix_repair_preserves_content_and_real_document_boundaries() {
    for source in [
        "'...'\n",
        "\"...\"\n",
        "|\n  ...\n",
        ">\n  ...\n",
        "value\n...\n",
        "---\n...\n",
        "# ...\n",
        "...content\n",
    ] {
        assert!(
            normalization::collection_spelling(source).is_none(),
            "{source:?}"
        );
    }
}

#[test]
fn compact_omitted_keys_preserve_complete_oracle_comparison() {
    for (source, expected) in [
        ("- :", "- \"\":"),
        ("- : value\n", "- \"\": value\n"),
        ("- - : value\n", "- - \"\": value\n"),
        ("- # keep\n  : value\n", "- # keep\n  \"\": value\n"),
        ("- :\n- normal\n", "- \"\":\n- normal\n"),
    ] {
        let before: serde_json::Value = noyalib::compat::serde_yaml::from_str(source).unwrap();
        assert!(serde_yaml::from_str::<serde_json::Value>(source).is_err());
        let normalized =
            normalization::collection_spelling(source).expect("compact empty key spelling");
        assert_eq!(normalized, expected);
        let after: serde_json::Value = noyalib::compat::serde_yaml::from_str(&normalized).unwrap();
        let oracle: serde_json::Value = serde_yaml::from_str(&normalized).unwrap();
        assert_eq!(before, after);
        assert_eq!(before, oracle);
    }
    for source in ["- key: value\n", "- ':'\n", "- \":\"\n", "|\n  - :\n"] {
        assert!(
            normalization::collection_spelling(source).is_none(),
            "{source:?}"
        );
    }
}

#[test]
fn decimal_128_bit_boundaries_match_the_archive() {
    for number in [
        "340282366920938463463374607431768211455",
        "340282366920938463463374607431768211456",
        "+340282366920938463463374607431768211456",
        "-170141183460469231731687303715884105728",
        "-170141183460469231731687303715884105729",
    ] {
        let source = format!("a: {number}\n");
        let expected = serde_yaml::from_str::<serde_json::Value>(&source);
        let actual = noyalib::compat::serde_yaml::from_str::<serde_json::Value>(&source);
        match (actual, expected) {
            (Ok(actual), Ok(expected)) => assert_eq!(actual, expected),
            (Err(actual), Err(expected)) => assert_eq!(actual.to_string(), expected.to_string()),
            (actual, expected) => panic!("{number}: {actual:?} != {expected:?}"),
        }
    }
}
