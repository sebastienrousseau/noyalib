// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Intelligent error suggestions: "Did you mean...?" for typos.
//!
//! Implements Levenshtein distance to suggest the closest matching field
//! when a key is not found. Zero external dependencies.
//!
//! Run: `cargo run --example suggest`

#[path = "support.rs"]
mod support;

use noyalib::{Value, from_str};

/// Levenshtein edit distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
    let mut curr = vec![0; b_chars.len() + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b_chars.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_chars.len()]
}

/// Find the closest matching key in a Value mapping.
fn suggest_key<'a>(value: &'a Value, typo: &str) -> Option<(&'a str, usize)> {
    let map = value.as_mapping()?;
    map.keys()
        .map(|k| (k.as_str(), levenshtein(k, typo)))
        .filter(|&(_, dist)| dist <= 3) // max edit distance
        .min_by_key(|&(_, dist)| dist)
}

/// Validate that all required keys exist, suggest fixes for typos.
fn validate_config(value: &Value, required: &[&str]) -> Vec<String> {
    let mut issues = Vec::new();
    for &key in required {
        if value.get(key).is_some() {
            continue;
        }
        match suggest_key(value, key) {
            Some((suggestion, dist)) => {
                issues.push(format!(
                    "field '{}' not found. Did you mean '{}'? (edit distance: {})",
                    key, suggestion, dist
                ));
            }
            None => {
                issues.push(format!("field '{}' not found. No similar keys.", key));
            }
        }
    }
    issues
}

fn main() {
    support::header("noyalib -- suggest");

    // ── Typo detection ───────────────────────────────────────────────
    support::task_with_output("Detect typos with suggestions", || {
        let yaml = "retry_limit: 3\ntimeout_ms: 5000\nmax_connections: 10\nenable_logging: true\n";
        let v: Value = from_str(yaml).unwrap();

        let typos = ["retries", "timout", "max_conections", "enabl_logging"];
        typos
            .iter()
            .map(|&typo| match suggest_key(&v, typo) {
                Some((suggestion, dist)) => {
                    format!("'{typo}' -> did you mean '{suggestion}'? (distance: {dist})")
                }
                None => format!("'{typo}' -> no suggestion"),
            })
            .collect()
    });

    // ── Config validation ────────────────────────────────────────────
    support::task_with_output("Validate config with required fields", || {
        let yaml = "hoost: localhost\npoort: 8080\nname: myapp\n";
        let v: Value = from_str(yaml).unwrap();

        let required = ["host", "port", "name", "version"];
        let mut lines = Vec::new();

        for &key in &required {
            if v.get(key).is_some() {
                lines.push(format!("'{key}' -> found"));
            } else {
                let issues = validate_config(&v, &[key]);
                for issue in issues {
                    lines.push(format!("'{key}' -> {issue}"));
                }
            }
        }
        lines
    });

    // ── Edit distance examples ───────────────────────────────────────
    support::task_with_output("Levenshtein distance examples", || {
        let pairs = [
            ("kitten", "sitting"),
            ("retry", "retries"),
            ("timeout", "timout"),
            ("host", "hoost"),
            ("exact", "exact"),
        ];
        pairs
            .iter()
            .map(|(a, b)| format!("d(\"{a}\", \"{b}\") = {}", levenshtein(a, b)))
            .collect()
    });

    support::summary(3);
}
