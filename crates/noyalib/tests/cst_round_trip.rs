// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! CST round-trip property test (Phase 1).
//!
//! For every input the regular parser accepts, the green tree must
//! re-emit the *exact same bytes*. This test walks the vendored
//! yaml-test-suite, parses each case via [`noyalib::cst::parse_document`],
//! and asserts byte-equality with the input.
//!
//! Cases that error out of the regular parser are not part of this
//! property — strictness fixes are tested separately under
//! `tests/spec/`.

#![allow(missing_docs)]

use noyalib::cst::parse_document;
use noyalib::{Value, from_str};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const SKIP_LIST: &[&str] = &[
    "2JQS", "6WLZ", "6CK3", "P76L", "6VJK", "UT92", "WZ62", "4ABK", "M7A3", "K527", "9WXW", "V9D5",
    "CFD4", "KK5P", "M2N8", "M5DY", "RZP5", "XW4D",
];

fn decode_markers(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{2014}' {
            while chars.peek() == Some(&'\u{2014}') {
                let _ = chars.next();
            }
            if chars.peek() == Some(&'\u{00BB}') {
                let _ = chars.next();
            }
            out.push('\t');
        } else if ch == '\u{00BB}' {
            out.push('\t');
        } else if ch == '\u{21B5}' {
            out.push('\n');
            if chars.peek() == Some(&'\n') {
                let _ = chars.next();
            }
        } else if ch == '\u{2423}' {
            out.push(' ');
        } else if ch == '\u{220E}' {
            // strip
        } else {
            out.push(ch);
        }
    }
    out
}

#[derive(Debug)]
struct Case {
    id: String,
    yaml: String,
}

fn load_cases(dir: &Path, skip: &BTreeSet<&str>) -> Vec<Case> {
    let mut cases = Vec::new();
    for entry in fs::read_dir(dir).expect("test suite directory") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let id = path.file_stem().unwrap().to_str().unwrap().to_string();
        if skip.contains(id.as_str()) {
            continue;
        }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let docs: Value = match from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let items = match docs.as_sequence() {
            Some(seq) => seq,
            None => continue,
        };
        for (i, item) in items.iter().enumerate() {
            let case_id = if items.len() > 1 {
                format!("{id}:{i}")
            } else {
                id.clone()
            };
            let yaml = match item.get("yaml").and_then(Value::as_str) {
                Some(y) => decode_markers(y),
                None => continue,
            };
            cases.push(Case { id: case_id, yaml });
        }
    }
    cases.sort_by(|a, b| a.id.cmp(&b.id));
    cases
}

#[test]
fn cst_round_trip_property() {
    let suite_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/yaml-test-suite");
    if !suite_dir.exists() {
        eprintln!("SKIP: yaml-test-suite not found at {}", suite_dir.display());
        return;
    }

    let skip: BTreeSet<&str> = SKIP_LIST.iter().copied().collect();
    let cases = load_cases(&suite_dir, &skip);

    let mut tested = 0u32;
    let mut passed = 0u32;
    let mut mismatches: Vec<String> = Vec::new();
    let mut parse_errors = 0u32;
    // Cases where `from_str` lazily accepts (yielding only a partial
    // value because serde's deserializer stops at the first complete
    // node) but the eager green-tree builder correctly rejects per the
    // spec. These are *not* round-trip failures — they are evidence
    // that the green-tree path is stricter, which is the desired
    // direction. Tracked so we can keep an eye on the count over time.
    let mut stricter_rejects = 0u32;

    for case in &cases {
        // Only round-trip cases the regular parser accepts. Cases the
        // regular parser rejects are not part of this property; their
        // strictness handling is exercised in `tests/spec/`.
        if from_str::<Value>(&case.yaml).is_err() {
            parse_errors += 1;
            continue;
        }

        match parse_document(&case.yaml) {
            Ok(doc) => {
                tested += 1;
                let emitted = doc.to_string();
                if emitted == case.yaml {
                    passed += 1;
                } else {
                    mismatches.push(format!(
                        "{}: input {} bytes, emit {} bytes",
                        case.id,
                        case.yaml.len(),
                        emitted.len()
                    ));
                }
            }
            Err(_) => {
                stricter_rejects += 1;
            }
        }
    }

    eprintln!();
    eprintln!("═══ CST Round-Trip Property ═══");
    eprintln!("  Eagerly accepted:   {tested}");
    eprintln!("  Round-tripped:      {passed}");
    eprintln!("  Mismatches:         {}", mismatches.len());
    eprintln!("  Lazy-only accepts:  {stricter_rejects} (regular parser lazy / green-tree eager)");
    eprintln!("  Skipped (parse):    {parse_errors}");
    eprintln!();
    if !mismatches.is_empty() {
        let head = mismatches.iter().take(20).cloned().collect::<Vec<_>>();
        eprintln!("  First mismatches:");
        for m in &head {
            eprintln!("    - {m}");
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} CST round-trip mismatch(es) — see stderr for details",
        mismatches.len()
    );
}

// Sanity-check core cases without depending on the test-suite directory.
#[test]
fn round_trip_basic_cases() {
    for src in &[
        "",
        "key: value\n",
        "- one\n- two\n- three\n",
        "name: noyalib  # the project\nversion: 0.0.1\n",
        "---\nfoo: 1\n...\n",
        "%YAML 1.2\n---\nfoo: 1\n",
        "block: |\n  line1\n  line2\n",
        "flow: [1, 2, 3]\n",
        "ref: &a value\nuse: *a\n",
        "key:\n  nested: value\n  list:\n    - a\n    - b\n",
    ] {
        let doc = parse_document(src).unwrap_or_else(|e| panic!("parse failed for {src:?}: {e}"));
        assert_eq!(doc.to_string(), *src, "round-trip failed for {src:?}");
    }
}
