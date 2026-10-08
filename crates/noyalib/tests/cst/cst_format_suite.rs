// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `cst::format` over every yaml-test-suite case: the output of a
//! document that parses must parse to the same values, or `format`
//! must refuse it. It must never turn a valid document invalid or
//! change what it means.

use noyalib::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Suite cases the formatter refuses (returns `Err`) because it cannot
/// re-layout them without changing their meaning. Refusing is the
/// documented outcome for these; corrupting them is not. Shrink this
/// list as the formatter learns the shapes.
const REFUSED: &[&str] = &[
    // A flow sequence whose continuation lines sit one column right of
    // a key the formatter re-indents by one column: kept verbatim, the
    // lines would no longer be indented past the key.
    "6HB6#0",
    // A literal scalar whose last line is spaces with no line break:
    // the spaces are content (one past the indent), but the formatter
    // trims a spaces-only last line.
    "L24T#1",
];

fn decode_markers(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '␣' => out.push(' '),
            '⇥' | '»' => out.push('\t'),
            '↵' => {
                out.push('\n');
                if chars.peek() == Some(&'\n') {
                    let _ = chars.next();
                }
            }
            '↓' => out.push('\r'),
            '⇔' => out.push('\u{feff}'),
            '∎' => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
            }
            '—' => {
                let mut count = 1;
                while chars.peek() == Some(&'—') {
                    let _ = chars.next();
                    count += 1;
                }
                if chars.peek() == Some(&'»') {
                    let _ = chars.next();
                    out.push('\t');
                } else {
                    out.push_str(&"—".repeat(count));
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Every `(id, yaml)` in the vendored suite, plus a few shapes the
/// suite does not cover.
fn cases() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = [
        ("flow-comment", "{a: 1, # c\n b: 2}\n"),
        ("tab-sep", "a:\t1\n"),
        ("hash-in-plain", "a: b#c\n"),
        ("lit-indent", "a: |2\n   x\n"),
        ("multi", "a: 1\n---\nb: 2\n...\n"),
        ("crlf", "a: 1\r\nb: |\r\n  x\r\n"),
        ("bom", "\u{FEFF}a: 1\n"),
        ("explicit", "? a\n: b\n? [c]\n: d\n"),
        ("tagdir", "%TAG !e! tag:e.com,2000:\n---\na: !e!x 1\n"),
    ]
    .iter()
    .map(|(n, y)| ((*n).to_owned(), (*y).to_owned()))
    .collect();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/yaml-test-suite");
    let mut paths: Vec<_> = fs::read_dir(dir)
        .expect("vendored suite")
        .map(|e| e.expect("entry").path())
        .collect();
    paths.sort();
    for path in paths {
        let text = fs::read_to_string(&path).expect("read case");
        let Ok(Value::Sequence(items)) = noyalib::from_str::<Value>(&text) else {
            continue;
        };
        let stem = path
            .file_stem()
            .expect("stem")
            .to_string_lossy()
            .into_owned();
        for (i, item) in items.iter().enumerate() {
            if let Some(y) = item.get("yaml").and_then(Value::as_str) {
                out.push((format!("{stem}#{i}"), decode_markers(y)));
            }
        }
    }
    out
}

fn load(y: &str) -> Result<Vec<Value>, String> {
    noyalib::load_all_as::<Value>(y).map_err(|e| e.to_string())
}

#[test]
fn format_preserves_the_meaning_of_every_suite_document() {
    let mut problems = Vec::new();
    let mut refused = BTreeSet::new();
    let mut checked = 0;
    for (name, y) in cases() {
        let Ok(before) = load(&y) else {
            continue;
        };
        checked += 1;
        let Ok(formatted) = noyalib::cst::format(&y) else {
            let _ = refused.insert(name);
            continue;
        };
        match load(&formatted) {
            Ok(after) if after == before => {}
            Ok(after) => problems.push(format!(
                "CHANGED {name}\n  in : {y:?}\n  out: {formatted:?}\n  {before:?}\n  {after:?}"
            )),
            Err(e) => problems.push(format!(
                "NOW INVALID {name}\n  in : {y:?}\n  out: {formatted:?}\n  {e}"
            )),
        }
    }
    assert!(checked > 250, "only {checked} suite documents parsed");
    assert!(
        problems.is_empty(),
        "{}\n{}",
        problems.len(),
        problems.join("\n")
    );
    let expected: BTreeSet<String> = REFUSED.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(refused, expected, "the set of refused cases changed");
}

#[test]
fn format_is_idempotent_on_what_it_accepts() {
    for (name, y) in cases() {
        let Ok(once) = noyalib::cst::format(&y) else {
            continue;
        };
        if load(&y).is_err() {
            continue;
        }
        let twice = noyalib::cst::format(&once)
            .unwrap_or_else(|e| panic!("{name}: formatted output refused: {e}\n{once:?}"));
        assert_eq!(
            load(&twice),
            load(&y),
            "{name}: second pass changed the meaning"
        );
    }
}
