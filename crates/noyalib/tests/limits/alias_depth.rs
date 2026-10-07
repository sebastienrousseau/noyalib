// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! An alias cannot build a tree deeper than `max_depth`.
//!
//! Each anchor below nests the previous one inside `d` brackets, so
//! every line of the document is `d` deep and passes the depth check
//! where it is written; but each alias splices the whole earlier tree in,
//! so the expanded value is `d` times the chain length deep. Every entry
//! point must refuse it with `RecursionLimitExceeded`, and must do so on
//! a 1 MiB stack: a value that deep overflows any recursive walk,
//! including the drop at the end of parsing, and a stack overflow aborts
//! the whole process.

use std::collections::BTreeMap;

use noyalib::{Error, ParserConfig, Value};

/// `chain` anchors, each `d` brackets around an alias to the previous.
fn alias_chain(d: usize, chain: usize) -> String {
    let mut y = format!("a0: &a0 {}1{}\n", "[".repeat(d), "]".repeat(d));
    for i in 1..=chain {
        y.push_str(&format!(
            "a{i}: &a{i} {}*a{}{}\n",
            "[".repeat(d),
            i - 1,
            "]".repeat(d)
        ));
    }
    y.push_str(&format!("top: *a{chain}\n"));
    y
}

/// Run `f` on a thread with a 1 MiB stack, the size many runtimes and
/// wasm hosts give a worker.
fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("no panic")
}

fn is_depth_refusal(err: &Error) -> bool {
    matches!(err, Error::RecursionLimitExceeded { .. })
        || err.to_string().contains("recursion limit")
}

type EntryPoint = fn(&str) -> Result<(), Error>;

fn ok<T>(r: Result<T, Error>) -> Result<(), Error> {
    r.map(std::mem::forget)
}

#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct Typed {
    a0: Value,
    top: Value,
}

fn entry_points() -> Vec<(&'static str, EntryPoint)> {
    #[allow(unused_mut)] // the pushes below are feature-gated
    let mut eps: Vec<(&'static str, EntryPoint)> = vec![
        ("from_str<Value>", |s| ok(noyalib::from_str::<Value>(s))),
        ("from_str<BTreeMap<String, Value>>", |s| {
            ok(noyalib::from_str::<BTreeMap<String, Value>>(s))
        }),
        ("from_str<struct>", |s| ok(noyalib::from_str::<Typed>(s))),
        ("from_str_with_config<struct> (span-full loader)", |s| {
            let cfg =
                ParserConfig::default().with_policy(noyalib::policy::MaxScalarLength(1 << 20));
            ok(noyalib::from_str_with_config::<Typed>(s, &cfg))
        }),
        ("borrowed::from_str_borrowed", |s| {
            ok(noyalib::borrowed::from_str_borrowed(s))
        }),
        ("from_str_borrowing<BTreeMap<&str, Value>>", |s| {
            ok(noyalib::from_str_borrowing::<BTreeMap<&str, Value>>(s))
        }),
        ("load_all", |s| {
            ok(noyalib::load_all(s).map(|d| d.collect::<Vec<_>>()))
        }),
    ];
    #[cfg(feature = "parallel")]
    eps.push(("parallel::values", |s| {
        let stream = format!("---\n{s}").repeat(4);
        let pool = noyalib::parallel::ThreadPoolBuilder::new()
            .num_threads(2)
            .stack_size(1024 * 1024)
            .build()
            .expect("pool");
        ok(noyalib::parallel::values_with_config_in_pool(
            &stream,
            &ParserConfig::default(),
            &pool,
        ))
    }));
    #[cfg(feature = "compat-serde-yaml")]
    eps.push(("compat::serde_yaml::from_str<Value>", |s| {
        ok(noyalib::compat::serde_yaml::from_str::<Value>(s).map_err(|e| e.into_inner()))
    }));
    eps
}

#[test]
fn an_alias_chain_cannot_build_a_value_deeper_than_max_depth() {
    let yaml = alias_chain(100, 100);
    assert!(yaml.len() < 25_000, "{} bytes", yaml.len());
    for (name, f) in entry_points() {
        let input = yaml.clone();
        let result = on_small_stack(move || f(&input));
        let err = result.expect_err(name);
        assert!(is_depth_refusal(&err), "{name}: {err:?}");
    }
}

#[cfg(feature = "recovery")]
#[test]
fn lenient_recovery_survives_the_alias_chain_on_a_small_stack() {
    let yaml = alias_chain(100, 100);
    let result = on_small_stack(move || {
        let r = noyalib::recovery::parse_lenient(&yaml);
        let refused = r.errors.iter().any(is_depth_refusal);
        std::mem::forget(r);
        refused
    });
    assert!(result, "recovery must report the depth refusal");
}

/// The charge is exact: an alias that lands exactly at `max_depth` is
/// accepted, one level deeper is refused, on every loader.
#[test]
fn the_alias_depth_charge_is_exact_at_the_boundary() {
    let cfg = ParserConfig::default().max_depth(4);
    // `top` opens one level; the anchor is three deep: 1 + 3 = 4.
    let fits = "a: &a [[[1]]]\ntop: *a\n";
    // Inside one more bracket: 2 + 3 = 5.
    let over = "a: &a [[[1]]]\ntop: [*a]\n";
    for (yaml, fits_budget) in [(fits, true), (over, false)] {
        let owned = noyalib::from_str_with_config::<Value>(yaml, &cfg);
        let typed = noyalib::from_str_with_config::<BTreeMap<String, Value>>(yaml, &cfg);
        let borrowed = noyalib::borrowed::from_str_borrowed_with_config(yaml, &cfg);
        assert_eq!(owned.is_ok(), fits_budget, "owned {yaml:?}: {owned:?}");
        assert_eq!(typed.is_ok(), fits_budget, "typed {yaml:?}: {typed:?}");
        assert_eq!(
            borrowed.is_ok(),
            fits_budget,
            "borrowed {yaml:?}: {borrowed:?}"
        );
        if !fits_budget {
            assert!(matches!(
                owned,
                Err(Error::RecursionLimitExceeded { depth: 5 })
            ));
        }
    }
}

/// Ordinary aliases far from the limit are untouched.
#[test]
fn shallow_aliases_still_expand() {
    let yaml = alias_chain(3, 5);
    let v: Value = noyalib::from_str(&yaml).expect("parses");
    assert!(v.get("top").is_some());
    let b = noyalib::borrowed::from_str_borrowed(&yaml).expect("parses");
    assert_eq!(b.into_owned(), v);
}
