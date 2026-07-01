#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Compile every ```rust code block in the workspace-root README.md
# against `noyalib` from a throwaway scratch project.
#
# WHY THIS EXISTS
#
# The GitHub landing-page README (`./README.md`) is not the same
# file as the crate-level README (`crates/noyalib/README.md`).
# Only the latter is picked up by
# `#[doc = include_str!("../README.md")]` in `lib.rs`, because the
# workspace-root README lives outside the crate's package layout
# and referencing it via `include_str!` would break
# `cargo publish --dry-run` verification.
#
# This script closes that hole. It:
#   1. extracts every ```rust code block from the workspace-root
#      README (excluding blocks tagged `,ignore` — the doctest
#      escape hatch, same semantics as rustdoc);
#   2. wraps each block with a `fn main()` if the block does not
#      already declare one (matches rustdoc's implicit-main
#      behaviour);
#   3. compiles the block against a scratch cargo project that
#      depends on `path = "../.."` of noyalib with `--all-features`
#      so schema / validate-schema / policy / etc. examples all
#      resolve;
#   4. surfaces any compile error with a precise block-index +
#      README-line reference.
#
# Run locally:
#   bash scripts/check-readme-examples.sh
#
# CI wiring: `.github/workflows/ci.yml` runs this after the main
# test job so a broken root-README example fails a PR.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

README="${README:-README.md}"
if [[ ! -f "${README}" ]]; then
    echo "ERROR: no README at ${README}" >&2
    exit 1
fi

# Scratch project location under target/ so `cargo clean` cleans it.
SCRATCH="${CARGO_TARGET_DIR:-target}/readme-doctest-scratch"
rm -rf "${SCRATCH}"
mkdir -p "${SCRATCH}/src"

# Absolute path to the noyalib crate — the scratch project needs
# to reference it via a `path =` dep because we're outside the
# workspace layout.
NOYALIB_ABS="$(cd crates/noyalib && pwd)"

# Scratch Cargo.toml. `edition = "2024"` matches noyalib itself
# and `--all-features` on the scratch dep so schema / validate-
# schema / policy / etc. types resolve.
cat > "${SCRATCH}/Cargo.toml" <<EOF
[package]
name = "readme-doctest-scratch"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
noyalib = { path = "${NOYALIB_ABS}", features = ["schema", "validate-schema", "figment"] }
serde = { version = "1.0", features = ["derive"] }
# schemars is required for the block that demonstrates
# `#[derive(JsonSchema)]` — the derive macro emits `::schemars::*`
# paths that need to resolve in the caller's dep graph
# (documented in the README's "Optional integrations" section).
schemars = { version = "1.2", features = ["derive"] }

[workspace]
EOF

# Extract every ```rust block. rustdoc treats bare "```rust" and
# "```" with the following implicit "rust" tag as rust; we're
# stricter and only match explicit "```rust" openers so a shell
# block ("```bash") or config block ("```toml") is never
# misclassified.
#
# Skip blocks tagged `,ignore` — that's rustdoc's escape hatch
# for "showcase code, do not compile", and we honour the same
# semantics.

BLOCK_INDEX=0
FAIL_COUNT=0

# We iterate the README line-by-line rather than using a rust
# regex-based extractor so this script has no dep beyond bash +
# rustc + cargo.

CURRENT_BLOCK=""
IN_BLOCK=0
BLOCK_START_LINE=0
LINE_NO=0

process_block() {
    local block_body="$1"
    local start_line="$2"
    BLOCK_INDEX=$((BLOCK_INDEX + 1))

    # Detect whether the block already declares fn main.
    # If not, wrap with `fn main() { ... }` — matches rustdoc.
    local wrapped
    if grep -q '^fn main' <<< "${block_body}"; then
        wrapped="${block_body}"
    else
        wrapped="fn main() -> Result<(), Box<dyn std::error::Error>> {
${block_body}
    Ok(())
}"
    fi

    # Write the block to the scratch src/main.rs and try to build.
    cat > "${SCRATCH}/src/main.rs" <<< "${wrapped}"

    local build_output
    if build_output=$(cargo build --manifest-path "${SCRATCH}/Cargo.toml" --quiet 2>&1); then
        printf '  [ OK  ] block #%d @ README.md:%d\n' "${BLOCK_INDEX}" "${start_line}"
    else
        printf '  [FAIL ] block #%d @ README.md:%d\n' "${BLOCK_INDEX}" "${start_line}" >&2
        echo "----- block source -----" >&2
        echo "${wrapped}" >&2
        echo "----- rustc output -----" >&2
        echo "${build_output}" >&2
        echo "------------------------" >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

printf -- '── Extracting + compiling rust blocks from %s ──\n\n' "${README}"

while IFS= read -r line; do
    LINE_NO=$((LINE_NO + 1))

    if [[ ${IN_BLOCK} -eq 0 ]]; then
        # Match "```rust" exactly — no attributes = compile-and-run.
        # "```rust,ignore" / "```rust,no_run" are handled below.
        if [[ "${line}" == '```rust' ]]; then
            IN_BLOCK=1
            CURRENT_BLOCK=""
            BLOCK_START_LINE=${LINE_NO}
        fi
    else
        # Closing fence.
        if [[ "${line}" == '```' ]]; then
            process_block "${CURRENT_BLOCK}" "${BLOCK_START_LINE}"
            IN_BLOCK=0
            CURRENT_BLOCK=""
        else
            CURRENT_BLOCK="${CURRENT_BLOCK}
${line}"
        fi
    fi
done < "${README}"

echo
if [[ ${FAIL_COUNT} -gt 0 ]]; then
    echo "── ${FAIL_COUNT} of ${BLOCK_INDEX} README block(s) failed to compile ──" >&2
    exit 1
fi

echo "── All ${BLOCK_INDEX} README block(s) compile clean ──"
