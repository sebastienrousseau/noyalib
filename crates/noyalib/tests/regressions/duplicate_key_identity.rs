// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `DuplicateKeyPolicy` judges keys by their resolved value on every
//! load path: `~` and `null` are the same null key. Found while
//! triaging fuzz_diff: the streaming typed path compared the raw
//! text, so it accepted `~: 1\nnull: 2` that the `Value` path refused.

use std::collections::BTreeMap;

use noyalib::{DuplicateKeyPolicy, ParserConfig, Value, from_str_with_config};

#[test]
fn same_resolved_key_is_a_duplicate_on_every_path() {
    let refuse = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    for doc in ["~: 1\nnull: 2\n", "0x1F: 1\n31: 2\n", "True: 1\ntrue: 2\n"] {
        assert!(
            from_str_with_config::<Value>(doc, &refuse).is_err(),
            "Value {doc:?}"
        );
        assert!(
            from_str_with_config::<BTreeMap<String, i32>>(doc, &refuse).is_err(),
            "typed {doc:?}"
        );
    }
}

#[test]
fn first_policy_keeps_the_first_of_a_resolved_duplicate() {
    let first = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let m: BTreeMap<String, i32> = from_str_with_config("~: 1\nnull: 2\n", &first).unwrap();
    assert_eq!(m.len(), 1, "{m:?}");
    assert_eq!(m.values().next(), Some(&1));
}
