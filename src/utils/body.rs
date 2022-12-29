//! body util

use crate::dto::ByteStream;
use crate::streams::multipart::{FileStream, FileStreamError};
use crate::utils::Apply;
use crate::{Body, BoxStdError};

use std::io;

use futures::stream::StreamExt;
use serde::de::DeserializeOwned;

/// deserialize xml body
pub async fn deserialize_xml_body<T: DeserializeOwned>(body: Body) -> Result<T, BoxStdError> {
    let bytes = hyper::body::to_bytes(body).await?;
    let ans: T = quick_xml::de::from_reader(&*bytes)?;
    Ok(ans)
}

/// transform `Body` into `ByteStream`
pub fn transform_body_stream(body: Body) -> ByteStream {
    body.map(|try_chunk| {
        try_chunk.map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Error obtaining chunk: {e}"))
        })
    })
    .apply(ByteStream::new)
}

/// transform `FileStream` into `ByteStream`
pub fn transform_file_stream(file_stream: FileStream) -> ByteStream {
    file_stream
        .map(|try_chunk| {
            try_chunk.map_err(|e| match e {
                FileStreamError::Incomplete => {
                    io::Error::new(io::ErrorKind::Other, format!("Error obtaining chunk: {e}"))
                }
                FileStreamError::Io(e) => e,
            })
        })
        .apply(ByteStream::new)
}
