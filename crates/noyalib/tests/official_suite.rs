// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! YAML Test Suite runner.
//!
//! Validates noyalib against the official YAML test suite.

use noyalib::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Decodes the markers used in the YAML test suite to represent
/// special characters. The full marker alphabet is documented at
/// <https://github.com/yaml/yaml-test-suite/blob/main/CONTRIBUTING.md>.
fn decode_test_suite_markers(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '␣' => out.push(' '),  // U+2423 OPEN BOX → space
            '⇥' => out.push('\t'), // U+21E5 RIGHTWARDS ARROW TO BAR → tab
            '↵' => {
                // U+21B5 DOWNWARDS ARROW WITH CORNER LEFTWARDS → newline.
                // Test suite convention: an `↵` on its own line both
                // marks the line break AND swallows the surrounding
                // newline so the canonical content stays unambiguous.
                out.push('\n');
                if chars.peek() == Some(&'\n') {
                    let _ = chars.next();
                }
            }
            '↓' => out.push('\r'),       // U+2193 DOWNWARDS ARROW → CR
            '⇔' => out.push('\u{feff}'), // U+21D4 LEFT RIGHT DOUBLE ARROW → BOM
            // U+220E END OF PROOF — sentinel that strips the *rest of
            // the current line*, including the line break that follows
            // it. Used by the test suite to anchor whether trailing
            // whitespace/newlines are part of the input. Not stripping
            // this character feeds literal `∎` into the parser and
            // breaks 7 cases (4RWC, JEF9, SM9W, UGM3, AVM7, 2G84, L24T).
            '∎' => {
                while let Some(&next) = chars.peek() {
                    if next == '\n' {
                        let _ = chars.next();
                        break;
                    }
                    let _ = chars.next();
                }
            }
            '—' => {
                // Check for '———»' or '————»' — a chain of em-dashes
                // optionally terminated by `»` represents a tab.
                let mut count = 1;
                while let Some(&next) = chars.peek() {
                    if next == '—' {
                        let _ = chars.next();
                        count += 1;
                    } else {
                        break;
                    }
                }
                if chars.peek() == Some(&'»') {
                    let _ = chars.next();
                    out.push('\t');
                } else {
                    for _ in 0..count {
                        out.push('—');
                    }
                }
            }
            '»' => out.push('\t'), // U+00BB RIGHT-POINTING DOUBLE ANGLE QUOTATION MARK → tab
            _ => out.push(c),
        }
    }
    out
}

/// Convert a `noyalib::Value` to a `serde_json::Value` using the
/// YAML 1.2 *data-model* projection: `Value::Tagged` is unwrapped
/// (the test-suite expected-JSON omits the tag layer), numbers and
/// strings become themselves, and collections recurse.
///
/// This is *not* the same projection as `Value::serialize` —
/// serde-bridge convention surfaces `Tagged` as a single-key map
/// (`{"!Tag": inner}`) for cross-format interop. The test suite's
/// expected JSON predates that convention, so we use a tag-stripping
/// projection here.
fn yaml_value_to_json(v: &Value) -> serde_json::Value {
    use noyalib::Number;
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Number(Number::Integer(n)) => serde_json::json!(*n),
        #[cfg(feature = "lossless-u64")]
        Value::Number(Number::Unsigned(n)) => serde_json::json!(*n),
        Value::Number(Number::Float(f)) => {
            if f.is_finite() && f.fract() == 0.0 && f.abs() < (i64::MAX as f64) {
                serde_json::json!(*f as i64)
            } else {
                serde_json::json!(*f)
            }
        }
        // `Number` is `#[non_exhaustive]`; future variants land here.
        Value::Number(_) => serde_json::Value::Null,
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_value_to_json).collect())
        }
        Value::Mapping(m) => {
            let obj: serde_json::Map<String, serde_json::Value> = m
                .iter()
                .map(|(k, v)| (k.clone(), yaml_value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Value::Tagged(t) => yaml_value_to_json(t.value()),
    }
}

/// Compare two JSON values for *semantic* equality, treating
/// numerically-equal Float/Integer pairs as equal. The YAML 1.2 core
/// schema resolves a scalar like `450.00` as a float, but the
/// reference test cases (which were authored against libyaml's
/// behaviour) sometimes express the same number as an integer in the
/// expected `json` block. `serde_json::Value`'s `PartialEq` is exact
/// (`Number(450.0) != Number(450)`), so a structural walk that
/// normalises numbers is required.
fn json_value_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value as V;
    match (a, b) {
        (V::Number(an), V::Number(bn)) => {
            // Compare as f64 — covers int↔float equivalence
            // (450.00 vs 450) without losing precision for the
            // values the YAML core schema can actually emit.
            an.as_f64() == bn.as_f64() && an.as_f64().is_some() || an == bn
        }
        (V::Array(av), V::Array(bv)) => {
            av.len() == bv.len()
                && av
                    .iter()
                    .zip(bv.iter())
                    .all(|(x, y)| json_value_equal(x, y))
        }
        (V::Object(am), V::Object(bm)) => {
            am.len() == bm.len()
                && am
                    .iter()
                    .all(|(k, v)| bm.get(k).is_some_and(|w| json_value_equal(v, w)))
        }
        _ => a == b,
    }
}

