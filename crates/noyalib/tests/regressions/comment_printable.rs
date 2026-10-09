// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A comment is limited to printable characters (YAML 1.2.2 §5.1 and
//! §6.6: comment text is nb-char), as scalar content already is. Found
//! by the serde_yaml compat fuzzer (input `#]\0`): a raw control
//! character in a comment was accepted.

use noyalib::{Value, from_str};

#[test]
fn raw_control_characters_in_comments_are_refused() {
    for doc in [
        "#]\0",
        "# a\u{7}\n",
        "a: 1 # x\u{1}\n",
        "a: 1\n#\0\n",
        "- x\n  # \u{1b}[31m\n",
        "a: |  # \u{7f}\n  text\n",
        "%YAML 1.2 # \u{1}\n---\na\n",
    ] {
        assert!(from_str::<Value>(doc).is_err(), "accepted {doc:?}");
    }
}

#[test]
fn printable_comments_still_parse() {
    for doc in [
        "# a\tb\n",
        "a: 1 # é ü 🦀\n",
        "#\n",
        "a: |  # header\n  text\n",
    ] {
        assert!(from_str::<Value>(doc).is_ok(), "refused {doc:?}");
    }
}
