//! Enum serialization example for noyalib.
//!
//! Demonstrates serializing various enum types.
//!
//! Run: `cargo run --example variants`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Shape {
    Rectangle { width: u32, height: u32 },
    Circle { radius: f64 },
    Triangle { base: u32, height: u32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Status {
    Active,
    Inactive,
    Pending,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Message {
    Text(String),
    Number(i32),
    Data { id: u32, payload: String },
}

fn main() {
    support::header("noyalib -- variants");

    support::task_with_output("Serialize and roundtrip struct variants", || {
        let shapes = vec![
            Shape::Rectangle {
                width: 10,
                height: 20,
            },
            Shape::Circle { radius: 5.0 },
            Shape::Triangle {
                base: 8,
                height: 12,
            },
        ];

        let yaml = to_string(&shapes).unwrap();

        let parsed: Vec<Shape> = from_str(&yaml).unwrap();
        assert_eq!(shapes, parsed);

        let mut lines = vec!["Shapes serialized:".to_string()];
        lines.extend(yaml.lines().map(|l| l.to_string()));
        lines
    });

    support::task_with_output("Serialize and roundtrip unit variants", || {
        let status = Status::Active;
        let yaml = to_string(&status).unwrap();

        let parsed: Status = from_str(&yaml).unwrap();
        assert_eq!(status, parsed);

        vec![format!("Status serialized: {}", yaml.trim_end())]
    });

    support::task_with_output("Serialize and roundtrip mixed variants", || {
        let messages = vec![
            Message::Text("Hello".to_string()),
            Message::Number(42),
            Message::Data {
                id: 1,
                payload: "test".to_string(),
            },
        ];

        let yaml = to_string(&messages).unwrap();

        let parsed: Vec<Message> = from_str(&yaml).unwrap();
        assert_eq!(messages, parsed);

        let mut lines = vec!["Messages serialized:".to_string()];
        lines.extend(yaml.lines().map(|l| l.to_string()));
        lines
    });

    support::summary(3);
}
