// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Complex serde-attribute interop scenarios.
//!
//! Exercises the combinations users hit in production
//! Kubernetes / IaC / config codebases:
//!
//! - `#[serde(flatten)]` combined with anchors, aliases, and
//!   `Spanned<T>`.
//! - Untagged enums mixed with internally / adjacently tagged
//!   variants — the parser must never get "stuck" or fail to
//!   backtrack across variant probes.

#![allow(missing_docs)]

use noyalib::{Spanned, Value, from_str};
use serde::{Deserialize, Serialize};

// ── Flatten + anchors / aliases ─────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq)]
struct Common {
    image: String,
    version: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Service {
    name: String,
    #[serde(flatten)]
    common: Common,
    replicas: u32,
}

#[test]
fn flatten_with_anchor_alias_expansion() {
    // `<<: *defaults` is YAML's merge-key idiom; combined with a
    // `#[serde(flatten)]` field the merged keys land in the
    // flattened struct.
    let yaml = "
defaults: &defaults
  image: registry.example.com/api
  version: 1.0.0

services:
  - name: api
    <<: *defaults
    replicas: 3
  - name: worker
    <<: *defaults
    replicas: 5
";
    #[derive(Debug, Deserialize, PartialEq)]
    struct Doc {
        services: Vec<Service>,
        #[allow(dead_code)]
        defaults: Common,
    }
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(doc.services.len(), 2);
    assert_eq!(doc.services[0].common.image, "registry.example.com/api");
    assert_eq!(doc.services[0].common.version, "1.0.0");
    assert_eq!(doc.services[0].replicas, 3);
    assert_eq!(doc.services[1].common.image, "registry.example.com/api");
    assert_eq!(doc.services[1].replicas, 5);
}

// ── Flatten + Spanned<T> ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SpannedFlatten {
    #[allow(dead_code)]
    name: Spanned<String>,
    #[serde(flatten)]
    #[allow(dead_code)]
    common: Common,
}

#[test]
fn flatten_pairs_with_spanned_field() {
    let yaml = "
name: api
image: registry.example.com/x
version: 1.0.0
";
    let doc: SpannedFlatten = from_str(yaml).unwrap();
    assert_eq!(doc.name.value, "api");
    assert!(doc.name.start.line() >= 1);
    assert_eq!(doc.common.image, "registry.example.com/x");
}

// ── Multiple flattened fields ───────────────────────────────────────

#[test]
fn multiple_flattened_fields_compose() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Resources {
        cpu: String,
        memory: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Combined {
        name: String,
        #[serde(flatten)]
        common: Common,
        #[serde(flatten)]
        resources: Resources,
    }
    let yaml = "
name: api
image: r/api
version: \"1.0.0\"
cpu: 500m
memory: 256Mi
";
    let doc: Combined = from_str(yaml).unwrap();
    assert_eq!(doc.common.image, "r/api");
    assert_eq!(doc.resources.cpu, "500m");
    assert_eq!(doc.resources.memory, "256Mi");
}

// ── Untagged enum: parser must backtrack across variants ────────────

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum Resource {
    Pod {
        containers: Vec<String>,
    },
    Service {
        ports: Vec<u16>,
    },
    ConfigMap {
        data: std::collections::BTreeMap<String, String>,
    },
}

#[test]
fn untagged_enum_distinguishes_pod_from_service() {
    let pod_yaml = "containers:\n  - api\n  - worker\n";
    let svc_yaml = "ports:\n  - 80\n  - 443\n";
    let cfg_yaml = "data:\n  log_level: info\n  region: us-west\n";

    let pod: Resource = from_str(pod_yaml).unwrap();
    let svc: Resource = from_str(svc_yaml).unwrap();
    let cfg: Resource = from_str(cfg_yaml).unwrap();

    assert!(matches!(pod, Resource::Pod { .. }));
    assert!(matches!(svc, Resource::Service { .. }));
    assert!(matches!(cfg, Resource::ConfigMap { .. }));
}

// ── Internally tagged ───────────────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind")]
enum K8sResource {
    Pod { containers: u32 },
    Service { ports: Vec<u16> },
    Job { parallelism: u32 },
}

#[test]
fn internally_tagged_enum_round_trips() {
    let yaml = "kind: Service\nports:\n  - 80\n  - 443\n";
    let r: K8sResource = from_str(yaml).unwrap();
    assert_eq!(
        r,
        K8sResource::Service {
            ports: vec![80, 443]
        }
    );
}

