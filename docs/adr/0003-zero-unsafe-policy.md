# 0003. `#![forbid(unsafe_code)]` workspace-wide

- **Status:** accepted
- **Date:** 2026-04-30
- **Authors:** Sebastien Rousseau

## Context

YAML parsing is a fertile ground for memory-safety bugs. The C
reference implementation `libyaml` has had multiple CVEs over the
years involving heap overruns, use-after-frees, and integer
overflows in the scanner state machine. Even pure-Rust
implementations have shipped subtle UB bugs — in 2023, a
widely-used Rust YAML parser carried an `unsafe` block that
violated stacked-borrow rules and only surfaced under Miri.

noyalib is positioned as the safe replacement. Allowing even one
`unsafe` block — anywhere in the workspace — would compromise the
core promise. Worse, it would invite "just one small `unsafe` for
performance" arguments later, and the policy would erode by a
thousand exceptions.

The cost of forbidding `unsafe` is real but bounded: parser hot
paths can still use SIMD via `memchr` (which contains audited
`unsafe` *internally* but exposes a safe API), and the SWAR
numeric parser uses pure-safe `u64` packing rather than
unaligned-load tricks.

## Decision

Every crate in the workspace begins with:

```rust
#![forbid(unsafe_code)]
```

This is enforced at the **lint level** (`forbid`, not `deny`) so
the attribute cannot be locally disabled with `#[allow]`. Any
PR that introduces an `unsafe` block — or even a `deny`-level
attribute that could be locally bypassed — will fail to compile
on every target.

Performance-critical code reaches for SIMD and unaligned
operations through `memchr` and well-vetted dependencies whose
own `unsafe` is audited externally. noyalib's own code stays
provably safe.

The policy applies workspace-wide:

- `noyalib` (core library) — forbidden
- `noyalib-lsp` — forbidden
- `noyalib-mcp` — forbidden
- `noyalib-wasm` — forbidden (the JsValue glue uses `wasm_bindgen`'s safe wrappers)
- `noya-cli` — forbidden
- `xtask` — forbidden

No exceptions, no gates, no profiles where it's relaxed.

## Consequences

**Positive:**

- The headline compliance claim ("zero unsafe") is mechanically
  enforced. A reviewer or auditor running `rg "unsafe"` against
  the workspace will find no matches in non-test code, period.
- Memory-safety bugs in parser code become impossible-by-construction.
  noyalib could still have logic bugs (off-by-ones, infinite
  loops, panics on malformed input), but it cannot have
  use-after-free, double-free, data races, or buffer overruns
  in its own code.
- Audit cost evaporates. A FIPS / DoD / supply-chain reviewer
  who would otherwise have to vet every `unsafe` block can
  short-circuit on the `forbid(unsafe_code)` attribute.
- `cargo geiger` reports zero `unsafe` lines for noyalib,
  improving its score on supply-chain dashboards.

**Negative:**

- Some optimisations are off the table. A handwritten unaligned
  SIMD load could shave ~10% off the structural-bitmask scan;
  we don't get that. Mitigation: the SWAR fallback and `memchr`
  cover the realistic upper bound; the perf bench numbers
  versus `serde_yaml_ng` show we're not leaving meaningful
  throughput on the floor.
- Some idioms require workarounds. Self-referential structs are
  out; recursive descent uses heap allocation where some C parsers
  use stack tricks. We pay 1× recursion overhead per level for
  this; in practice it's noise.
- Dependency choices are constrained. We cannot pull in a crate
  whose public API is `unsafe fn` even if its internals are sound,
  because the wrapper would force noyalib's surface to relax.
  Mitigation: every direct dep we need (memchr, indexmap,
  rustc-hash, itoa, ryu, smallvec) has a safe public API.

**Neutral:**

- The Miri test pass becomes evidence about *dependencies*, not
  about noyalib itself. UB found by Miri must be in a dep — which
  is still useful to know but reframes what a Miri failure
  means.

## Alternatives considered

### `deny(unsafe_code)` instead of `forbid`

`deny` allows local override via `#[allow(unsafe_code)]`. Rejected
because the whole point is mechanical enforcement that can't be
silently bypassed in a single file. `forbid` makes the attribute
a hard compile-time gate.

### Allow `unsafe` in performance-critical paths only

Carve out `parser/scanner.rs` for SIMD intrinsics and forbid
elsewhere. Rejected because (a) the SIMD perf wins via
`unsafe_code` are bounded, (b) the audit story degrades from
"zero unsafe" to "12 unsafe blocks, here's why each one is
sound" — a worse pitch and a worse maintenance burden, (c) the
exception would invite scope creep.

### Allow `unsafe` only in `xtask` / build tooling

These crates don't ship in user binaries, so an `unsafe`
exception would be invisible to the security-conscious user.
Rejected because the contributor experience matters too — every
contributor reading the codebase should see `forbid(unsafe_code)`
and understand it's universal.

### Use `cap-std` or sandboxed toolchains for runtime safety

These help against bugs in *dependencies*, not in noyalib itself.
Orthogonal to the policy. Considered for the CLI / LSP
deployment story but not for the core library.

## References

- The lint: https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unsafe-code
- Forbid vs deny: https://doc.rust-lang.org/rustc/lints/levels.html#forbid
- `cargo-geiger`: https://github.com/rust-secure-code/cargo-geiger
- Workspace lint config: each crate's `[lints.rust]` table sets
  `unsafe_code = "forbid"` so `cargo clippy --all-targets` enforces
  it across tests, examples, and benches as well as `src/`.
