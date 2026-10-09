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

    if !numeric_equal(&noya, &syml) && !anchor_name_outside_libyaml(s)
        && !flow_entry_starts_with_indicator(s) {
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
        (V::Object(am), V::Object(bm)) => object_equal(am, bm),
        // Known serde_yaml_ng quirk: a comment-shaped line inside a
        // block scalar's content is stripped as if it were a comment,
        // where the spec reads it as content (`>\n#` is the folded
        // scalar "#\n"; auto-detected indent 0 is valid content for a
        // root node), and ng ends the scalar there. noyalib's reading
        // is pinned in tests/competitor_bugs.rs.
        (V::String(a), V::String(b)) if cut_at_comment_line(a, b) || cut_at_comment_line(b, a) => {
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
        // Known divergence: a core-schema float literal that overflows
        // f64 (`3e999`) is infinity in noyalib, which JSON can only hold
        // as null; serde_yaml_ng keeps the text as a string.
        (V::Null, V::String(s)) | (V::String(s), V::Null)
            if s.parse::<f64>().is_ok_and(f64::is_infinite) =>
        {
            true
        }
        // Known resolver-scheme divergence: integer spellings only the
        // 1.1-flavoured resolver reads as numbers; see `is_yaml_11_int`.
        (V::Number(_), V::String(s)) | (V::String(s), V::Number(_)) if is_yaml_11_int(s) => true,
        _ => a == b,
    }
}

/// Mapping equality under `numeric_equal`.
fn object_equal(
    am: &serde_json::Map<String, serde_json::Value>,
    bm: &serde_json::Map<String, serde_json::Value>,
) -> bool {
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
            .all(|(k, v)| lookup_key(bm, k).is_some_and(|w| numeric_equal(v, w)))
}

/// Look `k` up in `m`, tolerating two known serde_yaml_ng quirks on
/// keys: it reads `?foo` as an explicit-key indicator and drops the
/// `?`, where YAML 1.2 (ns-plain-first; yaml-test-suite 652Z) reads
/// the plain key "?foo"; and a block-scalar key ending the input loses
/// its clipped final line break. noyalib's readings are pinned in
/// tests/regressions/competitor_bugs.rs.
fn lookup_key<'m>(
    m: &'m serde_json::Map<String, serde_json::Value>,
    k: &str,
) -> Option<&'m serde_json::Value> {
    m.get(k)
        .or_else(|| k.strip_prefix('?').and_then(|s| m.get(s)))
        .or_else(|| m.get(&format!("?{k}")))
        // The block-scalar clip quirk (see `numeric_equal`) on a key.
        .or_else(|| k.strip_suffix('\n').and_then(|s| m.get(s)))
        .or_else(|| m.get(&format!("{k}\n")))
}

/// Known serde_yaml_ng quirk: like libyaml, it allows only
/// `[A-Za-z0-9_-]` in an anchor name and ends the name at anything
/// else, so `&a:` becomes a mapping with an empty key and `&r??` the
/// string "??". YAML 1.2 §6.9.2 allows any ns-char except the flow
/// indicators, so noyalib anchors an empty node named `a:` or `r??`.
/// The divergence is structural, so such inputs are dropped rather
/// than modelled. noyalib's reading is pinned in
/// tests/regressions/competitor_bugs.rs.
fn anchor_name_outside_libyaml(s: &str) -> bool {
    s.split('&').skip(1).any(|rest| {
        rest.chars()
            .take_while(|c| !c.is_whitespace() && !",[]{}".contains(*c))
            .any(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
    })
}

/// Known serde_yaml_ng quirks at the start of a flow entry: it reads
/// a `?` followed by a non-space as the explicit-key indicator, so
/// `[?x]` becomes `[{"x": null}]` where YAML 1.2 (ns-plain-first;
/// yaml-test-suite 652Z) reads the plain scalar "?x"; and it cannot
/// read an entry whose key is empty (`[: x]`, `[?, :]`), which the
/// spec allows (yaml-test-suite CFD4). A flow mapping key with a `?`
/// is tolerated by `lookup_key`; any other such entry changes the
/// shape, so those inputs are dropped.
fn flow_entry_starts_with_indicator(s: &str) -> bool {
    // Look back past comments as well as white space.
    let stripped = without_comments(s);
    let b = stripped.as_bytes();
    b.iter().enumerate().any(|(i, &c)| {
        let question = c == b'?' && b.get(i + 1).is_some_and(|n| !n.is_ascii_whitespace());
        (question || c == b':')
            && b[..i]
                .iter()
                .rev()
                .find(|p| !p.is_ascii_whitespace())
                .is_some_and(|p| matches!(p, b'[' | b'{' | b','))
    })
}

/// An integer spelling YAML 1.2's core schema leaves a string and
/// serde_yaml_ng's 1.1-flavoured resolver reads as a number: a signed
/// radix integer (`+0x1F`, `-0o17`; the core hex and octal patterns
/// carry no sign) or any binary literal (`0b11`, `-0b0`; the core
/// schema has no binary form). noyalib's reading of the binary form is
/// pinned in tests/regressions/competitor_bugs.rs.
fn is_yaml_11_int(s: &str) -> bool {
    let unsigned = s.strip_prefix(['-', '+']);
    unsigned.is_some_and(|r| r.starts_with("0x") || r.starts_with("0o"))
        || unsigned.unwrap_or(s).starts_with("0b")
}

/// `s` with every comment (`#` at a line start or after white space,
/// to the end of the line) blanked to spaces, offsets unchanged.
fn without_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_comment = false;
    let mut prev_blank = true;
    for c in s.chars() {
        if c == '\n' || c == '\r' {
            in_comment = false;
        } else if c == '#' && prev_blank {
            in_comment = true;
        }
        prev_blank = c.is_whitespace();
        out.push(if in_comment { ' ' } else { c });
    }
    out
}

/// `cut` is `full` up to the start of a line that begins with `#`,
/// less any line breaks just before that line: the shape serde_yaml_ng
/// leaves when it reads that line of a block scalar as a comment and
/// drops it with everything after it (its chomping then trims the
/// breaks of a scalar left with no text).
fn cut_at_comment_line(full: &str, cut: &str) -> bool {
    full.match_indices('#').any(|(p, _)| {
        (p == 0 || full.as_bytes()[p - 1] == b'\n')
            && full[..p]
                .strip_prefix(cut)
                .is_some_and(|gap| gap.bytes().all(|b| b == b'\n'))
    })
}

/// `[-+]?0[0-9]+` — a decimal integer spelling with a leading zero.
fn is_leading_zero_int(s: &str) -> bool {
    let digits = s.strip_prefix(['-', '+']).unwrap_or(s);
    digits.len() >= 2 && digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit())
}
