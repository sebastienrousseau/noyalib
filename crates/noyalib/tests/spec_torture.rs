// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Three documents that each stress one corner of YAML 1.2.
//!
//! `complex-keys-and-chomping.yaml` uses a sequence and a mapping as
//! mapping keys, all three chomping indicators and an explicit
//! indentation indicator, and every spelling of null.
//! `anchor-lattice-and-tags.yaml` merges a map that itself contains
//! merged maps, and tags scalars with `!!binary`, `!!timestamp` and
//! `!!int` over hexadecimal and octal.
//! `collection-types.yaml` uses `!!set`, `!!omap` and `!!pairs`
//! together, including a duplicate in the set and repeated keys in the
//! pairs.
//!
//! Every expectation below was cross-checked against libyaml (through
//! Ruby Psych) and go-yaml (through yq). Where they disagree the
//! comment says so.

#![allow(missing_docs)]

use noyalib::{DuplicateKeyPolicy, ParserConfig, Value, from_str, from_str_with_config};
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/spec-torture")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn complex_keys_chomping_and_nulls() {
    let doc: Value = from_str(&fixture("complex-keys-and-chomping.yaml")).expect("parses");
    let m = &doc["matrix_configuration"];

    // A sequence and a mapping used as keys are addressable by their
    // flow rendering. libyaml and go-yaml keep them as native nodes;
    // this crate's mapping keys are strings, so the flow form is the
    // key. The information survives; the type of the key does not.
    let keys: Vec<&str> = m.as_mapping().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "[us-east-1, us-east-2]",
            "{provider: aws, service: eks}",
            "string_behaviors",
            "null_matrix"
        ]
    );
    assert_eq!(
        m["[us-east-1, us-east-2]"]["failover_target"].as_str(),
        Some("eu-west-1")
    );
    assert_eq!(
        m["{provider: aws, service: eks}"]["node_groups"][0]["capacity_type"].as_str(),
        Some("SPOT")
    );

    let s = &m["string_behaviors"];
    // Strip removes every trailing break, keep retains them all, and an
    // explicit indentation indicator folds only the lines at that
    // indentation. All three agree with libyaml and go-yaml.
    assert_eq!(
        s["strip_newlines"].as_str(),
        Some("This text block drops all trailing newlines at the end of the script.")
    );
    assert_eq!(
        s["keep_newlines"].as_str(),
        Some("This text block keeps all trailing blank lines exactly as typed.\n\n\n")
    );
    assert_eq!(
        s["folded_with_indentation"].as_str(),
        Some(
            "This block uses an explicit 4-space indentation indicator. Any deeper lines are preserved as block indents.\n  - Nested line 1\n  - Nested line 2\n"
        )
    );

    // Every spelling of null resolves to null.
    for key in ["empty_value", "canonical_null", "word_null", "tilde_null"] {
        assert!(
            m["null_matrix"][key].untag_ref().is_null(),
            "{key} is not null"
        );
    }
}

