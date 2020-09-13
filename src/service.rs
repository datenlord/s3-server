//! Generic S3 service which wraps a S3 storage

use crate::{
    error::{S3Error, S3Result},
    header::names::X_AMZ_COPY_SOURCE,
    ops,
    output::S3Output,
    path::S3Path,
    query::GetQuery,
    query::PostQuery,
    storage::S3Storage,
    utils::{Apply, RequestExt},
    BoxStdError, Request, Response,
};

use std::{
    future::Future,
    ops::Deref,
    sync::Arc,
    task::{Context, Poll},
};

use futures::future::BoxFuture;
use hyper::{header::AsHeaderName, Method};
use log::{debug, error};
use serde::de::DeserializeOwned;

/// Generic S3 service which wraps a S3 storage
#[derive(Debug)]
pub struct S3Service<T> {
    /// inner storage
    storage: T,
}

/// Shared S3 service
#[derive(Debug)]
pub struct SharedS3Service<T> {
    /// inner service
    inner: Arc<S3Service<T>>,
}

impl<T> S3Service<T> {
    /// Constructs a S3 service
    pub const fn new(storage: T) -> Self {
        Self { storage }
    }

    /// convert `S3Service<T>` to `SharedS3Service<T>`
    pub fn into_shared(self) -> SharedS3Service<T> {
        SharedS3Service {
            inner: Arc::new(self),
        }
    }
}

impl<T> AsRef<T> for S3Service<T> {
    fn as_ref(&self) -> &T {
        &self.storage
    }
}

impl<T> Deref for SharedS3Service<T> {
    type Target = S3Service<T>;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<T> Clone for SharedS3Service<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> hyper::service::Service<Request> for SharedS3Service<T>
where
    T: S3Storage + Send + Sync + 'static,
{
    type Response = Response;

    type Error = S3Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(())) // FIXME: back pressue
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let service = self.clone();
        Box::pin(async move { service.hyper_call(req).await })
    }
}

/// wrap handle
async fn wrap_handle<T>(f: impl Future<Output = Result<T, BoxStdError>> + Send) -> S3Result<T> {
    f.await.map_err(|e| S3Error::InvalidRequest(e))
}

/// wrap handle sync
fn wrap_handle_sync<T>(f: impl FnOnce() -> Result<T, BoxStdError>) -> S3Result<T> {
    f().map_err(|e| S3Error::InvalidRequest(e))
}

macro_rules! op_call{
    ($op:ident with () by $storage:expr)  => {{
        let input = wrap_handle_sync(ops::$op::extract)?;
        $storage.$op(input).await.try_into_response()
    }};

    ($op:ident with ($($arg:expr),+) by $storage:expr)  => {{
        let input = wrap_handle_sync(||ops::$op::extract($($arg),+))?;
        $storage.$op(input).await.try_into_response()
    }};

    ($op:ident with async ($($arg:expr),*) by $storage:expr)  => {{
        let input = wrap_handle(ops::$op::extract($($arg),*)).await?;
        $storage.$op(input).await.try_into_response()
    }};
}

/// helper function for parsing request path
fn parse_path(req: &Request) -> S3Result<S3Path<'_>> {
    match S3Path::try_from_path(req.uri().path()) {
        Ok(r) => Ok(r),
        Err(e) => Err(S3Error::InvalidRequest(e.into())),
    }
}

/// helper function for extracting url query
fn extract_query<Q: DeserializeOwned>(req: &Request) -> S3Result<Option<Q>> {
    match req.uri().query() {
        Some(s) => serde_urlencoded::from_str::<Q>(s)
            .map_err(|e| S3Error::InvalidRequest(e.into()))?
            .apply(Some),
        None => None,
    }
    .apply(Ok)
}

/// extract header
fn extract_header(req: &Request, name: impl AsHeaderName) -> S3Result<Option<&str>> {
    match req.get_header_str(name) {
        Ok(s) => s.apply(Ok),
        Err(e) => S3Error::InvalidRequest(e.into()).apply(Err),
    }
}

