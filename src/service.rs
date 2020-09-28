//! Generic S3 service which wraps a S3 storage

use crate::{
    auth::S3Auth,
    chunked_stream::ChunkedStream,
    error::{S3Error, S3Result, XmlErrorResponse},
    headers::names::{X_AMZ_CONTENT_SHA256, X_AMZ_COPY_SOURCE, X_AMZ_DATE},
    headers::AmzContentSha256,
    headers::AmzDate,
    headers::AuthorizationV4,
    headers::CredentialV4,
    multipart, ops,
    output::S3Output,
    path::S3Path,
    path::{self, S3PathErrorKind},
    query::GetQuery,
    query::PostQuery,
    query::PutQuery,
    signature_v4::{self, Payload},
    storage::S3Storage,
    utils::Also,
    utils::OrderedHeaders,
    utils::{crypto, time, Apply, OrderedQs, RequestExt},
    BoxStdError, Request, Response, S3ErrorCode,
};

use std::{
    fmt::{self, Debug},
    future::Future,
    io, mem,
    ops::Deref,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{future::BoxFuture, Stream, StreamExt};
use hyper::{
    header::{AsHeaderName, AUTHORIZATION, CONTENT_TYPE},
    Body, Method,
};
use log::{debug, error};
use mime::Mime;
use multipart::Multipart;
use serde::de::DeserializeOwned;

/// Generic S3 service which wraps a S3 storage
pub struct S3Service {
    /// storage
    storage: Box<dyn S3Storage + Send + Sync + 'static>,

    /// auth
    auth: Option<Box<dyn S3Auth + Send + Sync + 'static>>,
}

/// Shared S3 service
#[derive(Debug)]
pub struct SharedS3Service {
    /// inner service
    inner: Arc<S3Service>,
}

impl Debug for S3Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "S3Service{{...}}")
    }
}

impl Deref for SharedS3Service {
    type Target = S3Service;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl Clone for SharedS3Service {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl hyper::service::Service<Request> for SharedS3Service {
    type Response = Response;

    type Error = BoxStdError;

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

macro_rules! call_s3_operation{
    (@debug $op:ident)=>{
        debug!("call_s3_operation at {}:{}: {}", file!(), line!(), stringify!($op));
    };

    ($op:ident with () by $storage:expr)  => {{
        call_s3_operation!(@debug $op);
        let input = wrap_handle_sync(ops::$op::extract)?;
        $storage.$op(input).await.try_into_response()
    }};

    ($op:ident with ($($arg:expr),+) by $storage:expr)  => {{
        call_s3_operation!(@debug $op);
        let input = wrap_handle_sync(||ops::$op::extract($($arg),+))?;
        $storage.$op(input).await.try_into_response()
    }};

    ($op:ident with async ($($arg:expr),*) by $storage:expr)  => {{
        call_s3_operation!(@debug $op);
        let input = wrap_handle(ops::$op::extract($($arg),*)).await?;
        $storage.$op(input).await.try_into_response()
    }};
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

/// Request Context
#[derive(Debug)]
struct ReqContext<'a> {
    /// req
    req: &'a Request,
    /// ordered headers
    headers: OrderedHeaders<'a>,
    /// query strings
    query_strings: Option<OrderedQs>,
    /// body
    body: Body,
    /// s3 path
    path: S3Path<'a>,
    /// mime
    mime: Option<Mime>,
    /// multipart/form-data
    multipart: Option<Multipart>,
}

/// Create a `S3Error::Other`
macro_rules! code_error {
    ($code:expr,$msg:expr) => {{
        let msg_string: String = $msg.into();
        debug!(
            "generate code error at {}:{}: code = {:?}, msg = {}",
            file!(),
            line!(),
            $code,
            msg_string
        );
        S3Error::Other(XmlErrorResponse::from_code_msg($code, msg_string))
    }};
}

/// Create a `S3Error::NotSupported`
macro_rules! not_supported {
    () => {{
        debug!("generate NotSupported at {}:{}", file!(), line!());
        S3Error::NotSupported
    }};
}

/// extract `AmzContentSha256` from headers
fn extract_amz_content_sha256<'a>(
    headers: &'_ OrderedHeaders<'a>,
) -> S3Result<AmzContentSha256<'a>> {
    headers
        .get(&*X_AMZ_CONTENT_SHA256)
        .ok_or_else(|| {
            code_error!(
                S3ErrorCode::InvalidRequest,
                "Missing header: x-amz-content-sha256"
            )
        })?
        .apply(AmzContentSha256::from_header_str)
        .map_err(|_| {
            code_error!(
                S3ErrorCode::XAmzContentSHA256Mismatch,
                "Invalid header: x-amz-content-sha256"
            )
        })
}

/// extract `AuthorizationV4` from headers
fn extract_authorization_v4<'a>(headers: &'_ OrderedHeaders<'a>) -> S3Result<AuthorizationV4<'a>> {
    headers
        .get(AUTHORIZATION)
        .ok_or_else(|| code_error!(S3ErrorCode::InvalidRequest, "Missing header: Authorization"))?
        .apply(AuthorizationV4::from_header_str)
        .map_err(|_| code_error!(S3ErrorCode::InvalidRequest, "Invalid header: Authorization"))
}

/// extract `AmzDate` from headers
fn extract_amz_date(headers: &'_ OrderedHeaders<'_>) -> S3Result<AmzDate> {
    headers
        .get(&*X_AMZ_DATE)
        .ok_or_else(|| code_error!(S3ErrorCode::InvalidRequest, "Missing header: x-amz-date"))?
        .apply(AmzDate::from_header_str)
        .map_err(|_| code_error!(S3ErrorCode::InvalidRequest, "Invalid header: x-amz-date"))
}

/// replace `body` with an empty body and transform it to IO stream
fn take_io_body(body: &mut Body) -> impl Stream<Item = io::Result<Bytes>> + Send + 'static {
    mem::replace(body, Body::empty()).map(|try_chunk| {
        try_chunk.map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Error obtaining chunk: {}", e),
            )
        })
    })
}

