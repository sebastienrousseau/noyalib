<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# OSS-Fuzz integration

This directory is the [OSS-Fuzz](https://github.com/google/oss-fuzz)
project definition for noyalib, kept in-tree so it can be reviewed
and dry-run alongside the fuzz targets it ships.

## What ships

- `project.yaml` — project metadata (maintainer contact, engines,
  sanitizers).
- `Dockerfile` — clones this repository into the OSS-Fuzz
  `base-builder-rust` image.
- `build.sh` — builds every `[[bin]]` in `fuzz/Cargo.toml` with
  `cargo fuzz build -O --debug-assertions` and ships each binary
  with the shared seed corpus (`fuzz/corpus/seed/`) and the YAML
  token dictionary (`fuzz/yaml.dict`).

## Submitting

OSS-Fuzz onboarding is a PR to `google/oss-fuzz` adding these three
files as `projects/noyalib/`, opened by the maintainer named in
`project.yaml` (the contact receives crash reports and the
ClusterFuzz console access):

```sh
git clone https://github.com/google/oss-fuzz
mkdir oss-fuzz/projects/noyalib
cp fuzz/oss-fuzz/{project.yaml,Dockerfile,build.sh} oss-fuzz/projects/noyalib/
# open the PR from a fork
```

Note `build.sh` references `fuzz/yaml.dict`, which must be on the
default branch of this repository before the OSS-Fuzz build (which
clones `main`) can succeed.

## Dry-running locally

From an `oss-fuzz` checkout, against this working tree (no push
needed):

```sh
python3 infra/helper.py build_fuzzers noyalib /path/to/noyalib
python3 infra/helper.py check_build noyalib
python3 infra/helper.py run_fuzzer noyalib fuzz_parse -- -max_total_time=60
```

## Keeping it working

- A new `[[bin]]` in `fuzz/Cargo.toml` is picked up automatically
  (`build.sh` iterates `fuzz_targets/*.rs`).
- The differential target (`fuzz_diff`) builds `serde_yaml_ng` and
  `saphyr` — dependency breakage there fails the OSS-Fuzz build,
  which mails the `project.yaml` contact.
- OSS-Fuzz builds from `main`; the in-tree copy here is the source
  of truth, and changes should be mirrored to
  `google/oss-fuzz/projects/noyalib` in a follow-up PR.
