//! Phase 3 feature tests: singleton_map helper modules.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::collections::BTreeMap;

use noyalib::{from_str, to_string};

// ============================================================================
// singleton_map Tests
// ============================================================================

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
enum Action {
    Start,
    Stop,
    Restart { delay: u32 },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct Task {
    name: String,
    #[serde(with = "noyalib::with::singleton_map")]
    action: Action,
}

#[test]
fn test_singleton_map_unit_variant() {
    let task = Task {
        name: "service".to_string(),
        action: Action::Start,
    };

    let yaml = to_string(&task).unwrap();
    let parsed: Task = from_str(&yaml).unwrap();
    assert_eq!(parsed, task);
}

#[test]
fn test_singleton_map_struct_variant() {
    let task = Task {
        name: "service".to_string(),
        action: Action::Restart { delay: 5 },
    };

    let yaml = to_string(&task).unwrap();
    assert!(yaml.contains("Restart"));
    assert!(yaml.contains("delay: 5"));

    let parsed: Task = from_str(&yaml).unwrap();
    assert_eq!(parsed, task);
}

// ============================================================================
// singleton_map_optional Tests
// ============================================================================

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct OptionalTask {
    name: String,
    #[serde(
        with = "noyalib::with::singleton_map_optional",
        skip_serializing_if = "Option::is_none",
        default
    )]
    action: Option<Action>,
}

#[test]
fn test_singleton_map_optional_some() {
    let task = OptionalTask {
        name: "service".to_string(),
        action: Some(Action::Stop),
    };

    let yaml = to_string(&task).unwrap();
    let parsed: OptionalTask = from_str(&yaml).unwrap();
    assert_eq!(parsed, task);
}

#[test]
fn test_singleton_map_optional_none() {
    let task = OptionalTask {
        name: "service".to_string(),
        action: None,
    };

    let yaml = to_string(&task).unwrap();
    // Field should not appear due to skip_serializing_if
    assert!(!yaml.contains("action"));

    let parsed: OptionalTask = from_str(&yaml).unwrap();
    assert_eq!(parsed, task);
}

#[test]
fn test_singleton_map_optional_struct_variant() {
    let task = OptionalTask {
        name: "service".to_string(),
        action: Some(Action::Restart { delay: 10 }),
    };

    let yaml = to_string(&task).unwrap();
    assert!(yaml.contains("Restart"));

    let parsed: OptionalTask = from_str(&yaml).unwrap();
    assert_eq!(parsed, task);
}

// ============================================================================
// singleton_map_recursive Tests
// ============================================================================

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
enum Status {
    Active,
    Inactive,
    Error { code: i32 },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct ServiceList {
    name: String,
    #[serde(with = "noyalib::with::singleton_map_recursive")]
    services: Vec<Status>,
}

#[test]
fn test_singleton_map_recursive_vec() {
    let list = ServiceList {
        name: "cluster".to_string(),
        services: vec![Status::Active, Status::Error { code: 500 }],
    };

    let yaml = to_string(&list).unwrap();
    let parsed: ServiceList = from_str(&yaml).unwrap();
    assert_eq!(parsed, list);
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct ServiceMap {
    name: String,
    #[serde(with = "noyalib::with::singleton_map_recursive")]
    services: BTreeMap<String, Status>,
}

#[test]
fn test_singleton_map_recursive_map() {
    let mut services = BTreeMap::new();
    let _ = services.insert("web".to_string(), Status::Active);
    let _ = services.insert("db".to_string(), Status::Error { code: 503 });

    let map = ServiceMap {
        name: "cluster".to_string(),
        services,
    };

    let yaml = to_string(&map).unwrap();
    let parsed: ServiceMap = from_str(&yaml).unwrap();
    assert_eq!(parsed, map);
}

// ============================================================================
// nested_singleton_map Alias Tests
// ============================================================================

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct AliasTest {
    #[serde(with = "noyalib::with::nested_singleton_map")]
    items: Vec<Status>,
}

#[test]
fn test_nested_singleton_map_alias() {
    let test = AliasTest {
        items: vec![Status::Active, Status::Inactive],
    };

    let yaml = to_string(&test).unwrap();
    let parsed: AliasTest = from_str(&yaml).unwrap();
    assert_eq!(parsed, test);
}

// ============================================================================
// Complex Nested Structures
// ============================================================================

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
enum Command {
    Run { script: String },
    Wait { seconds: u32 },
    Parallel(Vec<Command>),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct Pipeline {
    name: String,
    #[serde(with = "noyalib::with::singleton_map_recursive")]
    steps: Vec<Command>,
}

#[test]
fn test_deeply_nested_enums() {
    let pipeline = Pipeline {
        name: "build".to_string(),
        steps: vec![
            Command::Run {
                script: "echo hello".to_string(),
            },
            Command::Parallel(vec![
                Command::Run {
                    script: "test1".to_string(),
                },
                Command::Run {
                    script: "test2".to_string(),
                },
            ]),
            Command::Wait { seconds: 5 },
        ],
    };

    let yaml = to_string(&pipeline).unwrap();
    let parsed: Pipeline = from_str(&yaml).unwrap();
    assert_eq!(parsed, pipeline);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_collections() {
    let list = ServiceList {
        name: "empty".to_string(),
        services: vec![],
    };

    let yaml = to_string(&list).unwrap();
    let parsed: ServiceList = from_str(&yaml).unwrap();
    assert_eq!(parsed, list);
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
enum Mixed {
    StringVal(String),
    IntVal(i32),
    Nested { inner: Box<Mixed> },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct MixedContainer {
    #[serde(with = "noyalib::with::singleton_map")]
    value: Mixed,
}

#[test]
fn test_newtype_variants() {
    let container = MixedContainer {
        value: Mixed::StringVal("hello".to_string()),
    };

    let yaml = to_string(&container).unwrap();
    let parsed: MixedContainer = from_str(&yaml).unwrap();
    assert_eq!(parsed, container);
}

#[test]
fn test_nested_box_enum() {
    let container = MixedContainer {
        value: Mixed::Nested {
            inner: Box::new(Mixed::IntVal(42)),
        },
    };

    let yaml = to_string(&container).unwrap();
    let parsed: MixedContainer = from_str(&yaml).unwrap();
    assert_eq!(parsed, container);
}