#[test]
fn nested_merges_and_explicit_tags() {
    let doc: Value = from_str(&fixture("anchor-lattice-and-tags.yaml")).expect("parses");

    // An explicit key beats the one the merge would have supplied, and
    // the rest of the merged hierarchy is still there.
    let core = &doc["deployment_matrix"]["core-api-service"];
    assert_eq!(
        core["scaling"]["min_replicas"].untag_ref().as_i64(),
        Some(5)
    );
    assert_eq!(
        core["scaling"]["max_replicas"].untag_ref().as_i64(),
        Some(50)
    );
    assert_eq!(core["hardware"]["cpu_cores"].untag_ref().as_i64(), Some(16));
    assert_eq!(core["networking"]["vpc_id"].as_str(), Some("vpc-0a1b2c3d"));

    // A merge inside a merge: the worker overrides one leaf of a
    // structure it inherited, and keeps its siblings.
    let worker = &doc["deployment_matrix"]["data-processing-worker"];
    assert_eq!(
        worker["hardware"]["storage"]["io_ops"].untag_ref().as_i64(),
        Some(500_000)
    );
    assert_eq!(worker["hardware"]["storage"]["type"].as_str(), Some("NVMe"));
    assert_eq!(
        worker["hardware"]["cpu_cores"].untag_ref().as_i64(),
        Some(16)
    );
    assert_eq!(
        worker["scaling"]["min_replicas"].untag_ref().as_i64(),
        Some(2)
    );

    let t = &doc["strict_typing_tests"];
    // YAML 1.2 core schema: `0x` is hexadecimal and `0o` is octal.
    // libyaml still reads YAML 1.1 here and returns the string
    // "0o755"; go-yaml agrees with this crate.
    assert_eq!(t["hexadecimal_int"].untag_ref().as_i64(), Some(6703));
    assert_eq!(t["octal_int"].untag_ref().as_i64(), Some(493));
    // `!!timestamp` is not in the YAML 1.2 core schema, so the tag is
    // kept and the scalar stays a string, as in go-yaml.
    assert_eq!(
        t["timestamp_canonical"].untag_ref().as_str(),
        Some("2026-09-06T23:45:00.00Z")
    );
    assert_eq!(
        t["timestamp_spaced"].untag_ref().as_str(),
        Some("2026-09-06 23:45:00 +01:00")
    );
    // `!!binary` keeps its tag and its base64 text in the dynamic tree.
    assert!(matches!(t["binary_payload"], Value::Tagged(_)));
}

#[test]
fn binary_decodes_into_a_byte_target() {
    // The base64 is decoded when the serde target asks for bytes. A
    // plain `Vec<u8>` asks serde for a sequence, so it needs
    // `serde_bytes`, exactly as with any other serde format.
    #[derive(serde::Deserialize)]
    struct Payload {
        #[serde(with = "serde_bytes")]
        binary_payload: Vec<u8>,
    }
    let src = fixture("anchor-lattice-and-tags.yaml");
    let start = src.find("strict_typing_tests:").expect("section");
    let section = src[start..].replace("strict_typing_tests:\n", "");
    let only_binary: String = section
        .lines()
        .take_while(|l| !l.trim_start().starts_with("timestamp_canonical"))
        .map(|l| format!("{}\n", l.strip_prefix("  ").unwrap_or(l)))
        .collect();
    let p: Payload = from_str(&only_binary).expect("!!binary decodes into a byte target");
    assert_eq!(
        &p.binary_payload[..4],
        b"GIF8",
        "decoded {} bytes",
        p.binary_payload.len()
    );
}

#[test]
fn set_omap_and_pairs_keep_their_contracts() {
    let src = fixture("collection-types.yaml");
    let doc: Value = from_str(&src).expect("parses");
    let c = &doc["collection_torture_chamber"];

    // `!!set` is a mapping with null values. The repeated member folds
    // into one under the default policy, which is YAML 1.2's "last one
    // wins" for duplicate keys.
    let set = c["unique_cluster_tags"]
        .untag_ref()
        .as_mapping()
        .expect("!!set is a mapping");
    assert_eq!(set.len(), 3);
    assert!(
        set.keys()
            .all(|k| ["production", "pci-compliant", "dmz-zone"].contains(&k.as_str()))
    );

    // `!!omap` keeps its order.
    let omap = c["execution_pipeline_steps"]
        .untag_ref()
        .as_sequence()
        .expect("!!omap");
    let first_keys: Vec<&str> = omap
        .iter()
        .map(|e| {
            e.as_mapping()
                .expect("single-key mapping")
                .keys()
                .next()
                .unwrap()
                .as_str()
        })
        .collect();
    assert_eq!(
        first_keys,
        [
            "01_pull_source",
            "02_security_scan",
            "03_compile_binary",
            "04_docker_push"
        ]
    );

    // `!!pairs` keeps repeats: all four `state_change` entries survive,
    // including the two that share the value "RUNNING". A mapping would
    // have collapsed them to one.
    let pairs = c["audit_trail_events"]
        .untag_ref()
        .as_sequence()
        .expect("!!pairs");
    assert_eq!(pairs.len(), 6);
    let state_changes = pairs
        .iter()
        .filter(|e| e.as_mapping().unwrap().contains_key("state_change"))
        .count();
    assert_eq!(state_changes, 4);

    // Flow and block styles nest freely.
    let hybrid = c["hybrid_flow_structures"].as_sequence().expect("hybrid");
    assert_eq!(hybrid[0]["metadata"][2].as_i64(), Some(42));
    assert!(hybrid[0]["metadata"][1].is_null());
    assert_eq!(hybrid[1][1]["sub_key"][2].as_i64(), Some(3));
}

