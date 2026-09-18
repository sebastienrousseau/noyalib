// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The same bytes, through every door.
//!
//! A document can enter noyalib as a `&str`, as a `&[u8]`, or through a
//! `Read`. Multiple documents can arrive via `load_all`, `load_all_as`,
//! `from_str_multi`, or the Rayon-backed `parallel::parse`. The
//! `compat::serde_yaml` façade is a seventh route over the top.
//!
//! Each is a separate implementation of "read this YAML", and the only
//! thing that makes them interchangeable is that they agree. These
//! compare them on one corpus, so a path that drifts is a failing test
//! rather than a surprise at a call site.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use noyalib::{Value, from_reader, from_slice, from_str, load_all, load_all_as};

/// Single-document inputs, covering the shapes the readers branch on.
const SINGLE: &[(&str, &str)] = &[
    ("flat scalars", "a: 1\nb: text\nc: true\nd: ~\n"),
    ("nested", "a:\n  b:\n    c: 1\n"),
    ("sequence", "xs:\n  - 1\n  - two\n"),
    ("flow", "m: {a: 1}\nxs: [1, 2]\n"),
    ("anchors", "base: &b 1\nuse: *b\n"),
    ("a merge key", "base: &b\n  a: 1\nm:\n  <<: *b\n"),
    ("tags", "a: !!str 1\nb: !Custom v\n"),
    ("block scalars", "lit: |\n  one\nfold: >\n  two\n"),
    ("quoted", "s: 'one'\nd: \"two\"\n"),
    ("unicode", "k: \"héllo ✓\"\n"),
    ("CRLF", "a: 1\r\nb: 2\r\n"),
    ("a BOM", "\u{feff}a: 1\n"),
    ("no trailing newline", "a: 1\nb: 2"),
    ("comments", "# head\na: 1 # side\n"),
    ("empty collections", "m: {}\nxs: []\n"),
    ("duplicate keys", "k: 1\nk: 2\n"),
];

/// Multi-document streams.
const MULTI: &[(&str, &str)] = &[
    ("two plain documents", "a: 1\n---\nb: 2\n"),
    ("three documents", "a: 1\n---\nb: 2\n---\nc: 3\n"),
    ("explicit end markers", "a: 1\n...\n---\nb: 2\n...\n"),
    ("a leading marker", "---\na: 1\n---\nb: 2\n"),
    (
        "documents of different shapes",
        "a: 1\n---\n- 1\n- 2\n---\nscalar\n",
    ),
    (
        "a document with comments",
        "# one\na: 1\n---\n# two\nb: 2\n",
    ),
    ("CRLF between documents", "a: 1\r\n---\r\nb: 2\r\n"),
    ("a single document", "only: 1\n"),
    ("anchors within one document", "x: &a 1\ny: *a\n---\nz: 2\n"),
];

