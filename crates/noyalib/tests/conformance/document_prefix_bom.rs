// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! YAML 1.2.2 §9.2 `l-yaml-stream`: every `l-document-prefix` may open
//! with a byte order mark, not only the first one in the stream. A BOM
//! inside a document stays an error (§5.2). Found by
//! fuzz_serde_yaml_compat, which had classified these as policy.

use noyalib::{Value, cst, from_str, load_all_as};

const BOM: &str = "\u{feff}";

#[test]
fn a_bom_opening_a_later_document_prefix_loads() {
    for (source, expected) in [
        (
            format!("a: 1\n...\n{BOM}---\nb: 2"),
            vec!["{\"a\":1}", "{\"b\":2}"],
        ),
        (
            format!("a: 1\n...\n{BOM}b: 2\n"),
            vec!["{\"a\":1}", "{\"b\":2}"],
        ),
        (format!(".\n...\n{BOM}"), vec!["\".\""]),
        (format!("a\n...\n# c\n{BOM}# d\nb"), vec!["\"a\"", "\"b\""]),
        (format!("a\n{BOM}---\nb"), vec!["\"a\"", "\"b\""]),
        (format!("a\n{BOM}...\n"), vec!["\"a\""]),
        (format!("-\r{BOM}---"), vec!["[null]", "null"]),
        (format!("\n{BOM}a: 1\n"), vec!["{\"a\":1}"]),
        (format!("# c\n{BOM}a: 1\n"), vec!["{\"a\":1}"]),
        (format!("{BOM}# c\n{BOM}a: 1\n"), vec!["{\"a\":1}"]),
        // A prefix need not be followed by a document.
        (format!("---\r{BOM}"), vec!["null"]),
        (format!("....\n#\n{BOM}"), vec!["\"....\""]),
        (format!("n:\n{BOM}"), vec!["{\"n\":null}"]),
        (format!("a\n{BOM}# c\n\n---\nb"), vec!["\"a\"", "\"b\""]),
        (format!("a\r{BOM}\r{BOM}---\rb"), vec!["\"a\"", "\"b\""]),
    ] {
        let docs: Vec<serde_json::Value> =
            load_all_as(&source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
        let docs: Vec<String> = docs.iter().map(ToString::to_string).collect();
        assert_eq!(docs, expected, "{source:?}");
        assert!(cst::parse_stream(&source).is_ok(), "CST {source:?}");
        #[cfg(feature = "std")]
        {
            let read: Vec<String> = noyalib::read::<_, serde_json::Value>(source.as_bytes())
                .unwrap_or_else(|e| panic!("read {source:?}: {e}"))
                .map(|doc| {
                    doc.unwrap_or_else(|e| panic!("read {source:?}: {e}"))
                        .to_string()
                })
                .collect();
            assert_eq!(read, expected, "read {source:?}");
        }
        // The parallel loader splits documents at `---` only.
        #[cfg(feature = "parallel")]
        if source.matches("---").count() + 1 >= expected.len() {
            let parallel: Vec<serde_json::Value> = noyalib::parallel::parse(&source)
                .unwrap_or_else(|e| panic!("parallel {source:?}: {e}"));
            let parallel: Vec<String> = parallel.iter().map(ToString::to_string).collect();
            assert_eq!(parallel, expected, "parallel {source:?}");
        }
    }
}

#[test]
fn a_bom_inside_a_document_is_still_refused() {
    for source in [
        format!("-\r{BOM}-"),
        format!("\n {BOM}"),
        format!("{BOM}{BOM}"),
        format!("a\n{BOM}b"),
        // Found by fuzz_serde_yaml_compat: a BOM right after a prefix
        // BOM is content, not a second prefix.
        format!("s\r{BOM}{BOM}"),
        format!("a\n{BOM}# c\nb"),
        format!("a\n{BOM} ---\nb"),
        format!("a\r{BOM}# c\rb"),
        format!("key: {BOM}value"),
        format!("[{BOM},]"),
        format!("[\n{BOM}---]"),
        format!("a:\r {BOM}---"),
        format!("%YAML 1.2\n{BOM}---\na"),
    ] {
        let error = from_str::<Value>(&source).expect_err(&source);
        assert!(
            error.to_string().contains("byte order mark") || source.starts_with('%'),
            "{source:?}: {error}"
        );
    }
}

#[test]
fn a_bom_right_after_a_line_start_bom_is_refused_at_the_first() {
    // Found by fuzz_serde_yaml_compat: the first BOM was taken for a
    // prefix because a BOM followed it, then the second was refused.
    let source = format!("s\r{BOM}{BOM}");
    let error = from_str::<Value>(&source).expect_err(&source);
    assert_eq!(error.location().map(|l| l.index()), Some(2), "{error}");
}
