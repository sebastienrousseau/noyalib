// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `Flattened<T>` — capture the raw `Value` alongside a typed
//! deserialisation.
//!
//! `#[serde(flatten)]` is idiomatic for collecting "extra" fields
//! into a residue type, but the built-in residues (`HashMap<String,
//! Value>`, `serde_json::Value`, `noyalib::Value`) erase the typed
//! view. `Flattened<T>` solves that by capturing the underlying
//! `Value` tree first, then re-running typed deserialisation
//! against it.
//!
//! The use-case in this example: a config envelope that carries a
//! typed inner section *and* lets the application layer inspect
//! unknown / metadata fields the schema doesn't declare.
//!
//! Run: `cargo run --example flattened`

#[path = "support.rs"]
mod support;

use noyalib::{Flattened, Value, from_str};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
    tls: bool,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    name: String,
    // Capture both the typed `ServerConfig` view AND the raw
    // mapping so the application layer can still walk metadata
    // fields the schema doesn't declare.
    server: Flattened<ServerConfig>,
}

const YAML: &str = "\
name: prod-api
server:
  host: api.example.com
  port: 8443
  tls: true
  # Fields below are application-specific metadata — Envelope's
  # ServerConfig schema doesn't declare them, but downstream
  # consumers still need to read them.
  region: us-west-2
  ami: ami-0abcd1234
  audit:
    owner: platform-team
    last-rotated: 2026-04-01
";

fn main() -> noyalib::Result<()> {
    support::header("Flattened<T> — typed view + raw metadata capture");

    let cfg: Envelope = from_str(YAML)?;

    // Typed view — the fields ServerConfig declares.
    println!("  Typed view (ServerConfig):");
    println!("    name:  {}", cfg.name);
    println!("    host:  {}", cfg.server.host);
    println!("    port:  {}", cfg.server.port);
    println!("    tls:   {}", cfg.server.tls);

    // Raw view — every key the source supplied, including the
    // metadata fields ServerConfig doesn't declare.
    println!();
    println!("  Raw metadata view (cfg.server.raw):");
    if let Value::Mapping(m) = &cfg.server.raw {
        for (k, v) in m.iter() {
            if matches!(k.as_str(), "host" | "port" | "tls") {
                continue;
            }
            print!("    {k}: ");
            match v {
                Value::Mapping(_) => println!("{{...}}"),
                _ => println!("{v}"),
            }
        }
    }

    println!();
    println!("  Both views from one parse pass — no double-deserialisation.");

    support::footer();
    Ok(())
}
