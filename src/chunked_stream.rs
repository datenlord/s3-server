//! aws-chunked stream

// FIXME: verify the correctness of the state machine

#![allow(clippy::redundant_pub_crate)]

use crate::{headers::AmzDate, signature_v4, utils::Apply};

use std::convert::TryInto;
use std::io;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::vec;

use bytes::Bytes;
use futures::stream::Stream;
use memchr::memchr;
use pin_project_lite::pin_project;

pin_project! {
    /// aws-chunked
    pub struct ChunkedStream<S> {
        #[pin]
        body: S,

        signature_ctx: SignatureCtx,

        state: State,
    }
}

/// signature ctx
#[derive(Debug)]
struct SignatureCtx {
    /// date
    amz_date: AmzDate,

    /// region
    region: Box<str>,

    /// secret_key
    secret_key: Box<str>,

    /// previous chunk's signature
    prev_signature: Box<str>,
}

/// state machine
#[derive(Debug)]
enum State {
    /// ReadindMeta
    ReadingMeta {
        /// previous bytes
        prev_bytes: Option<Bytes>,

        /// buf
        buf: Vec<u8>,
    },
    /// ReadingData
    ReadingData {
        /// bytes to read
        remaining_data_size: usize,
        /// chunk signature according to the meta
        expected_signature: Box<[u8]>,
        /// previous bytes
        prev_bytes: Option<Bytes>,
        /// buf
        buf: Vec<Bytes>,
    },
    /// ReleasingData
    ReleasingData {
        /// verified data
        data_iter: vec::IntoIter<Bytes>,
        /// remaining bytes
        remaining_bytes: Option<Bytes>,
    },
    /// Error
    Error {
        /// error kind
        kind: ErrorKind,
    },
}

#[derive(Debug, thiserror::Error)]
/// `ChunkedStreamError`
pub enum ChunkedStreamError {
    /// IO error
    #[error("ChunkedStreamError: Io: {}",.0)]
    Io(io::Error),
    /// Signature mismatch
    #[error("ChunkedStreamError: SignatureMismatch")]
    SignatureMismatch,
    /// Encoding error
    #[error("ChunkedStreamError: EncodingError")]
    EncodingError,
    /// Incomplete stream
    #[error("ChunkedStreamError: Incomplete")]
    Incomplete,
}

#[derive(Debug)]
/// unrecoverable error kind
enum ErrorKind {
    /// Signature mismatch
    SignatureMismatch,
    /// Encoding error
    EncodingError,
    /// Incomplete stream
    Incomplete,
}

impl<S> ChunkedStream<S>
where
    S: Stream<Item = io::Result<Bytes>> + Send + 'static,
{
    /// Constructs a new `AwsChunkedStream`
    pub fn new(
        body: S,
        seed_signature: Box<str>,
        amz_date: AmzDate,
        region: Box<str>,
        secret_key: Box<str>,
    ) -> Self {
        Self {
            body,
            state: State::ReadingMeta {
                prev_bytes: None,
                buf: Vec::new(),
            },
            signature_ctx: SignatureCtx {
                prev_signature: seed_signature,
                amz_date,
                region,
                secret_key,
            },
        }
    }
}

/// Chunk meta
struct ChunkMeta<'a> {
    /// chunk size
    size: usize,
    /// chunk signature
    signature: &'a [u8],
}

/// nom parser
fn parse_chunk_meta(mut input: &[u8]) -> nom::IResult<&[u8], ChunkMeta<'_>> {
    use nom::{
        bytes::complete::{tag, take, take_till1},
        combinator::{all_consuming, map_res},
        number::complete::hex_u32,
        sequence::tuple,
    };

    let parser = all_consuming(tuple((
        take_till1(|c| c == b';'),
        tag(b";chunk-signature="),
        take(64_usize),
        tag(b"\r\n"),
    )));

    let (size_str, signature) = parser(input)?.apply(|(remain, (size_str, _, signature, _))| {
        input = remain;
        (size_str, signature)
    });

    let (_, size) = map_res(hex_u32, |n| n.try_into())(size_str)?;

    Ok((input, ChunkMeta { size, signature }))
}

/// check signature
fn check_signature(ctx: &SignatureCtx, expected_signature: &[u8], chunk_data: &[Bytes]) -> bool {
    let string_to_sign = signature_v4::create_chunk_string_to_sign(
        &ctx.amz_date,
        &ctx.region,
        &ctx.prev_signature,
        chunk_data,
    );

    let chunk_signature = signature_v4::calculate_signature(
        &string_to_sign,
        &ctx.secret_key,
        &ctx.amz_date,
        &ctx.region,
    );

    chunk_signature.as_bytes() == expected_signature
}

