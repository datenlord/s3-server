#[cfg(feature = "rt-async-std")]
pub use with_async_std::{AsyncStdExecutor, AsyncStdListener};

#[cfg(feature = "rt-async-std")]
mod with_async_std {
    use async_std::net::{TcpListener, TcpStream};
    use async_std::task;
    use futures::io::{AsyncRead, AsyncWrite};
    use futures::stream::Stream;
    use std::future::Future;
    use std::io;
    use std::net::Shutdown;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[derive(Debug, Clone, Copy)]
    pub struct AsyncStdExecutor;

    impl<F> hyper::rt::Executor<F> for AsyncStdExecutor
    where
        F: Future + Send + 'static,
    {
        fn execute(&self, fut: F) {
            let _ = task::spawn(async { drop(fut.await) });
        }
    }

    #[derive(Debug)]
    pub struct AsyncStdListener {
        listener: TcpListener,
    }

    impl AsyncStdListener {
        pub const fn new(listener: TcpListener) -> Self {
            Self { listener }
        }
    }

    #[derive(Debug)]
    pub struct AsyncStdStream {
        stream: TcpStream,
    }

    impl hyper::server::accept::Accept for AsyncStdListener {
        type Conn = AsyncStdStream;
        type Error = io::Error;

        fn poll_accept(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
            let incoming = self.listener.incoming();
            futures::pin_mut!(incoming);

            match futures::ready!(incoming.poll_next(cx)) {
                None => Poll::Ready(None),
                Some(Ok(stream)) => Poll::Ready(Some(Ok(AsyncStdStream { stream }))),
                Some(Err(e)) => Poll::Ready(Some(Err(e))),
            }
        }
    }

    impl hyper::client::connect::Connection for AsyncStdStream {
        fn connected(&self) -> hyper::client::connect::Connected {
            hyper::client::connect::Connected::new()
        }
    }

    impl tokio::io::AsyncRead for AsyncStdStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            let stream = Pin::new(&mut self.stream);
            stream.poll_read(cx, buf)
        }
    }

    impl tokio::io::AsyncWrite for AsyncStdStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, io::Error>> {
            let stream = Pin::new(&mut self.stream);
            stream.poll_write(cx, buf)
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), io::Error>> {
            let stream = Pin::new(&mut self.stream);
            stream.poll_flush(cx)
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), io::Error>> {
            self.stream.shutdown(Shutdown::Write)?;
            Poll::Ready(Ok(()))
        }
    }
}
