//! multipart/form-data encoding for POST Object
//!
//! See <https://docs.aws.amazon.com/AmazonS3/latest/API/RESTObjectPOST.html>
//!

#![allow(dead_code)] // TODO: remove this

use crate::utils::{async_stream::AsyncTryStream, Also, Apply};

use std::{io, mem, pin::Pin, str::FromStr};

use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use memchr::memchr_iter;

/// form file

#[derive(Debug)]
pub struct File {
    /// name
    pub name: String,
    /// content type
    pub content_type: String,
    /// stream
    pub stream: FileStream,
}

/// multipart/form-data for POST Object
#[derive(Debug)]
pub struct Multipart {
    /// fields
    pub fields: Vec<(String, String)>,
    /// file
    pub file: File,
}

impl Multipart {
    /// find field value
    pub fn find_field_value<'a>(&'a self, name: &str) -> Option<&'a str> {
        self.fields.iter().rev().find_map(|(n, v)| {
            if n.eq_ignore_ascii_case(name) {
                Some(v.as_str())
            } else {
                None
            }
        })
    }

    /// assign from optional field
    pub fn assign_from_optional_field<T>(
        &self,
        name: &str,
        opt: &mut Option<T>,
    ) -> Result<(), T::Err>
    where
        T: FromStr,
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        if let Some(s) = self.find_field_value(name) {
            let v = s.parse()?;
            *opt = Some(v);
        }
        Ok(())
    }
}

/// transform multipart
pub async fn transform_multipart<S>(body_stream: S, boundary: &'_ [u8]) -> io::Result<Multipart>
where
    S: Stream<Item = io::Result<Bytes>> + Send + 'static,
{
    let mut buf = Vec::new();

    let mut body = Box::pin(body_stream);

    let mut pat: Box<[u8]> = Vec::with_capacity(boundary.len().saturating_add(4))
        .also(|v| v.extend_from_slice(b"--"))
        .also(|v| v.extend_from_slice(boundary))
        .also(|v| v.extend_from_slice(b"\r\n"))
        .into();

    let generate_format_error = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "multipart/form-data format error",
        )
    };

    let mut fields = Vec::new();

    loop {
        // copy bytes to buf
        match body.as_mut().next().await {
            None => return Err(generate_format_error()),
            Some(Err(e)) => return Err(e),
            Some(Ok(bytes)) => buf.extend_from_slice(&bytes),
        };

        // try to parse
        match try_parse(body, pat, &buf, &mut fields).await {
            Err((b, p)) => {
                body = b;
                pat = p;
            }
            Ok(ans) => return ans,
        }
    }
}

