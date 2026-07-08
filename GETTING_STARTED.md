# Getting started with noyalib

A focused walkthrough for someone who just landed on the repo and
wants to *use* the library. The full reference is the
[root README](./README.md); this page is the on-ramp.

## Install

### As a Rust library

```toml
[dependencies]
noyalib = "0.0.14"
```

`no_std` (alloc-only) and lean profiles are documented in the
[per-crate README](./crates/noyalib/README.md#install).

### As a CLI tool

The `noyafmt` and `noyavalidate` binaries ship through every
mainstream package channel:

```sh
cargo install noya-cli --locked        # crates.io (noyafmt + noyavalidate)
brew tap sebastienrousseau/tap && brew install noyalib   # macOS
yay -S noyalib-bin                     # Arch / AUR
```

The library crate is `noyalib`; the binary crate that produces
`noyafmt` / `noyavalidate` is `noya-cli`. See the
[Install](./README.md#install) section of the root README for the
full per-channel matrix.

Full per-channel install matrix lives in
[`pkg/PUBLISH.md`](./pkg/PUBLISH.md).

## First parse

```rust
use noyalib::{from_str, to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Config {
    name: String,
    port: u16,
    features: Vec<String>,
}

fn main() -> Result<(), noyalib::Error> {
    let yaml = "
name: noyalib
port: 8080
features:
  - parse
  - emit
";
    let cfg: Config = from_str(yaml)?;
    assert_eq!(cfg.port, 8080);

    let back = to_string(&cfg)?;
    assert!(back.contains("name: noyalib"));
    Ok(())
}
```

## What you probably want next

| If you want to… | Read |
|---|---|
| Migrate from `serde_yaml` 0.9 | [doc/MIGRATION-FROM-SERDE-YAML.md](./doc/MIGRATION-FROM-SERDE-YAML.md) |
| Understand the architecture | [doc/ARCHITECTURE.md](./doc/ARCHITECTURE.md) |
| Edit YAML lossy-free (preserving comments and layout) | [crates/noyalib/README.md § CST](./crates/noyalib/README.md) and `noyalib::cst::Document` |
| Validate YAML against a JSON Schema | `noyavalidate --schema schema.yaml input.yaml` |
| Format YAML | `noyafmt --write file.yaml` |
| Use the LSP server in your editor | [crates/noyalib-lsp/README.md](./crates/noyalib-lsp/README.md) |
| Drive noyalib from an AI agent (MCP) | [crates/noyalib-mcp/README.md](./crates/noyalib-mcp/README.md) |
| Run noyalib in the browser | [crates/noyalib-wasm/README.md](./crates/noyalib-wasm/README.md) |
| Look up a domain term | [GLOSSARY.md](./GLOSSARY.md) |
| Contribute code | [CONTRIBUTING.md](./CONTRIBUTING.md) |

## Building from source

```sh
git clone https://github.com/sebastienrousseau/noyalib.git
cd noyalib
make                # cargo check + clippy + test
make fmt            # rustfmt --check
make deny           # cargo-deny supply-chain audit
```

Prerequisites: **Rust 1.75.0+** for the core library, **1.85.0+**
for the CLI / LSP satellite crates (their dep trees pull edition-2024
transitives). `make` orchestrates the full local check suite.

## Running examples

The `crates/noyalib/examples/` directory contains 50+ runnable
examples covering every public surface — schema validation,
borrowed-path parsing, CST editing, no_std, robotics newtypes,
figment integration, and more. Each compiles independently:

```sh
cargo run --example hello
cargo run --example schema_validation --features validate-schema
cargo run --example figment --features figment
```

## Where to ask for help

- **Bug reports / feature requests:** [GitHub Issues](https://github.com/sebastienrousseau/noyalib/issues)
- **Security issues:** see [SECURITY.md](./SECURITY.md) for the disclosure process — do not file public issues for vulnerabilities
- **Migration questions:** [doc/MIGRATION-FROM-SERDE-YAML.md](./doc/MIGRATION-FROM-SERDE-YAML.md) covers the common gotchas
