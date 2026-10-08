// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `!include` directive — post-parse resolution + cycle / depth /
//! sandbox guards.

#![cfg(feature = "include")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::include::{IncludeRequest, IncludeResolver, InputSource};
use noyalib::{ParserConfig, Result, Value, from_str_with_config};
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
fn cycle_detection_uses_the_resolvers_canonical_identity() {
    let resolver = IncludeResolver::new(|req: IncludeRequest<'_>| -> Result<InputSource> {
        let bytes = match req.spec {
            "alias-a.yaml" => "next: !include alias-b.yaml\n",
            "alias-b.yaml" => "next: !include alias-a.yaml\n",
            _ => unreachable!(),
        };
        Ok(InputSource::new("/canonical/shared.yaml", bytes))
    });
    let cfg = ParserConfig::new().include_resolver(resolver);

    let error = from_str_with_config::<Value>("root: !include alias-a.yaml\n", &cfg).unwrap_err();
    assert!(error.to_string().contains("cycle"), "{error}");
    assert!(
        error.to_string().contains("/canonical/shared.yaml"),
        "{error}"
    );
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
fn include_source_count_is_bounded_across_siblings() {
    let mut files = HashMap::new();
    let _ = files.insert("a.yaml", "a: 1\n");
    let _ = files.insert("b.yaml", "b: 2\n");
    let cfg = ParserConfig::new()
        .include_resolver(mem_resolver(files))
        .max_include_sources(1);

    let error =
        from_str_with_config::<Value>("first: !include a.yaml\nsecond: !include b.yaml\n", &cfg)
            .unwrap_err();
    assert!(matches!(
        error,
        noyalib::Error::Budget(noyalib::BudgetBreach::MaxIncludeSources {
            limit: 1,
            observed: 2
        })
    ));
}

#[test]
fn cumulative_include_bytes_are_bounded() {
    let mut files = HashMap::new();
    let _ = files.insert("a.yaml", "value: 12345\n");
    let cfg = ParserConfig::new()
        .include_resolver(mem_resolver(files))
        .max_total_include_bytes(4);

    let error = from_str_with_config::<Value>("root: !include a.yaml\n", &cfg).unwrap_err();
    assert!(matches!(
        error,
        noyalib::Error::Budget(noyalib::BudgetBreach::MaxIncludeBytes { limit: 4, .. })
    ));
}

#[test]
fn expanded_include_nodes_share_the_document_budget() {
    let mut files = HashMap::new();
    let _ = files.insert("a.yaml", "value: 1\n");
    let _ = files.insert("b.yaml", "value: 2\n");
    let cfg = ParserConfig::new()
        .include_resolver(mem_resolver(files))
        // The root has five authored nodes and each included document has
        // three. All sources fit independently, but the expanded root has
        // nine nodes: its mapping, two keys, and two three-node mappings.
        .max_nodes(8);

    let error =
        from_str_with_config::<Value>("first: !include a.yaml\nsecond: !include b.yaml\n", &cfg)
            .unwrap_err();
    assert!(matches!(
        error,
        noyalib::Error::Budget(noyalib::BudgetBreach::MaxNodes {
            limit: 8,
            observed: 9
        })
    ));
}

