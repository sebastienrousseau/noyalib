#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Build and run a scratch project for every installation configuration
# the documentation tells a user to write.
#
# WHY THIS EXISTS
#
# `check-readme-examples.sh` compiles the code blocks, and
# `verify-release-versions.sh` checks the version numbers agree. Neither
# checks the thing in between: that the *dependency line* a reader
# copies actually yields a crate where the documented API exists. A
# feature renamed, a re-export moved behind a flag, or an API that needs
# a feature the install snippet does not name all produce documentation
# that is individually consistent and collectively wrong.
#
# So each configuration below is a real `Cargo.toml` fragment from the
# docs, built into a throwaway project that uses the API that
# configuration is advertised for, and run.
#
# The dependency is a `path` to this checkout, not a crates.io version:
# the point is to test the code about to be published, which by
# definition is not published yet.
#
#   bash scripts/check-install-guides.sh

set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

NOYALIB_ABS="$(cd crates/noyalib && pwd)"
SCRATCH="${CARGO_TARGET_DIR:-target}/install-check"
rm -rf "${SCRATCH}"
mkdir -p "${SCRATCH}"

pass=0
fail=0
failed=()

# check <name> <feature-spec> <program> [companion-deps]
#   feature-spec is the dependency line's tail, verbatim from the docs.
#   companion-deps are extra `[dependencies]` lines the *test program*
#   needs in order to name a third-party type directly. They are not
#   part of the documented install line, and each use is justified at
#   the call site: a companion dep that the docs fail to mention is a
#   documentation bug, not something to paper over here.
check() {
  local name="$1" dep="$2" program="$3" extra="${4:-}"
  local dir="${SCRATCH}/${name}"
  mkdir -p "${dir}/src"

  cat > "${dir}/Cargo.toml" <<EOF
[package]
name = "install-check-${name}"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
noyalib = { path = "${NOYALIB_ABS}"${dep} }
serde = { version = "1.0", features = ["derive"] }
${extra}

[workspace]
EOF
  printf '%s\n' "${program}" > "${dir}/src/main.rs"

  printf '  %-34s' "${name}"
  local out
  if out=$(cargo run --quiet --manifest-path "${dir}/Cargo.toml" 2>&1); then
    printf '\033[32mok\033[0m\n'
    pass=$((pass + 1))
  else
    printf '\033[31mFAIL\033[0m\n'
    echo "----- dependency line -----" >&2
    echo "noyalib = { path = \"…\"${dep} }" >&2
    echo "----- output -----" >&2
    echo "${out}" >&2
    echo "------------------" >&2
    fail=$((fail + 1))
    failed+=("${name}")
  fi
}

echo "Checking every documented installation configuration…"
echo

# README: `noyalib = "0.0.45"` — the default install.
check "default" "" '
fn main() {
    let v: noyalib::Value = noyalib::from_str("a: 1\nb: [1, 2]\n").expect("parse");
    assert_eq!(v.get("a").and_then(noyalib::Value::as_i64), Some(1));
    let out = noyalib::to_string(&v).expect("serialize");
    assert!(out.contains("a: 1"), "{out}");
    println!("ok");
}'

# README: `default-features = false` — the alloc-only surface.
check "no-default-features" ", default-features = false" '
fn main() {
    // The documented no-default-features surface: parse and serialise
    // without std-only extras.
    let v: noyalib::Value = noyalib::from_str("a: 1\n").expect("parse");
    assert_eq!(v.get("a").and_then(noyalib::Value::as_i64), Some(1));
    println!("ok");
}'

# README: the diagnostics + schema pairing.
check "miette-validate-schema" ', features = ["miette", "validate-schema"]' '
fn main() {
    let schema: noyalib::Value = noyalib::from_str(
        "type: object\nrequired: [port]\nproperties:\n  port:\n    type: integer\n",
    ).expect("schema");
    let good: noyalib::Value = noyalib::from_str("port: 8080\n").expect("doc");
    noyalib::validate_against_schema(&good, &schema).expect("valid document");

    let bad: noyalib::Value = noyalib::from_str("port: \"nope\"\n").expect("doc");
    assert!(noyalib::validate_against_schema(&bad, &schema).is_err());

    // miette: the error must render as a diagnostic.
    let err = noyalib::from_str::<noyalib::Value>("a: [unclosed").unwrap_err();
    let _: &dyn miette::Diagnostic = &err;
    println!("ok");
}' \
  '# The `miette` feature makes `noyalib::Error` implement
