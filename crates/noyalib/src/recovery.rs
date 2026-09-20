// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Error-recovering YAML parser for LSP / IDE partial parsing.
//!
//! The default `from_str` family returns `Err` at the first
//! syntax violation. Language Server Protocol implementations
//! need the opposite contract: keep going past errors, build a
//! best-effort partial tree, and collect every error encountered
//! so the editor can show a complete diagnostics list and offer
//! autocomplete on the recoverable subtrees.
//!
//! `parse_lenient` is that contract:
//!
//! * Top-level `---` document boundaries are scanned first; each
//!   document is parsed independently so one broken document
//!   never prevents the others from being recovered.
//! * Within a single document, if the strict pass fails, the
//!   recoverer retries with [`DuplicateKeyPolicy::Last`] — the
//!   most common LSP-time error mode (a user typing a new key
//!   while an old one is still on screen). Successful recovery
//!   yields the post-retry value plus the original error in the
//!   error list, so the editor still flags the duplicate.
//! * If that retry also fails, the recoverer performs **line
//!   truncation recovery**: drop trailing lines one by one and
//!   re-parse until either a parse succeeds or the input is
//!   exhausted. The successful prefix becomes the recovered
//!   value; everything past the truncation point is summarised
//!   as a synthetic [`Value::Null`].
//! * A configurable error cap (`LenientConfig::max_errors`)
//!   stops further recovery once enough diagnostics have been
//!   collected — useful when the document is so malformed that
//!   every line errors.
//!
//! Gated behind the `recovery` Cargo feature.
//!
//! # Output shape
//!
//! For multi-document input the result's `ParseResult::value`
//! is a `Value::Sequence` of per-document values (recovered or
//! `Null`) — this matches what an LSP would walk to label
//! per-document diagnostics. For single-document input the
//! result's `value` is the recovered document directly (not
//! wrapped in a sequence).
//!
//! # Example
//!
//! ```
//! # #[cfg(feature = "recovery")] {
//! let yaml = "a: 1\nb: [unclosed\nc: 3\n";
//! let result = noyalib::recovery::parse_lenient(yaml);
//! assert!(!result.is_complete);
//! assert!(!result.errors.is_empty());
//! // `value` is the best-effort tree the recoverer salvaged.
//! # }
//! ```

use crate::de::{DuplicateKeyPolicy, ParserConfig, from_str_with_config};
use crate::error::Error;
use crate::value::Value;

/// Result of an error-recovering parse pass.
///
/// `value` is the best-effort tree the recoverer was able to
/// salvage from the input. `errors` lists every error the
/// recoverer encountered, in the order it found them.
/// `is_complete` is `true` when no errors were collected — the
/// input parsed cleanly on the first attempt.
#[derive(Debug)]
#[non_exhaustive]
pub struct ParseResult {
    /// Best-effort recovered value. For multi-document input
    /// this is a [`Value::Sequence`] of per-document values
    /// (recovered or [`Value::Null`]); for single-document input
    /// it is the recovered document directly.
    pub value: Value,
    /// Every error the recoverer encountered, in source order.
    pub errors: Vec<Error>,
    /// `true` when no errors were collected.
    pub is_complete: bool,
}

/// Knobs for the recovery passes.
///
/// Constructed via [`LenientConfig::default`]; tweak fields
/// inline. The struct is intentionally not marked
/// `#[non_exhaustive]` so callers can use struct-literal syntax
/// like `LenientConfig { max_errors: 50, ..Default::default() }`.
/// Adding a field is a semver-minor breaking change pre-1.0.
#[derive(Debug, Clone)]
pub struct LenientConfig {
    /// Stop collecting diagnostics once this many errors have
    /// been recorded. Defaults to `100`.
    pub max_errors: usize,
    /// When the strict parse fails, retry with
    /// [`DuplicateKeyPolicy::Last`] before giving up. Defaults to
    /// `true`.
    pub recover_duplicate_keys: bool,
    /// When both the strict and duplicate-key retries fail,
    /// drop trailing lines one by one and re-parse. Defaults to
    /// `true`.
    pub line_truncation: bool,
    /// Base parser configuration. Defaults to
    /// [`ParserConfig::default`].
    pub base_config: ParserConfig,
    /// Cumulative byte budget for line-truncation retries
    /// across one document. Each retry costs `prefix.len()`
    /// from this budget; when the next candidate prefix would
    /// exceed it, the recoverer stops salvaging and returns
    /// `Null`. Defaults to `1 MiB`, enough to retry a few
    /// hundred candidates on a typical LSP-edit buffer while
    /// bounding worst-case CPU on adversarial input.
    pub truncation_event_budget: usize,
}

