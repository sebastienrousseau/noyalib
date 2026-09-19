// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Regression: `# comment\n` at the very start of a YAML stream must
//! be skipped, not treated as the start of a node. Commit 1e7dace
//! shipped a broken `Scanner::skip_blank` that inverted the meaning
//! of `simd::clean_prefix_len` (the helper returns the prefix that
//! is NOT in the needle set; we wanted the inverse). The bug
//! advanced past `#` thinking it was a blank, so leading-comment
//! YAML failed at line 1 column 2 with "unexpected character".
//!
//! The regression slipped past CI because
//! `tests/yaml_compliance_report.rs::load_suite` silently
//! `continue`s when its wrapper `from_str(&content)` fails — turning
//! a 406/406 strict pass into a vacuous 0/0 "pass". That report
//! file now hard-asserts `total >= 350`; this file pins the
//! specific input shape so the regression cannot recur.

use std::collections::BTreeMap;

#[test]
fn leading_comment_to_typed_btreemap() {
    let yaml = "# leading comment\nfoo: bar\n";
    let m: BTreeMap<String, String> = noyalib::from_str(yaml).expect("must parse");
    assert_eq!(m.get("foo").map(String::as_str), Some("bar"));
}

#[test]
fn leading_comment_to_value() {
    let yaml = "# leading comment\nfoo: bar\n";
    let v: noyalib::Value = noyalib::from_str(yaml).expect("must parse");
    let m = v.as_mapping().expect("mapping");
    assert_eq!(m.get("foo").and_then(|v| v.as_str()), Some("bar"));
}

#[test]
fn multiple_leading_comments_then_document_marker() {
    let yaml = "\
# Sets are represented as a
# Mapping where each key is
# associated with a null value
--- !!set
? Mark McGwire
? Sammy Sosa
";
    // We don't pin the exact `Value` shape (`!!set` may surface as
    // `Value::Tagged(_)` or `Value::Mapping(_)` depending on the
    // tag-passthrough policy). The regression to pin is that parsing
    // *succeeds* — the bug in 1e7dace panicked at line 1 column 2
    // before reaching any tag handling.
    let docs: Vec<noyalib::Value> =
        noyalib::load_all_as(yaml).expect("must parse multi-line comment + tagged set");
    assert_eq!(docs.len(), 1);
}