/// try to parse data buffer, pat: b"--{boundary}\r\n"
async fn try_parse<S>(
    body: Pin<Box<S>>,
    pat: Box<[u8]>,
    buf: &'_ [u8],
    fields: &'_ mut Vec<(String, String)>,
) -> Result<io::Result<Multipart>, (Pin<Box<S>>, Box<[u8]>)>
where
    S: Stream<Item = io::Result<Bytes>> + Send + 'static,
{
    let generate_format_error = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "multipart/form-data format error",
        )
    };

    let pat_without_crlf = pat
        .get(..pat.len().wrapping_sub(2))
        .unwrap_or_else(|| unreachable!());

    fields.clear();

    let mut lines = CrlfLines { slice: buf };

    // first line
    match lines.next_line() {
        None => return Err((body, pat)),
        Some([]) => {}
        Some(_) => return Ok(Err(generate_format_error())),
    };

    // first boundary
    match lines.next_line() {
        None => return Err((body, pat)),
        Some(line) => {
            if line != pat_without_crlf {
                return Ok(Err(generate_format_error()));
            };
        }
    }

    let mut headers = [httparse::EMPTY_HEADER; 2];
    loop {
        let (idx, parsed_headers) = match httparse::parse_headers(lines.slice, &mut headers) {
            Ok(httparse::Status::Complete(ans)) => ans,
            Ok(_) => return Err((body, pat)),
            Err(_) => return Ok(Err(generate_format_error())),
        };
        lines.slice = lines.slice.split_at(idx).1;

        let mut content_disposition_bytes = None;
        let mut content_type_bytes = None;
        for header in parsed_headers {
            if header.name.eq_ignore_ascii_case("Content-Disposition") {
                content_disposition_bytes = Some(header.value)
            } else if header.name.eq_ignore_ascii_case("Content-Type") {
                content_type_bytes = Some(header.value)
            } else {
                continue;
            }
        }
        let content_disposition = match content_disposition_bytes.map(parse_content_disposition) {
            None => return Err((body, pat)),
            Some(Err(_)) => return Ok(Err(generate_format_error())),
            Some(Ok((_, c))) => c,
        };
        match content_disposition.filename {
            None => {
                let value = match lines.split_to(pat_without_crlf) {
                    None => return Err((body, pat)),
                    Some(b) => {
                        match std::str::from_utf8(
                            b.get(..b.len().saturating_sub(2))
                                .unwrap_or_else(|| unreachable!()),
                        ) {
                            Err(_) => return Ok(Err(generate_format_error())),
                            Ok(s) => s,
                        }
                    }
                };
                fields.push((content_disposition.name.to_owned(), value.to_owned()));
            }
            Some(filename) => {
                let content_type = match content_type_bytes.map(std::str::from_utf8) {
                    None => return Err((body, pat)),
                    Some(Err(_)) => return Ok(Err(generate_format_error())),
                    Some(Ok(s)) => s,
                };
                let remaining_bytes = Bytes::copy_from_slice(lines.slice);
                let file_stream = FileStream::new(body, pat, remaining_bytes);
                let file = File {
                    name: filename.to_owned(),
                    content_type: content_type.to_owned(),
                    stream: file_stream,
                };

                return Ok(Ok(Multipart {
                    fields: fields.drain(..).collect(),
                    file,
                }));
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
/// File stream error
pub enum FileStreamError {
    /// Incomplete error
    #[error("FileStreamError: Incomplete")]
    Incomplete,
    /// IO error
    #[error("FileStreamError: IO: {}",.0)]
    Io(io::Error),
}

/// File stream
#[derive(Debug)]
pub struct FileStream {
    /// inner stream
    inner: AsyncTryStream<Bytes, FileStreamError>,
}

impl FileStream {
    /// Constructs a `FileStream`
    fn new<S>(mut body: Pin<Box<S>>, pat: Box<[u8]>, prev_bytes: Bytes) -> Self
    where
        S: Stream<Item = io::Result<Bytes>> + Send + 'static,
    {
        <AsyncTryStream<Bytes, FileStreamError>>::new(|mut y| {
            Box::pin(async move {
                let crlf_pat: Box<[u8]> = Vec::new()
                    .also(|v| v.extend_from_slice(b"\r\n"))
                    .also(|v| v.extend_from_slice(&pat))
                    .into();
                drop(pat);

                let mut pat_idx = 0;
                let mut push_bytes = |mut bytes: Bytes| {
                    if pat_idx > 0 {
                        let suffix = crlf_pat.get(pat_idx..).unwrap_or_else(|| unreachable!());
                        if bytes.starts_with(suffix) {
                            return None;
                        } else {
                            pat_idx = 0;
                        }
                    }
                    for idx in memchr_iter(b'\r', bytes.as_ref()) {
                        let remaining = bytes.get(idx..).unwrap_or_else(|| unreachable!());

                        if remaining.len() >= crlf_pat.len() {
                            if remaining.starts_with(&crlf_pat) {
                                bytes.truncate(idx);
                                y.yield_one(bytes);
                                return None;
                            } else {
                                continue;
                            }
                        } else if crlf_pat.starts_with(remaining) {
                            pat_idx = remaining.len();
                        } else {
                            continue;
                        }
                    }
                    y.yield_one(bytes);
                    Some(())
                };

                if push_bytes(prev_bytes).is_some() {
                    loop {
                        let bytes = match body.as_mut().next().await {
                            None => return Err(FileStreamError::Incomplete),
                            Some(Err(e)) => return Err(FileStreamError::Io(e)),
                            Some(Ok(b)) => b,
                        };
                        if push_bytes(bytes).is_none() {
                            break;
                        }
                    }
                }
                while let Some(ret) = body.as_mut().next().await {
                    if let Err(e) = ret {
                        return Err(FileStreamError::Io(e));
                    }
                }
                Ok(())
            })
        })
        .apply(|inner| Self { inner })
    }
}

impl Stream for FileStream {
    type Item = Result<Bytes, FileStreamError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

/// CRLF lines
struct CrlfLines<'a> {
    /// slice
    slice: &'a [u8],
}

impl<'a> CrlfLines<'a> {
    /// poll next line
    fn next_line(&mut self) -> Option<&'a [u8]> {
        for idx in memchr_iter(b'\n', self.slice) {
            if idx == 0 {
                continue;
            }

            let byte = *self
                .slice
                .get(idx.wrapping_sub(1))
                .unwrap_or_else(|| unreachable!());

            if byte == b'\r' {
                let left = self
                    .slice
                    .get(..idx.wrapping_sub(1))
                    .unwrap_or_else(|| unreachable!());
                let right = self
                    .slice
                    .get(idx.wrapping_add(1)..)
                    .unwrap_or_else(|| unreachable!());

                self.slice = right;
                return Some(left);
            }
        }

        Some(mem::replace(&mut self.slice, &[]))
    }

    /// split by pattern and return previous bytes
    fn split_to(&mut self, line_pat: &'_ [u8]) -> Option<&'a [u8]> {
        let mut len: usize = 0;
        let mut lines = Self { slice: self.slice };
        loop {
            let line = lines.next_line()?;
            if line == line_pat {
                len = len.min(self.slice.len());
                let ans = self.slice.get(..len).unwrap_or_else(|| unreachable!());
                self.slice = lines.slice;
                return Some(ans);
            } else {
                len = len.wrapping_add(line.len()).saturating_add(2);
            }
        }
    }
}

/// Content-Disposition
#[derive(Debug)]
struct ContentDisposition<'a> {
    /// name
    name: &'a str,
    /// filename
    filename: Option<&'a str>,
}

/// parse content disposition value
fn parse_content_disposition(input: &[u8]) -> nom::IResult<&[u8], ContentDisposition<'_>> {
    use nom::{
        bytes::complete::{tag, take, take_till1},
        combinator::{all_consuming, map_res, opt},
        sequence::{delimited, preceded, tuple},
    };

    let name_parser = delimited(
        tag(b"name=\""),
        map_res(take_till1(|c| c == b'"'), std::str::from_utf8),
        take(1_usize),
    );

    let filename_parser = delimited(
        tag(b"filename=\""),
        map_res(take_till1(|c| c == b'"'), std::str::from_utf8),
        take(1_usize),
    );

    let parser = all_consuming(tuple((
        preceded(tag(b"form-data; "), name_parser),
        opt(preceded(tag(b"; "), filename_parser)),
    )));

    let (remaining, (name, filename)) = parser(input)?;

    Ok((remaining, ContentDisposition { name, filename }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::TryStreamExt;

    #[test]
    fn content_disposition() {
        {
            let text = b"form-data; name=\"Signature\"";
            let (_, ans) = parse_content_disposition(text).unwrap();
            assert_eq!(ans.name, "Signature");
            assert_eq!(ans.filename, None);
        }
        {
            let text = b"form-data; name=\"file\"; filename=\"MyFilename.jpg\"";
            let (_, ans) = parse_content_disposition(text).unwrap();
            assert_eq!(ans.name, "file");
            assert_eq!(ans.filename, Some("MyFilename.jpg"));
        }
    }

    #[test]
    fn split_to() {
        let bytes = b"\r\n----\r\nasd\r\nqwe";
        let mut lines = CrlfLines { slice: bytes };
        assert_eq!(lines.split_to(b"----"), Some(b"\r\n".as_ref()));
        assert_eq!(lines.slice, b"asd\r\nqwe");
    }

    #[tokio::test]
    async fn multipart() {
        let fields = [
            ("key","acl"),
            ("tagging","<Tagging><TagSet><Tag><Key>Tag Name</Key><Value>Tag Value</Value></Tag></TagSet></Tagging>"),
            ("success_action_redirect","success_redirect"),
            ("Content-Type","content_type"),
            ("x-amz-meta-uuid","uuid"),
            ("x-amz-meta-tag","metadata"),
            ("AWSAccessKeyId","access-key-id"),
            ("Policy","encoded_policy"),
            ("Signature","signature="),
        ];

        let other_fields = [("submit", "Upload to Amazon S3")];

        let filename = "MyFilename.jpg";
        let content_type = "image/jpg";
        let boundary = "9431149156168";
        let file_content = "file_content";

        let body_bytes = {
            let mut s = Vec::new();
            s.push(format!("\r\n--{}\r\n", boundary));
            for &(n, v) in &fields {
                s.push(format!(
                    concat!(
                        "Content-Disposition: form-data; name=\"{}\"\r\n",
                        "\r\n",
                        "{}\r\n",
                        "--{}\r\n",
                    ),
                    n, v, boundary
                ));
            }
            s.push(format!(
                concat!(
                    "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                    "Content-Type: {}\r\n",
                    "\r\n",
                    "{}\r\n",
                    "--{}\r\n",
                ),
                "file", filename, content_type, file_content, boundary
            ));
            s.push(format!(
                concat!(
                    "Content-Disposition: form-data; name=\"{}\"\r\n",
                    "\r\n",
                    "{}\r\n",
                    "--{}--\r\n",
                ),
                other_fields[0].0, other_fields[0].1, boundary
            ));

            s.into_iter()
                .map(|s| s.into_bytes().apply(Bytes::from).apply(Ok))
                .collect::<Vec<io::Result<Bytes>>>()
        };

        let body_stream = futures::stream::iter(body_bytes);

        let ans = transform_multipart(body_stream, boundary.as_bytes())
            .await
            .unwrap();

        for (lhs, rhs) in ans.fields.iter().zip(fields.iter()) {
            assert_eq!(lhs.0, rhs.0);
            assert_eq!(lhs.1, rhs.1);
        }

        assert_eq!(ans.file.name, filename);
        assert_eq!(ans.file.content_type, content_type);

        let file_bytes = ans
            .file
            .stream
            .try_fold(Vec::new(), |buf, bytes| async move {
                buf.also(|v| v.extend_from_slice(&bytes)).apply(Ok)
            })
            .await
            .unwrap()
            .apply(String::from_utf8)
            .unwrap();

        assert_eq!(file_bytes, file_content);
    }
}