#[test]
fn the_repeated_set_member_is_reportable() {
    // The duplicate in `!!set` is silent by default. Ask for the strict
    // policy and it is refused, with the path and the position.
    let src = fixture("collection-types.yaml");
    let cfg = ParserConfig::new().duplicate_key_policy(DuplicateKeyPolicy::Error);
    let err = from_str_with_config::<Value>(&src, &cfg).expect_err("the repeat is refused");
    let msg = err.to_string();
    assert!(msg.contains("duplicate key"), "{msg}");
    assert!(msg.contains("production"), "{msg}");
    assert_eq!(err.location().map(|l| l.line()), Some(12), "{msg}");
}

// ── Four more corners, each cross-checked against libyaml and go-yaml ──

#[test]
fn a_self_referential_anchor_is_refused_by_name() {
    // YAML's representation graph may be cyclic; a `Value` is a tree
    // and cannot hold a cycle. libyaml and go-yaml build one with
    // pointers, so they accept this file. The refusal has to say what
    // is actually wrong, not call the anchor unknown.
    let err = from_str::<Value>(&fixture("graph-recursion-and-shadowing.yaml"))
        .expect_err("a self-referential node is refused");
    let msg = err.to_string();
    assert!(
        msg.contains("alias `*node_beta` points at `&node_beta`"),
        "{msg}"
    );
    assert!(msg.contains("still being defined"), "{msg}");
    assert!(msg.contains("cannot be represented as a tree"), "{msg}");
    // And it must not be confused with the cross-document case.
    assert!(!msg.contains("earlier document"), "{msg}");
}

#[test]
fn a_redefined_anchor_shadows_the_earlier_one() {
    // Re-anchoring a name is legal: later aliases see the newest
    // definition, earlier ones keep what they resolved to.
    let src = fixture("graph-recursion-and-shadowing.yaml").replace(
        "      - *node_beta # Recursive pointer. Parsers must not stack-overflow here.\n",
        "      []\n",
    );
    let doc: Value = from_str(&src).expect("parses once the cycle is gone");
    let a = &doc["anchor_shadowing_test"];
    assert_eq!(
        a["scope_one"]["reference"].as_str(),
        Some("Original String Value")
    );
    assert_eq!(a["scope_two"]["reference"][0].as_str(), Some("Shadowed"));
    assert_eq!(
        a["scope_three"]["final_verification"][3].as_str(),
        Some("Array")
    );
}

#[test]
fn a_multi_line_flow_key_is_refused_in_block_context() {
    // An implicit key must fit on one line (YAML 1.2.2 §7.4.2). All
    // three implementations refuse this file; only the wording differs.
    let err = from_str::<Value>(&fixture("flow-escapes.yaml")).expect_err("refused");
    let msg = err.to_string();
    assert!(
        msg.contains("implicit mapping key in block context cannot span multiple lines"),
        "{msg}"
    );
    assert_eq!(err.location().map(|l| l.line()), Some(10), "{msg}");
}

