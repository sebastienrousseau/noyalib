// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Surgical YAML patching: update single values in large documents.
//!
//! Demonstrates strategic merge patching — modify specific paths without
//! touching the rest of the document. Essential for IaC workflows.
//!
//! Run: `cargo run --example patch`

#[path = "support.rs"]
mod support;

use noyalib::{Mapping, Value, from_str, to_string};

/// Apply a patch at a dotted path. Creates intermediate mappings as needed.
fn patch_at(value: &mut Value, path: &str, new_value: Value) -> bool {
    let segments: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for (i, &seg) in segments.iter().enumerate() {
        if i == segments.len() - 1 {
            if let Some(map) = current.as_mapping_mut() {
                let _ = map.insert(seg.to_string(), new_value);
                return true;
            }
            return false;
        }
        if current.get(seg).is_none() {
            if let Some(map) = current.as_mapping_mut() {
                let _ = map.insert(seg.to_string(), Value::Mapping(Mapping::new()));
            }
        }
        match current.get_mut(seg) {
            Some(next) => current = next,
            None => return false,
        }
    }
    false
}

/// Remove a value at a dotted path.
fn remove_at(value: &mut Value, path: &str) -> bool {
    let segments: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for (i, &seg) in segments.iter().enumerate() {
        if i == segments.len() - 1 {
            if let Some(map) = current.as_mapping_mut() {
                return map.remove(seg).is_some();
            }
            return false;
        }
        match current.get_mut(seg) {
            Some(next) => current = next,
            None => return false,
        }
    }
    false
}

fn main() {
    support::header("noyalib -- patch");

    let manifest = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app
  labels:
    app: my-app
    version: "1.0"
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: app
          image: my-app:1.0
          resources:
            limits:
              cpu: "500m"
              memory: "128Mi"
"#;

    let mut doc: Value = from_str(manifest).unwrap();

    // ── Surgical update ──────────────────────────────────────────────
    support::task_with_output("Patch: update container image", || {
        let before = doc
            .get_path("spec.template.spec.containers[0].image")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        let _ = patch_at(
            &mut doc,
            "spec.template.spec",
            from_str::<Value>("containers:\n  - name: app\n    image: my-app:2.0\n    resources:\n      limits:\n        cpu: \"500m\"\n        memory: \"128Mi\"\n").unwrap(),
        );
        let after = doc
            .get_path("spec.template.spec.containers[0].image")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        vec![format!("before = {before}"), format!("after  = {after}")]
    });

    // Reload for clean state
    let mut doc: Value = from_str(manifest).unwrap();

    // ── Scale replicas ───────────────────────────────────────────────
    support::task_with_output("Patch: scale replicas", || {
        let before = doc
            .get_path("spec.replicas")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let _ = patch_at(&mut doc, "spec.replicas", Value::from(5));
        let after = doc
            .get_path("spec.replicas")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        vec![format!("before = {before}"), format!("after  = {after}")]
    });

    // ── Add new field ────────────────────────────────────────────────
    support::task_with_output("Patch: add annotation", || {
        let _ = patch_at(&mut doc, "metadata.annotations", {
            let mut m = Mapping::new();
            let _ = m.insert("deployed-by", Value::String("ci/cd".to_string()));
            let _ = m.insert(
                "deploy-time",
                Value::String("2026-04-19T12:00:00Z".to_string()),
            );
            Value::Mapping(m)
        });
        let has = doc.get_path("metadata.annotations.deployed-by").is_some();
        vec![format!("annotations added = {has}")]
    });

    // ── Remove field ─────────────────────────────────────────────────
    support::task_with_output("Patch: remove label", || {
        let before = doc.get_path("metadata.labels.version").is_some();
        let _ = remove_at(&mut doc, "metadata.labels.version");
        let after = doc.get_path("metadata.labels.version").is_some();
        vec![
            format!("version label before = {before}"),
            format!("version label after  = {after}"),
        ]
    });

    // ── Final document ───────────────────────────────────────────────
    support::task_with_output("Patched document", || {
        let yaml = to_string(&doc).unwrap();
        yaml.lines().map(|l| l.to_string()).collect()
    });

    support::summary(5);
}
