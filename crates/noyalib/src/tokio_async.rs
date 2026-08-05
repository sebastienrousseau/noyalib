// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Native async YAML parsing for [`tokio`](https://tokio.rs)
//! runtimes.
//!
//! Bridges noyalib's strict-parser entry points
//! ([`crate::from_str`], [`crate::from_slice`]) onto
//! `tokio::io::AsyncRead` sources without forcing the caller
//! through `tokio::task::spawn_blocking`. Two surface shapes
//! are provided so callers can pick the right ergonomics for
//! their workload:
//!
//! * `from_async_reader` — drain a single document out of any
//!   `tokio::io::AsyncRead` into the caller's `T`.
//! * `from_async_reader_multi` — drain every `---`-separated
//!   document and return `Vec<T>`.
//! * `YamlDecoder<T>` — `tokio_util::codec::Decoder`
//!   implementation for plugging YAML parsing into a
//!   `tokio_util::codec::Framed` pipeline (web-services /
//!   tower-middleware integration).
//!
//! # Backpressure
//!
//! The `from_async_reader` entry points buffer the full payload
//! into a `Vec<u8>` before parsing because the underlying parser
//! is synchronous. The codec surface is the streaming choice:
//! it emits one document per `decode` call as soon as a complete
//! `---` boundary is in the buffer.
//!
//! Gated behind the `tokio` Cargo feature (which transitively
//! enables `tokio-util` and `bytes` for the codec API).
//!
//! # Example
//!
//! ```no_run
//! # #[cfg(feature = "tokio")] {
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use tokio::io::BufReader;
//! let bytes: &[u8] = b"name: noyalib\nversion: 0.0.6\n";
//! let mut reader = BufReader::new(bytes);
//! #[derive(serde::Deserialize)]
//! struct Pkg { name: String, version: String }
//! let pkg: Pkg = noyalib::tokio_async::from_async_reader(&mut reader).await?;
//! assert_eq!(pkg.name, "noyalib");
//! # Ok(()) }
//! # }
//! ```

use bytes::BytesMut;
use core::marker::PhantomData;
use tokio::io::{AsyncRead, AsyncReadExt as _};
use tokio_util::codec::Decoder;

use crate::de::{ParserConfig, from_slice, from_slice_with_config};
use crate::error::{Error, Result};

/// Drain the supplied reader to end-of-stream, then parse the
/// buffered bytes as a single YAML document into `T`.
///
/// Uses the default [`ParserConfig`]; pair with
/// [`from_async_reader_with_config`] to pass custom limits.
///
/// # Errors
///
/// Returns the underlying [`Error`] from either the I/O drain
/// (including reads that exceed [`ParserConfig::max_document_length`])
/// or the parse step.
pub async fn from_async_reader<R, T>(reader: &mut R) -> Result<T>
where
    R: AsyncRead + Unpin,
    // `'static` is inherited from `from_slice_with_config` and
    // `from_str_with_config` — async I/O cannot generally hand
    // out a borrowed `&[u8]` that outlives the await point, so
    // dropping the bound here would only paper over the issue.
    T: serde_core::de::DeserializeOwned + 'static,
{
    from_async_reader_with_config(reader, &ParserConfig::default()).await
}

/// [`from_async_reader`] with a caller-supplied [`ParserConfig`].
///
/// The reader is capped at [`ParserConfig::max_document_length`]
/// bytes via [`tokio::io::AsyncReadExt::take`] so a slow-drip
/// adversary cannot drive the in-memory buffer beyond the
/// configured limit (security finding C3).
///
/// # Errors
///
/// Returns the underlying [`Error`] from either the I/O drain or
/// the parse step. An input larger than `max_document_length`
/// surfaces as [`Error::Io`] with the trailing bytes truncated;
/// the parser then enforces every other limit on the buffered
/// prefix.
pub async fn from_async_reader_with_config<R, T>(reader: &mut R, config: &ParserConfig) -> Result<T>
where
    R: AsyncRead + Unpin,
    // `'static` is inherited from `from_slice_with_config` and
    // `from_str_with_config` — async I/O cannot generally hand
    // out a borrowed `&[u8]` that outlives the await point, so
    // dropping the bound here would only paper over the issue.
    T: serde_core::de::DeserializeOwned + 'static,
{
    let buf = drain_bounded(reader, config.max_document_length).await?;
    let buf = strip_bom_owned(buf);
    from_slice_with_config(&buf, config)
}

