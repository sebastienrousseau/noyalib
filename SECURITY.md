# Security Policy

## Supported Versions

| Version | Supported |
|:--------|:---------:|
| 0.0.x   | Yes       |

## Reporting a Vulnerability

Report security vulnerabilities by emailing **sebastian.rousseau@gmail.com**.

Do not open a public issue for security reports.

Include:

- A description of the vulnerability.
- Steps to reproduce.
- Affected versions.
- Any suggested fix (optional).

Expect an initial response within 48 hours. A fix or mitigation plan will follow within 7 days of confirmation.

## Security Design

noyalib enforces safety at the compiler level:

- `#![forbid(unsafe_code)]` — zero unsafe blocks, guaranteed.
- No C dependencies, no FFI calls. Pure Rust only.
- No network I/O, no file system writes, no environment variable reads.

### Parser Hardening

Configurable limits protect against denial-of-service attacks:

| Limit | Default | Purpose |
|:------|:--------|:--------|
| `max_depth` | 128 | Prevents stack exhaustion from deep nesting |
| `max_document_length` | 64 MB | Rejects oversized input |
| `max_alias_expansions` | 1,024 | Prevents billion-laughs amplification |
| `max_mapping_keys` | 65,536 | Caps mapping size |
| `max_sequence_length` | 65,536 | Caps sequence size |

Use `ParserConfig::strict()` for a hardened preset suitable for untrusted input.

#### v0.0.6 optional surfaces

The three opt-in modules added in v0.0.6 (`recovery`, `sval`,
`tokio`) inherit every limit above and add their own DoS
mitigations:

| Surface | Guard | Knob |
|:--------|:------|:-----|
| `noyalib::recovery::parse_lenient` | `---`-marker count cap (defeats marker-spam OOM) | `ParserConfig::max_documents` |
| `noyalib::recovery::parse_lenient` | Cumulative byte budget across line-truncation retries (defeats O(n²) re-parse on 10k-line malformed input) | `LenientConfig::truncation_event_budget` (default 1 MiB) |
| `noyalib::tokio_async::from_async_reader` | Bounded `AsyncReadExt::take(max_document_length)` drain (defeats slow-drip OOM) | `ParserConfig::max_document_length` |
| `noyalib::tokio_async::YamlDecoder` | Optional inter-frame buffer cap (defeats codec buffer pinning by adversarial producer) | `YamlDecoder::max_frame_size(usize)` |

Untrusted-input deployments driving `YamlDecoder` over a network
stream **should** call `max_frame_size(_)` to a sane upper bound;
the default `None` matches the trust contract of a process-local
in-memory consumer.

#### `max_depth` guard correctness (issue #46)

The `max_depth` guard above relies on balanced increment /
decrement of the parser's internal depth counter on every
code path. Two correctness bugs were patched in v0.0.6 — one
that made `from_str::<Value>` falsely report
`RecursionLimitExceeded` on shallow `pnpm-lock.yaml` inputs,
and one in the no-span loader path that incremented depth
without ever checking the limit. See
[`docs/POLICIES.md` § Resource-limit gates](docs/POLICIES.md#resource-limit-gates)
for the full treatment.

### Supply Chain

- Runtime deps audited (`cargo-deny` in CI): license validation, advisory checks, source verification.
- `Cargo.lock` committed for deterministic builds.
- All GitHub Actions SHA-pinned.

### Build Provenance & Artefact Signing

Every tagged release goes through three layers of supply-chain integrity:

1. **SLSA Level 3 build provenance.** The `release.yml` workflow uses
   `actions/attest-build-provenance` to record the source commit, the
   builder identity, and the workflow invocation into the public Rekor
   transparency log. Verifiable from any clone of the repo:

   ```sh
   gh attestation verify --owner sebastienrousseau noyalib-0.0.1.crate
   ```

2. **Keyless sigstore signing.** The same workflow uses `cosign`
   keyless signing (Fulcio + Rekor) to produce a `.bundle` alongside
   every artefact. The certificate identity is bound to this repo and
   this workflow; verifiers should pin both:

   ```sh
   cosign verify-blob \
     --certificate-identity-regexp '^https://github\.com/sebastienrousseau/noyalib/' \
     --certificate-oidc-issuer https://token.actions.githubusercontent.com \
     --bundle noyalib-0.0.23.crate.bundle \
     noyalib-0.0.23.crate
   ```

   > Releases up to and including v0.0.23 shipped a `.bundle` rather
   > than the separate `.sig` / `.pem` this section used to describe.
   > cosign v4 deprecated `--output-signature` / `--output-certificate`
   > and writes only the bundle, which carries both the signature and
   > the Fulcio certificate. The old command could not succeed against
   > any published release.

3. **Detached GPG signature.** Additive to the above, for consumers who
   want an offline check with the `gpg` their distribution already
   ships. Every `.crate` and the SBOM carry a matching `.asc`:

   ```sh
   # Import the release-signing key (also in KEYS.asc at the repo root)
   gpg --recv-keys 4B7F16C909C7A8EE9BED338A4F047EDF5F90F638

   gpg --verify noyalib-0.0.23.crate.asc noyalib-0.0.23.crate
   ```

   **Release-signing key fingerprint:**

   ```text
   4B7F16C909C7A8EE9BED338A4F047EDF5F90F638
   ```

   Signing key `Sebastien Rousseau <sebastian.rousseau@gmail.com>`,
   ed25519, signing-only, expires 2028-08-16. Verify the fingerprint
   out of band before trusting it — a key fetched over the same channel
   as the artefact proves nothing on its own. The sigstore bundle needs
   no such step, which is why it remains the recommended check.

4. **Software bill of materials (SBOM).** Generated by `cargo tree` at
   build time, signed alongside the `.crate`, attached to the GitHub
   Release. Lets downstream consumers reproduce the dependency
   closure used at release time.

### Commit Integrity

All commits on the main branch must be signed. CI rejects unsigned pull request commits.
