// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Build script: generates man pages and shell completions during
//! `cargo build` so distro packagers (rpmbuild / dpkg-buildpackage)
//! get the artefacts without needing a separate `cargo xtask`
//! invocation.
//!
//! Off by default — enabled by setting `NOYA_GEN_ASSETS=1`.
//! Generation is deterministic (no clock reads, no env reads beyond
//! the trigger var), so the produced files are bit-reproducible.
//!
//! The same Command tree the binaries parse against — defined in
//! `src/lib.rs` — is reused here via `#[path]` inclusion, keeping
//! `build-dependencies` minimal: just `clap`, `clap_complete`, and
//! `clap_mangen`.
//!
//! Output layout under `OUT_DIR`:
//!
//!   <OUT_DIR>/
//!     noyafmt.1
//!     noyavalidate.1
//!     noyafmt.bash, noyafmt.fish, _noyafmt, _noyafmt.ps1
//!     noyavalidate.bash, noyavalidate.fish, _noyavalidate, _noyavalidate.ps1
//!
//! `ci/extract-assets.sh` finds and copies these into a stable
//! location for downstream packaging.

use std::path::{Path, PathBuf};

// Reuse the lib's clap Command builders without making the lib a
// build-dep of itself (which Cargo forbids). The `#[path]` include
// pulls the source in directly; build.rs and the lib both end up
// with their own copy of the types, but the *behaviour* is shared.
#[path = "src/lib.rs"]
mod codegen_source;

fn main() {
    println!("cargo:rerun-if-env-changed=NOYA_GEN_ASSETS");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("NOYA_GEN_ASSETS").as_deref() != Ok("1") {
        return;
    }

    let out_dir: PathBuf = std::env::var_os("OUT_DIR")
        .expect("OUT_DIR is set by Cargo for build scripts")
        .into();

    let cmds = [
        ("noyafmt", codegen_source::noyafmt_command()),
        ("noyavalidate", codegen_source::noyavalidate_command()),
    ];

    for (name, cmd) in cmds {
        gen_completions(&out_dir, name, &mut cmd.clone());
        gen_manpage(&out_dir, name, cmd);
    }
}

/// Write bash / fish / zsh / powershell completion files under
/// `<OUT_DIR>/{name}.{shell-ext}` using `clap_complete`.
fn gen_completions(out_dir: &Path, name: &str, cmd: &mut clap::Command) {
    use clap_complete::{Shell, generate_to};
    for shell in [Shell::Bash, Shell::Fish, Shell::Zsh, Shell::PowerShell] {
        let _ = generate_to(shell, cmd, name, out_dir)
            .expect("clap_complete::generate_to must succeed in build.rs");
    }
}

/// Write a roff-format man page to `<OUT_DIR>/{name}.1` using
/// `clap_mangen`.
fn gen_manpage(out_dir: &Path, name: &str, cmd: clap::Command) {
    let path = out_dir.join(format!("{name}.1"));
    let mut buffer: Vec<u8> = Vec::with_capacity(4096);
    clap_mangen::Man::new(cmd)
        .render(&mut buffer)
        .expect("clap_mangen::Man::render must succeed in build.rs");
    std::fs::write(&path, buffer).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
}
