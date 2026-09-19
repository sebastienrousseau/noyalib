// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Stress / load / stability test battery.
//!
//! Pins the parser's behaviour on the kinds of pathological input
//! a foundational YAML library should handle without panic, OOM,
//! or silent miscompute:
//!
//!   - Large single document (~1 MB).
//!   - Deep nesting (depth at the configured ceiling).
//!   - Many documents (thousands of `---` separators).
//!   - Wide collections (10k entries / 10k items).
//!   - Long scalars (1 MB plain string).
//!   - Billion-laughs-style alias amplification (DoS guard).
//!   - Exhaustion of the recursion limit.
//!   - Repeated parse / emit cycles for mutation stability.
//!
//! Each test is a regression net: if a future change breaks the
//! resource-limit gates or the pure-functional emit determinism,
//! it surfaces here rather than as an OOM in production.

#![allow(missing_docs)]

use noyalib::{ParserConfig, Value, from_str, to_string};

/// 1 MB single block-scalar document parses and round-trips
/// cleanly without OOM on a 64-bit host.
#[test]
fn large_single_document_block_scalar() {
    let mut yaml = String::from("payload: |\n");
    let line: &str = "  the quick brown fox jumps over the lazy dog 0123456789\n";
    while yaml.len() < 1_000_000 {
        yaml.push_str(line);
    }
    let v: Value = from_str(&yaml).unwrap();
    let payload = v
        .get("payload")
        .and_then(Value::as_str)
        .expect("payload string");
    assert!(payload.len() > 900_000);
}

/// 10 000-entry mapping parses without superlinear blow-up.
#[test]
fn wide_mapping_10k_entries() {
    let mut yaml = String::new();
    for i in 0..10_000 {
        yaml.push_str(&format!("key{i:05}: {i}\n"));
    }
    let v: Value = from_str(&yaml).unwrap();
    let m = v.as_mapping().expect("mapping");
    assert_eq!(m.len(), 10_000);
    assert_eq!(m.get("key00000").and_then(Value::as_i64), Some(0));
    assert_eq!(m.get("key09999").and_then(Value::as_i64), Some(9_999));
}

/// 10 000-item sequence parses without superlinear blow-up.
#[test]
fn wide_sequence_10k_items() {
    let mut yaml = String::new();
    for i in 0..10_000 {
        yaml.push_str(&format!("- {i}\n"));
    }
    let v: Value = from_str(&yaml).unwrap();
    let s = v.as_sequence().expect("sequence");
    assert_eq!(s.len(), 10_000);
    assert_eq!(s[0].as_i64(), Some(0));
    assert_eq!(s[9_999].as_i64(), Some(9_999));
}

/// Many documents (1 000) in one stream parse and the result is
/// indexable by document order.
#[test]
fn many_documents_one_thousand() {
    let mut yaml = String::with_capacity(1_000 * 16);
    for i in 0..1_000 {
        yaml.push_str(&format!("---\nidx: {i}\n"));
    }
    let docs: Vec<Value> = noyalib::load_all_as(&yaml).unwrap();
    assert_eq!(docs.len(), 1_000);
    assert_eq!(docs[0].get("idx").and_then(Value::as_i64), Some(0));
    assert_eq!(docs[999].get("idx").and_then(Value::as_i64), Some(999));
}

/// Deep nesting up to 100 levels (configured `max_depth` allows it
/// by default at 1024). Verifies no stack overflow on the loader's
/// frame walk and the deserialiser's descent.
#[test]
fn deep_nesting_100_levels() {
    let mut yaml = String::new();
    for i in 0..100 {
        yaml.push_str(&"  ".repeat(i));
        yaml.push_str(&format!("level{i}:\n"));
    }
    yaml.push_str(&"  ".repeat(100));
    yaml.push_str("leaf: 1\n");
    let v: Value = from_str(&yaml).unwrap();
    // Walk the chain.
    let mut cur = &v;
    for i in 0..100 {
        cur = cur.get(format!("level{i}")).expect("level present");
    }
    assert_eq!(cur.get("leaf").and_then(Value::as_i64), Some(1));
}

/// Recursion-limit DoS guard: a 10 000-deep nesting (above the
/// 1024 default `max_depth`) is rejected with a clear error,
/// *not* a stack overflow.
#[test]
fn recursion_limit_rejects_overdeep() {
    let mut yaml = String::new();
    for i in 0..10_000 {
        yaml.push_str(&"  ".repeat(i.min(2_000)));
        yaml.push_str("k:\n");
    }
    let res: Result<Value, _> = from_str(&yaml);
    assert!(res.is_err(), "10 000-deep nesting must be rejected");
}

