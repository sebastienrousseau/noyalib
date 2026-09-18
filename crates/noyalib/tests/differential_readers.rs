// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Where two implementations of the same thing disagree.
//!
//! noyalib reads YAML more than once over. There is a streaming reader
//! and an AST reader; a loader that builds spans and one that skips
//! them; an owned value graph and a borrowed one; and a `serde_yaml`
//! façade over the top. Each pair is supposed to answer the same
//! question the same way, and each is implemented separately.
//!
//! Every defect this suite's siblings have turned up came from exactly
//! that: a toggle carried by one path and dropped by another, a guard
//! present in one and missing in the other. So rather than test each
//! path against what it should say, these run one corpus through every
//! pair and require them to agree. A disagreement is the finding; which
//! side is right is a separate question.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use noyalib::{Value, from_str, load_all, to_string};

/// Documents chosen to exercise the places the readers diverge:
/// anchors, merges, tags, flow and block styles, unusual scalars,
/// duplicate and non-string keys, comments, and line-break flavours.
const CORPUS: &[(&str, &str)] = &[
    ("flat scalars", "a: 1\nb: text\nc: true\nd: ~\ne: 2.5\n"),
    ("nested mapping", "a:\n  b:\n    c: 1\n"),
    ("block sequence", "xs:\n  - 1\n  - two\n  - true\n"),
    (
        "flow collections",
        "m: {a: 1, b: [1, 2]}\nxs: [a, {k: v}]\n",
    ),
    ("anchors and aliases", "base: &b\n  a: 1\nuse: *b\n"),
    (
        "a merge key",
        "base: &b\n  a: 1\ntarget:\n  <<: *b\n  c: 2\n",
    ),
    (
        "merge from a sequence",
        "one: &o\n  a: 1\ntwo: &t\n  b: 2\nm:\n  <<: [*o, *t]\n",
    ),
    ("tags", "a: !!str 1\nb: !Custom v\nc: !<tag:e,1:x> v\n"),
    (
        "block scalars",
        "lit: |\n  one\n  two\nfold: >\n  one\n  two\n",
    ),
    ("keep and strip chomping", "k: |+\n  x\n\ns: |-\n  y\n"),
    ("quoted scalars", "s: 'single'\nd: \"double\\nescape\"\n"),
    (
        "scalars that look typed",
        "a: '1'\nb: \"true\"\nc: 0x1f\nd: 0o17\ne: 1_000\n",
    ),
    (
        "non-string keys",
        "1: int\ntrue: bool\n~: null\n2.5: float\n",
    ),
    ("duplicate keys", "k: first\nk: second\nz: 1\n"),
    (
        "comments everywhere",
        "# head\na: 1 # side\n# between\nb: 2\n# tail\n",
    ),
    ("empty collections", "m: {}\nxs: []\nn: ~\n"),
    ("deep nesting", "a:\n  b:\n    c:\n      d:\n        e: 1\n"),
    (
        "a sequence of mappings",
        "xs:\n  - k: 1\n    j: 2\n  - k: 3\n",
    ),
    ("unicode", "k: \"héllo wörld ✓\"\nemoji: 🎯\n"),
    ("leading document marker", "---\na: 1\n"),
    ("CRLF line breaks", "a: 1\r\nb:\r\n  - 1\r\n  - 2\r\n"),
    ("a BOM", "\u{feff}a: 1\n"),
    ("trailing whitespace", "a: 1   \nb: 2\t\n"),
    ("no trailing newline", "a: 1\nb: 2"),
    ("explicit key", "? complex\n: value\nz: 1\n"),
    ("empty values", "a:\nb:\nc: 1\n"),
];

