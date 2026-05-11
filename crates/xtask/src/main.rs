// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `xtask` — internal release-pipeline tooling for the noyalib
//! workspace.
//!
//! ```text
//! cargo xtask completions   # writes complete/*.{bash,fish,zsh,ps1}
//! cargo xtask manpages      # writes doc/*.1
//! cargo xtask notice        # writes NOTICE  (via cargo-about)
//! cargo xtask sbom          # writes SBOM.txt (via cargo tree)
//! cargo xtask vendor        # writes vendor/   (via cargo vendor)
//! cargo xtask pgo-build     # PGO-instrumented release build (5-15% extra)
//! cargo xtask all           # completions + manpages in one pass
//! ```
//!
//! The same [`clap::Command`] builders the binaries parse against
//! at runtime — exposed by `noya_cli::{noyafmt_command,
//! noyavalidate_command}` — drive both the build script
//! (`crates/noya-cli/build.rs`) and this xtask. So binaries, man
//! pages, and shell completions can never drift.
//!
//! `notice`, `sbom`, and `vendor` shell out to `cargo about`,
//! `cargo tree`, and `cargo vendor` respectively. They're
//! collected here so contributors only have one entry point to
//! remember and `make` targets can route through `cargo xtask
//! …` without conditional logic.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "xtask",
    about = "Internal release-pipeline tooling for noyalib",
    version = env!("CARGO_PKG_VERSION"),
)]
enum Cmd {
    /// Regenerate shell completions under `complete/`.
    Completions,
    /// Regenerate roff-format man pages under `doc/`.
    Manpages,
    /// Regenerate NOTICE via cargo-about (license attribution).
    Notice,
    /// Regenerate SBOM.txt via `cargo tree`.
    Sbom,
    /// Vendor every dependency under `vendor/` for offline builds.
    Vendor,
    /// Run the full Profile-Guided Optimisation (PGO) pipeline:
    /// instrumented build → train against bench fixtures → merge
    /// profdata → optimised rebuild. Produces a 5-15% extra speedup
    /// over `cargo build --release` on the parser hot paths.
    /// Wraps `scripts/pgo.sh`.
    PgoBuild {
        /// Optional training corpus YAML file. Defaults to the
        /// bundled `tests/yaml-test-suite/` + `benches/fixtures/`.
        #[arg(value_name = "TRAIN_INPUT")]
        train: Option<PathBuf>,
        /// Override the binary to drive (default: `noyafmt`).
        #[arg(long, value_name = "BIN")]
        target: Option<String>,
    },
    /// Run completions + manpages in one go.
    All,
}

fn main() -> std::io::Result<()> {
    match Cmd::parse() {
        Cmd::Completions => write_completions(&workspace_root()?)?,
        Cmd::Manpages => write_manpages(&workspace_root()?)?,
        Cmd::Notice => write_notice(&workspace_root()?)?,
        Cmd::Sbom => write_sbom(&workspace_root()?)?,
        Cmd::Vendor => write_vendor(&workspace_root()?)?,
        Cmd::PgoBuild { train, target } => run_pgo_build(&workspace_root()?, train, target)?,
        Cmd::All => {
            let root = workspace_root()?;
            write_completions(&root)?;
            write_manpages(&root)?;
        }
    }
    Ok(())
}

/// Locate the workspace root by walking upward from the xtask
/// crate until we find a `Cargo.toml` that contains a `[workspace]`
/// table. Cargo doesn't expose this directly, so we resolve via
/// `CARGO_MANIFEST_DIR` (the xtask's own dir) and pop two levels.
fn workspace_root() -> std::io::Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/xtask → crates/ → workspace root
    let root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "could not locate workspace root from xtask manifest dir",
            )
        })?
        .to_path_buf();
    Ok(root)
}

/// Write bash / fish / zsh / powershell completions to
/// `<root>/complete/`.
fn write_completions(root: &Path) -> std::io::Result<()> {
    use clap_complete::{Shell, generate_to};
    let out = root.join("complete");
    fs::create_dir_all(&out)?;
    for (name, mut cmd) in [
        ("noyafmt", noya_cli::noyafmt_command()),
        ("noyavalidate", noya_cli::noyavalidate_command()),
    ] {
        for shell in [Shell::Bash, Shell::Fish, Shell::Zsh, Shell::PowerShell] {
            let path = generate_to(shell, &mut cmd, name, &out).map_err(|e| {
                std::io::Error::other(format!("clap_complete::generate_to failed: {e}"))
            })?;
            eprintln!("→ {}", path.display());
        }
    }
    Ok(())
}