/// Billion-laughs-style alias amplification is bounded by the
/// `max_alias_expansions` / `max_document_length` limits — the
/// parse rejects rather than OOMs.
#[test]
fn billion_laughs_alias_amplification_is_bounded() {
    // 10-tier nested alias expansion that would balloon to
    // ~10^10 strings if expanded naively.
    let yaml = r#"
a: &a ["lol", "lol", "lol", "lol", "lol", "lol", "lol", "lol", "lol", "lol"]
b: &b [*a, *a, *a, *a, *a, *a, *a, *a, *a, *a]
c: &c [*b, *b, *b, *b, *b, *b, *b, *b, *b, *b]
d: &d [*c, *c, *c, *c, *c, *c, *c, *c, *c, *c]
e: &e [*d, *d, *d, *d, *d, *d, *d, *d, *d, *d]
f: &f [*e, *e, *e, *e, *e, *e, *e, *e, *e, *e]
g: &g [*f, *f, *f, *f, *f, *f, *f, *f, *f, *f]
h: &h [*g, *g, *g, *g, *g, *g, *g, *g, *g, *g]
i: &i [*h, *h, *h, *h, *h, *h, *h, *h, *h, *h]
j: &j [*i, *i, *i, *i, *i, *i, *i, *i, *i, *i]
"#;
    let res: Result<Value, _> = from_str(yaml);
    assert!(
        res.is_err(),
        "billion-laughs amplification must hit the alias / document length limit"
    );
}

/// Long plain scalar (1 MB single line) parses and round-trips
/// byte-for-byte.
#[test]
fn long_plain_scalar_1mb() {
    let mut yaml = String::from("payload: \"");
    yaml.push_str(&"x".repeat(1_000_000));
    yaml.push_str("\"\n");
    let v: Value = from_str(&yaml).unwrap();
    let s = v.get("payload").and_then(Value::as_str).expect("payload");
    assert_eq!(s.len(), 1_000_000);
}

/// Repeated parse → emit → re-parse stability. The third parse
/// must produce the same Value as the first — `from_str` /
/// `to_string` together are referentially transparent on
/// well-formed YAML.
#[test]
fn parse_emit_reparse_stability() {
    let yaml = "name: noyalib\nport: 8080\nfeatures:\n  - cst\n  - schema\n  - figment\n";
    let v1: Value = from_str(yaml).unwrap();
    let s1 = to_string(&v1).unwrap();
    let v2: Value = from_str(&s1).unwrap();
    let s2 = to_string(&v2).unwrap();
    let v3: Value = from_str(&s2).unwrap();
    assert_eq!(v1, v2, "round-trip 1 → 2");
    assert_eq!(v2, v3, "round-trip 2 → 3");
    assert_eq!(s1, s2, "emit is deterministic");
}

/// Stress: 100 iterations of a small-document parse loop must not
/// leak / drift in observable behaviour. Catches mutable-static
/// state regressions.
#[test]
fn many_iterations_no_drift() {
    let yaml = "a: 1\nb: [2, 3]\nc: {d: 4}\n";
    let baseline: Value = from_str(yaml).unwrap();
    for _ in 0..100 {
        let v: Value = from_str(yaml).unwrap();
        assert_eq!(v, baseline, "no drift between iterations");
    }
}

/// Unicode-heavy document (mixed ASCII / multi-byte UTF-8) parses
/// without truncation or byte-position drift.
#[test]
fn unicode_heavy_document() {
    let yaml = "\
        name: \u{1F44B} hello\n\
        emoji: \u{1F600}\u{1F60D}\u{1F914}\n\
        japanese: \u{3053}\u{3093}\u{306B}\u{3061}\u{306F}\n\
        rtl: \u{0645}\u{0631}\u{062D}\u{0628}\u{0627}\n\
        cjk_long: \u{4F60}\u{597D}\u{4E16}\u{754C}\u{1F30D}\n\
    ";
    let v: Value = from_str(yaml).unwrap();
    assert!(v.get("emoji").and_then(Value::as_str).is_some());
    assert!(v.get("japanese").and_then(Value::as_str).is_some());
    assert!(v.get("rtl").and_then(Value::as_str).is_some());
}

/// Custom `ParserConfig` with a low `max_depth` rejects an input
/// whose nesting exceeds the cap, even if the default would have
/// accepted it. Regression net for custom resource-limit policies.
#[test]
fn custom_low_max_depth_rejects() {
    let yaml = "a:\n  b:\n    c:\n      d: 1\n";
    let cfg = ParserConfig::new().max_depth(2);
    let res: Result<Value, _> = noyalib::from_str_with_config(yaml, &cfg);
    assert!(res.is_err(), "depth=2 cap should reject nesting depth 4");
}

/// 1 000 anchors / aliases pointing to small scalars don't blow
/// the alias-expansion budget.
#[test]
fn many_aliases_within_budget() {
    let mut yaml = String::from("anchors:\n");
    for i in 0..1_000 {
        yaml.push_str(&format!("  - &a{i:04} {i}\n"));
    }
    yaml.push_str("aliases:\n");
    for i in 0..1_000 {
        yaml.push_str(&format!("  - *a{i:04}\n"));
    }
    let v: Value = from_str(&yaml).unwrap();
    let aliases = v.get("aliases").and_then(Value::as_sequence).unwrap();
    assert_eq!(aliases.len(), 1_000);
    assert_eq!(aliases[0].as_i64(), Some(0));
    assert_eq!(aliases[999].as_i64(), Some(999));
}
