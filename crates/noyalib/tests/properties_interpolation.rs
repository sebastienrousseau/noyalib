// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `ParserConfig::properties` — `${KEY}` / `${KEY:-default}` /
//! `$$` / `${{` / `}}` substitution during parse.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::{from_str_with_config, ParserConfig, Value};
use std::collections::HashMap;
use std::sync::Arc;

fn props(pairs: &[(&str, &str)]) -> Arc<HashMap<String, String>> {
    let mut m = HashMap::new();
    for (k, v) in pairs {
        let _ = m.insert((*k).to_string(), (*v).to_string());
    }
    Arc::new(m)
}

#[test]
fn basic_substitution() {
    let cfg = ParserConfig::new().properties(props(&[("HOST", "localhost"), ("PORT", "8080")]));
    let v: Value = from_str_with_config("url: http://${HOST}:${PORT}/", &cfg).unwrap();
    assert_eq!(v["url"].as_str(), Some("http://localhost:8080/"));
}

#[test]
fn missing_key_default_value() {
    let cfg = ParserConfig::new().properties(props(&[]));
    let v: Value =
        from_str_with_config("level: ${LOG_LEVEL:-info}\nport: ${PORT:-3000}\n", &cfg).unwrap();
    assert_eq!(v["level"].as_str(), Some("info"));
    assert_eq!(v["port"].as_str(), Some("3000"));
}

#[test]
fn known_key_overrides_default() {
    let cfg = ParserConfig::new().properties(props(&[("LOG_LEVEL", "trace")]));
    let v: Value = from_str_with_config("level: ${LOG_LEVEL:-info}\n", &cfg).unwrap();
    assert_eq!(v["level"].as_str(), Some("trace"));
}

#[test]
fn strict_mode_errors_on_missing_key() {
    let cfg = ParserConfig::new()
        .properties(props(&[]))
        .strict_properties(true);
    let res: Result<Value, _> = from_str_with_config("x: ${MISSING}\n", &cfg);
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("MISSING"));
}

#[test]
fn lossy_mode_substitutes_missing_with_empty_string() {
    let cfg = ParserConfig::new().properties(props(&[]));
    let v: Value = from_str_with_config("x: ${MISSING}\n", &cfg).unwrap();
    assert_eq!(v["x"].as_str(), Some(""));
}

#[test]
fn dollar_dollar_escapes_literal() {
    let cfg = ParserConfig::new().properties(props(&[]));
    let v: Value = from_str_with_config("price: $$5.00\n", &cfg).unwrap();
    assert_eq!(v["price"].as_str(), Some("$5.00"));
}

#[test]
fn double_brace_escapes_literal_open_delim() {
    let cfg = ParserConfig::new().properties(props(&[]));
    let v: Value = from_str_with_config("template: \"${{name}\"\n", &cfg).unwrap();
    assert_eq!(v["template"].as_str(), Some("${name}"));
}

#[test]
fn nested_mapping_walks_recursively() {
    let cfg = ParserConfig::new().properties(props(&[("NAME", "noyalib"), ("VER", "0.0.2")]));
    let yaml = "service:\n  name: ${NAME}\n  versions:\n    - ${VER}\n    - ${VER}-rc1\n";
    let v: Value = from_str_with_config(yaml, &cfg).unwrap();
    assert_eq!(v["service"]["name"].as_str(), Some("noyalib"));
    let versions = v["service"]["versions"].as_sequence().unwrap();
    assert_eq!(versions[0].as_str(), Some("0.0.2"));
    assert_eq!(versions[1].as_str(), Some("0.0.2-rc1"));
}

#[test]
fn no_properties_set_is_a_noop() {
    let cfg = ParserConfig::new();
    let v: Value = from_str_with_config("x: ${LITERAL}\n", &cfg).unwrap();
    assert_eq!(v["x"].as_str(), Some("${LITERAL}"));
}

#[test]
fn strict_config_defaults_strict_properties_true() {
    let cfg = ParserConfig::strict().properties(props(&[]));
    let res: Result<Value, _> = from_str_with_config("x: ${MISSING}\n", &cfg);
    assert!(
        res.is_err(),
        "ParserConfig::strict() must set strict_properties=true"
    );
}

#[test]
fn typed_target_sees_substituted_value() {
    use serde::Deserialize;
    #[derive(Debug, Deserialize)]
    struct Cfg {
        url: String,
    }
    let cfg = ParserConfig::new().properties(props(&[("HOST", "127.0.0.1")]));
    let parsed: Cfg = from_str_with_config("url: http://${HOST}/api\n", &cfg).unwrap();
    assert_eq!(parsed.url, "http://127.0.0.1/api");
}

#[test]
fn invalid_placeholder_character_errors() {
    let cfg = ParserConfig::new().properties(props(&[("FOO", "bar")]));
    // Quote so the space stays inside the placeholder rather than
    // being consumed by YAML's plain-scalar trim rules.
    let res: Result<Value, _> = from_str_with_config("x: \"${FOO BAR}\"\n", &cfg);
    assert!(res.is_err(), "{:?}", res);
}

#[test]
fn unterminated_placeholder_errors() {
    let cfg = ParserConfig::new().properties(props(&[]));
    let res: Result<Value, _> = from_str_with_config("x: \"${FOO\"\n", &cfg);
    assert!(res.is_err());
}

#[test]
fn malformed_default_separator_errors() {
    // `:x` without `-` is rejected so the syntax stays unambiguous.
    let cfg = ParserConfig::new().properties(props(&[]));
    let res: Result<Value, _> = from_str_with_config("x: ${FOO:hello}\n", &cfg);
    assert!(res.is_err());
}
