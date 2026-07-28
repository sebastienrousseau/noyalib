// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! validator — the established declarative-validation crate.
//!
//! [`validator`] is the long-established sibling of `garde` —
//! attribute-driven, web-framework-friendly (Actix, Axum, Rocket
//! all integrate with it), and used by a large fraction of the
//! Rust web ecosystem. noyalib ships [`ValidatedValidator<T>`] so
//! parsed YAML can be checked against a validator schema in the
//! same idiomatic way.
//!
//! Run: `cargo run --example validation_validator --features validator`

#[path = "support.rs"]
mod support;

use noyalib::ValidatedValidator;
use validator::Validate;

#[derive(Debug, serde::Deserialize, Validate)]
#[allow(dead_code)]
struct UserAccount {
    #[validate(email)]
    email: String,

    #[validate(length(min = 3, max = 32))]
    username: String,

    #[validate(range(min = 13, max = 130))]
    age: u8,

    #[validate(url)]
    website: String,
}

#[derive(Debug, serde::Deserialize, Validate)]
#[allow(dead_code)]
struct ApiKey {
    #[validate(length(equal = 40))]
    token: String,

    #[validate(range(min = 1, max = 86_400))]
    ttl_seconds: u32,
}

fn main() {
    support::header("validator — declarative validation (Actix / Axum stack)");

    // ── Happy path ──────────────────────────────────────────────────
    support::task_with_output("Valid user account passes every rule", || {
        let yaml = "
email: alice@example.com
username: alice
age: 30
website: https://alice.example.com
";
        let cfg: ValidatedValidator<UserAccount> = noyalib::from_str(yaml).unwrap();
        vec![
            format!("email    = {}", cfg.email),
            format!("username = {}", cfg.username),
            format!("age      = {}", cfg.age),
            format!("website  = {}", cfg.website),
        ]
    });

    // ── email format rule ───────────────────────────────────────────
    support::task_with_output("Malformed email rejected by `validate(email)`", || {
        let yaml = "
email: not-an-email
username: alice
age: 30
website: https://alice.example.com
";
        let res: Result<ValidatedValidator<UserAccount>, _> = noyalib::from_str(yaml);
        match res {
            Ok(_) => vec!["unexpected: parse succeeded".into()],
            Err(e) => vec![format!("error: {e}")],
        }
    });

    // ── range rule ──────────────────────────────────────────────────
    support::task_with_output("Out-of-range age rejected", || {
        let yaml = "
email: bob@example.com
username: bob
age: 5
website: https://bob.example.com
";
        let res: Result<ValidatedValidator<UserAccount>, _> = noyalib::from_str(yaml);
        match res {
            Ok(_) => vec!["unexpected: parse succeeded".into()],
            Err(e) => vec![format!("error: {e}")],
        }
    });

    // ── exact-length rule ───────────────────────────────────────────
    support::task_with_output("Token of wrong length rejected", || {
        let yaml = "
token: short
ttl_seconds: 3600
";
        let res: Result<ValidatedValidator<ApiKey>, _> = noyalib::from_str(yaml);
        match res {
            Ok(_) => vec!["unexpected: parse succeeded".into()],
            Err(e) => vec![format!("error: {e}")],
        }
    });

    // ── token of correct length passes ──────────────────────────────
    support::task_with_output("Correctly-shaped token + sane TTL passes", || {
        let yaml = "
token: 0123456789abcdef0123456789abcdef01234567
ttl_seconds: 3600
";
        let cfg: ValidatedValidator<ApiKey> = noyalib::from_str(yaml).unwrap();
        vec![
            format!("token       = {}…", &cfg.token[..8]),
            format!("ttl_seconds = {}", cfg.ttl_seconds),
        ]
    });

    println!();
    println!("  `validator` is the idiomatic choice for HTTP-handler");
    println!("  payloads in Axum / Actix / Rocket. Pair it with noyalib's");
    println!("  `ValidatedValidator<T>` and a YAML config request body");
    println!("  is checked the same way the corresponding JSON one is.");

    support::footer();
}
