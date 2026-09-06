// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The resource budgets the loaders enforce, as pure predicates.
//!
//! Every limit a hostile document could exploit (nesting depth, alias
//! expansion, the alias-to-anchor ratio, the transitive repetition
//! charge, expanded bytes, node count) is decided here rather than
//! inline in the event loop. The functions take plain integers and
//! return plain answers, which is what lets them be proved rather than
//! only tested: the `proofs` module at the bottom is checked by
//! [Kani](https://model-checking.github.io/kani/) in CI over every
//! input value, not a sample of them.
//!
//! The loaders call these at the same points they always did; the
//! refactor changed no behaviour, and the fuzzers and the official test
//! suite cover the loaders' use of them.

/// Upper bound on the bytes an alias may expand to, cumulatively, in one
/// document, regardless of the configured document length.
pub(crate) const MAX_ALIAS_BYTES: usize = 1024 * 1024 * 32; // 32 MB

/// `true` once `depth` has exceeded `max_depth`.
///
/// Called after the depth counter is incremented for a collection start,
/// so a document nested exactly `max_depth` deep is accepted and one
/// level deeper is refused.
#[inline]
#[must_use]
pub(crate) const fn depth_exceeded(depth: usize, max_depth: usize) -> bool {
    depth > max_depth
}

/// `true` once `alias_count` has exceeded `max_alias_expansions`.
///
/// Counts alias *occurrences*, not the nodes they expand to; the
/// transitive cost is charged by [`jump_charge_exceeded`].
#[inline]
#[must_use]
pub(crate) const fn alias_count_exceeded(alias_count: usize, max_alias_expansions: usize) -> bool {
    alias_count > max_alias_expansions
}

/// The billion-laughs fingerprint: aliases vastly outnumbering anchors.
///
/// `None` disables the heuristic. A document with no anchors is treated
/// as having one, so the first alias of an anchorless document (already
/// an error elsewhere) still compares against `ratio` rather than
/// against zero. A non-finite `ratio` never trips: every comparison with
/// NaN is false, and a document cannot exceed infinity.
#[inline]
#[must_use]
pub(crate) fn alias_ratio_exceeded(
    alias_count: usize,
    anchor_count: usize,
    ratio: Option<f64>,
) -> bool {
    match ratio {
        None => false,
        Some(ratio) => {
            let anchors = anchor_count.max(1) as f64;
            (alias_count as f64) > ratio * anchors
        }
    }
}

/// The serde_yaml-profile transitive repetition budget.
///
/// Adds the expanded subtree's node count to the running charge and
/// refuses once the charge exceeds `events × factor`. Both operations
/// saturate, so no input can wrap the counters back under the limit.
/// Returns the new charge and whether the limit was exceeded.
#[inline]
#[must_use]
pub(crate) const fn jump_charge_exceeded(
    charge: usize,
    expanded_nodes: usize,
    event_count: usize,
    factor: usize,
) -> (usize, bool) {
    let charge = charge.saturating_add(expanded_nodes);
    (charge, charge > event_count.saturating_mul(factor))
}

/// Cumulative bytes produced by alias expansion, against both the
/// configured document length and the absolute ceiling.
///
/// Returns the new total and whether either limit was exceeded. The
/// addition saturates, so a total that would overflow is treated as
/// beyond every limit rather than wrapping to a small number.
#[inline]
#[must_use]
pub(crate) const fn alias_bytes_exceeded(
    alias_bytes: usize,
    expanded_bytes: usize,
    max_document_length: usize,
) -> (usize, bool) {
    let total = alias_bytes.saturating_add(expanded_bytes);
    (
        total,
        total > max_document_length || total > MAX_ALIAS_BYTES,
    )
}