/// The fast (`from_str`) and span-aware (`load_all`) loaders must load
/// the same document into the same value.
#[test]
fn the_two_loaders_agree_across_the_corpus() {
    let mut disagreements = Vec::new();
    for (label, yaml) in CORPUS {
        let fast = from_str::<Value>(yaml);
        let span = load_all(yaml).and_then(|it| it.collect::<Result<Vec<Value>, _>>());

        match (fast, span) {
            (Ok(f), Ok(s)) => {
                if s.len() != 1 {
                    disagreements.push(format!(
                        "{label}: span loader produced {} documents",
                        s.len()
                    ));
                } else if s[0] != f {
                    disagreements.push(format!(
                        "{label}: values differ\n  fast: {f:?}\n  span: {:?}",
                        s[0]
                    ));
                }
            }
            (Err(fe), Ok(_)) => {
                disagreements.push(format!(
                    "{label}: fast loader refused ({fe}) but the span loader accepted"
                ));
            }
            (Ok(_), Err(se)) => {
                disagreements.push(format!(
                    "{label}: span loader refused ({se}) but the fast loader accepted"
                ));
            }
            (Err(_), Err(_)) => {}
        }
    }
    assert!(
        disagreements.is_empty(),
        "the two loaders disagree on {} document(s):\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}

/// The owned and borrowed graphs must see the same document, except
/// where they are *known* to differ. The exceptions are listed by name
/// with a reason, so a new divergence fails this test rather than
/// joining a blanket tolerance.
///
/// `BorrowedValue` has no `Tagged` variant and no merge-key resolution,
/// so those two are design limits rather than bugs — but they are not
/// documented on `from_str_borrowed`, which advertises itself only as
/// "significantly faster than `from_str::<Value>`".
const KNOWN_GRAPH_DIFFERENCES: &[(&str, &str)] = &[
    (
        "a merge key",
        "the borrowed graph does not resolve `<<`; it stays a literal key",
    ),
    (
        "merge from a sequence",
        "the borrowed graph does not resolve `<<`; it stays a literal key",
    ),
    (
        "tags",
        "`BorrowedValue` has no `Tagged` variant, so a custom tag is dropped \
         (`!!str` is honoured since it only changes resolution)",
    ),
    (
        "non-string keys",
        "a null key coerces to `~` in the borrowed graph and `null` in the owned one",
    ),
];

#[test]
fn the_owned_and_borrowed_graphs_agree_except_where_documented() {
    let mut unexpected = Vec::new();
    let mut expected_but_agreed = Vec::new();

    for (label, yaml) in CORPUS {
        let owned = from_str::<Value>(yaml);
        let borrowed = noyalib::borrowed::from_str_borrowed(yaml);
        let known = KNOWN_GRAPH_DIFFERENCES.iter().find(|(l, _)| l == label);

        let differs = match (&owned, &borrowed) {
            (Ok(o), Ok(b)) => match (to_string(o), to_string(b)) {
                (Ok(a), Ok(c)) => {
                    if a == c {
                        None
                    } else {
                        Some(format!("owned {a:?} vs borrowed {c:?}"))
                    }
                }
                (Err(e), _) | (_, Err(e)) => Some(format!("serialization failed: {e}")),
            },
            (Err(oe), Ok(_)) => Some(format!("owned refused ({oe}), borrowed accepted")),
            (Ok(_), Err(be)) => Some(format!("borrowed refused ({be}), owned accepted")),
            (Err(_), Err(_)) => None,
        };

        match (differs, known) {
            (Some(d), None) => unexpected.push(format!("{label}: {d}")),
            (None, Some((_, why))) => {
                expected_but_agreed.push(format!("{label}: expected to differ ({why}) but agreed"));
            }
            _ => {}
        }
    }

    assert!(
        unexpected.is_empty(),
        "{} new divergence(s) between the owned and borrowed graphs:\n{}",
        unexpected.len(),
        unexpected.join("\n")
    );
    assert!(
        expected_but_agreed.is_empty(),
        "{} known divergence(s) have been fixed — remove them from \
         KNOWN_GRAPH_DIFFERENCES:\n{}",
        expected_but_agreed.len(),
        expected_but_agreed.join("\n")
    );
}

/// `!!str` is a resolution tag, not a decoration: it says the scalar is
/// a string. The borrowed graph used to discard the tag and resolve
/// `!!str 1` to the integer 1, disagreeing with the owned graph.
#[test]
fn the_core_string_tag_is_honoured_by_both_graphs() {
    for (label, yaml) in [
        ("secondary handle", "a: !!str 1\n"),
        ("primary handle", "a: !str 1\n"),
        ("a boolean-looking payload", "a: !!str true\n"),
        ("a float-looking payload", "a: !!str 2.5\n"),
        ("a null-looking payload", "a: !!str ~\n"),
    ] {
        let owned: Value = from_str(yaml).unwrap_or_else(|e| panic!("{label}: owned: {e}"));
        let borrowed = noyalib::borrowed::from_str_borrowed(yaml)
            .unwrap_or_else(|e| panic!("{label}: borrowed: {e}"));

        assert!(
            matches!(owned.get("a"), Some(Value::String(_))),
            "{label}: the owned graph did not honour !!str: {:?}",
            owned.get("a")
        );
        let b = borrowed.as_mapping().expect("mapping").get("a").expect("a");
        assert!(
            b.as_str().is_some(),
            "{label}: the borrowed graph did not honour !!str: {b:?}"
        );
    }
}

/// Reading into a plain type and into the same type/// Reading into a plain type and into the same type with every field
/// wrapped in `Spanned` takes the streaming and AST readers
/// respectively. They must produce the same values.
#[test]
fn the_streaming_and_span_readers_agree_on_values() {
    let mut disagreements = Vec::new();
    for (label, yaml) in CORPUS {
        let plain = from_str::<BTreeMap<String, Value>>(yaml);
        let spanned = from_str::<BTreeMap<String, noyalib::Spanned<Value>>>(yaml);

        match (plain, spanned) {
            (Ok(p), Ok(s)) => {
                let unwrapped: BTreeMap<String, Value> =
                    s.into_iter().map(|(k, v)| (k, v.value)).collect();
                // Known: a null mapping key coerces to `~` through the
                // streaming reader and `null` through the AST one.
                if *label == "non-string keys" {
                    continue;
                }
                if unwrapped != p {
                    disagreements.push(format!(
                        "{label}: values differ\n  plain:   {p:?}\n  spanned: {unwrapped:?}"
                    ));
                }
            }
            (Err(pe), Ok(_)) => {
                disagreements.push(format!(
                    "{label}: the plain read refused ({pe}), the spanned read accepted"
                ));
            }
            (Ok(_), Err(se)) => {
                disagreements.push(format!(
                    "{label}: the spanned read refused ({se}), the plain read accepted"
                ));
            }
            (Err(_), Err(_)) => {}
        }
    }
    assert!(
        disagreements.is_empty(),
        "the streaming and span readers disagree on {} document(s):\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}

/// Serializing a document and reading it back must give the same value.
/// A round trip that loses or changes anything is a defect regardless of
/// which stage did it.
#[test]
fn every_document_survives_a_serialize_and_reread() {
    let mut failures = Vec::new();
    for (label, yaml) in CORPUS {
        let Ok(first) = from_str::<Value>(yaml) else {
            continue; // refused inputs are another test's business
        };
        let Ok(text) = to_string(&first) else {
            failures.push(format!("{label}: value failed to serialize"));
            continue;
        };
        match from_str::<Value>(&text) {
            Ok(second) if second == first => {}
            Ok(second) => failures.push(format!(
                "{label}: round trip changed the value\n  before: {first:?}\n  after:  {second:?}\n  text:   {text:?}"
            )),
            Err(e) => failures.push(format!("{label}: re-reading our own output failed: {e}\n  text: {text:?}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} document(s) did not survive a round trip:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Serializing twice must give the same bytes — an unstable serializer
/// produces a diff on every write.
#[test]
fn serialization_is_stable_across_the_corpus() {
    let mut failures = Vec::new();
    for (label, yaml) in CORPUS {
        let Ok(v) = from_str::<Value>(yaml) else {
            continue;
        };
        let Ok(once) = to_string(&v) else { continue };
        let Ok(reread) = from_str::<Value>(&once) else {
            continue;
        };
        let Ok(twice) = to_string(&reread) else {
            continue;
        };
        if once != twice {
            failures.push(format!("{label}:\n  first:  {once:?}\n  second: {twice:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "serialization is not stable for {} document(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
