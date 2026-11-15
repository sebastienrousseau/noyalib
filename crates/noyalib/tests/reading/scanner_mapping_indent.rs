// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

use noyalib::{Value, from_str};

#[test]
fn an_unindented_scalar_cannot_supply_a_block_mapping_value() {
    for source in [
        include_str!("../fixtures/unindented-mapping-value.yaml"),
        "plain:\ninfinite\nnext: false\n",
        "root:\n  plain:\n  infinite\n  next: false\n",
        "plain:\n'quoted'\nnext: false\n",
        "anchor: &a value\nplain:\n*a\nnext: false\n",
    ] {
        assert_eq!(
            [
                from_str::<Value>(source).is_ok(),
                from_str::<serde_json::Value>(source).is_ok(),
                noyalib::cst::parse_document(source).is_ok()
            ],
            [false; 3],
            "[value, streaming, CST] accepted {source:?}",
        );
    }
}

#[test]
fn mapping_siblings_and_indentless_sequences_remain_valid() {
    for source in [
        "plain:\n  infinite\nnext: false\n",
        "plain:\n  'quoted'\nnext: false\n",
        "plain:\n- first\n- second\nnext: false\n",
        "plain:\nnext: false\n",
        "root:\n  plain:\n    infinite\n  next: false\n",
        "first: &a value\nplain:\n  *a\nnext: false\n",
        "? explicit\n: value\nnext: false\n",
    ] {
        let value: Value = from_str(source).expect(source);
        assert!(value.as_mapping().is_some());
        let streamed: serde_json::Value = from_str(source).expect(source);
        assert!(streamed.is_object());
        let document = noyalib::cst::parse_document(source).expect(source);
        assert_eq!(document.to_string(), source);
    }
}

#[test]
fn tabs_cannot_supply_plain_continuation_indentation() {
    for source in [
        include_str!("../fixtures/plain-continuation-tab-indent.yaml"),
        "- a\n\tb",
        "- a\r\tb",
        "- a\r\n\tb",
        "key: a\n\tb",
        "root:\n  key: a\n  \tb",
        "- a\n\t b",
    ] {
        assert_eq!(
            [
                from_str::<Value>(source).is_ok(),
                from_str::<serde_json::Value>(source).is_ok(),
                noyalib::cst::parse_document(source).is_ok()
            ],
            [false; 3],
            "[value, streaming, CST] accepted tab indentation {source:?}",
        );
    }
}

#[test]
fn plain_continuation_tabs_after_required_spaces_remain_valid() {
    for (source, expected) in [
        ("- a\n \tb", serde_json::json!(["a b"])),
        ("- a\r \tb", serde_json::json!(["a b"])),
        ("- a\r\n \tb", serde_json::json!(["a b"])),
        ("key: a\n \tb", serde_json::json!({"key": "a b"})),
        ("a\n\tb", serde_json::json!("a b")),
        ("- a\n\t\n b", serde_json::json!(["a\nb"])),
        (
            "key: a\n\t# comment\nnext: b",
            serde_json::json!({"key": "a", "next": "b"}),
        ),
    ] {
        let value: serde_json::Value = from_str(source).expect(source);
        assert_eq!(value, expected, "{source:?}");
        let owned: Value = from_str(source).expect(source);
        assert_eq!(serde_json::to_value(owned).unwrap(), expected, "{source:?}");
        assert_eq!(
            noyalib::cst::parse_document(source).unwrap().to_string(),
            source
        );
    }
}