/// state machine: poll read meta
fn poll_read_meta<S: Stream<Item = io::Result<Bytes>> + Send + 'static>(
    mut body: Pin<&mut S>,
    cx: &mut Context<'_>,
    prev_bytes: &mut Option<Bytes>,
    buf: &mut Vec<u8>,
) -> Poll<Option<Result<State, ChunkedStreamError>>> {
    let mut push_meta_bytes = |mut bytes: Bytes| {
        if let Some(idx) = memchr(b'\n', bytes.as_ref()) {
            let len = idx.wrapping_add(1); // NOTE: idx < bytes.len()
            let leading = bytes.split_to(len);
            buf.extend_from_slice(leading.as_ref());
            Some(bytes)
        } else {
            buf.extend_from_slice(bytes.as_ref());
            None
        }
    };

    let mut poll_meta = || {
        if let Some(bytes) = prev_bytes.take() {
            if let Some(remaining_bytes) = push_meta_bytes(bytes) {
                return Poll::Ready(Some(Ok(remaining_bytes)));
            }
        }
        loop {
            match futures::ready!(body.as_mut().poll_next(cx)) {
                None => return Poll::Ready(None),
                Some(Err(e)) => return Poll::Ready(Some(Err(ChunkedStreamError::Io(e)))),
                Some(Ok(bytes)) => {
                    if let Some(remaining_bytes) = push_meta_bytes(bytes) {
                        return Poll::Ready(Some(Ok(remaining_bytes)));
                    }
                }
            }
        }
    };

    let prev_bytes = match futures::ready!(poll_meta()?) {
        None => return Poll::Ready(None),
        Some(remaining_bytes) => {
            if remaining_bytes.is_empty() {
                None
            } else {
                Some(remaining_bytes)
            }
        }
    };

    match parse_chunk_meta(buf) {
        Ok((_, meta)) => State::ReadingData {
            remaining_data_size: meta.size,
            expected_signature: meta.signature.into(),
            prev_bytes,
            buf: Vec::new(),
        },
        Err(_) => State::Error {
            kind: ErrorKind::EncodingError,
        },
    }
    .apply(|s| Poll::Ready(Some(Ok(s))))
}

/// state machine: poll read data
fn poll_read_data<S: Stream<Item = io::Result<Bytes>> + Send + 'static>(
    mut body: Pin<&mut S>,
    cx: &mut Context<'_>,
    signature_ctx: &mut SignatureCtx,
    remaining_data_size: &mut usize,
    expected_signature: &[u8],
    prev_bytes: &mut Option<Bytes>,
    bytes_buffer: &mut Vec<Bytes>,
) -> Poll<Option<Result<State, ChunkedStreamError>>> {
    let mut push_bytes = |mut bytes: Bytes| {
        if *remaining_data_size == 0 {
            return Some(bytes);
        }
        if *remaining_data_size <= bytes.len() {
            let data = bytes.split_to(*remaining_data_size);
            bytes_buffer.push(data);
            *remaining_data_size = 0;
            Some(bytes)
        } else {
            *remaining_data_size = remaining_data_size.wrapping_sub(bytes.len());
            bytes_buffer.push(bytes);
            None
        }
    };
    let mut remaining_bytes = 'outer: loop {
        if let Some(bytes) = prev_bytes.take() {
            let opt = push_bytes(bytes);
            if opt.is_some() {
                break 'outer opt;
            }
        }
        loop {
            match futures::ready!(body.as_mut().poll_next(cx)) {
                None => {
                    return State::Error {
                        kind: ErrorKind::Incomplete,
                    }
                    .apply(|s| Poll::Ready(Some(Ok(s))))
                }
                Some(Err(e)) => {
                    return Poll::Ready(Some(Err(ChunkedStreamError::Io(e))));
                }
                Some(Ok(bytes)) => {
                    dbg!(&bytes);
                    let opt = push_bytes(bytes);
                    if opt.is_some() {
                        break 'outer opt;
                    }
                }
            }
        }
    };
    for expected_byte in b"\r\n" {
        loop {
            match remaining_bytes {
                None => match futures::ready!(body.as_mut().poll_next(cx)) {
                    None => {
                        return State::Error {
                            kind: ErrorKind::Incomplete,
                        }
                        .apply(|s| Poll::Ready(Some(Ok(s))))
                    }
                    Some(Err(e)) => {
                        return Poll::Ready(Some(Err(ChunkedStreamError::Io(e))));
                    }
                    Some(Ok(bytes)) => remaining_bytes = Some(bytes),
                },
                Some(ref mut bytes) => match bytes.as_ref() {
                    [] => {
                        remaining_bytes = None;
                        continue;
                    }
                    [x, ..] if x == expected_byte => {
                        drop(bytes.split_to(1));
                        break;
                    }
                    _ => {
                        return State::Error {
                            kind: ErrorKind::EncodingError,
                        }
                        .apply(|s| Poll::Ready(Some(Ok(s))));
                    }
                },
            }
        }
    }

    let remaining_bytes =
        remaining_bytes.and_then(|bytes| if bytes.is_empty() { None } else { Some(bytes) });

    if check_signature(signature_ctx, expected_signature, bytes_buffer) {
        signature_ctx.prev_signature = std::str::from_utf8(expected_signature)
            .unwrap_or_else(|_| unreachable!())
            .into();

        State::ReleasingData {
            data_iter: mem::take(bytes_buffer).into_iter(),
            remaining_bytes,
        }
    } else {
        State::Error {
            kind: ErrorKind::SignatureMismatch,
        }
    }
    .apply(|s| Poll::Ready(Some(Ok(s))))
}

