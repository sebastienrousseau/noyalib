//! Emitter corner cases where the output changed meaning on reload:
//! comment text carrying a line break, a `<<` key, multi-line strings
//! forced into single quotes, and the `...` marker after a keep-chomped
//! block scalar.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::{
    Commented, LitString, Mapping, SerializerConfig, Value, from_str, to_string,
    to_string_with_config,
};

fn s(x: &str) -> Value {
    Value::String(x.to_owned())
}

fn map(pairs: &[(&str, Value)]) -> Value {
    let mut m = Mapping::new();
    for (k, v) in pairs {
        let _ = m.insert(*k, v.clone());
    }
    Value::Mapping(m)
}

#[test]
fn comment_line_breaks_cannot_add_keys() {
    #[derive(serde::Serialize)]
    struct Doc {
        k: Commented<String>,
        z: Commented<Vec<i32>>,
    }
    for brk in ["\n", "\r", "\r\n", "\u{85}", "\u{2028}", "\u{2029}"] {
        let doc = Doc {
            k: Commented::new("user".into(), format!("x{brk}role: admin")),
            z: Commented::new(vec![1], format!("y{brk}other: 1")),
        };
        let out = to_string(&doc).unwrap();
        let back: Value = from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
        assert_eq!(
            back,
            map(&[
                ("k", s("user")),
                ("z", Value::Sequence(vec![Value::from(1)]))
            ]),
            "emitted {out:?}"
        );
    }
}

#[test]
fn comment_on_a_block_scalar_stays_out_of_its_content() {
    #[derive(serde::Serialize)]
    struct Doc {
        k: Commented<LitString>,
        n: Commented<String>,
        z: i32,
    }
    let doc = Doc {
        k: Commented::new(LitString("a\nb".into()), "c"),
        n: Commented::new("p\nq\n".into(), "two\nlines"),
        z: 1,
    };
    let out = to_string(&doc).unwrap();
    let back: Value = from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
    assert_eq!(
        back,
        map(&[("k", s("a\nb")), ("n", s("p\nq\n")), ("z", Value::from(1))]),
        "emitted {out:?}"
    );
}

#[test]
fn merge_key_string_is_not_a_merge_on_reload() {
    let v = map(&[("role", s("user")), ("<<", map(&[("role", s("admin"))]))]);
    for cfg in [
        SerializerConfig::new(),
        SerializerConfig::new().flow_style(noyalib::FlowStyle::Flow),
    ] {
        let out = to_string_with_config(&v, &cfg).unwrap();
        let back: Value = from_str(&out).unwrap();
        assert_eq!(back, v, "emitted {out:?}");
    }
}

#[test]
fn quote_all_multi_line_strings_round_trip() {
    let cfg = SerializerConfig::new().quote_all(true);
    for x in ["line1\nline2", "a\n\nb", "k\n", "\n", "a\tb"] {
        for v in [s(x), map(&[("k", s(x))])] {
            let out = to_string_with_config(&v, &cfg).unwrap();
            let back: Value = from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
            assert_eq!(back, v, "emitted {out:?}");
        }
    }
}

#[test]
fn dots_leading_multi_line_strings_round_trip() {
    for x in ["...\nx", "...\nb: 2", "...\n", "... \n\n"] {
        for v in [s(x), map(&[("k", s(x))])] {
            let out = to_string(&v).unwrap();
            let back: Value = from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
            assert_eq!(back, v, "emitted {out:?}");
        }
    }
}

#[test]
fn document_end_after_keep_chomped_block_round_trips() {
    let cfg = SerializerConfig::new().document_end(true);
    for x in ["a\n\n", "a\n\n\n", "\n", "a\n"] {
        for v in [s(x), map(&[("k", s(x))])] {
            let out = to_string_with_config(&v, &cfg).unwrap();
            let back: Value = from_str(&out).unwrap_or_else(|e| panic!("{out:?}: {e}"));
            assert_eq!(back, v, "emitted {out:?}");
        }
    }
}
