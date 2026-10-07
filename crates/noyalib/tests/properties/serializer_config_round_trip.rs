// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Every string survives every serializer configuration.
//!
//! An arbitrary string, drawn mostly from the characters that steer
//! YAML scalar resolution (line breaks of every kind, a BOM, tabs, flow
//! indicators, `# `, `: `, document markers, `<<`, the node and
//! directive indicators), is placed as a mapping key, a mapping value and
//! a sequence item, in block and nested positions, then written under
//! every combination of `flow_style`, `scalar_style`, `quote_all`,
//! `prefer_single_quotes` and the two document markers. Each output must
//! parse back to the identical `Value`, with no carve-outs.
//!
//! The case count is `PROPTEST_CASES` (default 64, the CI setting); run
//! with `PROPTEST_CASES=20000` or more locally after touching the
//! emitter.

#![allow(clippy::unwrap_used, missing_docs)]

use noyalib::{
    FlowStyle, Mapping, ScalarStyle, SerializerConfig, Value, from_str, to_string_with_config,
};
use proptest::prelude::*;

const ATOMS: &[&str] = &[
    "a",
    "b",
    "z",
    "0",
    "1",
    " ",
    "  ",
    "\t",
    "\n",
    "\r",
    "\r\n",
    "\u{85}",
    "\u{2028}",
    "\u{2029}",
    "\u{feff}",
    "\0",
    "\x07",
    "\x1b",
    "\u{7f}",
    "\u{9f}",
    "\u{a0}",
    "-",
    "- ",
    "?",
    "? ",
    ":",
    ": ",
    "#",
    " #",
    "# ",
    "&",
    "*",
    "!",
    "!!",
    "%",
    "@",
    "`",
    "|",
    ">",
    "'",
    "\"",
    "\\",
    "{",
    "}",
    "[",
    "]",
    ",",
    ", ",
    "---",
    "...",
    "~",
    "null",
    "true",
    "no",
    "0x1F",
    "1e3",
    ".inf",
    ".nan",
    "<<",
    "=",
    "\u{1F600}",
    "\u{fffe}",
    "\u{ffff}",
    "-1",
    "0o7",
    "+",
    ".",
    "_",
];

fn hostile_string() -> impl Strategy<Value = String> {
    prop_oneof![
        8 => proptest::collection::vec(proptest::sample::select(ATOMS), 0..7)
            .prop_map(|parts| parts.concat()),
        1 => any::<String>(),
    ]
}

/// The string as a key, a value and a sequence item, at the top level
/// and one collection down.
fn placements(s: &str) -> Value {
    let str_v = || Value::String(s.to_owned());
    let mut inner = Mapping::new();
    let _ = inner.insert(s, Value::Sequence(vec![str_v(), Value::from(1)]));
    let mut m = Mapping::new();
    let _ = m.insert(s, str_v());
    let _ = m.insert("seq", Value::Sequence(vec![str_v(), Value::from("z")]));
    let _ = m.insert("nested", Value::Mapping(inner));
    Value::Mapping(m)
}

fn configs() -> Vec<SerializerConfig> {
    let flows = [FlowStyle::Block, FlowStyle::Flow, FlowStyle::Auto];
    let scalars = [
        ScalarStyle::Auto,
        ScalarStyle::DoubleQuoted,
        ScalarStyle::SingleQuoted,
        ScalarStyle::Literal,
        ScalarStyle::Folded,
        ScalarStyle::Plain,
    ];
    let mut out = Vec::new();
    for flow in flows {
        for scalar in scalars {
            for bits in 0..16u8 {
                out.push(
                    SerializerConfig::new()
                        .flow_style(flow)
                        .scalar_style(scalar)
                        .quote_all(bits & 1 != 0)
                        .prefer_single_quotes(bits & 2 != 0)
                        .document_start(bits & 4 != 0)
                        .document_end(bits & 8 != 0),
                );
            }
        }
    }
    out
}

fn cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(64)
}

fn check(v: &Value, cfg: &SerializerConfig) -> Result<(), TestCaseError> {
    let out = to_string_with_config(v, cfg).unwrap();
    let back: Value = from_str(&out)
        .map_err(|e| TestCaseError::fail(format!("{cfg:?}\n{out:?}\nfailed to parse: {e}")))?;
    prop_assert_eq!(&back, v, "{:?}\nemitted {:?}", cfg, out);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases()))]

    #[test]
    fn every_string_round_trips_under_every_config(s in hostile_string()) {
        let configs = configs();
        for v in [Value::String(s.clone()), placements(&s)] {
            for cfg in &configs {
                check(&v, cfg)?;
            }
        }
    }
}
