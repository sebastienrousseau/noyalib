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
            am.len() == bm.len()
                && am
                    .iter()
                    .all(|(k, v)| bm.get(k).is_some_and(|w| numeric_equal(v, w)))
        }
        _ => a == b,
    }
}
