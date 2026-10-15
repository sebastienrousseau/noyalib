// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Regression tests for the v0.0.58 3-phase security audit findings.

use noyalib::{
    BudgetBreach, DuplicateKeyPolicy, Error, ParserConfig, Spanned, Value, from_str_with_config,
    load_all_as_with_config, load_all_with_config, policy::Policy, read_with_config,
};

#[cfg(feature = "std")]
#[test]
fn cst_set_respects_yaml_11_boolean_config_and_blocks_key_smuggling() {
    let cfg = ParserConfig::new()
        .legacy_booleans(true)
        .duplicate_key_policy(DuplicateKeyPolicy::First);
    let mut doc = noyalib::cst::parse_document_with_config("flag: yes\nother: 1\n", &cfg).unwrap();

    // Legitimate edit on a sibling key must not trip a false-positive shape
    // mismatch because the oracle re-parsed `yes` under YAML 1.2 defaults.
    doc.set("other", "2").unwrap();
    assert_eq!(doc.as_value()["flag"], Value::Bool(true));
    assert_eq!(doc.as_value()["other"], Value::from(2));

    // Smuggling a YAML 1.1 boolean alias (`yes`) that collides with an
    // existing `true` key must be caught under the document's config.
    let mut doc2 =
        noyalib::cst::parse_document_with_config("true: 1\ntarget: old\n", &cfg).unwrap();
    let res = doc2.set("target", "new\nyes: 2");
    assert!(
        res.is_err(),
        "expected duplicate key smuggling via YAML 1.1 boolean alias to be rejected: {res:?}"
    );
}

#[cfg(feature = "std")]
#[test]
fn cst_insert_and_collection_edits_preserve_document_config() {
    let cfg = ParserConfig::new().legacy_booleans(true);
    let mut doc =
        noyalib::cst::parse_document_with_config("flag: yes\nitems:\n  - zero\n", &cfg).unwrap();
    doc.push_back("items", "first").unwrap();
    doc.insert_after("items[0]", "on").unwrap();
    doc.insert_entry("", "added", "on").unwrap();
    assert_eq!(doc.as_value()["flag"], Value::Bool(true));
    assert_eq!(doc.as_value()["items"][1], Value::Bool(true));
    assert_eq!(doc.as_value()["added"], Value::Bool(true));
}

#[cfg(feature = "validate-schema")]
#[test]
fn coerce_to_schema_quotes_dotted_keys_and_avoids_noop_amplification() {
    let yaml = "\"a.b\": \"42\"\na:\n  b: \"keep\"\n";
    let mut doc = noyalib::cst::parse_document(yaml).unwrap();
    let schema: Value = noyalib::from_str(
        "type: object\nproperties:\n  \"a.b\":\n    type: integer\n    const: 99\n",
    )
    .unwrap();

    let applied = noyalib::cst::coerce_to_schema(&mut doc, &schema).unwrap();
    assert_eq!(
        applied, 1,
        "coercion must run once and stop when coerced == current"
    );
    assert_eq!(doc.as_value()["a.b"], Value::from(42));
    assert_eq!(doc.as_value()["a"]["b"], Value::String("keep".into()));
}

#[cfg(feature = "include")]
#[test]
fn include_enforces_cumulative_max_total_scalar_bytes() {
    use noyalib::include::{IncludeRequest, IncludeResolver, InputSource};

    let resolver = IncludeResolver::new(|req: IncludeRequest<'_>| {
        let payload = "discarded: \"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\"\nkeep: \"yyyyyyyyyyyyyyyyyyyy\"\n";
        Ok(InputSource::new(req.spec, payload))
    });
    let cfg = ParserConfig::new()
        .include_resolver(resolver)
        .max_total_scalar_bytes(80);
    let yaml = "first: !include a.yaml#keep\nsecond: !include b.yaml#keep\n";
    let err = from_str_with_config::<Value>(yaml, &cfg).unwrap_err();
    assert!(
        matches!(err, Error::Budget(BudgetBreach::MaxTotalScalarBytes { .. })),
        "expected MaxTotalScalarBytes across included sources, got {err:?}"
    );
}

#[cfg(feature = "tokio")]
#[test]
fn yaml_decoder_defers_chunk_boundary_triple_dash_until_lookahead() {
    use bytes::BytesMut;
    use tokio_util::codec::Decoder as _;

    let mut decoder: noyalib::tokio_async::YamlDecoder<Value> =
        noyalib::tokio_async::YamlDecoder::new();
    let mut buf = BytesMut::from(&b"a: 1\n---"[..]);

    // At a non-EOF chunk boundary, `---` at the end of the buffer may be the
    // prefix of `---: 2` and must wait for lookahead.
    assert!(decoder.decode(&mut buf).unwrap().is_none());

    buf.extend_from_slice(b": 2\n---\nb: 3\n");
    let first = decoder.decode(&mut buf).unwrap().unwrap();
    assert_eq!(first["a"], Value::from(1));
    assert_eq!(first["---"], Value::from(2));

    let second = decoder.decode_eof(&mut buf).unwrap().unwrap();
    assert_eq!(second["b"], Value::from(3));
}

