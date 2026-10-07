// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Cross-path parity for every budget and policy, on every entry point.
//!
//! A typed target with the default configuration is served by the
//! streaming deserializer; a `Value` target is served by the loaders.
//! Until v0.0.53 the streaming path never charged `max_events`,
//! `max_nodes`, `max_total_scalar_bytes`, `max_merge_keys` or the
//! alias ratio, so tightening any of them had no effect on the common
//! case. The first tests here drive the same input through both paths
//! and assert the same `BudgetBreach`.
//!
//! The table below generalises that: every public entry point that
//! reads YAML text, crossed with every limit and policy, each on a
//! hostile input built to break exactly that limit. Every cell must
//! refuse, for the right reason. `every_public_entry_point_has_a_row`
//! scans the crate's source for public parse functions, so an entry
//! point added without a row (or an explicit, reasoned exemption) fails
//! CI instead of shipping without enforcement.

#![allow(missing_docs)]

use std::collections::HashMap;

use noyalib::{BudgetBreach, Error, ParserConfig, Value, from_str_with_config};

#[derive(serde::Deserialize, Debug)]
struct Records {
    items: Vec<HashMap<String, String>>,
}

#[derive(serde::Deserialize, Debug)]
struct Merged {
    base: HashMap<String, i64>,
    items: Vec<HashMap<String, i64>>,
}

#[derive(serde::Deserialize, Debug)]
struct Aliased {
    base: i64,
    items: Vec<i64>,
}

/// `n` single-key records: about `2n + 6` parser events and `2n + 3`
/// nodes, so a limit of 50 trips well before the end for `n = 200`.
fn records(n: usize) -> String {
    let mut y = String::from("items:\n");
    for i in 0..n {
        y.push_str(&format!("  - k{i}: v{i}\n"));
    }
    y
}

fn breach(err: &Error) -> &BudgetBreach {
    match err {
        Error::Budget(b) => b,
        other => panic!("expected a budget breach, got {other:?}"),
    }
}

/// Both paths must fail, with the same breach, and the breach must be
/// the one the test is about.
fn assert_same_breach<T>(yaml: &str, cfg: &ParserConfig, is_expected: fn(&BudgetBreach) -> bool)
where
    T: for<'de> serde::Deserialize<'de> + std::fmt::Debug + 'static,
{
    let typed = from_str_with_config::<T>(yaml, cfg).expect_err("typed target must fail");
    let value = from_str_with_config::<Value>(yaml, cfg).expect_err("Value target must fail");
    assert!(is_expected(breach(&typed)), "typed path: {typed:?}");
    assert!(is_expected(breach(&value)), "Value path: {value:?}");
    assert_eq!(
        breach(&typed),
        breach(&value),
        "both paths must report the same breach"
    );
}

#[test]
fn max_events_is_charged_on_the_typed_path() {
    let cfg = ParserConfig::default().max_events(50);
    assert_same_breach::<Records>(&records(200), &cfg, |b| {
        matches!(b, BudgetBreach::MaxEvents { limit: 50, .. })
    });
}

#[test]
fn max_nodes_is_charged_on_the_typed_path() {
    let cfg = ParserConfig::default().max_nodes(50);
    assert_same_breach::<Records>(&records(200), &cfg, |b| {
        matches!(b, BudgetBreach::MaxNodes { limit: 50, .. })
    });
}

#[test]
fn max_total_scalar_bytes_is_charged_on_the_typed_path() {
    let cfg = ParserConfig::default().max_total_scalar_bytes(100);
    assert_same_breach::<Records>(&records(200), &cfg, |b| {
        matches!(b, BudgetBreach::MaxTotalScalarBytes { limit: 100, .. })
    });
}

#[test]
fn max_merge_keys_is_charged_on_the_typed_path() {
    let yaml = "base: &b {x: 1}\nitems:\n  - <<: *b\n  - <<: *b\n  - <<: *b\n";
    let cfg = ParserConfig::default().max_merge_keys(2);
    assert_same_breach::<Merged>(yaml, &cfg, |b| {
        matches!(b, BudgetBreach::MaxMergeKeys { limit: 2, .. })
    });
}