/// `true` once `node_count` has exceeded `max_nodes`.
#[inline]
#[must_use]
pub(crate) const fn nodes_exceeded(node_count: usize, max_nodes: usize) -> bool {
    node_count > max_nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_is_exact_at_the_boundary() {
        assert!(!depth_exceeded(128, 128));
        assert!(depth_exceeded(129, 128));
    }

    #[test]
    fn ratio_treats_no_anchors_as_one() {
        assert!(!alias_ratio_exceeded(10, 0, Some(10.0)));
        assert!(alias_ratio_exceeded(11, 0, Some(10.0)));
        assert!(!alias_ratio_exceeded(usize::MAX, 0, None));
        assert!(!alias_ratio_exceeded(usize::MAX, 1, Some(f64::NAN)));
        assert!(!alias_ratio_exceeded(usize::MAX, 1, Some(f64::INFINITY)));
    }

    #[test]
    fn jump_charge_saturates_instead_of_wrapping() {
        let (charge, over) = jump_charge_exceeded(usize::MAX - 1, 5, 10, 100);
        assert_eq!(charge, usize::MAX);
        assert!(over);
        let (_, over) = jump_charge_exceeded(0, 1, usize::MAX, 2);
        assert!(!over);
    }

    #[test]
    fn alias_bytes_respect_both_ceilings() {
        assert!(!alias_bytes_exceeded(0, 10, 100).1);
        assert!(alias_bytes_exceeded(95, 10, 100).1);
        assert!(alias_bytes_exceeded(0, MAX_ALIAS_BYTES + 1, usize::MAX).1);
        assert!(alias_bytes_exceeded(usize::MAX, 1, usize::MAX).1);
    }
}

/// Machine-checked properties. `cargo kani -p noyalib` explores every
/// value of every symbolic input; a counterexample fails the build.
#[cfg(kani)]
mod proofs {
    use super::*;

    /// The depth check is exactly `depth > max`, and refusing at some
    /// depth means refusing at every greater depth.
    #[kani::proof]
    fn depth_exact_and_monotone() {
        let depth: usize = kani::any();
        let max: usize = kani::any();
        assert_eq!(depth_exceeded(depth, max), depth > max);
        if depth_exceeded(depth, max) && depth < usize::MAX {
            assert!(depth_exceeded(depth + 1, max));
        }
    }

    /// Same shape for the alias occurrence budget and the node ceiling.
    #[kani::proof]
    fn counts_exact_and_monotone() {
        let n: usize = kani::any();
        let max: usize = kani::any();
        assert_eq!(alias_count_exceeded(n, max), n > max);
        assert_eq!(nodes_exceeded(n, max), n > max);
        if n < usize::MAX {
            assert!(!alias_count_exceeded(n, max) || alias_count_exceeded(n + 1, max));
        }
    }

    /// The ratio heuristic never panics, never trips while disabled,
    /// never trips on a non-finite ratio, and never trips while aliases
    /// do not outnumber anchors at a ratio of at least one.
    #[kani::proof]
    fn ratio_is_safe_and_conservative() {
        let aliases: usize = kani::any();
        let anchors: usize = kani::any();
        let ratio: f64 = kani::any();
        assert!(!alias_ratio_exceeded(aliases, anchors, None));
        let _ = alias_ratio_exceeded(aliases, anchors, Some(ratio));
        if !ratio.is_finite() {
            assert!(!alias_ratio_exceeded(aliases, anchors, Some(ratio)));
        }
        kani::assume(aliases <= 1 << 20 && anchors <= 1 << 20);
        if ratio >= 1.0 && aliases <= anchors {
            assert!(!alias_ratio_exceeded(aliases, anchors, Some(ratio)));
        }
    }

    /// The transitive charge never wraps: the new charge is at least the
    /// old one, and once over the limit a larger addition is still over.
    #[kani::proof]
    fn jump_charge_never_wraps() {
        let charge: usize = kani::any();
        let add: usize = kani::any();
        let events: usize = kani::any();
        let factor: usize = kani::any();
        let (next, over) = jump_charge_exceeded(charge, add, events, factor);
        assert!(next >= charge);
        assert!(next >= add);
        if over && add < usize::MAX {
            assert!(jump_charge_exceeded(charge, add + 1, events, factor).1);
        }
    }

    /// Expanded bytes never wrap either, and both ceilings hold.
    #[kani::proof]
    fn alias_bytes_never_wrap() {
        let bytes: usize = kani::any();
        let add: usize = kani::any();
        let max_doc: usize = kani::any();
        let (total, over) = alias_bytes_exceeded(bytes, add, max_doc);
        assert!(total >= bytes && total >= add);
        if total > MAX_ALIAS_BYTES || total > max_doc {
            assert!(over);
        }
        if !over {
            assert!(total <= max_doc && total <= MAX_ALIAS_BYTES);
        }
    }
}
