// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Singleton map enum serialization: singleton_map, optional, recursive.
//!
//! Run: `cargo run --example tags`

#[path = "support.rs"]
mod support;

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Status {
    Pending,
    Active,
    Completed { at: String },
    Error(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Task {
    name: String,
    #[serde(with = "noyalib::with::singleton_map")]
    status: Status,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct OptionalTask {
    name: String,
    #[serde(
        with = "noyalib::with::singleton_map_optional",
        skip_serializing_if = "Option::is_none",
        default
    )]
    status: Option<Status>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Action {
    Simple,
    WithData { value: i32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Workflow {
    name: String,
    #[serde(with = "noyalib::with::singleton_map_recursive")]
    actions: Vec<Action>,
}

fn main() {
    support::header("noyalib -- tags");

    // ── Basic: each variant type ─────────────────────────────────────
    support::task_with_output("Serialize enum variants as singleton maps", || {
        let tasks = [
            (
                "Pending",
                Task {
                    name: "Task 1".into(),
                    status: Status::Pending,
                },
            ),
            (
                "Active",
                Task {
                    name: "Task 2".into(),
                    status: Status::Active,
                },
            ),
            (
                "Completed",
                Task {
                    name: "Task 3".into(),
                    status: Status::Completed {
                        at: "2024-01-15".into(),
                    },
                },
            ),
            (
                "Error",
                Task {
                    name: "Task 4".into(),
                    status: Status::Error("Connection failed".into()),
                },
            ),
        ];
        tasks
            .iter()
            .map(|(label, task)| {
                let yaml = to_string(task).unwrap();
                // Extract the status block (everything after "status:")
                let status: String = yaml
                    .lines()
                    .skip_while(|l| !l.starts_with("status"))
                    .map(|l| l.trim().to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{label:<9} -> {status}")
            })
            .collect()
    });

    // ── Roundtrip ────────────────────────────────────────────────────
    support::task_with_output("Roundtrip verification", || {
        let original = Task {
            name: "Task 3".into(),
            status: Status::Completed {
                at: "2024-01-15".into(),
            },
        };
        let yaml = to_string(&original).unwrap();
        let parsed: Task = from_str(&yaml).unwrap();
        assert_eq!(original, parsed);
        vec![
            format!("Original = {:?}", original),
            format!("Parsed   = {:?}", parsed),
            "Status   = match".to_string(),
        ]
    });

    // ── Optional ─────────────────────────────────────────────────────
    support::task_with_output("Optional singleton_map (Some vs None)", || {
        let with = OptionalTask {
            name: "Has status".into(),
            status: Some(Status::Active),
        };
        let without = OptionalTask {
            name: "No status".into(),
            status: None,
        };
        let with_yaml = to_string(&with).unwrap();
        let without_yaml = to_string(&without).unwrap();
        vec![
            format!("Some(Active) -> {} lines", with_yaml.lines().count()),
            format!(
                "None         -> {} lines (status omitted)",
                without_yaml.lines().count()
            ),
        ]
    });

    // ── Recursive ────────────────────────────────────────────────────
    support::task_with_output("Recursive singleton_map (nested enums)", || {
        let workflow = Workflow {
            name: "My Workflow".into(),
            actions: vec![
                Action::Simple,
                Action::WithData { value: 42 },
                Action::Simple,
            ],
        };
        let yaml = to_string(&workflow).unwrap();
        let parsed: Workflow = from_str(&yaml).unwrap();
        assert_eq!(workflow, parsed);
        yaml.lines().map(|l| l.to_string()).collect()
    });

    // ── Parse singleton map YAML ─────────────────────────────────────
    support::task_with_output("Parse singleton map format", || {
        let yaml = "name: Parsed Task\nstatus:\n  Completed:\n    at: \"2024-12-01\"\n";
        let parsed: Task = from_str(yaml).unwrap();
        vec![
            format!("Task   = {}", parsed.name),
            format!("Status = Completed (at: 2024-12-01)"),
        ]
    });

    support::summary(5);
}