#[test]
fn alias_anchor_ratio_is_charged_on_the_typed_path() {
    let yaml = "base: &b 1\nitems: [*b, *b, *b]\n";
    let cfg = ParserConfig::default().alias_anchor_ratio(Some(2.0));
    assert_same_breach::<Aliased>(yaml, &cfg, |b| {
        matches!(b, BudgetBreach::AliasAnchorRatio { anchors: 1, .. })
    });
}

#[test]
fn max_mapping_keys_is_a_budget_breach_on_the_typed_path() {
    let cfg = ParserConfig::default().max_mapping_keys(2);
    assert_same_breach::<HashMap<String, i64>>("a: 1\nb: 2\nc: 3\n", &cfg, |b| {
        matches!(
            b,
            BudgetBreach::MaxMappingKeys {
                limit: 2,
                observed: 3
            }
        )
    });
}

#[test]
fn max_sequence_length_is_a_budget_breach_on_the_typed_path() {
    let cfg = ParserConfig::default().max_sequence_length(2);
    assert_same_breach::<Vec<i64>>("[1, 2, 3]", &cfg, |b| {
        matches!(
            b,
            BudgetBreach::MaxSequenceLength {
                limit: 2,
                observed: 3
            }
        )
    });
}

/// The charges must not over-count either: a document comfortably
/// inside every limit still parses on both paths.
#[test]
fn budgets_inside_the_limit_still_parse_on_both_paths() {
    let cfg = ParserConfig::default()
        .max_events(1000)
        .max_nodes(1000)
        .max_total_scalar_bytes(10_000)
        .max_merge_keys(1)
        .alias_anchor_ratio(Some(10.0));
    let yaml = records(10);
    let typed = from_str_with_config::<Records>(&yaml, &cfg).expect("typed target parses");
    assert_eq!(typed.items.len(), 10, "every record must survive");
    let value = from_str_with_config::<Value>(&yaml, &cfg).expect("Value target parses");
    assert_eq!(value["items"].as_sequence().map(Vec::len), Some(10));
}

/// Merge keys and aliases inside their limits still expand on the
/// typed path, with the counters charged but not tripped.
#[test]
fn merges_and_aliases_inside_the_limit_still_expand() {
    let merged_yaml = "base: &b {x: 1}\nitems:\n  - <<: *b\n  - <<: *b\n  - <<: *b\n";
    let cfg = ParserConfig::default().max_merge_keys(3);
    let merged = from_str_with_config::<Merged>(merged_yaml, &cfg).expect("three merges fit");
    assert_eq!(merged.base.get("x"), Some(&1));
    assert_eq!(merged.items.len(), 3, "every merged record must survive");
    assert!(merged.items.iter().all(|m| m.get("x") == Some(&1)));

    let aliased_yaml = "base: &b 1\nitems: [*b, *b, *b]\n";
    let cfg = ParserConfig::default().alias_anchor_ratio(Some(3.0));
    let aliased = from_str_with_config::<Aliased>(aliased_yaml, &cfg).expect("three aliases fit");
    assert_eq!(aliased.base, 1);
    assert_eq!(aliased.items, vec![1, 1, 1]);
}

// ── The entry-point × limit table ──────────────────────────────────

mod table {
    use std::collections::BTreeMap;
    use std::path::Path;

    use noyalib::policy::{DenyAnchors, Policy};
    use noyalib::{DuplicateKeyPolicy, ParserConfig, Value};
    use serde::de::IgnoredAny;

    type Outcome = Result<(), String>;
    type Probe = fn(&str, &ParserConfig) -> Outcome;

    fn done<T, E: std::fmt::Display>(r: Result<T, E>) -> Outcome {
        r.map(drop).map_err(|e| e.to_string())
    }

