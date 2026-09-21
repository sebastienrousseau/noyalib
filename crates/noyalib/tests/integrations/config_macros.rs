// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `parser_config!` / `serializer_config!` declarative builders.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::{DuplicateKeyPolicy, ParserConfig, YamlVersion, parser_config, serializer_config};

#[test]
fn parser_config_empty_form() {
    let macro_cfg = parser_config! {};
    let manual_cfg = ParserConfig::new();
    assert_eq!(macro_cfg.max_depth, manual_cfg.max_depth);
    assert_eq!(macro_cfg.strict_booleans, manual_cfg.strict_booleans);
}

#[test]
fn parser_config_single_field() {
    let cfg = parser_config! {
        max_depth: 32,
    };
    assert_eq!(cfg.max_depth, 32);
}

#[test]
fn parser_config_multi_field() {
    let cfg = parser_config! {
        max_depth: 32,
        max_alias_expansions: 200,
        strict_booleans: true,
        legacy_booleans: false,
    };
    assert_eq!(cfg.max_depth, 32);
    assert_eq!(cfg.max_alias_expansions, 200);
    assert!(cfg.strict_booleans);
    assert!(!cfg.legacy_booleans);
}

#[test]
fn parser_config_enum_value() {
    let cfg = parser_config! {
        duplicate_key_policy: DuplicateKeyPolicy::Error,
        version: YamlVersion::V1_1,
    };
    assert_eq!(cfg.duplicate_key_policy, DuplicateKeyPolicy::Error);
}

#[test]
fn parser_config_trailing_comma_optional() {
    // With and without trailing comma — both must parse.
    let with = parser_config! { max_depth: 7, };
    let without = parser_config! { max_depth: 7 };
    assert_eq!(with.max_depth, without.max_depth);
}

#[test]
fn parser_config_matches_manual_chain() {
    let macro_cfg = parser_config! {
        max_depth: 16,
        max_alias_expansions: 50,
        strict_booleans: true,
    };
    let manual_cfg = ParserConfig::new()
        .max_depth(16)
        .max_alias_expansions(50)
        .strict_booleans(true);
    assert_eq!(macro_cfg.max_depth, manual_cfg.max_depth);
    assert_eq!(
        macro_cfg.max_alias_expansions,
        manual_cfg.max_alias_expansions
    );
    assert_eq!(macro_cfg.strict_booleans, manual_cfg.strict_booleans);
}

#[test]
fn serializer_config_empty_form() {
    let _ = serializer_config! {};
}

#[test]
fn serializer_config_single_field() {
    let cfg = serializer_config! {
        indent: 4,
    };
    assert_eq!(cfg.indent, 4);
}

#[test]
fn serializer_config_multi_field() {
    let cfg = serializer_config! {
        indent: 2,
        quote_all: true,
    };
    assert_eq!(cfg.indent, 2);
    assert!(cfg.quote_all);
}
