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
//! * [`AsyncYamlStream`](crate::tokio_async::AsyncYamlStream): a
//!   backpressured `Stream` of parsed documents, constructed with
//!   [`async_yaml_stream`](crate::tokio_async::async_yaml_stream) or
//!   [`async_yaml_stream_with_config`](crate::tokio_async::async_yaml_stream_with_config).
//! * [`YamlDecoder`](crate::tokio_async::YamlDecoder): the lower-level
//!   `tokio_util::codec::Decoder`
//!   used by the stream surface and available for custom framed
//!   pipelines.
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
use tokio_util::codec::{Decoder, FramedRead};

use crate::de::{ParserConfig, from_slice_with_config};
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
/// The reader probes at most one byte beyond
/// [`ParserConfig::max_document_length`] via
/// [`tokio::io::AsyncReadExt::take`] so a slow-drip adversary
/// cannot drive the in-memory buffer beyond the configured limit.
///
/// # Errors
///
/// Returns the underlying [`Error`] from either the I/O drain or
/// the parse step. An input larger than `max_document_length`
/// returns [`Error::Io`] with [`std::io::ErrorKind::InvalidData`];
/// a truncated prefix is never parsed as a complete document.
pub async fn from_async_reader_with_config<R, T>(reader: &mut R, config: &ParserConfig) -> Result<T>
where
    R: AsyncRead + Unpin,
    // `'static` is inherited from `from_slice_with_config` and
    // `from_str_with_config` — async I/O cannot generally hand
    // out a borrowed `&[u8]` that outlives the await point, so
    // dropping the bound here would only paper over the issue.
    T: serde_core::de::DeserializeOwned + 'static,
{
    let buf = drain_bounded(reader, config.max_document_length, "max_document_length").await?;
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
/// [`ParserConfig::max_stream_bytes`]. Every split document is then
/// bounded independently by [`ParserConfig::max_document_length`].
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
    let buf = drain_bounded(reader, config.max_stream_bytes, "max_stream_bytes").await?;
    let buf = strip_bom_owned(buf);
    // Route UTF-8 invalidity through Error::Io (InvalidData)
    // rather than `Error::custom`, which is serde-flavoured and
    // would mislead callers matching on error kind (M9).
    let text = core::str::from_utf8(&buf)
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
    // Split on `---` with the marker cap honoured (security
    // finding C2), then deserialise each document under the
    // caller's config so every per-document limit fires.
    let docs = crate::doc_boundary::split_documents_checked(text, config.max_documents)?;
    let mut results = Vec::with_capacity(docs.len());
    for doc in docs {
        results.push(crate::from_str_with_config::<T>(doc, config)?);
    }
    Ok(results)
}

/// Read at most `max_bytes` from `reader` into a fresh `Vec<u8>`.
/// One additional byte is probed so an over-limit input is rejected
/// rather than silently truncated to a possibly valid YAML prefix.
/// Consistent with the synchronous parser, `max_bytes == 0` permits
/// only an empty input.
async fn drain_bounded<R>(reader: &mut R, max_bytes: usize, limit_name: &str) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let probe_bytes = max_bytes.saturating_add(1);
    let initial_capacity = max_bytes.min(16 * 1024);
    let mut buf = Vec::with_capacity(initial_capacity);
    let take = u64::try_from(probe_bytes).unwrap_or(u64::MAX);
    let mut limited = reader.take(take);
    let _ = limited.read_to_end(&mut buf).await.map_err(Error::from)?;
    if buf.len() > max_bytes {
        return Err(Error::from(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "noyalib async reader: input {} > {limit_name} {}",
                buf.len(),
                max_bytes
            ),
        )));
    }
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
    /// calls. Constructors derive it from
    /// `ParserConfig::max_document_length`; callers may tighten or
    /// raise it explicitly with [`Self::max_frame_size`].
    max_frame_size: Option<usize>,
    _marker: PhantomData<fn() -> T>,
}