#[test]
fn custom_local_tags_survive_and_numerics_follow_yaml_1_2() {
    // The numeric section carries two YAML 1.1 spellings that 1.2
    // dropped, so the file as a whole is refused. Take the tags on
    // their own first.
    let src = fixture("custom-tags-and-numerics.yaml");
    let tags_only: String = src[..src.find("# Edge cases").expect("section")].to_string();
    let doc: Value = from_str(&tags_only).expect("the custom tags parse");
    let s = &doc["secure_infrastructure"];
    for (key, tag) in [
        ("database_password", "!secret"),
        ("network_map", "!include"),
        ("custom_vector", "!vector3d"),
    ] {
        let Value::Tagged(t) = &s[key] else {
            panic!("{key} lost its tag: {:?}", s[key]);
        };
        assert_eq!(t.tag().to_string(), tag);
    }
    assert_eq!(s["custom_vector"].untag_ref()[0].as_f64(), Some(12.5));

    // Infinity and NaN are real floats, even though JSON cannot show
    // them; the JSON projection renders all three as null.
    let inf: Value = from_str("a: !!float .inf\nb: !!float -.Inf\nc: !!float .NaN\n").unwrap();
    assert_eq!(inf["a"].untag_ref().as_f64(), Some(f64::INFINITY));
    assert_eq!(inf["b"].untag_ref().as_f64(), Some(f64::NEG_INFINITY));
    assert!(inf["c"].untag_ref().as_f64().unwrap().is_nan());
    let sci: Value = from_str("a: !!float 1.23e-4\nb: !!float 4.56E+10\n").unwrap();
    assert_eq!(sci["a"].untag_ref().as_f64(), Some(1.23e-4));
    assert_eq!(sci["b"].untag_ref().as_f64(), Some(4.56E+10));
}

#[test]
fn yaml_1_1_integer_spellings_are_refused_with_the_decimal() {
    // `0b` literals and `_` separators were YAML 1.1. Under an explicit
    // `!!int` a 1.2 parser has to refuse them, and the message should
    // hand back the number the author meant. libyaml still accepts both
    // because it reads 1.1.
    for (src, want) in [
        ("v: !!int 0b101010\n", "write 42"),
        ("v: !!int -0b1111\n", "write -15"),
        ("v: !!int 100_000_000\n", "write 100000000"),
    ] {
        let msg = from_str::<Value>(src).expect_err(src).to_string();
        assert!(msg.contains("YAML 1.2 has no"), "{msg}");
        assert!(msg.contains(want), "{msg}");
    }
    // Untagged, the same spellings are simply strings, which is what
    // YAML 1.2's core schema says.
    let v: Value = from_str("a: 0b101010\nb: 100_000_000\n").unwrap();
    assert_eq!(v["a"].as_str(), Some("0b101010"));
    assert_eq!(v["b"].as_str(), Some("100_000_000"));
}

#[test]
fn null_boundaries_parse_and_colliding_keys_are_reported() {
    // The file's last three documents are all boundary shapes: an empty
    // document, an explicit null key with a null value, and a sequence
    // whose second item is missing.
    let src = fixture("null-boundaries.yaml");

    // As written it is refused, because a `true` key and a `!!str true`
    // key both become the string "true" in this crate's mapping model.
    // go-yaml accepts it and emits JSON with a repeated "true" key,
    // which is not something a consumer can read back unambiguously.
    let err = from_str::<Value>(&src).expect_err("the colliding keys are reported");
    assert!(
        err.to_string().contains("collide after string conversion"),
        "{err}"
    );

    // Without that one line every boundary shape parses.
    let cleaned = src.replace("!!str true: \"forced_string_type\"\n", "");
    let docs = noyalib::load_all_as::<Value>(&cleaned).expect("boundaries parse");
    assert_eq!(docs.len(), 4);
    assert!(docs[0].is_null(), "a comment-only document is null");
    // `? ` with `: ` is the null key holding null.
    assert_eq!(docs[1].as_mapping().map(|m| m.len()), Some(1));
    assert!(docs[1]["null"].is_null());
    // `~` and `null` are the same key, so they fold into one entry.
    assert_eq!(docs[2]["null"].as_null(), Some(()));
    assert_eq!(docs[2]["true"].as_str(), Some("true_as_a_string"));
    // A sequence item with nothing after the dash is null.
    let list = docs[3]["endpoint_list"].as_sequence().expect("sequence");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].as_str(), Some("https://api.internal"));
    assert!(list[1].is_null());
}