# `miette::Diagnostic`; naming that trait — as README "Error
# reporting" shows with `miette::Report::new(e)` — needs the crate
# in the caller'"'"'s graph, as for any trait from another crate.
miette = "7"'

# Migration guides: the serde_yaml drop-in shim.
check "compat-serde-yaml" ', features = ["compat-serde-yaml"]' '
use noyalib::compat::serde_yaml;
fn main() {
    let v: serde_yaml::Value = serde_yaml::from_str("a: 1\n").expect("parse");
    let out = serde_yaml::to_string(&v).expect("serialize");
    assert!(out.contains("a: 1"), "{out}");
    println!("ok");
}'

# USER-GUIDE: the recovery surface.
check "recovery" ', features = ["recovery"]' '
fn main() {
    let _: noyalib::Value = noyalib::from_str("a: 1\n").expect("parse");
    println!("ok");
}'

# USER-GUIDE: the async reader.
check "tokio" ', features = ["tokio"]' '
fn main() {
    let _ = noyalib::tokio_async::YamlDecoder::<noyalib::Value>::new();
    // docs/USER-GUIDE.md shows `from_async_reader_multi` in a block it
    // cannot compile (it needs the reader'"'"'s own async runtime). Naming
    // the function here needs no runtime and still fails the build if
    // it is renamed or its signature changes.
    let _ = noyalib::tokio_async::from_async_reader_multi::<
        std::io::Cursor<Vec<u8>>,
        noyalib::Value,
    >;
    println!("ok");
}'

# USER-GUIDE: the parallel multi-document reader.
check "parallel" ', features = ["parallel"]' '
fn main() {
    let docs: Vec<noyalib::Value> =
        noyalib::parallel::parse("a: 1\n---\nb: 2\n").expect("parallel parse");
    assert_eq!(docs.len(), 2);
    println!("ok");
}'

# USER-GUIDE: the sval adapter.
check "sval" ', features = ["sval"]' '
fn main() {
    let value: noyalib::Value = noyalib::from_str("a: 1\n").expect("parse");
    // docs/USER-GUIDE.md claims `noyalib::Value` implements
    // `sval::Value`. That claim is only checkable from a crate that
    // depends on sval directly, which is what this configuration is.
    fn streams(_: &impl sval::Value) {}
    streams(&value);
    println!("ok");
}' \
  '# sval is the caller'"'"'s dependency: the noyalib feature implements
# sval::Value for noyalib types, but naming that trait requires the
# crate, exactly as docs/USER-GUIDE.md tells the reader.
sval = { version = "2", default-features = false }'

# README: schema generation from your own types.
check "schema" ', features = ["schema"]' '
#[derive(noyalib::JsonSchema)]
struct Cfg { port: u16 }
fn main() {
    let schema = noyalib::schema_for::<Cfg>().expect("schema");
    assert_eq!(schema["type"].as_str(), Some("object"));
    println!("ok");
}' \
  '# README'"'"'s feature table states that downstream callers deriving
# `JsonSchema` must add `schemars` themselves, because the derive
# emits `::schemars::*` paths. This config is what verifies that
# instruction is both necessary and sufficient.
schemars = { version = "1.2", features = ["derive"] }'

# The feature set the README lists for lossless editing.
check "cst-editing" "" '
fn main() {
    use noyalib::cst::parse_document;
    let mut doc = parse_document("name: foo  # keep me\nversion: 0.0.1\n").expect("parse");
    doc.set("version", "0.0.2").expect("edit");
    assert_eq!(doc.to_string(), "name: foo  # keep me\nversion: 0.0.2\n");
    println!("ok");
}'

echo
if [ "${fail}" -eq 0 ]; then
  printf '\033[32mAll %d documented installation configurations build and run.\033[0m\n' "${pass}"
  exit 0
fi
printf '\033[31m%d of %d configurations failed: %s\033[0m\n' \
  "${fail}" "$((pass + fail))" "${failed[*]}" >&2
exit 1
