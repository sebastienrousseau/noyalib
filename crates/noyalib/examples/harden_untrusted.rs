// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Hardening a parser against hostile YAML.
//!
//! Untrusted YAML is an attack surface: anchors/aliases enable
//! "billion-laughs" amplification, deep nesting invites stack
//! exhaustion, and oversized or richly-typed payloads waste memory. The
//! library answers each with a *bounded, typed rejection* — never
//! unbounded work — through two layers:
//!
//!   * numeric budgets on [`noyalib::ParserConfig`]
//!     (`max_depth`, `max_alias_expansions`, `max_nodes`, …), and
//!   * pluggable [`noyalib::policy`] guards
//!     (`DenyAnchors`, `DenyTags`, `MaxScalarLength`, or your own).
//!
//! Every scenario below feeds a hostile document to a hardened config
//! and shows the specific defense that refuses it, then parses one
//! legitimate document to prove the config still accepts real input.
//!
//! All parses go through `load_all_with_config`, the AST-loader path on
//! which the budgets and policies are enforced.
//!
//! Run with: `cargo run --example harden_untrusted`

mod support;

use noyalib::policy::{DenyAnchors, DenyTags, MaxScalarLength};
use noyalib::{ParserConfig, load_all_with_config};

/// Parse the stream and report whether the hardened config accepted it
/// or which defense rejected it.
fn outcome(yaml: &str, cfg: &ParserConfig) -> Vec<String> {
    match load_all_with_config(yaml, cfg).and_then(Iterator::collect::<Result<Vec<_>, _>>) {
        Ok(docs) => vec![format!("accepted: {} document(s)", docs.len())],
        Err(e) => vec![format!("rejected: {e}")],
    }
}

fn main() {
    support::header("noyalib -- harden untrusted");

    // 1. Anchors/aliases disabled outright by policy.
    support::task_with_output("Policy: DenyAnchors refuses anchor definitions", || {
        let cfg = ParserConfig::new().with_policy(DenyAnchors);
        outcome("shared: &a 1\nuse: *a\n", &cfg)
    });

    // 2. Custom (non-standard) tags disabled by policy.
    support::task_with_output("Policy: DenyTags refuses custom tags", || {
        let cfg = ParserConfig::new().with_policy(DenyTags);
        outcome("payload: !SomeType { drop: table }\n", &cfg)
    });

    // 3. Per-scalar size cap by policy.
    support::task_with_output("Policy: MaxScalarLength caps scalar size", || {
        let cfg = ParserConfig::new().with_policy(MaxScalarLength(16));
        outcome("blob: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n", &cfg)
    });

    // 4. Billion-laughs: nested aliases that would expand exponentially.
    //    `max_alias_expansions` trips long before memory blows up.
    support::task_with_output(
        "Budget: max_alias_expansions defuses a billion-laughs bomb",
        || {
            let bomb = "l0: &l0 \"lol\"\n\
                    l1: &l1 [*l0,*l0,*l0,*l0,*l0,*l0,*l0,*l0,*l0,*l0]\n\
                    l2: &l2 [*l1,*l1,*l1,*l1,*l1,*l1,*l1,*l1,*l1,*l1]\n\
                    l3: &l3 [*l2,*l2,*l2,*l2,*l2,*l2,*l2,*l2,*l2,*l2]\n\
                    boom: *l3\n";
            let cfg = ParserConfig::new()
                .max_alias_expansions(10)
                .alias_anchor_ratio(None);
            outcome(bomb, &cfg)
        },
    );

    // 5. Node bomb: many tiny empty collections — trivial bytes, few
    //    events, but a huge node count. `max_nodes` is the only budget
    //    that bounds it.
    support::task_with_output(
        "Budget: max_nodes bounds an empty-collection node bomb",
        || {
            let mut bomb = String::from("[");
            for i in 0..200 {
                if i > 0 {
                    bomb.push(',');
                }
                bomb.push_str("{}");
            }
            bomb.push(']');
            let cfg = ParserConfig::new().max_nodes(64);
            outcome(&bomb, &cfg)
        },
    );

    // 6. Deep nesting: bounded by max_depth, no stack exhaustion.
    support::task_with_output("Budget: max_depth rejects deep nesting", || {
        let mut deep = String::new();
        for _ in 0..64 {
            deep.push_str("[ ");
        }
        deep.push('0');
        for _ in 0..64 {
            deep.push_str(" ]");
        }
        deep.push('\n');
        let cfg = ParserConfig::new().max_depth(8);
        outcome(&deep, &cfg)
    });

    // 7. A legitimate document still parses under a combined hardened
    //    profile — hardening rejects abuse, not real configuration.
    support::task_with_output(
        "Legitimate config accepted under a hardened profile",
        || {
            let cfg = ParserConfig::new()
                .max_depth(32)
                .max_alias_expansions(128)
                .max_nodes(10_000)
                .with_policy(DenyTags)
                .with_policy(MaxScalarLength(4096));
            let doc = "service: api\nreplicas: 3\nports:\n  - 8080\n  - 8443\nlimits:\n  cpu: \"500m\"\n  memory: 256Mi\n";
            outcome(doc, &cfg)
        },
    );

    support::summary(7);
}
