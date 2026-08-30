//! Regression coverage for `compact_list_indent(true)` only applying one
//! level deep (Refs #354).
//!
//! `write_mapping` already applied the compact rule (a sequence value
//! starts at its own key's indentation) at any nesting depth for a
//! mapping's *direct* key. But a mapping or sequence nested *inside a
//! sequence item* went through `write_sequence`'s own, separate
//! item-rendering code, which never consulted `compact_list_indent` at
//! all: a sequence-valued field of a mapping item always got a hardcoded
//! extra indent level, and a sequence item that was itself a sequence was
//! never inlined onto the outer dash's line.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{SerializerConfig, Value, from_str, to_string_with_config};

fn compact() -> SerializerConfig {
    SerializerConfig::new().compact_list_indent(true)
}

fn assert_round_trips(input: &str, config: &SerializerConfig, expected: &str) {
    let v: Value = from_str(input).unwrap();
    let out = to_string_with_config(&v, config).unwrap();
    assert_eq!(out, expected);
    let back: Value = from_str(&out).unwrap();
    assert_eq!(back, v);
}

#[test]
fn compact_sequence_under_a_key_nested_inside_a_sequence_item() {
    assert_round_trips("a:\n- b:\n  - c\n", &compact(), "a:\n- b:\n  - c");
}

#[test]
fn compact_applies_to_every_sequence_field_of_a_sequence_item() {
    assert_round_trips(
        "items:\n- name: x\n  tags:\n  - t1\n  - t2\n",
        &compact(),
        "items:\n- name: x\n  tags:\n  - t1\n  - t2",
    );
}

#[test]
fn compact_sequence_item_that_is_itself_a_sequence_is_inlined() {
    assert_round_trips(
        "seq:\n- - a\n  - b\n- - c\n",
        &compact(),
        "seq:\n- - a\n  - b\n- - c",
    );
}

#[test]
fn compact_sequence_directly_under_a_mapping_key_was_already_right() {
    assert_round_trips(
        "m:\n  inner:\n  - 1\n  - 2\n",
        &compact(),
        "m:\n  inner:\n  - 1\n  - 2",
    );
}

#[test]
fn default_non_compact_output_is_pinned_for_the_first_two_inputs() {
    // Guards against the compact fix leaking into the default
    // (`compact_list_indent` off) output, which must stay exactly what
    // it is today.
    let default = SerializerConfig::default();

    let v1: Value = from_str("a:\n- b:\n  - c\n").unwrap();
    assert_eq!(
        to_string_with_config(&v1, &default).unwrap(),
        "a:\n  - b:\n      - c"
    );

    let v2: Value = from_str("items:\n- name: x\n  tags:\n  - t1\n  - t2\n").unwrap();
    assert_eq!(
        to_string_with_config(&v2, &default).unwrap(),
        "items:\n  - name: x\n    tags:\n      - t1\n      - t2"
    );
}
