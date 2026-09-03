<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Fedora package review request

Fedora Rust packages are generated with `rust2rpm` — hand-written
specs are rejected. Steps (needs a Fedora account with packager
sponsorship or a sponsor via the review):

1. `dnf install rust2rpm` (or run in a Fedora container), then
   `rust2rpm noyalib` → produces `rust-noyalib.spec`.
2. Build the SRPM: `fedpkg --release rawhide mockbuild` or
   `rpmbuild -bs rust-noyalib.spec` with the crate in `SOURCES`.
3. Upload spec + SRPM somewhere public (COPR or fedorapeople).
4. File the review at
   <https://bugzilla.redhat.com/enter_bug.cgi?product=Fedora&component=Package%20Review>
   with the summary `Review Request: rust-noyalib - pure-Rust YAML
   1.2 library with serde and lossless editing`, linking spec + SRPM
   and noting: dual MIT/Apache-2.0, no unsafe code, vendored
   official test suite runs offline, upstream maintainer filing.
5. After approval: `fedpkg request-repo rust-noyalib <bug#>` and
   import. noya-cli follows the same flow once the library lands.
