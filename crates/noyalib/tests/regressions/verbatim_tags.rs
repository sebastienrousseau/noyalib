// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A verbatim tag `!<x>` names the tag URI `x` exactly. It used to be
//! scanned with the `!` handle, the same as the local shorthand `!x`, so
//! `!<int>` was read as the core integer tag and `!<foo>` as the local
//! tag `!foo`. A tag resolved through `%TAG !! ` with an empty prefix
//! (the URI `int`) was then written as `!<int>` and read back as an
//! integer, or failed to parse.

use noyalib::{Tag, TaggedValue, Value};

fn round_trip(src: &str) -> (Value, Value) {
    let first: Value = noyalib::from_str(src).expect("input parses");
    let out = noyalib::to_string(&first).expect("serialises");
    let second: Value =
        noyalib::from_str(&out).unwrap_or_else(|e| panic!("output {out:?} does not parse: {e}"));
    (first, second)
}

fn tag_of(src: &str) -> String {
    match noyalib::from_str::<Value>(src).expect("input parses") {
        Value::Tagged(t) => t.tag().as_str().to_owned(),
        other => panic!("{src:?} did not keep its tag: {other:?}"),
    }
}

#[test]
fn empty_secondary_prefix_round_trips() {
    for src in [
        "%TAG !! \n---\n!!int 1 - 3\n",
        "%TAG !! \n---\n!!int 1\n",
        "%TAG !! \n---\n!!str a b\n",
    ] {
        let (first, second) = round_trip(src);
        assert_eq!(first, second, "{src:?}");
    }
}

#[test]
fn verbatim_tag_is_the_uri_not_a_shorthand() {
    let v: Value = noyalib::from_str("!<int> 1\n").unwrap();
    assert_eq!(
        v,
        Value::Tagged(Box::new(TaggedValue::new(
            Tag::new("int"),
            Value::String("1".into())
        ))),
        "`!<int>` is the tag URI `int`, not the core integer tag"
    );
    // `Tag` equality ignores one leading `!` (serde_yaml's rule), so
    // compare the parsed tag strings: the parser must keep them apart.
    assert_eq!(tag_of("!foo x\n"), "!foo");
    assert_eq!(
        tag_of("!<foo> x\n"),
        "foo",
        "`!<foo>` is the URI `foo`, not `!foo`"
    );
    assert_eq!(
        tag_of("!<!foo> x\n"),
        "!foo",
        "`!<!foo>` spells the local tag `!foo`"
    );
}

#[test]
fn verbatim_core_uri_keeps_its_core_meaning() {
    let v: Value = noyalib::from_str("!<tag:yaml.org,2002:int> 7\n").unwrap();
    assert_eq!(v, Value::from(7));
    let s: Value = noyalib::from_str("!<tag:yaml.org,2002:str> 7\n").unwrap();
    assert_eq!(s, Value::from("7"));
}
