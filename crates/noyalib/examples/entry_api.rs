// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Entry proxy API — surgical, lossless edits on a complex YAML
//! document.
//!
//! The CST `Document::entry` accessor returns a `path`-shaped
//! handle that mirrors `std::collections::HashMap::Entry`. You can
//! `.or_insert(...)`, `.and_modify(...)`, and chain edits without
//! touching neighbouring keys, comments, or whitespace.
//!
//! This example walks through a Kubernetes-style deployment
//! manifest and patches it surgically. Every comment, indent, and
//! sibling entry round-trips byte-faithfully.
//!
//! Run: `cargo run --example entry_api`

#[path = "support.rs"]
mod support;

use noyalib::Value;
use noyalib::cst::parse_document;

const MANIFEST: &str = "\
# Production deployment for the api service.
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api
  labels:
    app: api
    tier: backend
spec:
  replicas: 3              # scale this for the prod tier
  selector:
    matchLabels:
      app: api
  template:
    metadata:
      labels:
        app: api
    spec:
      containers:
        - name: api
          image: registry.example.com/api:1.4.2
          ports:
            - containerPort: 8080
          # Resource limits set conservatively. Bump
          # for the prod tier when traffic ramps.
          resources:
            limits:
              cpu: \"500m\"
              memory: \"512Mi\"
";

fn main() -> noyalib::Result<()> {
    support::header("Entry API — surgical Kubernetes manifest patching");

    let mut doc = parse_document(MANIFEST)?;

    // 1. Bump the replica count for the prod tier.
    doc.entry("spec")
        .insert_value("replicas", &Value::Number(noyalib::Number::Integer(10)))?;

    // 2. Adjust the container image tag.
    doc.set(
        "spec.template.spec.containers[0].image",
        "registry.example.com/api:1.5.0",
    )?;

    // 3. Loosen the CPU / memory resource limits.
    let resources_path = "spec.template.spec.containers[0].resources.limits";
    doc.entry(resources_path)
        .insert_value("cpu", &Value::String("1500m".into()))?;
    doc.entry(resources_path)
        .insert_value("memory", &Value::String("1Gi".into()))?;

    // 4. Add a `tier` label that the original didn't have.
    doc.entry("spec.template.metadata.labels")
        .insert_value("tier", &Value::String("backend".into()))?;

    println!("{}", doc);

    println!();
    println!("  Comments and indentation preserved byte-for-byte.");
    println!("  Only the four targeted spans were rewritten.");

    support::footer();
    Ok(())
}