// ── Eight more, all three implementations agreeing ────────────────────
//
// Every expectation below was checked byte for byte against libyaml
// (Ruby Psych) and go-yaml (yq), which agree with this crate on all of
// them.

#[test]
fn tag_directives_resolve_shorthands() {
    let doc: Value = from_str(&fixture("directives-and-tag-shorthand.yaml")).expect("parses");
    let s = &doc["secure_service"];
    let tag_of = |v: &Value| match v {
        Value::Tagged(t) => t.tag().to_string(),
        other => panic!("not tagged: {other:?}"),
    };
    // `%TAG !crypto! tag:company.com,2026:crypto/` makes the shorthand
    // expand to the full URI.
    assert_eq!(
        tag_of(&s["algorithm"]),
        "tag:company.com,2026:crypto/rsa-4096"
    );
    assert_eq!(
        tag_of(&s["payload"]),
        "tag:company.com,2026:crypto/encrypted"
    );
    // A local tag with no matching directive stays as written.
    assert_eq!(tag_of(&s["metadata"]), "!local-config");
    // A `#` inside a literal block is content, not a comment.
    assert!(
        s["payload"]
            .untag_ref()
            .as_str()
            .unwrap()
            .contains("# Combining")
    );
}

#[test]
fn sequences_need_no_extra_indentation() {
    let doc: Value = from_str(&fixture("zero-indent-sequences.yaml")).expect("parses");
    let three = &doc["nested_matrix"]["level_one"][0]["level_two"][0]["level_three"];
    assert_eq!(three[0].as_str(), Some("item_a"));
    assert_eq!(three[1].as_str(), Some("item_b"));
    assert_eq!(
        doc["nested_matrix"]["level_one"][1]["level_two_sibling"][1].as_str(),
        Some("item_e")
    );
    let irregular = doc["irregular_blocks"]["mapping_key"]
        .as_sequence()
        .unwrap();
    assert_eq!(irregular[0].as_str(), Some("no indent on sequence item"));
    assert_eq!(
        irregular[1]["nested_map_inside_unindented_seq"]["valid_key"].as_str(),
        Some("value")
    );
}

#[test]
fn a_sequence_of_mappings_and_a_mapping_with_a_block_scalar_can_be_keys() {
    let doc: Value = from_str(&fixture("sequence-and-map-keys.yaml")).expect("parses");
    let m = doc["complex_matrix_dispatch"]
        .as_mapping()
        .expect("mapping");
    let keys: Vec<&str> = m.keys().map(String::as_str).collect();
    assert_eq!(keys.len(), 2);
    assert!(keys[0].starts_with("[{rule_id: 101"), "{}", keys[0]);
    // The literal block inside the key keeps its line break.
    assert!(keys[1].contains("#!/bin/sh\necho"), "{}", keys[1]);
    assert_eq!(m[keys[0]]["priority"].untag_ref().as_i64(), Some(999));
    assert_eq!(m[keys[1]]["verified"].untag_ref().as_bool(), Some(true));
}

#[test]
fn multibyte_keys_and_empty_typed_collections() {
    let doc: Value = from_str(&fixture("unicode-boundaries.yaml")).expect("parses");
    // Four-byte emoji either side of an ASCII run, and a Japanese key.
    let node = &doc["\u{1F31F}_mesh_node_\u{1F680}"];
    assert_eq!(node["status"].as_str(), Some("ONLINE"));
    assert_eq!(node["コンニチハ_world"].as_str(), Some("Japanese Key Test"));
    // A zero-width space is part of the key, not trimmed away.
    let key = doc["invisible_keys"]
        .as_mapping()
        .unwrap()
        .keys()
        .next()
        .unwrap();
    assert!(key.ends_with('\u{200b}'), "{key:?}");
    // A long plain scalar on one line stays one line.
    let marathon = doc["unquoted_marathon_scalar"].as_str().unwrap();
    assert_eq!(marathon.len(), 260);
    assert!(!marathon.contains('\n'));
    // Explicitly tagged empty collections keep their shape.
    let e = &doc["empty_typed_structures"];
    assert_eq!(
        e["empty_map"]
            .untag_ref()
            .as_mapping()
            .map(noyalib::Mapping::len),
        Some(0)
    );
    assert_eq!(
        e["empty_seq"].untag_ref().as_sequence().map(Vec::len),
        Some(0)
    );
    assert_eq!(e["empty_str"].untag_ref().as_str(), Some(""));
}

