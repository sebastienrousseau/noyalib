// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The stateful half of the budgets: one meter every loader charges.
//!
//! [`super::budget`] holds the limits as pure predicates. This module
//! holds the counters they are applied to, and the order they are
//! applied in, so the owned loaders, the streaming deserializer and the
//! borrowed builder cannot drift apart: each of them feeds its parser
//! events through [`Meter::charge_event`], every alias through
//! [`Meter::charge_alias`] and [`Meter::charge_expansion`], and every
//! merge key through [`Meter::charge_merge_key`]. A budget added here
//! is enforced on every path at once.
//!
//! The cost of an alias is measured by one estimator, [`AliasCost`],
//! whether the anchored node was kept as a [`Value`] (the loaders) or
//! as a recorded event buffer (the streaming deserializer).

use crate::error::{BudgetBreach, Error, Result};
use crate::parser::{Event, ParseConfig, budget};
use crate::prelude::*;
use crate::value::Value;

/// Bytes charged for each node an alias expansion materialises.
///
/// A [`Value`] is the unit of memory a loader allocates per node; a
/// mapping key is a `String` plus its index entry, which is smaller.
/// Charging every node at the size of a `Value` keeps the estimate at
/// or above what an expansion really allocates, so the alias-byte
/// budget bounds memory rather than a fraction of it.
pub(crate) const NODE_BYTES: usize = size_of::<Value>();

/// What expanding one alias costs, measured once on the anchored node.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct AliasCost {
    /// Nodes the expansion materialises, mapping keys included.
    pub(crate) nodes: usize,
    /// Scalar and key text bytes the expansion copies.
    pub(crate) text: usize,
    /// Collection nesting below the alias site: `0` for a scalar, `1`
    /// for a flat sequence or mapping.
    pub(crate) height: usize,
    /// The node count the serde_yaml-profile jump budget charges
    /// (`ParserConfig::alias_jump_event_factor`). Kept apart from
    /// `nodes` because that profile pins its own counting rule.
    pub(crate) jump: usize,
}

impl AliasCost {
    /// The estimated bytes of memory the expansion allocates.
    #[must_use]
    pub(crate) const fn bytes(&self) -> usize {
        self.nodes
            .saturating_mul(NODE_BYTES)
            .saturating_add(self.text)
    }

    /// Measure an anchored [`Value`], without recursion, so a value of
    /// any depth is measured on any stack.
    #[must_use]
    pub(crate) fn of_value(root: &Value) -> Self {
        let mut cost = Self::default();
        let mut pending: Vec<(&Value, usize)> = vec![(root, 0)];
        while let Some((value, level)) = pending.pop() {
            cost.nodes = cost.nodes.saturating_add(1);
            cost.jump = cost.jump.saturating_add(1);
            match value {
                Value::String(s) => cost.text = cost.text.saturating_add(s.len()),
                Value::Sequence(items) => {
                    cost.height = cost.height.max(level + 1);
                    pending.extend(items.iter().map(|item| (item, level + 1)));
                }
                Value::Mapping(map) => {
                    cost.height = cost.height.max(level + 1);
                    for (key, item) in map {
                        cost.nodes = cost.nodes.saturating_add(1);
                        cost.text = cost.text.saturating_add(key.len());
                        pending.push((item, level + 1));
                    }
                }
                Value::Tagged(tagged) => pending.push((tagged.value(), level)),
                Value::Null | Value::Bool(_) | Value::Number(_) => {}
            }
        }
        cost
    }
}

/// Measures a recorded event buffer the way [`AliasCost::of_value`]
/// measures a value: one node per scalar or collection start, scalar
/// text, and the deepest collection nesting.
///
/// An alias recorded inside the buffer costs nothing here. The
/// streaming deserializer replays it as its own alias, which is charged
/// when it is resolved, so the charges of one expansion add up to the
/// whole expanded tree without counting any part of it twice.
#[derive(Debug, Default)]
pub(crate) struct CostTally {
    cost: AliasCost,
    level: usize,
}

