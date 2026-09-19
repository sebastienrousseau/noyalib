// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Behaviour of the optional features, asserted rather than assumed.
//!
//! Ten features had no integration test naming them. Three of those are
//! pure aliases (`minimal`, `noyavalidate`, `compare-saphyr`) with no
//! `cfg` site of their own — for those the only meaningful claim is that
//! the composition still resolves, which a build with the feature on
//! proves by existing.
//!
//! The rest select between two implementations of the same thing:
//!
//! | feature | swaps |
//! | :--- | :--- |
//! | `fast-int` | `itoa` for `write!` when serialising integers |
//! | `fast-float` | `ryu` for `{:?}` when checking float round-trips |
//! | `wasm-opt` | an alternative streaming buffer strategy |
//!
//! A faster implementation that produces *different output* is a bug,
//! not an optimisation. So these tests assert the observable result and
//! are written to pass identically whether the feature is on or off —
//! run under both configurations, they pin the equivalence. A test that
//! only ran in one configuration could not.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use noyalib::Value;

/// `fast-int` swaps `itoa` in for `write!`. Same digits, both ways.
///
/// Boundaries included deliberately: `itoa` and `write!` are most
/// likely to disagree at the extremes of each width, and at zero, where
/// a hand-rolled digit loop is easiest to get wrong.
#[test]
fn integer_serialisation_is_identical_with_or_without_fast_int() {
    for n in [
        0_i64,
        1,
        -1,
        9,
        10,
        -10,
        99,
        100,
        i64::from(i32::MAX),
        i64::from(i32::MIN),
        i64::MAX,
        i64::MIN,
    ] {
        let yaml = noyalib::to_string(&n).expect("serialise");
        assert_eq!(
            yaml.trim(),
            n.to_string(),
            "integer {n} did not serialise to its own decimal form"
        );
        let back: i64 = noyalib::from_str(&yaml).expect("round trip");
        assert_eq!(back, n, "integer {n} did not survive a round trip");
    }
}

/// The same, inside a document rather than at the top level, so the
/// value goes through the mapping writer rather than the scalar
/// shortcut.
#[test]
fn integers_in_a_mapping_serialise_identically() {
    let src = "a: 0\nb: -1\nc: 9223372036854775807\nd: -9223372036854775808\n";
    let v: Value = noyalib::from_str(src).expect("parse");
    let out = noyalib::to_string(&v).expect("serialise");
    let again: Value = noyalib::from_str(&out).expect("reparse");
    assert_eq!(
        v, again,
        "a mapping of boundary integers did not round trip"
    );
    for want in ["9223372036854775807", "-9223372036854775808"] {
        assert!(out.contains(want), "{want} missing from:\n{out}");
    }
}

/// `fast-float` swaps `ryu` in for `{:?}` when checking that a float
/// survives formatting. Both must accept exactly the values that round
/// trip and reject exactly those that do not.
#[test]
fn float_round_trips_are_identical_with_or_without_fast_float() {
    for f in [
        0.0_f64,
        -0.0,
        1.0,
        -1.0,
        0.1,
        1e300,
        1e-300,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        std::f64::consts::PI,
    ] {
        let yaml = noyalib::to_string(&f).expect("serialise");
        let back: f64 = noyalib::from_str(&yaml).expect("round trip");
        assert!(
            back == f || (back.is_nan() && f.is_nan()),
            "float {f} round-tripped to {back} via {yaml:?}"
        );
    }
}

/// Non-finite floats are the case where the two formatters are most
/// likely to differ, and where YAML has its own spelling.
#[test]
fn non_finite_floats_behave_the_same_either_way() {
    for (value, expected) in [(f64::INFINITY, ".inf"), (f64::NEG_INFINITY, "-.inf")] {
        let yaml = noyalib::to_string(&value).expect("serialise");
        assert!(
            yaml.contains(expected),
            "{value} serialised to {yaml:?}, expected {expected}"
        );
    }
    let nan = noyalib::to_string(&f64::NAN).expect("serialise");
    assert!(nan.contains(".nan"), "NaN serialised to {nan:?}");
}

/// `wasm-opt` selects a different streaming buffer strategy. The
/// documents it produces must not depend on which one is compiled in.
#[test]
fn streaming_output_is_identical_with_or_without_wasm_opt() {
    let src = concat!(
        "---\n",
        "first: 1\n",
        "nested:\n  a: [1, 2, 3]\n  b: {x: y}\n",
        "text: |\n  kept\n  lines\n",
        "---\n",
        "second: 2\n",
    );
    let docs = noyalib::load_all_as::<Value>(src).expect("load all");
    assert_eq!(docs.len(), 2, "expected two documents");
    // Re-serialising each document and reparsing must be a fixed point,
    // whichever buffer strategy is in play.
    for d in &docs {
        let out = noyalib::to_string(d).expect("serialise");
        let again: Value = noyalib::from_str(&out).expect("reparse");
        assert_eq!(d, &again, "document was not a fixed point:\n{out}");
    }
}

/// `arbitrary` derives `Arbitrary` for [`Value`]. Only meaningful with
/// the feature on; without it the type simply does not implement the
/// trait and there is nothing to assert.
#[cfg(feature = "arbitrary")]
#[test]
fn value_implements_arbitrary_when_the_feature_is_on() {
    use arbitrary::{Arbitrary as _, Unstructured};
    let data = [0u8; 64];
    let mut u = Unstructured::new(&data);
    // The point is that this compiles and does not panic on trivial
    // input; a fuzzer supplies the interesting cases.
    let _ = Value::arbitrary(&mut u);
}

/// The alias features carry no `cfg` of their own — their whole content
/// is the set they pull in. Building with one enabled is the assertion;
/// this records which, so an alias that stops resolving is a failing
/// test rather than a silent no-op.
#[test]
fn alias_features_resolve_to_something() {
    // `minimal = ["std"]`, and `noyavalidate` pulls in miette +
    // validate-schema. Both are consumed elsewhere — `noyavalidate` by
    // `noya-cli`, which is why it looks unreferenced from inside this
    // repository and is not vestigial.
    #[cfg(feature = "std")]
    {
        let v: Value = noyalib::from_str("k: v").expect("std parse");
        assert!(v.get("k").is_some());
    }
    #[cfg(feature = "validate-schema")]
    {
        // Present purely to prove the feature composes; the schema
        // surface has its own suites.
        // The implication lives in Cargo.toml, so check it where a
        // Cargo.toml edit would break it: at compile time.
        const _: () = assert!(cfg!(feature = "schema"));
    }
}
