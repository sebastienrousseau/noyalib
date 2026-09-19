// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A digit-leading string is written plain when the parser would read it
//! back as a string, and quoted only when a reader could take it for a
//! number: the serializer asks the resolver instead of quoting everything
//! that starts with a digit.

use noyalib::{Mapping, Value, from_str, to_string};

// A colon anywhere in the text quotes on its own (`NEEDS_QUOTE_BYTE`), so
// `2026-12-31T10:00:00Z` is not in this list; that rule is separate from
// the digit check.
const PLAIN: &[&str] = &["2026-12-31", "2026-12", "1.2.3", "3rd", "8080abc", "1/2"];

// `007` and `0755` read as integers (YAML 1.2 decimal, YAML 1.1 octal) and
// `12:30:00` as a YAML 1.1 sexagesimal (its colons would quote it anyway),
// so they stay quoted for a reader with the legacy forms enabled. `++1`
// stays quoted because a permissive reader collapses the stacked signs.
const QUOTED: &[&str] = &[
    "42", "1.5", "1e3", "0x1F", "0o17", "007", "0755", "12:30:00", "+3", ".5", "++1",
];

fn emit(s: &str) -> String {
    let mut mapping = Mapping::new();
    let _ = mapping.insert("k", Value::String(s.to_string()));
    to_string(&Value::Mapping(mapping)).unwrap()
}

#[test]
fn digit_leading_strings_the_parser_keeps_are_written_plain() {
    for s in PLAIN {
        assert_eq!(
            emit(s),
            format!("k: {s}"),
            "{s} reads back as a string, so it needs no quotes"
        );
    }
}

#[test]
fn strings_a_reader_could_take_for_numbers_are_quoted() {
    for s in QUOTED {
        assert_eq!(
            emit(s),
            format!("k: \"{s}\""),
            "{s} would read back as a number"
        );
    }
}

#[test]
fn every_case_reads_back_as_the_same_string() {
    for s in PLAIN.iter().chain(QUOTED) {
        let doc = emit(s);
        let value: Value = from_str(&doc).unwrap();
        assert_eq!(
            value.get("k"),
            Some(&Value::String((*s).to_string())),
            "round trip of {doc:?}"
        );
    }
}
