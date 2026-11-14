// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The archive's reading of a flow entry that starts with `?` or `:`.

/// Look `k` up in `m`, tolerating a known libyaml quirk shared by
/// serde_yaml and serde_yaml_ng: it reads `?foo` as an explicit-key
/// indicator and drops the `?`, where YAML 1.2 (ns-plain-first;
/// yaml-test-suite 652Z) reads the plain key "?foo". The regression is
/// pinned in tests/regressions/competitor_bugs.rs.
pub(crate) fn lookup<'m>(
    m: &'m serde_json::Map<String, serde_json::Value>,
    k: &str,
) -> Option<&'m serde_json::Value> {
    m.get(k)
        .or_else(|| k.strip_prefix('?').and_then(|s| m.get(s)))
        .or_else(|| m.get(&format!("?{k}")))
}

/// Known serde_yaml_ng quirks at the start of a flow entry: it reads
/// a `?` followed by a non-space as the explicit-key indicator, so
/// `[?x]` becomes `[{"x": null}]` where YAML 1.2 (ns-plain-first;
/// yaml-test-suite 652Z) reads the plain scalar "?x"; and it cannot
/// read an entry whose key is empty (`[: x]`, `[?, :]`), which the
/// spec allows (yaml-test-suite CFD4). A flow mapping key with a `?`
/// is tolerated by [`lookup`]; any other such entry changes the
/// shape, so those inputs are dropped.
pub(crate) fn flow_entry_starts_with_indicator(s: &str) -> bool {
    // Look back past comments as well as white space.
    let stripped = without_comments(s);
    let b = stripped.as_bytes();
    let flow = s.contains(['[', '{']);
    b.iter().enumerate().any(|(i, &c)| {
        let question = c == b'?' && b.get(i + 1).is_some_and(|n| !n.is_ascii_whitespace());
        (question || c == b':') && entry_start(b, i)
            || flow && c == b':' && after_key_indicator_or_property(b, i)
    })
}

/// The last non-blank byte before `i` opens or separates a flow entry.
fn entry_start(b: &[u8], i: usize) -> bool {
    b[..i]
        .iter()
        .rev()
        .find(|p| !p.is_ascii_whitespace())
        .is_some_and(|p| matches!(p, b'[' | b'{' | b','))
}

/// `:` follows a standalone explicit-key `?` or a node property (`&a`,
/// `!t`) that itself starts an entry: the key node is empty and the `:`
/// starts a plain scalar (`{? :e}`, `{&a :e}`), which libyaml reads as
/// the value indicator.
fn after_key_indicator_or_property(b: &[u8], i: usize) -> bool {
    let Some(end) = b[..i].iter().rposition(|p| !p.is_ascii_whitespace()) else {
        return false;
    };
    let start = b[..end]
        .iter()
        .rposition(|p| p.is_ascii_whitespace() || matches!(p, b'[' | b'{' | b','))
        .map_or(0, |at| at + 1);
    let token = &b[start..=end];
    let before = |at: usize| {
        entry_start(b, at) || b[..at].iter().rev().find(|p| !p.is_ascii_whitespace()) == Some(&b'?')
    };
    match token.first() {
        Some(b'?') if token.len() == 1 => entry_start(b, start) || start == 0,
        Some(b'&' | b'!') => before(start),
        _ => false,
    }
}

/// `s` with every comment (`#` at a line start or after white space,
/// to the end of the line) blanked to spaces, offsets unchanged.
fn without_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_comment = false;
    let mut prev_blank = true;
    for c in s.chars() {
        if c == '\n' || c == '\r' {
            in_comment = false;
        } else if c == '#' && prev_blank {
            in_comment = true;
        }
        prev_blank = c.is_whitespace();
        out.push(if in_comment { ' ' } else { c });
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn plain_question_key_matches_the_archive_reading() {
        for source in ["{?~}", "{ ?foo: bar,\nbar: 42\n}\n"] {
            let ours: serde_json::Value = noyalib::compat::serde_yaml::from_str(source).unwrap();
            let theirs: serde_json::Value = serde_yaml::from_str(source).unwrap();
            assert_ne!(ours, theirs, "{source:?}");
            let (ours, theirs) = (ours.as_object().unwrap(), theirs.as_object().unwrap());
            assert_eq!(ours.len(), theirs.len());
            for (key, value) in ours {
                assert_eq!(super::lookup(theirs, key), Some(value), "{source:?}");
            }
        }
        assert!(super::flow_entry_starts_with_indicator("{?&d}"));
        assert!(super::flow_entry_starts_with_indicator("[a, # c\n:x]"));
        assert!(!super::flow_entry_starts_with_indicator("{a: ?b}"));
        for source in [
            "{? :e}",
            "{?\n:t}",
            "{&d :d}",
            "{! :e}",
            "{? &d\n:0}",
            "$: {&m :.\r,}",
        ] {
            assert!(
                super::flow_entry_starts_with_indicator(source),
                "{source:?}"
            );
        }
        for source in ["{a: b}", "{? a: b}", "{&d a: b}", "{x &d: :e}"] {
            assert!(
                !super::flow_entry_starts_with_indicator(source),
                "{source:?}"
            );
        }
        let map = serde_json::json!({"a": 1}).as_object().unwrap().clone();
        assert_eq!(super::lookup(&map, "?b"), None);
    }
}
