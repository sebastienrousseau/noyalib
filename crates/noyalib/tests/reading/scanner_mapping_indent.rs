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