/// A backpressured asynchronous stream of YAML documents.
///
/// The reader is consumed through Tokio's [`AsyncRead`] contract. Each
/// stream item is parsed only when the consumer polls for it, and source
/// order is preserved. The decoder buffers at most one incomplete document
/// up to its configured frame limit, although an individual read may also
/// contain later complete documents that remain buffered until subsequent
/// polls.
///
/// Construct this type with [`async_yaml_stream`] or
/// [`async_yaml_stream_with_config`]. Consumers can use any compatible
/// `StreamExt` implementation to await items.
pub type AsyncYamlStream<R, T> = FramedRead<R, YamlDecoder<T>>;

/// Wrap an asynchronous reader as a backpressured YAML document stream.
///
/// The default [`ParserConfig`] applies to each document independently.
/// Use [`async_yaml_stream_with_config`] for custom limits and policies.
#[must_use]
pub fn async_yaml_stream<R, T>(reader: R) -> AsyncYamlStream<R, T> {
    FramedRead::new(reader, YamlDecoder::new())
}

/// Wrap an asynchronous reader as a backpressured YAML document stream with
/// caller-supplied parser limits and policies.
#[must_use]
pub fn async_yaml_stream_with_config<R, T>(
    reader: R,
    config: ParserConfig,
) -> AsyncYamlStream<R, T> {
    FramedRead::new(reader, YamlDecoder::with_config(config))
}

impl<T> Default for YamlDecoder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> YamlDecoder<T> {
    /// Create a decoder with default [`ParserConfig`] limits. The
    /// frame-size cap defaults to `max_document_length`.
    #[must_use]
    pub fn new() -> Self {
        let config = ParserConfig::default();
        let max_frame_size = Some(config.max_document_length);
        Self {
            config,
            max_frame_size,
            _marker: PhantomData,
        }
    }