// ── Adjacently tagged ───────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", content = "spec")]
enum AdjResource {
    Pod { containers: u32 },
    Service { ports: Vec<u16> },
}

#[test]
fn adjacently_tagged_enum_round_trips() {
    let yaml = "kind: Service\nspec:\n  ports:\n    - 8080\n";
    let r: AdjResource = from_str(yaml).unwrap();
    assert_eq!(r, AdjResource::Service { ports: vec![8080] });
}

// ── Mixed enum constellation ────────────────────────────────────────

/// Wrap multiple enum strategies in one document. Each value
/// type uses a different serde-tagging strategy; the parser
/// must handle them all without cross-contamination.
#[test]
fn mixed_enum_strategies_in_one_document() {
    #[derive(Debug, Deserialize)]
    struct MixedDoc {
        // Untagged inner — variant inferred from shape
        worker: Resource,
        // Internally tagged — `kind` field discriminates
        primary: K8sResource,
        // Adjacently tagged — `kind`/`spec` pair
        secondary: AdjResource,
    }
    let yaml = "
worker:
  containers:
    - producer
    - consumer
primary:
  kind: Job
  parallelism: 4
secondary:
  kind: Pod
  spec:
    containers: 2
";
    let doc: MixedDoc = from_str(yaml).unwrap();
    assert!(matches!(doc.worker, Resource::Pod { .. }));
    assert_eq!(doc.primary, K8sResource::Job { parallelism: 4 });
    assert_eq!(doc.secondary, AdjResource::Pod { containers: 2 });
}

// ── Untagged enum with structurally similar variants ────────────────

#[test]
fn untagged_enum_picks_the_first_matching_variant() {
    // Two variants differ only in field names. Serde scans
    // variants top-to-bottom and picks the first that matches
    // — the parser must not get stuck on the failed first
    // probe and abandon the second.
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(untagged)]
    enum Either {
        Left { x: i32, y: i32 },
        Right { lat: f64, lon: f64 },
    }
    let yaml_left = "x: 1\ny: 2\n";
    let yaml_right = "lat: 51.5\nlon: -0.1\n";
    assert_eq!(
        from_str::<Either>(yaml_left).unwrap(),
        Either::Left { x: 1, y: 2 }
    );
    let r: Either = from_str(yaml_right).unwrap();
    if let Either::Right { lat, lon } = r {
        assert!((lat - 51.5).abs() < 1e-6);
        assert!((lon + 0.1).abs() < 1e-6);
    } else {
        panic!("expected Right variant");
    }
}

// ── Flatten + untagged Value escape hatch ──────────────────────────

#[test]
fn flatten_with_value_residue_captures_unknown_keys() {
    // The `extras: Value` field captures any unknown keys, so
    // the parse never fails on the document carrying
    // application-specific metadata.
    #[derive(Debug, Deserialize)]
    struct Envelope {
        name: String,
        version: String,
        #[serde(flatten)]
        extras: std::collections::HashMap<String, Value>,
    }
    let yaml = "
name: noyalib
version: 0.0.1
license: MIT
authors:
  - Sebastien
audit:
  owner: platform
";
    let env: Envelope = from_str(yaml).unwrap();
    assert_eq!(env.name, "noyalib");
    assert_eq!(env.version, "0.0.1");
    assert!(env.extras.contains_key("license"));
    assert!(env.extras.contains_key("authors"));
    assert!(env.extras.contains_key("audit"));
}

// ── Anchor expansion does not corrupt typed deserialise ────────────

#[test]
fn anchor_alias_in_typed_struct() {
    let yaml = "
default_port: &port 8080
services:
  api:
    port: *port
  worker:
    port: *port
";
    use std::collections::BTreeMap;
    #[derive(Debug, Deserialize)]
    struct Doc {
        services: BTreeMap<String, Endpoint>,
        #[allow(dead_code)]
        default_port: u16,
    }
    #[derive(Debug, Deserialize)]
    struct Endpoint {
        port: u16,
    }
    let doc: Doc = from_str(yaml).unwrap();
    assert_eq!(doc.services["api"].port, 8080);
    assert_eq!(doc.services["worker"].port, 8080);
}