#[test]
fn a_node_may_carry_its_tag_and_anchor_in_either_order() {
    let doc: Value = from_str(&fixture("property-ordering.yaml")).expect("parses");
    let p = &doc["property_ordering_tests"];
    assert_eq!(
        p["case_one"].untag_ref().as_str(),
        Some("Tag then Anchor declaration")
    );
    assert_eq!(
        p["case_two"].untag_ref().as_str(),
        Some("Anchor then Tag declaration")
    );
    // Anchors declared beside a tag are still aliasable, in both orders.
    let r = &p["resolutions"];
    assert_eq!(
        r["copied_str"].untag_ref().as_str(),
        Some("Tag then Anchor declaration")
    );
    assert_eq!(r["copied_int"].untag_ref().as_i64(), Some(42));
    // The fixture's own digits, read from the fixture: the point is
    // that the anchored float round-trips, not what the number is.
    let written: f64 = fixture("property-ordering.yaml")
        .split("!!float &float_anchor ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("the anchored float")
        .parse()
        .expect("a float");
    assert_eq!(r["copied_flt"].untag_ref().as_f64(), Some(written));
}

#[test]
fn an_anchored_block_scalar_key_can_be_aliased_as_a_value() {
    let doc: Value = from_str(&fixture("anchored-multiline-keys.yaml")).expect("parses");
    let t = &doc["nested_key_torture"];
    let sql = "SELECT * FROM telemetry\nWHERE event = 'ERROR'";
    // The strip-chomped literal is the key, and the same anchor resolves
    // to that string when used as a value elsewhere.
    assert_eq!(t[sql]["status"].as_str(), Some("ACTIVE_MONITOR_QUERY"));
    assert_eq!(t["query_archive"]["historical_sql"].as_str(), Some(sql));
    // A mapping whose own key is a mapping, used as a key.
    assert_eq!(
        t["{{deep_key: true}: mid_level_value}"].as_str(),
        Some("outermost_value")
    );
}

#[test]
fn plain_scalars_fold_every_line_including_indented_ones() {
    let doc: Value = from_str(&fixture("plain-scalar-folding.yaml")).expect("parses");
    let s = doc["plain_text_folding_rules"]["implicit_concatenation"]
        .as_str()
        .unwrap();
    // A blank line becomes one newline; every other line joins with a
    // space, including the more-indented ones. Only a *folded block*
    // scalar keeps more-indented lines apart, and this is a plain
    // scalar. libyaml returns exactly this string.
    assert_eq!(s.matches('\n').count(), 1);
    assert!(s.contains("the one below\nforces a paragraph break"), "{s}");
    assert!(s.contains("like this line and this line might be"), "{s}");
}

#[test]
fn empty_documents_and_trailing_comments_close_the_stream() {
    let docs = noyalib::load_all_as::<Value>(&fixture("stream-boundaries.yaml")).expect("parses");
    // `---` `---` `---` gives two empty documents before the real one,
    // and the comments after the final `...` end the stream cleanly.
    assert_eq!(docs.len(), 3);
    assert!(docs[0].is_null());
    assert!(docs[1].is_null());
    assert_eq!(
        docs[2]["valid_document_after_empty_breaks"]["stream_status"].as_str(),
        Some("STABLE")
    );
}

// ── Five that separate the implementations ────────────────────────────

#[test]
fn the_billion_laughs_payload_is_refused_by_the_budget() {
    // Seven levels of ten-fold alias expansion. The budget stops it
    // instead of expanding it. Of the three implementations checked,
    // only this one refuses: libyaml exhausts memory and go-yaml did
    // not finish inside a minute.
    let start = std::time::Instant::now();
    let err = from_str::<Value>(&fixture("billion-laughs.yaml")).expect_err("refused");
    let elapsed = start.elapsed();
    assert!(
        err.to_string().contains("alias expansion limit exceeded"),
        "{err}"
    );
    // Refusing has to be quick; the point of the budget is that the
    // work is bounded, not merely finite.
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "took {elapsed:?}"
    );
}