impl<T> S3Service<T>
where
    T: S3Storage + Send + Sync + 'static,
{
    /// Call the s3 service with `hyper::Request<hyper::Body>`
    /// # Errors
    /// Returns an `Err` if the service failed
    pub async fn hyper_call(&self, req: Request) -> S3Result<Response> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        debug!("{} \"{:?}\" request:\n{:#?}", method, uri, req);
        let result = self.handle(req).await;
        match &result {
            Ok(resp) => debug!("{} \"{:?}\" => response:\n{:#?}", method, uri, resp),
            Err(err) => error!("{} \"{:?}\" => error:\n{:#?}", method, uri, err),
        }
        result
    }

    /// handle request
    async fn handle(&self, req: Request) -> S3Result<Response> {
        let resp = match *req.method() {
            Method::GET => self.handle_get(req).await?,
            Method::POST => self.handle_post(req).await?,
            Method::PUT => self.handle_put(req).await?,
            Method::DELETE => self.handle_delete(req).await?,
            Method::HEAD => self.handle_head(req).await?,
            _ => return Err(S3Error::NotSupported),
        };
        Ok(resp)
    }

    /// handle GET request
    async fn handle_get(&self, req: Request) -> S3Result<Response> {
        let path = parse_path(&req)?;
        match path {
            S3Path::Root => op_call!(list_buckets with () by self.storage),
            S3Path::Bucket { bucket } => {
                let query = match extract_query::<GetQuery>(&req)? {
                    None => return op_call!(list_objects with (&req, None, bucket) by self.storage),
                    Some(query) => query,
                };

                if query.location.is_some() {
                    return op_call!(get_bucket_location with (bucket) by self.storage);
                }

                match query.list_type {
                    None => op_call!(list_objects with (&req,Some(query),bucket) by self.storage),
                    Some(2) => op_call!(list_objects_v2 with (&req,query,bucket) by self.storage),
                    Some(_) => Err(S3Error::NotSupported),
                }
            }
            S3Path::Object { bucket, key } => {
                op_call!(get_object with (&req, bucket, key) by self.storage)
            }
        }
    }

    /// handle POST request
    async fn handle_post(&self, mut req: Request) -> S3Result<Response> {
        let body = req.take_body();
        let path = parse_path(&req)?;
        match path {
            S3Path::Root => Err(S3Error::NotSupported), // TODO: impl handler
            S3Path::Bucket { bucket } => {
                let query = match extract_query::<PostQuery>(&req)? {
                    None => return Err(S3Error::NotSupported),
                    Some(query) => query,
                };

                if query.delete.is_some() {
                    return op_call!(delete_objects with async (&req, body, bucket) by self.storage);
                }

                // TODO: impl handler

                Err(S3Error::NotSupported)
            }
            S3Path::Object { bucket, key } => {
                dbg!((bucket, key)); // TODO: remove this place holder
                Err(S3Error::NotSupported) // TODO: impl handler
            }
        }
    }

    /// handle PUT request
    async fn handle_put(&self, mut req: Request) -> S3Result<Response> {
        let body = req.take_body();
        let path = parse_path(&req)?;
        match path {
            S3Path::Root => Err(S3Error::NotSupported), // TODO: impl handler
            S3Path::Bucket { bucket } => {
                op_call!(create_bucket with async (&req, body, bucket) by self.storage)
            }
            S3Path::Object { bucket, key } => {
                if let Some(copy_source) = extract_header(&req, &*X_AMZ_COPY_SOURCE)? {
                    return op_call!(copy_object with (&req, bucket,key,copy_source) by self.storage);
                }
                op_call!(put_object with (&req,body,bucket,key) by self.storage)
            }
        }
    }

    /// handle DELETE request
    async fn handle_delete(&self, req: Request) -> S3Result<Response> {
        let path = parse_path(&req)?;
        match path {
            S3Path::Root => Err(S3Error::NotSupported), // TODO: impl handler
            S3Path::Bucket { bucket } => op_call!(delete_bucket with (bucket) by self.storage),
            S3Path::Object { bucket, key } => {
                op_call!(delete_object with (&req, bucket,key) by self.storage)
            }
        }
    }

    /// handle HEAD request
    async fn handle_head(&self, req: Request) -> S3Result<Response> {
        let path = parse_path(&req)?;
        match path {
            S3Path::Root => Err(S3Error::NotSupported), // TODO: impl handler
            S3Path::Bucket { bucket } => op_call!(head_bucket with (bucket) by self.storage),
            S3Path::Object { bucket, key } => {
                op_call!(head_object with (&req, bucket, key) by self.storage)
            }
        }
    }
}
