//! async stream

use std::collections::VecDeque;
use std::fmt::{self, Debug};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use atomic_refcell::AtomicRefCell;
use futures::future::BoxFuture;
use futures::stream::Stream;

/// async try stream
pub struct AsyncTryStream<T, E, G = BoxFuture<'static, Result<(), E>>> {
    /// generator
    fut: Option<G>,
    /// queue
    q: Arc<AtomicRefCell<VecDeque<Result<T, E>>>>,
}

/// yielder
pub struct Yielder<T, E> {
    /// queue
    q: Arc<AtomicRefCell<VecDeque<Result<T, E>>>>,
}

impl<T, E> Yielder<T, E> {
    /// yield multiple values
    pub fn yield_iter(&mut self, iter: impl Iterator<Item = T>) {
        self.q.borrow_mut().extend(iter.map(Ok))
    }

    /// yield one value
    pub fn yield_one(&mut self, item: T) {
        self.q.borrow_mut().push_back(Ok(item));
    }
}

impl<T, E, G> AsyncTryStream<T, E, G>
where
    G: Future<Output = Result<(), E>>,
{
    /// Constructs a new `AsyncTryStream`
    ///
    /// NOTE: DO NOT send the yielder out of the future's scope.
    pub fn new(f: impl FnOnce(Yielder<T, E>) -> G) -> Self {
        let rx = Arc::new(AtomicRefCell::new(VecDeque::new()));
        let tx = Arc::clone(&rx);
        let yielder = Yielder { q: tx };
        let fut = Some(f(yielder));
        Self { q: rx, fut }
    }
}

impl<T, E, G> Stream for AsyncTryStream<T, E, G>
where
    G: Future<Output = Result<(), E>> + Unpin,
{
    type Item = Result<T, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some(item) = self.q.borrow_mut().pop_front() {
                return Poll::Ready(Some(item));
            }

            let fut = match self.fut.as_mut() {
                None => return Poll::Ready(None),
                Some(g) => Pin::new(g),
            };

            match fut.poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(ret) => {
                    self.fut = None;
                    if let Err(e) = ret {
                        self.q.borrow_mut().push_back(Err(e));
                    }
                }
            }
        }
    }
}

impl<T, E, G> Debug for AsyncTryStream<T, E, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AsyncTryStream{{...}}")
    }
}
