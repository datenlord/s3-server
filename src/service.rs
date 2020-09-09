//! Generic S3 service which wraps a S3 storage

#![allow(clippy::wildcard_imports)]

use crate::dto::*;
use crate::header::names::*;
use hyper::header::*;

use crate::path::S3Path;
use crate::query::GetQuery;
use crate::storage::S3Storage;
use crate::utils::Apply;
use crate::utils::RequestExt;
use crate::{
    dto,
    error::{S3Error, S3Result},
    BoxStdError,
};
use crate::{output::S3Output, query::PostQuery};
use crate::{Request, Response};

use std::{
    future::Future,
    io,
    ops::Deref,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{future::BoxFuture, stream::StreamExt};
use log::{debug, error};
use serde::de::DeserializeOwned;

use hyper::{Body, Method};

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
        Poll::Ready(Ok(())) // TODO: back pressue
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

/// deserialize xml body
async fn deserialize_xml_body<T: DeserializeOwned>(body: Body) -> S3Result<T> {
    wrap_handle(async move {
        let bytes = hyper::body::to_bytes(body).await?;
        let ans: T = quick_xml::de::from_reader(&*bytes)?;
        Ok(ans)
    })
    .await
}

macro_rules! assign_opt{
    (from $src:ident to $dst:ident: fields [$($field: tt,)+])=>{$(
        if $src.$field.is_some(){
            $dst.$field = $src.$field;
        }
    )+};

    (from $src:ident to $dst:ident: headers [$(($name:expr,$f:ident),)+])=>{$(
        if let Some(s) = $src.get_header_str($name)
            .map_err(|e| S3Error::InvalidRequest(e.into()))?
        {
            $dst.$f = Some(s.into());
        }
    )+};
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
            S3Path::Root => self.storage.list_buckets().await.try_into_response(),
            S3Path::Bucket { bucket } => match extract_query::<GetQuery>(&req)? {
                None => {
                    let input = ListObjectsRequest {
                        bucket: bucket.into(),
                        ..ListObjectsRequest::default()
                    };
                    self.storage.list_objects(input).await.try_into_response()
                }
                Some(query) => {
                    dbg!(&query);
                    if query.location.is_some() {
                        let input = GetBucketLocationRequest {
                            bucket: bucket.into(),
                        };
                        return self
                            .storage
                            .get_bucket_location(input)
                            .await
                            .try_into_response();
                    }

                    match query.list_type {
                        None => {
                            let mut input = ListObjectsRequest {
                                bucket: bucket.into(),
                                ..ListObjectsRequest::default()
                            };

                            assign_opt!(from query to input: fields [
                                delimiter,
                                encoding_type,
                                marker,
                                max_keys,
                                prefix,
                            ]);

                            assign_opt!(from req to input: headers [
                                (&*X_AMZ_REQUEST_PAYER, request_payer),
                            ]);

                            self.storage.list_objects(input).await.try_into_response()
                        }
                        Some(2) => {
                            let mut input = ListObjectsV2Request {
                                bucket: bucket.into(),
                                ..ListObjectsV2Request::default()
                            };

                            assign_opt!(from query to input: fields [
                                continuation_token,
                                delimiter,
                                encoding_type,
                                fetch_owner,
                                max_keys,
                                prefix,
                                start_after,
                            ]);

                            assign_opt!(from req to input: headers [
                                (&*X_AMZ_REQUEST_PAYER, request_payer),
                            ]);

                            self.storage
                                .list_objects_v2(input)
                                .await
                                .try_into_response()
                        }
                        Some(_) => Err(S3Error::NotSupported),
                    }
                }
            },
            S3Path::Object { bucket, key } => {
                let input = GetObjectRequest {
                    bucket: bucket.into(),
                    key: key.into(),
                    ..GetObjectRequest::default() // TODO: handle other fields
                };
                self.storage.get_object(input).await.try_into_response()
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
                match extract_query::<PostQuery>(&req)? {
                    Some(query) => {
                        if query.delete.is_some() {
                            let delete: dto::xml::Delete = deserialize_xml_body(body).await?;
                            let mut input: DeleteObjectsRequest = DeleteObjectsRequest {
                                delete: delete.into(),
                                bucket: bucket.into(),
                                ..dto::DeleteObjectsRequest::default()
                            };

                            assign_opt!(from req to input: headers [
                                (&*X_AMZ_MFA, mfa),
                                (&*X_AMZ_REQUEST_PAYER, request_payer),
                            ]);
                            // TODO: handle "x-amz-bypass-governance-retention"
                            self.storage.delete_objects(input).await.try_into_response()
                        } else {
                            Err(S3Error::NotSupported)
                        }
                    }
                    None => Err(S3Error::NotSupported),
                }
            } // TODO: impl handler
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
                self.storage.create_bucket(input).await.try_into_response()
            }
            S3Path::Object { bucket, key } => {
                let bucket = bucket.into();
                let key = key.into();

                if let Some(copy_source) = extract_header(&req, &*X_AMZ_COPY_SOURCE)? {
                    crate::header::CopySource::try_match(copy_source)
                        .map_err(|e| S3Error::InvalidRequest(e.into()))?;

                    let mut input: CopyObjectRequest = CopyObjectRequest {
                        bucket,
                        key,
                        copy_source: copy_source.into(),
                        ..CopyObjectRequest::default()
                    };

                    assign_opt!(from req to input: headers [
                            (&*X_AMZ_ACL, acl),
                            (CACHE_CONTROL, cache_control),
                            (CONTENT_DISPOSITION, content_disposition),
                            (CONTENT_ENCODING, content_encoding),
                            (CONTENT_LANGUAGE, content_language),
                            (CONTENT_TYPE, content_type),
                            (&*X_AMZ_COPY_SOURCE_IF_MATCH,copy_source_if_match),
                            (&*X_AMZ_COPY_SOURCE_IF_MODIFIED_SINCE,copy_source_if_modified_since),
                            (&*X_AMZ_COPY_SOURCE_IF_NONE_MATCH,copy_source_if_none_match),
                            (&*X_AMZ_COPY_SOURCE_IF_UNMODIFIED_SINCE,copy_source_if_unmodified_since),
                            (EXPIRES, expires),
                            (&*X_AMZ_GRANT_FULL_CONTROL, grant_full_control),
                            (&*X_AMZ_GRANT_READ, grant_read),
                            (&*X_AMZ_GRANT_READ_ACP, grant_read_acp),
                            (&*X_AMZ_GRANT_WRITE_ACP, grant_write_acp),
                            (&*X_AMZ_METADATA_DIRECTIVE, metadata_directive),
                            (&*X_AMZ_TAGGING_DIRECTIVE, tagging_directive),
                            (&*X_AMZ_SERVER_SIDE_ENCRYPTION,server_side_encryption),
                            (&*X_AMZ_STORAGE_CLASS, storage_class),
                            (&*X_AMZ_WEBSITE_REDIRECT_LOCATION,website_redirect_location),
                            (&*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,sse_customer_algorithm),
                            (&*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,sse_customer_key),
                            (&*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,sse_customer_key_md5),
                            (&*X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,ssekms_key_id),
                            (&*X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT,ssekms_encryption_context),
                            (&*X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,copy_source_sse_customer_algorithm),
                            (&*X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,copy_source_sse_customer_key),
                            (&*X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,copy_source_sse_customer_key_md5),
                            (&*X_AMZ_REQUEST_PAYER, request_payer),
                            (&*X_AMZ_TAGGING, tagging),
                            (&*X_AMZ_OBJECT_LOCK_MODE, object_lock_mode),
                            (&*X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE,object_lock_retain_until_date),
                            (&*X_AMZ_OBJECT_LOCK_LEGAL_HOLD, object_lock_legal_hold_status),
                    ]);

                    return self.storage.copy_object(input).await.try_into_response();
                }

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
                    body: Some(crate::dto::ByteStream::new(body)),
                    ..PutObjectRequest::default() // TODO: handle other fields
                };

                self.storage.put_object(input).await.try_into_response()
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
                self.storage.delete_bucket(input).await.try_into_response()
            }
            S3Path::Object { bucket, key } => {
                let input: DeleteObjectRequest = DeleteObjectRequest {
                    bucket: bucket.into(),
                    key: key.into(),
                    ..DeleteObjectRequest::default() // TODO: handle other fields
                };

                self.storage.delete_object(input).await.try_into_response()
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
                self.storage.head_bucket(input).await.try_into_response()
            }
            S3Path::Object { bucket, key } => {
                let input = HeadObjectRequest {
                    bucket: bucket.into(),
                    key: key.into(),
                    ..HeadObjectRequest::default() // TODO: handle other fields
                };
                self.storage.head_object(input).await.try_into_response()
            }
        }
    }
}
