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

#[cfg(feature = "include_fs")]
mod safe_file {
    use super::*;
    use noyalib::include::{SafeFileResolver, SymlinkPolicy};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("noyalib-include-{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn file_resolver_loads_basic_path() {
        let dir = temp_dir("basic");
        std::fs::write(dir.join("a.yaml"), "hello: world\n").unwrap();
        let cfg = ParserConfig::new().include_resolver(SafeFileResolver::new(&dir).into_resolver());
        let v: Value = from_str_with_config("inc: !include a.yaml\n", &cfg).unwrap();
        assert_eq!(v["inc"]["hello"].as_str(), Some("world"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn path_traversal_outside_root_errors() {
        // Stage a file *inside* root that legitimately resolves;
        // then attempt `..` to escape.
        let dir = temp_dir("traversal");
        std::fs::write(dir.join("ok.yaml"), "k: v\n").unwrap();
        let cfg = ParserConfig::new().include_resolver(SafeFileResolver::new(&dir).into_resolver());
        let res: Result<Value> = from_str_with_config("x: !include ../../etc/hosts\n", &cfg);
        assert!(res.is_err(), "must reject path-traversal");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_errors() {
        let dir = temp_dir("missing");
        let cfg = ParserConfig::new().include_resolver(SafeFileResolver::new(&dir).into_resolver());
        let res: Result<Value> = from_str_with_config("x: !include nope.yaml\n", &cfg);
        assert!(res.is_err(), "missing file must error");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reject_symlink_policy_blocks_symlinks() {
        let dir = temp_dir("symlink");
        std::fs::write(dir.join("real.yaml"), "v: 1\n").unwrap();
        // Best-effort symlink creation — on platforms without
        // privileges to symlink (Windows without dev-mode), skip.
        #[cfg(unix)]
        std::os::unix::fs::symlink(dir.join("real.yaml"), dir.join("link.yaml")).unwrap();
        #[cfg(not(unix))]
        return; // Windows/no-symlink path: nothing to assert.

        #[cfg(unix)]
        {
            let resolver = SafeFileResolver::new(&dir)
                .symlink_policy(SymlinkPolicy::Reject)
                .into_resolver();
            let cfg = ParserConfig::new().include_resolver(resolver);
            let res: Result<Value> = from_str_with_config("x: !include link.yaml\n", &cfg);
            assert!(
                res.is_err(),
                "SymlinkPolicy::Reject must block symlinked includes"
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn symlink_policy_default_is_follow_within_root() {
        assert_eq!(SymlinkPolicy::default(), SymlinkPolicy::FollowWithinRoot);
    }

    #[test]
    fn debug_impl_renders() {
        let r = SafeFileResolver::new("/srv/configs");
        let s = format!("{r:?}");
        assert!(s.contains("SafeFileResolver"));
    }

    #[test]
    fn split_fragment_round_trip() {
        use noyalib::include::split_fragment;
        assert_eq!(split_fragment("a.yaml#anchor"), ("a.yaml", Some("anchor")));
        assert_eq!(split_fragment("a.yaml"), ("a.yaml", None));
        assert_eq!(split_fragment(""), ("", None));
        assert_eq!(split_fragment("#anchor"), ("", Some("anchor")));
    }
}

#[test]
fn fragment_on_non_mapping_document_errors() {
    let mut files = HashMap::new();
    let _ = files.insert("scalar.yaml", "42\n");
    let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
    let res: Result<Value> = from_str_with_config("v: !include scalar.yaml#k\n", &cfg);
    let err = res.unwrap_err();
    assert!(err.to_string().contains("mapping"), "{err}");
}

#[test]
fn include_inside_sequence_is_resolved() {
    let mut files = HashMap::new();
    let _ = files.insert("item.yaml", "name: alpha\n");
    let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
    let yaml = "items:\n  - !include item.yaml\n  - !include item.yaml\n";
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    let seq = v["items"].as_sequence().unwrap();
    assert_eq!(seq.len(), 2);
    assert_eq!(seq[0]["name"].as_str(), Some("alpha"));
}

#[test]
fn non_include_tagged_values_pass_through() {
    let resolver = IncludeResolver::new(|_req: IncludeRequest<'_>| -> Result<InputSource> {
        unreachable!("non-!include tag must not invoke the resolver")
    });
    let cfg = ParserConfig::new().include_resolver(resolver);
    let yaml = "v: !custom 42\n";
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    let tag = v["v"].as_tagged().unwrap();
    assert_eq!(tag.tag().as_str(), "!custom");
}

#[test]
fn input_source_constructor_and_clone() {
    let src = InputSource::new("test.yaml", "k: v\n");
    assert_eq!(src.name, "test.yaml");
    assert_eq!(src.bytes, "k: v\n");
    let cloned = src.clone();
    assert_eq!(cloned.name, src.name);
}

#[test]
fn include_request_debug_renders_via_resolver_invocation() {
    use std::sync::Mutex;
    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let captured_clone = Arc::clone(&captured);
    let resolver = IncludeResolver::new(move |req: IncludeRequest<'_>| -> Result<InputSource> {
        *captured_clone.lock().unwrap() = Some(format!("{req:?}"));
        Ok(InputSource::new(req.spec, "ok: 1\n"))
    });
    let cfg = ParserConfig::new().include_resolver(resolver);
    let _: Value = from_str_with_config("x: !include some_spec.yaml\n", &cfg).unwrap();
    let dbg = captured.lock().unwrap().clone().unwrap();
    assert!(dbg.contains("IncludeRequest"));
    assert!(dbg.contains("some_spec.yaml"));
}

#[test]
fn resolver_debug_renders() {
    let r = IncludeResolver::new(|_| Ok(InputSource::new("n", "v: 1\n")));
    let s = format!("{r:?}");
    assert!(s.contains("IncludeResolver"));
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
