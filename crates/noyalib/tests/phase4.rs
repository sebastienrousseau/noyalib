//! Phase 4 feature tests: SerializerConfig, block scalars, path navigation.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{SerializerConfig, Value, from_str, to_string, to_string_with_config};
use serde::{Deserialize, Serialize};

// ============================================================================
// SerializerConfig Tests
// ============================================================================

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SimpleConfig {
    name: String,
    port: u16,
}

#[test]
fn test_default_config() {
    let config = SimpleConfig {
        name: "test".to_string(),
        port: 8080,
    };

    let yaml = to_string(&config).unwrap();
    assert!(yaml.contains("name: test"));
    assert!(yaml.contains("port: 8080"));
    // Default: no document markers
    assert!(!yaml.starts_with("---"));
}

#[test]
fn test_document_start_marker() {
    let config = SimpleConfig {
        name: "test".to_string(),
        port: 8080,
    };

    let yaml =
        to_string_with_config(&config, &SerializerConfig::new().document_start(true)).unwrap();

    assert!(yaml.starts_with("---\n"));
}

#[test]
fn test_document_end_marker() {
    let config = SimpleConfig {
        name: "test".to_string(),
        port: 8080,
    };

    let yaml = to_string_with_config(&config, &SerializerConfig::new().document_end(true)).unwrap();

    assert!(yaml.ends_with("\n..."));
}

