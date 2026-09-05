// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Structure-aware round trip: `Arbitrary` builds a `Value` tree rather
//! than bytes, so the fuzzer spends its budget on semantically valid
//! documents and proves invariants instead of only surviving them.
//!
//! 1. `from_str(to_string(v)) == v` for every finite `Value`, and
//!    `to_string` is idempotent on the re-parsed value.
//! 2. `cst::parse_document(to_string(v)).as_value() == v`: the lossless
//!    tree and the value loader agree on emitted output.
//! 3. The alias budget is a hard ceiling: a generated alias fan-out is
//!    refused exactly when its alias occurrences exceed
//!    `max_alias_expansions`, and accepted otherwise.
#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use noyalib::{Mapping, ParserConfig, Value};

#[derive(Arbitrary, Debug)]
enum Node {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Seq(Vec<Node>),
    Map(Vec<(String, Node)>),
}

#[derive(Arbitrary, Debug)]
struct Case {
    root: Node,
    anchors: u8,
    refs: u8,
    depth: u8,
    budget: u16,
}

/// Build a `Value`; `None` when the tree is too deep or holds a NaN,
/// which is the one float the equality invariant cannot cover.
fn build(n: &Node, depth: usize) -> Option<Value> {
    if depth > 24 {
        return None;
    }
    Some(match n {
        Node::Null => Value::Null,
        Node::Bool(b) => Value::Bool(*b),
        Node::Int(i) => Value::Number((*i).into()),
        Node::Float(f) => {
            if f.is_nan() {
                return None;
            }
            Value::Number((*f).into())
        }
        Node::Str(s) => Value::String(s.clone()),
        Node::Seq(items) => Value::Sequence(
            items
                .iter()
                .map(|i| build(i, depth + 1))
                .collect::<Option<Vec<_>>>()?,
        ),
        Node::Map(entries) => {
            let mut m = Mapping::new();
            for (k, v) in entries {
                m.insert(k.clone(), build(v, depth + 1)?);
            }
            Value::Mapping(m)
        }
    })
}

/// `anchors` anchored sequences, each referencing the previous one
/// `refs` times, then `depth` leaves aliasing the last. The loader's
/// budget counts alias occurrences, so the count is known exactly and
/// the budget check is a precise oracle. The fan-out is bounded so the
/// fully expanded tree (under a thousand nodes) stays clear of the
/// node ceiling, which is a different guard with a different error.
fn alias_bomb(anchors: u8, refs: u8, depth: u8) -> (String, usize) {
    let a = usize::from(anchors) % 4 + 1;
    let r = usize::from(refs) % 6;
    let d = usize::from(depth) % 4;
    let mut doc = String::from("base: &a0 [x]\n");
    let mut expansions = 0usize;
    for i in 1..=a {
        doc.push_str(&format!("n{i}: &a{i} ["));
        for _ in 0..r {
            doc.push_str(&format!("*a{}, ", i - 1));
        }
        doc.push_str("]\n");
        expansions += r;
    }
    for lvl in 0..d {
        doc.push_str(&format!("leaf{lvl}: *a{a}\n"));
        expansions += 1;
    }
    (doc, expansions)
}

fuzz_target!(|case: Case| {
    if let Some(v) = build(&case.root, 0) {
        let text = noyalib::to_string(&v).expect("every finite Value serialises");
        let back: Value = noyalib::from_str(&text).expect("emitted YAML re-parses");
        assert_eq!(back, v, "value round-trip drift:\n{text}");
        let again = noyalib::to_string(&back).expect("re-parsed Value serialises");
        assert_eq!(again, text, "to_string is not idempotent");
        let doc = noyalib::cst::parse_document(&text).expect("CST parses emitted YAML");
        assert_eq!(*doc.as_value(), v, "CST and loader disagree on emitted YAML");
    }

    let (bomb, expansions) = alias_bomb(case.anchors, case.refs, case.depth);
    let budget = usize::from(case.budget);
    // Only the occurrence budget is under test: the alias-to-anchor ratio
    // heuristic is a separate guard and is switched off here.
    let cfg = ParserConfig::new()
        .max_alias_expansions(budget)
        .alias_anchor_ratio(None);
    let res = noyalib::from_str_with_config::<Value>(&bomb, &cfg);
    if expansions > budget {
        assert!(
            res.is_err(),
            "budget {budget} exceeded by {expansions} expansions, parser accepted:\n{bomb}"
        );
    } else {
        assert!(
            res.is_ok(),
            "within budget {budget} ({expansions} expansions), parser refused:\n{bomb}\n{res:?}"
        );
    }
});
