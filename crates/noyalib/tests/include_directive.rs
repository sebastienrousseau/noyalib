// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `!include` directive — post-parse resolution + cycle / depth /
//! sandbox guards.

#![cfg(feature = "include")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::include::{IncludeRequest, IncludeResolver, InputSource};
use noyalib::{from_str_with_config, ParserConfig, Result, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Build a resolver backed by an in-memory map.
fn mem_resolver(files: HashMap<&'static str, &'static str>) -> IncludeResolver {
    let files: HashMap<String, String> = files
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    IncludeResolver::new(move |req: IncludeRequest<'_>| -> Result<InputSource> {
        let (path, _frag) = noyalib::include::split_fragment(req.spec);
        match files.get(path) {
            Some(b) => Ok(InputSource::new(path, b.clone())),
            None => Err(noyalib::Error::Custom(format!(
                "test mem resolver: missing `{path}`"
            ))),
        }
    })
}

#[test]
fn basic_include_substitutes_document_root() {
    let mut files = HashMap::new();
    let _ = files.insert("frag.yaml", "name: alpha\nversion: 1\n");
    let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
    let yaml = "service: !include frag.yaml\n";
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(v["service"]["name"].as_str(), Some("alpha"));
    assert_eq!(v["service"]["version"].as_i64(), Some(1));
}

#[test]
fn nested_include_resolves_recursively() {
    let mut files = HashMap::new();
    let _ = files.insert("inner.yaml", "v: 99\n");
    let _ = files.insert("outer.yaml", "inner: !include inner.yaml\n");
    let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
    let yaml = "wrap: !include outer.yaml\n";
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(v["wrap"]["inner"]["v"].as_i64(), Some(99));
}

#[test]
fn fragment_anchor_narrows_to_named_key() {
    let mut files = HashMap::new();
    let _ = files.insert(
        "defs.yaml",
        "users:\n  admin: { role: root }\n  guest: { role: anon }\n",
    );
    let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
    let yaml = "u: !include defs.yaml#users\n";
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(v["u"]["admin"]["role"].as_str(), Some("root"));
    assert_eq!(v["u"]["guest"]["role"].as_str(), Some("anon"));
}

#[test]
fn fragment_anchor_missing_key_errors_clearly() {
    let mut files = HashMap::new();
    let _ = files.insert("defs.yaml", "users:\n  admin: 1\n");
    let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
    let yaml = "u: !include defs.yaml#missing\n";
    let res: Result<Value> = from_str_with_config(yaml, &cfg);
    let err = res.unwrap_err();
    assert!(err.to_string().contains("fragment"), "{err}");
    assert!(err.to_string().contains("missing"), "{err}");
}

#[test]
fn cycle_detection_aborts_with_clear_error() {
    let mut files = HashMap::new();
    let _ = files.insert("a.yaml", "next: !include b.yaml\n");
    let _ = files.insert("b.yaml", "next: !include a.yaml\n");
    let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
    let yaml = "root: !include a.yaml\n";
    let res: Result<Value> = from_str_with_config(yaml, &cfg);
    let err = res.unwrap_err();
    assert!(err.to_string().contains("cycle"), "{err}");
}

#[test]
fn max_include_depth_caps_recursion() {
    // resolver always returns another !include — guaranteed
    // depth blow-up unless capped.
    let resolver = IncludeResolver::new(|_req: IncludeRequest<'_>| -> Result<InputSource> {
        Ok(InputSource::new("infinite", "deeper: !include infinite\n"))
    });
    let cfg = ParserConfig::new()
        .include_resolver(resolver)
        .max_include_depth(5);
    let yaml = "root: !include start\n";
    let res: Result<Value> = from_str_with_config(yaml, &cfg);
    assert!(res.is_err(), "max-depth must abort: {res:?}");
}

#[test]
fn no_resolver_set_means_no_walk() {
    // Without a resolver installed, the !include node stays as
    // a Tagged value in the output — the user can still inspect
    // it but no substitution happens.
    let cfg = ParserConfig::new();
    let yaml = "left_alone: !include frag.yaml\n";
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    let tag_str = v["left_alone"].as_tagged().map(|t| t.tag().as_str());
    assert_eq!(tag_str, Some("!include"));
}

#[test]
fn resolver_errors_propagate() {
    let resolver = IncludeResolver::new(|_req: IncludeRequest<'_>| -> Result<InputSource> {
        Err(noyalib::Error::Custom("synthetic resolver failure".into()))
    });
    let cfg = ParserConfig::new().include_resolver(resolver);
    let yaml = "v: !include anything\n";
    let res: Result<Value> = from_str_with_config(yaml, &cfg);
    let err = res.unwrap_err();
    assert!(err.to_string().contains("synthetic"), "{err}");
}

#[test]
fn non_string_spec_errors() {
    // `!include {x: 1}` — the spec must be a scalar string, not
    // a mapping. The walker should refuse instead of trying to
    // resolve a mapping-as-path.
    let resolver = IncludeResolver::new(|_req: IncludeRequest<'_>| -> Result<InputSource> {
        Ok(InputSource::new("noop", "k: v\n"))
    });
    let cfg = ParserConfig::new().include_resolver(resolver);
    let yaml = "bad: !include\n  not: a-scalar\n";
    let res: Result<Value> = from_str_with_config(yaml, &cfg);
    assert!(res.is_err());
}

#[test]
fn typed_target_sees_substituted_value() {
    use serde::Deserialize;
    #[derive(Debug, Deserialize)]
    struct Server {
        host: String,
        port: u16,
    }
    let mut files = HashMap::new();
    let _ = files.insert("server.yaml", "host: db.local\nport: 5432\n");
    let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
    let yaml = "server: !include server.yaml\n";
    #[derive(Debug, Deserialize)]
    struct Root {
        server: Server,
    }
    let root: Root = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(root.server.host, "db.local");
    assert_eq!(root.server.port, 5432);
}

#[test]
fn resolver_observes_increasing_depth() {
    // Track the depth value the resolver sees on each call. The
    // outer document is depth 0; nested includes are depth 1, 2…
    let depths: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let depths_clone = Arc::clone(&depths);
    let resolver = IncludeResolver::new(move |req: IncludeRequest<'_>| -> Result<InputSource> {
        depths_clone.lock().unwrap().push(req.depth);
        match req.spec {
            "a.yaml" => Ok(InputSource::new("a", "next: !include b.yaml\n")),
            "b.yaml" => Ok(InputSource::new("b", "leaf: 7\n")),
            _ => unreachable!(),
        }
    });
    let cfg = ParserConfig::new().include_resolver(resolver);
    let v: Value = from_str_with_config("r: !include a.yaml\n", &cfg).unwrap();
    assert_eq!(v["r"]["next"]["leaf"].as_i64(), Some(7));
    let observed = depths.lock().unwrap().clone();
    assert_eq!(observed, vec![0, 1]);
}
