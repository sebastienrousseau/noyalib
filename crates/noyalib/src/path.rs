//! Path tracking for YAML structure locations.
//!
//! This module provides the [`Path`] type for tracking the location within a
//! YAML document structure. It's primarily used for providing meaningful error
//! messages that indicate exactly where in the YAML structure an error
//! occurred.
//!
//! # Examples
//!
//! ```rust
//! use noyalib::Path;
//!
//! // Represent the path: root -> "dependencies" -> "serde" -> "version"
//! let root = Path::Root;
//! let deps = Path::Map {
//!     parent: &root,
//!     key: "dependencies",
//! };
//! let serde = Path::Map {
//!     parent: &deps,
//!     key: "serde",
//! };
//! let version = Path::Map {
//!     parent: &serde,
//!     key: "version",
//! };
//!
//! assert_eq!(version.to_string(), "dependencies.serde.version");
//! ```
//!
//! # Query paths
//!
//! The path strings read by [`Value::get_path`](crate::Value::get_path),
//! [`Value::query`](crate::Value::query), the borrowed reads, and every
//! path-taking `cst::Document` method share one grammar: `.` separates
//! mapping keys, `[n]` indexes a sequence, `*` and `[*]` match every
//! child, and `..` descends recursively. A key that itself contains one
//! of those characters is written as a bracket-quoted segment, `["a.b"]`
//! or `['a[0]']`, inside which `\` escapes the next character.
//! [`quote_key`] spells one such segment, [`push_key`] appends a key to
//! a path in whichever form reads back as that key, and [`join_keys`]
//! builds a whole path from literal keys:
//!
//! ```rust
//! use noyalib::path::{join_keys, quote_key};
//!
//! assert_eq!(quote_key("app.kubernetes.io/name"), r#"["app.kubernetes.io/name"]"#);
//! assert_eq!(
//!     join_keys(["labels", "app.kubernetes.io/name"]),
//!     r#"labels["app.kubernetes.io/name"]"#,
//! );
//! assert_eq!(join_keys(["server", "port"]), "server.port");
//! ```

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::fmt::Display;

/// Represents a path to a location within a YAML document structure.
///
/// `Path` is a recursive data structure that tracks the navigation path from
/// the root of a YAML document to a specific value. It's used to provide
/// context in error messages, helping users understand exactly where in their
/// YAML an issue occurred.
///
/// # Variants
///
/// - [`Root`](Path::Root): The root of the document
/// - [`Seq`](Path::Seq): An index into a sequence (array)
/// - [`Map`](Path::Map): A key in a mapping (object)
/// - [`Alias`](Path::Alias): Following an alias reference
/// - [`Unknown`](Path::Unknown): Unknown or unspecified location
///
/// # Examples
///
/// ```rust
/// use noyalib::Path;
///
/// // Building a path: servers[0].host
/// let root = Path::Root;
/// let servers = Path::Map {
///     parent: &root,
///     key: "servers",
/// };
/// let first = Path::Seq {
///     parent: &servers,
///     index: 0,
/// };
/// let host = Path::Map {
///     parent: &first,
///     key: "host",
/// };
///
/// assert_eq!(host.to_string(), "servers[0].host");
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Path<'a> {
    /// The root of the document.
    ///
    /// This is the starting point for all paths. When displayed, it shows as
    /// `.`.
    #[default]
    Root,

    /// An index into a sequence (array).
    ///
    /// # Fields
    ///
    /// - `parent`: The path to the sequence containing this element
    /// - `index`: The zero-based index into the sequence
    Seq {
        /// The parent path (the sequence itself).
        parent: &'a Self,
        /// The index within the sequence (zero-based).
        index: usize,
    },

    /// A key in a mapping (object).
    ///
    /// # Fields
    ///
    /// - `parent`: The path to the mapping containing this key
    /// - `key`: The key name
    Map {
        /// The parent path (the mapping itself).
        parent: &'a Self,
        /// The key within the mapping.
        key: &'a str,
    },

    /// Following an alias reference.
    ///
    /// This variant is used when traversing through a YAML alias (`*anchor`)
    /// to indicate the path continues through an alias.
    Alias {
        /// The parent path before the alias.
        parent: &'a Self,
    },

    /// Unknown or unspecified location.
    ///
    /// This variant is used when the exact location cannot be determined
    /// but we still want to track that we're somewhere nested.
    Unknown {
        /// The parent path.
        parent: &'a Self,
    },
}