fn json_values_equal(a: &[serde_json::Value], b: &[serde_json::Value]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| json_value_equal(x, y))
}

/// Test cases the YAML 1.2 spec exercises that this version does not yet
/// reproduce bit-for-bit. Each entry is a `(file_id, reason)` pair so a
/// future contributor can locate the exact spec corner without re-running
/// the suite. The pass-rate target is 100% on the *active* subset; this
/// list is the explicit, audited delta. Removing an entry implies the
/// underlying behaviour has been fixed.
const SKIP_LIST: &[(&str, &str)] = &[
    // ── Stricter rejection still missing (validation work) ─────────────
    // ── Block parser corners (explicit-key + nested block) ─────────────
];

#[derive(Debug)]
#[allow(dead_code)]
struct TestCase {
    id: String,
    name: String,
    yaml: String,
    should_fail: bool,
    json: Option<String>,
    tags: String,
}

fn load_test_suite(dir: &Path) -> Vec<TestCase> {
    let mut cases = Vec::new();

    for entry in fs::read_dir(dir).expect("test suite directory not found") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }

        let content = fs::read_to_string(&path).expect("read test case");
        let docs: Vec<HashMap<String, serde_yaml_ng::Value>> =
            serde_yaml_ng::from_str(&content).expect("parse test case wrapper");

        for doc in docs {
            let id = path.file_stem().unwrap().to_str().unwrap().to_string();
            let name = doc
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed")
                .to_string();
            let yaml = match doc.get("yaml").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue, // Skip cases without YAML
            };
            let fail = doc.get("fail").and_then(|v| v.as_bool()).unwrap_or(false);
            let json = doc
                .get("json")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let tags = doc
                .get("tags")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            cases.push(TestCase {
                id,
                name,
                yaml,
                should_fail: fail,
                json,
                tags,
            });
        }
    }

    cases.sort_by_key(|c| c.id.clone());
    cases
}

