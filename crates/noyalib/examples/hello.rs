// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Basic usage: serialize and deserialize structs.
//!
//! Run: `cargo run --example basic`

#[path = "support.rs"]
mod support;

use noyalib::{from_str, to_string};

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct Person {
    name: String,
    age: u32,
    city: String,
}

fn main() {
    support::header("noyalib -- hello");

    let person = Person {
        name: "John Doe".to_string(),
        age: 30,
        city: "New York".to_string(),
    };

    let yaml = support::task("Serialize struct to YAML", || to_string(&person).unwrap());

    let parsed: Person = support::task("Deserialize YAML to struct", || from_str(&yaml).unwrap());

    support::task("Verify round-trip", || assert_eq!(person, parsed));

    support::summary(3);
}