impl<'a> Path<'a> {
    /// Creates a new sequence index path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::Path;
    ///
    /// let root = Path::Root;
    /// let items = Path::Map {
    ///     parent: &root,
    ///     key: "items",
    /// };
    /// let first = items.index(0);
    ///
    /// assert_eq!(first.to_string(), "items[0]");
    /// ```
    #[must_use]
    pub fn index(&'a self, index: usize) -> Self {
        Path::Seq {
            parent: self,
            index,
        }
    }

    /// Creates a new map key path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::Path;
    ///
    /// let root = Path::Root;
    /// let config = root.key("config");
    /// let host = config.key("host");
    ///
    /// assert_eq!(host.to_string(), "config.host");
    /// ```
    #[must_use]
    pub fn key(&'a self, key: &'a str) -> Self {
        Path::Map { parent: self, key }
    }

    /// Creates a new alias path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::Path;
    ///
    /// let root = Path::Root;
    /// let alias_path = root.alias();
    /// ```
    #[must_use]
    pub fn alias(&'a self) -> Self {
        Path::Alias { parent: self }
    }

    /// Creates a new unknown path.
    #[must_use]
    pub fn unknown(&'a self) -> Self {
        Path::Unknown { parent: self }
    }

    /// Returns `true` if this is the root path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::Path;
    ///
    /// let root = Path::Root;
    /// assert!(root.is_root());
    ///
    /// let child = root.key("test");
    /// assert!(!child.is_root());
    /// ```
    #[must_use]
    pub fn is_root(&self) -> bool {
        matches!(self, Path::Root)
    }

    /// Returns the parent path, if any.
    ///
    /// Returns `None` if this is the root path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::Path;
    ///
    /// let root = Path::Root;
    /// let child = root.key("test");
    ///
    /// assert!(root.parent().is_none());
    /// assert!(child.parent().is_some());
    /// ```
    #[must_use]
    pub fn parent(&self) -> Option<&Self> {
        match self {
            Path::Root => None,
            Path::Seq { parent, .. }
            | Path::Map { parent, .. }
            | Path::Alias { parent }
            | Path::Unknown { parent } => Some(parent),
        }
    }

    /// Returns the depth of this path (number of segments from root).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noyalib::Path;
    ///
    /// let root = Path::Root;
    /// assert_eq!(root.depth(), 0);
    ///
    /// let child = root.key("a").key("b");
    /// // Note: each key() creates a new path on the stack
    /// ```
    #[must_use]
    pub fn depth(&self) -> usize {
        match self {
            Path::Root => 0,
            Path::Seq { parent, .. }
            | Path::Map { parent, .. }
            | Path::Alias { parent }
            | Path::Unknown { parent } => 1 + parent.depth(),
        }
    }
}

impl Display for Path<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        /// Helper struct to format the parent path with a trailing dot if
        /// needed.
        struct Parent<'a>(&'a Path<'a>);

        impl Display for Parent<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    Path::Root => Ok(()),
                    path => write!(f, "{path}."),
                }
            }
        }

        /// Helper struct to format the parent path without trailing dot for
        /// sequences.
        struct ParentNoDot<'a>(&'a Path<'a>);

        impl Display for ParentNoDot<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    Path::Root => Ok(()),
                    path => write!(f, "{path}"),
                }
            }
        }

        match self {
            Path::Root => f.write_str("."),
            Path::Seq { parent, index } => {
                write!(f, "{}[{}]", ParentNoDot(parent), index)
            }
            Path::Map { parent, key } => {
                write!(f, "{}{}", Parent(parent), key)
            }
            Path::Alias { parent } => {
                write!(f, "{}", Parent(parent))
            }
            Path::Unknown { parent } => {
                write!(f, "{}?", Parent(parent))
            }
        }
    }
}