#[test]
fn official_suite() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suite_dir = manifest_dir.join("tests").join("yaml-test-suite");
    let cases = load_test_suite(&suite_dir);

    let mut pass = 0;
    let mut fail = 0;
    let mut skip = 0;

    for case in cases {
        if SKIP_LIST.iter().any(|(id, _)| *id == case.id) {
            skip += 1;
            continue;
        }

        let yaml = decode_test_suite_markers(&case.yaml);
        let res = noyalib::load_all_as::<Value>(&yaml);

        match res {
            Ok(vals) => {
                if case.should_fail {
                    eprintln!("FAIL {}: expected error, got success", case.id);
                    fail += 1;
                } else if let Some(ref expected_json) = case.json {
                    // Project each parsed `Value` through the
                    // YAML-data-model JSON conversion (strips
                    // tag wrappers) so the comparison matches
                    // the suite's tag-less expected JSON shape.
                    let actual_vals: Vec<serde_json::Value> =
                        vals.iter().map(yaml_value_to_json).collect();

                    let expected_vals: Vec<serde_json::Value> =
                        serde_json::Deserializer::from_str(expected_json)
                            .into_iter::<serde_json::Value>()
                            .map(|v| v.unwrap_or(serde_json::Value::Null))
                            .collect();

                    if json_values_equal(&expected_vals, &actual_vals) {
                        pass += 1;
                    } else {
                        eprintln!("FAIL {}: value mismatch", case.id);
                        eprintln!("  Expected: {expected_json}");
                        eprintln!(
                            "  Actual:   {}",
                            serde_json::to_string(&actual_vals).unwrap()
                        );
                        fail += 1;
                    }
                } else {
                    pass += 1;
                }
            }
            Err(e) => {
                if case.should_fail {
                    pass += 1;
                } else {
                    eprintln!("FAIL {}: {}", case.id, e);
                    fail += 1;
                }
            }
        }
    }

    let total = pass + fail + skip;
    let compliance = if total > skip {
        (f64::from(pass) / f64::from(total - skip)) * 100.0
    } else {
        100.0
    };

    eprintln!();
    eprintln!("═══ YAML Test Suite Compliance ═══");
    eprintln!("  Total:      {total}");
    eprintln!("  Pass:       {pass}");
    eprintln!("  Fail:       {fail}");
    eprintln!("  Skip:       {skip}");
    eprintln!("  Compliance: {compliance:.1}%");
    eprintln!();

    // The README, CITATION.cff, and the crate description all claim
    // 406/406; the gate is the claim. A single failure is a regression.
    assert!(
        fail == 0,
        "official YAML test suite: {fail} failing case(s), compliance {compliance:.1}%"
    );
}

// ── Streams built from the suite (#407) ─────────────────────────────
//
// `cst::parse_stream` parses each document from its own slice and
// re-anchors error locations on the whole input. The unit tests in
// `cst_stream.rs` pin the reported cases; the property below runs the
// same check over every valid suite case: each one becomes a stream
// with a known-bad document injected first, in the middle and last,
// and the location the CST reports must be the byte we injected, with
// line and column recomputed on the stream, and identical to what the
// typed loaders report for the same bytes.

/// Documents with exactly one error each, and the byte at which that
/// error is reported when the document is parsed on its own.
const PROBES: &[(&str, &str, usize)] = &[
    ("block mapping alias", "---\nc: *nope\n", 7),
    ("root alias on the marker line", "--- *nope\n", 4),
    ("flow sequence on the marker line", "--- [*nope]\n", 5),
    ("sequence alias", "---\n- *nope\n", 6),
    ("flow mapping alias", "---\n{k: *nope}\n", 8),
    ("alias then explicit end marker", "---\nc: *nope\n...\n", 7),
    ("nested alias", "---\na:\n  b:\n    - *nope\n", 18),
    ("key collision", "---\n1: x\n\"1\": y\n", 9),
];

/// The case's YAML terminated so a following document starts on a
/// fresh line.
fn document_body(yaml: &str) -> String {
    if yaml.ends_with('\n') {
        yaml.to_string()
    } else {
        format!("{yaml}\n")
    }
}

/// The explicit end marker that separates `probe` from whatever
/// follows it. A case need not open with `---`: without the marker its
/// first lines would continue the probe's document (YAML 1.2.2 §9.1.4),
/// and a directive-led case would make the stream invalid for a second
/// reason.
fn close_after(probe: &str) -> &'static str {
    if probe.ends_with("...\n") {
        ""
    } else {
        "...\n"
    }
}