#[test]
fn expanded_include_nodes_accept_the_exact_budget() {
    let mut files = HashMap::new();
    let _ = files.insert("a.yaml", "value: 1\n");
    let _ = files.insert("b.yaml", "value: 2\n");
    let cfg = ParserConfig::new()
        .include_resolver(mem_resolver(files))
        .max_nodes(9);

    let value =
        from_str_with_config::<Value>("first: !include a.yaml\nsecond: !include b.yaml\n", &cfg)
            .unwrap();
    assert_eq!(value["first"]["value"].as_i64(), Some(1));
    assert_eq!(value["second"]["value"].as_i64(), Some(2));
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
    #[derive(Debug, serde::Deserialize)]
    struct Server {
        host: String,
        port: u16,
    }
    let mut files = HashMap::new();
    let _ = files.insert("server.yaml", "host: db.local\nport: 5432\n");
    let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
    let yaml = "server: !include server.yaml\n";
    #[derive(Debug, serde::Deserialize)]
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

    #[test]
    fn resolver_with_nonexistent_root_errors() {
        let dir = temp_dir("nonexistent");
        let _ = std::fs::remove_dir_all(&dir);
        let resolver = SafeFileResolver::new(&dir).into_resolver();
        let cfg = ParserConfig::new().include_resolver(resolver);
        let res: Result<Value> = from_str_with_config("x: !include a.yaml\n", &cfg);
        assert!(res.is_err(), "non-existent root must error");
        let msg = res.unwrap_err().to_string();
        assert!(
            msg.contains("cannot open root"),
            "expected root capability error, got: {msg}"
        );
    }

    #[test]
    fn reject_symlink_with_missing_file() {
        let dir = temp_dir("rej-missing");
        let resolver = SafeFileResolver::new(&dir)
            .symlink_policy(SymlinkPolicy::Reject)
            .into_resolver();
        let cfg = ParserConfig::new().include_resolver(resolver);
        let res: Result<Value> = from_str_with_config("x: !include nope.yaml\n", &cfg);
        assert!(res.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolver_reads_utf8_content() {
        let dir = temp_dir("utf8");
        std::fs::write(dir.join("multi.yaml"), "msg: 日本語の値\n").unwrap();
        let cfg = ParserConfig::new().include_resolver(SafeFileResolver::new(&dir).into_resolver());
        let v: Value = from_str_with_config("inc: !include multi.yaml\n", &cfg).unwrap();
        assert_eq!(v["inc"]["msg"].as_str(), Some("日本語の値"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escaping_sandbox_is_rejected() {
        // Symlink pointing outside the sandbox should be caught
        // by the post-canonicalisation root-prefix check (under
        // the default FollowWithinRoot policy).
        let dir = temp_dir("escape");
        // Stage an outside file.
        let outside = std::env::temp_dir().join("noyalib-include-escape-outside.yaml");
        std::fs::write(&outside, "secret: outside\n").unwrap();
        // Symlink from inside dir → outside file.
        std::os::unix::fs::symlink(&outside, dir.join("link.yaml")).unwrap();
        let resolver = SafeFileResolver::new(&dir).into_resolver();
        let cfg = ParserConfig::new().include_resolver(resolver);
        let res: Result<Value> = from_str_with_config("x: !include link.yaml\n", &cfg);
        assert!(res.is_err(), "symlink target outside root must be rejected");
        let msg = res.unwrap_err().to_string();
        assert!(msg.contains("escapes"), "{msg}");
        let _ = std::fs::remove_file(&outside);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn reject_policy_blocks_symlinked_parent_directory() {
        let dir = temp_dir("nested-symlink");
        std::fs::create_dir_all(dir.join("real")).unwrap();
        std::fs::write(dir.join("real/value.yaml"), "safe: true\n").unwrap();
        std::os::unix::fs::symlink("real", dir.join("linked")).unwrap();

        let resolver = SafeFileResolver::new(&dir)
            .symlink_policy(SymlinkPolicy::Reject)
            .into_resolver();
        let cfg = ParserConfig::new().include_resolver(resolver);
        let res: Result<Value> = from_str_with_config("x: !include linked/value.yaml\n", &cfg);
        assert!(res.is_err(), "parent-directory symlinks must be rejected");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn follow_policy_accepts_symlinked_parent_within_root() {
        let dir = temp_dir("nested-symlink-follow");
        std::fs::create_dir_all(dir.join("real")).unwrap();
        std::fs::write(dir.join("real/value.yaml"), "safe: true\n").unwrap();
        std::os::unix::fs::symlink("real", dir.join("linked")).unwrap();

        let cfg = ParserConfig::new().include_resolver(SafeFileResolver::new(&dir).into_resolver());
        let value: Value = from_str_with_config("x: !include linked/value.yaml\n", &cfg).unwrap();
        assert_eq!(value["x"]["safe"].as_bool(), Some(true));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn opened_root_cannot_be_redirected_by_path_replacement() {
        let dir = temp_dir("root-replacement");
        let moved = dir.with_extension("opened");
        let _ = std::fs::remove_dir_all(&moved);
        std::fs::write(dir.join("value.yaml"), "source: original\n").unwrap();

        let cfg = ParserConfig::new().include_resolver(SafeFileResolver::new(&dir).into_resolver());
        std::fs::rename(&dir, &moved).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("value.yaml"), "source: replacement\n").unwrap();

        let value: Value = from_str_with_config("x: !include value.yaml\n", &cfg).unwrap();
        assert_eq!(value["x"]["source"].as_str(), Some("original"));

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&moved);
    }
}

#[test]
fn multi_document_include_is_rejected() {
    // An included file holding several documents is refused, not
    // silently truncated to its first — the same single-document
    // policy `from_str` follows (#351). Also what made the include
    // path `no_std`-clean: resolution now parses through the
    // every-target `parse_exactly_one_value` instead of the
    // `std`-only unchecked form.
    let mut files = HashMap::new();
    let _ = files.insert("multi.yaml", "a: 1\n---\nb: 2\n");
    let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
    let res: Result<Value> = from_str_with_config("x: !include multi.yaml\n", &cfg);
    assert!(res.is_err(), "multi-document include must be rejected");
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

/// Resolver that hands out `doc0`, `doc1`, ... each nesting `per`
/// flow sequences around an include of the next one.
fn nested_chain_resolver(levels: usize, per: usize) -> IncludeResolver {
    IncludeResolver::new(move |req: IncludeRequest<'_>| -> Result<InputSource> {
        let i: usize = req.spec.trim_start_matches("doc").parse().unwrap();
        let inner = if i + 1 < levels {
            format!("!include doc{}", i + 1)
        } else {
            "leaf".to_string()
        };
        let text = format!("{}{}{}\n", "[".repeat(per), inner, "]".repeat(per));
        Ok(InputSource::new(req.spec, text))
    })
}

#[test]
fn nesting_depth_is_charged_across_include_levels() {
    // Three levels of six nested sequences make an 18-deep tree;
    // `max_depth(10)` must refuse it even though each source alone
    // is only 6 deep.
    let cfg = ParserConfig::new()
        .max_depth(10)
        .include_resolver(nested_chain_resolver(3, 6));
    let res: Result<Value> = from_str_with_config("!include doc0\n", &cfg);
    assert!(
        matches!(res, Err(noyalib::Error::RecursionLimitExceeded { .. })),
        "cumulative depth must be refused: {res:?}"
    );
    // Three levels of three (9 deep) fit under the same limit.
    let cfg = ParserConfig::new()
        .max_depth(10)
        .include_resolver(nested_chain_resolver(3, 3));
    let v: Value = from_str_with_config("!include doc0\n", &cfg).unwrap();
    assert!(v.is_sequence());
}

#[test]
fn nesting_depth_counts_the_including_documents_position() {
    // The include sits four collections deep, the included source
    // adds seven more: eleven exceeds ten.
    let cfg = ParserConfig::new()
        .max_depth(10)
        .include_resolver(nested_chain_resolver(1, 7));
    let res: Result<Value> = from_str_with_config("a: {b: {c: [!include doc0]}}\n", &cfg);
    assert!(
        res.is_err(),
        "position plus included depth must count: {res:?}"
    );
}

#[test]
fn deep_include_chain_under_defaults_does_not_exhaust_the_stack() {
    // 24 include levels of 120 nested sequences each, under default
    // limits, on a 2 MiB thread stack. Unbounded, this builds a
    // 2,880-deep tree and overflows the stack walking it.
    let handle = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let cfg = ParserConfig::new().include_resolver(nested_chain_resolver(24, 120));
            let res: Result<Value> = from_str_with_config("!include doc0\n", &cfg);
            res.is_err()
        })
        .unwrap();
    assert!(handle.join().unwrap(), "the deep chain must be refused");
}

#[test]
fn max_document_length_applies_to_each_included_source() {
    let body: &'static str = Box::leak(format!("k: {}\n", "x".repeat(200)).into_boxed_str());
    let mut files = HashMap::new();
    let _ = files.insert("big.yaml", body);
    let cfg = ParserConfig::new()
        .max_document_length(100)
        .include_resolver(mem_resolver(files));
    let res: Result<Value> = from_str_with_config("a: !include big.yaml\n", &cfg);
    let err = res.expect_err("an included source over max_document_length must be refused");
    assert!(err.to_string().contains("maximum length"), "got: {err}");
}

#[test]
fn resolver_is_told_the_remaining_byte_budget() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen2 = Arc::clone(&seen);
    let resolver = IncludeResolver::new(move |req: IncludeRequest<'_>| -> Result<InputSource> {
        seen2.lock().unwrap().push(req.max_bytes);
        Ok(InputSource::new(req.spec, "0123456789\n"))
    });
    let cfg = ParserConfig::new()
        .max_total_include_bytes(1_000)
        .max_document_length(500)
        .include_resolver(resolver);
    let _: Value = from_str_with_config("a: !include x\nb: !include y\n", &cfg).unwrap();
    let seen = seen.lock().unwrap();
    // The first request is capped by max_document_length, the second
    // still is: 1,000 - 11 bytes remain, more than 500.
    assert_eq!(*seen, vec![500, 500]);
}

#[cfg(feature = "include_fs")]
mod safe_file_budgets {
    use super::*;
    use noyalib::include::SafeFileResolver;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("noyalib-include-budget-{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn oversized_file_is_not_read_past_the_budget() {
        let dir = temp_dir("oversized");
        let f = std::fs::File::create(dir.join("huge.yaml")).unwrap();
        f.set_len(64 * 1024 * 1024).unwrap();
        let cfg = ParserConfig::new()
            .max_total_include_bytes(1024 * 1024)
            .include_resolver(SafeFileResolver::new(&dir).into_resolver());
        let res: Result<Value> = from_str_with_config("k: !include huge.yaml\n", &cfg);
        let _ = std::fs::remove_dir_all(&dir);
        match res {
            Err(noyalib::Error::Budget(noyalib::BudgetBreach::MaxIncludeBytes {
                limit,
                observed,
            })) => assert!(
                observed <= limit + 1,
                "read {observed} bytes against a {limit}-byte budget"
            ),
            other => panic!("expected the include byte budget to trip: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn fifo_is_refused_without_blocking() {
        let dir = temp_dir("fifo");
        let pipe = dir.join("pipe.yaml");
        let status = std::process::Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .unwrap();
        assert!(status.success());
        let (tx, rx) = std::sync::mpsc::channel();
        let root = dir.clone();
        let _ = std::thread::spawn(move || {
            let cfg =
                ParserConfig::new().include_resolver(SafeFileResolver::new(&root).into_resolver());
            let res: Result<Value> = from_str_with_config("k: !include pipe.yaml\n", &cfg);
            let _ = tx.send(res.map(|_| ()).map_err(|e| e.to_string()));
        });
        let outcome = rx.recv_timeout(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_dir_all(&dir);
        let msg = outcome
            .expect("including a FIFO must not block")
            .expect_err("a FIFO is not a regular file");
        assert!(msg.contains("regular file"), "got: {msg}");
    }

    #[test]
    fn directory_is_refused() {
        let dir = temp_dir("dir");
        std::fs::create_dir_all(dir.join("sub.yaml")).unwrap();
        let cfg = ParserConfig::new().include_resolver(SafeFileResolver::new(&dir).into_resolver());
        let res: Result<Value> = from_str_with_config("k: !include sub.yaml\n", &cfg);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(res.is_err(), "a directory must be refused");
    }

    #[test]
    fn errors_name_paths_relative_to_the_root() {
        let dir = temp_dir("relative-errors");
        let canonical = std::fs::canonicalize(&dir).unwrap();
        let cfg = ParserConfig::new().include_resolver(SafeFileResolver::new(&dir).into_resolver());
        let res: Result<Value> = from_str_with_config("k: !include sub/missing.yaml\n", &cfg);
        let _ = std::fs::remove_dir_all(&dir);
        let msg = res.expect_err("a missing file must error").to_string();
        assert!(msg.contains("sub/missing.yaml"), "got: {msg}");
        for absolute in [dir.display().to_string(), canonical.display().to_string()] {
            assert!(!msg.contains(&absolute), "absolute path leaked: {msg}");
        }
    }

    #[test]
    fn source_names_are_relative_to_the_root() {
        // The cycle error quotes the source's identity, which is the
        // `InputSource::name` the resolver returned.
        let dir = temp_dir("relative-names");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/a.yaml"), "k: !include sub/a.yaml\n").unwrap();
        let cfg = ParserConfig::new().include_resolver(SafeFileResolver::new(&dir).into_resolver());
        let canonical = std::fs::canonicalize(&dir).unwrap();
        let res: Result<Value> = from_str_with_config("k: !include sub/a.yaml\n", &cfg);
        let _ = std::fs::remove_dir_all(&dir);
        let msg = res.expect_err("a self-include must error").to_string();
        assert!(msg.contains("cycle"), "got: {msg}");
        assert!(msg.contains("`sub/a.yaml`"), "got: {msg}");
        assert!(
            !msg.contains(&canonical.display().to_string()),
            "absolute path leaked: {msg}"
        );
    }
}
