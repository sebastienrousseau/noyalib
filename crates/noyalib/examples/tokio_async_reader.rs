// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `tokio_async` — parse a YAML stream from a tokio `AsyncRead`
//! source without `spawn_blocking`, and from a
//! `tokio_util::codec::Framed` pipeline.
//!
//! Run: `cargo run --example tokio_async_reader --features tokio`

#[cfg(feature = "tokio")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    use bytes::BytesMut;
    use noyalib::tokio_async::{YamlDecoder, from_async_reader_multi};

    use tokio::io::BufReader;
    use tokio_util::codec::Decoder;

    #[derive(Debug, serde::Deserialize)]
    struct Pkg {
        name: String,
        version: String,
    }

    let stream = b"\
---
name: noyalib
version: 0.0.6
---
name: noya-cli
version: 0.0.6
---
name: noyalib-lsp
version: 0.0.6
";

    // ── Pattern 1: drain-and-parse via from_async_reader_multi ──
    let mut buf_reader = BufReader::new(&stream[..]);
    let docs: Vec<Pkg> = from_async_reader_multi(&mut buf_reader)
        .await
        .expect("parse multi");
    println!("from_async_reader_multi:");
    for (i, pkg) in docs.iter().enumerate() {
        println!("  [{i}] {} {}", pkg.name, pkg.version);
    }

    // ── Pattern 2: streaming codec via YamlDecoder directly ──
    //
    // For brevity the example feeds the whole buffer in one shot
    // and drives `decode` / `decode_eof` by hand. In a real
    // server you'd wrap the codec in `tokio_util::codec::FramedRead`
    // and pull items via `futures::StreamExt::next` — that pulls
    // in `futures-util`, which is intentionally not a noyalib dep.
    let mut decoder = YamlDecoder::<Pkg>::new();
    let mut buf = BytesMut::from(&stream[..]);
    println!("\nYamlDecoder (manual drive):");
    let mut i = 0;
    while let Some(pkg) = decoder.decode(&mut buf).expect("decode") {
        println!("  [{i}] {} {}", pkg.name, pkg.version);
        i += 1;
    }
    if let Some(pkg) = decoder.decode_eof(&mut buf).expect("decode_eof") {
        println!("  [{i}] {} {}", pkg.name, pkg.version);
    }

    // ── Pattern 3: untrusted-network frame-size cap (v0.0.6) ──
    //
    // Production services driving `YamlDecoder` over a network
    // stream MUST cap inter-frame buffer growth or an adversarial
    // producer that streams without `---` can pin arbitrary memory.
    // Set the cap with `max_frame_size(usize)` — when the buffer
    // exceeds it, the next `decode` call returns `Error::Io`.
    let mut guarded = YamlDecoder::<Pkg>::new().max_frame_size(64);
    // Stream a single doc whose first line is longer than 64 bytes
    // and no `---` boundary lands inside the cap.
    let oversize =
        b"name: a-very-long-package-name-that-definitely-exceeds-sixty-four-bytes\nversion: '1'\n";
    let mut buf = BytesMut::from(&oversize[..]);
    match guarded.decode(&mut buf) {
        Err(e) => println!("\nmax_frame_size(64) rejected the oversize frame: {e}"),
        Ok(_) => println!("\nunexpected: oversize frame should have been rejected"),
    }
}

#[cfg(not(feature = "tokio"))]
fn main() {
    eprintln!("This example requires the `tokio` feature.");
    eprintln!("Run with: cargo run --example tokio_async_reader --features tokio");
}