/// Drain the reader and parse every `---`-separated document
/// into `Vec<T>` using the default [`ParserConfig`].
///
/// Pair with [`from_async_reader_multi_with_config`] when the
/// caller needs custom limits (it is the version most production
/// services should pick).
///
/// # Errors
///
/// Returns the underlying [`Error`] from either the I/O drain or
/// the parse step.
pub async fn from_async_reader_multi<R, T>(reader: &mut R) -> Result<Vec<T>>
where
    R: AsyncRead + Unpin,
    // `'static` is inherited from `from_slice_with_config` and
    // `from_str_with_config` — async I/O cannot generally hand
    // out a borrowed `&[u8]` that outlives the await point, so
    // dropping the bound here would only paper over the issue.
    T: serde_core::de::DeserializeOwned + 'static,
{
    from_async_reader_multi_with_config(reader, &ParserConfig::default()).await
}

/// [`from_async_reader_multi`] with a caller-supplied
/// [`ParserConfig`]. The reader is bounded by
/// [`ParserConfig::max_document_length`] in exactly the same way
/// as [`from_async_reader_with_config`].
///
/// Uses [`crate::from_slice_with_config`] on the buffered bytes
/// when only one document is present; otherwise routes through
/// the standard multi-document loader so all per-document
/// semantics (duplicate-key policy, anchor budgets, …) match
/// [`crate::load_all_with_config`].
///
/// # Errors
///
/// Returns the underlying [`Error`] from either the I/O drain
/// (including bounded-read overflow and UTF-8 invalidity) or the
/// parse step.
pub async fn from_async_reader_multi_with_config<R, T>(
    reader: &mut R,
    config: &ParserConfig,
) -> Result<Vec<T>>
where
    R: AsyncRead + Unpin,
    // `'static` is inherited from `from_slice_with_config` and
    // `from_str_with_config` — async I/O cannot generally hand
    // out a borrowed `&[u8]` that outlives the await point, so
    // dropping the bound here would only paper over the issue.
    T: serde_core::de::DeserializeOwned + 'static,
{
    let buf = drain_bounded(reader, config.max_document_length).await?;
    let buf = strip_bom_owned(buf);
    // Route UTF-8 invalidity through Error::Io (InvalidData)
    // rather than `Error::custom`, which is serde-flavoured and
    // would mislead callers matching on error kind (M9).
    let text = core::str::from_utf8(&buf)
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
    // Split on `---` with the marker cap honoured (security
    // finding C2), then deserialise each document under the
    // caller's config so every per-document limit fires.
    let docs = crate::doc_boundary::split_documents(text, config.max_documents);
    let mut results = Vec::with_capacity(docs.len());
    for doc in docs {
        results.push(crate::from_str_with_config::<T>(doc, config)?);
    }
    Ok(results)
}

/// Read at most `max_bytes` from `reader` into a fresh `Vec<u8>`.
/// `max_bytes == 0` is treated as "unbounded" only when the
/// caller has explicitly set `ParserConfig::max_document_length`
/// to zero — by default this maps to a 64 MiB cap.
async fn drain_bounded<R>(reader: &mut R, max_bytes: usize) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut buf = Vec::new();
    let take = u64::try_from(max_bytes).unwrap_or(u64::MAX);
    let mut limited = reader.take(take);
    let _ = limited.read_to_end(&mut buf).await.map_err(Error::from)?;
    Ok(buf)
}

/// Strip a leading UTF-8 BOM in-place. The owned `Vec` form
/// keeps the buffer's allocation; we just elide the three BOM
/// bytes at the front.
fn strip_bom_owned(mut buf: Vec<u8>) -> Vec<u8> {
    if crate::doc_boundary::strip_bom(&buf) == 3 {
        let _ = buf.drain(..3);
    }
    buf
}