impl CostTally {
    /// A scalar of `len` bytes.
    pub(crate) fn scalar(&mut self, len: usize) {
        self.cost.nodes = self.cost.nodes.saturating_add(1);
        self.cost.jump = self.cost.jump.saturating_add(1);
        self.cost.text = self.cost.text.saturating_add(len);
    }

    /// A sequence or mapping opens.
    pub(crate) fn open(&mut self) {
        self.cost.nodes = self.cost.nodes.saturating_add(1);
        self.cost.jump = self.cost.jump.saturating_add(1);
        self.level += 1;
        self.cost.height = self.cost.height.max(self.level);
    }

    /// A sequence or mapping closes.
    pub(crate) fn close(&mut self) {
        self.level = self.level.saturating_sub(1);
    }

    /// The finished measurement.
    #[must_use]
    pub(crate) const fn finish(self) -> AliasCost {
        self.cost
    }
}

/// Running counters for one parse, charged in one fixed order.
#[derive(Debug, Default)]
pub(crate) struct Meter {
    events: usize,
    nodes: usize,
    scalar_bytes: usize,
    anchors: usize,
    documents: usize,
    aliases: usize,
    alias_bytes: usize,
    jump_charge: usize,
    merge_keys: usize,
}

impl Meter {
    /// Run the event policies and charge the per-event budgets: events,
    /// nodes, scalar bytes and anchors, then the document count when a
    /// document starts. Alias and merge counters are per document and
    /// restart there.
    pub(crate) fn charge_event(&mut self, event: &Event<'_>, config: &ParseConfig) -> Result<()> {
        if !config.policies.is_empty() {
            run_event_policies(event, &config.policies)?;
        }
        self.events = self.events.saturating_add(1);
        if self.events > config.max_events {
            return Err(Error::Budget(BudgetBreach::MaxEvents {
                limit: config.max_events,
                observed: self.events,
            }));
        }
        match event {
            Event::Scalar { value, anchor, .. } => {
                self.charge_node(anchor.is_some(), config)?;
                self.charge_scalar_bytes(value.len(), config)
            }
            Event::SequenceStart { anchor, .. } | Event::MappingStart { anchor, .. } => {
                self.charge_node(anchor.is_some(), config)
            }
            Event::DocumentStart => self.start_document(config),
            _ => Ok(()),
        }
    }

    fn charge_node(&mut self, anchored: bool, config: &ParseConfig) -> Result<()> {
        self.nodes = self.nodes.saturating_add(1);
        if budget::nodes_exceeded(self.nodes, config.max_nodes) {
            return Err(Error::Budget(BudgetBreach::MaxNodes {
                limit: config.max_nodes,
                observed: self.nodes,
            }));
        }
        if anchored {
            self.anchors = self.anchors.saturating_add(1);
        }
        Ok(())
    }

    fn charge_scalar_bytes(&mut self, len: usize, config: &ParseConfig) -> Result<()> {
        self.scalar_bytes = self.scalar_bytes.saturating_add(len);
        if self.scalar_bytes > config.max_total_scalar_bytes {
            return Err(Error::Budget(BudgetBreach::MaxTotalScalarBytes {
                limit: config.max_total_scalar_bytes,
                observed: self.scalar_bytes,
            }));
        }
        Ok(())
    }

    fn start_document(&mut self, config: &ParseConfig) -> Result<()> {
        self.aliases = 0;
        self.alias_bytes = 0;
        self.documents = self.documents.saturating_add(1);
        if self.documents > config.max_documents {
            return Err(Error::Budget(BudgetBreach::MaxDocuments {
                limit: config.max_documents,
                observed: self.documents,
            }));
        }
        Ok(())
    }

    /// Charge one alias occurrence: the expansion count, then the
    /// alias-to-anchor ratio. Runs before the anchor is looked up.
    pub(crate) fn charge_alias(&mut self, config: &ParseConfig) -> Result<()> {
        self.aliases = self.aliases.saturating_add(1);
        if budget::alias_count_exceeded(self.aliases, config.max_alias_expansions) {
            return Err(Error::RepetitionLimitExceeded);
        }
        if let Some(ratio) = config.alias_anchor_ratio {
            if budget::alias_ratio_exceeded(self.aliases, self.anchors, Some(ratio)) {
                return Err(Error::Budget(BudgetBreach::AliasAnchorRatio {
                    ratio,
                    anchors: self.anchors,
                    aliases: self.aliases,
                }));
            }
        }
        Ok(())
    }

