// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Regression test for issue #46 — pnpm-lock.yaml parsing.
//!
//! Reported behaviour: large but shallow `pnpm-lock.yaml` files trip
//! `RecursionLimitExceeded` with default settings even though the
//! YAML's real nesting depth is ~5 levels.

use std::fmt::Write;

/// Build a `pnpm-lock.yaml`-shaped document with `n` packages.
/// True nesting depth tops out at 5 (root → packages → key → field → value).
fn make_lockfile(n: usize) -> String {
    let mut yaml = String::new();
    let _ = writeln!(yaml, "lockfileVersion: '9.0'");
    let _ = writeln!(yaml, "settings:");
    let _ = writeln!(yaml, "  autoInstallPeers: true");
    let _ = writeln!(yaml, "  excludeLinksFromLockfile: false");
    let _ = writeln!(yaml, "importers:");
    let _ = writeln!(yaml, "  .:");
    let _ = writeln!(yaml, "    dependencies:");
    for i in 0..50.min(n) {
        let _ = writeln!(yaml, "      pkg-{i}:");
        let _ = writeln!(yaml, "        specifier: ^1.0.0");
        let _ = writeln!(yaml, "        version: 1.0.{i}");
    }
    let _ = writeln!(yaml, "packages:");
    for i in 0..n {
        let _ = writeln!(yaml, "  pkg-{i}@1.0.{i}:");
        let _ = writeln!(
            yaml,
            "    resolution: {{integrity: sha512-aaaa{i}, tarball: https://example.invalid/x}}"
        );
        let _ = writeln!(yaml, "    engines: {{node: '>=14'}}");
        let _ = writeln!(yaml, "    dependencies:");
        let _ = writeln!(yaml, "      pkg-{}: 1.0.{}", (i + 1) % n, (i + 1) % n);
    }
    let _ = writeln!(yaml, "snapshots:");
    for i in 0..n {
        let _ = writeln!(yaml, "  pkg-{i}@1.0.{i}:");
        let _ = writeln!(yaml, "    dependencies:");
        let _ = writeln!(yaml, "      pkg-{}: 1.0.{}", (i + 1) % n, (i + 1) % n);
    }
    yaml
}

#[test]
fn pnpm_lockfile_500_pkgs_parses_with_default_config() {
    let yaml = make_lockfile(500);
    let v: noyalib::Value = noyalib::from_str(&yaml).expect("default config should parse");
    match v {
        noyalib::Value::Mapping(m) => assert!(m.contains_key("packages")),
        _ => panic!("expected top-level mapping"),
    }
}

#[test]
fn pnpm_lockfile_2000_pkgs_parses_with_default_config() {
    // ~10k+ lines — matches the issue reporter's scale.
    let yaml = make_lockfile(2000);
    let v: noyalib::Value = noyalib::from_str(&yaml).expect("default config should parse");
    match v {
        noyalib::Value::Mapping(m) => assert!(m.contains_key("packages")),
        _ => panic!("expected top-level mapping"),
    }
}

#[test]
fn pnpm_lockfile_5000_pkgs_parses_with_default_config() {
    // Stress: very wide but still shallow.
    let yaml = make_lockfile(5000);
    let v: noyalib::Value = noyalib::from_str(&yaml).expect("default config should parse");
    match v {
        noyalib::Value::Mapping(m) => assert!(m.contains_key("packages")),
        _ => panic!("expected top-level mapping"),
    }
}

/// Regression: every count up to the default `max_depth` cliff
/// (128) used to fail with `RecursionLimitExceeded` because each
/// empty flow mapping `{}` leaked one depth count into the
/// parent scope. Now they all parse cleanly.
#[test]
fn empty_flow_mappings_under_max_depth_cliff_all_parse() {
    for n in [100usize, 128, 129, 130, 200, 500, 1000] {
        let mut yaml = String::from("packages:\n");
        for i in 0..n {
            let _ = writeln!(yaml, "  pkg-{i}: {{}}");
        }
        let r: Result<noyalib::Value, _> = noyalib::from_str(&yaml);
        assert!(r.is_ok(), "n={n}: {:?}", r.err());
    }
}

/// 10k+ lines with empty flow mappings (`{}`) — matches the
/// shape of real pnpm v9 entries that have no transitive deps.
/// Multiple flow `{}` per entry in a single block scope.
#[test]
fn pnpm_lockfile_with_empty_flow_mappings_parses() {
    let mut yaml = String::from("packages:\n");
    for i in 0..3000 {
        let _ = writeln!(yaml, "  pkg-{i}@1.0.{i}: {{}}");
    }
    let v: noyalib::Value = noyalib::from_str(&yaml).expect("default config should parse");
    match v {
        noyalib::Value::Mapping(m) => assert_eq!(m.len(), 1),
        _ => panic!("expected top-level mapping"),
    }
}

