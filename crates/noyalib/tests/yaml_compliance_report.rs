// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! YAML Test Suite — honest compliance gap report.
//!
//! Walks every case in `tests/yaml-test-suite/` and classifies each
//! against the YAML 1.2 expectations encoded in the suite. Unlike the
//! sibling `tests/official_suite.rs`, this runner does *not* score a
//! case as pass when the parser is more lenient than the spec, when
//! the parsed value diverges from the expected JSON, or when the
//! case is known to require non-scalar key support — those are
//! surfaced as distinct failure modes so the gap can be prioritised.
//!
//! Output:
//!   - Markdown report written to
//!     `${CARGO_MANIFEST_DIR}/target/yaml-compliance-report.md`
//!   - Summary printed to stderr (visible with `--nocapture`)
//!
//! No assertions — this test is a *report*, not a regression net.
//! The regression net lives in `tests/official_suite.rs`.

#![allow(missing_docs)]

use noyalib::{Value, from_str};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Cases excluded from scoring — must stay in sync with
/// `tests/official_suite.rs::SKIP_LIST` so both runners reason about
/// the same baseline.
const SKIP_LIST: &[&str] = &[];

/// Decode the YAML Test Suite's visual whitespace markers.
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
            // ↵ explicitly marks a line terminator. The wrapper preserves
            // both the marker and the source `\n` that follows it; consume
            // that source `\n` so we don't double-count the line break.
            out.push('\n');
            if chars.peek() == Some(&'\n') {
                let _ = chars.next();
            }
        } else if ch == '\u{2423}' {
            out.push(' ');
        } else if ch == '\u{220E}' {
            // end-of-input marker — strip
        } else {
            out.push(ch);
        }
    }
    out
}

#[derive(Debug)]
struct TestCase {
    id: String,
    name: String,
    yaml: String,
    should_fail: bool,
    json: Option<String>,
    tags: Vec<String>,
}

fn load_suite(dir: &Path) -> Vec<TestCase> {
    let mut cases = Vec::new();
    let entries = fs::read_dir(dir).expect("test suite directory not found");
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let id = path.file_stem().unwrap().to_str().unwrap().to_string();
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let docs: Value = match from_str(&content) {
            Ok(v) => v,
            // meta-circular: cases the parser cannot handle when wrapped
            // in YAML themselves are not directly classifiable here.
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
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let yaml = match item.get("yaml").and_then(Value::as_str) {
                Some(y) => decode_markers(y),
                None => continue,
            };
            let should_fail = item.get("fail").and_then(Value::as_bool).unwrap_or(false);
            let json = item.get("json").and_then(Value::as_str).map(str::to_string);
            let tags: Vec<String> = item
                .get("tags")
                .and_then(Value::as_str)
                .map(|s| s.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default();
            cases.push(TestCase {
                id: case_id,
                name,
                yaml,
                should_fail,
                json,
                tags,
            });
        }
    }
    cases.sort_by(|a, b| a.id.cmp(&b.id));
    cases
}

fn yaml_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => {
            let f = n.as_f64();
            if f.is_finite() && f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
                serde_json::json!(f as i64)
            } else {
                serde_json::json!(f)
            }
        }
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Sequence(seq) => serde_json::Value::Array(seq.iter().map(yaml_to_json).collect()),
        Value::Mapping(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), yaml_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Value::Tagged(t) => yaml_to_json(t.value()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Outcome {
    Pass,
    Skip,
    FailParseError,
    FailValueMismatch,
    FailLenient,
    FailNonScalarKey,
}

impl Outcome {
    fn label(self) -> &'static str {
        match self {
            Outcome::Pass => "pass",
            Outcome::Skip => "skip",
            Outcome::FailParseError => "fail-parse-error",
            Outcome::FailValueMismatch => "fail-value-mismatch",
            Outcome::FailLenient => "fail-lenient",
            Outcome::FailNonScalarKey => "fail-non-scalar-key",
        }
    }

    fn is_fail(self) -> bool {
        matches!(
            self,
            Outcome::FailParseError
                | Outcome::FailValueMismatch
                | Outcome::FailLenient
                | Outcome::FailNonScalarKey
        )
    }
}

const ALL_OUTCOMES: [Outcome; 6] = [
    Outcome::Pass,
    Outcome::Skip,
    Outcome::FailParseError,
    Outcome::FailValueMismatch,
    Outcome::FailLenient,
    Outcome::FailNonScalarKey,
];

fn classify(case: &TestCase, skip: &[&str]) -> (Outcome, Option<String>) {
    let base = case.id.split(':').next().unwrap_or(&case.id);
    if skip.contains(&base) {
        return (Outcome::Skip, None);
    }
    let parsed: Result<Value, _> = from_str(&case.yaml);
    if case.should_fail {
        return match parsed {
            Err(_) => (Outcome::Pass, None),
            Ok(_) => (
                Outcome::FailLenient,
                Some("parser accepted YAML the spec rejects".into()),
            ),
        };
    }
    match parsed {
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("non-scalar key") || msg.contains("expected string, found non-scalar") {
                (Outcome::FailNonScalarKey, Some(msg))
            } else {
                (Outcome::FailParseError, Some(msg))
            }
        }
        Ok(value) => match case.json.as_deref() {
            None => (Outcome::Pass, None),
            Some(json_str) => match serde_json::from_str::<serde_json::Value>(json_str) {
                Err(_) => (Outcome::Pass, None),
                Ok(expected) => {
                    let actual = yaml_to_json(&value);
                    if actual == expected {
                        (Outcome::Pass, None)
                    } else {
                        let detail = format!("expected {expected}, got {actual}");
                        (Outcome::FailValueMismatch, Some(detail))
                    }
                }
            },
        },
    }
}

