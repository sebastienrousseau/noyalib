// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The generated reference documents must match the source.
//!
//! `docs/errors.md` and `docs/internals.md` are inventories — every
//! error variant, every module. An inventory maintained by hand is
//! wrong the first time someone adds an item and forgets the document,
//! and nothing notices, because prose has no compiler.
//!
//! `scripts/generate-reference-docs.sh` writes both from the source.
//! This is what makes that generation load-bearing rather than
//! optional: it reads the source and the documents and fails when they
//! disagree, naming exactly what is missing and what to run.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    let root = crate_root();
    let Some(repo) = root.parent().and_then(Path::parent) else {
        panic!(
            "crates/noyalib should have two ancestors: {}",
            root.display()
        )
    };
    repo.to_path_buf()
}

/// Variant names declared directly inside `pub enum <name>` in error.rs.
fn declared_variants(source: &str, enum_name: &str) -> Vec<String> {
    // `find("pub enum Error")` also matches `pub enum ErrorKind`, which
    // is declared first — so require the next character to end the name.
    // Getting this wrong silently returns the wrong enum's variants.
    let needle = format!("pub enum {enum_name}");
    let start = source
        .match_indices(&needle)
        .find(|(i, _)| {
            source[i + needle.len()..]
                .chars()
                .next()
                .is_some_and(|c| !c.is_ascii_alphanumeric() && c != '_')
        })
        .map(|(i, _)| i);
    let Some(start) = start else {
        panic!("`{needle}` not found in error.rs")
    };
    let body = &source[start..];
    let end = body
        .find("\n}")
        .unwrap_or_else(|| panic!("`{needle}` has no closing brace"));
    body[..end]
        .lines()
        .filter_map(|line| {
            // Exactly four spaces of indent: a variant of this enum,
            // not a field of one of its struct-shaped variants.
            let rest = line.strip_prefix("    ")?;
            if rest.starts_with(' ') {
                return None;
            }
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            // A struct-shaped variant is written `Name {`, with a space
            // before the brace, so skip whitespace before deciding.
            let next = rest[name.len()..].trim_start().chars().next()?;
            (!name.is_empty()
                && name.starts_with(|c: char| c.is_ascii_uppercase())
                && matches!(next, '{' | '(' | ','))
            .then_some(name)
        })
        .collect()
}

#[test]
fn the_error_reference_lists_every_variant_and_kind() {
    let source = fs::read_to_string(crate_root().join("src/error.rs")).expect("error.rs");
    let doc = fs::read_to_string(repo_root().join("docs/errors.md"))
        .expect("docs/errors.md — run scripts/generate-reference-docs.sh");

    let variants = declared_variants(&source, "Error");
    let kinds = declared_variants(&source, "ErrorKind");
    assert!(
        variants.len() > 20,
        "suspiciously few variants parsed: {variants:?}"
    );
    assert!(kinds.len() > 5, "suspiciously few kinds parsed: {kinds:?}");

    let missing: Vec<_> = variants
        .iter()
        .filter(|v| !doc.contains(&format!("`Error::{v}`")))
        .collect();
    assert!(
        missing.is_empty(),
        "{} error variant(s) are not in docs/errors.md: {missing:?}\n\
         Run `bash scripts/generate-reference-docs.sh` and commit the result.",
        missing.len()
    );

    let missing: Vec<_> = kinds
        .iter()
        .filter(|k| !doc.contains(&format!("`ErrorKind::{k}`")))
        .collect();
    assert!(
        missing.is_empty(),
        "{} error kind(s) are not in docs/errors.md: {missing:?}\n\
         Run `bash scripts/generate-reference-docs.sh` and commit the result.",
        missing.len()
    );
}