impl Default for LenientConfig {
    fn default() -> Self {
        Self {
            max_errors: 100,
            recover_duplicate_keys: true,
            line_truncation: true,
            base_config: ParserConfig::default(),
            truncation_event_budget: 1024 * 1024,
        }
    }
}

/// Parse `input` with full error recovery.
///
/// Equivalent to [`parse_lenient_with`] with
/// [`LenientConfig::default`].
#[must_use]
pub fn parse_lenient(input: &str) -> ParseResult {
    parse_lenient_with(input, &LenientConfig::default())
}

/// Parse `input` with caller-supplied recovery knobs.
///
/// See the [module docs](self) for the recovery strategy.
///
/// Strips a leading UTF-8 BOM (`U+FEFF`) — Windows editors emit
/// one by default and recovery is the one entry point callers
/// expect to absorb it.
///
/// Hostile `---`-spam inputs are rejected at
/// [`ParserConfig::max_documents`]: the boundary scanner probes
/// one document beyond the cap instead of returning a truncated stream.
/// Per-document parsing then re-enforces every other
/// `ParserConfig` limit (`max_depth`, `max_events`,
/// `max_document_length`, …).
#[must_use]
pub fn parse_lenient_with(input: &str, config: &LenientConfig) -> ParseResult {
    // C5 — strip a leading BOM so Windows-saved buffers parse
    //      identically to the LF-on-Linux equivalent.
    let bom_skip = crate::doc_boundary::strip_bom(input.as_bytes());
    let input = &input[bom_skip..];

    let docs = match split_documents(input, &config.base_config) {
        Ok(docs) => docs,
        Err(error) => {
            return ParseResult {
                value: Value::Null,
                errors: vec![error],
                is_complete: false,
            };
        }
    };

    if docs.is_empty() {
        return ParseResult {
            value: Value::Null,
            errors: Vec::new(),
            is_complete: true,
        };
    }

    if docs.len() == 1 {
        let (value, errors) = recover_one(docs[0], config, config.max_errors);
        let is_complete = errors.is_empty();
        return ParseResult {
            value,
            errors,
            is_complete,
        };
    }

    let mut values: Vec<Value> = Vec::with_capacity(docs.len());
    let mut errors: Vec<Error> = Vec::new();
    let mut budget = config.max_errors;
    // M2 — preserve per-document index alignment for LSP
    //      diagnostic joiners by pushing `Null` for every
    //      document we skip after the budget runs out.
    let mut budget_exhausted = false;
    for doc in docs {
        if budget_exhausted {
            values.push(Value::Null);
            continue;
        }
        let (value, doc_errors) = recover_one(doc, config, budget);
        budget = budget.saturating_sub(doc_errors.len());
        errors.extend(doc_errors);
        values.push(value);
        if budget == 0 {
            budget_exhausted = true;
        }
    }
    let is_complete = errors.is_empty();
    ParseResult {
        value: Value::Sequence(values),
        errors,
        is_complete,
    }
}

