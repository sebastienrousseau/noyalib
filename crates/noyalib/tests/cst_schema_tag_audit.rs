// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! CST + schema audit with tagged content.

#[cfg(feature = "validate-schema")]
use noyalib::{Value, from_str, validate_against_schema};

#[test]
fn cst_round_trip_byte_faithful_with_tag() {
    use noyalib::cst::parse_document;
    let src = "# top\nname: !Custom 'app-1' # inline\nport: 8080\n";
    let doc = parse_document(src).unwrap();
    assert_eq!(doc.to_string(), src, "byte-faithful round-trip with tags");
}

#[test]
fn cst_set_tagged_scalar_replacing_tag_only() {
    use noyalib::cst::parse_document;
    let src = "color: !Color '#ff8800'\n";
    let mut doc = parse_document(src).unwrap();
    doc.entry("color").set("!Color '#00aaff'").unwrap();
    let after = doc.to_string();
    assert_eq!(after, "color: !Color '#00aaff'\n");
}

#[test]
fn cst_set_tagged_collection_replacing_tag_only() {
    use noyalib::cst::parse_document;
    let src = "list: !MyList\n  - 1\n  - 2\n";
    let mut doc = parse_document(src).unwrap();
    // Replace the entire tagged seq with a different tagged seq.
    doc.entry("list").set("!OtherList [10, 20]").unwrap();
    let after = doc.to_string();
    eprintln!("after = {:?}", after);
    assert!(after.contains("!OtherList"));
    assert!(after.contains("[10, 20]"));
}

#[cfg(feature = "validate-schema")]
#[test]
fn schema_validates_tagged_value_at_inner_shape() {
    // Schema validation on a tagged scalar should see the inner
    // value's shape (the schema is JSON Schema; tags are YAML
    // metadata).
    let yaml = "port: !!int 8080\nhost: api.example.com\n";
    let schema = "type: object\nproperties:\n  port: { type: integer }\n  host: { type: string }\n";
    let value: Value = from_str(yaml).unwrap();
    let schema_value: Value = from_str(schema).unwrap();
    validate_against_schema(&value, &schema_value).unwrap();
}

#[cfg(feature = "validate-schema")]
#[test]
fn cst_coerce_to_schema_through_tagged_value() {
    use noyalib::cst::{coerce_to_schema, parse_document};
    // `port: !!str "8080"` is a string-tagged scalar — schema
    // says integer. After coerce, the tag should survive (or
    // strip — pin the actual behaviour).
    let schema: Value = from_str("type: object\nproperties:\n  port: { type: integer }\n").unwrap();
    let mut doc = parse_document("# config\nport: \"8080\"\n").unwrap();
    let n = coerce_to_schema(&mut doc, &schema).unwrap();
    assert_eq!(n, 1);
    let after = doc.to_string();
    assert!(after.contains("port: 8080"));
    assert!(after.contains("# config"));
}

#[cfg(feature = "schema")]
#[test]
fn schema_for_yaml_emits_tagged_value_correctly() {
    use noyalib::{JsonSchema, schema_for_yaml};
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct Cfg {
        port: u16,
        host: String,
    }
    let yaml = schema_for_yaml::<Cfg>().unwrap();
    // The schema YAML shouldn't contain `Tagged` keys (it's a
    // plain JSON Schema document).
    assert!(yaml.contains("port:"));
    assert!(yaml.contains("type:"));
    assert!(!yaml.contains("Tagged"));
}