impl S3Service {
    /// Constructs a S3 service
    pub fn new(storage: impl S3Storage + Send + Sync + 'static) -> Self {
        Self {
            storage: Box::new(storage),
            auth: None,
        }
    }

    /// Set the authentication provider
    pub fn set_auth<A>(&mut self, auth: A)
    where
        A: S3Auth + Send + Sync + 'static,
    {
        self.auth = Some(Box::new(auth));
    }

    /// Converts `S3Service` to `SharedS3Service`
    #[must_use]
    pub fn into_shared(self) -> SharedS3Service {
        SharedS3Service {
            inner: Arc::new(self),
        }
    }

    /// Call the s3 service with `hyper::Request<hyper::Body>`
    /// # Errors
    /// Returns an `Err` if the service failed
    pub async fn hyper_call(&self, req: Request) -> Result<Response, BoxStdError> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        debug!("{} \"{:?}\" request:\n{:#?}", method, uri, req);

        let (result, duration) = time::count_duration(self.handle(req)).await;

        let result = result.or_else(|err| {
            if let S3Error::Other(e) = err {
                e.try_into_response()
            } else {
                Err(err)
            }
        });

        match result {
            Ok(resp) => {
                debug!(
                    "{} \"{:?}\" => duration: {:?}, response:\n{:#?}",
                    method, uri, duration, resp
                );
                resp
            }
            Err(err) => {
                error!(
                    "{} \"{:?}\" => duration: {:?}, error:\n{:#?}",
                    method, uri, duration, err
                );
                XmlErrorResponse::from_code_msg(S3ErrorCode::InternalError, err.to_string())
                    .try_into_response()?
            }
        }
        .apply(Ok)
    }

    /// handle request
    async fn handle(&self, mut req: Request) -> S3Result<Response> {
        let body = req.take_body();

        let path = S3Path::try_from_path(req.uri().path()).map_err(|e| {
            match e.kind() {
                S3PathErrorKind::InvalidPath => {
                    (S3ErrorCode::InvalidURI, "Couldn't parse the specified URI.")
                }
                S3PathErrorKind::InvalidBucketName => (
                    S3ErrorCode::InvalidBucketName,
                    "The specified bucket is not valid.",
                ),
                S3PathErrorKind::KeyTooLong => {
                    (S3ErrorCode::KeyTooLongError, "Your key is too long.")
                }
            }
            .apply(|(code, msg)| code_error!(code, msg))
        })?;

        let headers = OrderedHeaders::from_req(&req)
            .map_err(|_| code_error!(S3ErrorCode::InvalidRequest, "Invalid headers"))?;

        let query_strings: Option<OrderedQs> = req
            .uri()
            .query()
            .map(OrderedQs::from_query)
            .transpose()
            .map_err(|_| code_error!(S3ErrorCode::InvalidRequest, "Invalid query strings"))?;

        let mime = {
            headers
                .get(CONTENT_TYPE)
                .map(|value| {
                    value.parse::<Mime>().map_err(|_| {
                        code_error!(S3ErrorCode::InvalidRequest, "Invalid header: Content-Type")
                    })
                })
                .transpose()?
        };

        let mut ctx: ReqContext<'_> = ReqContext {
            req: &req,
            headers,
            query_strings,
            path,
            body,
            mime,
            multipart: None,
        };

        self.check_signature(&mut ctx).await?;

        let resp = match *ctx.req.method() {
            Method::GET => self.handle_get(ctx).await?,
            Method::POST => self.handle_post(ctx).await?,
            Method::PUT => self.handle_put(ctx).await?,
            Method::DELETE => self.handle_delete(ctx).await?,
            Method::HEAD => self.handle_head(ctx).await?,
            _ => return Err(not_supported!()),
        };
        Ok(resp)
    }

    /// check presigned url (v4)
    async fn check_presigned_url(&self, ctx: &mut ReqContext<'_>) -> S3Result<()> {
        let qs = ctx.query_strings.as_ref().unwrap_or_else(|| unreachable!());

        let presigned_url = signature_v4::PresignedUrl::from_query(qs)
            .map_err(|_| code_error!(S3ErrorCode::InvalidRequest, "Missing presigned fields"))?;

        if let Some(content_sha256) = ctx.headers.get(&*X_AMZ_CONTENT_SHA256) {
            if AmzContentSha256::from_header_str(content_sha256).is_err() {
                return Err(code_error!(
                    S3ErrorCode::XAmzContentSHA256Mismatch,
                    "Invalid header: x-amz-content-sha256"
                ));
            }
        }

        let auth_provider = match self.auth {
            Some(ref a) => &**a,
            None => return Err(not_supported!()),
        };

        let secret_key = auth_provider
            .get_secret_access_key(presigned_url.credential.access_key_id)
            .await?
            .ok_or_else(|| {
                code_error!(S3ErrorCode::NotSignedUp, "Your account is not signed up")
            })?;

        let signature = {
            let headers = ctx
                .headers
                .map_signed_headers(&presigned_url.signed_headers);

            let canonical_request = signature_v4::create_presigned_canonical_request(
                ctx.req.method(),
                ctx.req.uri().path(),
                qs.as_ref(),
                &headers,
            );

            let region = presigned_url.credential.aws_region;
            let amz_date = &presigned_url.amz_date;
            let string_to_sign =
                signature_v4::create_string_to_sign(&canonical_request, amz_date, region);

            signature_v4::calculate_signature(&string_to_sign, &secret_key, amz_date, region)
        };

        if signature != presigned_url.signature {
            return Err(code_error!(
                S3ErrorCode::SignatureDoesNotMatch,
                "The request signature we calculated does not match the signature you provided."
            ));
        }

        Ok(())
    }

    /// check POST signature (v4)
    async fn check_post_signature(&self, ctx: &mut ReqContext<'_>) -> S3Result<()> {
        /// util method
        fn find_info(multipart: &Multipart) -> Option<(&str, &str, &str, &str, &str)> {
            let policy = multipart.find_field_value("policy")?;
            let x_amz_algorithm = multipart.find_field_value("x-amz-algorithm")?;
            let x_amz_credential = multipart.find_field_value("x-amz-credential")?;
            let x_amz_date = multipart.find_field_value("x-amz-date")?;
            let x_amz_signature = multipart.find_field_value("x-amz-signature")?;
            Some((
                policy,
                x_amz_algorithm,
                x_amz_credential,
                x_amz_date,
                x_amz_signature,
            ))
        }

        let auth_provider = match self.auth {
            Some(ref a) => &**a,
            None => return Err(not_supported!()),
        };

        let mime = ctx.mime.as_ref().unwrap_or_else(|| unreachable!());

        let boundary = mime
            .get_param(mime::BOUNDARY)
            .ok_or_else(|| code_error!(S3ErrorCode::InvalidRequest, "Missing boundary"))?;

        let body = take_io_body(&mut ctx.body);

        let multipart = multipart::transform_multipart(body, boundary.as_str().as_bytes())
            .await
            .map_err(|e| code_error!(S3ErrorCode::InvalidRequest, e.to_string()))?;

        let (policy, x_amz_algorithm, x_amz_credential, x_amz_date, x_amz_signature) = {
            match find_info(&multipart) {
                None => {
                    return Err(code_error!(
                        S3ErrorCode::InvalidRequest,
                        "Missing required fields"
                    ))
                }
                Some(ans) => ans,
            }
        };

        // check policy
        if !crypto::is_base64_encoded(policy.as_bytes()) {
            return Err(code_error!(
                S3ErrorCode::InvalidRequest,
                "Invalid field: policy"
            ));
        }

        // check x_amz_algorithm
        if x_amz_algorithm != "AWS4-HMAC-SHA256" {
            return Err(not_supported!());
        }

        // check x_amz_credential
        let (_, credential) = CredentialV4::parse_by_nom(x_amz_credential).map_err(|_| {
            code_error!(
                S3ErrorCode::InvalidRequest,
                "Invalid field: x-amz-credential"
            )
        })?;

        // check x_amz_date
        let amz_date = AmzDate::from_header_str(x_amz_date)
            .map_err(|_| code_error!(S3ErrorCode::InvalidRequest, "Invalid field: x-amz-date"))?;

        // fetch secret_key
        let secret_key = auth_provider
            .get_secret_access_key(credential.access_key_id)
            .await?
            .ok_or_else(|| {
                code_error!(S3ErrorCode::NotSignedUp, "Your account is not signed up")
            })?;

        // calculate signature
        let string_to_sign = policy;
        let signature = signature_v4::calculate_signature(
            string_to_sign,
            &secret_key,
            &amz_date,
            credential.aws_region,
        );

        // check x_amz_signature
        if signature != x_amz_signature {
            return Err(code_error!(
                S3ErrorCode::SignatureDoesNotMatch,
                "The request signature we calculated does not match the signature you provided."
            ));
        }

        // store ctx value
        ctx.multipart = Some(multipart);

        Ok(())
    }

    /// check signature (v4)
    async fn check_signature(&self, ctx: &mut ReqContext<'_>) -> S3Result<()> {
        // --- POST auth ---
        if ctx.req.method() == Method::POST {
            if let Some(mime) = ctx.mime.as_ref() {
                if mime.type_() == mime::MULTIPART && mime.subtype() == mime::FORM_DATA {
                    return self.check_post_signature(ctx).await;
                }
            }
        }

        // --- query auth ---
        if let Some(qs) = ctx.query_strings.as_ref() {
            if qs.get("X-Amz-Signature").is_some() {
                return self.check_presigned_url(ctx).await;
            }
        }

        let amz_content_sha256 = extract_amz_content_sha256(&ctx.headers)?;

        // --- header auth ---
        let is_stream = match &amz_content_sha256 {
            AmzContentSha256::UnsignedPayload => return Ok(()),
            AmzContentSha256::SingleChunk { .. } => false,
            AmzContentSha256::MultipleChunks => true,
        };

        let auth_provider = match self.auth {
            Some(ref a) => &**a,
            None => return Err(not_supported!()),
        };

        let auth: AuthorizationV4<'_> =
            extract_authorization_v4(&ctx.headers)?.also(|auth| auth.signed_headers.sort());

        let secret_key = auth_provider
            .get_secret_access_key(auth.credential.access_key_id)
            .await?
            .ok_or_else(|| {
                code_error!(S3ErrorCode::NotSignedUp, "Your account is not signed up")
            })?;

        let amz_date = extract_amz_date(&ctx.headers)?;

        let signature = {
            let method = ctx.req.method();
            let uri_path = ctx.req.uri().path();
            let query_strings: &[(String, String)] =
                ctx.query_strings.as_ref().map_or(&[], AsRef::as_ref);

            // here requires that `auth.signed_headers` is sorted
            let headers = ctx.headers.map_signed_headers(&auth.signed_headers);

            let canonical_request = if is_stream {
                signature_v4::create_canonical_request(
                    method,
                    uri_path,
                    query_strings,
                    &headers,
                    Payload::MultipleChunks,
                )
            } else {
                let bytes = std::mem::replace(&mut ctx.body, Body::empty())
                    .apply(hyper::body::to_bytes)
                    .await
                    .map_err(|e| S3Error::InvalidRequest(e.into()))?;

                let payload = if bytes.is_empty() {
                    Payload::Empty
                } else {
                    Payload::SingleChunk(&bytes)
                };
                let ans = signature_v4::create_canonical_request(
                    method,
                    uri_path,
                    query_strings,
                    &headers,
                    payload,
                );

                ctx.body = Body::from(bytes);

                ans
            };

            let region = auth.credential.aws_region;
            let string_to_sign =
                signature_v4::create_string_to_sign(&canonical_request, &amz_date, region);

            signature_v4::calculate_signature(&string_to_sign, &secret_key, &amz_date, region)
        };

        if signature != auth.signature {
            return Err(code_error!(
                S3ErrorCode::SignatureDoesNotMatch,
                "The request signature we calculated does not match the signature you provided."
            ));
        }

        if is_stream {
            let body = take_io_body(&mut ctx.body);

            let chunked_stream = ChunkedStream::new(
                body,
                signature.into(),
                amz_date,
                auth.credential.aws_region.into(),
                secret_key.into(),
            );

            ctx.body = Body::wrap_stream(chunked_stream);
        }

        Ok(())
    }

    /// handle GET request
    async fn handle_get(&self, ctx: ReqContext<'_>) -> S3Result<Response> {
        let ReqContext { req, path, .. } = ctx;

        match path {
            S3Path::Root => call_s3_operation!(list_buckets with () by self.storage),
            S3Path::Bucket { bucket } => {
                let query = match extract_query::<GetQuery>(req)? {
                    None => {
                        return call_s3_operation!(list_objects with (req, None, bucket) by self.storage)
                    }
                    Some(query) => query,
                };

                if query.location.is_some() {
                    return call_s3_operation!(get_bucket_location with (bucket) by self.storage);
                }

                match query.list_type {
                    None => {
                        call_s3_operation!(list_objects with (req,Some(query),bucket) by self.storage)
                    }
                    Some(2) => {
                        call_s3_operation!(list_objects_v2 with (req,query,bucket) by self.storage)
                    }
                    Some(_) => Err(not_supported!()),
                }
            }
            S3Path::Object { bucket, key } => {
                call_s3_operation!(get_object with (req, bucket, key) by self.storage)
            }
        }
    }

    /// handle POST request
    async fn handle_post(&self, ctx: ReqContext<'_>) -> S3Result<Response> {
        let ReqContext {
            req,
            path,
            body,
            headers,
            multipart,
            ..
        } = ctx;

        match path {
            S3Path::Root => Err(not_supported!()), // TODO: impl handler
            S3Path::Bucket { bucket } => {
                if let Some(multipart) = multipart {
                    let key = {
                        let key_str = multipart.find_field_value("key").ok_or_else(|| {
                            code_error!(S3ErrorCode::UserKeyMustBeSpecified, "Missing key")
                        })?;

                        if path::check_key(key_str) {
                            key_str.to_owned()
                        } else {
                            return Err(code_error!(
                                S3ErrorCode::KeyTooLongError,
                                "Your key is too long."
                            ));
                        }
                    };
                    return call_s3_operation!(put_object with (req, body, bucket,&key,Some(multipart),&headers) by self.storage);
                }

                let query = match extract_query::<PostQuery>(req)? {
                    None => return Err(not_supported!()),
                    Some(query) => query,
                };

                if query.delete.is_some() {
                    return call_s3_operation!(delete_objects with async (req, body, bucket) by self.storage);
                }

                // TODO: impl handler

                Err(not_supported!())
            }
            S3Path::Object { bucket, key } => {
                if multipart.is_some() {
                    return Err(code_error!(
                        S3ErrorCode::MethodNotAllowed,
                        "The specified method is not allowed against this resource."
                    ));
                }

                let query = match extract_query::<PostQuery>(req)? {
                    None => return Err(not_supported!()),
                    Some(query) => query,
                };

                if query.uploads.is_some() {
                    return call_s3_operation!(create_multipart_upload with (req, bucket, key) by self.storage);
                }

                if let Some(upload_id) = query.upload_id {
                    return call_s3_operation!(complete_multipart_upload with async (req,body,bucket,key,upload_id) by self.storage);
                }

                Err(not_supported!()) // TODO: impl handler
            }
        }
    }

    /// handle PUT request
    async fn handle_put(&self, ctx: ReqContext<'_>) -> S3Result<Response> {
        let ReqContext {
            req,
            path,
            body,
            multipart,
            headers,
            ..
        } = ctx;

        match path {
            S3Path::Root => Err(not_supported!()), // TODO: impl handler
            S3Path::Bucket { bucket } => {
                call_s3_operation!(create_bucket with async (req, body, bucket) by self.storage)
            }
            S3Path::Object { bucket, key } => {
                if let Some(copy_source) = extract_header(req, &*X_AMZ_COPY_SOURCE)? {
                    return call_s3_operation!(copy_object with (req, bucket,key,copy_source) by self.storage);
                }
                let query = match extract_query::<PutQuery>(req)? {
                    None => {
                        return call_s3_operation!(put_object with (req,body,bucket,key,multipart,&headers) by self.storage)
                    }
                    Some(query) => query,
                };
                if let Some(part_number) = query.part_number {
                    if let Some(upload_id) = query.upload_id {
                        return call_s3_operation!(upload_part with (req,bucket, key,part_number,upload_id,body) by self.storage);
                    }
                }
                Err(not_supported!())
            }
        }
    }

    /// handle DELETE request
    async fn handle_delete(&self, ctx: ReqContext<'_>) -> S3Result<Response> {
        let ReqContext { req, path, .. } = ctx;

        match path {
            S3Path::Root => Err(not_supported!()), // TODO: impl handler
            S3Path::Bucket { bucket } => {
                call_s3_operation!(delete_bucket with (bucket) by self.storage)
            }
            S3Path::Object { bucket, key } => {
                call_s3_operation!(delete_object with (req, bucket,key) by self.storage)
            }
        }
    }

    /// handle HEAD request
    async fn handle_head(&self, ctx: ReqContext<'_>) -> S3Result<Response> {
        let ReqContext { req, path, .. } = ctx;

        match path {
            S3Path::Root => Err(not_supported!()), // TODO: impl handler
            S3Path::Bucket { bucket } => {
                call_s3_operation!(head_bucket with (bucket) by self.storage)
            }
            S3Path::Object { bucket, key } => {
                call_s3_operation!(head_object with (req, bucket, key) by self.storage)
            }
        }
    }
}
