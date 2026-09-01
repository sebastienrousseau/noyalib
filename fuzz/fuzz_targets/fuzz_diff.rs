//! Differential fuzz target: parse the same YAML through noyalib,
//! `serde_yaml_ng`, and `saphyr`, and flag *valid divergences* —
//! cases where every parser says "yes, this is YAML" but they
//! produce different `Value` shapes.
//!
//! Crash-free is the bar for the other fuzz targets; this target is
//! about *correctness alignment* with the de-facto Rust YAML
//! ecosystem. A divergence is not necessarily a noyalib bug —
//! noyalib is the most spec-compliant of the three, and `saphyr` /
//! `serde_yaml_ng` have known historical quirks. But every
//! divergence is data: it surfaces either a noyalib regression, a
//! competitor bug, or a spec-corner the test corpus has not yet
//! covered.
//!
//! Inputs that any of the parsers reject are dropped — we only
//! diff the cases all three accept.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    // Bound the input — avoid pathological inputs that consume all
    // CPU on one of the parsers and starve the campaign.
    if s.len() > 4096 {
        return;
    }

    let Ok(noya) = noyalib::from_str::<serde_json::Value>(s) else {
        return;
    };
    let Ok(syml) = serde_yaml_ng::from_str::<serde_json::Value>(s) else {
        return;
    };
    // saphyr returns its own value type; compare via JSON to put all
    // three on the same axis.
    let Ok(saph_str) =
        std::panic::catch_unwind(|| match {
                use saphyr::LoadableYamlNode as _;
                saphyr::Yaml::load_from_str(s)
            } {
            Ok(docs) => Some(format!("{:?}", docs)),
            Err(_) => None,
        })
    else {
        return;
    };

    if !numeric_equal(&noya, &syml) {
        // serde_yaml_ng vs noyalib divergence — abort so libfuzzer
        // saves the input as a unique crash artefact.
        let n = serde_json::to_string(&noya).unwrap_or_default();
        let y = serde_json::to_string(&syml).unwrap_or_default();
        panic!(
            "noyalib != serde_yaml_ng on input bytes (len {}):\n  noyalib    : {}\n  serde_yaml : {}",
            s.len(),
            n,
            y
        );
    }
    let _ = saph_str; // keep saphyr load result alive; expand the
                     // saphyr<->noyalib comparison once the
                     // saphyr→serde_json bridge lands.
});

/// JSON-Value equality that treats `Number(450.0) == Number(450)`.
/// YAML's core schema resolves `450.00` as a float; competing
/// libraries differ on whether `450` parses to an int or a float.
/// The core question we're after — "do they agree on the data" —
/// should not flip on that representational difference alone.
fn numeric_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value as V;
    match (a, b) {
        (V::Number(an), V::Number(bn)) => an.as_f64() == bn.as_f64(),
        (V::Array(av), V::Array(bv)) => {
            av.len() == bv.len()
                && av.iter().zip(bv.iter()).all(|(x, y)| numeric_equal(x, y))
        }
        (V::Object(am), V::Object(bm)) => {
            // Known policy divergence: `<<` merge keys. noyalib
            // resolves the merge (configurably; extensively covered
            // by its own merge_key test suites); serde_yaml_ng keeps
            // the literal entry. Any mapping where either side still
            // carries a `<<` key is in that divergent territory, so
            // it is excluded from the diff rather than half-modelled
            // here.
            if am.contains_key("<<") || bm.contains_key("<<") {
                return true;
            }
            am.len() == bm.len()
                && am
                    .iter()
                    .all(|(k, v)| bm.get(k).is_some_and(|w| numeric_equal(v, w)))
        }
        // Known serde_yaml_ng quirk: a comment-shaped line inside a
        // block scalar's content is stripped as if it were a comment,
        // where the spec reads it as content (`>\n#` is the folded
        // scalar "#\n"; auto-detected indent 0 is valid content for a
        // root node). noyalib's reading is pinned in
        // tests/competitor_bugs.rs.
        (V::String(a), V::String(b))
            if (a.is_empty() && comment_shaped(b)) || (b.is_empty() && comment_shaped(a)) =>
        {
            true
        }
        // Known serde_yaml_ng quirk: block-scalar chomping — the
        // default "clip" keeps the final line break (`>\n &` is the
        // folded scalar "&\n"); ng drops it. Same text either side of
        // one trailing newline is that divergence, not disagreement
        // about the data.
        (V::String(a), V::String(b))
            if a.strip_suffix('\n') == Some(b) || b.strip_suffix('\n') == Some(a) =>
        {
            true
        }
        // Known resolver-scheme divergence: a leading-zero integer
        // spelling (`02`, `-007`). YAML 1.2's core schema resolves it
        // as a decimal integer (noyalib, correctly); serde_yaml_ng's
        // 1.1-flavoured resolver keeps it a string to dodge the 1.1
        // octal ambiguity.
        (V::Number(n), V::String(s)) | (V::String(s), V::Number(n))
            if is_leading_zero_int(s) =>
        {
            // Same digits, two readings: allowed only when the string
            // spelling parses to exactly the number the other side
            // resolved (f64 parsing accepts the leading zeros).
            s.parse::<f64>().ok() == n.as_f64()
        }
        // Known resolver-scheme divergence: a *signed* radix integer
        // (`+0x1F`, `-0o17`). YAML 1.2's core schema has no sign in
        // its hex/octal patterns, so noyalib reads a string; the
        // 1.1-flavoured resolver accepts the sign and reads a number.
        (V::Number(_), V::String(s)) | (V::String(s), V::Number(_))
            if s.strip_prefix(['-', '+'])
                .is_some_and(|r| r.starts_with("0x") || r.starts_with("0o")) =>
        {
            true
        }
        _ => a == b,
    }
}

/// Every non-empty line starts with `#` — the shape serde_yaml_ng
/// strips out of block-scalar content as if it were comments.
fn comment_shaped(s: &str) -> bool {
    !s.is_empty() && s.lines().all(|l| l.is_empty() || l.starts_with('#'))
}

/// `[-+]?0[0-9]+` — a decimal integer spelling with a leading zero.
fn is_leading_zero_int(s: &str) -> bool {
    let digits = s.strip_prefix(['-', '+']).unwrap_or(s);
    digits.len() >= 2 && digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit())
}