#[test]
fn test_both_document_markers() {
    let config = SimpleConfig {
        name: "test".to_string(),
        port: 8080,
    };

    let yaml = to_string_with_config(
        &config,
        &SerializerConfig::new()
            .document_start(true)
            .document_end(true),
    )
    .unwrap();

    assert!(yaml.starts_with("---\n"));
    assert!(yaml.ends_with("\n..."));
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Nested {
    outer: Inner,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Inner {
    value: i32,
}

#[test]
fn test_custom_indent_2() {
    let data = Nested {
        outer: Inner { value: 42 },
    };

    let yaml = to_string_with_config(&data, &SerializerConfig::new().indent(2)).unwrap();

    // With 2-space indent (default), verify it parses back correctly
    let parsed: Nested = from_str(&yaml).unwrap();
    assert_eq!(parsed, data);
    // Check indentation is 2 spaces
    assert!(yaml.contains("  value:"));
}

#[test]
fn test_custom_indent_4() {
    let data = Nested {
        outer: Inner { value: 42 },
    };

    let yaml = to_string_with_config(&data, &SerializerConfig::new().indent(4)).unwrap();

    // With 4-space indent, verify it parses back correctly
    let parsed: Nested = from_str(&yaml).unwrap();
    assert_eq!(parsed, data);
    // Check indentation is 4 spaces
    assert!(yaml.contains("    value:"));
}

// ============================================================================
// Block Scalar Tests
// ============================================================================

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ScriptConfig {
    name: String,
    script: String,
}

#[test]
fn test_block_scalar_multiline() {
    let config = ScriptConfig {
        name: "build".to_string(),
        script: "echo hello\necho world\n".to_string(),
    };

    let yaml =
        to_string_with_config(&config, &SerializerConfig::new().block_scalars(true)).unwrap();

    // Should use literal block style
    assert!(yaml.contains("script: |"));

    // Verify round-trip
    let parsed: ScriptConfig = from_str(&yaml).unwrap();
    assert_eq!(parsed, config);
}

#[test]
fn test_block_scalar_disabled() {
    let config = ScriptConfig {
        name: "build".to_string(),
        script: "echo hello\necho world".to_string(),
    };

    let yaml =
        to_string_with_config(&config, &SerializerConfig::new().block_scalars(false)).unwrap();

    // Should use quoted style instead
    assert!(yaml.contains("script: \""));
    assert!(!yaml.contains("script: |"));
}

#[test]
fn test_block_scalar_single_line_no_block() {
    let config = ScriptConfig {
        name: "build".to_string(),
        script: "echo hello".to_string(),
    };

    let yaml =
        to_string_with_config(&config, &SerializerConfig::new().block_scalars(true)).unwrap();

    // Single line shouldn't use block style
    assert!(!yaml.contains("script: |"));
}

#[test]
fn test_block_scalar_threshold() {
    let config = ScriptConfig {
        name: "build".to_string(),
        script: "line1\nline2".to_string(), // Only 1 newline
    };

    // With threshold of 2, should not use block style
    let yaml = to_string_with_config(
        &config,
        &SerializerConfig::new()
            .block_scalars(true)
            .block_scalar_threshold(2),
    )
    .unwrap();

    assert!(!yaml.contains("script: |"));
}

#[test]
fn test_block_scalar_preserves_content() {
    let original = "#!/bin/bash\nset -e\necho \"Hello, World!\"\nexit 0\n";
    let config = ScriptConfig {
        name: "deploy".to_string(),
        script: original.to_string(),
    };

    let yaml = to_string(&config).unwrap();
    let parsed: ScriptConfig = from_str(&yaml).unwrap();

    assert_eq!(parsed.script, original);
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[test]
fn test_full_config_combination() {
    let config = ScriptConfig {
        name: "full-test".to_string(),
        script: "step1\nstep2\nstep3\n".to_string(),
    };

    let yaml = to_string_with_config(
        &config,
        &SerializerConfig::new()
            .indent(4)
            .document_start(true)
            .document_end(true)
            .block_scalars(true),
    )
    .unwrap();

    assert!(yaml.starts_with("---\n"));
    assert!(yaml.ends_with("\n..."));
    assert!(yaml.contains("script: |"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_string_not_block() {
    #[derive(Serialize)]
    struct Empty {
        value: String,
    }

    let data = Empty {
        value: String::new(),
    };

    let yaml = to_string(&data).unwrap();
    // Empty string should be quoted, not block
    assert!(yaml.contains("value: \"\""));
}

#[test]
fn test_config_builder_chaining() {
    let config = SerializerConfig::new()
        .indent(3)
        .document_start(true)
        .document_end(false)
        .block_scalars(true)
        .block_scalar_threshold(2);

    assert_eq!(config.indent, 3);
    assert!(config.document_start);
    assert!(!config.document_end);
    assert!(config.block_scalars);
    assert_eq!(config.block_scalar_threshold, 2);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DeepNested {
    level1: Level1,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Level1 {
    level2: Level2,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Level2 {
    value: String,
}

#[test]
fn test_deep_nesting_with_custom_indent() {
    let data = DeepNested {
        level1: Level1 {
            level2: Level2 {
                value: "deep".to_string(),
            },
        },
    };

    let yaml_2 = to_string_with_config(&data, &SerializerConfig::new().indent(2)).unwrap();
    let yaml_4 = to_string_with_config(&data, &SerializerConfig::new().indent(4)).unwrap();

    // Both should parse back correctly
    let parsed_2: DeepNested = from_str(&yaml_2).unwrap();
    let parsed_4: DeepNested = from_str(&yaml_4).unwrap();

    assert_eq!(parsed_2, data);
    assert_eq!(parsed_4, data);
}

// ============================================================================
// Path Navigation Tests
// ============================================================================

#[test]
fn test_get_path_simple() {
    let yaml = r#"
server:
  host: localhost
  port: 8080
"#;
    let value: Value = from_str(yaml).unwrap();

    assert_eq!(
        value.get_path("server.host").unwrap().as_str(),
        Some("localhost")
    );
    assert_eq!(value.get_path("server.port").unwrap().as_i64(), Some(8080));
}

#[test]
fn test_get_path_array_index() {
    let yaml = r#"
items:
  - first
  - second
  - third
"#;
    let value: Value = from_str(yaml).unwrap();

    assert_eq!(value.get_path("items[0]").unwrap().as_str(), Some("first"));
    assert_eq!(value.get_path("items[1]").unwrap().as_str(), Some("second"));
    assert_eq!(value.get_path("items[2]").unwrap().as_str(), Some("third"));
}

#[test]
fn test_get_path_nested_array() {
    let yaml = r#"
items:
  - name: first
    value: 1
  - name: second
    value: 2
"#;
    let value: Value = from_str(yaml).unwrap();

    assert_eq!(
        value.get_path("items[0].name").unwrap().as_str(),
        Some("first")
    );
    assert_eq!(value.get_path("items[0].value").unwrap().as_i64(), Some(1));
    assert_eq!(
        value.get_path("items[1].name").unwrap().as_str(),
        Some("second")
    );
    assert_eq!(value.get_path("items[1].value").unwrap().as_i64(), Some(2));
}

#[test]
fn test_get_path_deep_nesting() {
    let yaml = r#"
level1:
  level2:
    level3:
      value: deep
"#;
    let value: Value = from_str(yaml).unwrap();

    assert_eq!(
        value
            .get_path("level1.level2.level3.value")
            .unwrap()
            .as_str(),
        Some("deep")
    );
}

#[test]
fn test_get_path_not_found() {
    let yaml = "name: test\n";
    let value: Value = from_str(yaml).unwrap();

    assert!(value.get_path("nonexistent").is_none());
    assert!(value.get_path("name.nested").is_none());
}

#[test]
fn test_get_path_invalid_index() {
    let yaml = r#"
items:
  - first
"#;
    let value: Value = from_str(yaml).unwrap();

    assert!(value.get_path("items[99]").is_none());
}

#[test]
fn test_get_path_mut() {
    let yaml = r#"
server:
  port: 8080
"#;
    let mut value: Value = from_str(yaml).unwrap();

    if let Some(port) = value.get_path_mut("server.port") {
        *port = Value::from(9090);
    }

    assert_eq!(value.get_path("server.port").unwrap().as_i64(), Some(9090));
}

#[test]
fn test_get_path_array_mut() {
    let yaml = r#"
items:
  - old
"#;
    let mut value: Value = from_str(yaml).unwrap();

    if let Some(item) = value.get_path_mut("items[0]") {
        *item = Value::from("new");
    }

    assert_eq!(value.get_path("items[0]").unwrap().as_str(), Some("new"));
}

#[test]
fn test_get_path_empty() {
    let yaml = "value: 42\n";
    let value: Value = from_str(yaml).unwrap();

    // Empty path returns the root value itself
    let result = value.get_path("");
    assert!(result.is_some());
    assert!(result.unwrap().is_mapping());
}

#[test]
fn test_get_path_consecutive_dots() {
    let yaml = "a:\n  b: 1\n";
    let value: Value = from_str(yaml).unwrap();

    // "a..b" - empty segments between dots are skipped
    // so this is equivalent to "a.b"
    assert_eq!(value.get_path("a..b").unwrap().as_i64(), Some(1));
}
