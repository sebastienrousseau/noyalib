// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A two-document configuration that uses most of YAML at once:
//! anchors and merge keys at two depths, explicit `!!int`, `!!str`,
//! `!!bool` and `!!pairs` tags, literal and folded block scalars,
//! flow and block sequences, a sequence as a mapping key, and comments
//! everywhere. `fixtures/ultra-complex/valid.yaml` must parse through
//! the typed loaders and the CST and project onto exactly the JSON in
//! `valid.json`; the two invalid variants beside it must be refused
//! with the diagnostic that names the mistake.

#![allow(missing_docs)]

use noyalib::cst::{Document, parse_stream};
use noyalib::{Value, load_all_as};
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ultra-complex")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn valid_fixture_projects_onto_the_expected_json() {
    let src = fixture("valid.yaml");
    let expected: Vec<serde_json::Value> =
        serde_json::from_str(&fixture("valid.json")).expect("valid.json parses");
    let docs = load_all_as::<Value>(&src).expect("the fixture parses");
    assert_eq!(docs.len(), 2);
    let actual: Vec<serde_json::Value> = docs
        .into_iter()
        .map(|d| serde_json::to_value(d.untag()).expect("JSON model"))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn valid_fixture_semantics_spot_checks() {
    let src = fixture("valid.yaml");
    let docs = load_all_as::<Value>(&src).unwrap();
    let dev = &docs[0]["environments"]["development"];
    // Merge key with an override, and a nested merge with an override.
    assert_eq!(dev["pool"].as_i64(), Some(2));
    assert_eq!(dev["adapter"].as_str(), Some("postgresql"));
    assert_eq!(dev["credentials"]["username"].as_str(), Some("dev_user"));
    assert_eq!(
        dev["credentials"]["password"].as_str(),
        Some("SecureP@ssw0rd!")
    );
    // Explicit tags resolve to their types.
    assert_eq!(
        docs[0]["defaults"]["db_base"]["timeout"]
            .untag_ref()
            .as_i64(),
        Some(5000)
    );
    assert_eq!(
        docs[0]["environments"]["production"]["ssl"]["enabled"]
            .untag_ref()
            .as_bool(),
        Some(true)
    );
    let services = docs[1]["services"].as_sequence().expect("services");
    // Block scalars: literal keeps line breaks, folded joins them.
    assert!(
        services[0]["startup_script"]
            .as_str()
            .unwrap()
            .starts_with("#!/bin/bash\necho")
    );
    assert!(
        services[0]["description"]
            .as_str()
            .unwrap()
            .starts_with("This pipeline ingests")
    );
    assert!(
        !services[0]["description"]
            .as_str()
            .unwrap()
            .contains("telemetries,\n")
    );
    // A sequence as a mapping key is addressable by its flow rendering.
    assert_eq!(
        services[0]["[region, zone]"]["policy"].as_str(),
        Some("geo-replicated")
    );
    // `!!pairs` keeps its duplicate keys in order.
    let routes = services[1]["matrix_routes"]
        .untag_ref()
        .as_sequence()
        .expect("pairs");
    assert_eq!(routes.len(), 3);
    assert_eq!(routes[2]["sms"].as_str(), Some("backup.gateway.internal"));
}

#[test]
fn valid_fixture_round_trips_through_the_cst() {
    let src = fixture("valid.yaml");
    let docs = parse_stream(&src).expect("the CST parses the fixture");
    assert_eq!(docs.len(), 2);
    assert_eq!(docs.iter().map(Document::source).collect::<String>(), src);
    // Each document's typed view agrees with the typed loader.
    let typed = load_all_as::<Value>(&src).unwrap();
    for (doc, value) in docs.iter().zip(&typed) {
        assert_eq!(&*doc.as_value(), value);
    }
}

#[test]
fn triple_bang_tags_are_refused_with_a_hint() {
    let src = fixture("as-submitted.yaml");
    let err = load_all_as::<Value>(&src).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("tag suffix must not contain `!`"), "{msg}");
    assert!(msg.contains("did you mean `!!int`?"), "{msg}");
    assert_eq!(err.location().map(|l| l.line()), Some(12), "{msg}");
    let cst = parse_stream(&src).unwrap_err();
    assert_eq!(cst.location(), err.location());
}

#[test]
fn aliases_into_an_earlier_document_are_refused_with_the_definition() {
    let src = fixture("cross-document-alias.yaml");
    let err = load_all_as::<Value>(&src).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.starts_with("unknown anchor: database_template at line 83, column 24"),
        "{msg}"
    );
    assert!(
        msg.contains("defined at line 10, column 12, in an earlier document"),
        "{msg}"
    );
    assert!(msg.contains("anchors do not cross `---`"), "{msg}");
    let cst = parse_stream(&src).unwrap_err();
    assert_eq!(cst.to_string(), msg);
}

#[test]
fn formatter_output_reparses_and_projects_onto_the_same_json() {
    use noyalib::cst::{FormatConfig, format_with_config};
    let src = fixture("valid.yaml");
    let formatted = format_with_config(&src, &FormatConfig::default()).expect("formats");
    // The explicit key keeps the space after its indicator.
    assert!(
        formatted.contains("? [ region, zone ]") || formatted.contains("? [region, zone]"),
        "{formatted}"
    );
    let expected: Vec<serde_json::Value> = serde_json::from_str(&fixture("valid.json")).unwrap();
    let docs = load_all_as::<Value>(&formatted)
        .unwrap_or_else(|e| panic!("formatted output does not parse: {e}\n{formatted}"));
    let actual: Vec<serde_json::Value> = docs
        .into_iter()
        .map(|d| serde_json::to_value(d.untag()).unwrap())
        .collect();
    assert_eq!(actual, expected);
}

/// The three formatter defects the fixture exposed, on one-line inputs:
/// an explicit key's value indicator moved onto the key line (which
/// turned `? a` / `: b` into the mapping-keyed `? a: b`), the space
/// after `?` was dropped, and a tag or anchor alone on the line after a
/// colon lost its indentation. Each output must re-parse to the same
/// value as its input.
#[test]
fn formatter_keeps_explicit_keys_and_lone_properties_parseable() {
    use noyalib::cst::{FormatConfig, format_with_config};
    for (src, expected_out) in [
        ("? [a, b]\n: v\n", "? [a, b]\n: v\n"),
        ("? a\n: b\n", "? a\n: b\n"),
        ("k:\n  !!pairs\n  - a: 1\n", "k: !!pairs\n  - a: 1\n"),
        ("k:\n  !!set\n  ? a\n  ? b\n", "k: !!set\n  ? a\n  ? b\n"),
        ("k: !!pairs\n  - a: 1\n", "k: !!pairs\n  - a: 1\n"),
        ("k:\n  &x\n  - a\n", "k: &x\n  - a\n"),
    ] {
        let out = format_with_config(src, &FormatConfig::default()).unwrap();
        assert_eq!(out, expected_out, "input {src:?}");
        let before: Value = noyalib::from_str(src).unwrap();
        let after: Value =
            noyalib::from_str(&out).unwrap_or_else(|e| panic!("{src:?} -> {out:?}: {e}"));
        assert_eq!(before, after, "input {src:?}");
    }
}
