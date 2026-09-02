//! Whole-tree span invariants, adapted from the whole-tree
//! position proofs in zfb's `parse_to_ast` tests: every resolvable
//! path's span must be in-bounds, char-boundary aligned, and
//! contained by its parent's span. Hardened after the v0.0.30 span
//! model change (node spans start at their `!tag`/`&anchor`
//! properties) — these invariants are exactly what downstream
//! coordinate mapping builds on.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![cfg(feature = "std")]

use noyalib::Value;
use noyalib::cst::parse_document;
use proptest::prelude::*;

/// Enumerate every path in a value tree in `Document` path syntax.
fn paths(value: &Value, prefix: &str, out: &mut Vec<String>) {
    match value {
        Value::Mapping(m) => {
            for (k, v) in m {
                let key: &str = k;
                // Path syntax cannot address every key spelling;
                // stick to plain identifiers.
                if key.is_empty()
                    || !key
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    continue;
                }
                let p = if prefix.is_empty() {
                    key.to_owned()
                } else {
                    format!("{prefix}.{key}")
                };
                out.push(p.clone());
                paths(v, &p, out);
            }
        }
        Value::Sequence(s) => {
            for (i, v) in s.iter().enumerate() {
                let p = format!("{prefix}[{i}]");
                out.push(p.clone());
                paths(v, &p, out);
            }
        }
        Value::Tagged(t) => paths(t.value(), prefix, out),
        _ => {}
    }
}

/// Check every invariant for one document.
fn check(src: &str) {
    let Ok(doc) = parse_document(src) else {
        return; // not this test's job to decide what parses
    };
    assert_eq!(doc.to_string(), src, "byte-faithful round-trip");
    let value = doc.as_value();
    let mut all = Vec::new();
    paths(&value, "", &mut all);
    for path in &all {
        let Some((s, e)) = doc.span_at(path) else {
            continue; // implicit nulls report no span, by design
        };
        assert!(s <= e, "{path}: inverted span {s}..{e}");
        assert!(e <= src.len(), "{path}: span {s}..{e} out of bounds");
        assert!(
            src.is_char_boundary(s) && src.is_char_boundary(e),
            "{path}: span {s}..{e} splits a character"
        );
        // Containment: a child's span sits inside its parent's.
        //
        // KNOWN QUIRK (pre-dates v0.0.30, reproduced on the published
        // crate; tracked in #375): a block sequence appearing as a
        // mapping value records only its first `-` indicator as its
        // span (`seq:` in `seq:\n- 1` reports 5..6), while nested
        // block sequences record their full extent. Until that is
        // fixed on the breaking axis, containment is only asserted
        // against parents whose span actually covers their first
        // child - the degenerate indicator-only shape is tolerated
        // but everything else must nest.
        if let Some(dot) = path.rfind(['.', '[']) {
            let parent = &path[..dot];
            if !parent.is_empty() {
                if let Some((ps, pe)) = doc.span_at(parent) {
                    let degenerate_block_seq = pe <= s && pe - ps <= 2;
                    assert!(
                        (s >= ps && e <= pe) || degenerate_block_seq,
                        "{path} span {s}..{e} escapes parent {parent} span {ps}..{pe}"
                    );
                }
            }
        }
    }
}

#[test]
fn tricky_documents_hold_the_invariants() {
    for src in [
        "a: 1\n",
        "a: &x 1\nb: *x\n",
        "a: !tag value\n",
        "a: !tag &both [1, 2]\n",
        "outer:\n  mid:\n    inner: [a, {b: c}]\n",
        "seq:\n- 1\n- - nested\n  - deep\n",
        "unicode: \"日本語 😀\"\nafter: ok\n",
        "block: |\n  line one\n  line two\nafter: x\n",
        "flow: {a: 1, b: [2, 3], c: {d: 4}}\n",
        "défaults:\n  clé: &a valeur\ncopie:\n  clé2: *a\n",
    ] {
        check(src);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Serialize an arbitrary tree, parse it losslessly, and hold
    /// the invariants over every addressable node.
    #[test]
    fn generated_documents_hold_the_invariants(
        keys in proptest::collection::vec("[a-z][a-z0-9_]{0,6}", 1..5),
        ints in proptest::collection::vec(any::<i32>(), 1..5),
        strs in proptest::collection::vec("[ -~]{0,12}", 1..4),
        nest in 0usize..3,
    ) {
        let mut m = noyalib::Mapping::new();
        for (i, k) in keys.iter().enumerate() {
            let v = match i % 3 {
                0 => Value::Number(ints[i % ints.len()].into()),
                1 => Value::String(strs[i % strs.len()].clone()),
                _ => {
                    let mut inner = noyalib::Mapping::new();
                    let mut cur = Value::String("leaf".into());
                    for d in 0..nest {
                        let mut wrap = noyalib::Mapping::new();
                        let _ = wrap.insert(format!("level{d}"), cur);
                        cur = Value::Mapping(wrap);
                    }
                    let _ = inner.insert("nested", cur);
                    Value::Mapping(inner)
                }
            };
            let _ = m.insert(k.clone(), v);
        }
        let src = noyalib::to_string(&Value::Mapping(m)).unwrap();
        check(&src);
    }
}
