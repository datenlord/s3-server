use crate::error::S3Error;
use crate::s3_storage::S3Storage;
use crate::utils::{BoxStdError, Request, Response, StdResult};

use futures::future::BoxFuture;
use hyper::Method;
use std::sync::Arc;
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct S3Service<T> {
    inner: Arc<T>,
}

impl<T> S3Service<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl<T> Clone for S3Service<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> AsRef<T> for S3Service<T> {
    fn as_ref(&self) -> &T {
        &*self.inner
    }
}

impl<T> hyper::service::Service<Request> for S3Service<T>
where
    T: S3Storage + Send + Sync + 'static,
{
    type Response = Response;

    type Error = BoxStdError;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(())) // TODO: back pressue
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let service = self.clone();
        Box::pin(async move { service.hyper_call(req).await })
    }
}

impl<T> S3Service<T>
where
    T: S3Storage + Send + Sync + 'static,
{
    pub async fn hyper_call(&self, req: Request) -> StdResult<Response> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        log::debug!("{} \"{:?}\" request:\n{:#?}", method, uri, req);
        let result = self.handle(req).await;
        match &result {
            Ok(resp) => log::debug!("{} \"{:?}\" => response:\n{:#?}", method, uri, resp),
            Err(err) => log::debug!("{} \"{:?}\" => error:\n{:#?}", method, uri, err),
        }
        result
    }

    async fn handle(&self, req: Request) -> StdResult<Response> {
        match *req.method() {
            Method::GET => self.handle_get(req).await,
            Method::POST => self.handle_post(req).await,
            Method::PUT => self.handle_put(req).await,
            Method::DELETE => self.handle_delete(req).await,
            Method::HEAD => self.handle_head(req).await,
            _ => Err(<S3Error>::NotSupported.into()),
        }
    }

    async fn handle_get(&self, _req: Request) -> StdResult<Response> {
        todo!()
    }
    async fn handle_post(&self, _req: Request) -> StdResult<Response> {
        todo!()
    }
    async fn handle_put(&self, _req: Request) -> StdResult<Response> {
        todo!()
    }
    async fn handle_delete(&self, _req: Request) -> StdResult<Response> {
        todo!()
    }
    async fn handle_head(&self, _req: Request) -> StdResult<Response> {
        todo!()
    }
}
