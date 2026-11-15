// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! YAML 1.2.2 §7.3 `s-flow-line-prefix(n)`: every continuation line of a
//! flow scalar is indented past the surrounding block, like any other
//! flow line (yaml-test-suite 9C9N). Found by fuzz_serde_yaml_compat on
//! `-\t{4\n.\r,}`, where `- {\na}` was refused but `- {4\na}` loaded.

use noyalib::Value;

#[test]
fn under_indented_flow_scalar_continuations_are_refused() {
    for source in [
        "- {4\n.,}",
        "- {4\n.\r,}",
        "-\t{4\n.\r,}",
        "- [a\nb]",
        "k: {a\nb}",
        "k: [a,\n b\nc]",
        "- [a\n,b]",
        "- ['a\nb']",
        "- [\"a\nb\"]",
        "k: ['a\n']",
        "k: [\"a\n\"]",
        "outer:\n  k: [a\n  b]\n",
    ] {
        let error = noyalib::from_str::<Value>(source).expect_err(source);
        assert!(
            error
                .to_string()
                .contains("flow content must be indented more than the surrounding block"),
            "{source:?}: {error}"
        );
        assert!(
            noyalib::cst::parse_document(source).is_err(),
            "CST {source:?}"
        );
    }
}

#[test]
fn indented_and_root_flow_scalar_continuations_load() {
    for (source, expected) in [
        ("[a\nb]", "[\"a b\"]"),
        ("{a\nb}", "{\"a b\":null}"),
        ("k: [a\n b]", "{\"k\":[\"a b\"]}"),
        ("k: [a\n]", "{\"k\":[\"a\"]}"),
        ("k: ['a\n b']", "{\"k\":[\"a b\"]}"),
        ("k: [\"a\n b\"]", "{\"k\":[\"a b\"]}"),
        ("k: [\"a\n \"]", "{\"k\":[\"a \"]}"),
        ("k: [a\n\n b]", "{\"k\":[\"a\\nb\"]}"),
        ("- [a\n  b]", "[[\"a b\"]]"),
        ("outer:\n  k: [a\n   b]\n", "{\"outer\":{\"k\":[\"a b\"]}}"),
    ] {
        let value: serde_json::Value =
            noyalib::from_str(source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
        assert_eq!(value.to_string(), expected, "{source:?}");
    }
}