/// Recover a single document.
///
/// Returns the best-effort `Value` and the list of errors
/// encountered (every pass that emitted an `Err` contributes one
/// entry). Bounded by `budget` — once exhausted, the recoverer
/// returns whatever it has and stops.
fn recover_one(input: &str, config: &LenientConfig, budget: usize) -> (Value, Vec<Error>) {
    if budget == 0 {
        return (Value::Null, Vec::new());
    }

    // Pass 1: strict.
    let strict_err = match from_str_with_config::<Value>(input, &config.base_config) {
        Ok(v) => return (v, Vec::new()),
        Err(e) => e,
    };
    let mut errors = vec![strict_err];

    // Pass 2: duplicate-key recovery via DuplicateKeyPolicy::Last.
    //
    // M13 — clone the base config exactly once per `recover_one`
    //       so per-document hot paths on LSP keystrokes don't pay
    //       the per-pass clone tax.
    let mut tweaked_cfg: Option<ParserConfig> = None;
    if config.recover_duplicate_keys
        && config.base_config.duplicate_key_policy != DuplicateKeyPolicy::Last
        && errors.len() < budget
    {
        let cfg2 = tweaked_cfg.insert({
            let mut c = config.base_config.clone();
            c.duplicate_key_policy = DuplicateKeyPolicy::Last;
            c
        });
        match from_str_with_config::<Value>(input, cfg2) {
            Ok(v) => return (v, errors),
            // M1 — collect the Pass-2 error too so the editor
            //      sees every diagnostic, not just the first.
            Err(e) => errors.push(e),
        }
    }

    // Pass 3: line-truncation recovery, bounded by the per-document
    // event budget so an adversarial 10k-line input cannot drive
    // O(N×max_events) re-parses (security finding C1).
    if config.line_truncation && errors.len() < budget {
        let pass3_cfg = tweaked_cfg.as_ref().unwrap_or(&config.base_config);
        match try_line_truncation(input, pass3_cfg, config.truncation_event_budget) {
            TruncationOutcome::Recovered(v) => return (v, errors),
            // M1 — collect the final truncation-failure error so
            //      the editor can show what went wrong after
            //      every salvage attempt was exhausted.
            TruncationOutcome::Exhausted(Some(e)) if errors.len() < budget => errors.push(e),
            TruncationOutcome::Exhausted(_) => {}
        }
    }

    (Value::Null, errors)
}

/// Result of the line-truncation pass.
enum TruncationOutcome {
    /// A truncated prefix parsed cleanly.
    Recovered(Value),
    /// No prefix parsed; the final attempt's error is carried back
    /// so [`recover_one`] can surface it as a diagnostic.
    Exhausted(Option<Error>),
}

/// Drop trailing lines one at a time, retrying the parse, until
/// a prefix succeeds, the cumulative parser-event budget is
/// exhausted, or no candidate prefixes remain.
///
/// `event_budget` caps how many bytes the recovery loop may
/// re-feed into the parser **in total**. Each attempted prefix
/// costs `prefix.len()` from the budget; this turns a hostile
/// 10k-line input from O(N×input_len) into bounded work without
/// regressing recovery quality on realistic LSP-edit inputs.
///
/// Honours M3 — the buffer end itself is a candidate cut so a
/// malformed last line without a trailing newline (the universal
/// mid-typing case) is still tried.
fn try_line_truncation(
    input: &str,
    config: &ParserConfig,
    event_budget: usize,
) -> TruncationOutcome {
    // Collect line boundaries; the buffer end is a synthetic
    // candidate so the no-trailing-newline case is exercised.
    let mut boundaries: Vec<usize> = Vec::new();
    for (i, b) in input.as_bytes().iter().enumerate() {
        if *b == b'\n' {
            boundaries.push(i);
        }
    }
    if boundaries.last().copied() != Some(input.len()) {
        boundaries.push(input.len());
    }

    let mut budget_remaining = event_budget;
    let mut last_err: Option<Error> = None;
    for &cut in boundaries.iter().rev() {
        let candidate = &input[..cut];
        if candidate.trim().is_empty() {
            continue;
        }
        // Budget gate: re-parsing `candidate.len()` bytes costs
        // proportionally; saturating-sub avoids panic on overflow.
        let cost = candidate.len();
        if cost > budget_remaining {
            break;
        }
        budget_remaining = budget_remaining.saturating_sub(cost);
        match from_str_with_config::<Value>(candidate, config) {
            Ok(v) => return TruncationOutcome::Recovered(v),
            Err(e) => last_err = Some(e),
        }
    }
    TruncationOutcome::Exhausted(last_err)
}

