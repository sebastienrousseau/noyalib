// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Stream-wide budgets hold across documents on every multi-document
//! entry point.
//!
//! One loader charges `max_events`, `max_nodes`,
//! `max_total_scalar_bytes` and `max_documents` across a whole stream.
//! The entry points that split a stream and parse each document on its
//! own (`parallel`, `recovery`, the Tokio readers, the CST stream) used to give every
//! document a fresh budget, so four documents each under a limit passed
//! together even when the stream was far over it. `max_stream_bytes`
//! was only enforced by the Tokio multi-document reader.

#![allow(dead_code)]

use noyalib::ParserConfig;

/// Four documents, each a flow sequence of ten integers: 16 parser
/// events, 11 nodes and 10 scalar bytes per document.
fn four_docs() -> String {
    "---\n[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]\n".repeat(4)
}

fn cases() -> Vec<(&'static str, ParserConfig)> {
    let d = ParserConfig::default;
    vec![
        ("max_events", d().max_events(40)),
        ("max_nodes", d().max_nodes(25)),
        ("max_total_scalar_bytes", d().max_total_scalar_bytes(25)),
        (
            "max_stream_bytes",
            d().max_stream_bytes(64).max_document_length(64),
        ),
    ]
}

type MultiReader = fn(&str, &ParserConfig) -> Result<usize, String>;

fn readers() -> Vec<(&'static str, MultiReader)> {
    #[allow(unused_mut)] // the pushes below are feature-gated
    let mut r: Vec<(&'static str, MultiReader)> = vec![("load_all_with_config", |s, c| {
        noyalib::load_all_with_config(s, c)
            .map(|d| d.count())
            .map_err(|e| e.to_string())
    })];
    r.push(("cst::parse_stream_with_config", |s, c| {
        noyalib::cst::parse_stream_with_config(s, c)
            .map(|d| d.len())
            .map_err(|e| e.to_string())
    }));
    #[cfg(feature = "parallel")]
    r.push(("parallel::values_with_config", |s, c| {
        noyalib::parallel::values_with_config(s, c)
            .map(|v| v.len())
            .map_err(|e| e.to_string())
    }));
    #[cfg(feature = "recovery")]
    r.push(("recovery::parse_lenient_with", |s, c| {
        let lc = noyalib::recovery::LenientConfig {
            base_config: c.clone(),
            ..Default::default()
        };
        let r = noyalib::recovery::parse_lenient_with(s, &lc);
        match r.errors.first() {
            None => Ok(1),
            Some(e) => Err(e.to_string()),
        }
    }));
    #[cfg(feature = "tokio")]
    r.push(("tokio from_async_reader_multi_with_config", |s, c| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let mut reader = std::io::Cursor::new(s.as_bytes().to_vec());
            noyalib::tokio_async::from_async_reader_multi_with_config::<_, noyalib::Value>(
                &mut reader,
                c,
            )
            .await
            .map(|v| v.len())
            .map_err(|e| e.to_string())
        })
    }));
    #[cfg(feature = "tokio")]
    r.push(("tokio YamlDecoder", decode_all));
    r
}

#[cfg(feature = "tokio")]
fn decode_all(s: &str, c: &ParserConfig) -> Result<usize, String> {
    use tokio_util::codec::Decoder as _;
    let mut decoder = noyalib::tokio_async::YamlDecoder::<noyalib::Value>::with_config(c.clone());
    let mut buf = bytes::BytesMut::from(s.as_bytes());
    let mut n = 0;
    while let Some(_doc) = decoder.decode(&mut buf).map_err(|e| e.to_string())? {
        n += 1;
    }
    if decoder
        .decode_eof(&mut buf)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        n += 1;
    }
    Ok(n)
}

#[test]
fn stream_wide_budgets_span_every_document() {
    let yaml = four_docs();
    let mut gaps = Vec::new();
    for (limit, cfg) in cases() {
        for (name, read) in readers() {
            // The Tokio frame decoder has no whole-stream length to check.
            if limit == "max_stream_bytes" && name == "tokio YamlDecoder" {
                continue;
            }
            if let Ok(n) = read(&yaml, &cfg) {
                gaps.push(format!(
                    "{name} accepted the stream over {limit} ({n} documents)"
                ));
            }
        }
    }
    assert!(gaps.is_empty(), "{gaps:#?}");
}

#[test]
fn stream_wide_budgets_still_admit_a_stream_inside_them() {
    let yaml = four_docs();
    let cfg = ParserConfig::default()
        .max_events(200)
        .max_nodes(100)
        .max_total_scalar_bytes(100);
    for (name, read) in readers() {
        assert!(read(&yaml, &cfg).is_ok(), "{name}");
    }
}

/// The Tokio frame decoder counts documents across frames.
#[cfg(feature = "tokio")]
#[test]
fn yaml_decoder_enforces_max_documents_across_frames() {
    let yaml = "---\na: 1\n".repeat(5);
    let cfg = ParserConfig::default().max_documents(2);
    let err = decode_all(&yaml, &cfg).expect_err("five documents exceed a limit of two");
    assert!(err.contains("document"), "{err}");
    let ok = decode_all(&yaml, &ParserConfig::default().max_documents(5)).expect("five fit");
    assert_eq!(ok, 5);
}

#[test]
fn cst_stream_documents_are_edited_on_their_own_budget() {
    // Two documents use 32 of the stream's 40 events. Editing one
    // re-parses only that document, so it must not be charged against
    // what the stream already spent.
    let src = "---\n[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]\n".repeat(2);
    let cfg = ParserConfig::default().max_events(40);
    let mut docs = noyalib::cst::parse_stream_with_config(&src, &cfg).unwrap();
    for doc in &mut docs {
        doc.set("[0]", "9").unwrap();
    }
    assert!(docs[1].source().contains("[9, 1"), "{}", docs[1].source());
}