    /// Charge what one alias expands to, before anything is copied: the
    /// bytes it allocates and the serde_yaml-profile jump charge.
    pub(crate) fn charge_expansion(
        &mut self,
        cost: &AliasCost,
        config: &ParseConfig,
    ) -> Result<()> {
        let (bytes, over) = budget::alias_bytes_exceeded(
            self.alias_bytes,
            cost.bytes(),
            config.max_document_length,
        );
        self.alias_bytes = bytes;
        if over {
            return Err(Error::RepetitionLimitExceeded);
        }
        if let Some(factor) = config.alias_jump_event_factor {
            let (charge, over) =
                budget::jump_charge_exceeded(self.jump_charge, cost.jump, self.events, factor);
            self.jump_charge = charge;
            if over {
                return Err(Error::RepetitionLimitExceeded);
            }
        }
        Ok(())
    }

    /// Charge one merge key (`<<`) that will be expanded.
    pub(crate) fn charge_merge_key(&mut self, config: &ParseConfig) -> Result<()> {
        self.merge_keys = self.merge_keys.saturating_add(1);
        if self.merge_keys > config.max_merge_keys {
            return Err(Error::Budget(BudgetBreach::MaxMergeKeys {
                limit: config.max_merge_keys,
                observed: self.merge_keys,
            }));
        }
        Ok(())
    }
}

/// Run every registered policy against this parser event; the first
/// policy to reject aborts the parse.
fn run_event_policies(
    event: &Event<'_>,
    policies: &[Arc<dyn crate::policy::Policy>],
) -> Result<()> {
    use crate::policy::{PolicyEvent, PolicyEventKind};
    let (kind, anchor, tag, scalar) = match event {
        Event::Scalar {
            value, anchor, tag, ..
        } => (
            PolicyEventKind::Scalar,
            anchor.as_deref(),
            tag,
            Some(value.as_ref()),
        ),
        Event::SequenceStart { anchor, tag, .. } => {
            (PolicyEventKind::SequenceStart, anchor.as_deref(), tag, None)
        }
        Event::MappingStart { anchor, tag, .. } => {
            (PolicyEventKind::MappingStart, anchor.as_deref(), tag, None)
        }
        // `Event::Alias.anchor` names the *target* anchor, not a fresh
        // definition, so it is surfaced as a pure Alias kind without an
        // `anchor` field: policies can tell "this node is anchored" from
        // "this node dereferences an existing anchor".
        Event::Alias { .. } => (PolicyEventKind::Alias, None, &None, None),
        _ => return Ok(()),
    };
    let tag = tag.as_ref().map(|(h, s)| format!("{h}{s}"));
    let projected = PolicyEvent {
        kind,
        anchor,
        tag: tag.as_deref(),
        scalar,
    };
    for p in policies {
        p.check_event(projected)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_and_event_measures_agree() {
        let value: Value = crate::from_str("a: [x, {b: yz}]\n").unwrap();
        let mut tally = CostTally::default();
        tally.open(); // {
        tally.scalar(1); // a
        tally.open(); // [
        tally.scalar(1); // x
        tally.open(); // {
        tally.scalar(1); // b
        tally.scalar(2); // yz
        tally.close();
        tally.close();
        tally.close();
        let from_events = tally.finish();
        let from_value = AliasCost::of_value(&value);
        assert_eq!(from_value.nodes, from_events.nodes);
        assert_eq!(from_value.text, from_events.text);
        assert_eq!(from_value.height, from_events.height);
        assert_eq!(from_value.height, 3);
    }

    #[test]
    fn a_scalar_has_no_height_and_costs_at_least_a_node() {
        let cost = AliasCost::of_value(&Value::String("abc".into()));
        assert_eq!(cost.height, 0);
        assert_eq!(cost.bytes(), NODE_BYTES + 3);
    }
}
