// SPDX-FileCopyrightText: 2026 Noyalib
// SPDX-License-Identifier: MIT OR Apache-2.0

//! noyalib's reading of primary-handle tags named after core types.

/// noyalib resolves `!int`, `!float`, `!bool`, `!null` and `!str` (the
/// primary handle) as the core `!!` tags (`resolve_tagged_scalar`);
/// libyaml keeps them as unknown local tags. Such documents differ by
/// design, so no archive comparison applies.
pub(crate) fn present(s: &str) -> bool {
    let b = s.as_bytes();
    b.iter().enumerate().any(|(i, &c)| {
        c == b'!'
            && (i == 0 || b[i - 1] != b'!')
            && ["int", "float", "bool", "null", "str"].iter().any(|name| {
                s[i + 1..].starts_with(name)
                    && b.get(i + 1 + name.len()).is_none_or(|n| {
                        n.is_ascii_whitespace() || matches!(n, b',' | b'[' | b']' | b'{' | b'}')
                    })
            })
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn primary_handle_core_names_are_recognised() {
        for source in ["{!null }", "k: !str 1", "[!int '1']", "!bool\ttrue"] {
            assert!(super::present(source), "{source:?}");
        }
        for source in ["{!!null }", "k: !strict 1", "k: !<!str> 1", "k: !foo"] {
            assert!(!super::present(source), "{source:?}");
        }
    }
}