#[cfg(feature = "std")]
#[test]
fn cst_and_multi_doc_loaders_enforce_policies_and_options_parity() {
    use std::collections::HashMap;
    use std::sync::Arc;

    #[derive(Debug)]
    struct ForbidSecretKey;
    impl Policy for ForbidSecretKey {
        fn check_value(&self, value: &Value) -> noyalib::Result<()> {
            if value.as_mapping().is_some_and(|m| m.contains_key("secret")) {
                return Err(Error::Custom("secret key forbidden by policy".into()));
            }
            Ok(())
        }
    }

    let mut props = HashMap::new();
    let _ = props.insert("ENV".to_owned(), "prod".to_owned());
    let cfg = ParserConfig::new()
        .with_policy(ForbidSecretKey)
        .properties(Arc::new(props))
        .plain_scalar_strings(true);

    // CST parse + edit must enforce Policy::check_value.
    assert!(noyalib::cst::parse_document_with_config("secret: 1\n", &cfg).is_err());
    let mut doc = noyalib::cst::parse_document_with_config("safe: 1\n", &cfg).unwrap();
    assert!(doc.set("safe", "2\nsecret: 1").is_err());

    // Multi-document and reader loaders must expand properties and honour
    // plain_scalar_strings + SpanContext.
    let multi_yaml = "env: ${ENV}\nflag: true\n---\nenv: ${ENV}\nflag: false\n";
    let loaded: Vec<Value> = load_all_with_config(multi_yaml, &cfg)
        .unwrap()
        .collect::<noyalib::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(loaded[0]["env"], Value::String("prod".into()));
    assert_eq!(loaded[0]["flag"], Value::Bool(true));

    #[derive(Debug, serde::Deserialize)]
    struct Item {
        env: Spanned<String>,
        flag: String,
    }

    let typed: Vec<Item> = load_all_as_with_config(multi_yaml, &cfg).unwrap();
    assert_eq!(typed[0].env.value, "prod");
    assert_eq!(typed[0].flag, "true");
    assert_eq!(typed[0].env.start.line(), 1);

    let from_reader: Vec<Item> = read_with_config(std::io::Cursor::new(multi_yaml), &cfg)
        .unwrap()
        .collect::<noyalib::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(from_reader[0].env.value, "prod");
    assert_eq!(from_reader[0].flag, "true");
    assert_eq!(from_reader[1].env.start.line(), 4);
}

#[cfg(feature = "std")]
#[test]
fn property_interpolation_enforces_max_total_scalar_bytes() {
    use std::collections::HashMap;
    use std::sync::Arc;

    let mut props = HashMap::new();
    let _ = props.insert("BIG".to_owned(), "x".repeat(128));
    let cfg = ParserConfig::new()
        .properties(Arc::new(props))
        .max_total_scalar_bytes(200);
    let yaml = "a: ${BIG}${BIG}\n";
    let err = from_str_with_config::<Value>(yaml, &cfg).unwrap_err();
    assert!(
        matches!(err, Error::Budget(BudgetBreach::MaxTotalScalarBytes { .. })),
        "expected MaxTotalScalarBytes on property expansion, got {err:?}"
    );
}

#[cfg(all(feature = "std", feature = "figment"))]
#[test]
fn figment_file_rejects_non_regular_and_oversized_files() {
    use figment::Figment;
    use figment::providers::Format as _;
    use noyalib::figment::Yaml;

    let temp_dir =
        std::env::temp_dir().join(format!("noyalib-figment-audit-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);

    let dir_err = Figment::from(Yaml::file_with_config(&temp_dir, ParserConfig::new()))
        .extract::<serde_json::Value>();
    assert!(
        dir_err.is_err(),
        "directory must be rejected as non-regular file"
    );
    assert!(Yaml::from_path::<serde_json::Value>(&temp_dir).is_err());

    let file_path = temp_dir.join("oversized.yaml");
    std::fs::write(&file_path, "key: value_that_is_longer_than_eight_bytes\n").unwrap();
    let cfg = ParserConfig::new().max_document_length(8);
    let size_err =
        Figment::from(Yaml::file_with_config(&file_path, cfg)).extract::<serde_json::Value>();
    let _ = std::fs::remove_dir_all(&temp_dir);
    assert!(
        size_err.is_err(),
        "oversized file must be rejected by metadata check"
    );
}
