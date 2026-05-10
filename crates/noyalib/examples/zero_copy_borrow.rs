// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! True zero-copy `Deserialize<'de>` for `&'de str` and
//! `Cow<'de, str>` directly from the YAML input slice.
//!
//! Pairs with `noyalib::from_str_borrowing` — the streaming
//! deserialiser routes plain scalars through `visit_borrowed_str`
//! when the parser produced a `Cow::Borrowed` event. Scalars that
//! required transformation (escape decoding, line folding, alias
//! replay) fall back to owned buffers; see
//! [`noyalib::borrowed::TransformReason`] for the catalogue.
//!
//! Run: `cargo run --example zero_copy_borrow`

#[path = "support.rs"]
mod support;

use noyalib::borrowed::TransformReason;
use noyalib::from_str_borrowing;
use serde::Deserialize;
use std::borrow::Cow;

#[derive(Debug, Deserialize)]
struct Config<'a> {
    name: &'a str,
    role: &'a str,
}

#[derive(Debug, Deserialize)]
struct CowConfig<'a> {
    #[serde(borrow)]
    plain: Cow<'a, str>,
    #[serde(borrow)]
    escaped: Cow<'a, str>,
}

fn main() {
    support::header("noyalib -- zero_copy_borrow");

    support::task_with_output("&'a str fields borrow from the input slice", || {
        let yaml = "name: noyalib\nrole: parser\n";
        let cfg: Config<'_> = from_str_borrowing(yaml).unwrap();
        let yaml_range = yaml.as_ptr() as usize..(yaml.as_ptr() as usize + yaml.len());
        assert!(yaml_range.contains(&(cfg.name.as_ptr() as usize)));
        assert!(yaml_range.contains(&(cfg.role.as_ptr() as usize)));
        vec![
            format!("name={}", cfg.name),
            format!("role={}", cfg.role),
            "both slices borrow directly from the yaml input".into(),
        ]
    });

    support::task_with_output("Cow<str> accepts both borrowed and owned scalars", || {
        let yaml = "plain: hello\nescaped: \"line\\nbreak\"\n";
        let cfg: CowConfig<'_> = from_str_borrowing(yaml).unwrap();
        let plain_kind = if matches!(cfg.plain, Cow::Borrowed(_)) {
            "Cow::Borrowed"
        } else {
            "Cow::Owned"
        };
        let escaped_kind = if matches!(cfg.escaped, Cow::Borrowed(_)) {
            "Cow::Borrowed"
        } else {
            "Cow::Owned"
        };
        vec![
            format!("plain   = {} ({plain_kind})", cfg.plain),
            format!("escaped = {:?} ({escaped_kind})", cfg.escaped),
        ]
    });

    support::task_with_output("TransformReason enumerates why borrows fail", || {
        [
            TransformReason::EscapeSequence,
            TransformReason::LineFold,
            TransformReason::TagResolution,
            TransformReason::QuotedScalar,
            TransformReason::AliasExpansion,
        ]
        .iter()
        .map(|r| format!("{r:?}: {r}"))
        .collect()
    });
}
