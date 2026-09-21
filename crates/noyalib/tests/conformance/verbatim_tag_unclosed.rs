// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `c-verbatim-tag ::= "!" "<" ns-uri-char+ ">"`: the closing bracket
//! is part of the production. An unterminated `!<…` at end of input
//! was accepted with whatever it had swallowed as the tag name, and
//! `to_string` then wrote a tag the parser read back differently.
//! Found by `fuzz_roundtrip_alloc_only` on the three bytes `!<<`.

#![allow(clippy::unwrap_used, missing_docs)]

use noyalib::{Value, from_str, to_string};

#[test]
fn unterminated_verbatim_tag_is_refused() {
    // A bare `!<` trips the older "must not be empty" check first; every
    // other shape reaches the new "not closed" one. Both are refusals of
    // the same production.
    for src in ["!<<", "!<", "!<tag:a", "k: !<x", "- !<a,b"] {
        let err = from_str::<Value>(src).expect_err(src);
        let msg = err.to_string();
        assert!(
            msg.contains("not closed") || msg.contains("must not be empty"),
            "{src:?}: {msg}"
        );
    }
}

#[test]
fn closed_verbatim_tags_still_round_trip() {
    for src in [
        "!<tag:yaml.org,2002:str> x\n",
        "!<!local> x\n",
        "k: !<tag:a,2026:b> [1]\n",
    ] {
        let v: Value = from_str(src).unwrap();
        let out = to_string(&v).unwrap();
        let back: Value = from_str(&out).unwrap();
        assert_eq!(back, v, "{src:?} emitted as {out:?}");
    }
}
