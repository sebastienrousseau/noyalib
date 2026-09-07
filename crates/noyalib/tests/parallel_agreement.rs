// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `parallel::values` must see the same stream `load_all_as` sees.
//!
//! The parallel path finds document boundaries with its own scanner so
//! it can hand each document to a separate thread. That scanner and the
//! real parser must agree on how many documents a stream has, or a
//! caller who switches to the parallel path silently gets a different
//! shape. Before v0.0.39 any stream that opened with a comment came
//! back with one document too many, because the prologue was counted as
//! a document of its own.

#![cfg(feature = "parallel")]
#![allow(missing_docs)]

use noyalib::{Value, load_all_as, parallel};
use std::fs;
use std::path::PathBuf;

/// The split is a partition: concatenating the chunks gives the input.
fn assert_lossless(src: &str, label: &str) {
    let joined: String = parallel::split(src).concat();
    assert_eq!(joined, src, "{label}: split lost or duplicated bytes");
}

#[test]
fn agrees_with_load_all_on_stream_shapes() {
    for (label, src) in [
        (
            "comment before the first marker",
            "# note\n---\na: 1\n---\nb: 2\n",
        ),
        ("comment before a single document", "# note\n---\na: 1\n"),
        ("directive before the marker", "%YAML 1.2\n---\na: 1\n"),
        (
            "blank lines before the marker",
            "\n\n---\na: 1\n---\nb: 2\n",
        ),
        ("content closed by a marker", "a: 1\n---\nb: 2\n"),
        ("comment, content, marker", "# hi\na: 1\n---\nb: 2\n"),
        ("no prologue", "---\na: 1\n---\nb: 2\n"),
        ("explicit end marker", "---\na: 1\n...\n"),
        ("no markers at all", "a: 1\n"),
        ("comment and no marker", "# hi\na: 1\n"),
    ] {
        let par = parallel::values(src).unwrap_or_else(|e| panic!("{label}: {e}"));
        let seq = load_all_as::<Value>(src).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(par.len(), seq.len(), "{label}: document count");
        assert_eq!(par, seq, "{label}: documents");
        assert_lossless(src, label);
    }
}

#[test]
fn agrees_with_load_all_across_the_official_suite() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/yaml-test-suite");
    let mut checked = 0u32;
    let mut disagreed = Vec::new();
    for entry in fs::read_dir(&dir).expect("suite directory") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let src = fs::read_to_string(&path).expect("read case");
        let id = path.file_stem().unwrap().to_string_lossy().to_string();
        // The wrapper files are themselves YAML, which is what makes
        // them a good corpus: real documents with comments, markers and
        // block scalars.
        let Ok(seq) = load_all_as::<Value>(&src) else {
            continue;
        };
        assert_lossless(&src, &id);
        match parallel::values(&src) {
            Ok(par) if par.len() == seq.len() => checked += 1,
            Ok(par) => disagreed.push(format!(
                "{id}: parallel {} vs load_all {}",
                par.len(),
                seq.len()
            )),
            Err(e) => disagreed.push(format!(
                "{id}: parallel refused what load_all accepted: {e}"
            )),
        }
    }
    assert!(checked > 300, "only {checked} cases checked");
    assert!(
        disagreed.is_empty(),
        "{} disagreement(s): {:?}",
        disagreed.len(),
        &disagreed[..disagreed.len().min(10)]
    );
    eprintln!("parallel and load_all agree on {checked} suite files");
}

#[test]
fn agrees_on_the_ultra_complex_fixture() {
    let src = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ultra-complex/valid.yaml"),
    )
    .expect("fixture");
    let par = parallel::values(&src).expect("parallel parses the fixture");
    let seq = load_all_as::<Value>(&src).expect("load_all parses the fixture");
    assert_eq!(
        par.len(),
        2,
        "the fixture has two documents, not {}",
        par.len()
    );
    assert_eq!(par, seq);
    assert_lossless(&src, "ultra-complex");
}

#[test]
fn every_torture_fixture_agrees_between_the_two_paths() {
    // The obvious way to run a directory of documents is to concatenate
    // them and let the parallel path split the stream. That only works
    // if the split matches the parser's own document boundaries, which
    // is what this checks, fixture by fixture and then all at once.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/spec-torture");
    let mut names: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("fixtures")
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.extension()?.to_str()? == "yaml").then_some(p)
        })
        .collect();
    names.sort();
    assert!(names.len() >= 15, "only {} fixtures", names.len());

    let mut stream = String::new();
    for path in &names {
        let src = fs::read_to_string(path).expect("read");
        let label = path.file_name().unwrap().to_string_lossy().to_string();
        assert_lossless(&src, &label);
        // Only the fixtures that parse at all can be compared; the
        // deliberately invalid ones are covered in `spec_torture.rs`.
        if let Ok(seq) = load_all_as::<Value>(&src) {
            let par = parallel::values(&src).unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_eq!(par.len(), seq.len(), "{label}: document count");
            assert_eq!(par, seq, "{label}: documents");
            if !src.starts_with('%') && !src.contains("\n%") {
                // Directives belong to their own stream, so a fixture
                // carrying one is not concatenable with the others.
                stream.push_str(&src);
                if !stream.ends_with('\n') {
                    stream.push('\n');
                }
            }
        }
    }
    // The concatenation itself: both paths must see the same stream.
    let seq = load_all_as::<Value>(&stream).expect("the concatenated stream parses");
    let par = parallel::values(&stream).expect("the parallel path parses it too");
    assert_eq!(par.len(), seq.len(), "concatenated document count");
    assert_eq!(par, seq);
    assert_lossless(&stream, "concatenated");
    eprintln!(
        "concatenated {} fixtures into {} documents",
        names.len(),
        seq.len()
    );
}