#[test]
fn the_core_schema_leaves_the_norway_mines_alone() {
    // YAML 1.1 resolved `no`, `NO`, `off`, `n` to booleans and
    // `190:20:30` to a sexagesimal integer. YAML 1.2's core schema
    // dropped all of it, so every one of these stays a string. libyaml
    // still reads 1.1 here and turns `NO` into false; go-yaml agrees
    // with this crate.
    let doc: Value = from_str(&fixture("norway-and-legacy-coercion.yaml")).expect("parses");
    let c = &doc["legacy_coercion_checks"];
    assert_eq!(c["country_codes"][2].as_str(), Some("NO"));
    let b = &c["implicit_strings_vs_booleans"];
    for key in [
        "string_yes",
        "string_no",
        "string_on",
        "string_off",
        "string_y",
        "string_n",
    ] {
        assert!(
            b[key].as_str().is_some(),
            "{key} was coerced to {:?}",
            b[key]
        );
        assert!(b[key].as_bool().is_none(), "{key} was coerced to a bool");
    }
    // The two spellings the core schema does resolve.
    assert_eq!(b["actual_boolean_true"].as_bool(), Some(true));
    assert_eq!(b["actual_boolean_false"].as_bool(), Some(false));
    // Sexagesimal is gone: 190:20:30 is text, not 685230.
    assert_eq!(c["base_60_test"].as_str(), Some("190:20:30"));
}

#[test]
fn a_block_scalar_cannot_open_inside_a_flow_collection() {
    // All three implementations refuse this; only the wording differs.
    let err = from_str::<Value>(&fixture("block-scalar-inside-flow.yaml")).expect_err("refused");
    let msg = err.to_string();
    assert!(msg.contains("block scalar indicator"), "{msg}");
    assert!(
        msg.contains("not allowed inside a flow collection"),
        "{msg}"
    );
    assert_eq!(err.location().map(|l| l.line()), Some(12), "{msg}");
}

#[test]
fn a_colon_inside_a_plain_scalar_is_not_a_separator() {
    let doc: Value = from_str(&fixture("colon-boundaries.yaml")).expect("parses");
    let b = &doc["colon_boundary_conditions"];
    // `:` only separates when a space follows it, so a URL and a
    // Windows path survive unquoted.
    assert_eq!(b["url_path"].as_str(), Some("https://api.internal"));
    assert_eq!(
        b["windows_drive"].as_str(),
        Some(r"C:\ProgramFiles\App\config.bin")
    );
    // A key with nothing after its colon holds null, and its sibling is
    // still a sibling.
    let n = &b["nested_empty_value_dispatch"];
    assert!(n["key_with_trailing_colon"].is_null());
    assert_eq!(n["next_sibling_key"].as_str(), Some("value"));
    // The empty string is a usable key.
    assert_eq!(b[""]["nested_under_empty_key"].as_bool(), Some(true));
}

#[test]
fn an_explicit_key_may_be_a_multi_line_plain_scalar() {
    let doc: Value = from_str(&fixture("lookahead-plain-key.yaml")).expect("parses");
    let m = doc.as_mapping().expect("mapping");
    let (key, value) = m.iter().next().expect("one entry");
    // Four source lines folded into one key of 317 characters.
    assert_eq!(key.len(), 317);
    assert!(
        !key.contains('\n'),
        "the plain scalar key kept a line break"
    );
    assert!(key.starts_with("This is an incredibly lengthy unquoted plain scalar key"));
    assert!(key.ends_with("the target colon separator below."));
    assert_eq!(value.as_str(), Some("value_bound_to_the_max_lookahead_key"));
}
