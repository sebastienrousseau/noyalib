// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `parser_config!` / `serializer_config!` — declarative
//! field-value builders that expand to the existing chained
//! setter calls.
//!
//! Zero runtime overhead: the macro expansion is byte-identical
//! to writing the builder chain by hand. The motivating
//! ergonomics: dense call sites stay readable when you need to
//! flip half-a-dozen flags.
//!
//! Run: `cargo run --example config_macros`

#[path = "support/mod.rs"]
mod support;

use noyalib::{
    DuplicateKeyPolicy, ParserConfig, SerializerConfig, YamlVersion, parser_config,
    serializer_config,
};

fn main() {
    support::header("noyalib -- config_macros");

    support::task_with_output(
        "parser_config! — flip several knobs in one expression",
        || {
            let cfg = parser_config! {
                max_depth: 32,
                max_alias_expansions: 200,
                strict_booleans: true,
                duplicate_key_policy: DuplicateKeyPolicy::Error,
                version: YamlVersion::V1_1,
            };
            vec![
                format!("max_depth             = {}", cfg.max_depth),
                format!("max_alias_expansions  = {}", cfg.max_alias_expansions),
                format!("strict_booleans       = {}", cfg.strict_booleans),
                format!("duplicate_key_policy  = {:?}", cfg.duplicate_key_policy),
                format!("version               = {:?}", cfg.yaml_version),
            ]
        },
    );

    support::task_with_output("parser_config! {} — empty form == new()", || {
        let from_macro = parser_config! {};
        let from_new = ParserConfig::new();
        vec![
            format!("macro.max_depth = new.max_depth = {}", from_macro.max_depth),
            format!("both equal? {}", from_macro.max_depth == from_new.max_depth),
        ]
    });

    support::task_with_output("serializer_config! — same pattern for emit knobs", || {
        let cfg = serializer_config! {
            indent: 4,
            quote_all: true,
        };
        let _ = SerializerConfig::new(); // sanity import
        vec![
            format!("indent     = {}", cfg.indent),
            format!("quote_all  = {}", cfg.quote_all),
        ]
    });

    support::task_with_output(
        "Equivalent to chained builders — pick whichever reads cleaner",
        || {
            let from_macro = parser_config! {
                max_depth: 16,
                max_alias_expansions: 50,
            };
            let from_chain = ParserConfig::new().max_depth(16).max_alias_expansions(50);
            vec![format!(
                "max_depth match: {}, alias match: {}",
                from_macro.max_depth == from_chain.max_depth,
                from_macro.max_alias_expansions == from_chain.max_alias_expansions
            )]
        },
    );
}