    /// Create a decoder with a caller-supplied [`ParserConfig`].
    #[must_use]
    pub fn with_config(config: ParserConfig) -> Self {
        let max_frame_size = Some(config.max_document_length);
        Self {
            config,
            max_frame_size,
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
        // C6 — iterate rather than recurse so an all-whitespace
        //      preamble (or repeated `---` markers preceding the
        //      first real document) cannot blow the stack.
        loop {
            let bytes: &[u8] = src.as_ref();
            let boundary = find_doc_boundary(bytes);

            // Apply the cap to the next logical document, not the entire
            // read buffer. A single AsyncRead poll may legally return several
            // complete, individually bounded documents. Reject only when the
            // first document or incomplete frame exceeds the limit.
            if let Some(max) = self.max_frame_size {
                let frame_len = boundary.unwrap_or(bytes.len());
                if frame_len > max {
                    return Err(Error::from(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("noyalib YamlDecoder: frame {frame_len} > max_frame_size {max}"),
                    )));
                }
            }

            let Some(end) = boundary else {
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
        let parsed = from_slice_with_config::<T>(&doc, &self.config)?;
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
        // C3 — a valid prefix must never be accepted after the
        //      configured limit silently truncates its trailing data.
        let valid_prefix = "name: x\nversion: '1'\n";
        let yaml = format!("{valid_prefix}ignored: true\n");
        let cfg = ParserConfig {
            max_document_length: valid_prefix.len(),
            ..ParserConfig::default()
        };
        let mut r = BufReader::new(yaml.as_bytes());
        let err = from_async_reader_with_config::<_, Pkg>(&mut r, &cfg)
            .await
            .expect_err("over-limit input must be rejected, not truncated");
        assert!(err.to_string().contains("max_document_length"));
    }

    #[tokio::test]
    async fn reader_accepts_input_exactly_at_limit() {
        let yaml = "name: x\nversion: '1'\n";
        let cfg = ParserConfig {
            max_document_length: yaml.len(),
            ..ParserConfig::default()
        };
        let mut r = BufReader::new(yaml.as_bytes());
        let pkg = from_async_reader_with_config::<_, Pkg>(&mut r, &cfg)
            .await
            .unwrap();
        assert_eq!(pkg.name, "x");
    }

    #[tokio::test]
    async fn reader_zero_limit_rejects_nonempty_input() {
        let cfg = ParserConfig {
            max_document_length: 0,
            ..ParserConfig::default()
        };
        let mut r = BufReader::new(&b"null\n"[..]);
        let err = from_async_reader_with_config::<_, crate::Value>(&mut r, &cfg)
            .await
            .expect_err("a zero limit must reject nonempty input");
        assert!(err.to_string().contains("max_document_length 0"));
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
    fn decoder_accepts_multiple_buffered_documents_beyond_frame_cap() {
        let cfg = ParserConfig {
            max_document_length: 28,
            ..ParserConfig::default()
        };
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::with_config(cfg);
        let mut buf = BytesMut::from(&b"name: a\nversion: '1'\n---\nname: b\nversion: '2'\n"[..]);
        assert!(buf.len() > 28);

        let first = decoder.decode(&mut buf).unwrap().unwrap();
        assert_eq!(first.name, "a");
        let second = decoder.decode_eof(&mut buf).unwrap().unwrap();
        assert_eq!(second.name, "b");
    }

    #[test]
    fn stream_constructors_preserve_decoder_configuration() {
        let default_stream = async_yaml_stream::<_, Pkg>(&b""[..]);
        assert_eq!(
            default_stream.decoder().max_frame_size,
            Some(ParserConfig::default().max_document_length)
        );

        let cfg = ParserConfig {
            max_document_length: 17,
            ..ParserConfig::default()
        };
        let configured_stream = async_yaml_stream_with_config::<_, Pkg>(&b""[..], cfg);
        assert_eq!(configured_stream.decoder().max_frame_size, Some(17));
    }

    #[test]
    fn decoder_derives_frame_cap_from_parser_config() {
        let cfg = ParserConfig {
            max_document_length: 16,
            ..ParserConfig::default()
        };
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::with_config(cfg);
        let mut buf = BytesMut::from(&b"name: long-name-without-boundary"[..]);
        let err = decoder.decode(&mut buf).unwrap_err();
        assert!(err.to_string().contains("max_frame_size 16"));
    }

    #[test]
    fn decoder_eof_preserves_parser_config() {
        let cfg = ParserConfig::strict();
        let mut decoder: YamlDecoder<crate::Value> = YamlDecoder::with_config(cfg);
        let mut buf = BytesMut::from(&b"a: 1\na: 2\n"[..]);
        let err = decoder
            .decode_eof(&mut buf)
            .expect_err("strict duplicate-key policy must apply at EOF");
        assert!(matches!(
            err.kind(),
            crate::error::ErrorKind::DuplicateKey | crate::error::ErrorKind::KeyCollision
        ));
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

    #[tokio::test]
    async fn multi_reader_separates_stream_and_document_byte_limits() {
        let yaml = b"---\nname: a\nversion: '1'\n---\nname: b\nversion: '2'\n";
        let cfg = ParserConfig::default()
            .max_document_length(32)
            .max_stream_bytes(yaml.len());
        let mut reader = BufReader::new(&yaml[..]);

        let docs: Vec<Pkg> = from_async_reader_multi_with_config(&mut reader, &cfg)
            .await
            .unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[tokio::test]
    async fn multi_reader_rejects_stream_byte_overflow() {
        let yaml = b"---\nname: a\nversion: '1'\n---\nname: b\nversion: '2'\n";
        let cfg = ParserConfig::default().max_stream_bytes(yaml.len() - 1);
        let mut reader = BufReader::new(&yaml[..]);

        let error = from_async_reader_multi_with_config::<_, Pkg>(&mut reader, &cfg)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("max_stream_bytes"));
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