/// Run `cargo about generate` against the workspace's
/// `about.toml` plus the `about.hbs` template and write the
/// result to `<root>/NOTICE`. Requires `cargo-about` on PATH;
/// the Makefile `notice` target installs it on demand if absent.
fn write_notice(root: &Path) -> std::io::Result<()> {
    let about_toml = root.join("about.toml");
    let about_hbs = root.join("about.hbs");
    let notice = root.join("NOTICE");

    let status = Command::new("cargo")
        .args([
            "about",
            "generate",
            "-c",
            about_toml.to_str().unwrap(),
            "-o",
            notice.to_str().unwrap(),
            about_hbs.to_str().unwrap(),
        ])
        .current_dir(root)
        .status()?;

    if !status.success() {
        return Err(std::io::Error::other(format!(
            "cargo about generate failed (exit {status}); is `cargo-about` installed?"
        )));
    }
    eprintln!("→ {}", notice.display());
    Ok(())
}

/// Run `cargo tree --edges normal` and write the result to
/// `<root>/SBOM.txt`. Plain text by intent — every release
/// pipeline downstream of the GitHub Release attaches signed
/// machine-readable attestations alongside.
fn write_sbom(root: &Path) -> std::io::Result<()> {
    let path = root.join("SBOM.txt");
    let output = Command::new("cargo")
        .args([
            "tree", "--edges", "normal", "--prefix", "depth", "--format", "{p} {l}",
        ])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "cargo tree failed (exit {})",
            output.status,
        )));
    }
    fs::write(&path, &output.stdout)?;
    eprintln!("→ {}", path.display());
    Ok(())
}

/// Run `cargo vendor --versioned-dirs` against the workspace and
/// configure `.cargo/config.toml` to consume the result. Used by
/// air-gapped / RHEL / FIPS-bound build chains.
fn write_vendor(root: &Path) -> std::io::Result<()> {
    let vendor_dir = root.join("vendor");
    let status = Command::new("cargo")
        .args(["vendor", "--versioned-dirs", "vendor"])
        .current_dir(root)
        .status()?;

    if !status.success() {
        return Err(std::io::Error::other(format!(
            "cargo vendor failed (exit {status})"
        )));
    }
    eprintln!("→ {}", vendor_dir.display());
    eprintln!(
        "  configure cargo to use the vendored sources via\n\
         \n\
         \t[source.crates-io]\n\
         \treplace-with = \"vendored\"\n\
         \t[source.vendored]\n\
         \tdirectory = \"vendor\"\n\
         \n\
         in .cargo/config.toml, then build with `cargo build --offline`."
    );
    Ok(())
}

/// Drive the PGO pipeline at `scripts/pgo.sh`.
///
/// PGO ships as opt-in (not in the default `cargo build --release`
/// pipeline) because it requires `llvm-profdata` and a training
/// run that can take 10-30 s. End-state binary is at
/// `target/release/<TARGET>` with the optimised `-Cprofile-use`
/// flags applied.
fn run_pgo_build(
    root: &Path,
    train: Option<PathBuf>,
    target: Option<String>,
) -> std::io::Result<()> {
    let script = root.join("scripts").join("pgo.sh");
    if !script.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "PGO driver not found at {} (expected scripts/pgo.sh)",
                script.display()
            ),
        ));
    }
    let mut cmd = Command::new("bash");
    let _ = cmd.arg(&script).current_dir(root);
    if let Some(t) = target {
        let _ = cmd.env("PGO_TARGET", t);
    }
    if let Some(p) = train {
        let _ = cmd.arg(p);
    }
    let status = cmd.status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "scripts/pgo.sh exited with {status}"
        )));
    }
    Ok(())
}

/// Write roff-format man pages to `<root>/doc/`.
fn write_manpages(root: &Path) -> std::io::Result<()> {
    let out = root.join("doc");
    fs::create_dir_all(&out)?;
    for (name, cmd) in [
        ("noyafmt", noya_cli::noyafmt_command()),
        ("noyavalidate", noya_cli::noyavalidate_command()),
    ] {
        let path = out.join(format!("{name}.1"));
        let mut buffer: Vec<u8> = Vec::with_capacity(4096);
        clap_mangen::Man::new(cmd)
            .render(&mut buffer)
            .map_err(|e| std::io::Error::other(format!("clap_mangen::Man::render failed: {e}")))?;
        fs::write(&path, buffer)?;
        eprintln!("→ {}", path.display());
    }
    Ok(())
}
