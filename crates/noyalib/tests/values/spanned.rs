//! Spanned<T> tests.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{Spanned, from_str, to_string};

#[test]
fn test_spanned_basic() {
    #[derive(Debug, serde::Deserialize)]
    struct Config {
        port: Spanned<u16>,
    }

    let yaml = "port: 8080";
    let config: Config = from_str(yaml).unwrap();
    assert_eq!(*config.port, 8080);
    assert_eq!(config.port.value, 8080);
}

#[test]
fn test_spanned_serialize_transparent() {
    let val = Spanned::new(42i64);
    let yaml = to_string(&val).unwrap();
    assert_eq!(yaml.trim(), "42");
}

#[test]
fn test_spanned_locations_real() {
    let val: Spanned<String> = from_str("hello").unwrap();
    assert_eq!(val.value, "hello");
    // Real locations from parser
    assert_eq!(val.start.line(), 1);
    assert_eq!(val.start.column(), 1);
    assert_eq!(val.start.index(), 0);
    assert!(val.end.index() > 0);
}

#[test]
fn test_spanned_in_struct() {
    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Config {
        name: Spanned<String>,
        port: Spanned<u16>,
    }

    let config = Config {
        name: Spanned::new("myapp".to_string()),
        port: Spanned::new(8080),
    };

    let yaml = to_string(&config).unwrap();
    assert!(yaml.contains("name: myapp"));
    assert!(yaml.contains("port: 8080"));

    let parsed: Config = from_str(&yaml).unwrap();
    assert_eq!(parsed.name.value, "myapp");
    assert_eq!(parsed.port.value, 8080);
}

/// Many `Spanned` values in one large document: each location lookup
/// must not rescan the source from byte 0.
fn many_spanned_doc(items: usize) -> String {
    (0..items).map(|i| format!("- {i}\n")).collect()
}

#[test]
fn spanned_locations_stay_linear_on_large_documents() {
    let yaml = many_spanned_doc(40_000);
    let start = std::time::Instant::now();
    let items: Vec<Spanned<u32>> = from_str(&yaml).unwrap();
    let elapsed = start.elapsed();
    eprintln!("40,000 spanned values in {elapsed:?}");
    assert_eq!(items.len(), 40_000);
    let last = &items[39_999];
    assert_eq!(last.start.line(), 40_000);
    assert_eq!(last.start.column(), 3);
    assert_eq!(
        last.start,
        noyalib::Location::from_index(&yaml, last.start.index())
    );
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "40,000 spanned values took {elapsed:?}"
    );
}

#[test]
fn spanned_locations_on_one_long_line() {
    let yaml = format!(
        "[{}]",
        (0..3_000)
            .map(|i| format!("é{i}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let items: Vec<Spanned<String>> = from_str(&yaml).unwrap();
    for item in [&items[0], &items[1_499], &items[2_999]] {
        assert_eq!(
            item.start,
            noyalib::Location::from_index(&yaml, item.start.index())
        );
        assert_eq!(
            item.end,
            noyalib::Location::from_index(&yaml, item.end.index())
        );
    }
}
