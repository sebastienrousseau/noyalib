// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `DuplicateKeyPolicy` judges keys by their resolved value on every
//! load path: `~` and `null` are the same null key. Found while
//! triaging fuzz_diff: the streaming typed path compared the raw
//! text, so it accepted `~: 1\nnull: 2` that the `Value` path refused.

use std::collections::BTreeMap;

use noyalib::{
    DuplicateKeyPolicy, ErrorKind, MergeKeyPolicy, ParserConfig, TagRegistry, Value,
    from_str_with_config,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, PartialEq)]
struct Entries(Vec<(String, Value)>);

impl<'de> Deserialize<'de> for Entries {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Entries;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a mapping")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Entries, M::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }
                Ok(Entries(entries))
            }
        }
        de.deserialize_map(Visitor)
    }
}

#[test]
fn canonical_policy_keeps_position_value_and_selected_spelling() {
    for (first, last, canonical) in [
        ("~", "null", "null"),
        ("True", "true", "true"),
        ("0x1F", "31", "31"),
        ("+1", "1", "1"),
        ("1.0", "1e0", "1.0"),
    ] {
        let doc = format!("before: 0\n{first}: 1\nbetween: 2\n{last}: 3\nafter: 4\n");
        for policy in [
            DuplicateKeyPolicy::First,
            DuplicateKeyPolicy::Last,
            DuplicateKeyPolicy::Error,
        ] {
            for mode in [MergeKeyPolicy::Auto, MergeKeyPolicy::AsOrdinary] {
                let cfg = ParserConfig::new()
                    .duplicate_key_policy(policy)
                    .merge_key_policy(mode);
                let typed = from_str_with_config::<Entries>(&doc, &cfg);
                let value = from_str_with_config::<Value>(&doc, &cfg);
                if matches!(policy, DuplicateKeyPolicy::Error) {
                    assert_eq!(
                        typed.unwrap_err().kind(),
                        ErrorKind::DuplicateKey,
                        "{mode:?} {doc:?}"
                    );
                    assert_eq!(value.unwrap_err().kind(), ErrorKind::DuplicateKey);
                    continue;
                }
                let (key, number) = if matches!(policy, DuplicateKeyPolicy::First) {
                    (first, 1)
                } else {
                    (last, 3)
                };
                assert_eq!(
                    typed.unwrap().0,
                    [
                        ("before".into(), 0.into()),
                        (key.into(), number.into()),
                        ("between".into(), 2.into()),
                        ("after".into(), 4.into())
                    ],
                    "{policy:?} {mode:?} {doc:?}"
                );
                let value = value.unwrap();
                assert_eq!(value[canonical], Value::from(number));
                assert_eq!(
                    value
                        .as_mapping()
                        .unwrap()
                        .keys()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                    ["before", canonical, "between", "after"]
                );
            }
        }
    }
}

#[test]
fn original_empty_null_duplicate_and_identical_keys_reach_visitors_once() {
    for (source, key, value) in [
        ("{~: 1, ? }", "", Value::Null),
        ("a: 1\na: 2\n", "a", Value::from(2)),
    ] {
        let entries: Entries = noyalib::from_str(source).unwrap();
        assert_eq!(entries.0, [(key.to_owned(), value)], "{source:?}");
    }
    let source = std::str::from_utf8(include_bytes!(
        "../fixtures/canonical-duplicate-identity.bin"
    ))
    .unwrap();
    let cfg = ParserConfig::new();
    let normal: Entries = from_str_with_config(source, &cfg).unwrap();
    let fallback: Entries =
        from_str_with_config(source, &cfg.merge_key_policy(MergeKeyPolicy::AsOrdinary)).unwrap();
    assert_eq!(normal, fallback);
}

#[derive(Debug, Deserialize)]
struct Known {
    keep: i32,
}

#[test]
fn ignored_maps_apply_canonical_duplicate_and_collision_checks() {
    for (keys, kind) in [
        ("~: 1, null: 2", ErrorKind::DuplicateKey),
        ("1: 1, '1': 2", ErrorKind::KeyCollision),
    ] {
        let doc = format!("keep: 7\nignored: {{{keys}}}\n");
        let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
        assert_eq!(
            from_str_with_config::<Known>(&doc, &cfg)
                .unwrap_err()
                .kind(),
            kind,
            "{doc:?}"
        );
    }
    let doc = "keep: 7\nignored: {1: 1, '1': 2}\n";
    for policy in [DuplicateKeyPolicy::First, DuplicateKeyPolicy::Last] {
        assert_eq!(
            from_str_with_config::<Known>(doc, &ParserConfig::new().duplicate_key_policy(policy))
                .unwrap_err()
                .kind(),
            ErrorKind::KeyCollision
        );
    }
}

