# syntax=docker/dockerfile:1.7
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

# pkg/docker/Dockerfile — minimal distroless image for `noyafmt`.
#
# Two-stage build:
#   1. Compile the noya-cli binaries on a full Rust toolchain image.
#   2. Copy them into a distroless `cc-debian12:nonroot` runtime
#      (~25 MB total) for production deployment.
#
# Build:
#   docker buildx build \
#     --platform linux/amd64,linux/arm64 \
#     --build-arg VERSION=v0.0.1 \
#     -t ghcr.io/sebastienrousseau/noyafmt:0.0.1 \
#     -f pkg/docker/Dockerfile .
#
# Run:
#   docker run --rm -v "$(pwd):/work" -w /work \
#     ghcr.io/sebastienrousseau/noyafmt:0.0.1 \
#     --check ci/*.yaml

# ── Build stage ──────────────────────────────────────────────────
# Rust 1.85-bookworm matches the workspace MSRV (raised in v0.0.5
# when the edition bumped to 2024). Digest pinned for OpenSSF
# Scorecard Pinned-Dependencies; Dependabot's `docker` ecosystem
# bumps it on its weekly cadence.
FROM rust:1.96-bookworm@sha256:13c186980fa33cc12759b429662a1322939dbe697484b7c33b47dd2698d28460 AS build

WORKDIR /src
COPY . .

# Reproducible build flags. SOURCE_DATE_EPOCH is wired in by the
# release pipeline; we accept it via build-arg here.
ARG SOURCE_DATE_EPOCH
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-0}
ENV RUSTFLAGS="--remap-path-prefix=/src=/build --remap-path-prefix=/usr/local/cargo=/cargo"
ENV NOYA_GEN_ASSETS=1

RUN cargo build --release --locked \
    --manifest-path crates/noya-cli/Cargo.toml \
    --bin noyafmt --bin noyavalidate

# ── Runtime stage ────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:bd2899c12b335c827750ccf2359879eab09c09b206023dcebea408947d54127c

LABEL org.opencontainers.image.title="noyafmt"
LABEL org.opencontainers.image.description="Pure-Rust YAML 1.2 formatter (lossless CST). See ghcr.io/sebastienrousseau/noyalib-mcp for the MCP server image."
LABEL org.opencontainers.image.source="https://github.com/sebastienrousseau/noyalib"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"

COPY --from=build /src/target/release/noyafmt      /usr/local/bin/noyafmt
COPY --from=build /src/target/release/noyavalidate /usr/local/bin/noyavalidate

USER nonroot:nonroot
WORKDIR /work
ENTRYPOINT ["/usr/local/bin/noyafmt"]
