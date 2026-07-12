// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Formatting wrappers for per-value output control.
//!
//! Run: `cargo run --example style`

#[path = "support.rs"]
mod support;

use std::collections::BTreeMap;

use noyalib::fmt::{Commented, FlowMap, FlowSeq, LitString, SpaceAfter};
use noyalib::to_string;
use serde::Serialize;

#[derive(Serialize)]
struct Config {
    tags: FlowSeq<Vec<String>>,
    metadata: FlowMap<BTreeMap<String, String>>,
    script: LitString,
    name: Commented<String>,
    section: SpaceAfter<String>,
}

fn main() {
    support::header("noyalib -- style");

    support::task("FlowSeq: inline sequence [a, b, c]", || {
        let v = FlowSeq(vec!["yaml".to_string(), "serde".to_string()]);
        let _ = to_string(&v).unwrap();
    });

    support::task("FlowMap: inline mapping {k: v}", || {
        let mut m = BTreeMap::new();
        let _ = m.insert("version".to_string(), "1.0".to_string());
        let v = FlowMap(m);
        let _ = to_string(&v).unwrap();
    });

    support::task("LitString: literal block scalar |", || {
        let v = LitString("#!/bin/sh\necho hello\n".to_string());
        let _ = to_string(&v).unwrap();
    });

    support::task("Commented: inline comment", || {
        let v = Commented::new("noyalib".to_string(), "the project name");
        let _ = to_string(&v).unwrap();
    });

    support::task("SpaceAfter: trailing blank line", || {
        let v = SpaceAfter("end of header".to_string());
        let _ = to_string(&v).unwrap();
    });

    support::task_with_output("Combined struct serialization", || {
        let mut metadata = BTreeMap::new();
        let _ = metadata.insert("version".into(), "1.0".into());
        let _ = metadata.insert("author".into(), "team".into());

        let config = Config {
            tags: FlowSeq(vec!["yaml".into(), "serde".into(), "rust".into()]),
            metadata: FlowMap(metadata),
            script: LitString("#!/bin/sh\necho hello\nexit 0\n".into()),
            name: Commented::new("noyalib".into(), "the project name"),
            section: SpaceAfter("end of header".into()),
        };

        let yaml = to_string(&config).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    support::summary(6);
}