// ── Query path parsing ──────────────────────────────────────────────────
// Shared path parsing for value.rs and borrowed.rs query methods.

/// A segment in a query path expression.
#[derive(Debug, Clone)]
pub(crate) enum QuerySegment {
    /// A key in a mapping.
    Key(String),
    /// An index in a sequence.
    Index(usize),
    /// Wildcard: matches all keys or all indices.
    Wildcard,
    /// Recursive descent: matches at any depth.
    RecursiveDescent,
}

/// Parse a query path string into segments.
///
/// Supports:
/// - Dot notation: `"foo.bar.baz"`
/// - Bracket notation: `"items[0]"`
/// - Quoted keys: `"labels[\"app.kubernetes.io/name\"]"` — the text
///   between the quotes is one mapping key, whatever it contains, so a
///   key holding `.`, `[`, `]`, or `*` is addressable. Either quote
///   style works (`['a[0]']`, `["*"]`); inside both, `\` escapes the
///   next character, so `["say \"hi\""]` is the key `say "hi"`.
///   [`quote_key`] spells such a segment.
/// - Mixed: `"items[0].name"`, `"items[0][\"a.b\"].c"`
/// - Wildcard: `"items[*]"` or `"items.*"`
/// - Recursive descent: `"..name"` (find `name` at any depth)
///
/// The grammar is lenient: a bracket segment that is neither an index,
/// a wildcard, nor a quoted key is dropped, and a quoted key that is
/// never closed runs to the end of the path.
pub(crate) fn parse_query_path(path: &str) -> Vec<QuerySegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '.' => {
                if !current.is_empty() {
                    segments.push(QuerySegment::Key(core::mem::take(&mut current)));
                }
                if chars.peek() == Some(&'.') {
                    let _ = chars.next();
                    segments.push(QuerySegment::RecursiveDescent);
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(QuerySegment::Key(core::mem::take(&mut current)));
                }
                if let Some(&quote @ ('"' | '\'')) = chars.peek() {
                    let _ = chars.next();
                    segments.push(QuerySegment::Key(read_quoted_key(&mut chars, quote)));
                    if chars.peek() == Some(&']') {
                        let _ = chars.next();
                    }
                    continue;
                }
                let mut index_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ']' {
                        let _ = chars.next();
                        break;
                    }
                    index_str.push(c);
                    let _ = chars.next();
                }
                if index_str == "*" {
                    segments.push(QuerySegment::Wildcard);
                } else if let Ok(idx) = index_str.parse::<usize>() {
                    segments.push(QuerySegment::Index(idx));
                }
            }
            ']' => {}
            '*' => {
                if !current.is_empty() {
                    segments.push(QuerySegment::Key(core::mem::take(&mut current)));
                }
                segments.push(QuerySegment::Wildcard);
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        segments.push(QuerySegment::Key(current));
    }

    segments
}

/// Read the body of a quoted key segment up to its closing `quote`,
/// consuming the quote. `\` escapes the next character. An unterminated
/// key runs to the end of the input.
fn read_quoted_key(chars: &mut core::iter::Peekable<core::str::Chars<'_>>, quote: char) -> String {
    let mut key = String::new();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    key.push(escaped);
                }
            }
            c if c == quote => break,
            c => key.push(c),
        }
    }
    key
}

/// Whether the query grammar reads `key` back as itself when it is
/// written plain: it is not empty and holds none of `.`, `[`, `]`, `*`.
fn is_plain_key(key: &str) -> bool {
    !key.is_empty() && !key.contains(['.', '[', ']', '*'])
}