/// [`tokio_util::codec::Decoder`] that emits one parsed `T` per
/// `---`-delimited YAML document in the byte stream.
///
/// Drop this into a
/// [`tokio_util::codec::FramedRead`] / [`tokio_util::codec::FramedWrite`]
/// pipeline to plug streaming YAML parsing into a tower service
/// chain. Each `decode` call returns:
///
/// * `Ok(Some(T))` — a complete document was found and parsed.
/// * `Ok(None)` — no complete document yet; ask for more bytes.
/// * `Err(e)` — the next document failed to parse.
///
/// The decoder treats `---` at column 0 followed by whitespace
/// or end-of-line as the document terminator, matching the YAML
/// 1.2.2 §9.1.2 directive-end grammar.
#[derive(Debug, Clone)]
pub struct YamlDecoder<T> {
    config: ParserConfig,
    /// Hard cap on the `BytesMut` buffer size between `decode`
    /// calls. `None` means "no cap, trust the input source".
    /// Production services driving `YamlDecoder` over an
    /// untrusted network should set this to a sane upper bound
    /// — when exceeded, `decode` returns an `Error::Io` with
    /// `InvalidData`.
    max_frame_size: Option<usize>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for YamlDecoder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> YamlDecoder<T> {
    /// Create a decoder with default [`ParserConfig`] limits and
    /// no frame-size cap.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
            max_frame_size: None,
            _marker: PhantomData,
        }
    }

    /// Create a decoder with a caller-supplied [`ParserConfig`].
    #[must_use]
    pub fn with_config(config: ParserConfig) -> Self {
        Self {
            config,
            max_frame_size: None,
            _marker: PhantomData,
        }
    }

    /// Set a hard cap on the inter-frame buffer size. When the
    /// `BytesMut` passed to `decode` exceeds `max`, the next
    /// `decode` call returns an `Error::Io` with `InvalidData`
    /// rather than letting the buffer grow without bound.
    #[must_use]
    pub fn max_frame_size(mut self, max: usize) -> Self {
        self.max_frame_size = Some(max);
        self
    }
}

impl<T> Decoder for YamlDecoder<T>
where
    // `'static` is inherited from `from_slice_with_config` and
    // `from_str_with_config` — async I/O cannot generally hand
    // out a borrowed `&[u8]` that outlives the await point, so
    // dropping the bound here would only paper over the issue.
    T: serde_core::de::DeserializeOwned + 'static,
{
    type Item = T;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> core::result::Result<Option<T>, Error> {
        // Frame-size guard (M7) — defended before any scanning
        // work so adversarial slow-drip producers cannot pin
        // arbitrary memory by streaming without `---`.
        if let Some(max) = self.max_frame_size {
            if src.len() > max {
                return Err(Error::from(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "noyalib YamlDecoder: buffer {} > max_frame_size {}",
                        src.len(),
                        max
                    ),
                )));
            }
        }

        // C6 — iterate rather than recurse so an all-whitespace
        //      preamble (or repeated `---` markers preceding the
        //      first real document) cannot blow the stack.
        loop {
            let bytes: &[u8] = src.as_ref();
            let Some(end) = find_doc_boundary(bytes) else {
                return Ok(None);
            };

            // Split off everything up to (but not including) the
            // next `---` marker; the marker stays in `src` to be
            // picked up by the following call.
            let doc = src.split_to(end);
            if doc.iter().all(u8::is_ascii_whitespace) {
                // Skip an all-whitespace preamble silently and
                // retry from the new buffer head; no recursion.
                continue;
            }
            let parsed = from_slice_with_config::<T>(&doc, &self.config)?;
            return Ok(Some(parsed));
        }
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> core::result::Result<Option<T>, Error> {
        if src.is_empty() {
            return Ok(None);
        }
        // Last document — try a normal decode first (it may have
        // a trailing `---` from a previous frame), otherwise parse
        // the remainder.
        if let Some(v) = self.decode(src)? {
            return Ok(Some(v));
        }
        if src.iter().all(u8::is_ascii_whitespace) {
            src.clear();
            return Ok(None);
        }
        let doc = src.split();
        let parsed = from_slice::<T>(&doc)?;
        Ok(Some(parsed))
    }
}

