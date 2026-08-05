// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Recursive data structures: trees, graphs, org charts.
//!
//! Demonstrates Box<T>, Vec<Box<T>>, and Option for self-referential
//! types that YAML handles naturally but Rust needs indirection for.
//!
//! Run: `cargo run --example recursive`

#[path = "support.rs"]
mod support;

use noyalib::{from_str, to_string};

// ── File system tree ─────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
enum FsEntry {
    File { name: String, size: u64 },
    Dir { name: String, children: Vec<Self> },
}

// ── Org chart ────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct Person {
    name: String,
    title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    reports: Vec<Self>,
}

// ── Generic tree (Box<T> recursion) ──────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct TreeNode {
    label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    children: Vec<Self>,
}

fn count_tree(n: &TreeNode) -> usize {
    1 + n.children.iter().map(count_tree).sum::<usize>()
}

fn tree_depth(n: &TreeNode) -> usize {
    if n.children.is_empty() {
        1
    } else {
        1 + n.children.iter().map(tree_depth).max().unwrap_or(0)
    }
}

fn main() {
    support::header("noyalib -- recursive");

    // ── File system tree ─────────────────────────────────────────────
    support::task_with_output("File system tree (recursive Vec)", || {
        let yaml = r"
- name: src
  children:
    - name: lib.rs
      size: 2048
    - name: parser
      children:
        - name: scanner.rs
          size: 15360
        - name: events.rs
          size: 8192
        - name: loader.rs
          size: 12288
- name: Cargo.toml
  size: 1024
";
        let tree: Vec<FsEntry> = from_str(yaml).unwrap();
        let yaml_out = to_string(&tree).unwrap();
        let rt: Vec<FsEntry> = from_str(&yaml_out).unwrap();

        fn count_entries(entries: &[FsEntry]) -> usize {
            entries
                .iter()
                .map(|e| match e {
                    FsEntry::File { .. } => 1,
                    FsEntry::Dir { children, .. } => 1 + count_entries(children),
                })
                .sum()
        }

        vec![
            format!("entries   = {}", count_entries(&tree)),
            format!("depth     = 3 (src/parser/scanner.rs)"),
            format!("roundtrip = {}", tree == rt),
        ]
    });

    // ── Org chart ────────────────────────────────────────────────────
    support::task_with_output("Org chart (recursive struct)", || {
        let yaml = r"
name: Alice
title: CEO
reports:
  - name: Bob
    title: CTO
    reports:
      - name: Carol
        title: Lead Engineer
        reports:
          - name: Dave
            title: Engineer
      - name: Eve
        title: Engineer
  - name: Frank
    title: CFO
";
        let org: Person = from_str(yaml).unwrap();
        let yaml_out = to_string(&org).unwrap();
        let rt: Person = from_str(&yaml_out).unwrap();

        fn count_people(p: &Person) -> usize {
            1 + p.reports.iter().map(count_people).sum::<usize>()
        }

        fn max_depth(p: &Person) -> usize {
            if p.reports.is_empty() {
                1
            } else {
                1 + p.reports.iter().map(max_depth).max().unwrap_or(0)
            }
        }

        vec![
            format!("root      = {} ({})", org.name, org.title),
            format!("people    = {}", count_people(&org)),
            format!("depth     = {}", max_depth(&org)),
            format!("roundtrip = {}", org == rt),
        ]
    });

    // ── Generic tree (recursive children) ──────────────────────────────
    support::task_with_output("Generic tree (recursive Vec<TreeNode>)", || {
        let yaml = r"
label: root
children:
  - label: A
    children:
      - label: A1
      - label: A2
        children:
          - label: A2a
  - label: B
  - label: C
    children:
      - label: C1
";
        let tree: TreeNode = from_str(yaml).unwrap();
        let yaml_out = to_string(&tree).unwrap();
        let rt: TreeNode = from_str(&yaml_out).unwrap();

        vec![
            format!("root      = {}", tree.label),
            format!("nodes     = {}", count_tree(&tree)),
            format!("depth     = {}", tree_depth(&tree)),
            format!("roundtrip = {}", tree == rt),
        ]
    });

    // ── Deep nesting stress test ─────────────────────────────────────
    support::task_with_output("Deep nesting (50 levels)", || {
        // Build deeply nested TreeNode programmatically
        let mut node = TreeNode {
            label: "leaf".to_string(),
            children: vec![],
        };
        for i in (0..50).rev() {
            node = TreeNode {
                label: format!("level_{i}"),
                children: vec![node],
            };
        }

        let yaml = to_string(&node).unwrap();
        let rt: TreeNode = from_str(&yaml).unwrap();

        vec![
            format!("nodes     = {}", count_tree(&node)),
            format!("depth     = {}", tree_depth(&node)),
            format!("roundtrip = {}", node == rt),
            format!("yaml      = {} bytes", yaml.len()),
        ]
    });

    support::summary(4);
}