#[test]
fn registered_tagged_keys_keep_canonical_identity_even_when_ignored() {
    let cfg = ParserConfig::new().tag_registry(Arc::new(TagRegistry::new().with("!key")));
    for policy in [
        DuplicateKeyPolicy::First,
        DuplicateKeyPolicy::Last,
        DuplicateKeyPolicy::Error,
    ] {
        let cfg = cfg.clone().duplicate_key_policy(policy);
        for source in [
            "!key null: 1\n'null': 2\n",
            "keep: 7\nignored: {!key null: 1, 'null': 2}\n",
        ] {
            let kind = if source.starts_with("keep") {
                from_str_with_config::<Known>(source, &cfg)
                    .unwrap_err()
                    .kind()
            } else {
                from_str_with_config::<Entries>(source, &cfg)
                    .unwrap_err()
                    .kind()
            };
            assert_eq!(kind, ErrorKind::KeyCollision, "{policy:?} {source:?}");
        }
    }
}

#[test]
fn aliases_and_tagged_keys_obey_the_loader_policy() {
    for source in [
        "anchor: &n ~\nmap: {~: 1, *n : 2}\n",
        "anchor: &n ~\nmap: {*n : 1, null: 2}\n",
        "map: {!!int 0x1F: 1, 31: 2}\n",
    ] {
        for policy in [
            DuplicateKeyPolicy::First,
            DuplicateKeyPolicy::Last,
            DuplicateKeyPolicy::Error,
        ] {
            let cfg = ParserConfig::new().duplicate_key_policy(policy);
            let normal = from_str_with_config::<Entries>(source, &cfg);
            let fallback = from_str_with_config::<Entries>(
                source,
                &cfg.merge_key_policy(MergeKeyPolicy::AsOrdinary),
            );
            if matches!(policy, DuplicateKeyPolicy::Error) {
                assert_eq!(normal.unwrap_err().kind(), ErrorKind::DuplicateKey);
                assert_eq!(fallback.unwrap_err().kind(), ErrorKind::DuplicateKey);
            } else {
                assert_eq!(normal.unwrap(), fallback.unwrap(), "{policy:?} {source:?}");
            }
        }
    }
}

#[test]
fn canonical_duplicates_do_not_evade_mapping_budgets() {
    for policy in [
        DuplicateKeyPolicy::First,
        DuplicateKeyPolicy::Last,
        DuplicateKeyPolicy::Error,
    ] {
        let cfg = ParserConfig::new()
            .max_mapping_keys(1)
            .duplicate_key_policy(policy);
        for source in ["~: 1\nnull: 2\n", "ignored: {~: 1, null: 2}\n"] {
            assert!(
                from_str_with_config::<Entries>(source, &cfg).is_err(),
                "{policy:?} {source:?}"
            );
            assert!(
                from_str_with_config::<Value>(source, &cfg).is_err(),
                "{policy:?} {source:?}"
            );
            assert!(
                from_str_with_config::<serde::de::IgnoredAny>(source, &cfg).is_err(),
                "{policy:?} {source:?}"
            );
        }
    }
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Last);
    let value: Known = from_str_with_config("keep: 7\nignored: {~: 1, null: 2}\n", &cfg).unwrap();
    assert_eq!(value.keep, 7);
}

#[test]
fn same_resolved_key_is_a_duplicate_on_every_path() {
    let refuse = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    for doc in ["~: 1\nnull: 2\n", "0x1F: 1\n31: 2\n", "True: 1\ntrue: 2\n"] {
        assert!(
            from_str_with_config::<Value>(doc, &refuse).is_err(),
            "Value {doc:?}"
        );
        assert!(
            from_str_with_config::<BTreeMap<String, i32>>(doc, &refuse).is_err(),
            "typed {doc:?}"
        );
    }
}

#[test]
fn first_policy_keeps_the_first_of_a_resolved_duplicate() {
    let first = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::First);
    let m: BTreeMap<String, i32> = from_str_with_config("~: 1\nnull: 2\n", &first).unwrap();
    assert_eq!(m.len(), 1, "{m:?}");
    assert_eq!(m.values().next(), Some(&1));
}
