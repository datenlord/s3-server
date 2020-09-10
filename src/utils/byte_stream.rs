//! `ByteStream`

use bytes::Bytes;
use futures::stream::Stream;
use pin_project_lite::pin_project;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncRead;

pin_project! {
    pub struct ByteStream<R>{
        #[pin]
        reader: R,
        buf_size: usize,
    }
}

impl<R> ByteStream<R> {
    /// Constructs a `ByteStream`
    pub const fn new(reader: R, buf_size: usize) -> Self {
        Self { reader, buf_size }
    }
}

impl<R: AsyncRead> Stream for ByteStream<R> {
    type Item = io::Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // FIXME: reuse the buf
        let mut buf = vec![0_u8; self.buf_size];

        let this = self.project();

        let ret: io::Result<usize> = futures::ready!(this.reader.poll_read(cx, &mut buf));
        let ans: Option<io::Result<Bytes>> = match ret {
            Ok(n) => {
                if n == 0 {
                    None
                } else {
                    buf.truncate(n);
                    Some(Ok(buf.into()))
                }
            }
            Err(e) => Some(Err(e)),
        };

        Poll::Ready(ans)
    }
}