#[test]
fn yaml_compliance_report() {
    let suite_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/yaml-test-suite");
    if !suite_dir.exists() {
        eprintln!("SKIP: yaml-test-suite not found at {}", suite_dir.display());
        return;
    }

    let cases = load_suite(&suite_dir);
    let total = cases.len();

    // Hard floor: the bundled `tests/yaml-test-suite/` ships ~400 YAML
    // test cases. A `total == 0` here is almost certainly the symptom
    // of a parser regression that broke `from_str(&content)` on the
    // wrapper itself — `load_suite` silently `continue`s when a wrapper
    // fails to parse, so without this assertion the test would
    // vacuously "pass at 100% (0/0)" and let the regression land.
    // (This was the failure mode that shipped commit 1e7dace.)
    assert!(
        total >= 350,
        "yaml-test-suite loaded {total} cases — expected ≥ 350. \
         The wrapper-parse `from_str(&content)` may have regressed; \
         compliance reports become meaningless when the wrapper itself \
         fails to load.",
    );

    let mut counts: BTreeMap<Outcome, u32> = BTreeMap::new();
    let mut by_tag: BTreeMap<String, BTreeMap<Outcome, u32>> = BTreeMap::new();
    let mut details: Vec<(Outcome, String, String, String)> = Vec::new();

    for case in &cases {
        let (outcome, detail) = classify(case, SKIP_LIST);
        *counts.entry(outcome).or_default() += 1;
        for tag in &case.tags {
            *by_tag
                .entry(tag.clone())
                .or_default()
                .entry(outcome)
                .or_default() += 1;
        }
        if outcome.is_fail() {
            details.push((
                outcome,
                case.id.clone(),
                case.name.clone(),
                detail.unwrap_or_default(),
            ));
        }
    }

    let pass = counts.get(&Outcome::Pass).copied().unwrap_or(0);
    let skip = counts.get(&Outcome::Skip).copied().unwrap_or(0);
    let scored = total.saturating_sub(skip as usize);
    let strict_pct = if scored > 0 {
        f64::from(pass) * 100.0 / scored as f64
    } else {
        0.0
    };

    // ── Markdown report ──────────────────────────────────────────────
    let mut md = String::new();
    md.push_str("# YAML Test Suite — Compliance Gap Report\n\n");
    md.push_str("Generated by `tests/yaml_compliance_report.rs`. ");
    md.push_str("Distinct failure modes are surfaced so the gap can be prioritised — ");
    md.push_str("see the sibling `tests/official_suite.rs` for the regression-net assertion.\n\n");
    md.push_str(&format!("**Total cases**: {total}\n\n"));

    md.push_str("## Headline\n\n| Outcome | Count |\n| --- | ---: |\n");
    for outcome in ALL_OUTCOMES {
        let n = counts.get(&outcome).copied().unwrap_or(0);
        md.push_str(&format!("| {} | {} |\n", outcome.label(), n));
    }
    md.push_str(&format!(
        "\n**Strict compliance** (pass / (total − skip)): **{strict_pct:.1}%**  ({pass} / {scored})\n\n"
    ));

    md.push_str("## By tag\n\n| Tag | Pass | Fail | Skip | Strict % |\n| --- | ---: | ---: | ---: | ---: |\n");
    for (tag, tag_counts) in &by_tag {
        let tag_pass = tag_counts.get(&Outcome::Pass).copied().unwrap_or(0);
        let tag_skip = tag_counts.get(&Outcome::Skip).copied().unwrap_or(0);
        let tag_fail: u32 = tag_counts
            .iter()
            .filter(|(o, _)| o.is_fail())
            .map(|(_, n)| *n)
            .sum();
        let tag_scored = tag_pass + tag_fail;
        let tag_pct = if tag_scored > 0 {
            f64::from(tag_pass) * 100.0 / f64::from(tag_scored)
        } else {
            0.0
        };
        md.push_str(&format!(
            "| {tag} | {tag_pass} | {tag_fail} | {tag_skip} | {tag_pct:.0}% |\n"
        ));
    }

    md.push_str("\n## Failures\n");
    details.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut current_outcome: Option<Outcome> = None;
    for (outcome, id, name, detail) in &details {
        if Some(*outcome) != current_outcome {
            md.push_str(&format!("\n### {}\n\n", outcome.label()));
            current_outcome = Some(*outcome);
        }
        let detail_short: String = detail.chars().take(200).collect();
        md.push_str(&format!("- **{id}** — {name}\n  - {detail_short}\n"));
    }

    let report_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("yaml-compliance-report.md");
    if let Some(parent) = report_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&report_path, &md).expect("write yaml compliance report");

    // ── Stderr summary ───────────────────────────────────────────────
    eprintln!();
    eprintln!("═══ YAML Test Suite — Compliance Gap Report ═══");
    eprintln!("  Total: {total}");
    for outcome in ALL_OUTCOMES {
        let n = counts.get(&outcome).copied().unwrap_or(0);
        eprintln!("  {:<24} {n}", outcome.label());
    }
    eprintln!("  Strict compliance: {strict_pct:.1}% ({pass}/{scored})");
    eprintln!();
    eprintln!("  Report: {}", report_path.display());
    eprintln!();
}
