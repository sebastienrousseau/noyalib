// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `$include` key — modular configuration via cross-file references.
//!
//! Real-world configuration trees (Kubernetes Helm charts, Argo CD
//! ApplicationSets, Docker Compose stacks) split a single logical
//! document across many files for human-friendly editing. The
//! convention this example demonstrates is the same one JSON
//! Schema (`$ref`), Argo CD, and a long tail of in-house tools
//! adopt: a single-key mapping `{ $include: path/to/file.yaml }`
//! is a placeholder that gets replaced inline by the parsed
//! contents of the referenced file.
//!
//! The pattern: walk the parsed [`Value`] tree, recognise mappings
//! whose only key is `$include`, replace each with the contents of
//! the named file. Cycle detection prevents `a.yaml` → `b.yaml` →
//! `a.yaml` runaway recursion.
//!
//! Run: `cargo run --example include`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const ROOT_YAML: &str = "\
name: noyalib-stack
services:
  api:
    $include: partials/api.yaml
  worker:
    $include: partials/worker.yaml
metadata:
  owner: platform-team
";

const API_YAML: &str = "\
image: registry.example.com/api:1.0
port: 8080
replicas: 3
";

const WORKER_YAML: &str = "\
image: registry.example.com/worker:1.0
queue: jobs
concurrency: 8
";

fn main() -> noyalib::Result<()> {
    support::header("$include — modular YAML configuration");

    let tmp = std::env::temp_dir().join(format!("noyalib-include-example-{}", std::process::id()));
    let partials = tmp.join("partials");
    fs::create_dir_all(&partials).map_err(io_err)?;
    fs::write(partials.join("api.yaml"), API_YAML).map_err(io_err)?;
    fs::write(partials.join("worker.yaml"), WORKER_YAML).map_err(io_err)?;

    let mut value: Value = from_str(ROOT_YAML)?;
    let mut visiting = HashSet::<PathBuf>::new();
    resolve_includes(&mut value, &tmp, &mut visiting)?;

    println!("  Root document after $include resolution:\n");
    let formatted = noyalib::to_string(&value)?;
    for line in formatted.lines() {
        println!("    {line}");
    }

    let _ = fs::remove_dir_all(&tmp);

    println!();
    println!("  Each `{{ $include: <path> }}` placeholder was replaced by");
    println!("  the parsed contents of the referenced file — single");
    println!("  document, multiple human-friendly source files.");

    support::footer();
    Ok(())
}

/// Walk `value` and replace every single-key `$include` mapping with
/// the parsed contents of the referenced file. `base` is the directory
/// `path` is resolved against; `visiting` tracks the active include
/// chain so cycles are detected.
fn resolve_includes(
    value: &mut Value,
    base: &Path,
    visiting: &mut HashSet<PathBuf>,
) -> noyalib::Result<()> {
    if let Some(rel) = include_target(value) {
        let path = base.join(&rel);
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !visiting.insert(canonical.clone()) {
            return Err(noyalib::Error::Custom(format!(
                "include cycle detected: {}",
                canonical.display()
            )));
        }
        let content = fs::read_to_string(&path).map_err(io_err)?;
        let mut nested: Value = from_str(&content)?;
        let nested_base = path.parent().unwrap_or(base).to_path_buf();
        resolve_includes(&mut nested, &nested_base, visiting)?;
        let _ = visiting.remove(&canonical);
        *value = nested;
        return Ok(());
    }
    match value {
        Value::Sequence(seq) => {
            for item in seq.iter_mut() {
                resolve_includes(item, base, visiting)?;
            }
        }
        Value::Mapping(map) => {
            for (_, v) in map.iter_mut() {
                resolve_includes(v, base, visiting)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// If `value` is a single-entry mapping `{ $include: <string> }`,
/// return the string. Otherwise return `None`.
fn include_target(value: &Value) -> Option<String> {
    let map = value.as_mapping()?;
    if map.len() != 1 {
        return None;
    }
    let (k, v) = map.iter().next()?;
    if k != "$include" {
        return None;
    }
    Some(v.as_str()?.to_owned())
}

fn io_err(e: std::io::Error) -> noyalib::Error {
    noyalib::Error::Custom(format!("I/O error: {e}"))
}
