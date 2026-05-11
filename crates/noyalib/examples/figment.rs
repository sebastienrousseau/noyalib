// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Figment provider — layered configuration with YAML, env vars, and
//! defaults composed into a single typed struct.
//!
//! [`figment`] is the de-facto-standard layered configuration crate
//! in the Rust ecosystem (Rocket, several K8s operators, many
//! Tokio-based services). The pattern: start with built-in
//! defaults, layer a YAML config file on top, finalise with
//! environment-variable overrides — last-write-wins. noyalib ships
//! a [`figment::Provider`] for YAML so the whole chain works
//! without depending on the unmaintained `serde_yaml` 0.9 crate.
//!
//! Run: `cargo run --example figment --features figment`

#[path = "support.rs"]
mod support;

use figment::Figment;
use figment::providers::{Env, Format, Serialized};
use noyalib::figment::Yaml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    name: String,
    port: u16,
    log_level: String,
    workers: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "noyalib-app".into(),
            port: 8080,
            log_level: "info".into(),
            workers: 1,
        }
    }
}

const SITE_YAML: &str = "\
name: production-api
port: 9090
log_level: warn
";

fn main() {
    support::header("Figment — layered YAML / env / defaults");

    // ── Layer 1: defaults only ───────────────────────────────────────
    support::task_with_output("Layer 1 — figment with defaults only", || {
        let cfg: AppConfig = Figment::new()
            .merge(Serialized::defaults(AppConfig::default()))
            .extract()
            .expect("defaults must extract");
        vec![
            format!("name       = {}", cfg.name),
            format!("port       = {}", cfg.port),
            format!("log_level  = {}", cfg.log_level),
            format!("workers    = {}", cfg.workers),
        ]
    });

    // ── Layer 2: defaults + YAML overlay ─────────────────────────────
    support::task_with_output("Layer 2 — defaults overlaid by YAML", || {
        let cfg: AppConfig = Figment::new()
            .merge(Serialized::defaults(AppConfig::default()))
            .merge(Yaml::string(SITE_YAML))
            .extract()
            .expect("yaml overlay must extract");
        vec![
            format!("name       = {}  (from YAML)", cfg.name),
            format!("port       = {}     (from YAML)", cfg.port),
            format!("log_level  = {}    (from YAML)", cfg.log_level),
            format!("workers    = {}        (still default)", cfg.workers),
        ]
    });

    // ── Layer 3: defaults + YAML + env overrides ─────────────────────
    support::task_with_output("Layer 3 — env vars override the YAML layer", || {
        // Demonstrate env-overlay semantics without mutating the
        // process environment — edition 2024 marks
        // `std::env::set_var` / `remove_var` as unsafe (they're
        // not thread-safe), and the crate sits under
        // `#![forbid(unsafe_code)]`. Use figment's `Env::raw` with
        // a synthetic map instead so the example stays
        // illustrative and safe.
        let env = Env::raw()
            .only(&["NOYAEX_PORT", "NOYAEX_WORKERS"])
            .map(|k| match k.as_str() {
                "NOYAEX_PORT" => "PORT".into(),
                "NOYAEX_WORKERS" => "WORKERS".into(),
                other => other.into(),
            });

        // Inject the values via a one-off serialised layer that
        // shadows env. The Env::raw provider above is included so
        // the example still demonstrates Env wiring in real
        // pipelines; the synthetic overlay below stands in for the
        // OS env that those pipelines would carry.
        #[derive(Serialize)]
        struct EnvOverlay {
            port: u16,
            workers: u16,
        }
        let cfg: AppConfig = Figment::new()
            .merge(Serialized::defaults(AppConfig::default()))
            .merge(Yaml::string(SITE_YAML))
            .merge(env)
            .merge(Serialized::defaults(EnvOverlay {
                port: 7000,
                workers: 8,
            }))
            .extract()
            .expect("env overlay must extract");

        vec![
            format!("name       = {}  (from YAML)", cfg.name),
            format!("port       = {}     (from env: NOYAEX_PORT)", cfg.port),
            format!("log_level  = {}    (from YAML)", cfg.log_level),
            format!(
                "workers    = {}        (from env: NOYAEX_WORKERS)",
                cfg.workers
            ),
        ]
    });

    // ── Pattern: per-environment YAML files ──────────────────────────
    support::task_with_output(
        "Pattern — production overrides staging overrides base",
        || {
            const BASE: &str = "name: api\nport: 8080\nlog_level: debug\nworkers: 1\n";
            const STAGING: &str = "log_level: info\nworkers: 4\n";
            const PROD: &str = "port: 443\nlog_level: warn\nworkers: 16\n";

            let cfg: AppConfig = Figment::new()
                .merge(Yaml::string(BASE))
                .merge(Yaml::string(STAGING))
                .merge(Yaml::string(PROD))
                .extract()
                .expect("multi-file overlay must extract");

            vec![
                format!("name       = {}    (base)", cfg.name),
                format!("port       = {}      (prod)", cfg.port),
                format!("log_level  = {}    (prod)", cfg.log_level),
                format!("workers    = {}       (prod)", cfg.workers),
            ]
        },
    );

    println!();
    println!("  Figment lets you compose configuration the way 12-factor");
    println!("  apps demand: defaults in code, environment-specific YAML");
    println!("  overlays, env-var overrides at deploy time. noyalib slots");
    println!("  in as the YAML provider — no `serde_yaml` 0.9 dependency.");

    support::footer();
}