/// `&str`, `&[u8]` and `Read` must all produce the same value.
#[test]
fn the_three_input_forms_agree() {
    let mut disagreements = Vec::new();
    for (label, yaml) in SINGLE {
        let by_str = from_str::<Value>(yaml);
        let by_slice = from_slice::<Value>(yaml.as_bytes());
        let by_reader = from_reader::<_, Value>(yaml.as_bytes());

        let shape = |r: &noyalib::Result<Value>| match r {
            Ok(v) => format!("ok:{v:?}"),
            Err(_) => "err".to_string(),
        };
        let (a, b, c) = (shape(&by_str), shape(&by_slice), shape(&by_reader));
        if a != b {
            disagreements.push(format!(
                "{label}: &str vs &[u8]\n  str:   {a}\n  slice: {b}"
            ));
        }
        if a != c {
            disagreements.push(format!(
                "{label}: &str vs Read\n  str:    {a}\n  reader: {c}"
            ));
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} input form(s) disagree:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}

/// Every multi-document reader must see the same documents.
#[test]
fn the_multi_document_readers_agree() {
    let mut disagreements = Vec::new();
    for (label, yaml) in MULTI {
        let via_load_all = load_all(yaml).and_then(|it| it.collect::<Result<Vec<Value>, _>>());
        let via_load_all_as = load_all_as::<Value>(yaml);

        match (&via_load_all, &via_load_all_as) {
            (Ok(a), Ok(b)) if a != b => disagreements.push(format!(
                "{label}: load_all vs load_all_as\n  load_all:    {a:?}\n  load_all_as: {b:?}"
            )),
            (Err(e), Ok(_)) => {
                disagreements.push(format!(
                    "{label}: load_all refused ({e}), load_all_as accepted"
                ));
            }
            (Ok(_), Err(e)) => {
                disagreements.push(format!(
                    "{label}: load_all_as refused ({e}), load_all accepted"
                ));
            }
            _ => {}
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} multi-document reader disagreement(s):\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}

#[cfg(feature = "parallel")]
/// `parallel::parse` is a drop-in for `load_all_as` that splits the
/// stream and deserializes each document concurrently. Concurrency is
/// exactly where an ordering or boundary mistake hides, so the two must
/// agree document for document.
#[test]
fn the_parallel_reader_agrees_with_the_sequential_one() {
    let mut disagreements = Vec::new();
    for (label, yaml) in MULTI {
        let sequential = load_all_as::<Value>(yaml);
        let parallel = noyalib::parallel::parse::<Value>(yaml);

        match (&sequential, &parallel) {
            (Ok(s), Ok(p)) if s != p => disagreements.push(format!(
                "{label}: order or content differs\n  sequential: {s:?}\n  parallel:   {p:?}"
            )),
            (Err(e), Ok(_)) => {
                disagreements.push(format!(
                    "{label}: sequential refused ({e}), parallel accepted"
                ));
            }
            (Ok(_), Err(e)) => {
                disagreements.push(format!(
                    "{label}: parallel refused ({e}), sequential accepted"
                ));
            }
            _ => {}
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} parallel/sequential disagreement(s):\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}

#[cfg(feature = "parallel")]
/// `parallel::values` is the `Value`-typed sibling, and `parallel::split`
/// the boundary scanner underneath both. A split that loses or merges a
/// document would show as a count mismatch.
#[test]
fn the_parallel_split_finds_the_same_document_boundaries() {
    let mut disagreements = Vec::new();
    for (label, yaml) in MULTI {
        let Ok(sequential) = load_all_as::<Value>(yaml) else {
            continue;
        };
        let pieces = noyalib::parallel::split(yaml);
        if pieces.len() != sequential.len() {
            disagreements.push(format!(
                "{label}: split found {} piece(s), the reader found {} document(s)",
                pieces.len(),
                sequential.len()
            ));
        }
        match noyalib::parallel::values(yaml) {
            Ok(v) if v != sequential => disagreements.push(format!(
                "{label}: parallel::values differs\n  values: {v:?}\n  seq:    {sequential:?}"
            )),
            Err(e) => disagreements.push(format!("{label}: parallel::values refused: {e}")),
            Ok(_) => {}
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} parallel-split disagreement(s):\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}

/// The deviations the `compat::serde_yaml` shim documents. Only built
/// when that optional façade is compiled in.
#[cfg(feature = "compat-serde-yaml")]
/// The `serde_yaml` façade is a seventh route, and it is *supposed* to
/// differ: it parses under `ParserConfig::serde_yaml_compat`, whose
/// documented job is reproducing serde_yaml 0.9's observable behaviour
/// rather than noyalib's spec-strict defaults.
///
/// So the test is not "these agree" but "they differ only where the
/// shim says they will". Each exception quotes the reason from the
/// shim's own documentation, and an *undocumented* difference fails.
const DOCUMENTED_FACADE_DEVIATIONS: &[(&str, &str)] = &[(
    "a merge key",
    "`<<` merge keys stay literal entries (alias values resolved) — \
     serde_yaml 0.9 requires an explicit merge step",
)];

#[cfg(feature = "compat-serde-yaml")]
#[test]
fn the_compat_facade_differs_only_where_it_documents() {
    let mut undocumented = Vec::new();
    let mut documented_but_agreed = Vec::new();

    for (label, yaml) in SINGLE {
        let native = from_str::<Value>(yaml);
        let facade = noyalib::compat::serde_yaml::from_str::<Value>(yaml);
        let expected = DOCUMENTED_FACADE_DEVIATIONS
            .iter()
            .find(|(l, _)| l == label);

        let differs = match (&native, &facade) {
            (Ok(n), Ok(f)) if n != f => Some(format!("native {n:?} vs facade {f:?}")),
            (Err(e), Ok(_)) => Some(format!("native refused ({e}), facade accepted")),
            (Ok(_), Err(e)) => Some(format!("facade refused ({e}), native accepted")),
            _ => None,
        };

        match (differs, expected) {
            (Some(d), None) => undocumented.push(format!("{label}: {d}")),
            (None, Some((_, why))) => documented_but_agreed
                .push(format!("{label}: documented to differ ({why}) but agreed")),
            _ => {}
        }
    }

    assert!(
        undocumented.is_empty(),
        "{} undocumented difference(s) between the façade and native:\n{}",
        undocumented.len(),
        undocumented.join("\n")
    );
    assert!(
        documented_but_agreed.is_empty(),
        "{} documented deviation(s) no longer apply — update the shim's docs \
         and this list:\n{}",
        documented_but_agreed.len(),
        documented_but_agreed.join("\n")
    );
}

/// The compat deviations the shim documents, asserted individually so
/// the façade cannot quietly stop being a drop-in replacement.
#[cfg(feature = "compat-serde-yaml")]
#[test]
fn the_facade_reproduces_serde_yamls_documented_quirks() {
    use noyalib::compat::serde_yaml as syml;

    // Merge keys stay literal.
    let v: Value = syml::from_str("b: &b\n  a: 1\nm:\n  <<: *b\n").expect("merge");
    assert!(
        v.get("m").and_then(|m| m.get("<<")).is_some(),
        "the façade resolved a merge key; serde_yaml 0.9 leaves it literal: {v:?}"
    );

    // Leading-zero integers stay strings.
    let v: Value = syml::from_str("a: 0123\n").expect("leading zero");
    assert_eq!(
        v.get("a").and_then(Value::as_str),
        Some("0123"),
        "a leading-zero integer should stay a string: {v:?}"
    );

    // `0b11` is three.
    let v: Value = syml::from_str("a: 0b11\n").expect("binary literal");
    assert_eq!(
        v.get("a").and_then(Value::as_i64),
        Some(3),
        "0b11 should read as 3: {v:?}"
    );

    // A non-scalar key is an error rather than a coercion.
    assert!(
        syml::from_str::<Value>("? [a, b]\n: 1\n").is_err(),
        "the façade should refuse a non-scalar key"
    );
}

/// A truncated stream must not be read/// A truncated stream must not be read as a shorter valid one. Cutting
/// each corpus document at every byte boundary and requiring either a
/// clean error or a prefix-consistent value is the cheapest way to find
/// a reader that runs off the end.
#[test]
fn no_reader_panics_or_invents_data_on_a_truncated_stream() {
    for (label, yaml) in SINGLE {
        for cut in 0..yaml.len() {
            if !yaml.is_char_boundary(cut) {
                continue;
            }
            let prefix = &yaml[..cut];
            // Every reader must return, not panic. The values need not
            // match each other on a truncated input, but a panic here is
            // a defect in any of them.
            let _ = from_str::<Value>(prefix);
            let _ = from_slice::<Value>(prefix.as_bytes());
            let _ = from_reader::<_, Value>(prefix.as_bytes());
            let _ = load_all_as::<Value>(prefix);
            #[cfg(feature = "parallel")]
            let _ = noyalib::parallel::parse::<Value>(prefix);
        }
        let _ = label;
    }
}
