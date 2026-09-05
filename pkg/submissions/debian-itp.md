<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Debian ITP (Intent To Package)

Send the mail below from your Debian-known address to
`submit@bugs.debian.org` (plain text). The Debian Rust team then
usually packages library crates via `debcargo` from their team
workflow; offering the crate on `#debian-rust` (OFTC) after filing
speeds it up.

```text
To: submit@bugs.debian.org
Subject: ITP: rust-noyalib -- pure-Rust YAML 1.2 library with serde and lossless editing

Package: wnpp
Severity: wishlist
Owner: Sebastien Rousseau <sebastian.rousseau@gmail.com>
X-Debbugs-Cc: debian-rust@lists.debian.org

* Package name    : rust-noyalib
  Version         : 0.0.31
  Upstream Author : Sebastien Rousseau <sebastian.rousseau@gmail.com>
* URL             : https://github.com/sebastienrousseau/noyalib
* License         : MIT or Apache-2.0
  Programming Lang: Rust
  Description     : pure-Rust YAML 1.2 library with serde and lossless editing

noyalib is a YAML 1.2 library written entirely in safe Rust
(#![forbid(unsafe_code)]): serde data binding, a lossless CST for
byte-faithful editing, JSON Schema validation, and 406/406 on the
official YAML test suite. Releases are signed, SLSA-attested, and
ship a CycloneDX SBOM; the dependency tree is cargo-vet audited.

It is the engine of the noya-cli tools (noyafmt, noyavalidate),
which I intend to package as a follow-up once the library is in,
and of noyalib-serde-yaml, a drop-in replacement for the archived
serde_yaml crate. I am the upstream maintainer and will keep the
package in lockstep with upstream's +0.0.1 release cadence.

Packaging offline is supported first-class: lockfiles are committed,
`make vendor` produces the full dependency tree, and the official
YAML test suite is vendored so the compliance suite runs hermetically.
See doc: https://github.com/sebastienrousseau/noyalib/blob/main/docs/packaging.md
```
