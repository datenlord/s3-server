//! `BytesStream`

use std::pin::Pin;
use std::task::{Context, Poll};
use std::{io, mem};

use futures::stream::Stream;
use hyper::body::Bytes;
use pin_project_lite::pin_project;

pin_project! {
    pub struct BytesStream<R>{
        #[pin]
        reader: R,
        buf_size: usize,
        buf: Vec<u8>,
        limit: Option<usize>,
    }
}

impl<R> BytesStream<R> {
    /// Constructs a `BytesStream`
    pub const fn new(reader: R, buf_size: usize, limit: Option<usize>) -> Self {
        Self {
            reader,
            buf_size,
            buf: Vec::new(),
            limit,
        }
    }
}

impl<R: futures::AsyncRead> Stream for BytesStream<R> {
    type Item = io::Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        let buf_len = match *this.limit {
            Some(lim) => lim.min(*this.buf_size),
            None => *this.buf_size,
        };
        this.buf.resize(buf_len, 0);

        let ret: io::Result<usize> = futures::ready!(this.reader.poll_read(cx, this.buf));
        let ans: Option<io::Result<Bytes>> = match ret {
            Ok(n) if n == 0 => None,
            Ok(n) => {
                let nread = n.min(buf_len);
                this.buf.truncate(nread);
                let buf = Bytes::from(mem::take(this.buf));

                if let Some(ref mut lim) = *this.limit {
                    *lim = lim.wrapping_sub(nread);
                }

                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        };

        Poll::Ready(ans)
    }
}