/// Split `input` on top-level YAML `---` document markers.
///
/// Thin wrapper around [`crate::doc_boundary::split_documents`]
/// that bounds the marker cap by [`crate::de::ParserConfig::max_documents`].
///
/// Hostile `---`-spam inputs cannot drive unbounded `Vec`
/// growth because the underlying scanner stops after
/// `max_markers` boundaries.
fn split_documents<'a>(input: &'a str, config: &ParserConfig) -> crate::Result<Vec<&'a str>> {
    crate::doc_boundary::split_documents_checked(input, config.max_documents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_input_is_complete() {
        let r = parse_lenient("a: 1\nb: 2\n");
        assert!(r.is_complete);
        assert!(r.errors.is_empty());
        let m = r.value.as_mapping().unwrap();
        assert!(m.contains_key("a"));
        assert!(m.contains_key("b"));
    }

    #[test]
    fn empty_input_is_complete() {
        let r = parse_lenient("");
        assert!(r.is_complete);
        assert!(r.errors.is_empty());
        assert!(matches!(r.value, Value::Null));
    }

    #[test]
    fn duplicate_key_is_recovered() {
        // The recovery pass kicks in only when the base config
        // is strict about duplicate keys; the workspace default
        // (DuplicateKeyPolicy::Last) silently accepts them.
        let cfg = LenientConfig {
            base_config: ParserConfig::default().duplicate_key_policy(DuplicateKeyPolicy::Error),
            ..LenientConfig::default()
        };
        let r = parse_lenient_with("a: 1\na: 2\n", &cfg);
        assert!(!r.is_complete);
        assert_eq!(r.errors.len(), 1);
        let m = r.value.as_mapping().unwrap();
        let v = m.get("a").unwrap();
        assert_eq!(v.as_i64(), Some(2));
    }

    #[test]
    fn unrecoverable_input_yields_null_with_errors() {
        // `[` opens a flow sequence that never closes — no
        // truncation makes this valid.
        let r = parse_lenient("[\n");
        assert!(!r.is_complete);
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn line_truncation_recovers_trailing_garbage() {
        // First two lines parse; third line is malformed flow.
        let r = parse_lenient("a: 1\nb: 2\nc: [unclosed\n");
        assert!(!r.is_complete);
        // The recoverer should salvage at least the strict-error
        // for the malformed third line.
        assert!(!r.errors.is_empty());
        // Best-effort tree: should contain `a` (and may contain `b`).
        if let Value::Mapping(m) = &r.value {
            assert!(m.contains_key("a"));
        }
    }

    #[test]
    fn multi_doc_recovers_each_independently() {
        let yaml = "---\na: 1\n---\nb: [unclosed\n---\nc: 3\n";
        let r = parse_lenient(yaml);
        assert!(!r.is_complete);
        let seq = match &r.value {
            Value::Sequence(s) => s,
            _ => panic!("expected sequence for multi-doc input"),
        };
        assert_eq!(seq.len(), 3);
        // Docs 0 and 2 should recover; doc 1 is the bad one.
        assert!(matches!(&seq[0], Value::Mapping(_)));
        assert!(matches!(&seq[2], Value::Mapping(_)));
    }

    #[test]
    fn max_errors_caps_collection() {
        let cfg = LenientConfig {
            max_errors: 1,
            ..LenientConfig::default()
        };
        let yaml = "---\na: [bad\n---\nb: [bad\n---\nc: [bad\n";
        let r = parse_lenient_with(yaml, &cfg);
        assert!(r.errors.len() <= 1);
    }

    #[test]
    fn split_documents_handles_single() {
        let d = split_documents("a: 1\n", &ParserConfig::default()).unwrap();
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn split_documents_handles_empty() {
        let cfg = ParserConfig::default();
        assert!(split_documents("", &cfg).unwrap().is_empty());
        assert!(split_documents("   \n", &cfg).unwrap().is_empty());
    }

    #[test]
    fn recover_disabled_passes_just_collect_errors() {
        // With both recovery passes disabled, an invalid input
        // should still produce Null + the strict error — exercises
        // the "fall through every pass" branch in recover_one.
        let cfg = LenientConfig {
            recover_duplicate_keys: false,
            line_truncation: false,
            ..LenientConfig::default()
        };
        let r = parse_lenient_with("[unclosed", &cfg);
        assert!(!r.is_complete);
        assert_eq!(r.errors.len(), 1);
        assert!(matches!(r.value, Value::Null));
    }

    #[test]
    fn line_truncation_disabled_skips_third_pass() {
        let cfg = LenientConfig {
            line_truncation: false,
            ..LenientConfig::default()
        };
        let r = parse_lenient_with("a: 1\nb: [bad\n", &cfg);
        assert!(!r.is_complete);
        // With truncation off, no salvage attempt — value stays Null.
        assert!(matches!(r.value, Value::Null));
    }

    #[test]
    fn config_is_debug_and_clone() {
        // Cheap reflection — keeps Debug + Clone derives covered.
        let cfg = LenientConfig::default();
        let _printed = format!("{cfg:?}");
        let cloned = cfg.clone();
        assert_eq!(cloned.max_errors, cfg.max_errors);
    }

    #[test]
    fn parse_result_is_debug() {
        let r = parse_lenient("a: 1\n");
        let _printed = format!("{r:?}");
    }

    #[test]
    fn split_documents_handles_implicit_first_doc() {
        // Content before the first `---` is an implicit doc.
        let d = split_documents("name: pre\n---\nname: post\n", &ParserConfig::default()).unwrap();
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn split_documents_ignores_mid_line_dashes() {
        // `---` mid-line is not a document marker.
        let d = split_documents("a: ---\nb: 2\n", &ParserConfig::default()).unwrap();
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn crlf_input_recovers_cleanly() {
        // Windows-saved buffer with `\r\n` line endings.
        let r = parse_lenient("a: 1\r\nb: 2\r\n");
        assert!(r.is_complete);
        if let Value::Mapping(m) = &r.value {
            assert!(m.contains_key("a"));
            assert!(m.contains_key("b"));
        } else {
            panic!("expected mapping for CRLF input, got {:?}", r.value);
        }
    }

    #[test]
    fn bom_prefix_is_stripped() {
        let r = parse_lenient("\u{FEFF}a: 1\nb: 2\n");
        assert!(r.is_complete);
        if let Value::Mapping(m) = &r.value {
            assert!(m.contains_key("a"));
        } else {
            panic!("BOM-prefixed input should parse cleanly");
        }
    }

    #[test]
    fn marker_spam_is_bounded() {
        // 10k `---\n` markers in a row must produce one bounded
        // budget diagnostic, never a silently truncated result.
        let yaml = "---\n".repeat(10_000);
        let r = parse_lenient(&yaml);
        assert!(matches!(r.value, Value::Null));
        assert!(matches!(
            r.errors.as_slice(),
            [Error::Budget(crate::BudgetBreach::MaxDocuments {
                limit: 1000,
                ..
            })]
        ));
    }

    #[test]
    fn truncation_handles_no_trailing_newline() {
        // M3 — the prefix `"a: 1\nb: ["` ends without `\n`; the
        //      fix treats the buffer end as a truncation
        //      candidate so the prefix `"a: 1\n"` is salvaged.
        let r = parse_lenient("a: 1\nb: [bad");
        if let Value::Mapping(m) = &r.value {
            assert_eq!(m.get("a").and_then(|v| v.as_i64()), Some(1));
        }
    }

    #[test]
    fn budget_exhaustion_preserves_indices() {
        // M2 — when the budget runs out mid-stream, remaining
        //      docs become Null (not dropped) so per-doc
        //      diagnostic indices still line up.
        let cfg = LenientConfig {
            max_errors: 1,
            ..LenientConfig::default()
        };
        let yaml = "---\na: [bad\n---\nb: [bad\n---\nc: [bad\n";
        let r = parse_lenient_with(yaml, &cfg);
        if let Value::Sequence(s) = &r.value {
            assert_eq!(s.len(), 3);
        } else {
            panic!("expected sequence with all 3 indices preserved");
        }
    }

    #[test]
    fn truncation_budget_caps_retries() {
        // C1 — adversarial 10k-line input cannot drive unbounded
        //      re-parses. With a tiny truncation budget the
        //      recoverer gives up early but does not hang.
        let cfg = LenientConfig {
            truncation_event_budget: 64,
            ..LenientConfig::default()
        };
        // 10k malformed lines after one valid line.
        let mut yaml = String::from("a: 1\n");
        for _ in 0..10_000 {
            yaml.push_str("[bad\n");
        }
        let _r = parse_lenient_with(&yaml, &cfg);
        // No panic, no hang.
    }
}
