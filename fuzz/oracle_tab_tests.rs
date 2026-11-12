// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Preserve complete comparisons when libyaml rejects BOM-prefixed trivia.

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
