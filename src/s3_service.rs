//! Generic S3 service which wraps a S3 storage

use crate::error::{InvalidRequestError, S3Error, S3Result};
use crate::s3_output::S3Output;
use crate::s3_path::S3Path;
use crate::s3_storage::S3Storage;
use crate::utils::{Request, Response};

use futures::future::BoxFuture;
use futures::stream::StreamExt as _;
use hyper::Method;
use log::debug;
use std::io;
use std::sync::Arc;
use std::task::{Context, Poll};

use rusoto_s3::{
    CreateBucketRequest, DeleteBucketRequest, DeleteObjectRequest, GetObjectRequest,
    HeadBucketRequest, PutObjectRequest,
};

/// Generic S3 service which wraps a S3 storage
#[derive(Debug)]
pub struct S3Service<T> {
    /// inner storage
    inner: Arc<T>,
}

impl<T> S3Service<T> {
    /// Constructs a S3 service
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

    type Error = S3Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(())) // TODO: back pressue
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let service = self.clone();
        Box::pin(async move { service.hyper_call(req).await })
    }
}

/// helper function for parsing request path
fn parse_path(req: &Request) -> S3Result<S3Path<'_>> {
    match S3Path::try_from_path(req.uri().path()) {
        Ok(r) => Ok(r),
        Err(e) => Err(S3Error::InvalidRequest(InvalidRequestError::ParsePath(e))),
    }
}

#[allow(unused_variables)] // TODO: remove it
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
            Err(err) => debug!("{} \"{:?}\" => error:\n{:#?}", method, uri, err),
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
            S3Path::Root => {
                // list buckets
                self.inner.list_buckets().await.try_into_response()
            }
            S3Path::Bucket { bucket } => Err(S3Error::NotSupported), // TODO: impl handler
            S3Path::Object { bucket, key } => {
                let input = GetObjectRequest {
                    bucket: bucket.into(),
                    key: key.into(),
                    ..GetObjectRequest::default() // TODO: handle other fields
                };
                self.inner.get_object(input).await.try_into_response()
            }
        }
    }

    /// handle POST request
    async fn handle_post(&self, req: Request) -> S3Result<Response> {
        let path = parse_path(&req)?;
        match path {
            S3Path::Root => Err(S3Error::NotSupported), // TODO: impl handler
            S3Path::Bucket { bucket } => Err(S3Error::NotSupported), // TODO: impl handler
            S3Path::Object { bucket, key } => Err(S3Error::NotSupported), // TODO: impl handler
        }
    }

    /// handle PUT request
    async fn handle_put(&self, req: Request) -> S3Result<Response> {
        let path = parse_path(&req)?;
        match path {
            S3Path::Root => Err(S3Error::NotSupported), // TODO: impl handler
            S3Path::Bucket { bucket } => {
                let input: CreateBucketRequest = CreateBucketRequest {
                    bucket: bucket.into(),
                    ..CreateBucketRequest::default() // TODO: handle other fields
                };
                self.inner.create_bucket(input).await.try_into_response()
            }
            S3Path::Object { bucket, key } => {
                let bucket = bucket.into();
                let key = key.into();
                let body = req.into_body().map(|try_chunk| {
                    try_chunk.map(|c| c).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::Other,
                            format!("Error obtaining chunk: {}", e),
                        )
                    })
                });

                let input: PutObjectRequest = PutObjectRequest {
                    bucket,
                    key,
                    body: Some(rusoto_core::ByteStream::new(body)),
                    ..PutObjectRequest::default() // TODO: handle other fields
                };

                self.inner.put_object(input).await.try_into_response()
            }
        }
    }

    /// handle DELETE request
    async fn handle_delete(&self, req: Request) -> S3Result<Response> {
        let path = parse_path(&req)?;
        match path {
            S3Path::Root => Err(S3Error::NotSupported), // TODO: impl handler
            S3Path::Bucket { bucket } => {
                let input: DeleteBucketRequest = DeleteBucketRequest {
                    bucket: bucket.into(),
                };
                self.inner.delete_bucket(input).await.try_into_response()
            }
            S3Path::Object { bucket, key } => {
                let input: DeleteObjectRequest = DeleteObjectRequest {
                    bucket: bucket.into(),
                    key: key.into(),
                    ..DeleteObjectRequest::default() // TODO: handle other fields
                };

                self.inner.delete_object(input).await.try_into_response()
            }
        }
    }

    /// handle HEAD request
    async fn handle_head(&self, req: Request) -> S3Result<Response> {
        let path = parse_path(&req)?;
        match path {
            S3Path::Root => Err(S3Error::NotSupported), // TODO: impl handler
            S3Path::Bucket { bucket } => {
                // head bucket
                let input = HeadBucketRequest {
                    bucket: bucket.into(),
                };
                self.inner.head_bucket(input).await.try_into_response()
            }
            S3Path::Object { bucket, key } => Err(S3Error::NotSupported), // TODO: impl handler
        }
    }
}
