// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! A byte order mark (U+FEFF) inside a document.
//!
//! YAML 1.2.2 production [27] leaves the BOM out of `nb-char`, so the
//! default profile refuses it in plain and block scalars, comments,
//! anchor names and tags. Quoted scalars are built on `nb-json`
//! ([107], [117]), which keeps it. The `serde_yaml` profile reads it as
//! text where libyaml does; anchor names and tags refuse it in both.

use noyalib::{ParserConfig, Value};

/// Every entry point that parses a whole document under `cfg`, named,
/// so a failure says which one disagreed.
type EntryPoint = (&'static str, fn(&str, &ParserConfig) -> Result<(), String>);

fn entry_points() -> Vec<EntryPoint> {
    fn value(s: &str, c: &ParserConfig) -> Result<(), String> {
        noyalib::from_str_with_config::<Value>(s, c)
            .map(drop)
            .map_err(|e| e.to_string())
    }
    fn typed(s: &str, c: &ParserConfig) -> Result<(), String> {
        noyalib::from_str_with_config::<serde_json::Value>(s, c)
            .map(drop)
            .map_err(|e| e.to_string())
    }
    fn streaming(s: &str, c: &ParserConfig) -> Result<(), String> {
        use serde::Deserialize as _;
        let mut de = noyalib::StreamingDeserializer::with_config(s, c);
        let _ = serde::de::IgnoredAny::deserialize(&mut de).map_err(|e| e.to_string())?;
        de.end().map_err(|e| e.to_string())
    }
    fn borrowed(s: &str, c: &ParserConfig) -> Result<(), String> {
        noyalib::borrowed::from_str_borrowed_with_config(s, c)
            .map(drop)
            .map_err(|e| e.to_string())
    }
    fn load_all(s: &str, c: &ParserConfig) -> Result<(), String> {
        noyalib::load_all_with_config(s, c)
            .map(drop)
            .map_err(|e| e.to_string())
    }
    fn cst(s: &str, c: &ParserConfig) -> Result<(), String> {
        noyalib::cst::parse_document_with_config(s, c)
            .map(drop)
            .map_err(|e| e.to_string())
    }
    fn cst_stream(s: &str, c: &ParserConfig) -> Result<(), String> {
        noyalib::cst::parse_stream_with_config(s, c)
            .map(drop)
            .map_err(|e| e.to_string())
    }
    vec![
        ("from_str::<Value>", value),
        ("from_str::<serde_json::Value>", typed),
        ("StreamingDeserializer", streaming),
        ("borrowed", borrowed),
        ("load_all", load_all),
        ("cst::parse_document", cst),
        ("cst::parse_stream", cst_stream),
    ]
}

/// Sites built on `nb-char`, where YAML 1.2.2 has no room for a BOM.
const NB_CHAR_SITES: &[(&str, &str)] = &[
    ("plain, inside", "a: x\u{feff}y\n"),
    ("plain, first", "a: \u{feff}y\n"),
    ("plain, last", "a: y\u{feff}\n"),
    ("plain key", "k\u{feff}: 1\n"),
    ("plain key after an indent", "a:\n  \u{feff}b: 1\n"),
    ("flow plain", "[x\u{feff}y]\n"),
    ("flow plain, first", "[\u{feff}y]\n"),
    ("sequence entry, first", "- \u{feff}y\n"),
    ("plain continuation line", "a: x\n  \u{feff}y\n"),
    ("literal", "a: |\n  x\u{feff}y\n"),
    ("literal, first", "a: |\n  \u{feff}y\n"),
    ("folded", "a: >\n  x\u{feff}y\n"),
    ("trailing comment", "a: 1 # c\u{feff}\n"),
    ("comment line", "# c\u{feff}\na: 1\n"),
    ("block scalar header comment", "a: | # c\u{feff}\n  x\n"),
];

/// Sites libyaml refuses as well, so both profiles refuse them.
const NAME_SITES: &[(&str, &str)] = &[
    ("anchor", "a: &x\u{feff}y 1\n"),
    ("alias", "a: &x 1\nb: *x\u{feff}y\n"),
    ("tag", "a: !x\u{feff}y 1\n"),
    ("column 0, not a document prefix", "a: 1\n\u{feff}b: 2\n"),
];

/// Sites where a BOM is legal in YAML 1.2.2, with the value they load.
const LEGAL_SITES: &[(&str, &str, &str)] = &[
    ("single-quoted", "a: 'x\u{feff}y'\n", "x\u{feff}y"),
    ("single-quoted, first", "a: '\u{feff}y'\n", "\u{feff}y"),
    ("double-quoted", "a: \"x\u{feff}y\"\n", "x\u{feff}y"),
    ("double-quoted escape", "a: \"x\\uFEFFy\"\n", "x\u{feff}y"),
    ("stream start", "\u{feff}a: xy\n", "xy"),
];

#[test]
fn the_default_profile_refuses_a_bom_in_nb_char_text() {
    let cfg = ParserConfig::new();
    for (site, yaml) in NB_CHAR_SITES.iter().chain(NAME_SITES) {
        for (entry, parse) in entry_points() {
            let err = parse(yaml, &cfg).expect_err(&format!("{entry} accepted {site}: {yaml:?}"));
            assert!(
                err.contains("byte order mark"),
                "{entry} refused {site} ({yaml:?}) for another reason: {err}"
            );
        }
    }
}

#[test]
fn the_default_profile_keeps_a_bom_in_quoted_text() {
    let cfg = ParserConfig::new();
    for (site, yaml, want) in LEGAL_SITES {
        for (entry, parse) in entry_points() {
            parse(yaml, &cfg).unwrap_or_else(|e| panic!("{entry} refused {site}: {e}"));
        }
        let got: Value = noyalib::from_str(yaml).unwrap();
        assert_eq!(got["a"].as_str(), Some(*want), "{site}");
    }
}

#[test]
fn the_serde_yaml_profile_reads_a_bom_as_text_like_libyaml() {
    let cfg = ParserConfig::serde_yaml_compat();
    // Values are serde_yaml 0.9.34's own readings of the same input.
    let want: &[(&str, serde_json::Value)] = &[
        ("a: x\u{feff}y\n", serde_json::json!({"a": "x\u{feff}y"})),
        ("a: \u{feff}y\n", serde_json::json!({"a": "\u{feff}y"})),
        ("a: y\u{feff}\n", serde_json::json!({"a": "y\u{feff}"})),
        ("k\u{feff}: 1\n", serde_json::json!({"k\u{feff}": 1})),
        (
            "a:\n  \u{feff}b: 1\n",
            serde_json::json!({"a": {"\u{feff}b": 1}}),
        ),
        ("[x\u{feff}y]\n", serde_json::json!(["x\u{feff}y"])),
        ("[\u{feff}y]\n", serde_json::json!(["\u{feff}y"])),
        ("- \u{feff}y\n", serde_json::json!(["\u{feff}y"])),
        (
            "a: x\n  \u{feff}y\n",
            serde_json::json!({"a": "x \u{feff}y"}),
        ),
        (
            "a: |\n  x\u{feff}y\n",
            serde_json::json!({"a": "x\u{feff}y\n"}),
        ),
        (
            "a: |\n  \u{feff}y\n",
            serde_json::json!({"a": "\u{feff}y\n"}),
        ),
        (
            "a: >\n  x\u{feff}y\n",
            serde_json::json!({"a": "x\u{feff}y\n"}),
        ),
        ("a: 1 # c\u{feff}\n", serde_json::json!({"a": 1})),
        ("# c\u{feff}\na: 1\n", serde_json::json!({"a": 1})),
        ("a: | # c\u{feff}\n  x\n", serde_json::json!({"a": "x\n"})),
    ];
    assert_eq!(want.len(), NB_CHAR_SITES.len());
    // A direct `StreamingDeserializer` serves only stream-eligible
    // configurations and hands this profile back to the loader.
    let entries: Vec<_> = entry_points()
        .into_iter()
        .filter(|(name, _)| *name != "StreamingDeserializer")
        .collect();
    for (yaml, value) in want {
        for (entry, parse) in &entries {
            parse(yaml, &cfg).unwrap_or_else(|e| panic!("{entry} refused {yaml:?}: {e}"));
        }
        let got: serde_json::Value = noyalib::from_str_with_config(yaml, &cfg).unwrap();
        assert_eq!(&got, value, "{yaml:?}");
    }
    for (site, yaml) in NAME_SITES {
        for (entry, parse) in &entries {
            assert!(
                parse(yaml, &cfg).is_err(),
                "{entry} accepted {site}: {yaml:?}"
            );
        }
    }
}

/// A string holding a BOM must come back from the serializer in a form
/// the default profile reads, so it is quoted.
#[test]
fn a_string_holding_a_bom_round_trips_through_the_default_profile() {
    for text in ["x\u{feff}y", "\u{feff}y", "y\u{feff}", "a\nb\u{feff}c\n"] {
        let doc = Value::from(
            std::iter::once((text.to_owned(), Value::from(text))).collect::<noyalib::Mapping>(),
        );
        let yaml = noyalib::to_string(&doc).unwrap();
        let back: Value = noyalib::from_str(&yaml)
            .unwrap_or_else(|e| panic!("{text:?} was written as {yaml:?}, which reads as {e}"));
        assert_eq!(back, doc, "{yaml:?}");
    }
}

/// The refusal points at the byte order mark itself, not at wherever
/// the scanner stopped reading the scalar or comment that holds it.
#[test]
fn a_refused_bom_is_located_where_it_stands() {
    for (yaml, line, column) in [
        ("a: 1\nb: x\u{feff}y\n", 2, 5),
        ("a: 1\nb: x\u{feff}y\n  z\nc: 2\n", 2, 5),
        ("a: |\n  x\n  y\u{feff}z\n", 3, 4),
        ("a: 1 # c\u{feff}\n", 1, 9),
    ] {
        let err = noyalib::from_str::<Value>(yaml).unwrap_err();
        let at = err.location().unwrap();
        assert_eq!((at.line(), at.column()), (line, column), "{yaml:?}: {err}");
    }
}
