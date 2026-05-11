// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `!include` directive — compose YAML documents from multiple
//! sources via a user-supplied resolver.
//!
//! Two variants demonstrated:
//!
//! * In-memory resolver (works in any environment, including
//!   `no_std`-style sandboxes).
//! * `SafeFileResolver` against a temporary directory
//!   (`include_fs` feature only).
//!
//! Run: `cargo run --example include_directive --features include_fs`

#[path = "support.rs"]
mod support;

use noyalib::include::{IncludeRequest, IncludeResolver, InputSource, SafeFileResolver};
use noyalib::{from_str_with_config, ParserConfig, Result, Value};
use std::collections::HashMap;

fn mem_resolver(files: HashMap<&'static str, &'static str>) -> IncludeResolver {
    let files: HashMap<String, String> = files
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    IncludeResolver::new(move |req: IncludeRequest<'_>| -> Result<InputSource> {
        let (path, _frag) = noyalib::include::split_fragment(req.spec);
        match files.get(path) {
            Some(b) => Ok(InputSource::new(path, b.clone())),
            None => Err(noyalib::Error::Custom(format!("missing `{path}`"))),
        }
    })
}

fn main() {
    support::header("noyalib -- include_directive");

    support::task_with_output("In-memory resolver: substitute document root", || {
        let mut files = HashMap::new();
        let _ = files.insert("backend.yaml", "host: db.local\nport: 5432\n");
        let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
        let v: Value = from_str_with_config("service: !include backend.yaml\n", &cfg).unwrap();
        vec![
            format!("host = {}", v["service"]["host"].as_str().unwrap()),
            format!("port = {}", v["service"]["port"].as_i64().unwrap()),
        ]
    });

    support::task_with_output("Fragment anchor narrows to a named key", || {
        let mut files = HashMap::new();
        let _ = files.insert(
            "users.yaml",
            "admins:\n  alice: { role: root }\n  bob: { role: root }\n",
        );
        let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
        let v: Value = from_str_with_config("team: !include users.yaml#admins\n", &cfg).unwrap();
        v["team"]
            .as_mapping()
            .unwrap()
            .iter()
            .map(|(k, val)| format!("{k} → {}", val["role"].as_str().unwrap()))
            .collect()
    });

    support::task_with_output("Cycle detection aborts cleanly", || {
        let mut files = HashMap::new();
        let _ = files.insert("a.yaml", "next: !include b.yaml\n");
        let _ = files.insert("b.yaml", "next: !include a.yaml\n");
        let cfg = ParserConfig::new().include_resolver(mem_resolver(files));
        let res: Result<Value> = from_str_with_config("root: !include a.yaml\n", &cfg);
        vec![format!("err = {}", res.unwrap_err().to_string())]
    });

    support::task_with_output("SafeFileResolver sandboxes to a temp dir", || {
        // Stage two files in a temp dir under examples/_temp/.
        let dir = std::env::temp_dir().join("noyalib-include-example");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("greet.yaml"), "msg: hello from temp\n");
        let resolver = SafeFileResolver::new(&dir).into_resolver();
        let cfg = ParserConfig::new().include_resolver(resolver);
        let v: Value = from_str_with_config("greeting: !include greet.yaml\n", &cfg).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        vec![format!(
            "msg = {:?}  (resolved against {})",
            v["greeting"]["msg"].as_str().unwrap(),
            dir.display()
        )]
    });
}