impl<S> Stream for ChunkedStream<S>
where
    S: Stream<Item = io::Result<Bytes>> + Send + 'static,
{
    type Item = Result<Bytes, ChunkedStreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let mut body: Pin<&mut S> = this.body;
        let state: &mut State = this.state;
        let signature_ctx: &mut SignatureCtx = this.signature_ctx;

        'state_machine: loop {
            match state {
                State::ReadingMeta { prev_bytes, buf } => {
                    match futures::ready!(poll_read_meta(body.as_mut(), cx, prev_bytes, buf)?) {
                        None => return Poll::Ready(None),
                        Some(s) => *state = s,
                    }
                    continue 'state_machine;
                }
                State::ReadingData {
                    remaining_data_size,
                    expected_signature,
                    prev_bytes,
                    buf,
                } => {
                    match futures::ready!(poll_read_data(
                        body.as_mut(),
                        cx,
                        signature_ctx,
                        remaining_data_size,
                        expected_signature,
                        prev_bytes,
                        buf,
                    )?) {
                        None => return Poll::Ready(None),
                        Some(s) => *state = s,
                    }
                    continue 'state_machine;
                }
                State::ReleasingData {
                    data_iter,
                    remaining_bytes,
                } => {
                    if let Some(bytes) = data_iter.next() {
                        return Poll::Ready(Some(Ok(bytes)));
                    } else {
                        *state = State::ReadingMeta {
                            prev_bytes: remaining_bytes.take(),
                            buf: Vec::new(),
                        };
                        continue 'state_machine;
                    }
                }
                State::Error { kind } => {
                    return match kind {
                        ErrorKind::SignatureMismatch => ChunkedStreamError::SignatureMismatch,
                        ErrorKind::EncodingError => ChunkedStreamError::EncodingError,
                        ErrorKind::Incomplete => ChunkedStreamError::Incomplete,
                    }
                    .apply(|e| Poll::Ready(Some(Err(e))))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::Also;

    use bytes::BytesMut;
    use futures::stream::StreamExt;

    #[tokio::test]
    async fn example_put_object_chunked_stream() {
        let chunk1_meta = b"10000;chunk-signature=ad80c730a21e5b8d04586a2213dd63b9a0e99e0e2307b0ade35a65485a288648\r\n";
        let chunk2_meta = b"400;chunk-signature=0055627c9e194cb4542bae2aa5492e3c1575bbb81b612b7d234b86a503ef5497\r\n";
        let chunk3_meta = b"0;chunk-signature=b6c6ea8a5354eaf15b3cb7646744f4275b71ea724fed81ceb9323e279d449df9\r\n";

        let chunk1_data = vec![b'a'; 0x10000]; // 65536
        let chunk2_data = vec![b'a'; 1024];

        let chunk1 = BytesMut::from(chunk1_meta.as_ref())
            .also(|b| b.extend_from_slice(&chunk1_data))
            .also(|b| b.extend_from_slice(b"\r\n"))
            .freeze();

        let chunk2 = BytesMut::from(chunk2_meta.as_ref())
            .also(|b| b.extend_from_slice(&chunk2_data))
            .also(|b| b.extend_from_slice(b"\r\n"))
            .freeze();

        let chunk3 = BytesMut::from(chunk3_meta.as_ref())
            .also(|b| b.extend_from_slice(b"\r\n"))
            .freeze();

        let chunks = vec![
            Ok(chunk1),
            Err(io::Error::new(io::ErrorKind::Interrupted, "test")),
            Ok(chunk2),
            Ok(chunk3),
        ];

        let seed_signature = "4f232c4386841ef735655705268965c44a0e4690baa4adea153f7db9fa80a0a9";
        let timestamp = "20130524T000000Z";
        let region = "us-east-1";
        let secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

        let date = AmzDate::from_header_str(timestamp).unwrap();

        let stream = futures::stream::iter(chunks.into_iter());
        let mut chunked_stream = ChunkedStream::new(
            stream,
            seed_signature.into(),
            date,
            region.into(),
            secret_access_key.into(),
        );

        let ans1 = chunked_stream.next().await.unwrap();
        assert_eq!(ans1.unwrap(), chunk1_data.as_slice());

        let ans2 = chunked_stream.next().await.unwrap();
        assert!(matches!(ans2.unwrap_err(), ChunkedStreamError::Io(_)));

        let ans3 = chunked_stream.next().await.unwrap();
        assert_eq!(ans3.unwrap(), chunk2_data.as_slice());

        assert!(chunked_stream.next().await.is_none());
        assert!(chunked_stream.next().await.is_none());
        assert!(chunked_stream.next().await.is_none());
    }
}