/// Check one stream: the CST entry points and the typed loaders must
/// all reject it at byte `expected` of the stream.
fn check_stream(label: &str, stream: &str, expected: usize, failures: &mut Vec<String>) {
    use noyalib::cst::{parse_stream, parse_stream_with_config};
    use noyalib::{Location, ParserConfig};

    let want = Location::from_index(stream, expected);
    let mut located = |name: &str, res: Result<(), noyalib::Error>| -> Option<Location> {
        match res {
            Ok(()) => {
                failures.push(format!("{label}: {name} accepted the stream"));
                None
            }
            Err(err) => match err.location() {
                Some(loc) => Some(loc),
                None => {
                    failures.push(format!("{label}: {name} error has no location: {err}"));
                    None
                }
            },
        }
    };

    let cst = located("cst::parse_stream", parse_stream(stream).map(drop));
    let cst_cfg = located(
        "cst::parse_stream_with_config",
        parse_stream_with_config(stream, &ParserConfig::new()).map(drop),
    );
    let typed = located(
        "load_all_as::<Value>",
        noyalib::load_all_as::<Value>(stream).map(drop),
    );
    let untyped = located("load_all", noyalib::load_all(stream).map(drop));

    for (name, loc) in [
        ("cst::parse_stream", cst),
        ("cst::parse_stream_with_config", cst_cfg),
        ("load_all_as::<Value>", typed),
        ("load_all", untyped),
    ] {
        let Some(loc) = loc else { continue };
        if loc != want {
            failures.push(format!(
                "{label}: {name} reported index {} line {} column {}, expected index {} line {} column {}; stream {stream:?}",
                loc.index(),
                loc.line(),
                loc.column(),
                want.index(),
                want.line(),
                want.column()
            ));
        }
    }

    // The rendered error points at the stream line, not the document
    // line.
    if let Err(err) = parse_stream(stream) {
        let rendered = err.render(stream);
        let line_no = format!("{} |", want.line());
        if !rendered.contains(&line_no) {
            failures.push(format!(
                "{label}: rendered error does not show stream line {}:\n{rendered}",
                want.line()
            ));
        }
    }
}

#[test]
fn official_suite_streams_locate_errors_in_the_stream() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suite_dir = manifest_dir.join("tests").join("yaml-test-suite");
    let cases = load_test_suite(&suite_dir);

    // Every valid case, decoded, as a document body.
    let bodies: Vec<(String, String)> = cases
        .iter()
        .filter(|c| !c.should_fail && !SKIP_LIST.iter().any(|(id, _)| *id == c.id))
        .map(|c| (c.id.clone(), decode_test_suite_markers(&c.yaml)))
        .filter(|(_, yaml)| noyalib::load_all_as::<Value>(yaml).is_ok())
        .map(|(id, yaml)| (id, document_body(&yaml)))
        .collect();
    assert!(
        bodies.len() > 200,
        "only {} valid suite cases",
        bodies.len()
    );

    let mut failures = Vec::new();
    let mut streams = 0u32;

    for (i, (id, body)) in bodies.iter().enumerate() {
        let (next_id, next_body) = &bodies[(i + 1) % bodies.len()];
        for (probe_name, probe, local) in PROBES {
            // Probe last: the common shape, the case's own documents
            // first.
            let last = format!("{body}{probe}");
            check_stream(
                &format!("{id} then {probe_name}"),
                &last,
                body.len() + local,
                &mut failures,
            );
            // Probe first: base zero must be the identity.
            let first = format!("{probe}{}{body}", close_after(probe));
            check_stream(
                &format!("{probe_name} then {id}"),
                &first,
                *local,
                &mut failures,
            );
            // Probe in the middle of two cases.
            let middle = format!("{body}{probe}{}{next_body}", close_after(probe));
            check_stream(
                &format!("{id} then {probe_name} then {next_id}"),
                &middle,
                body.len() + local,
                &mut failures,
            );
            streams += 3;
        }
    }

    eprintln!();
    eprintln!("═══ YAML Test Suite Streams ═══");
    eprintln!("  Bodies:     {}", bodies.len());
    eprintln!("  Streams:    {streams}");
    eprintln!("  Failures:   {}", failures.len());
    for f in failures.iter().take(40) {
        eprintln!("  - {f}");
    }
    assert!(
        failures.is_empty(),
        "{} stream(s) reported a wrong error location",
        failures.len()
    );
}
