<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# 0017. Make allocation-based tooling available without std

- **Status:** accepted
- **Date:** 2026-11-08
- **Authors:** Noyalib contributors
- **Amends:** the portability restriction in [0001](0001-cst-rowan-shape.md)

## Context

The CST, property substitution, recursive `Rc` cells, strict unknown-field
helpers, and recovery parser use allocation and core facilities. Their std
gates prevented embedded callers from using existing functionality. Explicit
source contexts introduced for typed alloc-only loading remove the original
span-tracking obstacle. The maintainer requested broader no_std functionality.

## Decision

Expose CST parsing and mutation, property substitution, `RcRecursive`, and
`RcRecursion` with `alloc`. Let `strict-deserialise` expose its string and
slice helpers without std; retain the reader helper's std requirement. Remove
the std implication from `recovery`, whose implementation uses the internal
alloc prelude and existing atomic stream counters.

Keep hosted property-map signatures unchanged. Alloc-only property maps use
`hashbrown::HashMap` with `rustc_hash::FxBuildHasher`, matching the other
internal alloc-only hash maps. Preserve interpolation's post-processing order,
strict behavior, and expanded-scalar budget. Strict unknown-field collection
uses a local vector because its callback is synchronous.

Retain the shared scanner, separate green tree, source span ownership rules,
edit rollback guards, allocator requirement, and pointer-width atomics. No
unsafe code, new dependency, MSRV change, or new feature is needed.

IO readers and writers, filesystem resolvers, thread runtimes, and recursive
`Arc<Mutex<_>>` wrappers still require std. Automatic shared-identity
serialization also retains its scoped thread-local implementation; portable
callers can use the existing explicit anchor registries. Replacing that
mechanism or supplying synchronization on targets without pointer atomics
requires a separate design that preserves public serialization semantics.

## Consequences

- Embedded consumers can edit YAML byte-faithfully and recover partial documents.
- Enabling recovery no longer enables std as a side effect. Consumers needing
  hosted APIs must enable std explicitly, as with the other portable features.
- The same CST and portable regression suites exercise hosted and alloc-only
  builds; true bare-metal builds check dependency isolation.
- Alloc-only rustdoc must resolve without referring to unavailable hosted items.
- Existing hosted function signatures and default features stay compatible.

## Alternatives considered

Keep these algorithms std-only: rejected because their implementations need
only facilities already present in core and alloc.

Replace Arc with Rc on targets without atomics: rejected here because that
changes synchronization and Send/Sync guarantees. No fallback is silently added.