/// Companion finding to issue #46: the no-span loader path
/// (used by `from_str::<Value>`'s value-target fast path)
/// was missing the depth-limit check that the span-tracked
/// loader has. Adversarial deeply-nested input would consume
/// stack without ever firing `RecursionLimitExceeded`.
#[test]
fn no_span_loader_honours_max_depth() {
    // 200 levels of nesting — should fire RecursionLimitExceeded
    // under the default `max_depth = 128`.
    let mut yaml = String::new();
    for i in 0..200 {
        let _ = writeln!(yaml, "{:indent$}- ", "", indent = i * 2);
    }
    let r: Result<noyalib::Value, _> = noyalib::from_str(&yaml);
    match r {
        Err(noyalib::Error::RecursionLimitExceeded { depth }) => {
            assert!(depth > 128, "depth = {depth}");
        }
        Err(e) => panic!("expected RecursionLimitExceeded, got: {e}"),
        Ok(_) => panic!("expected RecursionLimitExceeded, got Ok"),
    }
}

/// Pathological key suffixes — pnpm v9 packages with many
/// peer-dependency parens in the key.
#[test]
fn pnpm_lockfile_with_deep_peer_suffixes() {
    let mut yaml = String::from("packages:\n");
    for i in 0..1000 {
        // 6 nested peer suffixes — matches some pnpm pkgs.
        let _ = writeln!(
            yaml,
            "  '@scope/pkg-{i}@1.0.0(p1@18.0.0)(p2@5.0.0)(p3@2.0.0)(p4@4.0.0)(p5@6.0.0)(p6@1.0.0)':"
        );
        let _ = writeln!(yaml, "    resolution: {{integrity: sha512-x}}");
    }
    let _v: noyalib::Value =
        noyalib::from_str(&yaml).expect("deep peer suffixes should parse cleanly");
}

/// Real-world scale: 50k packages — bigger than any actual
/// pnpm-lock — verifies the parser stays linear in input size.
#[test]
fn pnpm_lockfile_50000_pkgs_does_not_recursion_limit() {
    let yaml = make_lockfile(50_000);
    // 50k packages is deliberately larger than any real pnpm-lock and exceeds
    // the default `max_events` DoS budget (~1M events) — which the `Value`
    // fast path now enforces in parity with the span-full loader (v0.0.15).
    // Lift the event budget so the test exercises its actual concern (linear
    // scaling / no recursion limit) rather than the DoS budget.
    let cfg = noyalib::ParserConfig::new().max_events(usize::MAX);
    let v: noyalib::Value =
        noyalib::from_str_with_config(&yaml, &cfg).expect("50k packages should parse cleanly");
    match v {
        noyalib::Value::Mapping(m) => assert!(m.contains_key("packages")),
        _ => panic!("expected top-level mapping"),
    }
}

/// Typed-struct target — mirrors the most common shape users
/// migrate to when leaving `serde_yml`.
#[test]
fn pnpm_lockfile_typed_struct_parses_with_default_config() {
    use serde::Deserialize;
    use std::collections::BTreeMap;

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Lockfile {
        #[serde(rename = "lockfileVersion")]
        lockfile_version: String,
        settings: BTreeMap<String, serde_yaml_value::Value>,
        importers: BTreeMap<String, serde_yaml_value::Value>,
        packages: BTreeMap<String, serde_yaml_value::Value>,
        snapshots: BTreeMap<String, serde_yaml_value::Value>,
    }

    // Local re-export so we don't depend on serde-yaml directly.
    mod serde_yaml_value {
        pub(crate) type Value = serde_json::Value;
    }

    let yaml = make_lockfile(2000);
    let _lock: Lockfile = noyalib::from_str(&yaml).expect("default config should parse");
}

/// pnpm-lock keys contain `(peer)(meta)`-style parenthesised
/// suffixes. Some users hit issues with these in flow mappings.
#[test]
fn pnpm_lockfile_with_complex_keys() {
    let mut yaml = String::from("packages:\n");
    for i in 0..500 {
        let _ = writeln!(
            yaml,
            "  '@scope/pkg-{i}@1.0.{i}(peer-a@18.0.0)(peer-b@5.0.0)':"
        );
        let _ = writeln!(yaml, "    resolution: {{integrity: sha512-x}}");
        let _ = writeln!(yaml, "    peerDependencies:");
        let _ = writeln!(yaml, "      peer-a: '>=16'");
        let _ = writeln!(yaml, "      peer-b: '>=4'");
        let _ = writeln!(yaml, "    peerDependenciesMeta:");
        let _ = writeln!(yaml, "      peer-b:");
        let _ = writeln!(yaml, "        optional: true");
    }
    let _v: noyalib::Value =
        noyalib::from_str(&yaml).expect("complex pnpm keys should parse cleanly");
}
