//! S3 operations

#![allow(clippy::unnecessary_wraps, clippy::panic_in_result_fn)]

mod complete_multipart_upload;
mod copy_object;
mod create_bucket;
mod create_multipart_upload;
mod delete_bucket;
mod delete_object;
mod delete_objects;
mod get_bucket_location;
mod get_object;
mod head_bucket;
mod head_object;
mod list_buckets;
mod list_objects;
mod list_objects_v2;
mod put_object;
mod upload_part;

use crate::data_structures::{OrderedHeaders, OrderedQs};
use crate::errors::S3Result;
use crate::path::S3Path;
use crate::storage::S3Storage;
use crate::streams::multipart::Multipart;
use crate::{async_trait, Body, BoxStdError, Mime, Request, Response};

use std::fmt::Debug;
use std::mem;

use hyper::header::AsHeaderName;

/// setup handlers
pub fn setup_handlers() -> Vec<Box<dyn S3Handler + Send + Sync + 'static>> {
    macro_rules! zst_handlers{
        [$($m:ident,)+] => {vec![$(Box::new($m::Handler),)+]}
    }

    zst_handlers![
        complete_multipart_upload,
        copy_object,
        create_bucket,
        create_multipart_upload,
        delete_bucket,
        delete_object,
        delete_objects,
        get_bucket_location,
        get_object,
        head_bucket,
        head_object,
        list_buckets,
        list_objects,
        list_objects_v2,
        put_object,
        upload_part,
    ]
}

/// S3 operation handler
#[async_trait]
pub trait S3Handler {
    /// determine if the handler matches current request
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool;

    /// handle the request
    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response>;
}

/// Request Context
#[derive(Debug)]
pub struct ReqContext<'a> {
    /// req
    pub req: &'a Request,
    /// ordered headers
    pub headers: OrderedHeaders<'a>,
    /// query strings
    pub query_strings: Option<OrderedQs>,
    /// body
    pub body: Body,
    /// s3 path
    pub path: S3Path<'a>,
    /// mime
    pub mime: Option<Mime>,
    /// multipart/form-data
    pub multipart: Option<Multipart>,
}

impl<'a> ReqContext<'a> {
    /// take request body
    fn take_body(&mut self) -> Body {
        mem::take(&mut self.body)
    }

    /// get (bucket, key)
    fn unwrap_object_path(&self) -> (&'a str, &'a str) {
        match self.path {
            S3Path::Object { bucket, key } => (bucket, key),
            S3Path::Root | S3Path::Bucket { .. } => {
                panic!("expected S3Path::Object, found: {:?}", self.path)
            }
        }
    }

    /// get bucket
    fn unwrap_bucket_path(&self) -> &'a str {
        match self.path {
            S3Path::Bucket { bucket } => bucket,
            S3Path::Root | S3Path::Object { .. } => {
                panic!("expected S3Path::Bucket, found: {:?}", self.path)
            }
        }
    }

    /// get query string
    fn unwrap_qs(&self, name: &str) -> &str {
        match self.query_strings.as_ref().and_then(|qs| qs.get(name)) {
            Some(s) => s,
            None => panic!("expected query string: name = {:?}", name),
        }
    }

    /// get header
    fn unwrap_header(&self, name: impl AsHeaderName + Debug) -> &str {
        let s = match self.headers.get(name.as_str()) {
            Some(s) => s,
            None => panic!("expected header: name = {:?}", name),
        };
        drop(name);
        s
    }
}

/// wrap any error as an internal error
fn wrap_internal_error(
    f: impl FnOnce(&mut Response) -> Result<(), BoxStdError>,
) -> S3Result<Response> {
    let mut res = Response::new(Body::empty());
    match f(&mut res) {
        Ok(()) => Ok(res),
        Err(e) => Err(internal_error!(e)),
    }
}
