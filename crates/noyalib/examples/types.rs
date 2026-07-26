// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Custom YAML tags: local (!), global (!!), and user-defined types.
//!
//! Run: `cargo run --example types`

#[path = "support.rs"]
mod support;

use noyalib::{Tag, TaggedValue, Value, from_str, to_string};

fn main() {
    support::header("noyalib -- types");

    // ── Parse tagged values ──────────────────────────────────────────
    support::task_with_output("Parse tagged values", || {
        // Tags are resolved during parsing. Local tags (!) on scalars are
        // consumed by the schema resolver. To demonstrate tag handling,
        // construct tagged values programmatically.
        let tagged = Value::Tagged(Box::new(TaggedValue::new(
            Tag::new("!timestamp"),
            Value::String("2024-01-15T10:30:00Z".to_string()),
        )));
        match &tagged {
            Value::Tagged(t) => vec![
                format!("tag   = {}", t.tag()),
                format!("value = {}", t.value()),
            ],
            _ => vec!["unexpected".to_string()],
        }
    });

    // ── Construct tagged values programmatically ─────────────────────
    support::task_with_output("Construct tagged values", || {
        let tagged = Value::Tagged(Box::new(TaggedValue::new(
            Tag::new("!color"),
            Value::String("#FF5733".to_string()),
        )));
        let yaml = to_string(&tagged).unwrap();
        vec![format!("output: {}", yaml.trim())]
    });

    // ── Tag inspection and stripping ─────────────────────────────────
    support::task_with_output("Inspect and strip tags", || {
        let tagged = Value::Tagged(Box::new(TaggedValue::new(
            Tag::new("!secret"),
            Value::String("encrypted-data-here".to_string()),
        )));

        let mut lines = Vec::new();
        if let Value::Tagged(t) = &tagged {
            lines.push(format!("tag()    = {}", t.tag()));
            lines.push(format!("nobang() = {}", noyalib::nobang(t.tag().as_str())));
            lines.push(format!("value()  = {}", t.value()));
        }
        let stripped = tagged.untag();
        lines.push(format!("untag()  = {stripped}"));
        lines
    });

    // ── Global tags (!!type) ─────────────────────────────────────────
    support::task_with_output("Global tags (!!int, !!str)", || {
        let yaml = "typed_int: !!int 42\ntyped_str: !!str 42\n";
        let v: Value = from_str(yaml).unwrap();
        vec![
            format!(
                "!!int 42 -> {}",
                match &v["typed_int"] {
                    Value::Number(_) => "Number",
                    Value::Tagged(_) => "Tagged",
                    _ => "Other",
                }
            ),
            format!(
                "!!str 42 -> {}",
                match &v["typed_str"] {
                    Value::String(_) => "String",
                    Value::Tagged(_) => "Tagged",
                    _ => "Other",
                }
            ),
        ]
    });

    // ── Tag-based dispatch pattern ───────────────────────────────────
    support::task_with_output("Tag-based dispatch pattern", || {
        // Build tagged items programmatically to demonstrate dispatch
        let items = [
            ("!email", "user@example.com", "validate_email"),
            ("!url", "https://example.com", "validate_url"),
            ("!phone", "+1-555-0100", "validate_phone"),
        ];
        items
            .iter()
            .map(|(tag, val, handler)| format!("{tag} {val} -> {handler}()"))
            .collect()
    });

    support::summary(5);
}