/// The reverse direction: a variant removed from the source must not
/// linger in the document, or the reference documents something that
/// no longer exists.
#[test]
fn the_error_reference_lists_nothing_that_no_longer_exists() {
    let source = fs::read_to_string(crate_root().join("src/error.rs")).expect("error.rs");
    let doc = fs::read_to_string(repo_root().join("docs/errors.md")).expect("docs/errors.md");
    let variants = declared_variants(&source, "Error");

    let stale: Vec<String> = doc
        .lines()
        .filter_map(|l| {
            l.split("`Error::")
                .nth(1)?
                .split('`')
                .next()
                .map(str::to_owned)
        })
        // `Error::kind()` and friends are prose references to methods,
        // not variant names.
        .filter(|name| name.chars().all(|c| c.is_ascii_alphanumeric()))
        .filter(|name| !name.is_empty() && !variants.contains(name))
        .collect();
    assert!(
        stale.is_empty(),
        "docs/errors.md documents {} variant(s) that no longer exist: {stale:?}\n\
         Run `bash scripts/generate-reference-docs.sh` and commit the result.",
        stale.len()
    );
}

#[test]
fn the_module_map_lists_every_module() {
    let doc = fs::read_to_string(repo_root().join("docs/internals.md"))
        .expect("docs/internals.md — run scripts/generate-reference-docs.sh");

    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("readable src dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path.strip_prefix(root).expect("under src");
                if rel.file_name().is_some_and(|f| f == "lib.rs") {
                    continue;
                }
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    let src = crate_root().join("src");
    let mut modules = Vec::new();
    walk(&src, &src, &mut modules);
    modules.sort();
    assert!(
        modules.len() > 40,
        "suspiciously few modules found: {}",
        modules.len()
    );

    let missing: Vec<_> = modules
        .iter()
        .filter(|m| !doc.contains(&format!("`{m}`")))
        .collect();
    assert!(
        missing.is_empty(),
        "{} module(s) are not in docs/internals.md: {missing:?}\n\
         Run `bash scripts/generate-reference-docs.sh` and commit the result.",
        missing.len()
    );
}

/// Every Cargo feature must appear in the internals feature table.
///
/// The table this replaced was hand-written in a second copy of the
/// document that nothing checked. It listed 16 of 28 features, invented
/// a `robotics` feature that has never existed, and both READMEs linked
/// to it. Generation alone does not prevent that — a generator whose
/// pattern misses a line drops a feature just as silently, and the
/// first version of this one did exactly that to `std`, whose feature
/// list spans several lines. This test is what notices.
#[test]
fn the_feature_table_lists_every_feature() {
    let doc = fs::read_to_string(repo_root().join("docs/internals.md"))
        .expect("docs/internals.md — run scripts/generate-reference-docs.sh");
    let manifest = fs::read_to_string(crate_root().join("Cargo.toml")).expect("Cargo.toml");

    let features = manifest
        .split_once("\n[features]\n")
        .expect("a [features] table")
        .1;
    let features = features.split("\n[").next().expect("end of the table");

    let declared: Vec<&str> = features
        .lines()
        .filter_map(|line| {
            // Only a line that starts in column zero declares a feature;
            // the continuation lines of a multi-line list are indented.
            let (name, _) = line.split_once('=')?;
            let name = name.trim_end();
            (!name.is_empty()
                && !line.starts_with([' ', '\t', '#'])
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .then_some(name)
        })
        .collect();

    assert!(
        declared.len() > 20,
        "suspiciously few features parsed: {declared:?}"
    );

    // Anchored to the start of a line, not `contains`: the Enables
    // column of another feature's row is also of the form `| `std` |`,
    // so a substring search reports a dropped feature as present. The
    // first version of this test did, and passed when `std` was deleted
    // from the table.
    let missing: Vec<_> = declared
        .iter()
        .filter(|f| {
            let row = format!("| `{f}` |");
            !doc.lines().any(|line| line.starts_with(&row))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "{} feature(s) are not in the docs/internals.md feature table: {missing:?}\n\
         Run `bash scripts/generate-reference-docs.sh` and commit the result.",
        missing.len()
    );
}

/// Both documents must say they are generated, so nobody edits them by
/// hand and loses the edit on the next run.
#[test]
fn both_reference_documents_say_they_are_generated() {
    for name in ["docs/errors.md", "docs/internals.md"] {
        let doc =
            fs::read_to_string(repo_root().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            doc.contains("Generated") && doc.contains("generate-reference-docs.sh"),
            "{name} does not name the generator that writes it"
        );
    }
}