/// Find the byte offset of the next column-0 `---` document
/// boundary, accepting both `\n` and `\r\n` line terminators
/// (security/correctness finding C4 — the previous copy missed
/// CRLF inputs, so Windows-saved files would never frame).
///
/// Returns `None` if no boundary is present in the buffer.
/// The returned offset is the **first byte of the marker** so
/// callers may use [`bytes::BytesMut::split_to`] to consume the
/// preceding document while leaving the marker available for the
/// next frame.
fn find_doc_boundary(bytes: &[u8]) -> Option<usize> {
    crate::doc_boundary::next_marker_after(bytes, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    use tokio::io::BufReader;

    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Pkg {
        name: String,
        version: String,
    }

    #[tokio::test]
    async fn reader_parses_single_document() {
        let mut r = BufReader::new(&b"name: noyalib\nversion: 0.0.6\n"[..]);
        let p: Pkg = from_async_reader(&mut r).await.unwrap();
        assert_eq!(
            p,
            Pkg {
                name: "noyalib".into(),
                version: "0.0.6".into(),
            }
        );
    }

    #[tokio::test]
    async fn reader_multi_parses_each_document() {
        let yaml = b"---\nname: a\nversion: '1'\n---\nname: b\nversion: '2'\n";
        let mut r = BufReader::new(&yaml[..]);
        let docs: Vec<Pkg> = from_async_reader_multi(&mut r).await.unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].name, "a");
        assert_eq!(docs[1].name, "b");
    }

    #[test]
    fn decoder_emits_first_complete_document() {
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        let mut buf = BytesMut::from(&b"name: a\nversion: '1'\n---\nname: b\nversion: '2'\n"[..]);
        let first = decoder.decode(&mut buf).unwrap().unwrap();
        assert_eq!(first.name, "a");
        // The second document is still in the buffer, prefixed
        // with the `---` marker; decode_eof handles it.
        let second = decoder.decode_eof(&mut buf).unwrap().unwrap();
        assert_eq!(second.name, "b");
    }

    #[test]
    fn decoder_returns_none_on_incomplete_buffer() {
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        // No `---` boundary visible — pending.
        let mut buf = BytesMut::from(&b"name: a\n"[..]);
        assert!(decoder.decode(&mut buf).unwrap().is_none());
    }

    #[tokio::test]
    async fn reader_with_config_respects_overrides() {
        let mut r = BufReader::new(&b"name: x\nversion: '1'\n"[..]);
        let cfg = ParserConfig::default();
        let p: Pkg = from_async_reader_with_config(&mut r, &cfg).await.unwrap();
        assert_eq!(p.name, "x");
    }

    #[test]
    fn decoder_with_config_constructor() {
        let cfg = ParserConfig::default();
        let _d: YamlDecoder<Pkg> = YamlDecoder::with_config(cfg);
        let _d2: YamlDecoder<Pkg> = YamlDecoder::default();
        let _printed = format!("{:?}", YamlDecoder::<Pkg>::new());
    }

    #[test]
    fn decoder_eof_on_empty_buffer_returns_none() {
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        let mut buf = BytesMut::new();
        assert!(decoder.decode_eof(&mut buf).unwrap().is_none());
    }

    #[test]
    fn decoder_skips_whitespace_only_preamble() {
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        // Whitespace-only chunk before a `---` boundary — the
        // decoder recurses and emits the document after it.
        let mut buf = BytesMut::from(&b"\n\n---\nname: q\nversion: '2'\n"[..]);
        let p = decoder.decode_eof(&mut buf).unwrap().unwrap();
        assert_eq!(p.name, "q");
    }

    #[test]
    fn decoder_eof_drains_trailing_whitespace() {
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        // First emit the only doc; then EOF should clean up
        // any trailing whitespace without erroring.
        let mut buf = BytesMut::from(&b"name: r\nversion: '3'\n\n\n"[..]);
        let p = decoder.decode_eof(&mut buf).unwrap().unwrap();
        assert_eq!(p.name, "r");
    }

    #[tokio::test]
    async fn reader_multi_handles_invalid_utf8() {
        // 0xFF is invalid UTF-8 — exercises the from_utf8 branch
        // in from_async_reader_multi.
        let mut r = BufReader::new(&[0xFFu8, 0xFE, 0xFD][..]);
        let res: Result<Vec<Pkg>> = from_async_reader_multi(&mut r).await;
        assert!(res.is_err());
    }

    #[test]
    fn find_doc_boundary_handles_short_input() {
        assert!(find_doc_boundary(b"").is_none());
        assert!(find_doc_boundary(b"abc").is_none());
    }

    #[tokio::test]
    async fn reader_strips_leading_bom() {
        // C5 — BOM-prefixed payload must parse identically to
        //      the LF-on-Linux equivalent.
        let mut r = BufReader::new(&b"\xEF\xBB\xBFname: x\nversion: '1'\n"[..]);
        let p: Pkg = from_async_reader(&mut r).await.unwrap();
        assert_eq!(p.name, "x");
    }

    #[tokio::test]
    async fn reader_multi_accepts_crlf() {
        // C4 — Windows-saved YAML round-trips the multi path.
        let yaml = b"---\r\nname: a\r\nversion: '1'\r\n---\r\nname: b\r\nversion: '2'\r\n";
        let mut r = BufReader::new(&yaml[..]);
        let docs: Vec<Pkg> = from_async_reader_multi(&mut r).await.unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].name, "a");
        assert_eq!(docs[1].name, "b");
    }

    #[tokio::test]
    async fn reader_caps_at_max_document_length() {
        // C3 — slow-drip oversize input is truncated at the
        //      configured limit; the parser then surfaces a
        //      parse error rather than OOM.
        let yaml = "a: ".to_string() + &"x".repeat(10_000);
        let cfg = ParserConfig {
            max_document_length: 64,
            ..ParserConfig::default()
        };
        let mut r = BufReader::new(yaml.as_bytes());
        let _ = from_async_reader_with_config::<_, Pkg>(&mut r, &cfg)
            .await
            .expect_err("expected parse error after truncation");
    }

    #[test]
    fn decoder_rejects_oversize_frame() {
        // M7 — frame-size cap prevents adversarial buffer growth.
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new().max_frame_size(16);
        let mut buf = BytesMut::from(&b"name: long-name-no-marker-yet-need-more-bytes"[..]);
        let err = decoder.decode(&mut buf).err().unwrap();
        assert!(err.to_string().contains("max_frame_size"));
    }

    #[test]
    fn decoder_accepts_crlf_boundary() {
        // C4 — `\r\n---\r\n` framing must be recognised.
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        let mut buf =
            BytesMut::from(&b"name: a\r\nversion: '1'\r\n---\r\nname: b\r\nversion: '2'\r\n"[..]);
        let first = decoder.decode(&mut buf).unwrap().unwrap();
        assert_eq!(first.name, "a");
    }

    #[tokio::test]
    async fn reader_multi_with_config_routes_through() {
        // M6 — multi-with-config entry point.
        let yaml = b"---\nname: a\nversion: '1'\n---\nname: b\nversion: '2'\n";
        let mut r = BufReader::new(&yaml[..]);
        let cfg = ParserConfig::default();
        let docs: Vec<Pkg> = from_async_reader_multi_with_config(&mut r, &cfg)
            .await
            .unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn find_doc_boundary_skips_leading_marker() {
        // A leading `---` at byte 0 is not a boundary — it's the
        // start of the first document.
        assert!(find_doc_boundary(b"---\na: 1\n").is_none());
        // The second `---` is the boundary.
        let bs = b"---\na: 1\n---\nb: 2\n";
        let at = find_doc_boundary(bs).unwrap();
        // The boundary points at the `---` after the `\n` (byte 9).
        assert_eq!(at, 9);
    }
}