    /// The typed target: the hostile part always sits under `doc`.
    #[derive(serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct Doc {
        doc: Value,
    }

    /// A policy that refuses nothing: registering it moves a typed
    /// target off the streaming path onto the span-full loader.
    #[derive(Debug)]
    struct Inert;
    impl Policy for Inert {}

    fn on_span_full_loader(s: &str, c: &ParserConfig) -> Outcome {
        let c = c.clone().with_policy(Inert);
        done(noyalib::from_str_with_config::<Doc>(s, &c))
    }

    fn streaming_direct(s: &str, c: &ParserConfig) -> Outcome {
        use serde::Deserialize as _;
        let mut de = noyalib::StreamingDeserializer::with_config(s, c);
        let _doc = Doc::deserialize(&mut de).map_err(|e| e.to_string())?;
        done(de.end())
    }

    #[cfg(feature = "recovery")]
    fn recovery(s: &str, c: &ParserConfig) -> Outcome {
        let lc = noyalib::recovery::LenientConfig {
            base_config: c.clone(),
            ..Default::default()
        };
        let r = noyalib::recovery::parse_lenient_with(s, &lc);
        match r.errors.first() {
            None => Ok(()),
            Some(e) => Err(e.to_string()),
        }
    }

    #[cfg(feature = "tokio")]
    fn block_on<F: Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime")
            .block_on(f)
    }

    #[cfg(feature = "tokio")]
    fn tokio_single(s: &str, c: &ParserConfig) -> Outcome {
        let mut r = std::io::Cursor::new(s.as_bytes().to_vec());
        done(block_on(
            noyalib::tokio_async::from_async_reader_with_config::<_, Value>(&mut r, c),
        ))
    }

    #[cfg(feature = "tokio")]
    fn tokio_multi(s: &str, c: &ParserConfig) -> Outcome {
        let mut r = std::io::Cursor::new(s.as_bytes().to_vec());
        done(block_on(
            noyalib::tokio_async::from_async_reader_multi_with_config::<_, Value>(&mut r, c),
        ))
    }

    #[cfg(feature = "tokio")]
    fn yaml_decoder(s: &str, c: &ParserConfig) -> Outcome {
        use tokio_util::codec::Decoder as _;
        let mut d = noyalib::tokio_async::YamlDecoder::<Value>::with_config(c.clone());
        let mut buf = bytes::BytesMut::from(s.as_bytes());
        while d.decode(&mut buf).map_err(|e| e.to_string())?.is_some() {}
        done(d.decode_eof(&mut buf))
    }

    /// Entry points that read exactly one document.
    fn single_document() -> Vec<(&'static str, Probe)> {
        #[allow(unused_mut)] // the pushes below are feature-gated
        let mut eps: Vec<(&'static str, Probe)> = vec![
            ("from_str_with_config<Value>", |s, c| {
                done(noyalib::from_str_with_config::<Value>(s, c))
            }),
            ("from_str_with_config<struct> (streaming)", |s, c| {
                done(noyalib::from_str_with_config::<Doc>(s, c))
            }),
            ("from_str_with_config<BTreeMap>", |s, c| {
                done(noyalib::from_str_with_config::<BTreeMap<String, Value>>(
                    s, c,
                ))
            }),
            ("from_str_with_config<IgnoredAny>", |s, c| {
                done(noyalib::from_str_with_config::<IgnoredAny>(s, c))
            }),
            (
                "from_str_with_config<struct> (span-full loader)",
                on_span_full_loader,
            ),
            ("from_slice_with_config<Value>", |s, c| {
                done(noyalib::from_slice_with_config::<Value>(s.as_bytes(), c))
            }),
            ("from_reader_with_config<Value>", |s, c| {
                done(noyalib::from_reader_with_config::<_, Value>(
                    s.as_bytes(),
                    c,
                ))
            }),
            ("borrowed::from_str_borrowed_with_config", |s, c| {
                done(noyalib::borrowed::from_str_borrowed_with_config(s, c))
            }),
            (
                "from_str_borrowing_with_config<BTreeMap<&str, Value>>",
                |s, c| {
                    done(noyalib::from_str_borrowing_with_config::<
                        BTreeMap<&str, Value>,
                    >(s, c))
                },
            ),
            ("StreamingDeserializer::with_config + end", streaming_direct),
        ];
        #[cfg(feature = "tokio")]
        eps.push(("tokio::from_async_reader_with_config", tokio_single));
        eps
    }

    /// Entry points that read a stream of documents.
    fn multi_document() -> Vec<(&'static str, Probe)> {
        #[allow(unused_mut)] // the pushes below are feature-gated
        let mut eps: Vec<(&'static str, Probe)> = vec![
            ("load_all_with_config", |s, c| {
                done(noyalib::load_all_with_config(s, c))
            }),
            ("load_all_as_with_config<Value>", |s, c| {
                done(noyalib::load_all_as_with_config::<Value>(s, c))
            }),
            ("read_with_config<Value>", |s, c| {
                done(noyalib::read_with_config::<_, Value>(s.as_bytes(), c))
            }),
        ];
        #[cfg(feature = "parallel")]
        eps.push(("parallel::values_with_config", |s, c| {
            done(noyalib::parallel::values_with_config(s, c))
        }));
        #[cfg(feature = "recovery")]
        eps.push(("recovery::parse_lenient_with", recovery));
        #[cfg(feature = "tokio")]
        eps.push(("tokio::from_async_reader_multi_with_config", tokio_multi));
        #[cfg(feature = "tokio")]
        eps.push(("tokio::YamlDecoder", yaml_decoder));
        eps
    }

    /// One limit: a configuration that sets it, a document that breaks
    /// it, and the words a refusal for that reason contains.
    struct Limit {
        name: &'static str,
        config: ParserConfig,
        hostile: String,
        reason: &'static [&'static str],
    }

    fn limit(
        name: &'static str,
        config: ParserConfig,
        hostile: impl Into<String>,
        reason: &'static [&'static str],
    ) -> Limit {
        Limit {
            name,
            config,
            hostile: hostile.into(),
            reason,
        }
    }

    fn seq(n: usize) -> String {
        let items: Vec<String> = (0..n).map(|i| i.to_string()).collect();
        format!("[{}]", items.join(", "))
    }

    fn map(n: usize) -> String {
        let items: Vec<String> = (0..n).map(|i| format!("k{i}: {i}")).collect();
        format!("{{{}}}", items.join(", "))
    }

    const ALIAS: &[&str] = &["alias expansion limit"];
    const DEPTH: &[&str] = &["recursion"];

    /// Limits that hold per document: every entry point must refuse.
    /// The hostile part sits under `doc` so one typed target fits all.
    fn per_document_limits() -> Vec<Limit> {
        let mut limits = depth_length_and_alias_limits();
        limits.extend(count_and_policy_limits());
        limits
    }

    fn depth_length_and_alias_limits() -> Vec<Limit> {
        let d = ParserConfig::default;
        vec![
            limit(
                "max_depth",
                d().max_depth(8),
                format!("doc: {}1{}\n", "[".repeat(10), "]".repeat(10)),
                DEPTH,
            ),
            limit(
                "max_depth through an alias",
                d().max_depth(8),
                "doc: {a: &a [[[[1]]]], b: [[[[[*a]]]]]}\n",
                DEPTH,
            ),
            limit(
                "max_document_length",
                d().max_document_length(64),
                format!("doc: {}\n", "x".repeat(100)),
                &["maximum length", "max_document_length", "max_frame_size"],
            ),
            limit(
                "max_alias_expansions",
                d().max_alias_expansions(4),
                "doc: {a: &a 1, b: [*a, *a, *a, *a, *a, *a]}\n",
                ALIAS,
            ),
            limit(
                "alias expansion bytes",
                d().max_document_length(2000),
                format!("doc: {{a: &a [{}], b: *a}}\n", vec!["''"; 100].join(", ")),
                ALIAS,
            ),
            limit(
                "alias_anchor_ratio",
                d().alias_anchor_ratio(Some(2.0)),
                "doc: {a: &a 1, b: [*a, *a, *a, *a, *a]}\n",
                &["alias_anchor_ratio"],
            ),
        ]
    }

    fn count_and_policy_limits() -> Vec<Limit> {
        let d = ParserConfig::default;
        vec![
            limit(
                "max_events",
                d().max_events(20),
                format!("doc: {}\n", seq(30)),
                &["max_events"],
            ),
            limit(
                "max_nodes",
                d().max_nodes(10),
                format!("doc: {}\n", seq(30)),
                &["max_nodes"],
            ),
            limit(
                "max_total_scalar_bytes",
                d().max_total_scalar_bytes(16),
                format!("doc: {}\n", "x".repeat(40)),
                &["max_total_scalar_bytes"],
            ),
            limit(
                "max_mapping_keys",
                d().max_mapping_keys(4),
                format!("doc: {}\n", map(6)),
                &["max_mapping_keys"],
            ),
            limit(
                "max_sequence_length",
                d().max_sequence_length(4),
                format!("doc: {}\n", seq(6)),
                &["max_sequence_length"],
            ),
            limit(
                "max_merge_keys",
                d().max_merge_keys(1),
                "doc: {b: &b {x: 1}, m: {<<: *b}, n: {<<: *b}, o: {<<: *b}}\n",
                &["max_merge_keys"],
            ),
            limit(
                "DuplicateKeyPolicy::Error",
                d().duplicate_key_policy(DuplicateKeyPolicy::Error),
                "doc: {a: 1, a: 2}\n",
                &["duplicate"],
            ),
            limit(
                "DenyAnchors policy",
                d().with_policy(DenyAnchors),
                "doc: {a: &x 1, b: *x}\n",
                &["DenyAnchors"],
            ),
        ]
    }

    /// Four documents, each a ten-integer flow sequence under `doc`.
    fn four_docs() -> String {
        format!("---\ndoc: {}\n", seq(10)).repeat(4)
    }

    /// Limits that hold across a stream: each document fits, the
    /// stream does not. Only multi-document entry points apply.
    fn stream_limits() -> Vec<Limit> {
        let d = ParserConfig::default;
        vec![
            limit(
                "max_events across documents",
                d().max_events(50),
                four_docs(),
                &["max_events"],
            ),
            limit(
                "max_nodes across documents",
                d().max_nodes(30),
                four_docs(),
                &["max_nodes"],
            ),
            limit(
                "max_total_scalar_bytes across documents",
                d().max_total_scalar_bytes(30),
                four_docs(),
                &["max_total_scalar_bytes"],
            ),
            limit(
                "max_documents",
                d().max_documents(2),
                four_docs(),
                &["max_documents", "documents"],
            ),
            limit(
                "max_stream_bytes",
                d().max_stream_bytes(64).max_document_length(64),
                four_docs(),
                &["max_stream_bytes", "maximum length"],
            ),
        ]
    }

    /// Cells known not to hold yet, each with what closes it. Remove an
    /// entry when its fix lands; the table then holds the cell.
    const KNOWN_GAPS: &[(&str, &str, &str)] = &[];

    fn known_gap(ep: &str, limit: &str) -> bool {
        // A frame decoder sees frames, not a stream length.
        (limit == "max_stream_bytes" && ep == "tokio::YamlDecoder")
            || KNOWN_GAPS.iter().any(|(e, l, _)| *e == ep && *l == limit)
    }

    /// Run every probe on every limit; list each cell that does not
    /// refuse for the limit's reason.
    fn gaps(probes: &[(&'static str, Probe)], limits: &[Limit]) -> Vec<String> {
        let mut out = Vec::new();
        for l in limits {
            for (ep, probe) in probes {
                if known_gap(ep, l.name) {
                    continue;
                }
                match probe(&l.hostile, &l.config) {
                    Ok(()) => out.push(format!("{ep} × {}: accepted", l.name)),
                    Err(e) if !l.reason.iter().any(|r| e.contains(r)) => {
                        out.push(format!(
                            "{ep} × {}: refused for another reason: {e}",
                            l.name
                        ));
                    }
                    Err(_) => {}
                }
            }
        }
        out
    }

    #[test]
    fn every_entry_point_enforces_every_per_document_limit() {
        let mut probes = single_document();
        probes.extend(multi_document());
        let gaps = gaps(&probes, &per_document_limits());
        assert!(
            gaps.is_empty(),
            "{} gap(s):\n{}",
            gaps.len(),
            gaps.join("\n")
        );
    }

    #[test]
    fn every_multi_document_entry_point_enforces_every_stream_limit() {
        let gaps = gaps(&multi_document(), &stream_limits());
        assert!(
            gaps.is_empty(),
            "{} gap(s):\n{}",
            gaps.len(),
            gaps.join("\n")
        );
    }

    /// The hostile inputs are hostile only through their limit: under
    /// the default configuration every cell accepts them, so a refusal
    /// above is the limit at work and not something else.
    #[test]
    fn the_hostile_inputs_parse_without_their_limit() {
        let mut probes = single_document();
        probes.extend(multi_document());
        let mut wrong = Vec::new();
        for l in per_document_limits() {
            for (ep, probe) in &probes {
                if let Err(e) = probe(&l.hostile, &ParserConfig::default()) {
                    wrong.push(format!("{ep} × {}: {e}", l.name));
                }
            }
        }
        for l in stream_limits() {
            for (ep, probe) in multi_document() {
                if let Err(e) = probe(&l.hostile, &ParserConfig::default()) {
                    wrong.push(format!("{ep} × {}: {e}", l.name));
                }
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// The `serde_yaml` façade parses under its own profile, so it is
    /// driven at that profile's limits, not the table's configurations.
    #[cfg(feature = "compat-serde-yaml")]
    #[test]
    fn the_compat_facade_enforces_its_profile_limits() {
        use noyalib::compat::serde_yaml as syml;
        let deep = format!("{}1{}", "[".repeat(200), "]".repeat(200));
        let mut laughs = String::from("l0: &l0 [lol, lol, lol, lol, lol, lol, lol, lol, lol]\n");
        for i in 1..=9 {
            let refs = vec![format!("*l{}", i - 1); 9].join(", ");
            laughs.push_str(&format!("l{i}: &l{i} [{refs}]\n"));
        }
        let mut chain = format!("a0: &a0 {}1{}\n", "[".repeat(100), "]".repeat(100));
        for i in 1..=3 {
            chain.push_str(&format!(
                "a{i}: &a{i} {}*a{}{}\n",
                "[".repeat(100),
                i - 1,
                "]".repeat(100)
            ));
        }
        let documents = "---\na: 1\n".repeat(1001);
        let cases: [(&str, &str); 4] = [
            ("max_depth", &deep),
            ("alias expansion", &laughs),
            ("max_depth through an alias", &chain),
            ("duplicate keys", "a: 1\na: 2\n"),
        ];
        let mut gaps = Vec::new();
        for (name, yaml) in cases {
            if syml::from_str::<Value>(yaml).is_ok() {
                gaps.push(format!("from_str × {name}"));
            }
            if syml::from_str_multi::<Value>(yaml).is_ok() {
                gaps.push(format!("from_str_multi × {name}"));
            }
        }
        if syml::from_str_multi::<Value>(&documents).is_ok() {
            gaps.push("from_str_multi × max_documents".into());
        }
        assert!(gaps.is_empty(), "{gaps:#?}");
    }

    /// Public functions that read YAML text and are exercised above,
    /// directly or through the function they delegate to.
    const COVERED: &[&str] = &[
        "from_str", // delegates to from_str_with_config
        "from_str_with_config",
        "from_slice", // delegates to from_slice_with_config
        "from_slice_with_config",
        "from_reader", // delegates to from_reader_with_config
        "from_reader_with_config",
        "from_str_borrowing", // delegates to the _with_config form
        "from_str_borrowing_with_config",
        "from_str_borrowed", // delegates to the _with_config form
        "from_str_borrowed_with_config",
        "load_all", // delegates to load_all_with_config
        "load_all_with_config",
        "try_load_all", // alias of load_all
        "load_all_as",  // delegates to load_all_as_with_config
        "load_all_as_with_config",
        "read", // delegates to read_with_config
        "read_with_config",
        "parse", // parallel; delegates to parse_with_config
        "parse_with_config",
        "parse_with_config_in_pool", // parse_with_config on a caller's pool
        "values",                    // parallel; parse::<Value>
        "values_with_config",
        "values_with_config_in_pool",
        "parse_lenient", // delegates to parse_lenient_with
        "parse_lenient_with",
        "from_async_reader", // delegates to the _with_config form
        "from_async_reader_with_config",
        "from_async_reader_multi", // delegates to the _with_config form
        "from_async_reader_multi_with_config",
        "from_str_multi", // compat
    ];

    /// Public functions the scan finds that do not read YAML text under
    /// a `ParserConfig`, with the reason.
    const EXEMPT: &[(&str, &str)] = &[
        ("from_value", "deserializes an existing Value; no parsing"),
        (
            "from_str_strict",
            "from_str_with_config plus an unknown-field check",
        ),
        ("from_slice_strict", "from_str_strict on UTF-8 bytes"),
        ("from_reader_strict", "from_str_strict on a bounded read"),
        (
            "load_comments",
            "collects comments from the scanner; builds no value",
        ),
    ];

    /// Every `pub fn` / `pub async fn` whose name starts like a parse
    /// entry point, in the crate's non-CST, non-value modules.
    fn public_parse_functions() -> Vec<(String, String)> {
        fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
            for entry in std::fs::read_dir(dir).expect("src is readable").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                for line in text.lines() {
                    let t = line.trim_start();
                    let Some(rest) = t
                        .strip_prefix("pub fn ")
                        .or_else(|| t.strip_prefix("pub async fn "))
                    else {
                        continue;
                    };
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    let parse_like = [
                        "from_str",
                        "from_slice",
                        "from_reader",
                        "from_async",
                        "load_",
                        "parse",
                        "read",
                        "values",
                        "try_load",
                    ]
                    .iter()
                    .any(|p| name.starts_with(p) || name == p.trim_end_matches('_'));
                    if parse_like {
                        out.push((name, path.display().to_string()));
                    }
                }
            }
        }
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut out = Vec::new();
        walk(&src, &mut out);
        // The CST, value, schema and error modules are not text-to-value
        // entry points, and their own suites cover their limits.
        out.retain(|(_, file)| {
            ![
                "/cst/",
                "/value",
                "/schema",
                "/error.rs",
                "/simd.rs",
                "/path.rs",
                "/with/",
            ]
            .iter()
            .any(|skip| file.contains(skip))
        });
        out
    }

    #[test]
    fn every_public_entry_point_has_a_row() {
        let missing: Vec<String> = public_parse_functions()
            .into_iter()
            .filter(|(name, _)| {
                !COVERED.contains(&name.as_str()) && !EXEMPT.iter().any(|(e, _)| e == name)
            })
            .map(|(name, file)| format!("{name} ({file})"))
            .collect();
        assert!(
            missing.is_empty(),
            "public parse entry points with no row in the parity table and no exemption; \
             add each to the table (and COVERED) or to EXEMPT with a reason:\n{}",
            missing.join("\n")
        );
    }
}