/// Spell `key` as one bracket-quoted path segment, `["key"]`, that
/// [`Value::get_path`](crate::Value::get_path) and every path-taking
/// `cst::Document` method read back as exactly that mapping key. `"`
/// and `\` inside the key are escaped.
///
/// # Examples
///
/// ```rust
/// use noyalib::path::quote_key;
///
/// assert_eq!(quote_key("app.kubernetes.io/name"), r#"["app.kubernetes.io/name"]"#);
/// assert_eq!(quote_key("*"), r#"["*"]"#);
/// assert_eq!(quote_key(r#"say "hi""#), r#"["say \"hi\""]"#);
/// ```
#[must_use]
pub fn quote_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len() + 4);
    out.push_str("[\"");
    for c in key.chars() {
        if matches!(c, '"' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push_str("\"]");
    out
}

/// Append the mapping key `key` to `path`: in dot notation when the
/// grammar reads the plain spelling back as that key, otherwise as the
/// segment [`quote_key`] spells. A quoted segment needs no separator.
///
/// # Examples
///
/// ```rust
/// use noyalib::path::push_key;
///
/// let mut path = String::from("items[0]");
/// push_key(&mut path, "labels");
/// push_key(&mut path, "app.kubernetes.io/name");
/// assert_eq!(path, r#"items[0].labels["app.kubernetes.io/name"]"#);
/// ```
pub fn push_key(path: &mut String, key: &str) {
    if is_plain_key(key) {
        if !path.is_empty() {
            path.push('.');
        }
        path.push_str(key);
    } else {
        path.push_str(&quote_key(key));
    }
}

/// Build a path from literal mapping keys, one [`push_key`] per key, so
/// every key reads back as itself whatever it contains.
///
/// # Examples
///
/// ```rust
/// use noyalib::path::join_keys;
///
/// assert_eq!(join_keys(["server", "port"]), "server.port");
/// assert_eq!(join_keys(["a", "b.c", "*"]), r#"a["b.c"]["*"]"#);
/// assert_eq!(join_keys(Vec::<&str>::new()), "");
/// ```
#[must_use]
pub fn join_keys<K: AsRef<str>>(keys: impl IntoIterator<Item = K>) -> String {
    let mut path = String::new();
    for key in keys {
        push_key(&mut path, key.as_ref());
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_root_display() {
        let root = Path::Root;
        assert_eq!(root.to_string(), ".");
    }

    #[test]
    fn test_path_map_display() {
        let root = Path::Root;
        let key1 = Path::Map {
            parent: &root,
            key: "config",
        };
        assert_eq!(key1.to_string(), "config");

        let key2 = Path::Map {
            parent: &key1,
            key: "server",
        };
        assert_eq!(key2.to_string(), "config.server");

        let key3 = Path::Map {
            parent: &key2,
            key: "host",
        };
        assert_eq!(key3.to_string(), "config.server.host");
    }

    #[test]
    fn test_path_seq_display() {
        let root = Path::Root;
        let items = Path::Map {
            parent: &root,
            key: "items",
        };
        let first = Path::Seq {
            parent: &items,
            index: 0,
        };
        assert_eq!(first.to_string(), "items[0]");

        let second = Path::Seq {
            parent: &items,
            index: 1,
        };
        assert_eq!(second.to_string(), "items[1]");
    }

    #[test]
    fn test_path_mixed_display() {
        let root = Path::Root;
        let servers = Path::Map {
            parent: &root,
            key: "servers",
        };
        let first = Path::Seq {
            parent: &servers,
            index: 0,
        };
        let host = Path::Map {
            parent: &first,
            key: "host",
        };
        assert_eq!(host.to_string(), "servers[0].host");
    }

    #[test]
    fn test_path_unknown_display() {
        let root = Path::Root;
        let unknown = Path::Unknown { parent: &root };
        assert_eq!(unknown.to_string(), "?"); // Root doesn't add a dot

        let key = Path::Map {
            parent: &root,
            key: "test",
        };
        let unknown2 = Path::Unknown { parent: &key };
        assert_eq!(unknown2.to_string(), "test.?");
    }

    #[test]
    fn test_path_builder_methods() {
        let root = Path::Root;
        let config = root.key("config");
        let items = config.key("items");
        let first = items.index(0);
        let name = first.key("name");

        assert_eq!(name.to_string(), "config.items[0].name");
    }

    #[test]
    fn test_path_is_root() {
        let root = Path::Root;
        assert!(root.is_root());

        let child = root.key("test");
        assert!(!child.is_root());
    }

    #[test]
    fn test_path_parent() {
        let root = Path::Root;
        assert!(root.parent().is_none());

        let child = Path::Map {
            parent: &root,
            key: "test",
        };
        assert_eq!(child.parent(), Some(&root));
    }

    #[test]
    fn test_path_depth() {
        let root = Path::Root;
        assert_eq!(root.depth(), 0);

        let d1 = Path::Map {
            parent: &root,
            key: "a",
        };
        assert_eq!(d1.depth(), 1);

        let d2 = Path::Map {
            parent: &d1,
            key: "b",
        };
        assert_eq!(d2.depth(), 2);

        let d3 = Path::Seq {
            parent: &d2,
            index: 0,
        };
        assert_eq!(d3.depth(), 3);
    }

    #[test]
    fn test_path_default() {
        let path: Path = Path::default();
        assert!(path.is_root());
    }

    #[test]
    fn test_path_equality() {
        let root1 = Path::Root;
        let root2 = Path::Root;
        assert_eq!(root1, root2);

        let key1 = Path::Map {
            parent: &root1,
            key: "test",
        };
        let key2 = Path::Map {
            parent: &root2,
            key: "test",
        };
        assert_eq!(key1, key2);

        let key3 = Path::Map {
            parent: &root1,
            key: "other",
        };
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_path_alias() {
        let root = Path::Root;
        let alias = root.alias();
        assert!(matches!(alias, Path::Alias { .. }));
    }

    #[test]
    fn test_path_complex_real_world() {
        // Simulate a real-world path like: dependencies.serde.features[0]
        let root = Path::Root;
        let deps = root.key("dependencies");
        let serde = deps.key("serde");
        let features = serde.key("features");
        let first_feature = features.index(0);

        assert_eq!(first_feature.to_string(), "dependencies.serde.features[0]");
    }

    #[test]
    fn test_path_deeply_nested() {
        let root = Path::Root;
        let a = root.key("a");
        let b = a.key("b");
        let c = b.key("c");
        let d = c.key("d");
        let e = d.key("e");

        assert_eq!(e.to_string(), "a.b.c.d.e");
        assert_eq!(e.depth(), 5);
    }

    #[test]
    fn parse_query_path_handles_inline_star_after_key() {
        // `field*` — the `*` glyph mid-path with a non-empty
        // accumulated key. The parser must flush the key first,
        // then push a Wildcard segment.
        let segments = parse_query_path("field*");
        assert_eq!(segments.len(), 2);
        assert!(matches!(&segments[0], QuerySegment::Key(s) if s == "field"));
        assert!(matches!(&segments[1], QuerySegment::Wildcard));
    }

    #[test]
    fn parse_query_path_drops_unparseable_bracket_content() {
        // `items[abc]` — bracket content is neither `*` nor a
        // numeric index. The current contract silently drops the
        // bracket; callers get back the leading key only.
        let segments = parse_query_path("items[abc]");
        assert_eq!(segments.len(), 1);
        assert!(matches!(&segments[0], QuerySegment::Key(s) if s == "items"));
    }

    #[test]
    fn parse_query_path_handles_standalone_star_segment() {
        // `*` on its own — recursive descent's "match anything at
        // this level" form.
        let segments = parse_query_path("*");
        assert_eq!(segments.len(), 1);
        assert!(matches!(&segments[0], QuerySegment::Wildcard));
    }

    #[test]
    fn parse_query_path_handles_recursive_descent() {
        // `..name` — descend recursively, then look up `name`.
        let segments = parse_query_path("..name");
        assert_eq!(segments.len(), 2);
        assert!(matches!(&segments[0], QuerySegment::RecursiveDescent));
        assert!(matches!(&segments[1], QuerySegment::Key(s) if s == "name"));
    }

    fn keys(segments: &[QuerySegment]) -> Vec<&str> {
        segments
            .iter()
            .map(|s| match s {
                QuerySegment::Key(k) => k.as_str(),
                QuerySegment::Index(_) => "<index>",
                QuerySegment::Wildcard => "<wildcard>",
                QuerySegment::RecursiveDescent => "<descent>",
            })
            .collect()
    }

    #[test]
    fn parse_query_path_reads_a_quoted_bracket_segment_as_one_key() {
        let segments = parse_query_path(r#"labels["app.kubernetes.io/name"]"#);
        assert_eq!(keys(&segments), ["labels", "app.kubernetes.io/name"]);
    }

    #[test]
    fn parse_query_path_quoted_key_may_hold_any_grammar_character() {
        for (path, key) in [
            (r#"["*"]"#, "*"),
            (r#"["a[0]"]"#, "a[0]"),
            ("['a]b']", "a]b"),
            (r#"[""]"#, ""),
            (r#"["say \"hi\""]"#, "say \"hi\""),
            (r#"["back\\slash"]"#, "back\\slash"),
            (r"['it\'s']", "it's"),
            (r#"['double "quotes"']"#, "double \"quotes\""),
            (r#"["..x"]"#, "..x"),
        ] {
            let segments = parse_query_path(path);
            assert_eq!(keys(&segments), [key], "{path}");
        }
    }

    #[test]
    fn parse_query_path_mixes_quoted_keys_with_the_other_segments() {
        let segments = parse_query_path(r#"items[0]["a.b"].c[*]['d']"#);
        assert_eq!(
            keys(&segments),
            ["items", "<index>", "a.b", "c", "<wildcard>", "d"]
        );
        assert!(matches!(segments[1], QuerySegment::Index(0)));
    }

    #[test]
    fn parse_query_path_unterminated_quoted_key_runs_to_the_end() {
        assert_eq!(keys(&parse_query_path(r#"a["bc"#)), ["a", "bc"]);
        assert_eq!(keys(&parse_query_path(r#"a["bc"#)), ["a", "bc"]);
    }

    #[test]
    fn quote_key_round_trips_every_key_through_the_parser() {
        for key in [
            "plain",
            "a.b",
            "*",
            "a[0]",
            "]",
            "[",
            "",
            "say \"hi\"",
            "back\\slash",
            "'",
            "\\\"",
            "app.kubernetes.io/name",
            "..",
            "[*]",
            "trailing\\",
        ] {
            let segments = parse_query_path(&quote_key(key));
            assert_eq!(keys(&segments), [key], "{key:?} -> {:?}", quote_key(key));
        }
    }

    #[test]
    fn join_keys_quotes_only_the_keys_the_grammar_would_misread() {
        assert_eq!(join_keys(["server", "port"]), "server.port");
        assert_eq!(join_keys(["only"]), "only");
        assert_eq!(join_keys(Vec::<&str>::new()), "");
        let path = join_keys(["labels", "app.io/name", "tier", "", "x*", "y"]);
        assert_eq!(path, r#"labels["app.io/name"].tier[""]["x*"].y"#);
        assert_eq!(
            keys(&parse_query_path(&path)),
            ["labels", "app.io/name", "tier", "", "x*", "y"]
        );
    }

    #[test]
    fn push_key_after_an_index_or_a_quoted_segment_needs_no_separator() {
        let mut path = String::from("items[0]");
        push_key(&mut path, "name");
        assert_eq!(path, "items[0].name");
        let mut path = quote_key("a.b");
        push_key(&mut path, "c");
        push_key(&mut path, "d]");
        assert_eq!(path, r#"["a.b"].c["d]"]"#);
        assert_eq!(keys(&parse_query_path(&path)), ["a.b", "c", "d]"]);
    }
}
